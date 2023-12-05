use std::collections::BTreeMap;

use env_logger::{Builder, Env};
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64},
};
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

use ulx_primitives::{
	block_seal::{BlockSealAuthorityId, Host, MiningAuthority, VoteMinimum},
	notebook::NotebookNumber,
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
	pub static XorClosest: Option<MiningAuthority<BlockSealAuthorityId>> = None;
	pub static VotingRoots: BTreeMap<(NotaryId, u32), (H256, NotebookNumber)> = BTreeMap::new();
	pub static ParentVotingKey: Option<H256> = None;
	pub static GrandpaVoteMinimum: Option<VoteMinimum> = None;
	pub static MinerZero: Option<(u64, MiningAuthority<BlockSealAuthorityId>)> = None;
}

pub struct StaticAuthorityProvider;
impl AuthorityProvider<BlockSealAuthorityId, Block, u64> for StaticAuthorityProvider {
	fn miner_zero() -> Option<(u16, BlockSealAuthorityId, Vec<Host>, u64)> {
		MinerZero::get().map(|(account_id, auth)| {
			(auth.authority_index, auth.authority_id, auth.rpc_hosts.into_inner(), account_id)
		})
	}
	fn authorities() -> Vec<BlockSealAuthorityId> {
		AuthorityList::get().iter().map(|(_account, id)| id.clone()).collect()
	}
	fn authority_id_by_index() -> BTreeMap<u16, BlockSealAuthorityId> {
		let mut map = BTreeMap::new();
		for (i, id) in AuthorityList::get().into_iter().enumerate() {
			map.insert(i as u16, id.1);
		}
		map
	}
	fn authority_count() -> u16 {
		AuthorityList::get().len() as u16
	}
	fn is_active(authority_id: &BlockSealAuthorityId) -> bool {
		Self::authorities().contains(authority_id)
	}
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
	fn block_peer(
		_block_hash: &<Block as sp_runtime::traits::Block>::Hash,
		_account_id: &AccountId32,
	) -> Option<MiningAuthority<BlockSealAuthorityId>> {
		XorClosest::get().clone()
	}
}

pub struct StaticNotebookProvider;
impl NotebookProvider for StaticNotebookProvider {
	fn get_eligible_block_votes_root(
		notary_id: NotaryId,
		block_number: u32,
	) -> Option<(H256, NotebookNumber)> {
		VotingRoots::get().get(&(notary_id, block_number)).cloned()
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
	type RuntimeEvent = RuntimeEvent;
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
