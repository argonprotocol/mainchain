use crate::{
	migrations::v5,
	pallet::{ActiveMinersByIndex, NextSlotCohort},
	Config, Registration,
};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::{argon_primitives::TickProvider, *};

pub struct InnerMigrate<T: Config>(PhantomData<T>);

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
struct Model<T: Config> {
	next_cohort: Vec<v5::Registration<T>>,
	active_miners_by_index: Vec<(u32, v5::Registration<T>)>,
}

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let next_cohort = v5::NextSlotCohort::<T>::get().to_vec();
		let active_miners_by_index = v5::ActiveMinersByIndex::<T>::iter().collect::<Vec<_>>();

		Ok(Model::<T> { next_cohort, active_miners_by_index }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating mining slot pallet storage");
		let old = v5::NextSlotCohort::<T>::take();
		// just set all bids to current tick
		let bid_at_tick = T::TickProvider::current_tick();
		let new = old
			.into_iter()
			.map(|x| Registration::<T> {
				account_id: x.account_id,
				external_funding_account: None,
				reward_destination: x.reward_destination,
				bid: x.bid,
				argonots: x.argonots,
				authority_keys: x.authority_keys,
				cohort_id: x.cohort_id,
				bid_at_tick,
			})
			.collect::<Vec<_>>();
		NextSlotCohort::<T>::put(BoundedVec::truncate_from(new));
		count += 1;
		ActiveMinersByIndex::<T>::translate::<v5::Registration<T>, _>(|_id, reg| {
			count += 1;
			Some(Registration::<T> {
				account_id: reg.account_id,
				external_funding_account: None,
				reward_destination: reg.reward_destination,
				bid: reg.bid,
				argonots: reg.argonots,
				authority_keys: reg.authority_keys,
				cohort_id: reg.cohort_id,
				bid_at_tick,
			})
		});
		log::info!("{} mining registrations migrated", count);

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;
		use sp_core::Decode;

		let old = <Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		let new = NextSlotCohort::<T>::get();
		ensure!(old.next_cohort.len() == new.len(), "New next cohort not set correctly");

		let old_active_miners_by_index = old.active_miners_by_index;
		let new_active_miners_by_index = ActiveMinersByIndex::<T>::iter().collect::<Vec<_>>();
		ensure!(
			old_active_miners_by_index.len() == new_active_miners_by_index.len(),
			"New active miners value not set correctly"
		);

		Ok(())
	}
}

pub type BiddingTickMigration<T> = frame_support::migrations::VersionedMigration<
	5, // The migration will only execute when the on-chain storage version is 1
	6, // The on-chain storage version will be set to 2 after the migration is complete
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use argon_primitives::block_seal::RewardDestination::Owner;
	use frame_support::assert_ok;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			v5::NextSlotCohort::<Test>::mutate(|v| {
				let _ = v.try_push(v5::Registration::<Test> {
					account_id: 1,
					external_funding_account: Some(1),
					argonots: 1,
					bid: 100u128,
					authority_keys: 1u64.into(),
					reward_destination: Owner,
					cohort_id: 1,
				});
				let _ = v.try_push(v5::Registration::<Test> {
					account_id: 2,
					external_funding_account: Some(2),
					argonots: 2,
					bid: 100u128,
					authority_keys: 2u64.into(),
					reward_destination: Owner,
					cohort_id: 2,
				});
			});

			v5::ActiveMinersByIndex::<Test>::insert(
				0,
				v5::Registration::<Test> {
					account_id: 1,
					external_funding_account: Some(1),
					argonots: 1,
					bid: 100u128,
					authority_keys: 1u64.into(),
					reward_destination: Owner,
					cohort_id: 1,
				},
			);
			v5::ActiveMinersByIndex::<Test>::insert(
				1,
				v5::Registration::<Test> {
					account_id: 2,
					external_funding_account: Some(2),
					argonots: 1000,
					bid: 123u128,
					authority_keys: 2u64.into(),
					reward_destination: Owner,
					cohort_id: 1,
				},
			);

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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 3));

			// Check the new value
			let new = NextSlotCohort::<Test>::get();
			assert_eq!(new.len(), 2);
			assert_eq!(new[0].account_id, 1);
			assert_eq!(new[1].account_id, 2);

			let new_active_miners_by_index =
				ActiveMinersByIndex::<Test>::iter().collect::<Vec<_>>();
			assert_eq!(new_active_miners_by_index.len(), 2);
			assert_eq!(new_active_miners_by_index[0].0, 0);
			assert_eq!(new_active_miners_by_index[1].0, 1);
			assert_eq!(new_active_miners_by_index[0].1.bid, 100u128);
			assert_eq!(new_active_miners_by_index[1].1.bid, 123u128);
		});
	}
}
