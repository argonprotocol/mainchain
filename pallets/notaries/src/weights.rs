use argon_primitives::notary::NotaryProviderWeightInfo;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	// Actual extrinsics
	fn propose() -> Weight;
	fn activate() -> Weight;
	fn update() -> Weight;

	// on_initialize hooks
	fn on_initialize_with_meta_changes(meta_count: u32) -> Weight;
	fn on_initialize_with_expiring_proposals(expiring_count: u32) -> Weight;
	fn provider_active_notaries() -> Weight;
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> NotaryProviderWeightInfo for ProviderWeightAdapter<T> {
	fn active_notaries() -> Weight {
		<T as crate::Config>::WeightInfo::provider_active_notaries()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn propose() -> Weight {
		Weight::zero()
	}

	fn activate() -> Weight {
		Weight::zero()
	}

	fn update() -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_meta_changes(_meta_count: u32) -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_expiring_proposals(_expiring_count: u32) -> Weight {
		Weight::zero()
	}

	fn provider_active_notaries() -> Weight {
		Weight::zero()
	}
}
