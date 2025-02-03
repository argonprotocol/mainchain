use crate::{
	pallet::{ActiveMinersByIndex, MiningConfig, NextSlotCohort},
	Config,
};
use alloc::vec::Vec;
use argon_primitives::block_seal::{MiningRegistration, MiningSlotConfig};
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};
use log::info;
use sp_runtime::traits::UniqueSaturatedInto;

mod v1 {
	use crate::Config;
	use argon_primitives::{
		block_seal::{MinerIndex, RewardDestination, RewardSharing},
		prelude::Tick,
		BlockNumber, BondId,
	};
	use codec::MaxEncodedLen;
	use frame_support::{pallet_prelude::*, storage_alias, Blake2_128Concat, BoundedVec};
	use scale_info::TypeInfo;

	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct MiningRegistration<T: Config> {
		pub account_id: <T as frame_system::Config>::AccountId,
		pub reward_destination: RewardDestination<<T as frame_system::Config>::AccountId>,
		pub bond_id: Option<BondId>,
		#[codec(compact)]
		pub bond_amount: <T as Config>::Balance,
		#[codec(compact)]
		pub ownership_tokens: <T as Config>::Balance,
		pub reward_sharing: Option<RewardSharing<<T as frame_system::Config>::AccountId>>,
		pub authority_keys: <T as Config>::Keys,
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

	#[derive(Encode, Decode, Default, TypeInfo)]
	pub struct MiningSlotConfig {
		/// How many blocks before the end of a slot can the bid close
		#[codec(compact)]
		pub blocks_before_bid_end_for_vrf_close: BlockNumber,
		/// How many ticks transpire between slots
		#[codec(compact)]
		pub blocks_between_slots: BlockNumber,
		/// The tick when bidding will start (eg, Slot "1")
		#[codec(compact)]
		pub slot_bidding_start_after_ticks: Tick,
	}

	#[storage_alias]
	pub(super) type MiningConfig<T: Config> =
		StorageValue<crate::Pallet<T>, MiningSlotConfig, ValueQuery>;

	#[cfg(feature = "try-runtime")]
	#[derive(Encode, Decode, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct MiningStructure<T: Config> {
		pub active_miners: alloc::vec::Vec<(MinerIndex, MiningRegistration<T>)>,
		pub next_cohort: BoundedVec<MiningRegistration<T>, <T as Config>::MaxCohortSize>,
		pub mining_config: MiningSlotConfig,
	}
}

