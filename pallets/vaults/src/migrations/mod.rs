use crate::{Config, Pallet};
use argon_primitives::{
	bitcoin::{BitcoinHeight, Satoshis},
	vault::{Vault, VaultName, VaultTerms},
};
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;
use sp_runtime::BoundedBTreeMap;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, Debug)]
struct VaultTermsV14<Balance> {
	#[codec(compact)]
	pub bitcoin_annual_percent_rate: FixedU128,
	#[codec(compact)]
	pub bitcoin_base_fee: Balance,
	#[codec(compact)]
	pub treasury_profit_sharing: Permill,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, Debug)]
struct VaultV14<AccountId, Balance> {
	pub operator_account_id: AccountId,
	pub bitcoin_lock_delegate_account: Option<AccountId>,
	pub name: Option<VaultName>,
	pub last_name_change_tick: Option<Tick>,
	#[codec(compact)]
	pub securitization: Balance,
	#[codec(compact)]
	pub securitization_target: Balance,
	#[codec(compact)]
	pub securitization_locked: Balance,
	#[codec(compact)]
	pub securitization_pending_activation: Balance,
	#[codec(compact)]
	pub locked_satoshis: Satoshis,
	#[codec(compact)]
	pub securitized_satoshis: Satoshis,
	pub securitization_release_schedule: BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
	#[codec(compact)]
	pub securitization_ratio: FixedU128,
	pub is_closed: bool,
	pub terms: VaultTermsV14<Balance>,
	pub pending_terms: Option<(Tick, VaultTermsV14<Balance>)>,
	#[codec(compact)]
	pub opened_tick: Tick,
	pub operational_minimum_release_tick: Option<Tick>,
}

mod v14 {
	use super::*;

	#[storage_alias]
	pub(super) type VaultsById<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		VaultV14<<T as frame_system::Config>::AccountId, <T as crate::Config>::Balance>,
		OptionQuery,
	>;
}

fn capped_bonus_profit_sharing(sharing: Permill) -> Permill {
	let remaining_parts = Permill::one().deconstruct().saturating_sub(sharing.deconstruct());
	Permill::from_parts(remaining_parts.min(sharing.deconstruct()))
}

pub struct MigrateVaultBonusSharing<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateVaultBonusSharing<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let count = v14::VaultsById::<T>::iter().count() as u64;
		Ok(count.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let migrated = v14::VaultsById::<T>::iter_keys().count() as u64;

		crate::VaultsById::<T>::translate::<VaultV14<T::AccountId, T::Balance>, _>(
			|vault_id, old_vault| {
				let migrate_terms = |terms: VaultTermsV14<T::Balance>| {
					let terms = VaultTerms {
						bitcoin_annual_percent_rate: terms.bitcoin_annual_percent_rate,
						bitcoin_base_fee: terms.bitcoin_base_fee,
						treasury_profit_sharing: terms.treasury_profit_sharing,
						treasury_bonus_profit_sharing: capped_bonus_profit_sharing(
							terms.treasury_profit_sharing,
						),
					};
					debug_assert!(
						Pallet::<T>::is_valid_bond_sharing_terms(&terms),
						"Vault {vault_id} has treasury sharing terms above 100%",
					);
					terms
				};

				Some(Vault {
					operator_account_id: old_vault.operator_account_id,
					delegate_account_id: old_vault.bitcoin_lock_delegate_account,
					name: old_vault.name,
					last_name_change_tick: old_vault.last_name_change_tick,
					securitization: old_vault.securitization,
					securitization_target: old_vault.securitization_target,
					securitization_locked: old_vault.securitization_locked,
					securitization_pending_activation: old_vault.securitization_pending_activation,
					locked_satoshis: old_vault.locked_satoshis,
					securitized_satoshis: old_vault.securitized_satoshis,
					securitization_release_schedule: old_vault.securitization_release_schedule,
					securitization_ratio: old_vault.securitization_ratio,
					is_closed: old_vault.is_closed,
					terms: migrate_terms(old_vault.terms),
					pending_terms: old_vault
						.pending_terms
						.map(|(tick, terms)| (tick, migrate_terms(terms))),
					opened_tick: old_vault.opened_tick,
					operational_minimum_release_tick: old_vault.operational_minimum_release_tick,
				})
			},
		);

		T::DbWeight::get().reads_writes(migrated, migrated)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;

		let expected_count = u64::decode(&mut &state[..])
			.map_err(|_| sp_runtime::TryRuntimeError::Other("Failed to decode vault count"))?;

		ensure!(
			frame_support::traits::StorageVersion::get::<Pallet<T>>() == StorageVersion::new(15),
			"Vault storage version mismatch after migration",
		);

		let mut actual_count = 0u64;
		for (_, vault) in crate::VaultsById::<T>::iter() {
			actual_count = actual_count.saturating_add(1);
			ensure!(
				vault.terms.treasury_bonus_profit_sharing ==
					capped_bonus_profit_sharing(vault.terms.treasury_profit_sharing),
				"Vault treasury bonus profit sharing did not cap against treasury sharing",
			);
			ensure!(
				Pallet::<T>::is_valid_bond_sharing_terms(&vault.terms),
				"Vault terms remain above 100% after migration",
			);
			if let Some((_, pending_terms)) = vault.pending_terms {
				ensure!(
					pending_terms.treasury_bonus_profit_sharing ==
						capped_bonus_profit_sharing(pending_terms.treasury_profit_sharing),
					"Pending vault treasury bonus profit sharing did not cap against treasury sharing",
				);
				ensure!(
					Pallet::<T>::is_valid_bond_sharing_terms(&pending_terms),
					"Pending vault terms remain above 100% after migration",
				);
			}
		}

		ensure!(actual_count == expected_count, "Vault count changed during migration");
		Ok(())
	}
}

