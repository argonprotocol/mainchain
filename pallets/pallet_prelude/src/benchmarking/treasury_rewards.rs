//! Storage-free Treasury boundary for benchmarks that compose provider costs separately.
use argon_primitives::treasury::TreasuryRewardProvider;
use core::sync::atomic::{AtomicBool, Ordering};
use polkadot_sdk::sp_runtime::{DispatchError, DispatchResult};

static FAIL_PAYMENT: AtomicBool = AtomicBool::new(false);

pub fn set_benchmark_treasury_reward_payment_failure(fail: bool) {
	FAIL_PAYMENT.store(fail, Ordering::Relaxed);
}

pub struct BenchmarkTreasuryRewardProvider;

impl<AccountId> TreasuryRewardProvider<AccountId> for BenchmarkTreasuryRewardProvider {
	type Balance = u128;
	type Weights = ();

	fn available_budget() -> u128 {
		u128::MAX
	}
	fn reserve_budget(_: u128) -> DispatchResult {
		Ok(())
	}
	fn commit_reward(_: u128) -> DispatchResult {
		Ok(())
	}
	fn return_budget(_: u128) -> DispatchResult {
		Ok(())
	}
	fn pay_reward(_: &AccountId, _: u128) -> DispatchResult {
		if FAIL_PAYMENT.load(Ordering::Relaxed) {
			return Err(DispatchError::Other("benchmark reward payment failed"));
		}
		Ok(())
	}
}
