use crate as pallet_vaults;
use argon_bitcoin::{
	primitives::{
		BitcoinCosignScriptPubkey, BitcoinSignature, CompressedBitcoinPubkey, Satoshis, UtxoId,
		UtxoRef,
	},
	CosignReleaser,
};
use argon_primitives::{
	bitcoin::{BitcoinHeight, BitcoinNetwork},
	tick::Ticker,
	CollectBlockerProvider, MiningFrameProvider, OperationalAccountProvider, TickProvider,
	VotingSchedule,
};
use frame_support::traits::Currency;
use pallet_bitcoin_locks::BitcoinVerifier;
use pallet_prelude::{
	argon_primitives::{
		ArgonCPI, BitcoinUtxoTracker, MiningFrameTransitionProvider, PriceProvider, UtxoLockEvents,
	},
	*,
};
use std::collections::{BTreeMap, BTreeSet};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Vaults: pallet_vaults,
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
	pub static GetBitcoinNetwork: BitcoinNetwork = BitcoinNetwork::Regtest;
}

impl pallet_balances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type ReserveIdentifier = ();
	type FreezeIdentifier = ();
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type MaxFreezes = ();
	type DoneSlashHandler = ();
}

pub fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

parameter_types! {
	pub static NextSlot: BlockNumberFor<Test> = 100;
	pub static MiningWindowBlocks: BlockNumberFor<Test> = 100;
	pub const FundingChangeBlockDelay: BlockNumberFor<Test> = 60;

	pub static CurrentTick: Tick = 1;
	pub static DidStartNewFrame: bool = false;
	pub static PreviousTick: Tick = 1;
	pub static ElapsedTicks: Tick = 4;
	pub static CurrentFrameId: FrameId = 1;

	pub static LastBitcoinHeightChange: (BitcoinHeight, BitcoinHeight) = (10, 11);
	pub static IsSlotBiddingStarted: bool = false;
	pub const TicksPerFrame: Tick = 10;

	pub const BidPoolAccountId: u64 = 10000;
	pub const TreasuryReservesAccountId: u64 = 10001;

	pub static LastBidPoolDistribution: (FrameId, Tick) = (0, 0);

	pub static MaxTreasuryContributors: u32 = 10;
	pub static MinimumArgonsPerContributor: u128 = 100_000_000;
	pub static MaxVaultsPerPool: u32 = 100;
	pub static MaxPendingUnlocksPerFrame: u32 = 100;
	pub static TreasuryExitDelayFrames: FrameId = 10;
	pub static VaultPalletId: PalletId = PalletId(*b"bidPools");
	pub const OperationalMinimumVaultSecuritization: Balance = 2_000;
	pub const OperationalMinimumVaultLockTicks: Tick = 1_440 * 365;
	pub const RecentCapacityDropBlockWindow: u32 = 8;
	pub const MaxRecentCapacityDropsPerVault: u32 = 64;
	pub const CapacityDropAttemptUnit: Balance = 100;

	pub static PercentForTreasuryReserves: Percent = Percent::from_percent(20);
	pub static OverdueCollectBlockers: BTreeSet<u64> = BTreeSet::new();
	pub static OperationalAccountsInviteOnly: bool = false;
	pub static RegisteredOperationalAccounts: BTreeSet<u64> = BTreeSet::new();

}
pub struct StaticMiningFrameProvider;
impl MiningFrameTransitionProvider for StaticMiningFrameProvider {
	fn is_new_frame_started() -> Option<FrameId> {
		Some(CurrentFrameId::get())
	}
	fn get_current_frame_id() -> FrameId {
		CurrentFrameId::get()
	}
}

impl MiningFrameProvider for StaticMiningFrameProvider {
	fn get_next_frame_tick() -> Tick {
		NextSlot::get()
	}

