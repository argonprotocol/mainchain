use env_logger::{Builder, Env};
use frame_support::{derive_impl, parameter_types};
use pallet_balances::AccountData;
use sp_arithmetic::{FixedI128, FixedU128};
use sp_runtime::BuildStorage;

use crate as pallet_mint;
use ulx_primitives::{
	block_seal::RewardSharing, BlockRewardAccountsProvider, PriceProvider, RewardShare,
};

pub type Balance = u128;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		UlixeeMint: pallet_mint
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
}

parameter_types! {
	pub static MaxPendingMintUtxos: u32 = 10;
	pub static BitcoinPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(62000.00));
	pub static ArgonPricePerUsd: Option<FixedU128> = Some(FixedU128::from_float(1.00));
	pub static ArgonCPI: Option<ulx_primitives::ArgonCPI> = Some(FixedI128::from_float(-1.00));
	pub static MinerRewardsAccounts: Vec<(u64, Option<RewardShare>)> = vec![];
}

pub struct StaticPriceProvider;
impl PriceProvider<Balance> for StaticPriceProvider {
	fn get_argon_cpi() -> Option<ulx_primitives::ArgonCPI> {
		ArgonCPI::get()
	}
	fn get_latest_argon_price_in_us_cents() -> Option<FixedU128> {
		ArgonPricePerUsd::get()
	}
	fn get_latest_btc_price_in_us_cents() -> Option<FixedU128> {
		BitcoinPricePerUsd::get()
	}
}

pub struct StaticBlockRewardAccountsProvider;
impl BlockRewardAccountsProvider<u64> for StaticBlockRewardAccountsProvider {
	fn get_rewards_account(_author: &u64) -> (Option<u64>, Option<RewardSharing<u64>>) {
		todo!("not used by mint")
	}

	fn get_all_rewards_accounts() -> Vec<(u64, Option<RewardShare>)> {
		MinerRewardsAccounts::get()
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
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let env = Env::new().default_filter_or("debug");
	let _ = Builder::from_env(env).is_test(true).try_init();
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
