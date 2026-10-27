use argon_primitives::providers::{FeelessCallWeightInfo, PriceProviderWeightInfo};
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn provider_get_average_cpi_for_ticks() -> Weight;
	fn provider_is_feeless() -> Weight;
	fn on_initialize() -> Weight;
	fn submit() -> Weight;
	fn set_operator() -> Weight;
	fn provider_get_lowest_microgons_per_argonot() -> Weight;
	fn provider_get_average_microgons_per_argonot() -> Weight;
	fn provider_get_ethereum_prices() -> Weight;
	fn provider_get_liquidity_change_needed() -> Weight;
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> FeelessCallWeightInfo<T::RuntimeCall> for ProviderWeightAdapter<T>
where
	T::RuntimeCall: frame_support::traits::IsSubType<crate::Call<T>>,
{
	fn is_feeless(call: &T::RuntimeCall) -> Weight {
		if matches!(
			<T::RuntimeCall as frame_support::traits::IsSubType<crate::Call<T>>>::is_sub_type(call),
			Some(crate::Call::submit { .. })
		) {
			return T::WeightInfo::provider_is_feeless();
		}
		Weight::zero()
	}
}
impl<T: crate::Config> PriceProviderWeightInfo for ProviderWeightAdapter<T> {
	fn get_average_cpi_for_ticks() -> Weight {
		T::WeightInfo::provider_get_average_cpi_for_ticks()
	}

	fn get_lowest_microgons_per_argonot() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_lowest_microgons_per_argonot()
	}

	fn get_average_microgons_per_argonot() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_average_microgons_per_argonot()
	}

	fn get_ethereum_prices() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_ethereum_prices()
	}

	fn get_liquidity_change_needed() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_liquidity_change_needed()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn provider_get_average_cpi_for_ticks() -> Weight {
		Weight::zero()
	}

	fn provider_is_feeless() -> Weight {
		Weight::zero()
	}
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
	fn provider_get_ethereum_prices() -> Weight {
		Weight::zero()
	}
	fn provider_get_liquidity_change_needed() -> Weight {
		Weight::zero()
	}
}
