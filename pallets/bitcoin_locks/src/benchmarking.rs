#![cfg(feature = "runtime-benchmarks")]

extern crate alloc;

use super::*;
use alloc::vec::Vec;
use argon_bitcoin::{derive_xpub, xpriv_from_seed};
use argon_primitives::{
	BitcoinUtxoEvents, PriceProvider,
	bitcoin::{
		BitcoinHeight, BitcoinNetwork, BitcoinScriptPubkey, BitcoinSignature, BitcoinXPub,
		CompressedBitcoinPubkey, H256Le, OpaqueBitcoinXpub, SATOSHIS_PER_BITCOIN, Satoshis, UtxoId,
		UtxoRef,
	},
	vault::{BitcoinVaultProvider, Vault, VaultTerms},
};
use frame_benchmarking::v2::*;
use pallet_prelude::benchmarking::{
	BenchmarkBitcoinLocksRuntimeState, BenchmarkBitcoinUtxoTrackerState,
	BenchmarkPriceProviderState, benchmark_bitcoin_utxo_tracker_state,
	benchmark_bitcoin_vault_provider_state, reset_benchmark_bitcoin_locks_runtime_state,
	reset_benchmark_bitcoin_utxo_tracker_state, reset_benchmark_bitcoin_vault_provider_state,
	reset_benchmark_price_provider_state, reset_benchmark_utxo_lock_events_state,
	set_benchmark_bitcoin_locks_runtime_state, set_benchmark_bitcoin_utxo_tracker_state,
	set_benchmark_bitcoin_vault_provider_state, set_benchmark_price_provider_state,
};

// Small linear fit ranges used to generate the per-item slope for hook weight components.
const EXPIRING_LOCKS_BENCH_RANGE_END: u32 = 20;
const OVERDUE_RELEASES_BENCH_RANGE_END: u32 = 20;
const ORPHAN_EXPIRATIONS_BENCH_RANGE_END: u32 = 20;

#[benchmarks(where <T as frame_system::Config>::AccountId: Ord)]
mod benchmarks {
	use super::*;
	use frame_support::traits::Hooks;
	use frame_system::RawOrigin;

	#[benchmark]
	fn initialize() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let owner: T::AccountId = account("bitcoin-lock-owner", 0, 0);
		let operator: T::AccountId = account("vault-operator", 0, 0);
		let satoshis = benchmark_satoshis::<T>();
		let owner_pubkey = benchmark_pubkey::<T>(1)?;
		seed_price_state(100_000, 1, 1);
		let vault_id = create_vault::<T>(&operator, 1, benchmark_vault_securitization())?;
		let options = benchmark_lock_options::<T>(vault_id, &owner, satoshis, 11)?;
		whitelist_account!(owner);

		#[extrinsic_call]
		_(RawOrigin::Signed(owner.clone()), vault_id, satoshis, owner_pubkey, options);

