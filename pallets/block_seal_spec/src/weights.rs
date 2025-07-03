use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn configure() -> Weight;
	fn on_initialize_with_digest() -> Weight;
	fn on_finalize_with_vote_adjustment() -> Weight;
	fn notebook_submitted() -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn configure() -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_digest() -> Weight {
		Weight::zero()
	}

	fn on_finalize_with_vote_adjustment() -> Weight {
		Weight::zero()
	}

	fn notebook_submitted() -> Weight {
		Weight::zero()
	}
}
