use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types};
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{traits::IdentityLookup, BuildStorage};

use argon_primitives::{
	tick::{Tick, Ticker},
	TickProvider,
};

use crate as pallet_data_domain;

pub(crate) type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		DataDomain: pallet_data_domain
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = ();
}

parameter_types! {

	pub static DomainExpirationTicks :u32 = 1000;
	pub static CurrentTick: Tick = 0;
	pub static HistoricalPaymentAddressTicksToKeep: u32 = 100;
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

impl pallet_data_domain::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type TickProvider = StaticTickProvider;
	type DomainExpirationTicks = DomainExpirationTicks;
	type HistoricalPaymentAddressTicksToKeep = HistoricalPaymentAddressTicksToKeep;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
