use std::collections::BTreeMap;

use pallet_prelude::*;

use crate as pallet_block_seal_spec;
use argon_primitives::{
	NotebookSecret, VotingSchedule, block_seal::MiningAuthority, inherents::BlockSealInherent,
	notebook::NotebookNumber, providers::*, tick::Ticker,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		BlockSealSpec: pallet_block_seal_spec,
		Timestamp: pallet_timestamp,
	}
);

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type DbWeight = RocksDbWeight;
}

parameter_types! {
	pub const TargetBlockVotes: u64 = 100;
	pub static AuthorityList: Vec<(u64, BlockSealAuthorityId)> = vec![];
	pub static BestMinerNonce: Option<MiningAuthority<BlockSealAuthorityId, u64>> = None;
	pub static VotingRoots: BTreeMap<(NotaryId, Tick), (H256, NotebookNumber)> = BTreeMap::new();
	pub static ParentVotingKey: Option<H256> = None;
	pub static MinerZero: Option<(u64, MiningAuthority<BlockSealAuthorityId, u64>)> = None;
	pub static MiningSlotsInitiatingTaxProof: u32 = 10;
	pub static CurrentSeal: BlockSealInherent = BlockSealInherent::Compute;
	pub static TargetComputeBlockPercent: Percent = Percent::from_percent(50);
	pub const MaxNotaries: u32 = 100;
	pub static LockedNotaries: Vec<(NotaryId, Tick)> = vec![];
	pub static TickDuration: u64 = 200;
	pub static HistoricalComputeBlocksForAverage: u32 = 10;

	pub static CurrentTick: Tick = 0;
}

pub struct StaticAuthorityProvider;
impl AuthorityProvider<BlockSealAuthorityId, Block, u64> for StaticAuthorityProvider {
	fn authority_count() -> u32 {
		AuthorityList::get().len() as u32
	}
	fn get_authority(author: u64) -> Option<BlockSealAuthorityId> {
		AuthorityList::get()
			.iter()
			.find_map(|(account, id)| if *account == author { Some(id.clone()) } else { None })
	}

	fn get_winning_managed_authority(
		_seal_proof: U256,
		_signing_key: Option<BlockSealAuthorityId>,
		_miner_nonce_score: Option<U256>,
	) -> Option<(MiningAuthority<BlockSealAuthorityId, u64>, U256, Permill)> {
		todo!()
	}
	fn get_authority_score(
		_seal_proof: U256,
		_authority_id: &BlockSealAuthorityId,
		_account: &u64,
	) -> Option<U256> {
		todo!()
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
	fn previous_tick() -> Tick {
		todo!()
	}
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn elapsed_ticks() -> Tick {
		CurrentTick::get()
	}
	fn ticker() -> Ticker {
		Ticker::new(TickDuration::get(), 2)
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		vec![]
	}
	fn voting_schedule() -> VotingSchedule {
		VotingSchedule::from_runtime_current_tick(CurrentTick::get())
	}
}

impl pallet_block_seal_spec::Config for Test {
	type WeightInfo = ();
	type TargetBlockVotes = TargetBlockVotes;
	type AuthorityProvider = StaticAuthorityProvider;
	type NotebookProvider = StaticNotebookProvider;
	type SealInherent = CurrentSeal;
	type HistoricalVoteBlocksForAverage = ConstU32<10>;
	type HistoricalComputeBlocksForAverage = HistoricalComputeBlocksForAverage;
	type TargetComputeBlockPercent = TargetComputeBlockPercent;
	type TickProvider = StaticTickProvider;
	type MaxActiveNotaries = MaxNotaries;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(initial_vote_minimum: u128, initial_compute_difficulty: u128) -> TestState {
	new_test_with_genesis::<Test>(|t: &mut Storage| {
		pallet_block_seal_spec::GenesisConfig::<Test> {
			initial_vote_minimum,
			initial_compute_difficulty,
			_phantom: Default::default(),
		}
		.assimilate_storage(t)
		.unwrap();
	})
}
