use crate::{Config, Pallet};
#[cfg(feature = "try-runtime")]
use alloc::collections::BTreeMap;
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::{storage::migration, *};

mod old_storage {
	use crate::{Config, TreasuryPool};
	use alloc::collections::BTreeMap;
	use core::marker::PhantomData;
	use pallet_prelude::{BoundedBTreeMap, FrameId, StorageMap, Twox64Concat, ValueQuery, VaultId};
	use polkadot_sdk::frame_support;
	#[allow(dead_code)]
	pub struct LiquidityPoolPrefix<T>(PhantomData<T>);

	impl<T: Config> frame_support::traits::StorageInstance for LiquidityPoolPrefix<T> {
		fn pallet_prefix() -> &'static str {
			"LiquidityPools"
		}
		const STORAGE_PREFIX: &'static str = "VaultPoolsByFrame";
	}

	#[allow(dead_code)]
	pub type VaultPoolsByFrame<T> = StorageMap<
		LiquidityPoolPrefix<T>,
		Twox64Concat,
		FrameId,
		BoundedBTreeMap<VaultId, TreasuryPool<T>, <T as Config>::MaxBidPoolVaultParticipants>,
		ValueQuery,
	>;

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub vault_pools_by_frame: BTreeMap<
			FrameId,
			BoundedBTreeMap<VaultId, TreasuryPool<T>, T::MaxBidPoolVaultParticipants>,
		>,
	}
}
pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use pallet_prelude::Encode;
		let vault_pools_by_frame =
			old_storage::VaultPoolsByFrame::<T>::iter().collect::<BTreeMap<_, _>>();
		Ok(old_storage::Model { vault_pools_by_frame }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let count = 1;

		migration::move_pallet(b"LiquidityPools", Pallet::<T>::storage_metadata().prefix.as_ref());

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use crate::VaultPoolsByFrame;
		use pallet_prelude::Decode;

		let old = <old_storage::Model<T>>::decode(&mut &state[..])
			.map_err(|_| {
				sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
			})?
			.vault_pools_by_frame;

		let new = VaultPoolsByFrame::<T>::iter().collect::<BTreeMap<_, _>>();
		assert_eq!(old, new, "Storage values do not match after migration");
		for (frame_id, pools) in new {
			log::info!("Frame ID: {}, Pools: {:?}", frame_id, pools);
			let old_pools = old.get(&frame_id).expect("Vault pools not found");
			assert_eq!(pools.len(), old_pools.len(), "Mismatch in pools for frame ID {}", frame_id);
			for (vault_id, pool) in pools {
				let old_pool = old_pools.get(&vault_id).ok_or_else(|| {
					sp_runtime::TryRuntimeError::Other("Missing vault ID in old storage")
				})?;
				assert_eq!(pool, *old_pool, "Mismatch in pool data for vault ID {}", vault_id);
			}
		}

		Ok(())
	}
}

pub type PalletMigrate<T> = frame_support::migrations::VersionedMigration<
	0,
	1,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::{
		VaultPoolsByFrame,
		mock::{Test, new_test_ext},
	};
	use frame_support::assert_ok;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			old_storage::VaultPoolsByFrame::<Test>::mutate(1, |a| {
				a.try_insert(
					42,
					crate::TreasuryPool {
						contributor_balances: BoundedVec::truncate_from(vec![
							(1u64.into(), 1_000u128.into()),
							(2, 2_000),
						]),
						do_not_renew: Default::default(),
						distributed_profits: None,
						vault_sharing_percent: Permill::from_percent(10),
						is_rolled_over: false,
					},
				)
				.unwrap()
			});
			old_storage::VaultPoolsByFrame::<Test>::mutate(2, |a| {
				a.try_insert(
					43,
					crate::TreasuryPool {
						contributor_balances: BoundedVec::truncate_from(vec![
							(3u64.into(), 3_000u128.into()),
							(4, 4_000),
						]),
						do_not_renew: Default::default(),
						distributed_profits: None,
						vault_sharing_percent: Permill::from_percent(20),
						is_rolled_over: false,
					},
				)
				.unwrap()
			});

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrate::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};
			// Execute the migration
			let _weight = InnerMigrate::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrate::<Test>::post_upgrade(bytes));

			// Verify the new storage has the expected values
			let new_value_1 = VaultPoolsByFrame::<Test>::get(1);
			let new_value_2 = VaultPoolsByFrame::<Test>::get(2);
			assert_eq!(new_value_1.len(), 1);
			assert_eq!(new_value_2.len(), 1);
			assert_eq!(new_value_1.get(&42).unwrap().contributor_balances.len(), 2);
			assert_eq!(new_value_2.get(&43).unwrap().contributor_balances.len(), 2);
		});
	}
}
