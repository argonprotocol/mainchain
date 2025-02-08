use crate::Runtime;
use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
use log::info;
use pallet_grandpa::{Config, CurrentSetId};
use sp_api::runtime_decl_for_core::Core;
use sp_core::Get;

/// This migration adds a missing set id to the CurrentSetId storage item in Grandpa
pub struct UpdateMissingGrandpaSetId<T>(core::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for UpdateMissingGrandpaSetId<T> {
	fn on_runtime_upgrade() -> Weight {
		if CurrentSetId::<T>::get() == 0 && Runtime::version().spec_version == 110 {
			info!("Migration: setting missing Grandpa set id to {}", 1);
			CurrentSetId::<T>::put(1);
			return T::DbWeight::get().reads_writes(1, 1)
		}
		T::DbWeight::get().reads(0)
	}
}
