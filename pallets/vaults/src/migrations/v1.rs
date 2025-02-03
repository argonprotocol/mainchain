use crate::{Config, Pallet};
#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
use argon_primitives::VaultId;
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};
use log::info;

mod v0 {
	use super::*;
	use crate::Config;
	use argon_primitives::{
		bond::{VaultArgons, VaultTerms},
		RewardShare,
	};
	use codec::{Decode, MaxEncodedLen};
	use frame_support::storage_alias;
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_core::{Encode, RuntimeDebug};
	use sp_runtime::FixedU128;

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Vault<T: Config> {
		pub operator_account_id: T::AccountId,
		pub bitcoin_argons: VaultArgons<T::Balance>,
		#[codec(compact)]
		pub securitization_percent: FixedU128,
		#[codec(compact)]
		pub securitized_argons: T::Balance,
		pub mining_argons: VaultArgons<T::Balance>,
		#[codec(compact)]
		pub mining_reward_sharing_percent_take: RewardShare,
		pub is_closed: bool,
		pub pending_terms: Option<(BlockNumberFor<T>, VaultTerms<T::Balance>)>,
	}

	#[storage_alias]
	pub type PendingBitcoinsByVault<T: crate::Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		VaultId,
		<T as crate::Config>::Balance,
		ValueQuery,
	>;

	#[storage_alias]
	pub type VaultsById<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, VaultId, Vault<T>, OptionQuery>;
}

pub mod v1_storage {
	use crate::Config;
	use argon_primitives::{
		bond::{VaultArgons, VaultTerms},
		RewardShare, VaultId,
	};
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		__private::RuntimeDebug,
		pallet_prelude::{OptionQuery, TypeInfo},
		storage_alias, Twox64Concat,
	};
	use frame_system::pallet_prelude::BlockNumberFor;
	use sp_runtime::FixedU128;

	#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Vault<T: Config> {
		pub operator_account_id: T::AccountId,
		pub bitcoin_argons: VaultArgons<T::Balance>,
		#[codec(compact)]
		pub securitization_percent: FixedU128,
		#[codec(compact)]
		pub securitized_argons: T::Balance,
		pub mining_argons: VaultArgons<T::Balance>,
		#[codec(compact)]
		pub mining_reward_sharing_percent_take: RewardShare,
		pub is_closed: bool,
		pub pending_terms: Option<(BlockNumberFor<T>, VaultTerms<T::Balance>)>,
		pub pending_mining_argons: Option<(BlockNumberFor<T>, T::Balance)>,
		pub pending_bitcoins: T::Balance,
	}
	#[storage_alias]
	pub type VaultsById<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, VaultId, Vault<T>, OptionQuery>;
}

pub struct InnerMigrateV0ToV1<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV0ToV1<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let all_values = v0::VaultsById::<T>::iter().collect::<Vec<_>>();
		// Return it as an encoded `Vec<u8>`
		Ok(all_values.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		info!("Migrating Vaults from v0 to v1");
		v1_storage::VaultsById::<T>::translate::<v0::Vault<T>, _>(|id, vault| {
			let pending_bitcoins = v0::PendingBitcoinsByVault::<T>::take(id);
			info!(
				"Migration: Translating vault with id {:?} and pending bitcoins {:?}",
				id, pending_bitcoins
			);
			count += 1;
			let vault = v1_storage::Vault {
				operator_account_id: vault.operator_account_id,
				bitcoin_argons: vault.bitcoin_argons,
				securitization_percent: vault.securitization_percent,
				securitized_argons: vault.securitized_argons,
				mining_argons: vault.mining_argons,
				mining_reward_sharing_percent_take: vault.mining_reward_sharing_percent_take,
				is_closed: vault.is_closed,
				pending_terms: vault.pending_terms,
				pending_mining_argons: None,
				pending_bitcoins,
			};
			Some(vault)
		});

		StorageVersion::new(2).put::<Pallet<T>>();
		T::DbWeight::get().reads_writes(count as u64 + 1, count as u64 + 1)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;
		use frame_support::ensure;

		let old_value = <Vec<(VaultId, v0::Vault<T>)>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		let actual_new_value = v1_storage::VaultsById::<T>::iter().collect::<Vec<_>>();

		ensure!(old_value.len() == actual_new_value.len(), "New value not set correctly");
		for vault in actual_new_value {
			ensure!(old_value.iter().any(|(id, _)| id == &vault.0), "Vault missing in translation");
			ensure!(
				vault.1.pending_mining_argons.is_none(),
				"New value not set cor\
				rectly"
			);
		}
		Ok(())
	}
}

pub type MigrateV0ToV1<T> = frame_support::migrations::VersionedMigration<
	0, // The migration will only execute when the on-chain storage version is 0
	1, // The on-chain storage version will be set to 1 after the migration is complete
	InnerMigrateV0ToV1<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrateV0ToV1;
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use argon_primitives::bond::VaultArgons;
	use frame_support::assert_ok;
	use sp_runtime::FixedU128;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			let mining_argons = VaultArgons {
				allocated: 10,
				base_fee: 0,
				annual_percent_rate: FixedU128::from(0),
				bonded: 10,
			};
			let bitcoin_argons = VaultArgons {
				allocated: 15,
				base_fee: 0,
				annual_percent_rate: FixedU128::from(1),
				bonded: 15,
			};
			v0::PendingBitcoinsByVault::<Test>::insert(1, 10);
			let mining_reward_sharing_percent_take = FixedU128::from_float(0.1);
			let securitization_percent = FixedU128::from_float(0.1);
			let is_closed = false;

			v0::VaultsById::<Test>::insert(
				1,
				v0::Vault {
					mining_argons: mining_argons.clone(),
					bitcoin_argons: bitcoin_argons.clone(),
					mining_reward_sharing_percent_take,
					operator_account_id: Default::default(),
					pending_terms: None,
					securitized_argons: 0,
					securitization_percent,
					is_closed,
				},
			);

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrateV0ToV1::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};

			// Execute the migration
			let weight = InnerMigrateV0ToV1::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrateV0ToV1::<Test>::post_upgrade(bytes));

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(2, 2));

			// After the migration, the new value should be set as the `current` value.
			assert_eq!(crate::VaultsById::<Test>::iter_keys().collect::<Vec<_>>(), vec![1]);
			let new_value = v1_storage::VaultsById::<Test>::get(1).unwrap();
			assert_eq!(
				new_value,
				v1_storage::Vault {
					operator_account_id: Default::default(),
					bitcoin_argons,
					securitization_percent,
					securitized_argons: 0,
					mining_argons,
					is_closed,
					pending_mining_argons: None,
					pending_bitcoins: 10,
					mining_reward_sharing_percent_take,
					pending_terms: None,
				}
			);
		})
	}
}
