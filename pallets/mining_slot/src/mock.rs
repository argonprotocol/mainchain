use crate as pallet_mining_slot;
use argon_primitives::{
	block_seal::MiningSlotConfig, providers::OnNewSlot, tick::Ticker, vault::MiningBidPoolProvider,
	BlockNumber, TickProvider, VotingSchedule,
};

use frame_support::traits::{Currency, StorageMapShim};
use pallet_prelude::*;
use sp_runtime::{impl_opaque_keys, testing::UintAuthorityId};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		MiningSlots: pallet_mining_slot,
		Balances: pallet_balances::<Instance1>,
		Ownership: pallet_balances::<Instance2>,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub static TicksBetweenSlots: u64 = 1;
	pub static MaxCohortSize: u32 = 5;
	pub static MaxMiners: u32 = 10;
	pub static BlocksBeforeBidEndForVrfClose: u64 = 0;
	pub static SlotBiddingStartAfterTicks: u64 = 3;
	pub static TargetBidsPerSlot: u32 = 5;
	pub static MinOwnershipBondAmount: Balance = 1;
	pub static MaxOwnershipPercent: Percent = Percent::from_float(0.8);
	pub const ArgonotsPercentAdjustmentDamper: FixedU128 = FixedU128::from_rational(20, 100);

	pub const BidIncrements: u128 = 10_000; // 1 cent

	pub static ExistentialDeposit: Balance = 1;
}

type ArgonToken = pallet_balances::Instance1;
impl pallet_balances::Config<ArgonToken> for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
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

pub fn set_ownership(account_id: u64, amount: Balance) {
	let _ = Ownership::make_free_balance_be(&account_id, amount);
	drop(Ownership::issue(amount));
}

pub fn set_argons(account_id: u64, amount: Balance) {
	let _ = Balances::make_free_balance_be(&account_id, amount);
	drop(Balances::issue(amount));
}

pub(crate) type OwnershipToken = pallet_balances::Instance2;
impl pallet_balances::Config<OwnershipToken> for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type ReserveIdentifier = [u8; 8];
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, OwnershipToken>,
		Self::AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub static LastSlotRemoved: Vec<(u64, UintAuthorityId)> = vec![];
	pub static LastSlotAdded: Vec<(u64, UintAuthorityId)> = vec![];
	pub static GrandaRotations: Vec<FrameId> = vec![];

	// set slot bidding active by default
	pub static ElapsedTicks: u64 = 3;
	pub static CurrentTick: Tick = 1;
	pub static PreviousTick: Tick = 0;

	pub static NextSlot: BlockNumberFor<Test> = 100;
	pub static MiningWindowBlocks: BlockNumberFor<Test> = 100;

	pub static IsSlotBiddingStarted: bool = false;

	pub static GrandpaRotationFrequency: BlockNumber = 10;

	pub const BidPoolAccountId: u64 = 10000;

	pub static LastBidPoolDistribution: (FrameId, Tick) = (0, 0);
}

pub struct StaticBondProvider;
impl MiningBidPoolProvider for StaticBondProvider {
	type Balance = Balance;
	type AccountId = u64;

	fn get_bid_pool_account() -> Self::AccountId {
		BidPoolAccountId::get()
	}
}

pub struct StaticNewSlotEvent;
impl OnNewSlot<u64> for StaticNewSlotEvent {
	type Key = UintAuthorityId;
	fn rotate_grandpas(
		current_frame_id: FrameId,
		removed_authorities: Vec<(&u64, Self::Key)>,
		added_authorities: Vec<(&u64, Self::Key)>,
	) {
		LastSlotRemoved::set(removed_authorities.into_iter().map(|(a, b)| (*a, b)).collect());
		LastSlotAdded::set(added_authorities.into_iter().map(|(a, b)| (*a, b)).collect());
		GrandaRotations::mutate(|a| a.push(current_frame_id));
		LastBidPoolDistribution::set((current_frame_id, CurrentTick::get()));
	}
}

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: UintAuthorityId,
	}
}

impl Get<UintAuthorityId> for MockSessionKeys {
	fn get() -> UintAuthorityId {
		MockSessionKeys { dummy: 0.into() }.dummy
	}
}

impl From<UintAuthorityId> for MockSessionKeys {
	fn from(dummy: UintAuthorityId) -> Self {
		Self { dummy }
	}
}

impl From<u64> for MockSessionKeys {
	fn from(dummy: u64) -> Self {
		Self { dummy: dummy.into() }
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

impl pallet_mining_slot::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxMiners = MaxMiners;
	type MaxCohortSize = MaxCohortSize;
	type ArgonotsPercentAdjustmentDamper = ArgonotsPercentAdjustmentDamper;
	type MinimumArgonotsPerSeat = MinOwnershipBondAmount;
	type MaximumArgonotProrataPercent = MaxOwnershipPercent;
	type TargetBidsPerSlot = TargetBidsPerSlot;
	type Balance = Balance;
	type OwnershipCurrency = Ownership;
	type ArgonCurrency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type BidPoolProvider = StaticBondProvider;
	type SlotEvents = (StaticNewSlotEvent,);
	type GrandpaRotationBlocks = GrandpaRotationFrequency;
	type MiningAuthorityId = UintAuthorityId;
	type Keys = MockSessionKeys;
	type TickProvider = StaticTickProvider;
	type BidIncrements = BidIncrements;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|t: &mut Storage| {
		let mining_config = MiningSlotConfig {
			slot_bidding_start_after_ticks: SlotBiddingStartAfterTicks::get(),
			ticks_between_slots: TicksBetweenSlots::get(),
			ticks_before_bid_end_for_vrf_close: BlocksBeforeBidEndForVrfClose::get(),
		};

		pallet_mining_slot::GenesisConfig::<Test> { mining_config, _phantom: Default::default() }
			.assimilate_storage(t)
			.unwrap();
	})
}
