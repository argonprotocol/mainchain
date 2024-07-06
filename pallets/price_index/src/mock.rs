use crate as pallet_price_index;
use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types};
use sp_arithmetic::FixedU128;
use sp_runtime::{traits::IdentityLookup, BuildStorage};
use ulx_primitives::tick::Tick;

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

parameter_types! {
	pub const MaxDowntimeBeforeReset: Tick = 60; // 1 hour
	pub static MaxPriceAgeInTicks: Tick = 1440; // 1 day
	pub static CurrentTick:Tick = 0;
	pub const MaxArgonChangePerTickAwayFromTarget: FixedU128 = FixedU128::from_rational(1, 100);
	pub const MaxArgonTargetChangePerTick: FixedU128 = FixedU128::from_rational(1, 100);
}

impl pallet_price_index::Config for Test {
	type RuntimeEvent = RuntimeEvent;

	type CurrentTick = CurrentTick;
	type WeightInfo = ();
	type Balance = u128;
	type MaxDowntimeTicksBeforeReset = MaxDowntimeBeforeReset;
	type MaxPriceAgeInTicks = MaxPriceAgeInTicks;
	type MaxArgonChangePerTickAwayFromTarget = MaxArgonChangePerTickAwayFromTarget;
	type MaxArgonTargetChangePerTick = MaxArgonTargetChangePerTick;
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
