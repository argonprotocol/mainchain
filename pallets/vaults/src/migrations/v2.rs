use crate::{pallet::VaultsById, Config};
use alloc::vec::Vec;
use argon_primitives::bitcoin::Satoshis;
use frame_support::{migration, pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};
use log::info;

pub mod v1_p2 {
	use crate::Config;
	use argon_primitives::VaultId;
	use frame_support::{pallet_prelude::ValueQuery, storage_alias, Twox64Concat};
	use frame_system::pallet_prelude::BlockNumberFor;
	use sp_core::ConstU32;
	use sp_runtime::BoundedVec;

	#[storage_alias]
	pub(super) type PendingTermsModificationsByBlock<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<VaultId, ConstU32<100>>,
		ValueQuery,
	>;

	#[storage_alias]
	pub(super) type PendingFundingModificationsByBlock<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<VaultId, ConstU32<100>>,
		ValueQuery,
	>;
}

pub struct InnerMigrateV1ToV2<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV1ToV2<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok(Vec::new())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		info!("Clearing testnet data for Vaults from v1 to v2");
		let a = VaultsById::<T>::drain().collect::<Vec<_>>();

		let b = v1_p2::PendingTermsModificationsByBlock::<T>::drain().collect::<Vec<_>>();
		let c = v1_p2::PendingFundingModificationsByBlock::<T>::drain().collect::<Vec<_>>();
		let minimum_satoshis =
			migration::get_storage_value::<Satoshis>(b"bonds", b"MinimumBitcoinBondSatoshis", &[]);
		if let Some(minimum_sats) = minimum_satoshis {
			info!("Setting minimum BitcoinLock satoshis to {}", minimum_sats);
			migration::put_storage_value(b"bitcoin_locks", b"MinimumSatoshis", &[], minimum_sats);
		}
		let result = migration::clear_storage_prefix(b"bonds", &[], &[], None, None);
		let count = (a.len() + b.len() + c.len() + result.backend as usize) as u64;

		T::DbWeight::get().reads_writes(count, count)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;
		use frame_support::ensure;

		let actual_new_value = VaultsById::<T>::iter().collect::<Vec<_>>();

		ensure!(actual_new_value.len() == 0, "New value not set correctly");
		Ok(())
	}
}

pub type MigrateV1ToV2<T> = frame_support::migrations::VersionedMigration<
	1, // The migration will only execute when the on-chain storage version is 0
	2, // The on-chain storage version will be set to 1 after the migration is complete
	InnerMigrateV1ToV2<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::{
		migrations::v1::v1_storage as v1,
		mock::{new_test_ext, CurrentTick, System, Test},
	};
	use argon_primitives::vault::VaultArgons;
	use frame_support::assert_ok;
	use sp_runtime::FixedU128;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			CurrentTick::set(1000);
			System::set_block_number(1000);
			let bonded_argons = VaultArgons {
				allocated: 10,
				base_fee: 0,
				annual_percent_rate: FixedU128::from(0),
				reserved: 10,
			};
			let bitcoin_argons = VaultArgons {
				allocated: 15,
				base_fee: 0,
				annual_percent_rate: FixedU128::from(1),
				reserved: 15,
			};
			let mining_reward_sharing_percent_take = FixedU128::from_float(0.1);
			let added_securitization_percent = FixedU128::from_float(0.1);
			let is_closed = false;

			v1::VaultsById::<Test>::insert(
				1,
				v1::Vault {
					bonded_argons: v1::VaultArgons {
						allocated: bonded_argons.allocated,
						base_fee: bonded_argons.base_fee,
						annual_percent_rate: bonded_argons.annual_percent_rate,
						bonded: bonded_argons.reserved,
					},
					bitcoin_argons: v1::VaultArgons {
						allocated: bitcoin_argons.allocated,
						base_fee: bitcoin_argons.base_fee,
						annual_percent_rate: bitcoin_argons.annual_percent_rate,
						bonded: bitcoin_argons.reserved,
					},
					mining_reward_sharing_percent_take,
					operator_account_id: Default::default(),
					pending_terms: None,
					securitized_argons: 0,
					added_securitization_percent,
					is_closed,
					pending_bonded_argons: Some((1010, 10)),
					pending_bitcoins: 0,
				},
			);
			v1_p2::PendingFundingModificationsByBlock::<Test>::insert(
				1010,
				BoundedVec::truncate_from(vec![1]),
			);

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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(2, 2));

			// After the migration, the new value should be set as the `current` value.
			assert_eq!(crate::VaultsById::<Test>::iter_keys().collect::<Vec<_>>().len(), 0);
			assert_eq!(
				crate::PendingFundingModificationsByTick::<Test>::iter_keys()
					.collect::<Vec<_>>()
					.len(),
				0
			);
		})
	}
}
