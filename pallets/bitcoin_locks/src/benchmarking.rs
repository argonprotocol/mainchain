#![cfg(feature = "runtime-benchmarks")]

extern crate alloc;

use super::*;
use alloc::vec::Vec;
use argon_bitcoin::{derive_xpub, xpriv_from_seed};
use argon_primitives::{
	bitcoin::{
		BitcoinHeight, BitcoinNetwork, BitcoinScriptPubkey, BitcoinSignature, BitcoinXPub,
		CompressedBitcoinPubkey, H256Le, OpaqueBitcoinXpub, Satoshis, UtxoId, UtxoRef,
		SATOSHIS_PER_BITCOIN,
	},
	providers::BitcoinFissionLockProvider,
	vault::{Vault, VaultTerms},
	BitcoinUtxoEvents, PriceProvider,
};
use frame_benchmarking::v2::*;
use pallet_prelude::benchmarking::{
	benchmark_bitcoin_utxo_tracker_state, benchmark_bitcoin_vault_provider_state,
	reset_benchmark_bitcoin_locks_runtime_state, reset_benchmark_bitcoin_utxo_tracker_state,
	reset_benchmark_bitcoin_vault_provider_state, reset_benchmark_price_provider_state,
	set_benchmark_bitcoin_locks_runtime_state, set_benchmark_bitcoin_utxo_tracker_state,
	set_benchmark_bitcoin_vault_provider_state, set_benchmark_price_provider_state,
	BenchmarkBitcoinLocksRuntimeState, BenchmarkBitcoinUtxoTrackerState,
	BenchmarkPriceProviderState,
};

// Small linear fit ranges used to generate the per-item slope for hook weight components.
const EXPIRING_LOCKS_BENCH_RANGE_END: u32 = 20;
const OVERDUE_RELEASES_BENCH_RANGE_END: u32 = 20;
const ORPHAN_EXPIRATIONS_BENCH_RANGE_END: u32 = 20;
const PENDING_FUNDING_BENCH_RANGE_END: u32 = 20;

#[benchmarks(
	where
		<T as frame_system::Config>::AccountId: Ord,
		T: pallet_bitcoin_fissions::Config<Balance = <T as Config>::Balance>,
)]
mod benchmarks {
	use super::*;
	use frame_support::traits::Hooks;
	use frame_system::RawOrigin;

	#[benchmark]
	fn create_receive_address() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let owner: T::AccountId = account("bitcoin-lock-owner", 0, 0);
		let operator: T::AccountId = account("vault-operator", 0, 0);
		let satoshis = benchmark_satoshis::<T>();
		let owner_pubkey = benchmark_pubkey::<T>(1)?;
		seed_price_state(100_000, 1, 1);
		let vault_id = create_vault::<T>(&operator, 1, benchmark_vault_securitization())?;
		let delegate: T::AccountId = account("bitcoin-lock-delegate", 0, 0);
		let mut state =
			benchmark_bitcoin_vault_provider_state::<T::AccountId, <T as Config>::Balance>();
		state
			.vaults
			.get_mut(&vault_id)
			.ok_or(BenchmarkError::Stop("missing benchmark vault"))?
			.delegate_account_id = Some(delegate.clone());
		set_benchmark_bitcoin_vault_provider_state(state);
		let microgons_at_target_per_btc = benchmark_microgons_at_target_per_btc::<T>()?;
		seed_microgons_at_target_per_btc_history::<T>(microgons_at_target_per_btc)?;
		let signature = T::FeeCouponSignature::decode(
			&mut polkadot_sdk::sp_runtime::traits::TrailingZeroInput::zeroes(),
		)
		.map_err(|_| BenchmarkError::Stop("failed to decode benchmark fee coupon signature"))?;
		let options = Some(LockOptions {
			microgons_at_target_per_btc,
			fee_coupon: Some(FeeCoupon {
				fee_discount: <T as Config>::Balance::zero(),
				securitization_space_to_unreserve: <T as Config>::Balance::zero(),
				expires_at_frame: T::CurrentFrameId::get(),
				nonce: 1,
				signature,
			}),
		});
		whitelist_account!(owner);

