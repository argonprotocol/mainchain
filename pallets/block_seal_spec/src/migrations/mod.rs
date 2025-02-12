use crate::{pallet::PastComputeBlockTimes, Config};
#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};

pub mod v1 {
	use crate::Config;
	use frame_support::{pallet_prelude::ValueQuery, storage_alias};
	use sp_core::ConstU32;
	use sp_runtime::BoundedVec;

	#[storage_alias]
	pub type PastComputeBlockTimes<T: Config> =
		StorageValue<crate::Pallet<T>, BoundedVec<u64, ConstU32<360>>, ValueQuery>;
}

pub struct InnerMigrateV0ToV1<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV0ToV1<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let times = v1::PastComputeBlockTimes::<T>::get();
		Ok(times.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let entry = v1::PastComputeBlockTimes::<T>::get();
		let count = T::HistoricalComputeBlocksForAverage::get() as usize;
		let last_x = entry.to_vec()[entry.len() - count..].to_vec();
		PastComputeBlockTimes::<T>::put(BoundedVec::truncate_from(last_x));

		T::DbWeight::get().reads_writes(1, 1)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;

		let old_value = BoundedVec::<u64, ConstU32<360>>::decode(&mut &state[..])
			.map_err(|_| sp_runtime::TryRuntimeError::from("Failed to decode old value"))?;
		let actual_new_value = PastComputeBlockTimes::<T>::get();

		ensure!(actual_new_value.len() <= old_value.len(), "New value not set correctly");
		Ok(())
	}
}

pub type MigrateV0ToV1<T> = frame_support::migrations::VersionedMigration<
	0, // The migration will only execute when the on-chain storage version is 0
	1, // The on-chain storage version will be set to 1 after the migration is complete
	InnerMigrateV0ToV1<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{new_test_ext, HistoricalComputeBlocksForAverage, Test};
	use frame_support::assert_ok;

	#[test]
	fn handles_existing_value() {
		HistoricalComputeBlocksForAverage::set(10);
		new_test_ext(10, 10).execute_with(|| {
			v1::PastComputeBlockTimes::<Test>::mutate(|a| {
				for i in 0..360 {
					let _ = a.try_push(i);
				}
			});

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrateV0ToV1::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};

			// Execute the migration
			let weight = InnerMigrateV0ToV1::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrateV0ToV1::<Test>::post_upgrade(bytes));

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1));

			let next_value = PastComputeBlockTimes::<Test>::get();

			assert_eq!(next_value.len(), 10);
			assert_eq!(next_value[0], 350);
			assert_eq!(next_value.last().unwrap(), &359);
		})
	}
}
