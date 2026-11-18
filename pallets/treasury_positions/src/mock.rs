use super::*;
use alloc::collections::BTreeMap;
use argon_primitives::block_seal::FrameId;
use polkadot_sdk::pallet_balances;
use sp_runtime::traits::ConstU128;

frame_support::construct_runtime!(pub enum Test {
	System: frame_system,
	Balances: pallet_balances,
	TreasuryPositions: crate,
});
#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = frame_system::mocking::MockBlock<Test>;
	type AccountData = pallet_balances::AccountData<u128>;
}
#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type AccountStore = System;
	type ExistentialDeposit = ConstU128<10>;
}
impl Config for Test {
	type BitcoinPositionProvider = ();
	type TreasuryPoolProvider = ();
	type StabilizerPoolPositionProvider = MockStabilizerPools;
	type MiningFrameTransitionProvider = MockFrameProvider;
	type Currency = Balances;
	type OperationalAccountProvider = MockOperationalAccounts;
	type Balance = u128;
	type WeightInfo = ();
}

frame_support::parameter_types! {
	pub static AfterFrameStart: bool = false;
	pub static ProvedThrough: BTreeMap<u64, [FrameId; 2]> = BTreeMap::new();
	pub storage CollectedThrough: BTreeMap<u64, [FrameId; 2]> = BTreeMap::new();
}

pub struct MockFrameProvider;
impl MiningFrameTransitionProvider for MockFrameProvider {
	type Weights = ();
	fn is_first_block_after_frame_start() -> bool {
		AfterFrameStart::get()
	}
	fn get_current_frame_id() -> FrameId {
		14
	}
	fn is_new_frame_started() -> Option<FrameId> {
		None
	}
}

pub struct MockStabilizerPools;
impl StabilizerPoolPositionProvider<u64> for MockStabilizerPools {
	type Weights = ();

	fn settle_account(account: &u64) -> Result<u128, DispatchError> {
		let proved = ProvedThrough::get().get(account).copied().unwrap_or_default();
		let mut cursors = CollectedThrough::get();
		let collected = cursors.entry(*account).or_default();
		let mut frames_to_pay = 0u64;
		for (last, eligible) in collected.iter_mut().zip(proved) {
			frames_to_pay += eligible.saturating_sub(*last);
			*last = eligible;
		}
		CollectedThrough::set(&cursors);
		Ok(u128::from(frames_to_pay) * 7)
	}
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
