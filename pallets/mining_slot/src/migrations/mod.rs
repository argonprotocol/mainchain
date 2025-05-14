use crate::{
	pallet::{ActiveMinersByIndex, BidsForNextSlotCohort, NextFrameId},
	Config, Registration,
};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use crate::Config;
	use argon_primitives::block_seal::{FrameId, MinerIndex};
	use frame_support_procedural::storage_alias;
	use pallet_prelude::*;
	use sp_runtime::traits::OpaqueKeys;

	pub type Registration<T> = MiningRegistration<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
		<T as Config>::Keys,
	>;

	/// A destination account for validator rewards
	#[derive(
		PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default,
	)]
	pub enum RewardDestination<AccountId> {
		#[default]
		Owner,
		/// Pay into a specified account.
		Account(AccountId),
	}

	#[cfg(feature = "try-runtime")]
	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub next_cohort: BoundedVec<Registration<T>, <T as Config>::MaxMiners>,
		pub active_miners_by_index: Vec<(u32, Registration<T>)>,
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
		/// The account id the miner will operate as
		pub account_id: AccountId,
		/// The account that bids and argonots come from
		pub external_funding_account: Option<AccountId>,
		/// The account that rewards are paid to
		pub reward_destination: RewardDestination<AccountId>,
		/// How much was bid for the mining slot
		#[codec(compact)]
		pub bid: Balance,
		/// The argonots put on hold to run a mining seat
		#[codec(compact)]
		pub argonots: Balance,
		/// The signing keys for the miner
		pub authority_keys: Keys,
		/// Which cohort the miner is in
		#[codec(compact)]
		pub cohort_id: FrameId,
		/// When the bid was placed
		#[codec(compact)]
		pub bid_at_tick: Tick,
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
	pub(super) type NextFrameId<T: Config> = StorageValue<crate::Pallet<T>, FrameId, ValueQuery>;
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

		Ok(old_storage::Model::<T> { next_cohort, active_miners_by_index }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating mining slot pallet storage");
		let old = old_storage::NextSlotCohort::<T>::take();
		let new = old
			.into_iter()
			.map(|x| Registration::<T> {
				account_id: x.account_id,
				external_funding_account: None,
				bid: x.bid,
				argonots: x.argonots,
				authority_keys: x.authority_keys,
				cohort_frame_id: x.cohort_id,
				bid_at_tick: x.bid_at_tick,
			})
			.collect::<Vec<_>>();
		BidsForNextSlotCohort::<T>::put(BoundedVec::truncate_from(new));
		count += 1;
		ActiveMinersByIndex::<T>::translate::<old_storage::Registration<T>, _>(|_id, reg| {
			count += 1;
			Some(Registration::<T> {
				account_id: reg.account_id,
				external_funding_account: None,
				bid: reg.bid,
				argonots: reg.argonots,
				authority_keys: reg.authority_keys,
				cohort_frame_id: reg.cohort_id,
				bid_at_tick: reg.bid_at_tick,
			})
		});
		log::info!("{} mining registrations migrated", count);

		let frame_id = old_storage::NextFrameId::<T>::take();
		NextFrameId::<T>::set(frame_id);
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

		let new = BidsForNextSlotCohort::<T>::get();
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

pub type FrameMigration<T> = frame_support::migrations::VersionedMigration<
	6,
	7,
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::assert_ok;
	use old_storage::RewardDestination::Owner;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			old_storage::NextSlotCohort::<Test>::mutate(|v| {
				let _ = v.try_push(old_storage::Registration::<Test> {
					account_id: 1,
					external_funding_account: None,
					argonots: 1,
					bid: 100u128,
					bid_at_tick: 1,
					authority_keys: 1u64.into(),
					reward_destination: Owner,
					cohort_id: 3,
				});
				let _ = v.try_push(old_storage::Registration::<Test> {
					account_id: 2,
					external_funding_account: Some(3),
					argonots: 2,
					bid: 101u128,
					bid_at_tick: 2,
					authority_keys: 2u64.into(),
					reward_destination: Owner,
					cohort_id: 3,
				});
			});

			old_storage::ActiveMinersByIndex::<Test>::insert(
				0,
				old_storage::Registration::<Test> {
					account_id: 5,
					external_funding_account: None,
					argonots: 1,
					bid: 102u128,
					bid_at_tick: 1,
					authority_keys: 1u64.into(),
					reward_destination: Owner,
					cohort_id: 2,
				},
			);
			old_storage::ActiveMinersByIndex::<Test>::insert(
				1,
				old_storage::Registration::<Test> {
					account_id: 6,
					external_funding_account: None,
					argonots: 1,
					bid: 123u128,
					bid_at_tick: 2,
					authority_keys: 1u64.into(),
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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(4, 4));

			// Check the new value
			let new = BidsForNextSlotCohort::<Test>::get();
			assert_eq!(new.len(), 2);
			assert_eq!(new[0].account_id, 1);
			assert_eq!(new[1].account_id, 2);

			let new_active_miners_by_index =
				ActiveMinersByIndex::<Test>::iter().collect::<Vec<_>>();
			assert_eq!(new_active_miners_by_index.len(), 2);
			assert_eq!(new_active_miners_by_index[0].0, 0);
			assert_eq!(new_active_miners_by_index[1].0, 1);
			assert_eq!(new_active_miners_by_index[0].1.bid, 102u128);
			assert_eq!(new_active_miners_by_index[1].1.bid, 123u128);
		});
	}
}