pub type VaultBonusSharingMigration<T> = frame_support::migrations::VersionedMigration<
	14,
	15,
	MigrateVaultBonusSharing<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::{assert_ok, traits::OnRuntimeUpgrade};

	#[test]
	fn migrates_vault_terms_and_pending_terms() {
		new_test_ext().execute_with(|| {
			frame_support::traits::StorageVersion::new(14).put::<Pallet<Test>>();
			v14::VaultsById::<Test>::insert(
				1,
				VaultV14 {
					operator_account_id: 1,
					bitcoin_lock_delegate_account: Some(9),
					name: None,
					last_name_change_tick: None,
					securitization: 100_000,
					securitization_target: 100_000,
					securitization_locked: 0,
					securitization_pending_activation: 0,
					locked_satoshis: 0,
					securitized_satoshis: 0,
					securitization_release_schedule: Default::default(),
					securitization_ratio: FixedU128::one(),
					is_closed: false,
					terms: VaultTermsV14 {
						bitcoin_annual_percent_rate: FixedU128::one(),
						bitcoin_base_fee: 0,
						treasury_profit_sharing: Permill::from_percent(40),
					},
					pending_terms: Some((
						5,
						VaultTermsV14 {
							bitcoin_annual_percent_rate: FixedU128::from_rational(11u128, 10u128),
							bitcoin_base_fee: 5,
							treasury_profit_sharing: Permill::from_percent(25),
						},
					)),
					opened_tick: 1,
					operational_minimum_release_tick: None,
				},
			);
			let bytes = VaultBonusSharingMigration::<Test>::pre_upgrade().unwrap();
			let _ = VaultBonusSharingMigration::<Test>::on_runtime_upgrade();
			assert_ok!(VaultBonusSharingMigration::<Test>::post_upgrade(bytes));

			let vault = crate::VaultsById::<Test>::get(1).expect("vault should exist");
			assert_eq!(vault.delegate_account_id, Some(9));
			assert_eq!(vault.terms.treasury_profit_sharing, Permill::from_percent(40));
			assert_eq!(vault.terms.treasury_bonus_profit_sharing, Permill::from_percent(40));
			let (_, pending_terms) = vault.pending_terms.expect("pending terms should exist");
			assert_eq!(pending_terms.treasury_profit_sharing, Permill::from_percent(25));
			assert_eq!(pending_terms.treasury_bonus_profit_sharing, Permill::from_percent(25));
		});
	}

	#[test]
	fn migration_caps_vault_terms_above_one_hundred_percent() {
		new_test_ext().execute_with(|| {
			frame_support::traits::StorageVersion::new(14).put::<Pallet<Test>>();
			v14::VaultsById::<Test>::insert(
				1,
				VaultV14 {
					operator_account_id: 1,
					bitcoin_lock_delegate_account: None,
					name: None,
					last_name_change_tick: None,
					securitization: 100_000,
					securitization_target: 100_000,
					securitization_locked: 0,
					securitization_pending_activation: 0,
					locked_satoshis: 0,
					securitized_satoshis: 0,
					securitization_release_schedule: Default::default(),
					securitization_ratio: FixedU128::one(),
					is_closed: false,
					terms: VaultTermsV14 {
						bitcoin_annual_percent_rate: FixedU128::one(),
						bitcoin_base_fee: 0,
						treasury_profit_sharing: Permill::from_percent(60),
					},
					pending_terms: None,
					opened_tick: 1,
					operational_minimum_release_tick: None,
				},
			);

			let bytes = VaultBonusSharingMigration::<Test>::pre_upgrade().unwrap();
			let _ = VaultBonusSharingMigration::<Test>::on_runtime_upgrade();
			assert_ok!(VaultBonusSharingMigration::<Test>::post_upgrade(bytes));

			let vault = crate::VaultsById::<Test>::get(1).expect("vault should exist");
			assert_eq!(vault.terms.treasury_profit_sharing, Permill::from_percent(60));
			assert_eq!(vault.terms.treasury_bonus_profit_sharing, Permill::from_percent(40));
		});
	}
}
