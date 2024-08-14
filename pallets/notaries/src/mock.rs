use argon_primitives::{
	tick::{Tick, Ticker},
	TickProvider,
};
use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types, traits::ConstU16};
use sp_core::{ConstU32, H256};
use sp_runtime::{traits::IdentityLookup, BuildStorage};

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
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn ticker() -> Ticker {
		Ticker::new(1, 1, 2)
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
	type MetaChangesTickDelay = ConstU32<1>;
	type MaxNotaryHosts = MaxNotaryHosts;
	type MaxTicksForKeyHistory = MaxTicksForKeyHistory;
	type TickProvider = StaticTickProvider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
