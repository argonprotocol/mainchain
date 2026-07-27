use crate as pallet_bootstrap;
use pallet_prelude::*;
use polkadot_sdk::sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::AccountId32;

pub(crate) type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Bootstrap: pallet_bootstrap,
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
	pub const MaxEncryptedPayloadLen: u32 = 256;
}

impl pallet_bootstrap::Config for Test {
	type MaxEncryptedPayloadLen = MaxEncryptedPayloadLen;
	type WeightInfo = ();
}

pub fn new_test_ext() -> TestState {
	let mut ext = new_test_with_genesis::<Test>(|_| {});
	ext.register_extension(KeystoreExt::new(MemoryKeystore::new()));
	ext
}
