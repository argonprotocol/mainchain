use pallet_prelude::*;
use polkadot_sdk::frame_support::traits::StorageInstance;

use crate as pallet_bitcoin_fissions;
use argon_primitives::{
	bitcoin::{FissionId, Satoshis, UtxoId},
	providers::{
		BitcoinFissionLockError, BitcoinFissionLockProvider, BitcoinFissionMinting,
		OperationalAccountsHook,
	},
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		BitcoinFissions: pallet_bitcoin_fissions,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u128>;
}

parameter_types! {
	pub static ExistentialDeposit: u128 = 10;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = u128;
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

impl pallet_bitcoin_fissions::Config for Test {
	type WeightInfo = ();
	type Balance = u128;
	type LockProvider = MockLockProvider;
	type Minting = MockFissionMinting;
	type OperationalAccountsHook = MockOperationalAccounts;
	type Currency = Balances;
	type MaxFissionsPerLock = ConstU32<2>;
	type MinimumRatchetPercent = MinimumRatchetPercent;
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct MockLock {
	pub owner: u64,
	pub funded_satoshis: Satoshis,
	pub fissioned_satoshis: Satoshis,
	pub microgons_at_target_per_btc: u128,
}

parameter_types! {
	pub static RedemptionMicrogonsPerSatoshi: u128 = 100;
	pub static LiquidityMultiplierPercent: u128 = 100;
	pub static MockLastRatchetTick: Tick = 1;
	pub static MinimumRatchetPercent: Percent = Percent::from_percent(10);
}

pub struct MockLocksInstance;
impl StorageInstance for MockLocksInstance {
	fn pallet_prefix() -> &'static str {
		"BitcoinFissionsTest"
	}

	const STORAGE_PREFIX: &'static str = "Locks";
}

pub type MockLocks = StorageMap<MockLocksInstance, Twox64Concat, UtxoId, MockLock, OptionQuery>;

pub struct MockMintRequestsInstance;
impl StorageInstance for MockMintRequestsInstance {
	fn pallet_prefix() -> &'static str {
		"BitcoinFissionsTest"
	}

	const STORAGE_PREFIX: &'static str = "MintRequests";
}

pub type MockMintRequests =
	StorageValue<MockMintRequestsInstance, Vec<(u64, FissionId, UtxoId, u128)>, ValueQuery>;

pub struct MockFissionRedemptionBurnsInstance;
impl StorageInstance for MockFissionRedemptionBurnsInstance {
	fn pallet_prefix() -> &'static str {
		"BitcoinFissionsTest"
	}

	const STORAGE_PREFIX: &'static str = "LiquidRedemptionBurns";
}

pub type MockFissionRedemptionBurns =
	StorageValue<MockFissionRedemptionBurnsInstance, u128, ValueQuery>;

pub struct MockAccountBitcoinChangesInstance;
impl StorageInstance for MockAccountBitcoinChangesInstance {
	fn pallet_prefix() -> &'static str {
		"BitcoinFissionsTest"
	}

	const STORAGE_PREFIX: &'static str = "AccountBitcoinChanges";
}

pub type MockAccountBitcoinChanges =
	StorageValue<MockAccountBitcoinChangesInstance, Vec<(u64, u128, bool)>, ValueQuery>;

pub struct MockOperationalAccounts;
impl OperationalAccountsHook<u64, u128> for MockOperationalAccounts {
	fn vault_created_weight() -> Weight {
		Weight::zero()
	}

	fn vault_bitcoin_lock_funded_weight() -> Weight {
		Weight::zero()
	}

	fn mining_seat_won_weight() -> Weight {
		Weight::zero()
	}

	fn account_bitcoin_amount_changed_weight() -> Weight {
		Weight::zero()
	}

	fn account_bitcoin_amount_changed(account_id: &u64, amount: u128, is_increase: bool) {
		MockAccountBitcoinChanges::mutate(|changes| {
			changes.push((*account_id, amount, is_increase))
		});
	}

	fn account_vault_bond_total_updated_weight() -> Weight {
		Weight::zero()
	}

	fn account_uniswap_argon_transfers_in_updated_weight() -> Weight {
		Weight::zero()
	}
}

pub struct MockFissionMinting;
impl BitcoinFissionMinting<u64, u128> for MockFissionMinting {
	type Weights = ();

