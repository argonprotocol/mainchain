#![cfg(feature = "runtime-benchmarks")]

extern crate alloc;

use super::*;
use alloc::vec::Vec;
use argon_bitcoin::{derive_xpub, xpriv_from_seed};
use argon_primitives::{
	bitcoin::{
		BitcoinHeight, BitcoinLockId, BitcoinNetwork, BitcoinScriptPubkey, BitcoinSignature,
		BitcoinXPub, CompressedBitcoinPubkey, H256Le, OpaqueBitcoinXpub, Satoshis, UtxoRef,
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
const SECURITIZATION_HOLD_EXPIRATIONS_BENCH_RANGE_END: u32 = 20;
const MAX_UTXOS_PER_LOCK_BENCH: u32 = 100;

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

		let lock_id =
			NextBitcoinLockId::<T>::get().ok_or(BenchmarkError::Stop("missing utxo id"))?;
		assert!(LocksById::<T>::contains_key(lock_id));
		Ok(())
	}

	#[benchmark]
	fn request_release() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(3)?;
		let input_count = T::MaxUtxosPerLock::get();
		fund_lock_with_utxos::<T>(&context, input_count, 10_000)?;
		MinimumSatoshis::<T>::put(1);
		let release_script_pubkey = benchmark_script_pubkey(1)?;
		let bitcoin_network_fee: Satoshis = 1_000;
		let destination_satoshis = context.satoshis / 2;
		let owner = context.owner.clone();
		whitelist_account!(owner);

		#[extrinsic_call]
		_(
			RawOrigin::Signed(owner),
			context.lock_id,
			release_script_pubkey.clone(),
			destination_satoshis,
			bitcoin_network_fee,
		);

		let request = LockReleaseRequestsById::<T>::get(context.lock_id)
			.ok_or(BenchmarkError::Stop("missing release request"))?;
		assert_eq!(request.bitcoin_network_fee, bitcoin_network_fee);
		assert_eq!(request.destination_satoshis, destination_satoshis);
		assert_eq!(request.to_script_pubkey, release_script_pubkey);
		Ok(())
	}

	#[benchmark]
	fn cosign_release(i: Linear<1, MAX_UTXOS_PER_LOCK_BENCH>) -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		if i > T::MaxUtxosPerLock::get() {
			return Err(BenchmarkError::Stop("benchmark exceeds MaxUtxosPerLock"))
		}
		let context = create_unfunded_lock::<T>(4)?;
		fund_lock_with_utxos::<T>(&context, i, 20_000)?;
		let release_script_pubkey = benchmark_script_pubkey(2)?;
		let signature = benchmark_signature()?;
		let signatures = BoundedVec::<_, T::MaxUtxosPerLock>::try_from(vec![signature; i as usize])
			.map_err(|_| BenchmarkError::Stop("signature count exceeds MaxUtxosPerLock"))?;
		Pallet::<T>::request_release(
			RawOrigin::Signed(context.owner.clone()).into(),
			context.lock_id,
			release_script_pubkey,
			context.satoshis.saturating_sub(1_000),
			1_000,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed release request"))?;
		let operator = context.operator.clone();
		whitelist_account!(operator);

		#[extrinsic_call]
		_(RawOrigin::Signed(operator), context.lock_id, signatures);

		assert!(!LocksById::<T>::contains_key(context.lock_id));
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
		let mut expiring_lock_ids = Vec::new();

		for index in 0..e {
			let context = create_funded_lock::<T>(10u8.saturating_add(index as u8))?;
			expiring_lock_ids.push(context.lock_id);
		}

		#[block]
		{
			let _ = Pallet::<T>::process_expiring_locks(expiring_lock_ids);
		}

		Ok(())
	}

	#[benchmark]
	fn on_initialize_overdue_releases(
		o: Linear<1, OVERDUE_RELEASES_BENCH_RANGE_END>,
	) -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let current_frame = T::CurrentFrameId::get();
		let mut overdue_lock_ids = Vec::new();

		for index in 0..o {
			let context = create_funded_lock::<T>(40u8.saturating_add(index as u8))?;
			seed_overdue_release_request::<T>(&context, current_frame)?;
			overdue_lock_ids.push(context.lock_id);
		}

		#[block]
		{
			let _ = Pallet::<T>::process_overdue_releases(overdue_lock_ids);
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
	fn on_initialize_securitization_hold_expirations(
		p: Linear<1, SECURITIZATION_HOLD_EXPIRATIONS_BENCH_RANGE_END>,
	) -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let mut expiring_lock_ids = Vec::new();

		for index in 0..p {
			let context = create_unfunded_lock::<T>(100u8.saturating_add(index as u8))?;
			expiring_lock_ids.push(context.lock_id);
		}

		#[block]
		{
			let _ = Pallet::<T>::process_securitization_hold_expirations(
				expiring_lock_ids,
				T::BitcoinBlockHeightChange::get().1,
			);
		}

		Ok(())
	}

	#[benchmark]
	fn admin_modify_minimum_locked_sats() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let new_minimum = benchmark_satoshis::<T>().saturating_add(1_000);
		LockReleaseRequestsById::<T>::insert(
			1,
			LockReleaseRequest {
				lock_id: 1,
				vault_id: 1,
				release_number: 1,
				bitcoin_network_fee: 1_000,
				destination_satoshis: new_minimum,
				change_satoshis: new_minimum,
				cosign_due_frame: T::CurrentFrameId::get(),
				to_script_pubkey: benchmark_script_pubkey(20)?,
				expected_transaction_id: H256Le([20; 32]),
				securitization_at_risk: <T as Config>::Balance::zero(),
			},
		);
		PendingPartialReleaseByLockId::<T>::insert(
			2,
			PendingPartialRelease {
				release_number: 1,
				expected_change_utxo_ref: benchmark_utxo_ref(20),
				change_satoshis: new_minimum,
			},
		);

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
		let lock = LocksById::<T>::get(context.lock_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		let securitized_satoshis = context.satoshis.saturating_add(10_000);
		let microgons_at_target_per_btc = lock.securitization_basis.microgons_at_target_per_btc;
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
			context.lock_id,
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

		let lock = LocksById::<T>::get(context.lock_id)
			.ok_or(BenchmarkError::Stop("missing lock after resecuritization"))?;
		assert_eq!(lock.securitization_basis.satoshis, securitized_satoshis);
		Ok(())
	}

	#[benchmark]
	fn set_flexible() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(17)?;
		let mut lock = LocksById::<T>::get(context.lock_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		LockIdsByOwnerAccount::<T>::remove(&context.owner, context.lock_id);
		lock.owner_account = context.operator.clone();
		LocksById::<T>::insert(context.lock_id, lock);
		LockIdsByOwnerAccount::<T>::insert(&context.operator, context.lock_id, ());
		let operator = context.operator;
		whitelist_account!(operator);

		#[extrinsic_call]
		_(RawOrigin::Signed(operator), context.lock_id, true);

		assert!(
			LocksById::<T>::get(context.lock_id)
				.ok_or(BenchmarkError::Stop("missing flexible lock"))?
				.is_flexible
		);
		Ok(())
	}

	#[benchmark]
	fn provider_fission_satoshis() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(18)?;
		let lock = LocksById::<T>::get(context.lock_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		let fissioned_satoshis = context.satoshis / 2;
		seed_microgons_at_target_per_btc_history::<T>(
			lock.securitization_basis.microgons_at_target_per_btc,
		)?;

		#[block]
		{
			<Pallet<T> as BitcoinFissionLockProvider<
				T::AccountId,
				<T as Config>::Balance,
			>>::fission_satoshis(
				&context.owner,
				context.lock_id,
				fissioned_satoshis,
				lock.securitization_basis.microgons_at_target_per_btc,
			)
			.map_err(|_| BenchmarkError::Stop("Fission allocation failed"))?;
		}

		assert_eq!(
			LocksById::<T>::get(context.lock_id)
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
		let lock = LocksById::<T>::get(context.lock_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		let fissioned_satoshis = context.satoshis / 2;
		seed_microgons_at_target_per_btc_history::<T>(
			lock.securitization_basis.microgons_at_target_per_btc,
		)?;
		let (liquidity_promised, last_ratchet_tick) = <Pallet<T> as BitcoinFissionLockProvider<
			T::AccountId,
			<T as Config>::Balance,
		>>::fission_satoshis(
			&context.owner,
			context.lock_id,
			fissioned_satoshis,
			lock.securitization_basis.microgons_at_target_per_btc,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed Fission allocation"))?;
		pallet_bitcoin_fissions::FissionByOwnerAndId::<T>::insert(
			&context.owner,
			0,
			pallet_bitcoin_fissions::Fission {
				liquid_id: 0,
				lock_id: context.lock_id,
				satoshis: fissioned_satoshis,
				microgons_at_target_per_btc: lock.securitization_basis.microgons_at_target_per_btc,
				last_ratchet_tick,
				liquidity_promised,
				created_at_argon_block: frame_system::Pallet::<T>::block_number(),
				ratchet_number: 0,
				last_updated_argon_block: frame_system::Pallet::<T>::block_number(),
			},
		);
		pallet_bitcoin_fissions::FissionIdsByLockId::<T>::try_mutate(
			context.lock_id,
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
				context.lock_id,
				fissioned_satoshis,
				lock.securitization_basis.microgons_at_target_per_btc,
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
		let lock = LocksById::<T>::get(context.lock_id)
			.ok_or(BenchmarkError::Stop("missing benchmark lock"))?;
		let fissioned_satoshis = context.satoshis / 2;
		seed_microgons_at_target_per_btc_history::<T>(
			lock.securitization_basis.microgons_at_target_per_btc,
		)?;
		<Pallet<T> as BitcoinFissionLockProvider<
			T::AccountId,
			<T as Config>::Balance,
		>>::fission_satoshis(
			&context.owner,
			context.lock_id,
			fissioned_satoshis,
			lock.securitization_basis.microgons_at_target_per_btc,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed Fission allocation"))?;

		#[block]
		{
			<Pallet<T> as BitcoinFissionLockProvider<
				T::AccountId,
				<T as Config>::Balance,
			>>::fuse_satoshis(
				&context.owner,
				context.lock_id,
				fissioned_satoshis,
				lock.securitization_basis.microgons_at_target_per_btc,
			)
			.map_err(|_| BenchmarkError::Stop("Fission deallocation failed"))?;
		}

		assert_eq!(
			LocksById::<T>::get(context.lock_id)
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
		let input_count = T::MaxUtxosPerLock::get();
		fund_lock_with_utxos::<T>(&context, input_count, 30_000)?;
		MinimumSatoshis::<T>::put(1);
		let release_script_pubkey = benchmark_script_pubkey(11)?;
		let bitcoin_network_fee = 1_000;
		let destination_satoshis = context.satoshis / 2;
		Pallet::<T>::request_release(
			RawOrigin::Signed(context.owner.clone()).into(),
			context.lock_id,
			release_script_pubkey,
			destination_satoshis,
			bitcoin_network_fee,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed partial release request"))?;
		let request = LockReleaseRequestsById::<T>::get(context.lock_id)
			.ok_or(BenchmarkError::Stop("missing release request"))?;
		let signatures = BoundedVec::<_, T::MaxUtxosPerLock>::try_from(vec![
			benchmark_signature()?;
			input_count as usize
		])
		.map_err(|_| BenchmarkError::Stop("signature count exceeds MaxUtxosPerLock"))?;
		Pallet::<T>::cosign_release(
			RawOrigin::Signed(context.operator.clone()).into(),
			context.lock_id,
			signatures,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed partial release cosign"))?;
		let change_ref = UtxoRef { txid: request.expected_transaction_id.clone(), output_index: 1 };
		PendingPartialReleaseByLockId::<T>::insert(
			context.lock_id,
			PendingPartialRelease {
				release_number: request.release_number,
				expected_change_utxo_ref: change_ref.clone(),
				change_satoshis: request.change_satoshis,
			},
		);
		let expiration_height = LocksById::<T>::get(context.lock_id)
			.ok_or(BenchmarkError::Stop("missing partial release Lock"))?
			.vault_claim_height;
		seed_bitcoin_heights(expiration_height, expiration_height);

		#[block]
		{
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::utxo_detected(
				context.lock_id,
				change_ref,
				request.change_satoshis,
				expiration_height,
			)
			.map_err(|_| BenchmarkError::Stop("utxo_detected failed"))?;
		}

		assert!(!LocksById::<T>::contains_key(context.lock_id));
		assert!(!LockReleaseRequestsById::<T>::contains_key(context.lock_id));
		assert!(!PendingPartialReleaseByLockId::<T>::contains_key(context.lock_id));
		Ok(())
	}

	#[benchmark]
	fn provider_spent() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(16)?;
		MinimumSatoshis::<T>::put(1);
		let orphan_ref = benchmark_utxo_ref(1_104);
		let release_script_pubkey = benchmark_script_pubkey(11)?;
		seed_orphan_with_request::<T>(&context, orphan_ref)?;
		Pallet::<T>::request_release(
			RawOrigin::Signed(context.owner.clone()).into(),
			context.lock_id,
			release_script_pubkey,
			(context.satoshis / 2).saturating_sub(1_000),
			1_000,
		)
		.map_err(|_| BenchmarkError::Stop("failed to seed release request"))?;
		#[block]
		{
			let funding_ref = LocksById::<T>::get(context.lock_id)
				.and_then(|lock| lock.funding_utxos.keys().next().cloned())
				.ok_or(BenchmarkError::Stop("missing funding UTXO ref"))?;
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::spent(
				context.lock_id,
				funding_ref,
				T::BitcoinBlockHeightChange::get().1,
			)
			.map_err(|_| BenchmarkError::Stop("spent failed"))?;
		}

		assert!(!LocksById::<T>::contains_key(context.lock_id));
		assert!(!LockReleaseRequestsById::<T>::contains_key(context.lock_id));
		Ok(())
	}
}

struct LockBenchmarkContext<T: Config> {
	owner: T::AccountId,
	operator: T::AccountId,
	lock_id: BitcoinLockId,
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
	NextBitcoinLockId::<T>::kill();
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
		total_satoshis: 0,
		securitized_satoshis: 0,
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
	let lock_id =
		NextBitcoinLockId::<T>::get().ok_or(BenchmarkError::Stop("missing benchmark utxo id"))?;
	Ok(LockBenchmarkContext { owner, operator, lock_id, satoshis })
}

fn create_funded_lock<T>(seed_hint: u8) -> Result<LockBenchmarkContext<T>, BenchmarkError>
where
	T: Config,
	T::AccountId: Ord,
{
	let context = create_unfunded_lock::<T>(seed_hint)?;
	fund_lock_with_utxos::<T>(&context, 1, 10_000u32.saturating_add(seed_hint as u32))?;
	Ok(context)
}

fn fund_lock_with_utxos<T>(
	context: &LockBenchmarkContext<T>,
	input_count: u32,
	first_seed: u32,
) -> Result<(), BenchmarkError>
where
	T: Config,
{
	if input_count == 0 || input_count > T::MaxUtxosPerLock::get() {
		return Err(BenchmarkError::Stop("invalid benchmark UTXO count"))
	}
	let satoshis_per_utxo = context.satoshis / input_count as u64;
	for index in 0..input_count {
		let satoshis = if index + 1 == input_count {
			context.satoshis.saturating_sub(satoshis_per_utxo.saturating_mul(index as u64))
		} else {
			satoshis_per_utxo
		};
		<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::utxo_detected(
			context.lock_id,
			benchmark_utxo_ref(first_seed.saturating_add(index)),
			satoshis,
			T::BitcoinBlockHeightChange::get().1,
		)
		.map_err(|_| BenchmarkError::Stop("failed to fund benchmark lock"))?;
	}
	Ok(())
}

fn seed_orphan<T>(
	context: &LockBenchmarkContext<T>,
	utxo_ref: UtxoRef,
) -> Result<(), BenchmarkError>
where
	T: Config,
{
	Pallet::<T>::orphaned_utxo_detected(context.lock_id, context.satoshis, utxo_ref)
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
		context.lock_id,
		benchmark_script_pubkey(10)?,
		context.satoshis.saturating_sub(1_000),
		1_000,
	)
	.map_err(|_| BenchmarkError::Stop("failed to seed release request"))?;
	let mut request = LockReleaseRequestsById::<T>::get(context.lock_id)
		.ok_or(BenchmarkError::Stop("missing seeded release request"))?;
	let cosign_due_frame = request.cosign_due_frame;
	LockCosignDueByFrame::<T>::mutate(cosign_due_frame, |entries| {
		entries.remove(&context.lock_id);
	});
	request.cosign_due_frame = current_frame;
	LockReleaseRequestsById::<T>::insert(context.lock_id, request);
	LockCosignDueByFrame::<T>::try_mutate(current_frame, |entries| {
		entries
			.try_insert(context.lock_id)
			.map_err(|_| BenchmarkError::Stop("overdue cosign set overflow"))
	})?;
	Ok(())
}
