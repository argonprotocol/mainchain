use crate::{
	configs::{Get, Weight},
	Runtime,
};
use frame_support::traits::OnRuntimeUpgrade;
use log::info;
use pallet_grandpa::{Config, CurrentSetId};
use sp_api::runtime_decl_for_core::Core;
use sp_arithmetic::traits::{Saturating, UniqueSaturatedInto};
use sp_runtime::traits::NumberFor;

/// This migration adds a missing set id to the CurrentSetId storage item in Grandpa
pub struct UpdateMissingGrandpaSetId<T>(core::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for UpdateMissingGrandpaSetId<T> {
	fn on_runtime_upgrade() -> Weight {
		if CurrentSetId::<T>::get() == 0 && Runtime::version().spec_version == 110 {
			#[cfg(not(feature = "canary"))]
			let set_id = 1;

			#[cfg(feature = "canary")]
			let set_id = calculate_testnet_set_id::<T>(<frame_system::Pallet<T>>::block_number());

			info!("Migration: setting missing Grandpa set id to {}", set_id);
			CurrentSetId::<T>::put(set_id);
			return T::DbWeight::get().reads_writes(1, 1)
		}
		T::DbWeight::get().reads(0)
	}
}

fn calculate_testnet_set_id<T: Config>(current_block: NumberFor<T::Block>) -> u64 {
	let set_rotation_start = NumberFor::<T::Block>::from(38881u32);
	let set_id_start = NumberFor::<T::Block>::from(3u32);
	let add_ons =
		current_block.saturating_sub(set_rotation_start) / NumberFor::<T::Block>::from(1440u32);
	(set_id_start + add_ons).unique_saturated_into()
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn calculates_latest_set_id() {
		assert_eq!(calculate_testnet_set_id::<Runtime>(38881), 3);
		assert_eq!(calculate_testnet_set_id::<Runtime>(40320), 3);
		assert_eq!(calculate_testnet_set_id::<Runtime>(40321), 4);
		assert_eq!(calculate_testnet_set_id::<Runtime>(47424), 8);
	}
}
