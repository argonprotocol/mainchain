use super::Config;
use argon_primitives::UtxoLockEventsWeightInfo;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn on_initialize(utxo_count: u32) -> Weight;
	fn provider_utxo_locked() -> Weight;
	fn provider_utxo_released() -> Weight;
	fn provider_utxo_released_with_pending_mints() -> Weight;
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: Config> UtxoLockEventsWeightInfo for ProviderWeightAdapter<T> {
	fn utxo_locked() -> Weight {
		<T as Config>::WeightInfo::provider_utxo_locked()
	}

	fn utxo_released() -> Weight {
		<T as Config>::WeightInfo::provider_utxo_released()
	}

	fn utxo_released_with_pending_mints() -> Weight {
		<T as Config>::WeightInfo::provider_utxo_released_with_pending_mints()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn on_initialize(_utxo_count: u32) -> Weight {
		Weight::zero()
	}

	fn provider_utxo_locked() -> Weight {
		Weight::zero()
	}

	fn provider_utxo_released() -> Weight {
		Weight::zero()
	}

	fn provider_utxo_released_with_pending_mints() -> Weight {
		Weight::zero()
	}
}