		#[extrinsic_call]
		_(RawOrigin::Signed(owner.clone()), vault_id, satoshis, owner_pubkey, options);

		let utxo_id = NextUtxoId::<T>::get().ok_or(BenchmarkError::Stop("missing utxo id"))?;
		assert!(LocksByUtxoId::<T>::contains_key(utxo_id));
		Ok(())
	}

	#[benchmark]
	fn request_release() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(3)?;
		let release_script_pubkey = benchmark_script_pubkey(1)?;
		let bitcoin_network_fee: Satoshis = 1_000;
		let owner = context.owner.clone();
		whitelist_account!(owner);

		#[extrinsic_call]
		_(
			RawOrigin::Signed(owner),
			context.utxo_id,
			release_script_pubkey.clone(),
			bitcoin_network_fee,
		);

		let request = LockReleaseRequestsByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing release request"))?;
		assert_eq!(request.bitcoin_network_fee, bitcoin_network_fee);
		assert_eq!(request.to_script_pubkey, release_script_pubkey);
		Ok(())
	}

	#[benchmark]
	fn cosign_release() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(4)?;
		let release_script_pubkey = benchmark_script_pubkey(2)?;
		let signature = benchmark_signature()?;
		Pallet::<T>::request_release(
			RawOrigin::Signed(context.owner.clone()).into(),
			context.utxo_id,
			release_script_pubkey,
			1_000,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed release request"))?;
		seed_migrated_release_hold::<T>(&context)?;
		let operator = context.operator.clone();
		whitelist_account!(operator);

		#[extrinsic_call]
		_(RawOrigin::Signed(operator), context.utxo_id, signature);

		assert!(!LocksByUtxoId::<T>::contains_key(context.utxo_id));
		Ok(())
	}

	#[benchmark]
	fn on_initialize_base() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let current_height: BitcoinHeight = 500;
		seed_bitcoin_heights(current_height, current_height);

		#[block]
		{
			let _ = Pallet::<T>::on_initialize(1u32.into());
		}

		Ok(())
	}

	#[benchmark]
	fn on_initialize_expiring_locks(
		e: Linear<1, EXPIRING_LOCKS_BENCH_RANGE_END>,
	) -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let mut expiring_utxo_ids = Vec::new();

		for index in 0..e {
			let context = create_funded_lock::<T>(10u8.saturating_add(index as u8))?;
			expiring_utxo_ids.push(context.utxo_id);
		}

		#[block]
		{
			let _ = Pallet::<T>::process_expiring_locks(expiring_utxo_ids);
		}

		Ok(())
	}

	#[benchmark]
	fn on_initialize_overdue_releases(
		o: Linear<1, OVERDUE_RELEASES_BENCH_RANGE_END>,
	) -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let current_frame = T::CurrentFrameId::get();
		let mut overdue_utxo_ids = Vec::new();

		for index in 0..o {
			let context = create_funded_lock::<T>(40u8.saturating_add(index as u8))?;
			seed_overdue_release_request::<T>(&context, current_frame)?;
			seed_migrated_release_hold::<T>(&context)?;
			overdue_utxo_ids.push(context.utxo_id);
		}

		#[block]
		{
			let _ = Pallet::<T>::process_overdue_releases(overdue_utxo_ids);
		}

		Ok(())
	}

	#[benchmark]
	fn on_initialize_orphan_expirations(
		r: Linear<1, ORPHAN_EXPIRATIONS_BENCH_RANGE_END>,
	) -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let mut orphan_expirations = Vec::new();

		for index in 0..r {
			let context = create_unfunded_lock::<T>(70u8.saturating_add(index as u8))?;
			let orphan_ref = benchmark_utxo_ref(20_000u32.saturating_add(index));
			seed_orphan_with_request::<T>(&context, orphan_ref.clone())?;
			orphan_expirations.push((context.owner.clone(), orphan_ref));
		}

		#[block]
		{
			let _ = Pallet::<T>::process_orphaned_utxo_expirations(orphan_expirations);
		}

		Ok(())
	}

	#[benchmark]
	fn on_initialize_pending_funding(
		p: Linear<1, PENDING_FUNDING_BENCH_RANGE_END>,
	) -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let mut pending_utxo_ids = Vec::new();

		for index in 0..p {
			let context = create_unfunded_lock::<T>(100u8.saturating_add(index as u8))?;
			pending_utxo_ids.push(context.utxo_id);
		}

		#[block]
		{
			let _ = Pallet::<T>::process_pending_funding_expirations(
				pending_utxo_ids,
				T::BitcoinBlockHeightChange::get().1,
			);
		}

		Ok(())
	}

	#[benchmark]
	fn admin_modify_minimum_locked_sats() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let new_minimum = benchmark_satoshis::<T>().saturating_add(1_000);

		#[extrinsic_call]
		_(RawOrigin::Root, new_minimum);

		assert_eq!(MinimumSatoshis::<T>::get(), new_minimum);
		Ok(())
	}

	#[benchmark]
	fn request_orphaned_utxo_release() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(6)?;
		let orphan_ref = benchmark_utxo_ref(1_001);
		let release_script_pubkey = benchmark_script_pubkey(3)?;
		seed_orphan::<T>(&context, orphan_ref.clone())?;
		let owner = context.owner.clone();
		whitelist_account!(owner);

		#[extrinsic_call]
		_(RawOrigin::Signed(owner), orphan_ref.clone(), release_script_pubkey.clone(), 1_000);

		let orphan = OrphanedUtxosByAccount::<T>::get(&context.owner, &orphan_ref)
			.ok_or(BenchmarkError::Stop("missing orphan after release request"))?;
		assert!(orphan.cosign_request.is_some());
		Ok(())
	}

	#[benchmark]
	fn cosign_orphaned_utxo_release() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(7)?;
		let orphan_ref = benchmark_utxo_ref(1_002);
		seed_orphan_with_request::<T>(&context, orphan_ref.clone())?;
		let signature = benchmark_signature()?;
		let operator = context.operator.clone();
		whitelist_account!(operator);

		#[extrinsic_call]
		_(RawOrigin::Signed(operator), context.owner.clone(), orphan_ref.clone(), signature);

		assert!(!OrphanedUtxosByAccount::<T>::contains_key(&context.owner, &orphan_ref));
		Ok(())
	}

	#[benchmark]
	fn resecuritize() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(9)?;
		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		let securitized_satoshis = context.satoshis.saturating_add(10_000);
		let microgons_at_target_per_btc = lock.microgons_at_target_per_btc;
		seed_microgons_at_target_per_btc_history::<T>(microgons_at_target_per_btc)?;
		seed_bitcoin_heights(100, 101);
		let delegate: T::AccountId = account("bitcoin-lock-delegate", 9, 0);
		let mut state =
			benchmark_bitcoin_vault_provider_state::<T::AccountId, <T as Config>::Balance>();
		state
			.vaults
			.get_mut(&lock.vault_id)
			.ok_or(BenchmarkError::Stop("missing benchmark vault"))?
			.delegate_account_id = Some(delegate);
		set_benchmark_bitcoin_vault_provider_state(state);
		let signature = T::FeeCouponSignature::decode(
			&mut polkadot_sdk::sp_runtime::traits::TrailingZeroInput::zeroes(),
		)
		.map_err(|_| BenchmarkError::Stop("failed to decode benchmark fee coupon signature"))?;
		let owner = context.owner.clone();
		whitelist_account!(owner);

		#[extrinsic_call]
		_(
			RawOrigin::Signed(owner),
			context.utxo_id,
			securitized_satoshis,
			Some(LockOptions {
				microgons_at_target_per_btc,
				fee_coupon: Some(FeeCoupon {
					fee_discount: <T as Config>::Balance::zero(),
					securitization_space_to_unreserve: <T as Config>::Balance::zero(),
					expires_at_frame: T::CurrentFrameId::get(),
					nonce: 1,
					signature,
				}),
			}),
		);

		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing lock after resecuritization"))?;
		assert_eq!(lock.securitized_satoshis, securitized_satoshis);
		Ok(())
	}

	#[benchmark]
	fn set_flexible() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(17)?;
		let mut lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		UtxoIdsByOwnerAccount::<T>::remove(&context.owner, context.utxo_id);
		lock.owner_account = context.operator.clone();
		LocksByUtxoId::<T>::insert(context.utxo_id, lock);
		UtxoIdsByOwnerAccount::<T>::insert(&context.operator, context.utxo_id, ());
		let operator = context.operator;
		whitelist_account!(operator);

		#[extrinsic_call]
		_(RawOrigin::Signed(operator), context.utxo_id, true);

		assert!(
			LocksByUtxoId::<T>::get(context.utxo_id)
				.ok_or(BenchmarkError::Stop("missing flexible lock"))?
				.is_flexible
		);
		Ok(())
	}

	#[benchmark]
	fn provider_fission_satoshis() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(18)?;
		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		let fissioned_satoshis = context.satoshis / 2;
		seed_microgons_at_target_per_btc_history::<T>(lock.microgons_at_target_per_btc)?;

		#[block]
		{
			<Pallet<T> as BitcoinFissionLockProvider<
				T::AccountId,
				<T as Config>::Balance,
			>>::fission_satoshis(
				&context.owner,
				context.utxo_id,
				fissioned_satoshis,
				lock.microgons_at_target_per_btc,
			)
			.map_err(|_| BenchmarkError::Stop("Fission allocation failed"))?;
		}

		assert_eq!(
			LocksByUtxoId::<T>::get(context.utxo_id)
				.ok_or(BenchmarkError::Stop("missing allocated lock"))?
				.fissioned_satoshis,
			fissioned_satoshis
		);
		Ok(())
	}

	#[benchmark]
	fn provider_validate_fission() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(19)?;
		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		let fissioned_satoshis = context.satoshis / 2;
		seed_microgons_at_target_per_btc_history::<T>(lock.microgons_at_target_per_btc)?;
		let (liquidity_promised, last_ratchet_tick) = <Pallet<T> as BitcoinFissionLockProvider<
			T::AccountId,
			<T as Config>::Balance,
		>>::fission_satoshis(
			&context.owner,
			context.utxo_id,
			fissioned_satoshis,
			lock.microgons_at_target_per_btc,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed Fission allocation"))?;
		pallet_bitcoin_fissions::FissionByOwnerAndId::<T>::insert(
			&context.owner,
			0,
			pallet_bitcoin_fissions::Fission {
				liquid_id: 0,
				utxo_id: context.utxo_id,
				satoshis: fissioned_satoshis,
				microgons_at_target_per_btc: lock.microgons_at_target_per_btc,
				last_ratchet_tick,
				liquidity_promised,
				created_at_argon_block: frame_system::Pallet::<T>::block_number(),
				ratchet_number: 0,
				last_updated_argon_block: frame_system::Pallet::<T>::block_number(),
			},
		);
		pallet_bitcoin_fissions::FissionIdsByLockId::<T>::try_mutate(
			context.utxo_id,
			|fission_ids| fission_ids.try_insert(0).map(|_| ()),
		)
		.map_err(|_| BenchmarkError::Stop("failed to index benchmark Fission"))?;

		#[block]
		{
			<Pallet<T> as BitcoinFissionLockProvider<
				T::AccountId,
				<T as Config>::Balance,
			>>::validate_fission(
				&context.owner,
				context.utxo_id,
				fissioned_satoshis,
				lock.microgons_at_target_per_btc,
				lock.securitization_tick,
				liquidity_promised,
				liquidity_promised,
			)
			.map_err(|_| BenchmarkError::Stop("Fission validation failed"))?;
		}

		Ok(())
	}

	#[benchmark]
	fn provider_calculate_liquidity_promised() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		seed_price_state(100_000, 1, 1);
		let fissioned_satoshis = benchmark_satoshis::<T>() / 2;
		let microgons_at_target_per_btc = benchmark_microgons_at_target_per_btc::<T>()?;
		let liquidity_promised;

		#[block]
		{
			liquidity_promised = <Pallet<T> as BitcoinFissionLockProvider<
				T::AccountId,
				<T as Config>::Balance,
			>>::calculate_liquidity_promised(
				fissioned_satoshis, microgons_at_target_per_btc
			)
			.map_err(|_| BenchmarkError::Stop("Fission liability calculation failed"))?;
		}

		assert!(!liquidity_promised.is_zero());
		Ok(())
	}

	#[benchmark]
	fn provider_fuse_satoshis() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(20)?;
		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		let fissioned_satoshis = context.satoshis / 2;
		seed_microgons_at_target_per_btc_history::<T>(lock.microgons_at_target_per_btc)?;
		<Pallet<T> as BitcoinFissionLockProvider<
			T::AccountId,
			<T as Config>::Balance,
		>>::fission_satoshis(
			&context.owner,
			context.utxo_id,
			fissioned_satoshis,
			lock.microgons_at_target_per_btc,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed Fission allocation"))?;

		#[block]
		{
			<Pallet<T> as BitcoinFissionLockProvider<
				T::AccountId,
				<T as Config>::Balance,
			>>::fuse_satoshis(
				&context.owner,
				context.utxo_id,
				fissioned_satoshis,
				lock.microgons_at_target_per_btc,
			)
			.map_err(|_| BenchmarkError::Stop("Fission deallocation failed"))?;
		}

		assert_eq!(
			LocksByUtxoId::<T>::get(context.utxo_id)
				.ok_or(BenchmarkError::Stop("missing deallocated lock"))?
				.fissioned_satoshis,
			0
		);
		Ok(())
	}

	#[benchmark]
	fn provider_utxo_detected() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(11)?;
		let funded_satoshis = context.satoshis.saturating_sub(1_000);

		#[block]
		{
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::utxo_detected(
				context.utxo_id,
				benchmark_utxo_ref(1_099),
				funded_satoshis,
				T::BitcoinBlockHeightChange::get().1,
			)
			.map_err(|_| BenchmarkError::Stop("utxo_detected failed"))?;
		}

		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing funded lock"))?;
		assert!(lock.is_funded());
		assert_eq!(lock.funded_satoshis, funded_satoshis);
		Ok(())
	}

	#[benchmark]
	fn provider_spent() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(16)?;
		let orphan_ref = benchmark_utxo_ref(1_104);
		let release_script_pubkey = benchmark_script_pubkey(11)?;
		seed_orphan_with_request::<T>(&context, orphan_ref)?;
		Pallet::<T>::request_release(
			RawOrigin::Signed(context.owner.clone()).into(),
			context.utxo_id,
			release_script_pubkey,
			1_000,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed release request"))?;
		seed_migrated_release_hold::<T>(&context)?;

		#[block]
		{
			let funding_ref = UtxoIdToFundingUtxoRef::<T>::get(context.utxo_id)
				.ok_or(BenchmarkError::Stop("missing funding UTXO ref"))?;
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::spent(context.utxo_id, funding_ref)
				.map_err(|_| BenchmarkError::Stop("spent failed"))?;
		}

		assert!(!LocksByUtxoId::<T>::contains_key(context.utxo_id));
		Ok(())
	}
}

