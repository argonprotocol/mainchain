#![allow(clippy::inconsistent_digit_grouping)]
use std::collections::{BTreeMap, BTreeSet};

use bitcoin::PublicKey;
use pallet_prelude::*;

use crate as pallet_bitcoin_locks;
use crate::BitcoinVerifier;
use argon_bitcoin::CosignReleaser;
use argon_primitives::{
	BitcoinUtxoTracker, PriceProvider, UtxoLockEvents,
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinNetwork, BitcoinSignature, BitcoinXPub,
		CompressedBitcoinPubkey, NetworkKind, Satoshis, UtxoId, UtxoRef,
	},
	ensure,
	vault::{BitcoinVaultProvider, LockExtension, Vault, VaultError, VaultTerms},
};
use frame_support::traits::Currency;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		BitcoinLocks: pallet_bitcoin_locks
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub static ExistentialDeposit: Balance = 10;
	pub const BlocksPerYear:u32 = 1440*365;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

pub fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

parameter_types! {
	pub static MaxConcurrentlyReleasingLocks: u32 = 10;
	pub static BitcoinPriceInUsd: Option<FixedU128> = Some(FixedU128::from_rational(62_000_00, 100));
	pub static ArgonPriceInUsd: Option<FixedU128> = Some(FixedU128::from_rational(100, 100));
	pub static ArgonCPI: Option<argon_primitives::ArgonCPI> = Some(FixedI128::from_float(0.1));
	pub static ArgonTargetPriceInUsd: Option<FixedU128> = Some(FixedU128::from_rational(100, 100));
	pub static LockReleaseCosignDeadlineFrames: FrameId = 5;
	pub static LockReclamationBlocks: BitcoinHeight = 30;
	pub static LockDurationBlocks: BitcoinHeight = 144 * 365;
	pub static BitcoinBlockHeightChange: (BitcoinHeight, BitcoinHeight) = (0, 0);
	pub static MinimumLockSatoshis: Satoshis = 10_000_000;
	pub static DefaultVault: Vault<u64, Balance> = Vault {
		securitization:  200_000_000_000,
		argons_locked: 0,
		terms: VaultTerms {
			bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
			bitcoin_base_fee: 0,
			liquidity_pool_profit_sharing: Permill::from_float(0.0),
		},
		opened_tick: 1,
		operator_account_id: 1,
		securitization_ratio: FixedU128::from_float(1.0),
		argons_scheduled_for_release: BoundedBTreeMap::new(),
		is_closed: false,
		pending_terms: None,
		argons_pending_activation: 0,
	};

	pub static NextUtxoId: UtxoId = 1;
	pub static WatchedUtxosById: BTreeMap<UtxoId, (BitcoinCosignScriptPubkey, Satoshis, BitcoinHeight)> = BTreeMap::new();

	pub static GetUtxoRef: Option<UtxoRef> = None;

	pub static LastLockEvent: Option<(UtxoId, u64, Balance)> = None;
	pub static LastReleaseEvent: Option<(UtxoId, bool, Balance)> = None;

	pub static GetBitcoinNetwork: BitcoinNetwork = BitcoinNetwork::Regtest;

	pub static DefaultVaultBitcoinPubkey: PublicKey = "02e3af28965693b9ce1228f9d468149b831d6a0540b25e8a9900f71372c11fb277".parse::<PublicKey>().unwrap();
	pub static DefaultVaultReclaimBitcoinPubkey: PublicKey = "026c468be64d22761c30cd2f12cbc7de255d592d7904b1bab07236897cc4c2e766".parse::<PublicKey>().unwrap();

	pub static CurrentFrameId: FrameId = 1;

	pub static CanceledLocks: Vec<(VaultId, Balance)> = Vec::new();

	pub static ChargeFee: bool = false;

	pub static VaultViewOfCosignPendingLocks: BTreeMap<VaultId,  BTreeSet<UtxoId>> = BTreeMap::new();

	pub const TicksPerBitcoinBlock: u64 = 10;
	pub const ArgonTicksPerDay: u64 = 1440;
}

