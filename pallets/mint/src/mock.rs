use pallet_balances::AccountData;
use pallet_prelude::*;

use crate as pallet_mint;
use argon_primitives::{BlockRewardAccountsProvider, PriceProvider};
use pallet_prelude::argon_primitives::MiningFrameProvider;

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
	pub static AverageCPI: argon_primitives::ArgonCPI = FixedI128::from_float(-1.00);
	pub static MinerRewardsAccounts: Vec<(u64, FrameId)> = vec![];
	pub static ArgonCirculation: Balance = 100_000;
	pub static IsNewFrameStart: Option<u64> = None;
	pub static MaxMiners: u32 = 1000;
	pub static NextMiningTick: Tick = 10;
	pub static MiningFrameTicks: (Tick, Tick) = (0, 10);
}

pub struct StaticMiningFrameProvider;
impl MiningFrameProvider for StaticMiningFrameProvider {
	fn get_next_frame_tick() -> Tick {
		NextMiningTick::get()
	}

	fn is_new_frame_started() -> Option<FrameId> {
		IsNewFrameStart::get()
	}

	fn is_seat_bidding_started() -> bool {
		true
	}
	fn get_tick_range_for_frame(_frame_id: FrameId) -> Option<(Tick, Tick)> {
		Some(MiningFrameTicks::get())
	}
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
	fn get_redemption_r_value() -> Option<FixedU128> {
		None
	}
	fn get_circulation() -> Balance {
		ArgonCirculation::get()
	}
	fn get_average_cpi_for_ticks(_tick_range: (Tick, Tick)) -> argon_primitives::ArgonCPI {
		AverageCPI::get()
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
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type MaxPendingMintUtxos = MaxPendingMintUtxos;
	type PriceProvider = StaticPriceProvider;
	type BlockRewardAccountsProvider = StaticBlockRewardAccountsProvider;
	type MaxMintHistoryToMaintain = ConstU32<10>;
	type MaxPossibleMiners = MaxMiners;
	type MiningFrameProvider = StaticMiningFrameProvider;
}

pub fn new_test_ext() -> TestState {
	new_test_with_genesis::<Test>(|_t| {})
}
