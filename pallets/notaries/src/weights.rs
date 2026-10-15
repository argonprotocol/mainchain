use argon_primitives::{notary::NotaryProviderWeightInfo, TickProvider, TickProviderWeightInfo};
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	// Actual extrinsics
	fn propose() -> Weight;
	fn activate() -> Weight;
	fn update() -> Weight;

	// on_initialize hooks
	fn on_initialize_without_meta_changes() -> Weight;
	fn on_initialize_with_meta_changes(meta_count: u32) -> Weight;
	fn on_finalize_with_expiring_proposals(expiring_count: u32) -> Weight;
	fn provider_active_notaries() -> Weight;
}

type TickProviderWeights<T> = <<T as crate::Config>::TickProvider as TickProvider<
	<T as frame_system::Config>::Block,
>>::Weights;

pub struct WithProviderWeights<T, Base, TickProviderWeight = TickProviderWeights<T>>(
	PhantomData<(T, Base, TickProviderWeight)>,
);
impl<T, Base, TickProviderWeight> WeightInfo for WithProviderWeights<T, Base, TickProviderWeight>
where
	T: crate::Config,
	Base: WeightInfo,
	TickProviderWeight: TickProviderWeightInfo,
{
	fn propose() -> Weight {
		Base::propose()
	}

	fn activate() -> Weight {
		Base::activate().saturating_add(TickProviderWeight::current_tick())
	}

	fn update() -> Weight {
		Base::update().saturating_add(TickProviderWeight::current_tick())
	}

	fn on_initialize_without_meta_changes() -> Weight {
		Base::on_initialize_without_meta_changes()
			.saturating_add(TickProviderWeight::current_tick())
	}

	fn on_initialize_with_meta_changes(meta_count: u32) -> Weight {
		Base::on_initialize_with_meta_changes(meta_count)
			.saturating_add(TickProviderWeight::current_tick())
	}

	fn on_finalize_with_expiring_proposals(expiring_count: u32) -> Weight {
		Base::on_finalize_with_expiring_proposals(expiring_count)
	}

	fn provider_active_notaries() -> Weight {
		Base::provider_active_notaries()
	}
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

	fn on_initialize_without_meta_changes() -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_meta_changes(_meta_count: u32) -> Weight {
		Weight::zero()
	}

	fn on_finalize_with_expiring_proposals(_expiring_count: u32) -> Weight {
		Weight::zero()
	}

	fn provider_active_notaries() -> Weight {
		Weight::zero()
	}
}