struct LockBenchmarkContext<T: Config> {
	owner: T::AccountId,
	operator: T::AccountId,
	utxo_id: UtxoId,
	satoshis: Satoshis,
}

fn reset_benchmark_environment<T: Config>()
where
	T::AccountId: Ord,
{
	reset_benchmark_price_provider_state();
	reset_benchmark_bitcoin_utxo_tracker_state();
	reset_benchmark_bitcoin_locks_runtime_state();
	reset_benchmark_bitcoin_vault_provider_state();
	set_benchmark_price_provider_state(BenchmarkPriceProviderState::default());
	set_benchmark_bitcoin_utxo_tracker_state(BenchmarkBitcoinUtxoTrackerState {
		bitcoin_network: BitcoinNetwork::Regtest,
		bitcoin_block_height_change: (100, 100),
		..Default::default()
	});
	set_benchmark_bitcoin_locks_runtime_state(BenchmarkBitcoinLocksRuntimeState::default());
	frame_system::Pallet::<T>::set_block_number(1u32.into());
	NextUtxoId::<T>::kill();
	MinimumSatoshis::<T>::put(benchmark_satoshis::<T>().saturating_sub(1));
	MicrogonsAtTargetPerBtcHistory::<T>::kill();
}

fn benchmark_satoshis<T: Config>() -> Satoshis {
	100_000_000
}

