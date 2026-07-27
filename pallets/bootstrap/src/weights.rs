use pallet_prelude::*;

pub trait WeightInfo {
	fn set_recovery_payload() -> Weight;
	fn set_endpoint() -> Weight;
}

impl WeightInfo for () {
	fn set_recovery_payload() -> Weight {
		Weight::zero()
	}

	fn set_endpoint() -> Weight {
		Weight::zero()
	}
}
