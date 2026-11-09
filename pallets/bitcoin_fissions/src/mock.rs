use argon_primitives::{
	bitcoin::LiquidId, tick::Ticker, CrosschainGatewayProvider, GatewayState,
	LiquidRepaymentProvider, SourceChain, TickProvider, VotingSchedule,
};
use core::time::Duration;
use frame_support::traits::UnixTime;
use pallet_prelude::*;
use polkadot_sdk::frame_support::traits::StorageInstance;

use crate as pallet_bitcoin_fissions;
use crate::{Error, FissionByOwnerAndId, FissionIdsByOwnerAndLiquid};
use argon_primitives::{
	bitcoin::{BitcoinHeight, BitcoinLockId, FissionId, Satoshis},
	providers::{
		BitcoinFissionLockError, BitcoinFissionLockProvider, BitcoinFissionMinting,
		OperationalAccountsHook,
	},
	vault::{BitcoinSecuritizationBasis, CosignerGuarantee},
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		BitcoinFissions: pallet_bitcoin_fissions,
		TreasuryPositions: pallet_treasury_positions,
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
	type FeeCoupon = ();
	type WeightInfo = ();
	type Balance = u128;
	type LockProvider = MockLockProvider;
	type Minting = MockFissionMinting;
	type PositionProvider = TreasuryPositions;
	type OperationalAccountsHook = MockOperationalAccounts;
	type Currency = Balances;
	type LiquidRepaymentProvider = Test;
	type MaxFissionsPerLock = ConstU32<2>;
	type MaxExpectedFissionsPerLiquid = ConstU32<2>;
	type MinimumRatchetPercent = MinimumRatchetPercent;
	type MiningFrameTransitionProvider = FissionFrames;
	type TickProvider = FissionTick;
	type TimeProvider = FissionTick;
	type CrosschainGatewayProvider = Test;
	type RValueHistoryFrames = ConstU32<30>;
	type MaxRValueHistoryPrunesPerBlock = ConstU32<16>;
}

impl pallet_treasury_positions::Config for Test {
	type Balance = u128;
	type BitcoinPositionProvider = ();
	type TreasuryPoolProvider = ();
	type OperationalAccountProvider = ();
	type WeightInfo = ();
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct MockLock {
	pub owner: u64,
	pub funded_satoshis: Satoshis,
	pub fissioned_satoshis: Satoshis,
	pub microgons_at_target_per_btc: u128,
}

pub(crate) fn lock(owner: u64, funded_satoshis: u64) -> MockLock {
	MockLock { owner, funded_satoshis, fissioned_satoshis: 0, microgons_at_target_per_btc: 100 }
}

parameter_types! {
	pub static RedemptionMicrogonsPerSatoshi: u128 = 100;
	pub static LiquidityMultiplierPercent: u128 = 100;
	pub static MockLastRatchetTick: Tick = 1;
	pub static MinimumRatchetPercent: Percent = Percent::from_percent(10);
	pub static PendingLiquidRepayment: Option<(u64, LiquidId)> = None;
	pub static PendingPriceHistoryTick: Option<Tick> = None;
	pub static FissionTimestamp: u64 = 0;
	pub static RepaymentWindowUnavailableForLock: Option<BitcoinLockId> = None;
}

impl LiquidRepaymentProvider<u64> for Test {
	type Weights = ();

	fn has_pending_repayment(account: &u64, liquid_id: LiquidId) -> bool {
		PendingLiquidRepayment::get() == Some((*account, liquid_id))
	}
	fn oldest_pending_purchase_tick() -> Option<Tick> {
		PendingPriceHistoryTick::get()
	}
}

pub struct MockLocksInstance;
impl StorageInstance for MockLocksInstance {
	fn pallet_prefix() -> &'static str {
		"BitcoinFissionsTest"
	}

	const STORAGE_PREFIX: &'static str = "Locks";
}

pub type MockLocks =
	StorageMap<MockLocksInstance, Twox64Concat, BitcoinLockId, MockLock, OptionQuery>;

pub struct MockMintRequestsInstance;
impl StorageInstance for MockMintRequestsInstance {
	fn pallet_prefix() -> &'static str {
		"BitcoinFissionsTest"
	}

	const STORAGE_PREFIX: &'static str = "MintRequests";
}

