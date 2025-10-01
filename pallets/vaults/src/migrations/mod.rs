use crate::{Config, VaultIdByOperator, pallet::VaultsById};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use crate::Config;
	use alloc::collections::BTreeMap;
	use pallet_prelude::*;

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub vault_owners: BTreeMap<T::AccountId, VaultId>,
	}
}
pub struct InnerMigrate<T: crate::Config + pallet_liquidity_pools::Config>(
	core::marker::PhantomData<T>,
);

impl<T: Config + pallet_liquidity_pools::Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use alloc::collections::BTreeMap;
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let vault_owners = VaultsById::<T>::iter()
			.map(|a| (a.1.operator_account_id, a.0))
			.collect::<BTreeMap<_, _>>();

		Ok(old_storage::Model::<T> { vault_owners }.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating vaults");

		for (vault_id, vault) in VaultsById::<T>::iter() {
			count += 1;
			VaultIdByOperator::<T>::insert(vault.operator_account_id, vault_id);
		}

		T::DbWeight::get().reads_writes((count) as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use alloc::collections::BTreeMap;
		use sp_core::Decode;

		let old = <old_storage::Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		let new_vault_owners = VaultIdByOperator::<T>::iter().collect::<BTreeMap<_, _>>();
		ensure!(old.vault_owners == new_vault_owners, "Vault owner data mismatch");

		Ok(())
	}
}

pub type SingleVaultOwenr<T> = frame_support::migrations::VersionedMigration<
	7,
	8,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{Test, new_test_ext};
	use frame_support::assert_ok;
	use pallet_prelude::argon_primitives::vault::{Vault, VaultTerms};

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			VaultsById::<Test>::insert(
				1,
				Vault {
					operator_account_id: 1,
					securitization: 100,
					argons_scheduled_for_release: Default::default(),
					argons_locked: 100,
					argons_pending_activation: 0,
					securitization_ratio: FixedU128::one(),
					is_closed: false,
					terms: VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
						bitcoin_base_fee: 0,
						liquidity_pool_profit_sharing: Permill::from_percent(10),
					},
					pending_terms: None,
					opened_tick: 0,
				},
			);
			VaultsById::<Test>::insert(
				2,
				Vault {
					operator_account_id: 2,
					securitization: 200,
					argons_scheduled_for_release: Default::default(),
					argons_locked: 100,
					argons_pending_activation: 0,
					securitization_ratio: FixedU128::one(),
					is_closed: false,
					terms: VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::from_float(0.1),
						bitcoin_base_fee: 0,
						liquidity_pool_profit_sharing: Permill::from_percent(10),
					},
					pending_terms: None,
					opened_tick: 0,
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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(2, 2));

			assert_eq!(VaultIdByOperator::<Test>::get(1), Some(1));
			assert_eq!(VaultIdByOperator::<Test>::get(2), Some(2));
		});
	}
}
