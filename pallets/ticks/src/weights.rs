use argon_primitives::providers::TickProviderWeightInfo;
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_initialize_with_cleanup(cleanup_range: u32) -> Weight;
	fn provider_previous_tick() -> Weight;
	fn provider_current_tick() -> Weight;
	fn provider_elapsed_ticks() -> Weight;
	fn provider_ticker() -> Weight;
	fn provider_blocks_at_tick() -> Weight;
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> TickProviderWeightInfo for ProviderWeightAdapter<T> {
	fn previous_tick() -> Weight {
		<T as crate::Config>::WeightInfo::provider_previous_tick()
	}

	fn current_tick() -> Weight {
		<T as crate::Config>::WeightInfo::provider_current_tick()
	}

	fn elapsed_ticks() -> Weight {
		<T as crate::Config>::WeightInfo::provider_elapsed_ticks()
	}

	fn ticker() -> Weight {
		<T as crate::Config>::WeightInfo::provider_ticker()
	}

	fn blocks_at_tick() -> Weight {
		<T as crate::Config>::WeightInfo::provider_blocks_at_tick()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_initialize_with_cleanup(_cleanup_range: u32) -> Weight {
		Weight::zero()
	}

	fn provider_previous_tick() -> Weight {
		Weight::zero()
	}

	fn provider_current_tick() -> Weight {
		Weight::zero()
	}

	fn provider_elapsed_ticks() -> Weight {
		Weight::zero()
	}

	fn provider_ticker() -> Weight {
		Weight::zero()
	}

	fn provider_blocks_at_tick() -> Weight {
		Weight::zero()
	}
}