pub type MockMintRequests =
	StorageValue<MockMintRequestsInstance, Vec<(u64, FissionId, BitcoinLockId, u128)>, ValueQuery>;

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
	type Weights = ();
	fn account_bitcoin_amount_changed(account_id: &u64, amount: u128, is_increase: bool) {
		MockAccountBitcoinChanges::mutate(|changes| {
			changes.push((*account_id, amount, is_increase))
		});
	}
}

pub struct MockFissionMinting;
impl BitcoinFissionMinting<u64, u128> for MockFissionMinting {
	type Weights = ();

	fn request_mint(
		account_id: &u64,
		fission_id: FissionId,
		lock_id: BitcoinLockId,
		amount: u128,
	) -> DispatchResult {
		MockMintRequests::mutate(|requests| {
			requests.push((*account_id, fission_id, lock_id, amount))
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
	type FeeCoupon = ();

	fn fission_satoshis(
		account_id: &u64,
		lock_id: BitcoinLockId,
		satoshis: Satoshis,
		microgons_at_target_per_btc: u128,
		_fee_coupon: Option<Self::FeeCoupon>,
	) -> Result<(u128, Tick, CosignerGuarantee<u128>), BitcoinFissionLockError> {
		MockLocks::try_mutate(lock_id, |lock| {
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
				CosignerGuarantee {
					basis: BitcoinSecuritizationBasis {
						satoshis,
						microgons_at_target_per_btc: lock.microgons_at_target_per_btc,
						tick: MockLastRatchetTick::get(),
					},
					guaranteed_microgons: Self::calculate_liquidity_promised(
						satoshis,
						lock.microgons_at_target_per_btc,
					)?,
					fund_hold_extensions: Default::default(),
				},
			))
		})
	}

	fn update_fission_guarantee(
		account_id: &u64,
		lock_id: BitcoinLockId,
		satoshis: Satoshis,
		_guarantee: &mut CosignerGuarantee<u128>,
		microgons_at_target_per_btc: u128,
		minimum_last_ratchet_price_tick: Tick,
		_fee_coupon: Option<Self::FeeCoupon>,
	) -> Result<Tick, BitcoinFissionLockError> {
		let lock = MockLocks::get(lock_id).ok_or(BitcoinFissionLockError::LockNotFound)?;
		if lock.owner != *account_id {
			return Err(BitcoinFissionLockError::NoPermissions);
		}
		if satoshis > lock.fissioned_satoshis {
			return Err(BitcoinFissionLockError::InsufficientFissionedSatoshis);
		}
		if microgons_at_target_per_btc > lock.microgons_at_target_per_btc {
			return Err(BitcoinFissionLockError::InsufficientSecuritization);
		}
		let last_ratchet_price_tick = MockLastRatchetTick::get();
		if last_ratchet_price_tick < minimum_last_ratchet_price_tick {
			return Err(BitcoinFissionLockError::MicrogonsAtTargetPerBtcTickOlderThanCurrent);
		}

		Ok(last_ratchet_price_tick)
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

	fn can_repay_before_expiry(lock_id: BitcoinLockId, _: FrameId) -> bool {
		RepaymentWindowUnavailableForLock::get() != Some(lock_id)
	}

	fn get_lock_expiration(
		lock_id: BitcoinLockId,
	) -> Result<BitcoinHeight, BitcoinFissionLockError> {
		MockLocks::get(lock_id)
			.map(|_| lock_id)
			.ok_or(BitcoinFissionLockError::LockNotFound)
	}
	fn calculate_redemption_amount(
		satoshis: Satoshis,
		_: u128,
	) -> Result<u128, BitcoinFissionLockError> {
		(satoshis as u128)
			.checked_mul(RedemptionMicrogonsPerSatoshi::get())
			.ok_or(BitcoinFissionLockError::Overflow)
	}

	fn fuse_satoshis(
		account_id: &u64,
		lock_id: BitcoinLockId,
		satoshis: Satoshis,
		_guarantee: CosignerGuarantee<u128>,
	) -> Result<(), BitcoinFissionLockError> {
		MockLocks::try_mutate(lock_id, |lock| {
			let lock = lock.as_mut().ok_or(BitcoinFissionLockError::LockNotFound)?;
			if lock.owner != *account_id {
				return Err(BitcoinFissionLockError::NoPermissions);
			}
			lock.fissioned_satoshis = lock
				.fissioned_satoshis
				.checked_sub(satoshis)
				.ok_or(BitcoinFissionLockError::InsufficientFissionedSatoshis)?;
			Ok(())
		})
	}
}

pub fn new_test_ext() -> TestState {
	RepaymentWindowUnavailableForLock::set(None);
	FissionTimestamp::set(0);
	FissionFrame::set(1);
	FissionTick::set(1);
	FissionNewFrame::set(None);
	LastGatewayState::set(None);
	LiquidityMultiplierPercent::set(100);
	MockLastRatchetTick::set(1);
	PendingLiquidRepayment::set(None);
	PendingPriceHistoryTick::set(None);
	new_test_with_genesis::<Test>(|storage| {
		pallet_balances::GenesisConfig::<Test> {
			balances: vec![(1, 20_000), (2, 20_000)],
			dev_accounts: None,
		}
		.assimilate_storage(storage)
		.expect("balances genesis");
	})
}

/// Exercise partial accounting inside the transaction supplied by a production caller.
pub fn repay_fission(
	account_id: &u64,
	fission_id: FissionId,
	satoshis: Satoshis,
) -> Result<u128, DispatchError> {
	with_storage_layer(|| {
		let fission = FissionByOwnerAndId::<Test>::get(account_id, fission_id)
			.ok_or(Error::<Test>::FissionNotFound)?;
		let liquid_id = fission.liquid_id;
		let closes = satoshis == fission.satoshis;
		let amount = MockLockProvider::calculate_redemption_amount(
			satoshis,
			fission.microgons_at_target_per_btc,
		)
		.map_err(Error::<Test>::from)?;
		let liquidity =
			BitcoinFissions::apply_repayment(account_id, fission_id, fission, satoshis, amount)?;
		Balances::burn_from(
			account_id,
			amount,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Force,
		)
		.map_err(|_| Error::<Test>::InsufficientFunds)?;
		BitcoinFissions::record_repayment(account_id, amount, liquidity)?;
		if closes {
			FissionIdsByOwnerAndLiquid::<Test>::remove((account_id, liquid_id), fission_id);
		}
		Ok(amount)
	})
}

parameter_types! {
	pub static FissionFrame: FrameId = 1;
	pub static FissionTick: Tick = 1;
	pub static FissionNewFrame: Option<FrameId> = None;
	pub static LastGatewayState: Option<GatewayState<u128>> = None;
	pub HistoryTicker: Ticker = Ticker::new(1_000, 2);
}
pub struct FissionFrames;
impl argon_primitives::MiningFrameTransitionProvider for FissionFrames {
	type Weights = ();

	fn get_current_frame_id() -> FrameId {
		FissionFrame::get()
	}
	fn is_new_frame_started() -> Option<FrameId> {
		FissionNewFrame::get()
	}
}

impl CrosschainGatewayProvider<u128> for Test {
	type Weights = ();

	fn last_state_by_chain(_source_chain: SourceChain) -> Option<GatewayState<u128>> {
		LastGatewayState::get()
	}
}

impl TickProvider<Block> for FissionTick {
	type Weights = ();
	fn current_tick() -> Tick {
		FissionTick::get()
	}
	fn previous_tick() -> Tick {
		FissionTick::get().saturating_sub(1)
	}
	fn elapsed_ticks() -> Tick {
		FissionTick::get()
	}
	fn voting_schedule() -> VotingSchedule {
		VotingSchedule::from_runtime_current_tick(FissionTick::get())
	}
	fn ticker() -> Ticker {
		HistoryTicker::get()
	}
	fn blocks_at_tick(_tick: Tick) -> Vec<H256> {
		Vec::new()
	}
}

impl UnixTime for FissionTick {
	fn now() -> Duration {
		Duration::from_millis(FissionTimestamp::get())
	}
}
