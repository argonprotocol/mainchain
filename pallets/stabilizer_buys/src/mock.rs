use crate as pallet_stabilizer_buys;
use crate::*;
use argon_primitives::MiningFrameTransitionProvider;
use frame_support::{derive_impl, storage::unhashed};
use pallet_prelude::{new_test_with_genesis, TestState};
use polkadot_sdk::{
	pallet_balances,
	sp_runtime::{
		traits::{ConstU128, ConstU32, ConstU8},
		ArithmeticError, BuildStorage,
	},
};

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
	pub static CurrentFrame: FrameId = 1;
	pub static CurrentTick: Tick = 1_000;
	pub static Prices: Option<PurchasePriceReference> = Some(PurchasePriceReference {
		argon_target_usd: FixedU128::one(), usdc_usd: FixedU128::one(),
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
	fn pay_reward(account: &u64, amount: Balance) -> DispatchResult {
		let outstanding = Self::outstanding_rewards()
			.checked_sub(amount)
			.ok_or(ArithmeticError::Underflow)?;
		unhashed::put(b"test_treasury_outstanding_rewards", &outstanding);
		Balances::mint_into(account, amount)?;
		ensure!(FailPaymentFor::get() != Some(*account), DispatchError::Other("payment failed"));
		Ok(())
	}
}

impl WeightInfo for () {
	fn record_prices() -> Weight {
		Weight::zero()
	}
	fn record_purchase() -> Weight {
		Weight::zero()
	}
	fn record_purchase_first_hold() -> Weight {
		Weight::zero()
	}
	fn bid() -> Weight {
		Weight::zero()
	}
	fn process_payouts_failed(_: u32) -> Weight {
		Weight::zero()
	}
	fn process_payouts(_: u32) -> Weight {
		Weight::zero()
	}
	fn process_auctions(_: u32) -> Weight {
		Weight::zero()
	}
	fn set_approved_pool() -> Weight {
		Weight::zero()
	}
	fn set_matching_rate() -> Weight {
		Weight::zero()
	}
	fn on_frame_start() -> Weight {
		Weight::zero()
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
	type CurrentTick = CurrentTick;
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
	type MiningFrameTransitionProvider = Frame;
	type CurrentTick = CurrentTick;
	type MaxRecentPriceTicks = ConstU32<360>;
	type PriceHistoryBucketTicks = polkadot_sdk::sp_runtime::traits::ConstU64<60>;
	type MaxHourlyPriceReferences = ConstU32<720>;
	type MaxCurrentHourPriceAgeTicks = polkadot_sdk::sp_runtime::traits::ConstU64<10>;
	type TreasuryRewardProvider = Budget;
	type MaxPrincipalPayouts = ConstU32<16>;
	type MaxPayoutsPerBlock = ConstU32<2>;
	type MaxPositiveBids = ConstU8<4>;
	type AuctionBudgetPercent = BudgetPercent;
	type RewardMaturityTicks = RewardMaturityTicks;
	type WeightInfo = ();
}

parameter_types! {
	pub const BudgetPercent: Percent = Percent::from_percent(90);
	pub const RewardMaturityTicks: Tick = 30 * 1_440;
}

pub fn new_test_ext() -> TestState {
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
		ApprovedPool::<Test>::put(Pool::get());
		pallet_price_index::Operator::<Test>::put(1);
		CurrentFrame::set(1);
		CurrentTick::set(1_000);
		Prices::set(Some(PurchasePriceReference {
			argon_target_usd: FixedU128::one(),
			usdc_usd: FixedU128::one(),
		}));
		CurrentTick::set(1);
		<StabilizerBuys as argon_primitives::PriceIndexUpdated>::on_price_index(
			&argon_primitives::PriceIndex {
				btc_usd_price: FixedU128::from_u32(60_000),
				argonot_usd_price: FixedU128::one(),
				argon_usd_price: FixedU128::one(),
				argon_usd_target_price: FixedU128::one(),
				argon_time_weighted_average_liquidity: 1,
				tick: 1,
				usdc_usd_price: Some(FixedU128::one()),
			},
		);
		CurrentTick::set(1_000);
		FailReserve::set(false);
		FailReturn::set(false);
		FailCommitAt::set(None);
		FailPaymentFor::set(None);
		unhashed::put(b"test_liquidity_buy_budget", &100_000_000u128);
	});
	ext
}

pub fn purchase() -> UniswapPurchase {
	let pool = Pool::get();
	UniswapPurchase {
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
