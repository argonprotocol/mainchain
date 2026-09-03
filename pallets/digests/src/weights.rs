use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_initialize(notebook_count: u32) -> Weight;
	fn on_finalize() -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_initialize(_notebook_count: u32) -> Weight {
		Weight::zero()
	}

	fn on_finalize() -> Weight {
		Weight::zero()
	}
}
