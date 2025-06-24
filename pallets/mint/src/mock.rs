use pallet_balances::AccountData;
use pallet_prelude::*;

use crate as pallet_mint;
use argon_primitives::{BlockRewardAccountsProvider, PriceProvider};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Mint: pallet_mint
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = AccountData<Balance>;
}

parameter_types! {
	pub static ExistentialDeposit: Balance = 10;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub static MaxPendingMintUtxos: u32 = 10;
	pub static BitcoinPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(62000.00));
	pub static ArgonPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(1.00));
	pub static ArgonCPI: Option<argon_primitives::ArgonCPI> = Some(FixedI128::from_float(-1.00));
	pub static MinerRewardsAccounts: Vec<(u64, FrameId)> = vec![];
	pub static UniswapLiquidity: Balance = 100_000;
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	fn get_argon_cpi() -> Option<argon_primitives::ArgonCPI> {
		ArgonCPI::get()
	}
	fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
		ArgonPricePerUsd::get()
	}
	fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
		BitcoinPricePerUsd::get()
	}
	fn get_argon_pool_liquidity() -> Option<Balance> {
		Some(UniswapLiquidity::get())
	}
	fn get_redemption_r_value() -> Option<FixedU128> {
		None
	}
}

pub struct StaticBlockRewardAccountsProvider;
impl BlockRewardAccountsProvider<u64> for StaticBlockRewardAccountsProvider {
	fn get_block_rewards_account(_author: &u64) -> Option<(u64, FrameId)> {
		todo!("not used by mint")
	}

	fn get_mint_rewards_accounts() -> Vec<(u64, FrameId)> {
		MinerRewardsAccounts::get()
	}
	fn is_compute_block_eligible_for_rewards() -> bool {
		todo!()
	}
}

impl pallet_mint::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type MaxPendingMintUtxos = MaxPendingMintUtxos;
	type PriceProvider = StaticPriceProvider;
	type BlockRewardAccountsProvider = StaticBlockRewardAccountsProvider;
	type MaxMintHistoryToMaintain = ConstU32<10>;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
