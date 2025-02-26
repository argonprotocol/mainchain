use crate::{pallet::NextSlotCohort, Config};
#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};
use log::info;

mod old_storage {
	use crate::{Config, Registration};
	use frame_support::{pallet_prelude::*, storage_alias, BoundedVec};

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub next_cohort: BoundedVec<Registration<T>, <T as Config>::MaxCohortSize>,
	}

	#[storage_alias]
	pub(super) type NextSlotCohort<T: Config> = StorageValue<
		crate::Pallet<T>,
		BoundedVec<Registration<T>, <T as Config>::MaxCohortSize>,
		ValueQuery,
	>;
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let next_cohort = old_storage::NextSlotCohort::<T>::get();

		Ok(old_storage::Model::<T> { next_cohort }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		info!("Migrating Next Cohort Length");
		let old = old_storage::NextSlotCohort::<T>::take();

		count += 1;
		NextSlotCohort::<T>::put(BoundedVec::truncate_from(old.to_vec()));

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;
		use sp_core::Decode;

		let old = <old_storage::Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		let new = NextSlotCohort::<T>::get();

		ensure!(old.next_cohort.len() == new.len(), "New value not set correctly");

		Ok(())
	}
}

pub type MiningSlotMigration<T> = frame_support::migrations::VersionedMigration<
	3, // The migration will only execute when the on-chain storage version is 1
	4, // The on-chain storage version will be set to 2 after the migration is complete
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::{
		mock::{new_test_ext, Test},
		Registration,
	};
	use argon_primitives::block_seal::RewardDestination::Owner;
	use frame_support::assert_ok;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			old_storage::NextSlotCohort::<Test>::mutate(|v| {
				let _ = v.try_push(Registration::<Test> {
					account_id: 1,
					obligation_id: Some(1),
					argonots: 1,
					bonded_argons: 100u128,
					authority_keys: 1u64.into(),
					reward_destination: Owner,
					reward_sharing: None,
					cohort_id: 1,
				});
				let _ = v.try_push(Registration::<Test> {
					account_id: 2,
					obligation_id: Some(2),
					argonots: 2,
					bonded_argons: 100u128,
					authority_keys: 2u64.into(),
					reward_destination: Owner,
					reward_sharing: None,
					cohort_id: 2,
				});
			});

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrate::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};
			// Execute the migration
			let weight = InnerMigrate::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrate::<Test>::post_upgrade(bytes));

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1));

			// Check the new value
			let new = NextSlotCohort::<Test>::get();
			assert_eq!(new.len(), 2);
			assert_eq!(new[0].account_id, 1);
			assert_eq!(new[1].account_id, 2);
		});
	}
}
