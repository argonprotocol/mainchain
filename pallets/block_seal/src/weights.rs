use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn apply() -> Weight;
	fn on_finalize_with_notebooks(n: u32) -> Weight;
	fn on_initialize_with_notebooks(notebook_count: u32) -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn apply() -> Weight {
		Weight::zero()
	}
	fn on_finalize_with_notebooks(_n: u32) -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_notebooks(_notebook_count: u32) -> Weight {
		Weight::zero()
	}
}
