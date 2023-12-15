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
	block_seal::{BlockSealAuthorityId, MiningAuthority},
	block_vote::VoteMinimum,
	notebook::NotebookNumber,
	tick::Tick,
	AuthorityProvider, BlockVotingProvider, NotaryId, NotebookProvider,
};

use crate as pallet_block_seal;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		BlockSeal: pallet_block_seal,
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type RuntimeCall = RuntimeCall;
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
}

parameter_types! {
	pub static AuthorityList: Vec<(u64, BlockSealAuthorityId)> = vec![];
	pub static XorClosest: Option<MiningAuthority<BlockSealAuthorityId, u64>> = None;
	pub static VotingRoots: BTreeMap<(NotaryId, Tick), (H256, NotebookNumber)> = BTreeMap::new();
	pub static ParentVotingKey: Option<H256> = None;
	pub static GrandpaVoteMinimum: Option<VoteMinimum> = None;
	pub static MinerZero: Option<(u64, MiningAuthority<BlockSealAuthorityId, u64>)> = None;
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

pub struct StaticNotebookProvider;
impl NotebookProvider for StaticNotebookProvider {
	fn get_eligible_tick_votes_root(
		notary_id: NotaryId,
		tick: Tick,
	) -> Option<(H256, NotebookNumber)> {
		VotingRoots::get().get(&(notary_id, tick)).cloned()
	}
}

pub struct StaticBlockVotingProvider;
impl BlockVotingProvider<Block> for StaticBlockVotingProvider {
	fn grandparent_vote_minimum() -> Option<VoteMinimum> {
		GrandpaVoteMinimum::get()
	}
	fn parent_voting_key() -> Option<H256> {
		ParentVotingKey::get()
	}
}

impl pallet_block_seal::Config for Test {
	type WeightInfo = ();
	type AuthorityId = BlockSealAuthorityId;
	type AuthorityProvider = StaticAuthorityProvider;
	type NotebookProvider = StaticNotebookProvider;
	type BlockVotingProvider = StaticBlockVotingProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();

	sp_io::TestExternalities::new(t)
}
