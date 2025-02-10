use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types, traits::StorageMapShim};
use sp_core::{ConstU32, H256};
use sp_runtime::{traits::IdentityLookup, BuildStorage, FixedU128};

use crate as pallet_block_rewards;
use crate::GrowthPath;
use argon_primitives::{
	block_seal::RewardSharing,
	notary::{NotaryProvider, NotarySignature},
	tick::{Tick, Ticker},
	BlockRewardAccountsProvider, BlockSealerInfo, BlockSealerProvider, NotaryId, NotebookNumber,
	NotebookProvider, NotebookSecret, RewardShare, TickProvider, VotingSchedule,
};

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type AccountId = u64;

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
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

pub type Balance = u128;
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
		AccountId,
		pallet_balances::AccountData<Balance>,
	>;
	type WeightInfo = ();
	type FreezeIdentifier = RuntimeFreezeReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = RuntimeHoldReason;
}

parameter_types! {
	pub static ArgonsPerBlock :u32 = 5_000;
	pub static StartingOwnershipTokensPerBlock :u32 = 5_000;
	pub static IncrementalGrowth :GrowthPath<Balance> = (1_000, 100, 500_000);
	pub static HalvingBeginBlock: u32 = 1000;
	pub static HalvingBlocks :u32 = 100;
	pub static MaturationBlocks :u32 = 5;
	pub static MinerPayoutPercent :FixedU128 = FixedU128::from_rational(75, 100);
	pub static ActiveNotaries: Vec<NotaryId> = vec![1];
	pub static NotebookTick: Tick = 0;
	pub static ElapsedTicks: Tick = 0;

	pub static GetRewardSharing: Option<RewardSharing<u64>> = None;
	pub static NotebooksInBlock: Vec<(NotaryId, NotebookNumber, Tick)> = vec![];

	pub static BlockSealer:BlockSealerInfo<u64> = BlockSealerInfo {
		block_vote_rewards_account: Some(1),
		block_author_account_id: 1,
		block_seal_authority: None
	};
}

pub struct StaticBlockSealerProvider;
impl BlockSealerProvider<u64> for StaticBlockSealerProvider {
	fn get_sealer_info() -> BlockSealerInfo<u64> {
		BlockSealer::get()
	}
}

pub struct TestProvider;
impl NotaryProvider<Block, AccountId> for TestProvider {
	fn verify_signature(_: NotaryId, _: Tick, _: &H256, _: &NotarySignature) -> bool {
		true
	}
	fn active_notaries() -> Vec<NotaryId> {
		ActiveNotaries::get()
	}
	fn notary_operator_account_id(_notary_id: NotaryId) -> Option<AccountId> {
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
	fn get_rewards_account(author: &u64) -> (Option<u64>, Option<RewardSharing<u64>>) {
		let res = GetRewardSharing::get();
		if let Some(delegate) = res {
			(Some(*author), Some(delegate))
		} else {
			(None, None)
		}
	}

	fn get_all_rewards_accounts() -> Vec<(u64, Option<RewardShare>)> {
		todo!("not used by rewards")
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

impl pallet_block_rewards::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type ArgonCurrency = Balances;
	type OwnershipCurrency = Ownership;
	type StartingArgonsPerBlock = ArgonsPerBlock;
	type StartingOwnershipTokensPerBlock = StartingOwnershipTokensPerBlock;
	type MaturationBlocks = MaturationBlocks;
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
	type EventHandler = ();
	type BlockRewardAccountsProvider = StaticBlockRewardAccountsProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
