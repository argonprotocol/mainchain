use crate as pallet_stabilizer_buys;
use crate::*;
use argon_primitives::{
	tick::Ticker, CrosschainGatewayProvider, GatewayState, MiningFrameTransitionProvider,
	RValueHighWaterMark, SourceChain, TickProvider, VotingSchedule,
};
use frame_support::{derive_impl, storage::unhashed};
use pallet_prelude::{new_test_with_genesis, TestState};
use polkadot_sdk::{
	pallet_balances,
	sp_runtime::{
		traits::{ConstU128, ConstU32, ConstU64, ConstU8},
		ArithmeticError, BuildStorage,
	},
};
use std::collections::BTreeMap;

type Block = frame_system::mocking::MockBlock<Test>;
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		StabilizerBuys: pallet_stabilizer_buys,
		PriceIndex: pallet_price_index,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type AccountStore = System;
	type ExistentialDeposit = ConstU128<10_000>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	pub static FrameHighs: BTreeMap<FrameId, RValueHighWaterMark> = BTreeMap::new();
	pub static MaxPriceWindowSteps: u32 = 64;
	pub static LastGatewayState: Option<GatewayState<u128>> = None;
	pub HistoryTicker: Ticker = Ticker::new(1_000, 2);
	pub static CurrentFrame: FrameId = 1;
	pub static CurrentTick: Tick = 1_000;
	pub static Prices: Option<PurchasePriceReference> = Some(PurchasePriceReference {
		r_value: FixedU128::one(), argon_usd_target_price: FixedU128::one(),
		microgons_at_target_per_btc: FixedU128::saturating_from_integer(60_000_000_000u128),
		microgons_at_target_per_usdc: FixedU128::from_u32(1_000_000),
	});
	pub static FailReserve: bool = false;
	pub static FailReturn: bool = false;
	pub static FailCommitAt: Option<Balance> = None;
	pub static FailPaymentFor: Option<u64> = None;
	pub Pool: PurchasePool = PurchasePool {
		chain_id: 1,
		pool_manager: H160::repeat_byte(1),
		pool_id: H256::repeat_byte(2),
		usdc_token: H160::repeat_byte(3),
		argon_is_token0: true,
	};
}

pub struct Frame;
impl MiningFrameTransitionProvider for Frame {
	type Weights = ();

	fn get_current_frame_id() -> FrameId {
		CurrentFrame::get()
	}
	fn is_new_frame_started() -> Option<FrameId> {
		None
	}
}

pub struct Budget;
impl Budget {
	pub fn outstanding_rewards() -> Balance {
		unhashed::get(b"test_treasury_outstanding_rewards").unwrap_or(0)
	}
}

impl TreasuryRewardProvider<u64> for Budget {
	type Balance = Balance;
	type Weights = ();

	fn available_budget() -> Balance {
		unhashed::get(b"test_liquidity_buy_budget").unwrap_or(0)
	}
	fn reserve_budget(amount: Balance) -> DispatchResult {
		let remaining =
			Self::available_budget().checked_sub(amount).ok_or(ArithmeticError::Underflow)?;
		unhashed::put(b"test_liquidity_buy_budget", &remaining);
		ensure!(!FailReserve::get(), DispatchError::Other("reserve failed"));
		Ok(())
	}
	fn commit_reward(amount: Balance) -> DispatchResult {
		let outstanding = Self::outstanding_rewards()
			.checked_add(amount)
			.ok_or(ArithmeticError::Overflow)?;
		unhashed::put(b"test_treasury_outstanding_rewards", &outstanding);
		if let Some(limit) = FailCommitAt::get() {
			ensure!(outstanding < limit, DispatchError::Other("commit failed"));
		}
		Ok(())
	}
	fn return_budget(amount: Balance) -> DispatchResult {
		let available =
			Self::available_budget().checked_add(amount).ok_or(ArithmeticError::Overflow)?;
		unhashed::put(b"test_liquidity_buy_budget", &available);
		ensure!(!FailReturn::get(), DispatchError::Other("return failed"));
		Ok(())
	}
	fn pay_reward(account: &u64, amount: Balance) -> Result<Balance, DispatchError> {
		let outstanding = Self::outstanding_rewards()
			.checked_sub(amount)
			.ok_or(ArithmeticError::Underflow)?;
		unhashed::put(b"test_treasury_outstanding_rewards", &outstanding);
		let paid = match Balances::mint_into(account, amount) {
			Ok(paid) => paid,
			Err(error) if error == TokenError::BelowMinimum.into() => 0,
			Err(error) => return Err(error),
		};
		ensure!(FailPaymentFor::get() != Some(*account), DispatchError::Other("payment failed"));
		Ok(paid)
	}
}

