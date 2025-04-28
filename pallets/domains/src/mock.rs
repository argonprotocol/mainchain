use pallet_prelude::*;

use crate as pallet_domains;

pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
use sp_runtime::AccountId32;
// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Domain: pallet_domains
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
	pub static NotebookTick: Tick = 0;
	pub static HistoricalPaymentAddressTicksToKeep: u32 = 100;
}

impl pallet_domains::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type NotebookTick = NotebookTick;
	type DomainExpirationTicks = DomainExpirationTicks;
	type HistoricalPaymentAddressTicksToKeep = HistoricalPaymentAddressTicksToKeep;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
