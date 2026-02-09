use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	// Actual extrinsics
	fn bid() -> Weight;
	fn configure_mining_slot_delay() -> Weight;

	// Hooks with variance
	fn on_finalize_record_block_author() -> Weight;
	fn on_finalize_grandpa_rotation() -> Weight;
	fn start_new_frame(m: u32) -> Weight;

	// Frame adjustment operations
	fn on_finalize_frame_adjustments() -> Weight;

	// Event handlers
	fn block_seal_read_vote() -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn bid() -> Weight {
		Weight::zero()
	}

	fn configure_mining_slot_delay() -> Weight {
		Weight::zero()
	}

	fn on_finalize_grandpa_rotation() -> Weight {
		Weight::zero()
	}

	fn on_finalize_record_block_author() -> Weight {
		Weight::zero()
	}

	fn start_new_frame(_m: u32) -> Weight {
		Weight::zero()
	}

	fn on_finalize_frame_adjustments() -> Weight {
		Weight::zero()
	}

	fn block_seal_read_vote() -> Weight {
		Weight::zero()
	}
}