impl Get<FrameId> for Frame {
	fn get() -> FrameId {
		CurrentFrame::get()
	}
}

parameter_types! {
	pub const MaxPriceChange: FixedU128 = FixedU128::from_rational(1,100);
}

impl pallet_price_index::Config for Test {
	type OnPriceIndexUpdated = StabilizerBuys;
	type WeightInfo = ();
	type Currency = Balances;
	type Balance = Balance;
	type MaxDowntimeTicksBeforeReset = polkadot_sdk::sp_runtime::traits::ConstU64<60>;
	type MaxPriceAgeInTicks = polkadot_sdk::sp_runtime::traits::ConstU64<1440>;
	type TickProvider = CurrentTick;
	type CurrentFrameId = Frame;
	type MiningFrameTransitionProvider = Frame;
	type MaxArgonotFloorHistoryFrames = ConstU32<10>;
	type MaxArgonotAverageHistoryFrames = ConstU32<10>;
	type MaxEthereumPriceHistoryFrames = ConstU32<10>;
	type MaxArgonChangePerTickAwayFromTarget = MaxPriceChange;
	type MaxArgonTargetChangePerTick = MaxPriceChange;
}

impl Config for Test {
	type Currency = Balances;
	type FissionsProvider = Repayments;
	type MiningFrameTransitionProvider = Frame;
	type TickProvider = CurrentTick;
	type CrosschainGatewayProvider = Test;
	type MaxPriceAgeTicks = ConstU64<10>;
	type MaxPriceHistoryPrunesPerBlock = ConstU32<16>;
	type MaxPriceWindowSteps = MaxPriceWindowSteps;
	type TreasuryRewardProvider = Budget;
	type MaxFramesPerPayout = ConstU32<16>;
	type MaxPayoutsPerBlock = ConstU32<2>;
	type MaxPositiveBids = ConstU8<4>;
	type AuctionInclusionPercent = InclusionPercent;
	type PurchaseHoldFrames = PurchaseHoldFrames;
	type LiquidRepaymentRewardPercent = RepaymentRewardPercent;
	type MaxLiquidRepaysPerBlock = ConstU32<1>;
	type WeightInfo = ();
}

parameter_types! {
	pub const InclusionPercent: Percent = Percent::from_percent(90);
	pub const PurchaseHoldFrames: FrameId = 30;
	pub const RepaymentRewardPercent: Percent = Percent::from_percent(5);
}

pub fn new_test_ext() -> TestState {
	FrameHighs::set(BTreeMap::new());
	MaxPriceWindowSteps::set(64);
	RepaymentBurn::set(0);
	FailRepayment::set(false);
	HasRepaymentWindow::set(true);
	let mut ext = new_test_with_genesis::<Test>(|storage| {
		pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 1_000_000_000),
				(2, 1_000_000_000),
				(3, 1_000_000_000),
				(4, 10_000),
				(5, 100_000_000),
			],
			..Default::default()
		}
		.assimilate_storage(storage)
		.unwrap();
	});
	ext.execute_with(|| {
		System::set_block_number(1);
		LastGatewayState::set(None);
		ApprovedPool::<Test>::put(Pool::get());
		pallet_price_index::Operator::<Test>::put(1);
		CurrentFrame::set(1);
		CurrentTick::set(1_000);
		Prices::set(Some(PurchasePriceReference {
			r_value: FixedU128::one(),
			argon_usd_target_price: FixedU128::one(),
			microgons_at_target_per_btc: FixedU128::saturating_from_integer(60_000_000_000u128),
			microgons_at_target_per_usdc: FixedU128::from_u32(1_000_000),
		}));
		CurrentTick::set(1);
		<StabilizerBuys as argon_primitives::PriceIndexUpdated>::on_price_index(&price_index(1));
		CurrentTick::set(1_000);
		FailReserve::set(false);
		FailReturn::set(false);
		FailCommitAt::set(None);
		FailPaymentFor::set(None);
		unhashed::put(b"test_liquidity_buy_budget", &100_000_000u128);
	});
	ext
}