pub struct InnerMigrateV1ToV2<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV1ToV2<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let active_miners = v1::ActiveMinersByIndex::<T>::iter().collect::<Vec<_>>();
		let next_cohort = v1::NextSlotCohort::<T>::get();
		let mining_config = v1::MiningConfig::<T>::get();
		// Return it as an encoded `Vec<u8>`
		Ok(v1::MiningStructure { mining_config, active_miners, next_cohort }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		info!("Migrating Mining Slots from v1 to v2");
		ActiveMinersByIndex::<T>::translate::<v1::MiningRegistration<T>, _>(|id, reg| {
			info!("Migration: Translating mining registration with id {:?}", id);
			count += 1;
			Some(MiningRegistration {
				account_id: reg.account_id,
				reward_destination: reg.reward_destination,
				bond_id: reg.bond_id,
				bond_amount: reg.bond_amount,
				ownership_tokens: reg.ownership_tokens,
				reward_sharing: reg.reward_sharing,
				authority_keys: reg.authority_keys,
				// since this is only possible on testnet, we'll set to 0
				slot_id: 0,
			})
		});
		let _ = NextSlotCohort::<T>::translate::<
			BoundedVec<v1::MiningRegistration<T>, <T as Config>::MaxCohortSize>,
			_,
		>(|cohort| {
			if let Some(cohort) = cohort {
				count += 1;
				let next = cohort
					.into_iter()
					.map(|reg| {
						MiningRegistration {
							account_id: reg.account_id,
							reward_destination: reg.reward_destination,
							bond_id: reg.bond_id,
							bond_amount: reg.bond_amount,
							ownership_tokens: reg.ownership_tokens,
							reward_sharing: reg.reward_sharing,
							authority_keys: reg.authority_keys,
							// since this is only possible on testnet, we'll set to 0
							slot_id: 0,
						}
					})
					.collect::<Vec<_>>();
				return Some(BoundedVec::truncate_from(next));
			}
			None
		});

		MiningConfig::<T>::translate::<v1::MiningSlotConfig, _>(|config| {
			let config = config.unwrap();
			Some(MiningSlotConfig {
				slot_bidding_start_after_ticks: config.slot_bidding_start_after_ticks,
				ticks_between_slots: UniqueSaturatedInto::<u64>::unique_saturated_into(
					config.blocks_between_slots,
				),
				ticks_before_bid_end_for_vrf_close:
					UniqueSaturatedInto::<u64>::unique_saturated_into(
						config.blocks_before_bid_end_for_vrf_close,
					),
			})
		})
		.expect("Should translate");

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;
		use sp_core::Decode;

		let v1::MiningStructure {
			active_miners: old_active_miners,
			next_cohort: old_next_cohort,
			mining_config: old_config,
		} = <v1::MiningStructure<T>>::decode(&mut &state[..]).map_err(|_| {
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
		ensure!(
			old_config.blocks_before_bid_end_for_vrf_close as u32 ==
				old_config.blocks_before_bid_end_for_vrf_close,
			"vrf matches"
		);
		Ok(())
	}
}

pub type MigrateV1ToV2<T> = frame_support::migrations::VersionedMigration<
	1, // The migration will only execute when the on-chain storage version is 1
	2, // The on-chain storage version will be set to 2 after the migration is complete
	InnerMigrateV1ToV2<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrateV1ToV2;
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use argon_primitives::block_seal::RewardDestination::Owner;
	use frame_support::assert_ok;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			v1::ActiveMinersByIndex::<Test>::insert(
				1,
				v1::MiningRegistration {
					account_id: 1,
					bond_id: Some(1),
					bond_amount: 100u128,
					authority_keys: 1u64.into(),
					ownership_tokens: 100u128,
					reward_destination: Owner,
					reward_sharing: None,
				},
			);
			v1::ActiveMinersByIndex::<Test>::insert(
				2,
				v1::MiningRegistration {
					account_id: 2,
					bond_id: Some(2),
					bond_amount: 100u128,
					authority_keys: 2u64.into(),
					ownership_tokens: 100u128,
					reward_destination: Owner,
					reward_sharing: None,
				},
			);

			v1::NextSlotCohort::<Test>::put(BoundedVec::truncate_from(vec![
				v1::MiningRegistration {
					account_id: 3,
					bond_id: None,
					bond_amount: 100u128,
					authority_keys: 2u64.into(),
					ownership_tokens: 100u128,
					reward_destination: Owner,
					reward_sharing: None,
				},
			]));

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrateV1ToV2::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};

			// Execute the migration
			let weight = InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrateV1ToV2::<Test>::post_upgrade(bytes));

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 3));

			// After the migration, the new value should be set as the `current` value.
			assert_eq!(
				crate::NextSlotCohort::<Test>::get()
					.iter()
					.map(|a| (a.slot_id, a.account_id))
					.collect::<Vec<_>>(),
				vec![(0, 3)]
			);
			let new_value = crate::ActiveMinersByIndex::<Test>::get(1).unwrap();
			assert_eq!(
				new_value,
				MiningRegistration {
					account_id: 1,
					bond_id: Some(1),
					bond_amount: 100u128,
					authority_keys: 1u64.into(),
					ownership_tokens: 100u128,
					reward_destination: Owner,
					reward_sharing: None,
					slot_id: 0
				},
			);
		})
	}
}
