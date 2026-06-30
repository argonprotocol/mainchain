use crate as pallet_mining_slot;
use argon_primitives::{
	block_seal::MiningSlotConfig, providers::OnNewSlot, tick::Ticker, ArgonCPI, BlockNumber,
	BlockSealerInfo, BlockSealerProvider, OperationalAccountProvider, OperationalAccountsHook,
	PriceProvider, TickProvider, VotingSchedule,
};
use frame_support::traits::{Currency, StorageMapShim};
use pallet_prelude::*;
use sp_runtime::{impl_opaque_keys, testing::UintAuthorityId};
use std::collections::{BTreeMap, BTreeSet};

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
	pub static MinCohortSize: u32 = 5;
	pub static MaxCohortSize: u32 = 5;
	pub static FramesPerMiningTerm: u32 = 2;
	pub static BlocksBeforeBidEndForVrfClose: u64 = 0;
	pub static SlotBiddingStartAfterTicks: u64 = 3;
	pub static InitialArgonotsPerSeat: Balance = 1;
	pub const PricePerSeatAdjustmentDamper: FixedU128 = FixedU128::from_rational(20, 100);
	pub static TargetPricePerSeat: Balance = 10 * 1_000_000; // 10 Argons
	pub const ArgonotBidCollateralMultiple: u128 = 2;

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

pub fn set_average_microgons_per_argonot(frame_id: FrameId, amount: Balance) {
	AverageMicrogonsPerArgonotByFrame::mutate(|history| {
		history.insert(frame_id, amount);
	});
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
	pub static MiningSeatsWon: Vec<u64> = vec![];
	pub static OperationalAccountsInviteOnly: bool = false;
	pub static RegisteredOperationalAccounts: BTreeSet<u64> = BTreeSet::new();

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
	pub static BlockSealer: BlockSealerInfo<u64, UintAuthorityId> = BlockSealerInfo {
		block_seal_authority: None,
		block_vote_rewards_account: Some(1),
		block_author_account_id: 1,
	};
	pub static IsBlockVoteSeal: bool = false;
	pub static AverageMicrogonsPerArgonotByFrame: BTreeMap<FrameId, Balance> = BTreeMap::new();
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

pub struct StaticOperationalAccountsHook;
impl OperationalAccountsHook<u64, Balance> for StaticOperationalAccountsHook {
	fn vault_created_weight() -> Weight {
		Weight::zero()
	}

	fn vault_bitcoin_lock_funded_weight() -> Weight {
		Weight::zero()
	}

	fn mining_seat_won_weight() -> Weight {
		Weight::zero()
	}

	fn account_vault_bond_total_updated_weight() -> Weight {
		Weight::zero()
	}

	fn account_uniswap_argon_transfers_in_updated_weight() -> Weight {
		Weight::zero()
	}

	fn mining_seat_won(miner_account: &u64) {
		MiningSeatsWon::mutate(|accounts| accounts.push(*miner_account));
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

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: UintAuthorityId,
	}
}
impl MaxEncodedLen for MockSessionKeys {
	fn max_encoded_len() -> usize {
		<UintAuthorityId as MaxEncodedLen>::max_encoded_len()
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

pub struct StaticBlockSealerProvider;
impl BlockSealerProvider<u64, UintAuthorityId> for StaticBlockSealerProvider {
	type Weights = ();

	fn get_sealer_info() -> BlockSealerInfo<u64, UintAuthorityId> {
		BlockSealer::get()
	}
	fn is_block_vote_seal() -> bool {
		IsBlockVoteSeal::get()
	}
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	type Weights = ();

	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		None
	}

	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		None
	}

	fn get_argonot_price_in_usd() -> Option<FixedU128> {
		None
	}

	fn get_target_argon_price_in_usd() -> Option<FixedU128> {
		None
	}

	fn get_average_microgons_per_argonot(frame_id: FrameId) -> Option<Balance> {
		AverageMicrogonsPerArgonotByFrame::get().get(&frame_id).copied()
	}

	fn get_argon_cpi() -> Option<ArgonCPI> {
		None
	}

	fn get_average_cpi_for_ticks(_tick_range: (Tick, Tick)) -> ArgonCPI {
		ArgonCPI::zero()
	}

	fn get_circulation() -> Balance {
		0
	}

	fn get_redemption_r_value() -> Option<FixedU128> {
		Some(FixedU128::from_u32(1))
	}
}

impl pallet_mining_slot::Config for Test {
	type WeightInfo = ();
	type MinCohortSize = MinCohortSize;
	type MaxCohortSize = MaxCohortSize;
	type PricePerSeatAdjustmentDamper = PricePerSeatAdjustmentDamper;
	type FramesPerMiningTerm = FramesPerMiningTerm;
	type TargetPricePerSeat = TargetPricePerSeat;
	type InitialArgonotsPerSeat = InitialArgonotsPerSeat;
	type ArgonotBidCollateralMultiple = ArgonotBidCollateralMultiple;
	type Balance = Balance;
	type OwnershipCurrency = Ownership;
	type ArgonCurrency = Balances;
	type PriceProvider = StaticPriceProvider;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MiningBidPoolAccount = BidPoolAccountId;
	type OperationalAccountProvider = MockOperationalAccountProvider;
	type OperationalAccountsHook = StaticOperationalAccountsHook;
	type SlotEvents = (StaticNewSlotEvent,);
	type GrandpaRotationBlocks = GrandpaRotationFrequency;
	type MiningAuthorityId = UintAuthorityId;
	type Keys = MockSessionKeys;
	type TickProvider = StaticTickProvider;
	type BidIncrements = BidIncrements;
	type SealerInfo = StaticBlockSealerProvider;
}

pub fn new_test_ext() -> TestState {
	AverageMicrogonsPerArgonotByFrame::set(BTreeMap::new());
	OperationalAccountsInviteOnly::set(false);
	RegisteredOperationalAccounts::set(BTreeSet::new());
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