fn benchmark_vault_securitization() -> u128 {
	200_000_000_000
}

fn benchmark_microgons_at_target_per_btc<T: Config>() -> Result<T::Balance, BenchmarkError> {
	let market_rate = T::PriceProvider::get_btc_price_in_market_microgons(SATOSHIS_PER_BITCOIN)
		.ok_or(BenchmarkError::Stop("benchmark bitcoin price should be available"))?;
	let r = T::PriceProvider::get_redemption_r_value()
		.ok_or(BenchmarkError::Stop("benchmark redemption r value should be available"))?;
	Ok(r.saturating_mul_int(market_rate))
}

fn seed_microgons_at_target_per_btc_history<T: Config>(
	microgons_at_target_per_btc: T::Balance,
) -> Result<(), BenchmarkError> {
	let history = BoundedVec::try_from(vec![(T::CurrentTick::get(), microgons_at_target_per_btc)])
		.map_err(|_| BenchmarkError::Stop("benchmark microgons per btc history overflow"))?;
	MicrogonsAtTargetPerBtcHistory::<T>::put(history);
	Ok(())
}

fn benchmark_block_hash(seed: u8) -> H256Le {
	H256Le([seed; 32])
}

fn benchmark_signature() -> Result<BitcoinSignature, BenchmarkError> {
	BitcoinSignature::try_from(vec![1u8])
		.map_err(|_| BenchmarkError::Stop("benchmark bitcoin signature should fit"))
}

