use pallet_prelude::*;

/// Weight functions needed for pallet_notebook.
pub trait WeightInfo {
	fn submit(n: u32) -> Weight;
	fn submit_with_account_origins(a: u32) -> Weight;
	fn unlock() -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn submit(_n: u32) -> Weight {
		Weight::zero()
	}

	fn submit_with_account_origins(_a: u32) -> Weight {
		Weight::zero()
	}

	fn unlock() -> Weight {
		Weight::zero()
	}
}
