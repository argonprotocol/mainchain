use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	// Actual extrinsics
	fn bid() -> Weight;
	fn configure_mining_slot_delay() -> Weight;

	// Hooks with variance
	fn on_finalize_grandpa_rotation(m: u32) -> Weight;
	fn start_new_frame(m: u32) -> Weight;

	// Weight prediction and frame adjustment operations
	fn on_initialize_with_frame_start() -> Weight;
	fn on_finalize_frame_adjustments() -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn bid() -> Weight {
		Weight::zero()
	}

	fn configure_mining_slot_delay() -> Weight {
		Weight::zero()
	}

	fn on_finalize_grandpa_rotation(_m: u32) -> Weight {
		Weight::zero()
	}

	fn start_new_frame(_m: u32) -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_frame_start() -> Weight {
		Weight::zero()
	}

	fn on_finalize_frame_adjustments() -> Weight {
		Weight::zero()
	}
}
