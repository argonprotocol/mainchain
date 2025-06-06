use argon_primitives::{TickProvider, VotingSchedule, tick::Ticker};
use pallet_prelude::*;

use frame_support::traits::ConstU16;

use crate as pallet_notaries;

pub(crate) type Block = frame_system::mocking::MockBlockU32<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Notaries: pallet_notaries
	}
);

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Nonce = u64;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type SS58Prefix = ConstU16<42>;
}

parameter_types! {
	pub static MaxProposalHoldBlocks: u32 = 10;
	pub static MaxActiveNotaries: u32 = 2;
	pub static MaxProposalsPerBlock:u32 = 1;
	pub static MaxNotaryHosts:u32 = 1;
	pub static MaxTicksForKeyHistory:u32 = 10;
	pub static CurrentTick: Tick = 1;
}

pub struct StaticTickProvider;
impl TickProvider<Block> for StaticTickProvider {
	fn previous_tick() -> Tick {
		todo!()
	}
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn voting_schedule() -> VotingSchedule {
		todo!()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 2)
	}
	fn elapsed_ticks() -> Tick {
		CurrentTick::get()
	}
	fn blocks_at_tick(_: Tick) -> Vec<H256> {
		todo!()
	}
}

impl pallet_notaries::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxProposalHoldBlocks = MaxProposalHoldBlocks;
	type MaxActiveNotaries = MaxActiveNotaries;
	type MaxProposalsPerBlock = MaxProposalsPerBlock;
	type MetaChangesTickDelay = ConstU64<1>;
	type MaxNotaryHosts = MaxNotaryHosts;
	type MaxTicksForKeyHistory = MaxTicksForKeyHistory;
	type TickProvider = StaticTickProvider;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
