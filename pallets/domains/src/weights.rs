use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	// Actual extrinsics
	fn set_zone_record() -> Weight;

	// Hooks with variance
	fn on_initialize_with_expiring_domains(n: u32) -> Weight;

	// Event handlers
	fn notebook_submitted_event_handler(d: u32) -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn set_zone_record() -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_expiring_domains(_n: u32) -> Weight {
		Weight::zero()
	}

	fn notebook_submitted_event_handler(_d: u32) -> Weight {
		Weight::zero()
	}
}