fn benchmark_utxo_ref(seed: u32) -> UtxoRef {
	UtxoRef { txid: benchmark_block_hash(seed as u8), output_index: seed }
}

fn benchmark_script_pubkey(seed: u8) -> Result<BitcoinScriptPubkey, BenchmarkError> {
	BitcoinScriptPubkey::try_from(vec![seed; 32])
		.map_err(|_| BenchmarkError::Stop("benchmark script pubkey should fit"))
}

fn benchmark_pubkey<T: Config>(seed_hint: u8) -> Result<CompressedBitcoinPubkey, BenchmarkError> {
	let mut seed = [0u8; 32];
	seed[0] = seed_hint;
	let xpriv = xpriv_from_seed(&seed, T::GetBitcoinNetwork::get())
		.map_err(|_| BenchmarkError::Stop("benchmark xpriv generation failed"))?;
	let xpub = derive_xpub(&xpriv, "m/84'/0'/0'")
		.map_err(|_| BenchmarkError::Stop("benchmark xpub derivation failed"))?;
	Ok(xpub.public_key.serialize().into())
}

fn benchmark_xpub<T: Config>(seed_hint: u8) -> Result<BitcoinXPub, BenchmarkError> {
	let mut seed = [0u8; 32];
	seed[0] = seed_hint;
	let xpriv = xpriv_from_seed(&seed, T::GetBitcoinNetwork::get())
		.map_err(|_| BenchmarkError::Stop("benchmark xpriv generation failed"))?;
	let xpub = derive_xpub(&xpriv, "m/84'/0'/0'")
		.map_err(|_| BenchmarkError::Stop("benchmark xpub derivation failed"))?;
	BitcoinXPub::try_from(OpaqueBitcoinXpub::from(xpub))
		.map_err(|_| BenchmarkError::Stop("benchmark xpub conversion failed"))
}

