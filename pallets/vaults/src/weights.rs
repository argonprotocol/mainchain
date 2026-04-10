use argon_primitives::{
	providers::{TickProvider, TickProviderWeightInfo},
	vault::BitcoinVaultProviderWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for pallet_vaults.
pub trait WeightInfo {
	fn create() -> Weight;
	fn modify_funding() -> Weight;
	fn modify_terms() -> Weight;
	fn close() -> Weight;
	fn replace_bitcoin_xpub() -> Weight;
	fn set_bitcoin_lock_delegate() -> Weight;
	fn set_name() -> Weight;
	fn on_initialize_with_vault_releases(
		height_range: u32,
		bitcoin_release_vault_count: u32,
		operational_unlock_work: u32,
	) -> Weight;
	fn collect() -> Weight;
	fn on_frame_start(vault_count: u32) -> Weight;
	fn provider_get_registration_vault_data() -> Weight;
	fn provider_account_became_operational() -> Weight;
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
	fn create() -> Weight {
		Base::create().saturating_add(TickProviderWeight::current_tick())
	}

	fn modify_funding() -> Weight {
		Base::modify_funding()
	}

	fn modify_terms() -> Weight {
		Base::modify_terms().saturating_add(TickProviderWeight::current_tick())
	}

	fn close() -> Weight {
		Base::close()
	}

	fn replace_bitcoin_xpub() -> Weight {
		Base::replace_bitcoin_xpub()
	}

	fn set_bitcoin_lock_delegate() -> Weight {
		Base::set_bitcoin_lock_delegate()
	}

	fn set_name() -> Weight {
		Base::set_name().saturating_add(TickProviderWeight::current_tick())
	}

	fn on_initialize_with_vault_releases(
		height_range: u32,
		bitcoin_release_vault_count: u32,
		operational_unlock_work: u32,
	) -> Weight {
		Base::on_initialize_with_vault_releases(
			height_range,
			bitcoin_release_vault_count,
			operational_unlock_work,
		)
		.saturating_add(TickProviderWeight::previous_tick())
		.saturating_add(TickProviderWeight::current_tick())
	}

	fn collect() -> Weight {
		Base::collect()
	}

	fn on_frame_start(vault_count: u32) -> Weight {
		Base::on_frame_start(vault_count).saturating_add(TickProviderWeight::current_tick())
	}

	fn provider_get_registration_vault_data() -> Weight {
		Base::provider_get_registration_vault_data()
	}

	fn provider_account_became_operational() -> Weight {
		Base::provider_account_became_operational()
			.saturating_add(TickProviderWeight::current_tick())
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> BitcoinVaultProviderWeightInfo for ProviderWeightAdapter<T> {
	fn get_registration_vault_data() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_registration_vault_data()
	}

	fn account_became_operational() -> Weight {
		<T as crate::Config>::WeightInfo::provider_account_became_operational()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn create() -> Weight {
		Weight::zero()
	}
	fn modify_funding() -> Weight {
		Weight::zero()
	}
	fn modify_terms() -> Weight {
		Weight::zero()
	}
	fn close() -> Weight {
		Weight::zero()
	}
	fn replace_bitcoin_xpub() -> Weight {
		Weight::zero()
	}
	fn set_bitcoin_lock_delegate() -> Weight {
		Weight::zero()
	}
	fn set_name() -> Weight {
		Weight::zero()
	}
	fn on_initialize_with_vault_releases(
		_height_range: u32,
		_bitcoin_release_vault_count: u32,
		_operational_unlock_work: u32,
	) -> Weight {
		Weight::zero()
	}
	fn collect() -> Weight {
		Weight::zero()
	}
	fn on_frame_start(_vault_count: u32) -> Weight {
		Weight::zero()
	}
	fn provider_get_registration_vault_data() -> Weight {
		Weight::zero()
	}

	fn provider_account_became_operational() -> Weight {
		Weight::zero()
	}
}
