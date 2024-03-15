use env_logger::{Builder, Env};
use frame_support::{parameter_types, traits::ConstU16};
use sp_core::{ConstU32, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

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
	type BlockHashCount = ConstU32<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type RuntimeTask = ();
}

parameter_types! {
	pub static MaxProposalHoldBlocks: u32 = 10;
	pub static MaxActiveNotaries: u32 = 2;
	pub static MaxProposalsPerBlock:u32 = 1;
	pub static MaxNotaryHosts:u32 = 1;
	pub static MaxBlocksForKeyHistory:u32 = 10;
}

impl pallet_notaries::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxProposalHoldBlocks = MaxProposalHoldBlocks;
	type MaxActiveNotaries = MaxActiveNotaries;
	type MaxProposalsPerBlock = MaxProposalsPerBlock;
	type MetaChangesBlockDelay = ConstU32<1>;
	type MaxNotaryHosts = MaxNotaryHosts;
	type MaxBlocksForKeyHistory = MaxBlocksForKeyHistory;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
