use std::collections::BTreeMap;

use env_logger::{Builder, Env};
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64},
};
use sp_core::{H256, U256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

use ulx_primitives::{
	block_seal::MiningAuthority,
	block_vote::VoteMinimum,
	inherents::BlockSealInherent,
	notebook::NotebookNumber,
	tick::{Tick, Ticker},
	AuthorityProvider, BlockSealAuthorityId, BlockVotingProvider, NotaryId, NotebookProvider,
	NotebookSecret, TickProvider,
};

use crate as pallet_block_seal_spec;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		SealMinimums: pallet_block_seal_spec,
		Timestamp: pallet_timestamp,
	}
);

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type RuntimeTask = ();
}

parameter_types! {
	pub const TargetBlockVotes: u64 = 100;
	pub const ChangePeriod: u32 = 10;
	pub static AuthorityList: Vec<(u64, BlockSealAuthorityId)> = vec![];
	pub static XorClosest: Option<MiningAuthority<BlockSealAuthorityId, u64>> = None;
	pub static VotingRoots: BTreeMap<(NotaryId, Tick), (H256, NotebookNumber)> = BTreeMap::new();
	pub static ParentVotingKey: Option<H256> = None;
	pub static GrandpaVoteMinimum: Option<VoteMinimum> = None;
	pub static MinerZero: Option<(u64, MiningAuthority<BlockSealAuthorityId, u64>)> = None;
	pub static MiningSlotsInitiatingTaxProof: u32 = 10;
	pub static CurrentSeal: BlockSealInherent = BlockSealInherent::Compute;
	pub static TargetComputeBlockTime: u64 = 100;
	pub const MaxNotaries: u32 = 100;
	pub static LockedNotaries: Vec<(NotaryId, Tick)> = vec![];

	pub static CurrentTick: Tick = 0;
}

pub struct StaticAuthorityProvider;
impl AuthorityProvider<BlockSealAuthorityId, Block, u64> for StaticAuthorityProvider {
	fn get_authority(author: u64) -> Option<BlockSealAuthorityId> {
		AuthorityList::get().iter().find_map(|(account, id)| {
			if *account == author {
				Some(id.clone())
			} else {
				None
			}
		})
	}
	fn get_rewards_account(author: u64) -> Option<u64> {
		Some(author)
	}
	fn xor_closest_authority(_: U256) -> Option<MiningAuthority<BlockSealAuthorityId, u64>> {
		XorClosest::get().clone()
	}
}

pub struct StaticBlockVotingProvider;
impl BlockVotingProvider<Block> for StaticBlockVotingProvider {
	fn grandparent_vote_minimum() -> Option<VoteMinimum> {
		GrandpaVoteMinimum::get()
	}
}
pub struct StaticNotebookProvider;

impl NotebookProvider for StaticNotebookProvider {
	fn get_eligible_tick_votes_root(
		notary_id: NotaryId,
		tick: Tick,
	) -> Option<(H256, NotebookNumber)> {
		VotingRoots::get().get(&(notary_id, tick)).cloned()
	}
	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		todo!()
	}
	fn notebooks_at_tick(_tick: Tick) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		todo!()
	}
	fn is_notary_locked_at_tick(notary_id: NotaryId, tick: Tick) -> bool {
		LockedNotaries::get().contains(&(notary_id, tick))
	}
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 1)
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		vec![]
	}
}

impl pallet_block_seal_spec::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type TargetBlockVotes = TargetBlockVotes;
	type ChangePeriod = ChangePeriod;
	type AuthorityProvider = StaticAuthorityProvider;
	type NotebookProvider = StaticNotebookProvider;
	type SealInherent = CurrentSeal;
	type TargetComputeBlockTime = TargetComputeBlockTime;
	type TickProvider = StaticTickProvider;
	type MaxActiveNotaries = MaxNotaries;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(
	initial_vote_minimum: u128,
	initial_compute_difficulty: u128,
) -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();

	pallet_block_seal_spec::GenesisConfig::<Test> {
		initial_vote_minimum,
		initial_compute_difficulty,
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::new(t)
}
