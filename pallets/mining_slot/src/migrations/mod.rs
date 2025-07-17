use crate::{
	Config, Registration,
	pallet::{
		AccountIndexLookup, BidsForNextSlotCohort, MinerXorKeysByCohort, MinersByCohort,
		NextCohortSize, NextFrameId,
	},
};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use crate::Config;
	use argon_primitives::block_seal::FrameId;
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
		pub next_cohort: Vec<Registration<T>>,
		pub active_miners_by_index: Vec<(MinerIndex, Registration<T>)>,
		pub xor_keys: Vec<(MinerIndex, U256)>,
		pub account_index_lookup: Vec<(T::AccountId, MinerIndex)>,
		pub next_cohort_id: FrameId,
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

	type MinerIndex = u32;

	#[storage_alias]
	pub(super) type NextSlotCohort<T: Config> = StorageValue<
		crate::Pallet<T>,
		BoundedVec<Registration<T>, <T as Config>::MaxCohortSize>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type ActiveMinersByIndex<T: Config> =
		StorageMap<crate::Pallet<T>, Blake2_128Concat, MinerIndex, Registration<T>, OptionQuery>;

	#[storage_alias]
	pub(super) type MinerXorKeyByIndex<T: Config> = StorageValue<
		crate::Pallet<T>,
		BoundedBTreeMap<MinerIndex, U256, ConstU32<100>>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type AccountIndexLookup<T: Config> = StorageMap<
		crate::Pallet<T>,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId,
		MinerIndex,
		OptionQuery,
	>;

	#[storage_alias]
	pub(super) type NextCohortId<T: Config> = StorageValue<crate::Pallet<T>, FrameId, ValueQuery>;
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let next_cohort = old_storage::NextSlotCohort::<T>::get().into_iter().collect::<Vec<_>>();
		let next_cohort_id = old_storage::NextCohortId::<T>::get();
		let active_miners_by_index =
			old_storage::ActiveMinersByIndex::<T>::iter().collect::<Vec<_>>();
		let xor_keys = old_storage::MinerXorKeyByIndex::<T>::get().into_iter().collect::<Vec<_>>();
		let account_index_lookup = old_storage::AccountIndexLookup::<T>::iter().collect::<Vec<_>>();

		Ok(old_storage::Model::<T> {
			next_cohort,
			active_miners_by_index,
			xor_keys,
			account_index_lookup,
			next_cohort_id,
		}
		.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating mining slot pallet storage");
		let old = old_storage::NextSlotCohort::<T>::take();
		let new = old
			.into_iter()
			.map(|x| Registration::<T> {
				account_id: x.account_id,
				external_funding_account: x.external_funding_account,
				bid: x.bid,
				argonots: x.argonots,
				authority_keys: x.authority_keys,
				starting_frame_id: x.cohort_id,
				bid_at_tick: x.bid_at_tick,
			})
			.collect::<Vec<_>>();
		BidsForNextSlotCohort::<T>::put(BoundedVec::truncate_from(new));
		count += 1;

		let old_miners = old_storage::ActiveMinersByIndex::<T>::drain().collect::<Vec<_>>();
		let _ = old_storage::AccountIndexLookup::<T>::drain();
		let old_xor = old_storage::MinerXorKeyByIndex::<T>::take();

		for (old_index, old_reg) in old_miners {
			count += 3;
			let frame_id = old_reg.cohort_id;
			let xor = old_xor.get(&old_index).unwrap();
			// store the index lookup
			let mut new_index = 0;
			MinersByCohort::<T>::mutate(frame_id, |a| {
				new_index = a.len();
				a.try_push(Registration::<T> {
					account_id: old_reg.account_id.clone(),
					external_funding_account: old_reg.external_funding_account,
					bid: old_reg.bid,
					argonots: old_reg.argonots,
					authority_keys: old_reg.authority_keys,
					starting_frame_id: frame_id,
					bid_at_tick: old_reg.bid_at_tick,
				})
				.expect("Failed to insert miner");
			});
			AccountIndexLookup::<T>::insert(old_reg.account_id, (frame_id, new_index as u32));
			// Store the miner xor key
			MinerXorKeysByCohort::<T>::mutate(|a| {
				if !a.contains_key(&frame_id) {
					a.try_insert(frame_id, Default::default()).expect("Failed to insert xor key");
				}
				a.get_mut(&frame_id).unwrap().try_insert(new_index, *xor).unwrap()
			});
		}

		log::info!("{} mining registrations migrated", count);

		let frame_id = old_storage::NextCohortId::<T>::take();
		NextFrameId::<T>::set(frame_id);
		count += 1;

		NextCohortSize::<T>::set(<T as Config>::MinCohortSize::get());

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
		let new_active_miners_by_index = MinersByCohort::<T>::iter()
			.flat_map(|(frame_id, miners)| {
				miners
					.into_iter()
					.enumerate()
					.map(move |(i, reg)| ((frame_id, i as u32), reg.clone()))
			})
			.collect::<Vec<_>>();
		ensure!(
			old_active_miners_by_index.len() == new_active_miners_by_index.len(),
			"New active miners value not set correctly"
		);

		let old_xor_keys = old.xor_keys;
		let new_xor_keys = MinerXorKeysByCohort::<T>::get()
			.into_iter()
			.flat_map(|(frame_id, keys)| {
				keys.into_iter().enumerate().map(move |(i, xor)| ((frame_id, i as u32), xor))
			})
			.collect::<Vec<_>>();
		ensure!(old_xor_keys.len() == new_xor_keys.len(), "New xor keys value not set correctly");

		let old_account_index_lookup = old.account_index_lookup;
		let new_account_index_lookup = AccountIndexLookup::<T>::iter().collect::<Vec<_>>();
		ensure!(
			old_account_index_lookup.len() == new_account_index_lookup.len(),
			"New account index lookup value not set correctly"
		);

		let next_frame_id = NextFrameId::<T>::get();
		ensure!(next_frame_id == old.next_cohort_id, "Next cohort id not set correctly");

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
	use crate::mock::{Test, new_test_ext};
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
			old_storage::MinerXorKeyByIndex::<Test>::mutate(|a| {
				a.try_insert(1, U256::from(1u8)).unwrap();
				a.try_insert(2, U256::from(2u8)).unwrap();
			});

			old_storage::AccountIndexLookup::<Test>::insert(1, 1);
			old_storage::ActiveMinersByIndex::<Test>::insert(
				1,
				old_storage::Registration::<Test> {
					account_id: 1,
					external_funding_account: None,
					argonots: 1,
					bid: 101u128,
					bid_at_tick: 1,
					authority_keys: 1u64.into(),
					reward_destination: Owner,
					cohort_id: 1,
				},
			);
			old_storage::AccountIndexLookup::<Test>::insert(2, 2);
			old_storage::ActiveMinersByIndex::<Test>::insert(
				2,
				old_storage::Registration::<Test> {
					account_id: 2,
					external_funding_account: None,
					argonots: 2,
					bid: 102u128,
					bid_at_tick: 2,
					authority_keys: 2u64.into(),
					reward_destination: Owner,
					cohort_id: 2,
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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(8, 8));

			// Check the new value
			let new = BidsForNextSlotCohort::<Test>::get();
			assert_eq!(new.len(), 2);
			assert_eq!(new[0].account_id, 1);
			assert_eq!(new[1].account_id, 2);

			let new_miners_by_cohort = MinersByCohort::<Test>::iter().collect::<Vec<_>>();
			assert_eq!(new_miners_by_cohort.len(), 2);

			assert_eq!(MinersByCohort::<Test>::get(&1).len(), 1);
			assert_eq!(MinersByCohort::<Test>::get(&1)[0].account_id, 1);

			assert_eq!(MinersByCohort::<Test>::get(&2).len(), 1);
			assert_eq!(MinersByCohort::<Test>::get(&2)[0].account_id, 2);

			// Check the new xor keys
			let new_xor_keys = MinerXorKeysByCohort::<Test>::get();
			assert_eq!(new_xor_keys.len(), 2);
			assert_eq!(new_xor_keys.get(&1).unwrap().to_vec(), vec![U256::from(1u8)]);
			assert_eq!(new_xor_keys.get(&2).unwrap().to_vec(), vec![U256::from(2u8)]);

			// Check the new account index lookup
			let new_account_index_lookup = AccountIndexLookup::<Test>::iter().collect::<Vec<_>>();
			assert_eq!(new_account_index_lookup.len(), 2);

			assert_eq!(AccountIndexLookup::<Test>::get(&1), Some((1, 0)));
			assert_eq!(AccountIndexLookup::<Test>::get(&2), Some((2, 0)));
		});
	}
}
