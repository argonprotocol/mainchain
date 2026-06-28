use argon_primitives::providers::PriceProviderWeightInfo;
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_initialize() -> Weight;
	fn submit() -> Weight;
	fn set_operator() -> Weight;
	fn provider_get_lowest_microgons_per_argonot() -> Weight;
	fn provider_get_average_microgons_per_argonot() -> Weight;
	fn provider_get_liquidity_change_needed() -> Weight;
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> PriceProviderWeightInfo for ProviderWeightAdapter<T> {
	fn get_lowest_microgons_per_argonot() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_lowest_microgons_per_argonot()
	}

	fn get_average_microgons_per_argonot() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_average_microgons_per_argonot()
	}

	fn get_liquidity_change_needed() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_liquidity_change_needed()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_initialize() -> Weight {
		Weight::zero()
	}
	fn submit() -> Weight {
		Weight::zero()
	}
	fn set_operator() -> Weight {
		Weight::zero()
	}
	fn provider_get_lowest_microgons_per_argonot() -> Weight {
		Weight::zero()
	}
	fn provider_get_average_microgons_per_argonot() -> Weight {
		Weight::zero()
	}
	fn provider_get_liquidity_change_needed() -> Weight {
		Weight::zero()
	}
}
