use crate::{
	pallet::{CapitalActive, CapitalRaising, VaultPoolsByFrame},
	Config,
};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use crate::{Config, LiquidityPool, LiquidityPoolCapital, Pallet};
	use argon_primitives::block_seal::FrameId;
	use frame_support_procedural::storage_alias;
	use pallet_prelude::*;

	#[storage_alias]
	pub(super) type LiquidityPoolsByCohort<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		FrameId,
		BoundedBTreeMap<VaultId, LiquidityPool<T>, <T as Config>::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type OpenLiquidityPoolCapital<T: Config> = StorageValue<
		Pallet<T>,
		BoundedVec<LiquidityPoolCapital<T>, <T as Config>::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type NextLiquidityPoolCapital<T: Config> = StorageValue<
		Pallet<T>,
		BoundedVec<LiquidityPoolCapital<T>, <T as Config>::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok(vec![])
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating liquidity pools");
		let old = old_storage::NextLiquidityPoolCapital::<T>::take();
		CapitalRaising::<T>::set(old);
		count += 1;
		let old = old_storage::OpenLiquidityPoolCapital::<T>::take();
		CapitalActive::<T>::set(old);
		count += 1;
		let old = old_storage::LiquidityPoolsByCohort::<T>::drain();
		for (id, pools) in old {
			count += 1;
			VaultPoolsByFrame::<T>::insert(id, pools);
		}

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		Ok(())
	}
}

pub type NamingMigration<T> = frame_support::migrations::VersionedMigration<
	0, // The migration will only execute when the on-chain storage version is 1
	1, // The on-chain storage version will be set to 2 after the migration is complete
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
		LiquidityPool, LiquidityPoolCapital,
	};
	use frame_support::assert_ok;
	use polkadot_sdk::sp_core::bounded_vec;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			old_storage::LiquidityPoolsByCohort::<Test>::mutate(1, |v| {
				let _ = v.try_insert(
					1,
					LiquidityPool {
						contributor_balances: bounded_vec![
							(1, 100u128.into()),
							(2, 200u128.into()),
						],
						is_rolled_over: false,
						distributed_profits: 0u128.into(),
						do_not_renew: Default::default(),
						vault_sharing_percent: Permill::from_percent(20),
					},
				);
				let _ = v.try_insert(
					2,
					LiquidityPool {
						contributor_balances: bounded_vec![
							(1, 101u128.into()),
							(2, 201u128.into()),
						],
						is_rolled_over: false,
						distributed_profits: 0u128.into(),
						do_not_renew: Default::default(),
						vault_sharing_percent: Permill::from_percent(20),
					},
				);
			});

			old_storage::NextLiquidityPoolCapital::<Test>::mutate(|x| {
				let _ = x.try_push(LiquidityPoolCapital {
					vault_id: 1,
					activated_capital: 5000u128.into(),
					frame_id: 2,
				});
				let _ = x.try_push(LiquidityPoolCapital {
					vault_id: 2,
					activated_capital: 6000u128.into(),
					frame_id: 2,
				});
			});
			old_storage::OpenLiquidityPoolCapital::<Test>::mutate(|x| {
				let _ = x.try_push(LiquidityPoolCapital {
					vault_id: 1,
					activated_capital: 4000u128.into(),
					frame_id: 1,
				});
				let _ = x.try_push(LiquidityPoolCapital {
					vault_id: 2,
					activated_capital: 7000u128.into(),
					frame_id: 1,
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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 3));

			// Check the new value
			let new = VaultPoolsByFrame::<Test>::get(1);
			assert_eq!(new.len(), 2);
			assert_eq!(new.get(&1).unwrap().contributor_balances[0].1, 100);
			assert_eq!(new.get(&1).unwrap().contributor_balances[1].1, 200);
			assert_eq!(new.get(&2).unwrap().contributor_balances[0].1, 101);
			assert_eq!(new.get(&2).unwrap().contributor_balances[1].1, 201);

			let new = CapitalRaising::<Test>::get();
			assert_eq!(new.len(), 2);
			assert_eq!(new.get(0).unwrap().vault_id, 1);
			assert_eq!(new.get(0).unwrap().activated_capital, 5000u128.into());
			assert_eq!(new.get(0).unwrap().frame_id, 2);
			assert_eq!(new.get(1).unwrap().vault_id, 2);
			assert_eq!(new.get(1).unwrap().activated_capital, 6000u128.into());
			assert_eq!(new.get(1).unwrap().frame_id, 2);

			let new = CapitalActive::<Test>::get();
			assert_eq!(new.len(), 2);
			assert_eq!(new.get(0).unwrap().vault_id, 1);
			assert_eq!(new.get(0).unwrap().activated_capital, 4000u128.into());
			assert_eq!(new.get(0).unwrap().frame_id, 1);
			assert_eq!(new.get(1).unwrap().vault_id, 2);
			assert_eq!(new.get(1).unwrap().activated_capital, 7000u128.into());
			assert_eq!(new.get(1).unwrap().frame_id, 1);
		});
	}
}
