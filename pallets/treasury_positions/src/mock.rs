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
	type EarningsSettlementProvider = MockStabilizerPools;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type VaultEarningsProvider = ();
	#[cfg(feature = "runtime-benchmarks")]
	type VaultEarningsProvider = pallet_prelude::benchmarking::BenchmarkVaultEarningsProvider;
	type PalletId = TreasuryPositionsPalletId;
	type MiningFrameTransitionProvider = CurrentFrame;
	type MaxEarningChangesPerCall = ConstU32<32>;
	type Currency = Balances;
	type OperationalAccountProvider = MockOperationalAccounts;
	type Balance = u128;
	type WeightInfo = ();
}

frame_support::parameter_types! {
	pub const TreasuryPositionsPalletId: frame_support::PalletId = frame_support::PalletId(*b"arg/colt");
	pub static CurrentFrame: FrameId = 1;
	pub static AfterFrameStart: bool = false;
	pub static ProvedThrough: BTreeMap<u64, [FrameId; 2]> = BTreeMap::new();
	pub storage CollectedThrough: BTreeMap<u64, [FrameId; 2]> = BTreeMap::new();
}

pub struct MockStabilizerPools;
impl EarningsSettlementProvider<u64> for MockStabilizerPools {
	type Weights = ();

	fn settle_account(account: &u64) -> DispatchResult {
		let proved = ProvedThrough::get().get(account).copied().unwrap_or_default();
		let mut cursors = CollectedThrough::get();
		let collected = cursors.entry(*account).or_default();
		let mut frames_to_pay = 0u64;
		for (last, eligible) in collected.iter_mut().zip(proved) {
			frames_to_pay += eligible.saturating_sub(*last);
			*last = eligible;
		}
		CollectedThrough::set(&cursors);
		let amount = u128::from(frames_to_pay) * 7;
		TreasuryPositions::fund(
			argon_primitives::treasury::CollectSource::StabilizerPools,
			amount,
		)?;
		TreasuryPositions::accrue(
			account,
			argon_primitives::treasury::CollectSource::StabilizerPools,
			amount,
		)?;
		Ok(())
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

pub fn new_test_ext() -> polkadot_sdk::sp_io::TestExternalities {
	CurrentFrame::set(1);
	ProvedThrough::set(BTreeMap::new());
	use sp_runtime::BuildStorage;
	RuntimeGenesisConfig::default().build_storage().unwrap().into()
}

impl argon_primitives::MiningFrameTransitionProvider for CurrentFrame {
	type Weights = ();
	fn get_current_frame_id() -> FrameId {
		Self::get()
	}
	fn is_new_frame_started() -> Option<FrameId> {
		None
	}
}

pub fn index_frame(
	frame: FrameId,
	quantities: PositionQuantities<u128>,
	amounts: CollectAmounts<u128>,
) {
	assert_ok!(TreasuryPositions::prepare_collect());
	assert_ok!(Balances::mint_into(&TreasuryPositions::account_id(), amounts.total()));
	assert_ok!(TreasuryPositions::allocate_frame_earnings(frame, &quantities, amounts));
}
