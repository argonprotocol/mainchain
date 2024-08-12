use std::collections::{BTreeMap, BTreeSet};

use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types};
use sp_core::{H256, U256};
use sp_runtime::BuildStorage;

use crate as pallet_block_seal;
use argon_primitives::{
	block_seal::{BlockSealAuthorityId, MiningAuthority},
	block_vote::VoteMinimum,
	notebook::NotebookNumber,
	tick::{Tick, Ticker},
	AuthorityProvider, BlockVotingProvider, DataDomainHash, DataDomainProvider, HashOutput,
	NotaryId, NotebookProvider, NotebookSecret, TickProvider,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		BlockSeal: pallet_block_seal,
	}
);
#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

parameter_types! {
	pub static AuthorityList: Vec<(u64, BlockSealAuthorityId)> = vec![];
	pub static XorClosest: Option<MiningAuthority<BlockSealAuthorityId, u64>> = None;
	pub static VotingRoots: BTreeMap<(NotaryId, Tick), (H256, NotebookNumber)> = BTreeMap::new();
	pub static GrandpaVoteMinimum: Option<VoteMinimum> = None;
	pub static MinerZero: Option<(u64, MiningAuthority<BlockSealAuthorityId, u64>)> = None;
	pub static NotebooksAtTick: BTreeMap<Tick, Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)>> = BTreeMap::new();
	pub static CurrentTick: Tick = 0;
	pub static BlocksAtTick: BTreeMap<Tick, Vec<HashOutput>> = BTreeMap::new();
	pub static RegisteredDataDomains: BTreeSet<DataDomainHash> = BTreeSet::new();
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
	fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
		todo!()
	}
	fn notebooks_at_tick(tick: Tick) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
		NotebooksAtTick::get().get(&tick).cloned().unwrap_or_default()
	}
	fn is_notary_locked_at_tick(_: NotaryId, _: Tick) -> bool {
		false
	}
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 1, 2)
	}
	fn blocks_at_tick(tick: Tick) -> Vec<HashOutput> {
		BlocksAtTick::get().get(&tick).cloned().unwrap_or_default()
	}
}

pub struct StaticBlockVotingProvider;
impl BlockVotingProvider<Block> for StaticBlockVotingProvider {
	fn grandparent_vote_minimum() -> Option<VoteMinimum> {
		GrandpaVoteMinimum::get()
	}
}
pub struct StaticDataDomainProvider;
impl DataDomainProvider<u64> for StaticDataDomainProvider {
	fn is_registered_payment_account(
		data_domain_hash: &DataDomainHash,
		_account_id: &u64,
		_tick_range: (Tick, Tick),
	) -> bool {
		RegisteredDataDomains::get().contains(data_domain_hash)
	}
}
impl pallet_block_seal::Config for Test {
	type WeightInfo = ();
	type AuthorityId = BlockSealAuthorityId;
	type AuthorityProvider = StaticAuthorityProvider;
	type NotebookProvider = StaticNotebookProvider;
	type BlockVotingProvider = StaticBlockVotingProvider;
	type TickProvider = StaticTickProvider;
	type DataDomainProvider = StaticDataDomainProvider;
	type EventHandler = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	sp_io::TestExternalities::new(t)
}
