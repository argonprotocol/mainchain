//! Storage-free Treasury boundary for benchmarks that compose provider costs separately.
use argon_primitives::treasury::TreasuryRewardProvider;
use polkadot_sdk::sp_runtime::DispatchResult;

pub struct BenchmarkTreasuryRewardProvider;

impl TreasuryRewardProvider for BenchmarkTreasuryRewardProvider {
	type Balance = u128;
	type Weights = ();

	fn available_budget() -> u128 {
		u128::MAX
	}
	fn reserve_budget(_: u128) -> DispatchResult {
		Ok(())
	}
	fn return_budget(_: u128) -> DispatchResult {
		Ok(())
	}
}
