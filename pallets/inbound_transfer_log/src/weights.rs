use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_initialize_cleanup(c: u32) -> Weight;
	fn on_token_gateway_request_recorded() -> Weight;
	fn on_token_gateway_request_dropped() -> Weight;
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_initialize_cleanup(_c: u32) -> Weight {
		Weight::zero()
	}

	fn on_token_gateway_request_recorded() -> Weight {
		Weight::zero()
	}

	fn on_token_gateway_request_dropped() -> Weight {
		Weight::zero()
	}
}
