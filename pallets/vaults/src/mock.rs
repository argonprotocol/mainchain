use crate as pallet_vaults;
use argon_primitives::{
	MiningSlotProvider, TickProvider, VotingSchedule,
	bitcoin::{BitcoinHeight, BitcoinNetwork},
	tick::Ticker,
};
use frame_support::traits::Currency;
use pallet_prelude::*;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Vaults: pallet_vaults,
		Treasury: pallet_treasury,
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
	pub static PreviousTick: Tick = 1;
	pub static ElapsedTicks: Tick = 4;
	pub static CurrentFrameId: FrameId = 1;

	pub static LastBitcoinHeightChange: (BitcoinHeight, BitcoinHeight) = (10, 11);
	pub static IsSlotBiddingStarted: bool = false;

	pub const BidPoolAccountId: u64 = 10000;

	pub static LastBidPoolDistribution: (FrameId, Tick) = (0, 0);

	pub static MaxTreasuryContributors: u32 = 10;
	pub static MinimumArgonsPerContributor: u128 = 100_000_000;
	pub static MaxBidPoolVaultParticipants: u32 = 100;
	pub static VaultPalletId: PalletId = PalletId(*b"bidPools");

	pub static BurnFromBidPoolAmount: Percent = Percent::from_percent(20);

}
pub struct StaticMiningSlotProvider;
impl MiningSlotProvider for StaticMiningSlotProvider {
	fn get_next_slot_tick() -> Tick {
		NextSlot::get()
	}

	fn mining_window_ticks() -> Tick {
		MiningWindowBlocks::get()
	}
	fn is_slot_bidding_started() -> bool {
		IsSlotBiddingStarted::get()
	}
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
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

impl pallet_vaults::Config for Test {
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxPendingTermModificationsPerTick = ConstU32<100>;
	type CurrentFrameId = CurrentFrameId;
	type MiningSlotProvider = StaticMiningSlotProvider;
	type GetBitcoinNetwork = GetBitcoinNetwork;
	type BitcoinBlockHeightChange = LastBitcoinHeightChange;
	type TickProvider = StaticTickProvider;
	type MaxVaults = ConstU32<100>;
	type MaxPendingCosignsPerVault = ConstU32<100>;
	type RevenueCollectionExpirationFrames = ConstU64<10>;
}

impl pallet_treasury::Config for Test {
	type WeightInfo = ();
	type Balance = Balance;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type TreasuryVaultProvider = Vaults;
	type MaxTreasuryContributors = MaxTreasuryContributors;
	type MinimumArgonsPerContributor = MinimumArgonsPerContributor;
	type PalletId = VaultPalletId;
	type BidPoolBurnPercent = BurnFromBidPoolAmount;
	type MaxBidPoolVaultParticipants = MaxBidPoolVaultParticipants;
	type GetCurrentFrameId = CurrentFrameId;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