fn seed_price_state(btc_usd_price: u128, argon_usd_price: u128, argon_target_price: u128) {
	set_benchmark_price_provider_state(BenchmarkPriceProviderState {
		btc_price_in_usd: Some(FixedU128::saturating_from_integer(btc_usd_price)),
		argon_price_in_usd: Some(FixedU128::saturating_from_integer(argon_usd_price)),
		argonot_price_in_usd: Some(FixedU128::saturating_from_integer(argon_usd_price)),
		argon_target_price_in_usd: Some(FixedU128::saturating_from_integer(argon_target_price)),
		circulation: 1_000_000,
	});
}

fn seed_bitcoin_heights(previous_height: BitcoinHeight, current_height: BitcoinHeight) {
	let mut state = benchmark_bitcoin_utxo_tracker_state();
	state.bitcoin_block_height_change = (previous_height, current_height);
	set_benchmark_bitcoin_utxo_tracker_state(state);
}

fn create_vault<T>(
	operator: &T::AccountId,
	seed_hint: u8,
	securitization: u128,
) -> Result<VaultId, BenchmarkError>
where
	T: Config,
	T::AccountId: Ord,
{
	let vault_id = seed_hint as VaultId + 1;
	let terms = VaultTerms {
		bitcoin_annual_percent_rate: FixedU128::from_rational(110u128, 100u128),
		bitcoin_base_fee: 1_000u128.into(),
		treasury_profit_sharing: Permill::from_percent(20),
	};
	let vault = Vault {
		operator_account_id: operator.clone(),
		delegate_account_id: None,
		securitization: securitization.into(),
		securitization_target: securitization.into(),
		securitization_locked: T::Balance::zero(),
		flexible_securitization_locked: T::Balance::zero(),
		reserved_securitization_space: T::Balance::zero(),
		securitization_pending_activation: T::Balance::zero(),
		locked_satoshis: 0,
		ratio_adjusted_satoshis: 0,
		flexible_ratio_adjusted_satoshis: 0,
		securitization_release_schedule: BoundedBTreeMap::default(),
		securitization_ratio: FixedU128::one(),
		is_closed: false,
		terms,
		pending_terms: None,
		opened_tick: 1,
		operational_minimum_release_tick: None,
	};
	let mut state = benchmark_bitcoin_vault_provider_state::<T::AccountId, T::Balance>();
	state.vaults.insert(vault_id, vault);
	state.vault_xpubs_by_id.insert(
		vault_id,
		(benchmark_xpub::<T>(seed_hint)?, benchmark_xpub::<T>(seed_hint.saturating_add(1))?),
	);
	set_benchmark_bitcoin_vault_provider_state(state);
	Ok(vault_id)
}

