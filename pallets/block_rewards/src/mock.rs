use frame_support::traits::StorageMapShim;
use pallet_prelude::*;

use crate as pallet_block_rewards;
use crate::GrowthPath;
use argon_primitives::{
	BlockRewardAccountsProvider, BlockRewardsEventHandler, BlockSealerInfo, BlockSealerProvider,
	NotebookProvider, NotebookSecret, PriceProvider, TickProvider, VotingSchedule,
	block_seal::BlockPayout,
	notary::{NotaryProvider, NotarySignature},
	tick::Ticker,
};

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type TestAccountId = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		BlockRewards: pallet_block_rewards,
		Balances: pallet_balances::<Instance1>::{Pallet, Call, Storage, Config<T>, Event<T>},
		Ownership: pallet_balances::<Instance2>::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);
#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = TestAccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = RocksDbWeight;
}

parameter_types! {

	pub static ExistentialDeposit: Balance = 10;
}

type ArgonToken = pallet_balances::Instance1;
impl pallet_balances::Config<ArgonToken> for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

type OwnershipToken = pallet_balances::Instance2;
impl pallet_balances::Config<OwnershipToken> for Test {
	type MaxLocks = ConstU32<0>;
	type MaxReserves = ConstU32<0>;
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = StorageMapShim<
		pallet_balances::Account<Test, OwnershipToken>,
		TestAccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type WeightInfo = ();
	type FreezeIdentifier = RuntimeFreezeReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub static StartingArgonsPerBlock :u32 = 5_000;
	pub static StartingOwnershipTokensPerBlock :u32 = 5_000;
	pub static IncrementalGrowth :GrowthPath<Balance> = (1_000, 100, 500_000);
	pub static HalvingBeginBlock: u32 = 1000;
	pub static HalvingBlocks :u32 = 100;
	pub static MinerPayoutPercent :FixedU128 = FixedU128::from_rational(75, 100);
	pub static ActiveNotaries: Vec<NotaryId> = vec![1];
	pub static NotebookTick: Tick = 0;
	pub static ElapsedTicks: Tick = 0;

	pub static NotebooksInBlock: Vec<(NotaryId, NotebookNumber, Tick)> = vec![];

	pub static BlockSealer:BlockSealerInfo<u64> = BlockSealerInfo {
		block_vote_rewards_account: Some(1),
		block_author_account_id: 1,
		block_seal_authority: None
	};
	pub static IsMiningSlotsActive: bool = false;
	pub static IsBlockVoteSeal: bool = false;
	pub static LastRewards: Vec<BlockPayout<TestAccountId, Balance>> = vec![];
	pub static AccountCohorts: Vec<(TestAccountId, FrameId)> = vec![];
}

pub struct StaticBlockSealerProvider;
impl BlockSealerProvider<u64> for StaticBlockSealerProvider {
	fn get_sealer_info() -> BlockSealerInfo<u64> {
		BlockSealer::get()
	}
	fn is_block_vote_seal() -> bool {
		IsBlockVoteSeal::get()
	}
}

pub struct TestProvider;
impl NotaryProvider<Block, TestAccountId> for TestProvider {
	fn verify_signature(_: NotaryId, _: Tick, _: &H256, _: &NotarySignature) -> bool {
		true
	}
	fn active_notaries() -> Vec<NotaryId> {
		ActiveNotaries::get()
	}
	fn notary_operator_account_id(_notary_id: NotaryId) -> Option<TestAccountId> {
		todo!()
	}
}
impl NotebookProvider for TestProvider {
	fn get_eligible_tick_votes_root(
		_notary_id: NotaryId,
		_tick: Tick,
	) -> Option<(H256, NotebookNumber)> {
		todo!()
	}
	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		NotebooksInBlock::get()
	}
	fn notebooks_at_tick(_tick: Tick) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		todo!()
	}
	fn is_notary_locked_at_tick(_notary_id: NotaryId, _tick: Tick) -> bool {
		todo!()
	}
}

pub struct StaticBlockRewardAccountsProvider;
impl BlockRewardAccountsProvider<u64> for StaticBlockRewardAccountsProvider {
	fn get_block_rewards_account(author: &u64) -> Option<(u64, FrameId)> {
		AccountCohorts::get().iter().find(|(a, _)| a == author).cloned()
	}

	fn get_mint_rewards_accounts() -> Vec<(u64, FrameId)> {
		todo!("not used by rewards")
	}
	fn is_compute_block_eligible_for_rewards() -> bool {
		IsBlockVoteSeal::get() || !IsMiningSlotsActive::get()
	}
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
	fn previous_tick() -> Tick {
		todo!()
	}
	fn current_tick() -> Tick {
		NotebookTick::get() + 1
	}
	fn elapsed_ticks() -> Tick {
		ElapsedTicks::get()
	}
	fn ticker() -> Ticker {
		Ticker::new(2000, 2)
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		vec![]
	}
	fn voting_schedule() -> VotingSchedule {
		VotingSchedule::on_notebook_tick_state(NotebookTick::get())
	}
}

pub struct RewardEvents;
impl BlockRewardsEventHandler<TestAccountId, Balance> for RewardEvents {
	fn rewards_created(payout: &[BlockPayout<TestAccountId, Balance>]) {
		LastRewards::set(payout.to_vec());
	}
}
parameter_types! {
	pub static BitcoinPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(62000.00));
	pub static ArgonPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(1.00));
	pub static ArgonCPI: argon_primitives::ArgonCPI = FixedI128::from_float(0.00);
	pub static ActiveCohorts: u32 = 10;
	pub static UniswapLiquidity: Balance = 1_000_000;
	pub static BlockRewardsDampener: FixedU128 = FixedU128::from_float(0.8);
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	fn get_argon_cpi() -> Option<argon_primitives::ArgonCPI> {
		Some(ArgonCPI::get())
	}
	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		ArgonPricePerUsd::get()
	}
	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		BitcoinPricePerUsd::get()
	}
	fn get_argon_pool_liquidity() -> Option<Balance> {
		Some(UniswapLiquidity::get())
	}
	fn get_redemption_r_value() -> Option<FixedU128> {
		None
	}
}

impl pallet_block_rewards::Config for Test {
	type WeightInfo = ();
	type ArgonCurrency = Balances;
	type OwnershipCurrency = Ownership;
	type StartingArgonsPerBlock = StartingArgonsPerBlock;
	type StartingOwnershipTokensPerBlock = StartingOwnershipTokensPerBlock;
	type Balance = Balance;
	type IncrementalGrowth = IncrementalGrowth;
	type HalvingTicks = HalvingBlocks;
	type HalvingBeginTick = HalvingBeginBlock;
	type MinerPayoutPercent = MinerPayoutPercent;
	type BlockSealerProvider = StaticBlockSealerProvider;
	type NotaryProvider = TestProvider;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type TickProvider = StaticTickProvider;
	type NotebookProvider = TestProvider;
	type EventHandler = (RewardEvents,);
	type BlockRewardAccountsProvider = StaticBlockRewardAccountsProvider;
	type PriceProvider = StaticPriceProvider;
	type CohortBlockRewardsToKeep = ActiveCohorts;
	type PayoutHistoryBlocks = ConstU32<5>;
	type SlotWindowTicks = ConstU64<14_400>;
	type PerBlockArgonReducerPercent = BlockRewardsDampener;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