		let utxo_id = NextUtxoId::<T>::get().ok_or(BenchmarkError::Stop("missing utxo id"))?;
		assert!(LocksByUtxoId::<T>::contains_key(utxo_id));
		Ok(())
	}

	#[benchmark]
	fn initialize_for() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let delegate: T::AccountId = account("bitcoin-lock-delegate", 0, 0);
		let beneficiary: T::AccountId = account("bitcoin-lock-beneficiary", 0, 0);
		let operator: T::AccountId = account("vault-operator", 0, 0);
		let satoshis = benchmark_satoshis::<T>();
		let owner_pubkey = benchmark_pubkey::<T>(2)?;
		seed_price_state(100_000, 1, 1);
		let vault_id = create_vault::<T>(&operator, 2, benchmark_vault_securitization())?;
		let mut state = benchmark_bitcoin_vault_provider_state::<T::AccountId, T::Balance>();
		state
			.vaults
			.get_mut(&vault_id)
			.ok_or(BenchmarkError::Stop("missing benchmark vault"))?
			.bitcoin_lock_delegate_account = Some(delegate.clone());
		set_benchmark_bitcoin_vault_provider_state(state);
		let options = benchmark_lock_options::<T>(vault_id, &beneficiary, satoshis, 12)?;
		whitelist_account!(delegate);
		whitelist_account!(beneficiary);

		#[extrinsic_call]
		_(
			RawOrigin::Signed(delegate),
			beneficiary.clone(),
			vault_id,
			satoshis,
			owner_pubkey,
			options,
		);

		let utxo_id = NextUtxoId::<T>::get().ok_or(BenchmarkError::Stop("missing utxo id"))?;
		let lock = LocksByUtxoId::<T>::get(utxo_id)
			.ok_or(BenchmarkError::Stop("missing initialized lock"))?;
		assert_eq!(lock.owner_account, beneficiary);
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
		let operator = context.operator.clone();
		whitelist_account!(operator);

		#[extrinsic_call]
		_(RawOrigin::Signed(operator), context.utxo_id, signature);

		assert!(!LocksByUtxoId::<T>::contains_key(context.utxo_id));
		Ok(())
	}

	#[benchmark]
	fn ratchet() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_funded_lock::<T>(5)?;
		seed_price_state(80_000, 1, 1);
		let options =
			benchmark_lock_options::<T>(context.vault_id, &context.owner, context.satoshis, 13)?;
		let owner = context.owner.clone();
		whitelist_account!(owner);

		#[extrinsic_call]
		_(RawOrigin::Signed(owner), context.utxo_id, options);

		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing lock after ratchet"))?;
		assert!(lock.is_funded);
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
		seed_candidate(context.utxo_id, orphan_ref.clone(), context.satoshis);
		let signature = benchmark_signature()?;
		let operator = context.operator.clone();
		whitelist_account!(operator);

		#[extrinsic_call]
		_(RawOrigin::Signed(operator), context.owner.clone(), orphan_ref.clone(), signature);

		assert!(!OrphanedUtxosByAccount::<T>::contains_key(&context.owner, &orphan_ref));
		Ok(())
	}

	#[benchmark]
	fn increase_securitization() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(9)?;
		let new_satoshis = context.satoshis.saturating_add(10_000);
		let owner = context.owner.clone();
		whitelist_account!(owner);

		#[extrinsic_call]
		_(RawOrigin::Signed(owner), context.utxo_id, new_satoshis);

		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing lock after increase"))?;
		assert_eq!(lock.satoshis, new_satoshis);
		Ok(())
	}

	#[benchmark]
	fn provider_funding_received() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(11)?;
		let received_satoshis = context.satoshis.saturating_sub(1_000);

		#[block]
		{
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::funding_received(
				context.utxo_id,
				received_satoshis,
			)
			.map_err(|_| BenchmarkError::Stop("funding_received failed"))?;
		}

		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing funded lock"))?;
		assert!(lock.is_funded);
		assert_eq!(lock.satoshis, received_satoshis);
		Ok(())
	}

	#[benchmark]
	fn provider_timeout_waiting_for_funding() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(12)?;
		seed_orphan::<T>(&context, benchmark_utxo_ref(1_100))?;

		#[block]
		{
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::timeout_waiting_for_funding(
				context.utxo_id,
			)
			.map_err(|_| BenchmarkError::Stop("timeout_waiting_for_funding failed"))?;
		}

		assert!(!LocksByUtxoId::<T>::contains_key(context.utxo_id));
		Ok(())
	}

	#[benchmark]
	fn provider_funding_promoted_by_account() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(13)?;
		let orphan_ref = benchmark_utxo_ref(1_101);
		let received_satoshis = context.satoshis.saturating_sub(1_000);
		seed_orphan::<T>(&context, orphan_ref.clone())?;

		#[block]
		{
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::funding_promoted_by_account(
				context.utxo_id,
				received_satoshis,
				&context.owner,
				&orphan_ref,
			)
			.map_err(|_| BenchmarkError::Stop("funding_promoted_by_account failed"))?;
		}

		let lock = LocksByUtxoId::<T>::get(context.utxo_id)
			.ok_or(BenchmarkError::Stop("missing promoted lock"))?;
		assert!(lock.is_funded);
		assert!(!OrphanedUtxosByAccount::<T>::contains_key(&context.owner, &orphan_ref));
		Ok(())
	}

	#[benchmark]
	fn provider_candidate_rejected_by_account() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(14)?;
		let orphan_ref = benchmark_utxo_ref(1_102);

		#[block]
		{
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::candidate_rejected_by_account(
				context.utxo_id,
				context.satoshis,
				&context.owner,
				&orphan_ref,
			)
			.map_err(|_| BenchmarkError::Stop("candidate_rejected_by_account failed"))?;
		}

		assert!(OrphanedUtxosByAccount::<T>::contains_key(&context.owner, &orphan_ref));
		Ok(())
	}

	#[benchmark]
	fn provider_orphaned_utxo_detected() -> Result<(), BenchmarkError> {
		reset_benchmark_environment::<T>();
		let context = create_unfunded_lock::<T>(15)?;
		let orphan_ref = benchmark_utxo_ref(1_103);

		#[block]
		{
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::orphaned_utxo_detected(
				context.utxo_id,
				context.satoshis,
				orphan_ref.clone(),
			)
			.map_err(|_| BenchmarkError::Stop("orphaned_utxo_detected failed"))?;
		}

		assert!(OrphanedUtxosByAccount::<T>::contains_key(&context.owner, &orphan_ref));
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

		#[block]
		{
			<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::spent(context.utxo_id)
				.map_err(|_| BenchmarkError::Stop("spent failed"))?;
		}

		assert!(!LocksByUtxoId::<T>::contains_key(context.utxo_id));
		Ok(())
	}
}

