use crate as pallet_validator_cohorts;
use env_logger::{Builder, Env};
use frame_support::{
	parameter_types,
	traits::{ConstU16, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		ValidatorCohorts: pallet_validator_cohorts,
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
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
	pub static BlocksBetweenCohorts: u32 = 1;
	pub static MaxCohortSize: u32 = 5;
	pub static MaxValidators: u32 = 10;
	pub static MaxPendingCohorts: u32 = 2;
}

impl pallet_validator_cohorts::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type BlocksBetweenCohorts = BlocksBetweenCohorts;
	type MaxCohortSize = MaxCohortSize;
	type MaxPendingCohorts = MaxPendingCohorts;
	type MaxValidators = MaxValidators;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
