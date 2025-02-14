use crate::{
	pallet::{ActiveMinersByIndex, NextSlotCohort},
	runtime_decl_for_mining_slot_api::LastActivatedCohortId,
	Config,
};
use alloc::vec::Vec;
use argon_primitives::block_seal::MiningRegistration;
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};
use log::info;

mod old_storage {
	use crate::Config;
	use argon_primitives::{
		block_seal::{MinerIndex, RewardDestination, RewardSharing},
		ObligationId,
	};
	use codec::MaxEncodedLen;
	use frame_support::{pallet_prelude::*, storage_alias, Blake2_128Concat, BoundedVec};
	use scale_info::TypeInfo;

	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct MiningRegistration<T: Config> {
		pub account_id: <T as frame_system::Config>::AccountId,
		pub reward_destination: RewardDestination<<T as frame_system::Config>::AccountId>,
		pub obligation_id: Option<ObligationId>,
		#[codec(compact)]
		pub bond_amount: <T as Config>::Balance,
		#[codec(compact)]
		pub ownership_tokens: <T as Config>::Balance,
		pub reward_sharing: Option<RewardSharing<<T as frame_system::Config>::AccountId>>,
		pub authority_keys: <T as Config>::Keys,
		pub slot_id: u64,
	}

	#[storage_alias]
	pub(super) type ActiveMinersByIndex<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		MinerIndex,
		MiningRegistration<T>,
		OptionQuery,
	>;

	#[storage_alias]
	pub(super) type NextSlotCohort<T: Config> = StorageValue<
		crate::Pallet<T>,
		BoundedVec<MiningRegistration<T>, <T as Config>::MaxCohortSize>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type CurrentSlotId<T: Config> = StorageValue<crate::Pallet<T>, u64, ValueQuery>;

	#[cfg(feature = "try-runtime")]
	#[derive(Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct MiningStructure<T: Config> {
		pub active_miners: alloc::vec::Vec<(MinerIndex, MiningRegistration<T>)>,
		pub next_cohort: BoundedVec<MiningRegistration<T>, <T as Config>::MaxCohortSize>,
		pub slot_id: u64,
	}
}

pub struct InnerMigrateV2Tov3<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV2Tov3<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let active_miners = old_storage::ActiveMinersByIndex::<T>::iter().collect::<Vec<_>>();
		let next_cohort = old_storage::NextSlotCohort::<T>::get();
		let slot_id = old_storage::CurrentSlotId::<T>::get();
		// Return it as an encoded `Vec<u8>`
		Ok(old_storage::MiningStructure { slot_id, active_miners, next_cohort }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		info!("Migrating Mining Slots from v2 to v3");
		ActiveMinersByIndex::<T>::translate::<old_storage::MiningRegistration<T>, _>(|id, reg| {
			info!("Migration: Translating mining registration with id {:?}", id);
			count += 1;
			Some(MiningRegistration {
				account_id: reg.account_id,
				reward_destination: reg.reward_destination,
				obligation_id: reg.obligation_id,
				bonded_argons: reg.bond_amount,
				argonots: reg.ownership_tokens,
				reward_sharing: reg.reward_sharing,
				authority_keys: reg.authority_keys,
				cohort_id: reg.slot_id,
			})
		});
		let _ = NextSlotCohort::<T>::translate::<
			BoundedVec<old_storage::MiningRegistration<T>, <T as Config>::MaxCohortSize>,
			_,
		>(|cohort| {
			if let Some(cohort) = cohort {
				count += 1;
				let next = cohort
					.into_iter()
					.map(|reg| MiningRegistration {
						account_id: reg.account_id,
						reward_destination: reg.reward_destination,
						obligation_id: reg.obligation_id,
						bonded_argons: reg.bond_amount,
						argonots: reg.ownership_tokens,
						reward_sharing: reg.reward_sharing,
						authority_keys: reg.authority_keys,
						cohort_id: reg.slot_id,
					})
					.collect::<Vec<_>>();
				return Some(BoundedVec::truncate_from(next));
			}
			None
		});

		count += 1;
		LastActivatedCohortId::<T>::put(old_storage::CurrentSlotId::<T>::get());

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;
		use sp_core::Decode;

		let old_storage::MiningStructure {
			active_miners: old_active_miners,
			next_cohort: old_next_cohort,
			slot_id,
		} = <old_storage::MiningStructure<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		let new_active_miners = ActiveMinersByIndex::<T>::iter().collect::<Vec<_>>();

		ensure!(old_active_miners.len() == new_active_miners.len(), "New value not set correctly");
		for x in new_active_miners {
			ensure!(
				old_active_miners.iter().any(|(id, _)| id == &x.0),
				"Miner missing in translation"
			);
		}

		ensure!(NextSlotCohort::<T>::get().len() == old_next_cohort.len(), "read cohort correctly");
		ensure!(slot_id == LastActivatedCohortId::<T>::get(), "slotid matches");
		Ok(())
	}
}

pub type MigrateV2Tov3<T> = frame_support::migrations::VersionedMigration<
	2, // The migration will only execute when the on-chain storage version is 1
	3, // The on-chain storage version will be set to 2 after the migration is complete
	InnerMigrateV2Tov3<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrateV2Tov3;
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use argon_primitives::block_seal::RewardDestination::Owner;
	use frame_support::assert_ok;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			old_storage::ActiveMinersByIndex::<Test>::insert(
				1,
				old_storage::MiningRegistration {
					account_id: 1,
					obligation_id: Some(1),
					bond_amount: 100u128,
					authority_keys: 1u64.into(),
					ownership_tokens: 100u128,
					reward_destination: Owner,
					reward_sharing: None,
					slot_id: 1,
				},
			);
			old_storage::ActiveMinersByIndex::<Test>::insert(
				2,
				old_storage::MiningRegistration {
					account_id: 2,
					obligation_id: Some(2),
					bond_amount: 100u128,
					authority_keys: 2u64.into(),
					ownership_tokens: 100u128,
					reward_destination: Owner,
					reward_sharing: None,
					slot_id: 2,
				},
			);

			old_storage::NextSlotCohort::<Test>::put(BoundedVec::truncate_from(vec![
				old_storage::MiningRegistration {
					account_id: 3,
					obligation_id: None,
					bond_amount: 100u128,
					authority_keys: 2u64.into(),
					ownership_tokens: 100u128,
					reward_destination: Owner,
					reward_sharing: None,
					slot_id: 3,
				},
			]));
			old_storage::CurrentSlotId::<Test>::put(2);

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrateV2Tov3::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};

			// Execute the migration
			let weight = InnerMigrateV2Tov3::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrateV2Tov3::<Test>::post_upgrade(bytes));

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(4, 4));

			// After the migration, the new value should be set as the `current` value.
			assert_eq!(
				crate::NextSlotCohort::<Test>::get()
					.iter()
					.map(|a| (a.cohort_id, a.account_id))
					.collect::<Vec<_>>(),
				vec![(3, 3)]
			);
			let new_value = crate::ActiveMinersByIndex::<Test>::get(1).unwrap();
			assert_eq!(
				new_value,
				MiningRegistration {
					account_id: 1,
					obligation_id: Some(1),
					bonded_argons: 100u128,
					authority_keys: 1u64.into(),
					argonots: 100u128,
					reward_destination: Owner,
					reward_sharing: None,
					cohort_id: 1,
				},
			);
			assert_eq!(crate::ActiveMinersByIndex::<Test>::get(2).unwrap().cohort_id, 2);
			assert_eq!(crate::LastActivatedCohortId::<Test>::get(), 2);
		})
	}
}
