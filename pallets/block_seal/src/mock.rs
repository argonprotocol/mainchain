use std::collections::{BTreeMap, BTreeSet};

use crate as pallet_block_seal;
use argon_notary_audit::VerifyError;
use argon_primitives::{
	block_seal::{BlockSealAuthorityId, MiningAuthority},
	block_vote::VoteMinimum,
	digests::Digestset,
	notebook::NotebookNumber,
	tick::{Tick, Ticker},
	AuthorityProvider, BlockSealSpecProvider, BlockVoteDigest, ComputeDifficulty, DomainHash,
	HashOutput, NotaryId, NotebookAuditResult, NotebookDigest, NotebookProvider, NotebookSecret,
	TickDigest, TickProvider, VotingSchedule,
};
use env_logger::{Builder, Env};
use frame_support::{__private::Get, derive_impl, parameter_types};
use sp_core::{H256, U256};
use sp_runtime::{traits::Block as BlockT, BuildStorage, DispatchError};

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
	pub static BlocksAtTick: BTreeMap<Tick, HashOutput> = BTreeMap::new();
	pub static RegisteredDomains: BTreeSet<DomainHash> = BTreeSet::new();

	pub static Digests: Digestset<VerifyError, u64> = Digestset {
		block_vote: BlockVoteDigest { voting_power: 500, votes_count: 1 },
		author: 1,
		voting_key: None,
		tick: TickDigest { tick: 2 },
		notebooks: NotebookDigest {
			notebooks: vec![NotebookAuditResult::<VerifyError> {
				notary_id: 1,
				notebook_number: 1,
				tick: 1,
				audit_first_failure: None,
			}],
		},
	};
}

pub struct DigestGetter;
impl Get<Result<Digestset<VerifyError, u64>, DispatchError>> for DigestGetter {
	fn get() -> Result<Digestset<VerifyError, u64>, DispatchError> {
		Ok(Digests::get())
	}
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
		let mut res = vec![];
		for (tick, notebooks) in NotebooksAtTick::get() {
			for (notary_id, notebook_number, _) in notebooks {
				res.push((notary_id, notebook_number, tick));
			}
		}
		res
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
	fn block_at_tick(tick: Tick) -> Option<HashOutput> {
		BlocksAtTick::get().get(&tick).cloned()
	}
	fn voting_schedule() -> VotingSchedule {
		VotingSchedule::from_runtime_current_tick(CurrentTick::get())
	}
}

pub struct StaticBlockSealSpecProvider;
impl BlockSealSpecProvider<Block> for StaticBlockSealSpecProvider {
	fn grandparent_vote_minimum() -> Option<VoteMinimum> {
		GrandpaVoteMinimum::get()
	}
	fn compute_difficulty() -> ComputeDifficulty {
		0
	}
	fn compute_key_block_hash() -> Option<<Block as BlockT>::Hash> {
		todo!()
	}
}
impl pallet_block_seal::Config for Test {
	type WeightInfo = ();
	type AuthorityId = BlockSealAuthorityId;
	type AuthorityProvider = StaticAuthorityProvider;
	type NotebookProvider = StaticNotebookProvider;
	type BlockSealSpecProvider = StaticBlockSealSpecProvider;
	type TickProvider = StaticTickProvider;
	type EventHandler = ();
	type Digests = DigestGetter;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	sp_io::TestExternalities::new(t)
}
