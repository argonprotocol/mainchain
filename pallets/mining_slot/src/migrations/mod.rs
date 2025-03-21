use crate::{
	pallet::{ActiveMinersByIndex, MinerXorKeyByIndex, NextSlotCohort},
	Config, Registration,
};
use alloc::vec::Vec;
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};
use log::info;

mod old_storage {
	use crate::Config;
	use argon_primitives::{
		block_seal::{CohortId, MinerIndex, RewardDestination},
		ObligationId,
	};

	use crate::runtime_decl_for_mining_slot_api::U256;
	#[cfg(feature = "try-runtime")]
	use alloc::vec::Vec;
	use frame_support::{pallet_prelude::*, storage_alias, BoundedVec};
	use sp_runtime::{traits::OpaqueKeys, FixedU128};

	pub type Registration<T> = MiningRegistration<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
		<T as Config>::Keys,
	>;

	#[cfg(feature = "try-runtime")]
	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub next_cohort: BoundedVec<Registration<T>, <T as Config>::MaxMiners>,
		pub active_miners_by_index: Vec<(u32, Registration<T>)>,
		pub old_hash_count: u32,
	}

	#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct RewardSharing<AccountId> {
		pub account_id: AccountId,
		#[codec(compact)]
		pub percent_take: FixedU128,
	}

	#[derive(
		PartialEqNoBound,
		EqNoBound,
		CloneNoBound,
		Encode,
		Decode,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(MaxHosts))]
	pub struct MiningRegistration<
		AccountId: Parameter,
		Balance: Parameter + MaxEncodedLen,
		Keys: OpaqueKeys + Parameter,
	> {
		pub account_id: AccountId,
		pub reward_destination: RewardDestination<AccountId>,
		pub obligation_id: Option<ObligationId>,
		#[codec(compact)]
		pub bonded_argons: Balance,
		#[codec(compact)]
		pub argonots: Balance,
		pub reward_sharing: Option<RewardSharing<AccountId>>,
		pub authority_keys: Keys,
		#[codec(compact)]
		pub cohort_id: CohortId,
	}

	#[storage_alias]
	pub(super) type NextSlotCohort<T: Config> = StorageValue<
		crate::Pallet<T>,
		BoundedVec<Registration<T>, <T as Config>::MaxMiners>,
		ValueQuery,
	>;
	#[storage_alias]
	pub(super) type ActiveMinersByIndex<T: Config> =
		StorageMap<crate::Pallet<T>, Blake2_128Concat, MinerIndex, Registration<T>, OptionQuery>;

	#[storage_alias]
	pub(super) type AuthorityHashByIndex<T: Config> = StorageValue<
		crate::Pallet<T>,
		BoundedBTreeMap<MinerIndex, U256, <T as Config>::MaxMiners>,
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
		let active_miners_by_index =
			old_storage::ActiveMinersByIndex::<T>::iter().collect::<Vec<_>>();

		let old_hash_count = old_storage::AuthorityHashByIndex::<T>::get().len() as u32;

		Ok(old_storage::Model::<T> { next_cohort, active_miners_by_index, old_hash_count }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		info!("Migrating mining slot pallet storage");
		let old = old_storage::NextSlotCohort::<T>::take();
		let new = old
			.into_iter()
			.map(|x| Registration::<T> {
				account_id: x.account_id,
				external_funding_account: None,
				reward_destination: x.reward_destination,
				bid: x.bonded_argons,
				argonots: x.argonots,
				authority_keys: x.authority_keys,
				cohort_id: x.cohort_id,
			})
			.collect::<Vec<_>>();
		NextSlotCohort::<T>::put(BoundedVec::truncate_from(new));
		count += 1;
		ActiveMinersByIndex::<T>::translate::<old_storage::Registration<T>, _>(|_id, reg| {
			count += 1;
			Some(Registration::<T> {
				account_id: reg.account_id,
				external_funding_account: None,
				reward_destination: reg.reward_destination,
				bid: reg.bonded_argons,
				argonots: reg.argonots,
				authority_keys: reg.authority_keys,
				cohort_id: reg.cohort_id,
			})
		});
		info!("{} mining registrations migrated", count);

		let authority_hash = old_storage::AuthorityHashByIndex::<T>::take();
		MinerXorKeyByIndex::<T>::set(authority_hash);
		count += 2;

		old_storage::AuthorityHashByIndex::<T>::take();
		count += 1;

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
		ensure!(old.next_cohort.len() == new.len(), "New next cohort not set correctly");

		let old_active_miners_by_index = old.active_miners_by_index;
		let new_active_miners_by_index = ActiveMinersByIndex::<T>::iter().collect::<Vec<_>>();
		ensure!(
			old_active_miners_by_index.len() == new_active_miners_by_index.len(),
			"New active miners value not set correctly"
		);

		let old_hash_count = old.old_hash_count;
		let new_hash_count = MinerXorKeyByIndex::<T>::get().len() as u32;
		ensure!(old_hash_count == new_hash_count, "New hash count not set correctly");

		Ok(())
	}
}

pub type BiddingMigration<T> = frame_support::migrations::VersionedMigration<
	4, // The migration will only execute when the on-chain storage version is 1
	5, // The on-chain storage version will be set to 2 after the migration is complete
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
	use sp_core::bounded_btree_map;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			old_storage::NextSlotCohort::<Test>::mutate(|v| {
				let _ = v.try_push(old_storage::Registration::<Test> {
					account_id: 1,
					obligation_id: Some(1),
					argonots: 1,
					bonded_argons: 100u128,
					authority_keys: 1u64.into(),
					reward_destination: Owner,
					reward_sharing: None,
					cohort_id: 1,
				});
				let _ = v.try_push(old_storage::Registration::<Test> {
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

			old_storage::ActiveMinersByIndex::<Test>::insert(
				0,
				old_storage::Registration::<Test> {
					account_id: 1,
					obligation_id: Some(1),
					argonots: 1,
					bonded_argons: 100u128,
					authority_keys: 1u64.into(),
					reward_destination: Owner,
					reward_sharing: None,
					cohort_id: 1,
				},
			);
			old_storage::ActiveMinersByIndex::<Test>::insert(
				1,
				old_storage::Registration::<Test> {
					account_id: 2,
					obligation_id: Some(2),
					argonots: 1000,
					bonded_argons: 123u128,
					authority_keys: 2u64.into(),
					reward_destination: Owner,
					reward_sharing: None,
					cohort_id: 1,
				},
			);

			old_storage::AuthorityHashByIndex::<Test>::set(
				bounded_btree_map!(0 => 100u128.into(), 1 => 123u128.into()),
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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(6, 6));

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