	fn is_seat_bidding_started() -> bool {
		IsSlotBiddingStarted::get()
	}
	fn get_tick_range_for_frame(frame_id: FrameId) -> Option<(Tick, Tick)> {
		let current_frame = CurrentFrameId::get();
		let frame_offset = frame_id.checked_sub(current_frame)?;
		let starting_tick = NextSlot::get()
			.saturating_sub(TicksPerFrame::get())
			.saturating_add(frame_offset.saturating_mul(TicksPerFrame::get()));
		Some((starting_tick, starting_tick.saturating_add(TicksPerFrame::get())))
	}
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
	type Weights = ();

	fn previous_tick() -> Tick {
		PreviousTick::get()
	}
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn elapsed_ticks() -> Tick {
		ElapsedTicks::get()
	}
	fn voting_schedule() -> VotingSchedule {
		todo!()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 2)
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		todo!()
	}
}

pub struct MockCollectBlockerProvider;
impl CollectBlockerProvider<u64> for MockCollectBlockerProvider {
	type Weights = ();

	fn has_overdue_collect_blocker(account_id: &u64) -> bool {
		OverdueCollectBlockers::get().contains(account_id)
	}
}

pub struct MockOperationalAccountProvider;
impl OperationalAccountProvider<u64> for MockOperationalAccountProvider {
	type Weights = ();

	fn is_eligible(account_id: &u64) -> bool {
		!OperationalAccountsInviteOnly::get() ||
			RegisteredOperationalAccounts::get().contains(account_id)
	}
}

impl pallet_vaults::Config for Test {
	type WeightInfo = ();
	type Currency = Balances;
	type OwnershipCurrency = Balances;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxPendingTermModificationsPerTick = ConstU32<100>;
	type CurrentFrameId = CurrentFrameId;
	type MiningFrameProvider = StaticMiningFrameProvider;
	type GetBitcoinNetwork = GetBitcoinNetwork;
	type BitcoinBlockHeightChange = LastBitcoinHeightChange;
	type TicksPerBitcoinBlock = TicksPerBitcoinBlock;
	type TicksPerFrame = TicksPerFrame;
	type TickProvider = StaticTickProvider;
	type MaxVaults = ConstU32<100>;
	type MaxPendingCosignsPerVault = ConstU32<100>;
	type RevenueCollectionExpirationFrames = ConstU64<10>;
	type OperationalMinimumVaultSecuritization = OperationalMinimumVaultSecuritization;
	type OperationalMinimumVaultLockTicks = OperationalMinimumVaultLockTicks;
	type RecentCapacityDropBlockWindow = RecentCapacityDropBlockWindow;
	type MaxRecentCapacityDropsPerVault = MaxRecentCapacityDropsPerVault;
	type CapacityDropAttemptUnit = CapacityDropAttemptUnit;
	type OperationalAccountsHook = ();
	type OperationalAccountProvider = MockOperationalAccountProvider;
	type CollectBlockerProvider = MockCollectBlockerProvider;
}

