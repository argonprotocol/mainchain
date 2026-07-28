use argon_primitives::{
	bitcoin::{BitcoinHeight, Satoshis},
	vault::{Vault, VaultName, VaultTerms},
	VaultId,
};
use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

use crate::{Config, Pallet, VaultsById};

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::{ensure, traits::StorageVersion};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Encode, Decode)]
struct VaultV15<AccountId, Balance>
where
	AccountId: Codec,
	Balance: Codec + Copy + MaxEncodedLen + Default + AtLeast32BitUnsigned + TypeInfo,
{
	operator_account_id: AccountId,
	delegate_account_id: Option<AccountId>,
	name: Option<VaultName>,
	last_name_change_tick: Option<Tick>,
	#[codec(compact)]
	securitization: Balance,
	#[codec(compact)]
	securitization_target: Balance,
	#[codec(compact)]
	securitization_locked: Balance,
	#[codec(compact)]
	securitization_pending_activation: Balance,
	#[codec(compact)]
	locked_satoshis: Satoshis,
	#[codec(compact)]
	securitized_satoshis: Satoshis,
	securitization_release_schedule: BoundedBTreeMap<BitcoinHeight, Balance, ConstU32<366>>,
	#[codec(compact)]
	securitization_ratio: FixedU128,
	is_closed: bool,
	terms: VaultTerms<Balance>,
	pending_terms: Option<(Tick, VaultTerms<Balance>)>,
	#[codec(compact)]
	opened_tick: Tick,
	operational_minimum_release_tick: Option<Tick>,
}

mod v15 {
	use super::*;

	#[storage_alias]
	pub(super) type VaultsById<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		VaultV15<<T as frame_system::Config>::AccountId, <T as crate::Config>::Balance>,
		OptionQuery,
	>;
}

pub struct AddBackfillFields<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddBackfillFields<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut vault_count = 0u64;
		VaultsById::<T>::translate::<
			VaultV15<<T as frame_system::Config>::AccountId, <T as crate::Config>::Balance>,
			_,
		>(|_, vault| {
			vault_count = vault_count.saturating_add(1);
			Some(Vault {
				operator_account_id: vault.operator_account_id,
				delegate_account_id: vault.delegate_account_id,
				name: vault.name,
				last_name_change_tick: vault.last_name_change_tick,
				securitization: vault.securitization,
				securitization_target: vault.securitization_target,
				securitization_locked: vault.securitization_locked,
				backfill_securitization_locked: T::Balance::zero(),
				backfill_securitization_reserved: T::Balance::zero(),
				securitization_pending_activation: vault.securitization_pending_activation,
				locked_satoshis: vault.locked_satoshis,
				securitized_satoshis: vault.securitized_satoshis,
				backfill_securitized_satoshis: 0,
				securitization_release_schedule: vault.securitization_release_schedule,
				securitization_ratio: vault.securitization_ratio,
				is_closed: vault.is_closed,
				terms: vault.terms,
				pending_terms: vault.pending_terms,
				opened_tick: vault.opened_tick,
				operational_minimum_release_tick: vault.operational_minimum_release_tick,
			})
		});

		T::DbWeight::get().reads_writes(vault_count, vault_count)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		ensure!(
			StorageVersion::get::<Pallet<T>>() == 15,
			TryRuntimeError::Other("vault storage version must be 15 before migration"),
		);

		let vault_count =
			v15::VaultsById::<T>::iter().fold(0u64, |count, _| count.saturating_add(1));
		Ok(vault_count.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		ensure!(
			StorageVersion::get::<Pallet<T>>() == 16,
			TryRuntimeError::Other("vault storage version was not updated"),
		);

		let expected_vault_count = u64::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode vault count"))?;
		let mut migrated_vault_count = 0u64;
		for (_, vault) in VaultsById::<T>::iter() {
			migrated_vault_count = migrated_vault_count.saturating_add(1);
			ensure!(
				vault.backfill_securitization_locked.is_zero() &&
					vault.backfill_securitization_reserved.is_zero() &&
					vault.backfill_securitized_satoshis == 0,
				TryRuntimeError::Other("migrated vault has nonzero backfill state"),
			);
		}
		ensure!(
			migrated_vault_count == expected_vault_count,
			TryRuntimeError::Other("vault count changed during migration"),
		);

		Ok(())
	}
}

pub type AddBackfillFieldsMigration<T> = frame_support::migrations::VersionedMigration<
	15,
	16,
	AddBackfillFields<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::traits::OnRuntimeUpgrade;

	#[test]
	fn adds_empty_backfill_state() {
		new_test_ext().execute_with(|| {
			v15::VaultsById::<Test>::insert(
				1,
				VaultV15 {
					operator_account_id: 1,
					delegate_account_id: Some(2),
					name: None,
					last_name_change_tick: None,
					securitization: 100,
					securitization_target: 100,
					securitization_locked: 40,
					securitization_pending_activation: 10,
					locked_satoshis: 20,
					securitized_satoshis: 20,
					securitization_release_schedule: BoundedBTreeMap::default(),
					securitization_ratio: FixedU128::one(),
					is_closed: false,
					terms: VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::zero(),
						bitcoin_base_fee: 0,
						treasury_profit_sharing: Permill::zero(),
						treasury_bonus_profit_sharing: Permill::zero(),
					},
					pending_terms: None,
					opened_tick: 1,
					operational_minimum_release_tick: None,
				},
			);
			StorageVersion::new(15).put::<Pallet<Test>>();

			let state = AddBackfillFieldsMigration::<Test>::pre_upgrade().unwrap();
			AddBackfillFieldsMigration::<Test>::on_runtime_upgrade();
			AddBackfillFieldsMigration::<Test>::post_upgrade(state).unwrap();

			let vault = VaultsById::<Test>::get(1).expect("migrated vault");
			assert_eq!(vault.backfill_securitization_locked, 0);
			assert_eq!(vault.backfill_securitization_reserved, 0);
			assert_eq!(vault.backfill_securitized_satoshis, 0);
			assert_eq!(vault.available_for_lock(false), 60);
			assert_eq!(vault.available_for_lock(true), 60);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 16);
		});
	}
}
