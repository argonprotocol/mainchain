use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_frame_transition() -> Weight;
	fn try_pay_reward() -> Weight;
	// Main extrinsic functions
	fn bond_argons() -> Weight;
	fn unbond_argons() -> Weight;
	fn vault_operator_prebond() -> Weight;

	// Frame transition operations (DoS attack prevention)
	fn distribute_bid_pool(v: u32, c: u32) -> Weight;
	fn rollover_contributors(v: u32, c: u32) -> Weight;
	fn end_pool_capital_raise(v: u32, c: u32) -> Weight;
	fn release_rolling_contributors(v: u32, c: u32) -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_frame_transition() -> Weight {
		Weight::zero()
	}
	fn try_pay_reward() -> Weight {
		// Conservative placeholder until pallet_treasury runtime benchmarks are wired.
		Weight::from_parts(100_000_000, 0)
	}
	fn bond_argons() -> Weight {
		Weight::zero()
	}

	fn unbond_argons() -> Weight {
		Weight::zero()
	}

	fn vault_operator_prebond() -> Weight {
		Weight::zero()
	}

	fn distribute_bid_pool(_v: u32, _c: u32) -> Weight {
		Weight::zero()
	}

	fn rollover_contributors(_v: u32, _c: u32) -> Weight {
		Weight::zero()
	}

	fn end_pool_capital_raise(_v: u32, _c: u32) -> Weight {
		Weight::zero()
	}

	fn release_rolling_contributors(_v: u32, _c: u32) -> Weight {
		Weight::zero()
	}
}