pub struct StaticBitcoinUtxoTracker;
impl BitcoinUtxoTracker for StaticBitcoinUtxoTracker {
	fn get_funding_utxo_ref(_utxo_id: UtxoId) -> Option<UtxoRef> {
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

	fn unwatch_candidate(utxo_id: UtxoId, utxo_ref: &UtxoRef) -> Option<(UtxoRef, Satoshis)> {
		let _ = (utxo_id, utxo_ref);
		None
	}
}

parameter_types! {
	pub static MaxConcurrentlyReleasingLocks: u32 = 10;
	pub static BitcoinPriceInUsd: Option<FixedU128> = Some(FixedU128::from_rational(62_000, 1));
	pub static ArgonPriceInUsd: Option<FixedU128> = Some(FixedU128::from_rational(100, 100));
	pub static ArgonTargetPriceInUsd: Option<FixedU128> = Some(FixedU128::from_rational(100, 100));
	pub static LockReleaseCosignDeadlineFrames: FrameId = 5;
	pub static OrphanedUtxoReleaseExpiryFrames: FrameId = 5;
	pub static LockReclamationBlocks: BitcoinHeight = 30;
	pub static LockDurationBlocks: BitcoinHeight = 144 * 365;
	pub static BitcoinBlockHeightChange: (BitcoinHeight, BitcoinHeight) = (0, 0);
	pub static MinimumLockSatoshis: Satoshis = 10_000_000;

	pub static NextUtxoId: UtxoId = 1;
	pub static WatchedUtxosById: BTreeMap<UtxoId, (BitcoinCosignScriptPubkey, Satoshis, BitcoinHeight)> = BTreeMap::new();

	pub static GetUtxoRef: Option<UtxoRef> = None;

	pub static LastLockEvent: Option<(UtxoId, u64, Balance)> = None;
	pub static LastReleaseEvent: Option<(UtxoId, bool, Balance, Balance)> = None;

	pub static CanceledLocks: Vec<(VaultId, Balance)> = Vec::new();

	pub static ChargeFee: bool = false;

	pub static VaultViewOfCosignPendingLocks: BTreeMap<VaultId,  BTreeSet<UtxoId>> = BTreeMap::new();

	pub const TicksPerBitcoinBlock: u64 = 10;
	pub const ArgonTicksPerDay: u64 = 1440;
}

pub struct EventHandler;
impl UtxoLockEvents<u64, Balance> for EventHandler {
	type Weights = ();

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
		_account_id: &u64,
		remove_pending_mints: bool,
		amount_burned: Balance,
		original_liquidity_promised: Balance,
	) -> DispatchResult {
		LastReleaseEvent::set(Some((
			utxo_id,
			remove_pending_mints,
			amount_burned,
			original_liquidity_promised,
		)));

		Ok(())
	}
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	type Weights = ();

	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		BitcoinPriceInUsd::get()
	}
	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		ArgonPriceInUsd::get()
	}
	fn get_argonot_price_in_usd() -> Option<FixedU128> {
		ArgonPriceInUsd::get()
	}
	fn get_target_argon_price_in_usd() -> Option<FixedU128> {
		ArgonTargetPriceInUsd::get()
	}
	fn get_argon_cpi() -> Option<ArgonCPI> {
		let ratio = ArgonTargetPriceInUsd::get()? / ArgonPriceInUsd::get()?;
		let ratio_as_cpi = ArgonCPI::from_inner(ratio.into_inner() as i128);
		Some(ratio_as_cpi - One::one())
	}
	fn get_redemption_r_value() -> Option<FixedU128> {
		Some(ArgonPriceInUsd::get()? / ArgonTargetPriceInUsd::get()?)
	}
	fn get_circulation() -> Balance {
		1000
	}
	fn get_average_cpi_for_ticks(_tick_range: (Tick, Tick)) -> ArgonCPI {
		Self::get_argon_cpi().unwrap_or_default()
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
	type VaultProvider = Vaults;
	type ArgonTicksPerDay = ArgonTicksPerDay;
	type MaxConcurrentlyReleasingLocks = MaxConcurrentlyReleasingLocks;
	type LockDurationBlocks = LockDurationBlocks;
	type LockReclamationBlocks = LockReclamationBlocks;
	type LockReleaseCosignDeadlineFrames = LockReleaseCosignDeadlineFrames;
	type OrphanedUtxoReleaseExpiryFrames = OrphanedUtxoReleaseExpiryFrames;
	type BitcoinBlockHeightChange = BitcoinBlockHeightChange;
	type MaxConcurrentlyExpiringLocks = ConstU32<100>;
	type CurrentFrameId = CurrentFrameId;
	type TicksPerBitcoinBlock = TicksPerBitcoinBlock;
	type DidStartNewFrame = DidStartNewFrame;
	type MaxBtcPriceTickAge = ConstU32<100>;
	type CurrentTick = CurrentTick;
}

pub fn new_test_ext() -> TestState {
	OverdueCollectBlockers::set(BTreeSet::new());
	OperationalAccountsInviteOnly::set(false);
	RegisteredOperationalAccounts::set(BTreeSet::new());
	new_test_with_genesis::<Test>(|_t| {})
}
