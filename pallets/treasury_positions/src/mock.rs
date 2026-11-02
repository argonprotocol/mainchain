use super::*;
frame_support::construct_runtime!(pub enum Test { System: frame_system, TreasuryPositions: crate });
#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = frame_system::mocking::MockBlock<Test>;
}
impl Config for Test {
	type BitcoinPositionProvider = ();
	type TreasuryPoolProvider = ();
	type OperationalAccountProvider = MockOperationalAccounts;
	type Balance = u128;
	type WeightInfo = ();
}

pub struct MockOperationalAccounts;
impl OperationalAccountProvider<u64> for MockOperationalAccounts {
	type Weights = ();
	fn is_eligible(_: &u64) -> bool {
		true
	}
	fn upstream_vault(account: &u64) -> Option<(u64, VaultId)> {
		(*account == 8 || *account == 9).then_some((9, 3))
	}
}