struct LockBenchmarkContext<T: Config> {
	owner: T::AccountId,
	operator: T::AccountId,
	vault_id: VaultId,
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
	reset_benchmark_utxo_lock_events_state();
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
	MicrogonPerBtcHistory::<T>::kill();
}

fn benchmark_satoshis<T: Config>() -> Satoshis {
	100_000_000
}

fn benchmark_vault_securitization() -> u128 {
	200_000_000_000
}

fn benchmark_microgons_per_btc<T: Config>() -> Result<T::Balance, BenchmarkError> {
	T::PriceProvider::get_bitcoin_argon_price(SATOSHIS_PER_BITCOIN)
		.ok_or(BenchmarkError::Stop("benchmark bitcoin price should be available"))
}

fn seed_microgons_per_btc_history<T: Config>(
	microgons_per_btc: T::Balance,
) -> Result<(), BenchmarkError> {
	let history = BoundedVec::try_from(vec![(T::CurrentTick::get(), microgons_per_btc)])
		.map_err(|_| BenchmarkError::Stop("benchmark microgons per btc history overflow"))?;
	MicrogonPerBtcHistory::<T>::put(history);
	Ok(())
}

fn benchmark_lock_options<T: Config>(
	_vault_id: VaultId,
	_account_id: &T::AccountId,
	_max_satoshis: Satoshis,
	_seed_hint: u8,
) -> Result<Option<LockOptions<T>>, BenchmarkError> {
	let microgons_per_btc = benchmark_microgons_per_btc::<T>()?;
	seed_microgons_per_btc_history::<T>(microgons_per_btc)?;
	Ok(Some(LockOptions::V1 { microgons_per_btc: Some(microgons_per_btc) }))
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
		bitcoin_lock_delegate_account: None,
		securitization: securitization.into(),
		securitization_target: securitization.into(),
		securitization_locked: T::Balance::zero(),
		securitization_pending_activation: T::Balance::zero(),
		locked_satoshis: 0,
		securitized_satoshis: 0,
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
	T::Currency::mint_into(&owner, 1_000_000_000_000u128.into())
		.map_err(|_| BenchmarkError::Stop("failed to fund benchmark lock owner"))?;
	let vault_id = create_vault::<T>(&operator, seed_hint, benchmark_vault_securitization())?;
	Pallet::<T>::initialize(
		frame_system::RawOrigin::Signed(owner.clone()).into(),
		vault_id,
		satoshis,
		owner_pubkey,
		None,
	)
	.map_err(|_| BenchmarkError::Stop("failed to create benchmark lock"))?;
	let utxo_id =
		NextUtxoId::<T>::get().ok_or(BenchmarkError::Stop("missing benchmark utxo id"))?;
	Ok(LockBenchmarkContext { owner, operator, vault_id, utxo_id, satoshis })
}

fn create_funded_lock<T>(seed_hint: u8) -> Result<LockBenchmarkContext<T>, BenchmarkError>
where
	T: Config,
	T::AccountId: Ord,
{
	let context = create_unfunded_lock::<T>(seed_hint)?;
	let mut tracker_state = benchmark_bitcoin_utxo_tracker_state();
	tracker_state
		.funding_utxo_refs_by_id
		.insert(context.utxo_id, benchmark_utxo_ref(10_000u32.saturating_add(seed_hint as u32)));
	set_benchmark_bitcoin_utxo_tracker_state(tracker_state);
	<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::funding_received(
		context.utxo_id,
		context.satoshis,
	)
	.map_err(|_| BenchmarkError::Stop("failed to fund benchmark lock"))?;
	Ok(context)
}

fn seed_candidate(utxo_id: UtxoId, utxo_ref: UtxoRef, satoshis: Satoshis) {
	let mut state = benchmark_bitcoin_utxo_tracker_state();
	state.candidate_utxos_by_ref.insert(utxo_ref, (utxo_id, satoshis));
	set_benchmark_bitcoin_utxo_tracker_state(state);
}

fn seed_orphan<T>(
	context: &LockBenchmarkContext<T>,
	utxo_ref: UtxoRef,
) -> Result<(), BenchmarkError>
where
	T: Config,
{
	<Pallet<T> as BitcoinUtxoEvents<T::AccountId>>::orphaned_utxo_detected(
		context.utxo_id,
		context.satoshis,
		utxo_ref,
	)
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