pub struct EventHandler;
impl UtxoLockEvents<u64, Balance> for EventHandler {
	fn utxo_locked(
		utxo_id: UtxoId,
		account_id: &u64,
		amount: Balance,
	) -> Result<(), DispatchError> {
		LastLockEvent::set(Some((utxo_id, *account_id, amount)));
		Ok(())
	}
	fn utxo_released(
		utxo_id: UtxoId,
		remove_pending_mints: bool,
		amount_burned: Balance,
	) -> DispatchResult {
		LastReleaseEvent::set(Some((utxo_id, remove_pending_mints, amount_burned)));

		Ok(())
	}
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		BitcoinPriceInUsd::get()
	}
	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		ArgonPriceInUsd::get()
	}
	fn get_argon_cpi() -> Option<argon_primitives::ArgonCPI> {
		ArgonCPI::get()
	}
	fn get_argon_pool_liquidity() -> Option<Balance> {
		todo!()
	}
	fn get_redemption_r_value() -> Option<FixedU128> {
		ArgonPriceInUsd::get()?.checked_div(&ArgonTargetPriceInUsd::get().unwrap())
	}
}

pub struct StaticVaultProvider;

impl BitcoinVaultProvider for StaticVaultProvider {
	type Balance = Balance;
	type AccountId = u64;

	fn is_owner(vault_id: VaultId, account_id: &Self::AccountId) -> bool {
		if vault_id == 1 {
			return DefaultVault::get().operator_account_id == *account_id;
		}
		false
	}

	fn cancel(vault_id: VaultId, amount: Self::Balance) -> Result<(), VaultError> {
		DefaultVault::mutate(|v| {
			v.release_locked_funds(amount);
		});
		CanceledLocks::mutate(|a| a.push((vault_id, amount)));
		Ok(())
	}

	fn lock(
		_vault_id: VaultId,
		locker: &Self::AccountId,
		amount: Self::Balance,
		_satoshis: Satoshis,
		extension: Option<(FixedU128, &mut LockExtension<Self::Balance>)>,
	) -> Result<Self::Balance, VaultError> {
		ensure!(
			DefaultVault::get().available_for_lock() >= amount,
			VaultError::InsufficientVaultFunds
		);
		let term = extension.as_ref().map(|(a, _)| *a).unwrap_or(FixedU128::one());
		DefaultVault::mutate(|a| {
			if let Some((_, lock_extension)) = extension {
				a.extend_lock(amount, lock_extension)
			} else {
				a.lock(amount)
			}
		})?;
		let terms = DefaultVault::get().terms.clone();
		let total_fee = terms
			.bitcoin_annual_percent_rate
			.saturating_mul(term)
			.saturating_mul_int(amount)
			.saturating_add(terms.bitcoin_base_fee);
		if ChargeFee::get() {
			Balances::burn_from(
				locker,
				total_fee,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)
			.map_err(|_| VaultError::InsufficientFunds)?;
		}
		Ok(total_fee)
	}

	fn schedule_for_release(
		_vault_id: VaultId,
		locked_argons: Self::Balance,
		_satoshis: Satoshis,
		lock_extensions: &LockExtension<Self::Balance>,
	) -> Result<(), VaultError> {
		DefaultVault::mutate(|a| a.schedule_for_release(locked_argons, lock_extensions))?;
		Ok(())
	}

	fn compensate_lost_bitcoin(
		_vault_id: VaultId,
		_beneficiary: &Self::AccountId,
		lock_amount: Self::Balance,
		market_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
	) -> Result<Self::Balance, VaultError> {
		let result = DefaultVault::mutate(|a| a.burn(lock_amount, market_rate, lock_extension))?;
		Ok(result.burned_amount)
	}

	fn burn(
		_vault_id: VaultId,
		lock_amount: Self::Balance,
		redemption_rate: Self::Balance,
		lock_extension: &LockExtension<Self::Balance>,
	) -> Result<Self::Balance, VaultError> {
		let result =
			DefaultVault::mutate(|a| a.burn(lock_amount, redemption_rate, lock_extension))?;
		Ok(result.burned_amount)
	}

