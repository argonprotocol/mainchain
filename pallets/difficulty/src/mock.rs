use crate as pallet_difficulty;
use env_logger::{Builder, Env};
use frame_support::traits::{ConstU16, ConstU64};
use sp_core::{ConstU32, H256};
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
		DifficultyModule: pallet_difficulty,
		Timestamp: pallet_timestamp,
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

impl pallet_difficulty::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type TargetBlockTime = ConstU64<100>;
	type BlockChangePeriod = ConstU32<10>;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(start_difficulty: u64) -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();

	pallet_difficulty::GenesisConfig::<Test> {
		initial_difficulty: start_difficulty.into(),
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	sp_io::TestExternalities::new(t)
}
