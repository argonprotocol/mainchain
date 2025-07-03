use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_initialize_with_cleanup(cleanup_range: u32) -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_initialize_with_cleanup(_cleanup_range: u32) -> Weight {
		Weight::zero()
	}
}