	fn request_mint(
		account_id: &u64,
		fission_id: FissionId,
		utxo_id: UtxoId,
		amount: u128,
	) -> DispatchResult {
		MockMintRequests::mutate(|requests| {
			requests.push((*account_id, fission_id, utxo_id, amount))
		});
		Ok(())
	}

	fn record_mint_repayment(amount: u128) {
		MockFissionRedemptionBurns::mutate(|repaid| repaid.saturating_accrue(amount));
	}
}

pub struct MockLockProvider;
impl BitcoinFissionLockProvider<u64, u128> for MockLockProvider {
	type Weights = ();

	fn fission_satoshis(
		account_id: &u64,
		utxo_id: UtxoId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: u128,
	) -> Result<(u128, Tick), BitcoinFissionLockError> {
		MockLocks::try_mutate(utxo_id, |lock| {
			let lock = lock.as_mut().ok_or(BitcoinFissionLockError::LockNotFound)?;
			if lock.owner != *account_id {
				return Err(BitcoinFissionLockError::NoPermissions);
			}
			let allocated = lock
				.fissioned_satoshis
				.checked_add(satoshis)
				.ok_or(BitcoinFissionLockError::Overflow)?;
			if allocated > lock.funded_satoshis {
				return Err(BitcoinFissionLockError::InsufficientFundedSatoshis);
			}
			lock.fissioned_satoshis = allocated;
			Ok((
				(satoshis as u128)
					.saturating_mul(microgons_at_target_per_btc)
					.saturating_mul(LiquidityMultiplierPercent::get()) /
					100,
				MockLastRatchetTick::get(),
			))
		})
	}

	fn validate_fission(
		account_id: &u64,
		utxo_id: UtxoId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: u128,
		minimum_last_ratchet_tick: Tick,
		_current_liquidity_promised: u128,
		_replacement_liquidity_promised: u128,
	) -> Result<Tick, BitcoinFissionLockError> {
		let lock = MockLocks::get(utxo_id).ok_or(BitcoinFissionLockError::LockNotFound)?;
		if lock.owner != *account_id {
			return Err(BitcoinFissionLockError::NoPermissions);
		}
		if satoshis > lock.fissioned_satoshis {
			return Err(BitcoinFissionLockError::InsufficientFissionedSatoshis);
		}
		if microgons_at_target_per_btc > lock.microgons_at_target_per_btc {
			return Err(BitcoinFissionLockError::InsufficientSecuritization);
		}
		let last_ratchet_tick = MockLastRatchetTick::get();
		if last_ratchet_tick < minimum_last_ratchet_tick {
			return Err(BitcoinFissionLockError::MicrogonsAtTargetPerBtcTickOlderThanCurrent);
		}

		Ok(last_ratchet_tick)
	}

	fn calculate_liquidity_promised(
		satoshis: Satoshis,
		microgons_at_target_per_btc: u128,
	) -> Result<u128, BitcoinFissionLockError> {
		(satoshis as u128)
			.checked_mul(microgons_at_target_per_btc)
			.and_then(|amount| amount.checked_mul(LiquidityMultiplierPercent::get()))
			.map(|amount| amount / 100)
			.ok_or(BitcoinFissionLockError::Overflow)
	}

	fn fuse_satoshis(
		account_id: &u64,
		utxo_id: UtxoId,
		satoshis: Satoshis,
		_microgons_at_target_per_btc: u128,
	) -> Result<u128, BitcoinFissionLockError> {
		MockLocks::try_mutate(utxo_id, |lock| {
			let lock = lock.as_mut().ok_or(BitcoinFissionLockError::LockNotFound)?;
			if lock.owner != *account_id {
				return Err(BitcoinFissionLockError::NoPermissions);
			}
			lock.fissioned_satoshis = lock
				.fissioned_satoshis
				.checked_sub(satoshis)
				.ok_or(BitcoinFissionLockError::InsufficientFissionedSatoshis)?;
			let redemption_rate = RedemptionMicrogonsPerSatoshi::get();
			(satoshis as u128)
				.checked_mul(redemption_rate)
				.ok_or(BitcoinFissionLockError::Overflow)
		})
	}
}

pub fn new_test_ext() -> TestState {
	LiquidityMultiplierPercent::set(100);
	MockLastRatchetTick::set(1);
	new_test_with_genesis::<Test>(|storage| {
		pallet_balances::GenesisConfig::<Test> {
			balances: vec![(1, 20_000), (2, 20_000)],
			dev_accounts: None,
		}
		.assimilate_storage(storage)
		.expect("balances genesis");
	})
}