fn create_unfunded_lock<T>(seed_hint: u8) -> Result<LockBenchmarkContext<T>, BenchmarkError>
where
	T: Config,
	T::AccountId: Ord,
{
	let owner: T::AccountId = account("benchmark-lock-owner", seed_hint as u32, 0);
	let operator: T::AccountId = account("benchmark-vault-operator", seed_hint as u32, 0);
	let satoshis = benchmark_satoshis::<T>();
	let owner_pubkey = benchmark_pubkey::<T>(seed_hint.saturating_add(100))?;
	seed_price_state(100_000, 1, 1);
	seed_bitcoin_heights(100, 100);
	let vault_id = create_vault::<T>(&operator, seed_hint, benchmark_vault_securitization())?;
	Pallet::<T>::create_receive_address(
		frame_system::RawOrigin::Signed(owner.clone()).into(),
		vault_id,
		satoshis,
		owner_pubkey,
		None,
	)
	.map_err(|_| BenchmarkError::Stop("failed to create benchmark lock"))?;
	let utxo_id =
		NextUtxoId::<T>::get().ok_or(BenchmarkError::Stop("missing benchmark utxo id"))?;
	Ok(LockBenchmarkContext { owner, operator, utxo_id, satoshis })
}

fn create_funded_lock<T>(seed_hint: u8) -> Result<LockBenchmarkContext<T>, BenchmarkError>
where
	T: Config,
	T::AccountId: Ord,
{
	let context = create_unfunded_lock::<T>(seed_hint)?;
	let funding_ref = benchmark_utxo_ref(10_000u32.saturating_add(seed_hint as u32));
	<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::utxo_detected(
		context.utxo_id,
		funding_ref,
		context.satoshis,
		T::BitcoinBlockHeightChange::get().1,
	)
	.map_err(|_| BenchmarkError::Stop("failed to fund benchmark lock"))?;
	Ok(context)
}