/// Gateway delivery owns the transaction around purchase admission.
pub fn record_purchase(account: &u64, purchase: &UniswapPurchase) -> DispatchResult {
	with_storage_layer(|| StabilizerBuys::record_transferred_purchase(account, purchase))
}

pub fn purchase() -> UniswapPurchase {
	let pool = Pool::get();
	UniswapPurchase {
		gateway_activity_nonce: 1,
		repay_liquid_id: None,
		chain_id: pool.chain_id,
		pool_manager: pool.pool_manager,
		pool_id: pool.pool_id,
		usdc_token: pool.usdc_token,
		usdc_spent: U256::from(90_000_000u64),
		principal: 100_000_000,
		// Raw token1/token0 prices: starting ARGON price is 10^12 / 2^40 USD,
		// and ending price is four times higher. The whole purchase still nets a $10 discount.
		sqrt_price_before_x96: U256::one() << 76,
		sqrt_price_after_x96: U256::one() << 77,
		tick: 1,
	}
}

pub fn start_frame(frame: FrameId) {
	CurrentFrame::set(frame);
	<StabilizerBuys as OnNewSlot<u64>>::on_frame_start(frame);
}

pub fn price_index(tick: Tick) -> argon_primitives::PriceIndex {
	argon_primitives::PriceIndex {
		btc_usd_price: FixedU128::from_u32(60_000),
		argonot_usd_price: FixedU128::one(),
		argon_usd_price: FixedU128::one(),
		argon_usd_target_price: FixedU128::one(),
		argon_time_weighted_average_liquidity: 1,
		tick,
		usdc_usd_price: Some(FixedU128::one()),
		stabilizer_pool_sqrt_price_x96: None,
	}
}

parameter_types! {
	pub static RepaymentBurn: Balance = 0;
	pub static FailRepayment: bool = false;
	pub static HasRepaymentWindow: bool = true;
}
pub struct Repayments;
impl BitcoinFissionsProvider<u64, Balance> for Repayments {
	type Weights = ();
	fn is_liquid_repayment_valid(_: &u64, _: LiquidId, _: Tick, _: FrameId) -> bool {
		HasRepaymentWindow::get()
	}

	fn next_r_value_high_water(from_frame: FrameId) -> Option<(FrameId, RValueHighWaterMark)> {
		FrameHighs::get()
			.range(from_frame..)
			.next()
			.map(|(frame, high)| (*frame, *high))
	}
	fn get_account_fission_liquidity(_: &u64) -> Balance {
		0
	}
	fn repay_liquid<BurnCallback: FnMut(Balance) -> DispatchResult>(
		_account: &u64,
		_: LiquidId,
		budget: Balance,
		_prices: LiquidRepayPrices,
		mut burn_callback: BurnCallback,
	) -> Result<Balance, DispatchError> {
		let burn = RepaymentBurn::get().min(budget);
		burn_callback(burn)?;
		ensure!(!FailRepayment::get(), DispatchError::Other("repayment failed"));
		Ok(burn)
	}
}

impl CrosschainGatewayProvider<u128> for Test {
	type Weights = ();

	fn last_state_by_chain(_source_chain: SourceChain) -> Option<GatewayState<u128>> {
		LastGatewayState::get()
	}
}

impl TickProvider<Block> for CurrentTick {
	type Weights = ();
	fn current_tick() -> Tick {
		CurrentTick::get()
	}
	fn previous_tick() -> Tick {
		CurrentTick::get().saturating_sub(1)
	}
	fn elapsed_ticks() -> Tick {
		CurrentTick::get()
	}
	fn voting_schedule() -> VotingSchedule {
		VotingSchedule::from_runtime_current_tick(CurrentTick::get())
	}
	fn ticker() -> Ticker {
		HistoryTicker::get()
	}
	fn blocks_at_tick(_tick: Tick) -> Vec<H256> {
		Vec::new()
	}
}
