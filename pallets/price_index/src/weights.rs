use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn submit() -> Weight;
	fn set_operator() -> Weight;
	fn on_finalize_stale_price() -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn submit() -> Weight {
		Weight::zero()
	}
	fn set_operator() -> Weight {
		Weight::zero()
	}
	fn on_finalize_stale_price() -> Weight {
		Weight::zero()
	}
}
