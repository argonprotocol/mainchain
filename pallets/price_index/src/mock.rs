use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types, traits::Time};
use sp_runtime::{traits::IdentityLookup, BuildStorage};
use std::time::SystemTime;

use crate as pallet_price_index;

pub(crate) type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		PriceIndex: pallet_price_index
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = ();
}

type Moment = u64;

parameter_types! {
	pub const MaxDowntimeBeforeReset: Moment = 60 * 60 * 1000; // 1 hour
	pub static OldestHistoryToKeep: Moment = 24 * 60 * 60 * 1000; // 1 day
	pub static Now: Moment = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as Moment;
}

pub struct Timestamp;

impl Time for Timestamp {
	type Moment = Moment;

	fn now() -> Self::Moment {
		Now::get()
	}
}

impl pallet_price_index::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Time = Timestamp;
	type WeightInfo = ();
	type Balance = u128;
	type MaxDowntimeBeforeReset = MaxDowntimeBeforeReset;
	type OldestPriceAllowed = OldestHistoryToKeep;
}

pub fn new_test_ext(operator: Option<u64>) -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_price_index::GenesisConfig::<Test> { operator }
		.assimilate_storage(&mut t)
		.unwrap();

	sp_io::TestExternalities::new(t)
}
