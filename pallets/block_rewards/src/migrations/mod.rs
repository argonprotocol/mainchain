use crate::{pallet::PayoutsByBlock, Config, FreezeReason};
use frame_support::traits::{fungible::MutateFreeze, UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok(Default::default())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating block rewards to remove maturation period");
		let freeze_id = FreezeReason::MaturationPeriod.into();
		let zero = T::Balance::zero();
		for (_block, unlocks) in PayoutsByBlock::<T>::iter() {
			for reward in unlocks {
				let account_id = reward.account_id;
				if reward.argons > zero {
					count += 1;
					if let Err(e) = T::ArgonCurrency::set_freeze(&freeze_id, &account_id, zero) {
						log::error!(
							"Failed to unfreeze argons for account: {:?}, {:?}",
							account_id,
							e
						);
					}
				}
				if reward.ownership > zero {
					count += 1;
					if let Err(e) = T::OwnershipCurrency::set_freeze(&freeze_id, &account_id, zero)
					{
						log::error!(
							"Failed to unfreeze ownership for account: {:?}, {:?}",
							account_id,
							e
						);
					}
				}
			}
		}
		log::info!("{} rewards unfrozen", count);

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		Ok(())
	}
}

pub type RewardFreezeMigration<T> = frame_support::migrations::VersionedMigration<
	0,
	1,
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::{
		mock::{new_test_ext, Balances, Ownership, Test},
		tests::test_authority,
	};
	use argon_primitives::block_seal::{BlockPayout, BlockRewardType};
	use frame_support::traits::fungible::{InspectFreeze, Mutate};

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			let id = test_authority([1; 32]);
			PayoutsByBlock::<Test>::try_append(
				1,
				BlockPayout {
					account_id: 1,
					argons: 200_000,
					ownership: 100_000,
					reward_type: BlockRewardType::Miner,
					block_seal_authority: Some(id.clone()),
				},
			)
			.unwrap();
			PayoutsByBlock::<Test>::try_append(
				2,
				BlockPayout {
					account_id: 2,
					argons: 200_000,
					ownership: 100_000,
					reward_type: BlockRewardType::Miner,
					block_seal_authority: Some(id),
				},
			)
			.unwrap();

			Ownership::mint_into(&1, 100_000).unwrap();
			Ownership::mint_into(&2, 150_000).unwrap();
			Ownership::set_freeze(&FreezeReason::MaturationPeriod.into(), &1, 100_000).unwrap();
			Ownership::set_freeze(&FreezeReason::MaturationPeriod.into(), &2, 100_000).unwrap();
			Balances::mint_into(&1, 200_000).unwrap();
			Balances::mint_into(&2, 300_000).unwrap();
			Balances::set_freeze(&FreezeReason::MaturationPeriod.into(), &1, 200_000).unwrap();
			Balances::set_freeze(&FreezeReason::MaturationPeriod.into(), &2, 200_000).unwrap();
			assert_eq!(PayoutsByBlock::<Test>::iter().count(), 2);

			// Execute the migration
			let weight = InnerMigrate::<Test>::on_runtime_upgrade();

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(4, 4));

			assert_eq!(PayoutsByBlock::<Test>::iter().count(), 2);
			assert_eq!(Balances::free_balance(&1), 200_000);
			assert_eq!(Balances::free_balance(&2), 300_000);
			assert_eq!(Ownership::free_balance(&1), 100_000);
			assert_eq!(Ownership::free_balance(&2), 150_000);
			assert_eq!(Balances::balance_frozen(&FreezeReason::MaturationPeriod.into(), &1), 0);
			assert_eq!(Balances::balance_frozen(&FreezeReason::MaturationPeriod.into(), &2), 0);
			assert_eq!(Ownership::balance_frozen(&FreezeReason::MaturationPeriod.into(), &1), 0);
			assert_eq!(Ownership::balance_frozen(&FreezeReason::MaturationPeriod.into(), &2), 0);
		});
	}
}