fn seed_orphan<T>(
	context: &LockBenchmarkContext<T>,
	utxo_ref: UtxoRef,
) -> Result<(), BenchmarkError>
where
	T: Config,
{
	Pallet::<T>::orphaned_utxo_detected(context.utxo_id, context.satoshis, utxo_ref)
		.map_err(|_| BenchmarkError::Stop("failed to seed orphan"))
}

fn seed_orphan_with_request<T>(
	context: &LockBenchmarkContext<T>,
	utxo_ref: UtxoRef,
) -> Result<(), BenchmarkError>
where
	T: Config,
{
	seed_orphan::<T>(context, utxo_ref.clone())?;
	Pallet::<T>::request_orphaned_utxo_release(
		frame_system::RawOrigin::Signed(context.owner.clone()).into(),
		utxo_ref,
		benchmark_script_pubkey(9)?,
		1_000,
	)
	.map_err(|_| BenchmarkError::Stop("failed to seed orphan release request"))
}

fn seed_overdue_release_request<T>(
	context: &LockBenchmarkContext<T>,
	current_frame: FrameId,
) -> Result<(), BenchmarkError>
where
	T: Config,
{
	Pallet::<T>::request_release(
		frame_system::RawOrigin::Signed(context.owner.clone()).into(),
		context.utxo_id,
		benchmark_script_pubkey(10)?,
		1_000,
	)
	.map_err(|_| BenchmarkError::Stop("failed to seed release request"))?;
	let mut request = LockReleaseRequestsByUtxoId::<T>::get(context.utxo_id)
		.ok_or(BenchmarkError::Stop("missing seeded release request"))?;
	LockCosignDueByFrame::<T>::mutate(request.cosign_due_frame, |entries| {
		entries.remove(&context.utxo_id);
	});
	request.cosign_due_frame = current_frame;
	LockReleaseRequestsByUtxoId::<T>::insert(context.utxo_id, request);
	LockCosignDueByFrame::<T>::try_mutate(current_frame, |entries| {
		entries
			.try_insert(context.utxo_id)
			.map_err(|_| BenchmarkError::Stop("overdue cosign set overflow"))
	})?;
	Ok(())
}

fn seed_migrated_release_hold<T>(context: &LockBenchmarkContext<T>) -> Result<(), BenchmarkError>
where
	T: Config,
{
	let release_hold = LockReleaseRequestsByUtxoId::<T>::get(context.utxo_id)
		.ok_or(BenchmarkError::Stop("missing seeded release request"))?
		.securitization_at_risk;
	let owner_balance = T::Currency::minimum_balance().saturating_add(release_hold);
	T::Currency::mint_into(&context.owner, owner_balance)
		.map_err(|_| BenchmarkError::Stop("failed to seed release hold balance"))?;
	T::Currency::hold(&HoldReason::ReleaseBitcoinLock.into(), &context.owner, release_hold)
		.map_err(|_| BenchmarkError::Stop("failed to seed migrated release hold"))?;
	frame_system::Pallet::<T>::inc_providers(&context.owner);
	MigratedReleaseHoldByUtxoId::<T>::insert(context.utxo_id, release_hold);
	Ok(())
}
