use crate::{
	migrations::v1::v1_storage as v1,
	pallet::{PendingFundingModificationsByTick, PendingTermsModificationsByTick, VaultsById},
	Config,
};
use alloc::vec::Vec;
use argon_primitives::{
	prelude::Tick,
	vault::{Vault, VaultArgons},
	TickProvider,
};
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade};
use log::info;
use sp_runtime::{
	traits::{BlockNumberProvider, UniqueSaturatedInto},
	Saturating,
};

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
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let all_values = v1::VaultsById::<T>::iter().collect::<Vec<_>>();
		// Return it as an encoded `Vec<u8>`
		Ok(all_values.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		info!("Migrating Vaults from v1 to v2");
		let current_block = <frame_system::Pallet<T>>::current_block_number();
		let current_tick = T::TickProvider::current_tick();
		VaultsById::<T>::translate::<v1::Vault<T>, _>(|_id, vault| {
			let pending_terms = vault.pending_terms.map(|(bl, terms)| {
				let offset = UniqueSaturatedInto::<Tick>::unique_saturated_into(
					bl.saturating_sub(current_block),
				);
				(current_tick + offset, terms)
			});
			let pending_bonded_argons = vault.pending_bonded_argons.map(|(bl, ar)| {
				let offset = UniqueSaturatedInto::<Tick>::unique_saturated_into(
					bl.saturating_sub(current_block),
				);
				(current_tick + offset, ar)
			});

			count += 1;
			let vault = Vault {
				operator_account_id: vault.operator_account_id,
				bitcoin_argons: VaultArgons {
					annual_percent_rate: vault.bitcoin_argons.annual_percent_rate,
					allocated: vault.bitcoin_argons.allocated,
					reserved: vault.bitcoin_argons.bonded,
					base_fee: vault.bitcoin_argons.base_fee,
				},
				added_securitization_percent: vault.added_securitization_percent,
				added_securitization_argons: vault.securitized_argons,
				bonded_argons: VaultArgons {
					annual_percent_rate: vault.bonded_argons.annual_percent_rate,
					allocated: vault.bonded_argons.allocated,
					reserved: vault.bonded_argons.bonded,
					base_fee: vault.bonded_argons.base_fee,
				},
				mining_reward_sharing_percent_take: vault.mining_reward_sharing_percent_take,
				is_closed: vault.is_closed,
				pending_terms,
				pending_bonded_argons,
				pending_bitcoins: vault.pending_bitcoins,
			};
			Some(vault)
		});

		let terms = v1_p2::PendingTermsModificationsByBlock::<T>::drain().collect::<Vec<_>>();
		for (bl, list) in terms {
			let offset = UniqueSaturatedInto::<Tick>::unique_saturated_into(
				bl.saturating_sub(current_block),
			);
			count += 1;
			PendingTermsModificationsByTick::<T>::insert(
				current_tick + offset,
				BoundedVec::truncate_from(list.to_vec()),
			);
		}

		let terms = v1_p2::PendingFundingModificationsByBlock::<T>::drain().collect::<Vec<_>>();
		for (bl, list) in terms {
			let offset = UniqueSaturatedInto::<Tick>::unique_saturated_into(
				bl.saturating_sub(current_block),
			);
			count += 1;
			PendingFundingModificationsByTick::<T>::insert(
				current_tick + offset,
				BoundedVec::truncate_from(list.to_vec()),
			);
		}

		T::DbWeight::get().reads_writes(count, count)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use argon_primitives::VaultId;
		use codec::Decode;
		use frame_support::ensure;

		let old_value = <Vec<(VaultId, v1::Vault<T>)>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		let actual_new_value = VaultsById::<T>::iter().collect::<Vec<_>>();

		ensure!(old_value.len() == actual_new_value.len(), "New value not set correctly");
		for vault in actual_new_value {
			let old = old_value.iter().find(|(id, _)| id == &vault.0);
			ensure!(old.is_some(), "Vault missing in translation");
			if let Some(old_mining) = old.unwrap().1.pending_bonded_argons {
				if let Some((_tick, amount)) = vault.1.pending_bonded_argons {
					ensure!(amount == old_mining.1, "amounts must match");
				}
			}
		}
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
	use crate::mock::{new_test_ext, CurrentTick, System, Test};
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
			assert_eq!(crate::VaultsById::<Test>::iter_keys().collect::<Vec<_>>(), vec![1]);
			let new_value = crate::VaultsById::<Test>::get(1).unwrap();
			assert_eq!(
				new_value,
				Vault {
					operator_account_id: Default::default(),
					bitcoin_argons,
					added_securitization_percent,
					added_securitization_argons: 0,
					bonded_argons,
					is_closed,
					pending_bonded_argons: Some((1010, 10)),
					pending_bitcoins: 0,
					mining_reward_sharing_percent_take,
					pending_terms: None,
				}
			);

			let pending = crate::PendingFundingModificationsByTick::<Test>::get(1010);
			assert_eq!(pending.to_vec(), vec![1]);
		})
	}
}
