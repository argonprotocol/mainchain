use crate::{Config, FunderStateByVaultAndAccount, FundersByVaultId};
use alloc::collections::BTreeMap;
use frame_support::{traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

pub struct BackfillFundersByVaultId<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for BackfillFundersByVaultId<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let count = FunderStateByVaultAndAccount::<T>::iter()
			.filter(|(_, _, state)| !state.held_principal.is_zero())
			.count() as u64;
		Ok(count.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut reads = 0u64;
		let mut writes = 0u64;
		let mut rebuilt =
			BTreeMap::<VaultId, BoundedBTreeSet<T::AccountId, T::MaxTreasuryContributors>>::new();

		log::info!("Rebuilding FundersByVaultId index from FunderStateByVaultAndAccount");

		for (vault_id, account_id, state) in FunderStateByVaultAndAccount::<T>::iter() {
			reads += 1;
			if state.held_principal.is_zero() {
				continue;
			}

			let funders = rebuilt.entry(vault_id).or_default();
			if funders.try_insert(account_id.clone()).is_err() {
				log::error!(
					"FundersByVaultId overflow while rebuilding vault {vault_id:?}; skipping extra account"
				);
			}
		}

		for (vault_id, funders) in rebuilt {
			FundersByVaultId::<T>::set(vault_id, funders);
			writes += 1;
		}

		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;
		use sp_core::Decode;

		let expected_count = u64::decode(&mut &state[..])
			.map_err(|_| sp_runtime::TryRuntimeError::Other("Failed to decode active count"))?;

		let index_count = FundersByVaultId::<T>::iter()
			.fold(0u64, |count, (_, funders)| count.saturating_add(funders.len() as u64));
		ensure!(expected_count == index_count, "FundersByVaultId count mismatch");

		for (vault_id, account_id, state) in FunderStateByVaultAndAccount::<T>::iter() {
			let is_indexed = FundersByVaultId::<T>::get(vault_id).contains(&account_id);
			if !state.held_principal.is_zero() {
				ensure!(is_indexed, "Missing funder index entry");
			} else {
				ensure!(!is_indexed, "Inactive funder unexpectedly indexed");
			}
		}

		Ok(())
	}
}

pub type BackfillFundersByVaultIdMigration<T> = frame_support::migrations::VersionedMigration<
	2,
	3,
	BackfillFundersByVaultId<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod test {
	use super::*;
	use crate::{
		FunderState,
		mock::{Test, new_test_ext},
	};

	#[test]
	fn rebuilds_funders_by_vault_id_from_funder_state() {
		new_test_ext().execute_with(|| {
			let active = FunderState::<Test> {
				target_principal: 100,
				held_principal: 100,
				..Default::default()
			};
			let bonded = FunderState::<Test> { bonded_principal: 50, ..Default::default() };
			let inactive = FunderState::<Test>::default();

			FunderStateByVaultAndAccount::<Test>::insert(1, 10, active);
			FunderStateByVaultAndAccount::<Test>::insert(1, 11, inactive);
			FunderStateByVaultAndAccount::<Test>::insert(2, 20, bonded);

			BackfillFundersByVaultId::<Test>::on_runtime_upgrade();

			assert_eq!(FundersByVaultId::<Test>::get(1).into_iter().collect::<Vec<_>>(), vec![10]);
			assert!(!FundersByVaultId::<Test>::contains_key(2));
			assert!(FundersByVaultId::<Test>::get(1).get(&11).is_none());
		});
	}
}
