use env_logger::{Builder, Env};
pub use frame_support::{derive_impl, parameter_types, weights::constants::RocksDbWeight};
use polkadot_sdk::*;

pub use frame_support::{
	assert_err, assert_err_ignore_postinfo, assert_error_encoded_size, assert_noop, assert_ok,
	assert_storage_noop, storage_alias,
};
pub use frame_system::mocking::*;
pub use sp_core::{storage::Storage, Blake2Hasher};
pub use sp_io::TestExternalities as TestState;
pub use sp_keyring::*;
pub use sp_runtime::{traits::IdentityLookup, BuildStorage};

// Build genesis storage according to the mock runtime.
pub fn new_test_with_genesis<Test>(apply: impl FnOnce(&mut Storage)) -> TestState
where
	Test: polkadot_sdk::frame_system::Config,
{
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();

	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	apply(&mut t);
	TestState::new(t)
}