	fn create_utxo_script_pubkey(
		_vault_id: VaultId,
		_owner_pubkey: CompressedBitcoinPubkey,
		_vault_claim_height: BitcoinHeight,
		_open_claim_height: BitcoinHeight,
		_current_height: BitcoinHeight,
	) -> Result<(BitcoinXPub, BitcoinXPub, BitcoinCosignScriptPubkey), VaultError> {
		Ok((
			BitcoinXPub {
				public_key: DefaultVaultBitcoinPubkey::get().into(),
				chain_code: [0; 32],
				depth: 0,
				parent_fingerprint: [0; 4],
				child_number: 0,
				network: NetworkKind::Test,
			},
			BitcoinXPub {
				public_key: DefaultVaultReclaimBitcoinPubkey::get().into(),
				chain_code: [0; 32],
				depth: 0,
				parent_fingerprint: [0; 4],
				child_number: 1,
				network: NetworkKind::Test,
			},
			BitcoinCosignScriptPubkey::P2WSH { wscript_hash: H256::from([0; 32]) },
		))
	}

	fn remove_pending(_vault_id: VaultId, amount: Self::Balance) -> Result<(), VaultError> {
		DefaultVault::mutate(|a| {
			a.argons_pending_activation.saturating_reduce(amount);
		});
		Ok(())
	}

	fn update_pending_cosign_list(
		vault_id: VaultId,
		utxo_id: UtxoId,
		should_remove: bool,
	) -> Result<(), VaultError> {
		VaultViewOfCosignPendingLocks::mutate(|l| {
			let list = l.entry(vault_id).or_default();
			if should_remove {
				list.remove(&utxo_id);
			} else {
				list.insert(utxo_id);
			}
		});
		Ok(())
	}
}

pub struct StaticBitcoinVerifier;
impl BitcoinVerifier<Test> for StaticBitcoinVerifier {
	fn verify_signature(
		_utxo_releaseer: CosignReleaser,
		_pubkey: CompressedBitcoinPubkey,
		_signature: &BitcoinSignature,
	) -> Result<bool, DispatchError> {
		Ok(true)
	}
}

pub struct StaticBitcoinUtxoTracker;
impl BitcoinUtxoTracker for StaticBitcoinUtxoTracker {
	fn get(_utxo_id: UtxoId) -> Option<UtxoRef> {
		GetUtxoRef::get()
	}

	fn watch_for_utxo(
		utxo_id: UtxoId,
		script_pubkey: BitcoinCosignScriptPubkey,
		satoshis: Satoshis,
		watch_for_spent_until: BitcoinHeight,
	) -> Result<(), DispatchError> {
		WatchedUtxosById::mutate(|watched_utxos| {
			watched_utxos.insert(utxo_id, (script_pubkey, satoshis, watch_for_spent_until));
		});
		Ok(())
	}

	fn unwatch(utxo_id: UtxoId) {
		WatchedUtxosById::mutate(|watched_utxos| {
			watched_utxos.remove(&utxo_id);
		});
	}
}

pub(crate) fn set_bitcoin_height(height: BitcoinHeight) {
	BitcoinBlockHeightChange::set((height, height));
}

impl pallet_bitcoin_locks::Config for Test {
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type LockEvents = (EventHandler,);
	type BitcoinUtxoTracker = StaticBitcoinUtxoTracker;
	type PriceProvider = StaticPriceProvider;
	type BitcoinSignatureVerifier = StaticBitcoinVerifier;
	type GetBitcoinNetwork = GetBitcoinNetwork;
	type VaultProvider = StaticVaultProvider;
	type ArgonTicksPerDay = ArgonTicksPerDay;
	type MaxConcurrentlyReleasingLocks = MaxConcurrentlyReleasingLocks;
	type LockDurationBlocks = LockDurationBlocks;
	type LockReclamationBlocks = LockReclamationBlocks;
	type LockReleaseCosignDeadlineFrames = LockReleaseCosignDeadlineFrames;
	type BitcoinBlockHeightChange = BitcoinBlockHeightChange;
	type MaxConcurrentlyExpiringLocks = ConstU32<100>;
	type CurrentFrameId = CurrentFrameId;
	type TicksPerBitcoinBlock = TicksPerBitcoinBlock;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|t: &mut Storage| {
		pallet_bitcoin_locks::GenesisConfig::<Test> {
			minimum_bitcoin_lock_satoshis: MinimumLockSatoshis::get(),
			_phantom: Default::default(),
		}
		.assimilate_storage(t)
		.unwrap();
	})
}
