use crate as pallet_ticks;
use argon_primitives::tick::Ticker;
use env_logger::{Builder, Env};
use frame_support::{derive_impl, traits::ConstU64};
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Timestamp: pallet_timestamp,
		Ticks: pallet_ticks,
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
}

impl pallet_ticks::Config for Test {
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(tick_duration_millis: u64, genesis_utc_time: u64) -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_ticks::GenesisConfig::<Test> {
		ticker: Ticker::new(tick_duration_millis, genesis_utc_time, 2),
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::new(t)
}
