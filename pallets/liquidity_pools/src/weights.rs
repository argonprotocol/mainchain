use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	// Main extrinsic functions
	fn bond_argons() -> Weight;
	fn unbond_argons() -> Weight;

	// Frame transition operations (DoS attack prevention)
	fn distribute_bid_pool(v: u32, c: u32) -> Weight;
	fn rollover_contributors(v: u32, c: u32) -> Weight;
	fn end_pool_capital_raise(v: u32, c: u32) -> Weight;
	fn release_rolling_contributors(v: u32, c: u32) -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn bond_argons() -> Weight {
		Weight::zero()
	}

	fn unbond_argons() -> Weight {
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
