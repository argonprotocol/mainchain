use argon_primitives::{bitcoin::Satoshis, VaultId};
use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

use crate::{Config, Pallet, RevenuePerFrameByVault, VaultFrameRevenue};

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::{ensure, traits::StorageVersion};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Encode, Decode)]
struct VaultFrameRevenueV17<T: Config> {
	#[codec(compact)]
	frame_id: FrameId,
	#[codec(compact)]
	bitcoin_lock_fee_revenue: T::Balance,
	#[codec(compact)]
	bitcoin_lock_fee_coupon_value_used: T::Balance,
	#[codec(compact)]
	bitcoin_locks_created: u32,
	#[codec(compact)]
	bitcoin_locks_new_securitization: T::Balance,
	#[codec(compact)]
	bitcoin_locks_released_securitization: T::Balance,
	#[codec(compact)]
	bitcoin_locks_added_satoshis: Satoshis,
	#[codec(compact)]
	bitcoin_locks_released_satoshis: Satoshis,
	#[codec(compact)]
	securitization_activated: T::Balance,
	#[codec(compact)]
	securitization_relockable: T::Balance,
	#[codec(compact)]
	securitization: T::Balance,
	#[codec(compact)]
	treasury_vault_earnings: T::Balance,
	#[codec(compact)]
	treasury_total_earnings: T::Balance,
	#[codec(compact)]
	treasury_vault_capital: T::Balance,
	#[codec(compact)]
	treasury_external_capital: T::Balance,
	#[codec(compact)]
	uncollected_revenue: T::Balance,
}

mod v17 {
	use super::*;

	#[storage_alias]
	pub(super) type RevenuePerFrameByVault<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		BoundedVec<VaultFrameRevenueV17<T>, ConstU32<12>>,
		ValueQuery,
	>;
}

pub struct AddVaultBondEarningsHistory<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddVaultBondEarningsHistory<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		ensure!(
			StorageVersion::get::<Pallet<T>>() == 17,
			TryRuntimeError::Other("vault storage version must be 17 before earnings migration"),
		);
		let vault_count = v17::RevenuePerFrameByVault::<T>::iter_keys().count() as u64;
		let revenue_count = v17::RevenuePerFrameByVault::<T>::iter_values()
			.fold(0u64, |count, entries| count.saturating_add(entries.len() as u64));
		Ok((vault_count, revenue_count).encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut vault_count = 0u64;
		RevenuePerFrameByVault::<T>::translate::<
			BoundedVec<VaultFrameRevenueV17<T>, ConstU32<12>>,
			_,
		>(|_, entries| {
			vault_count.saturating_accrue(1);
			let mut migrated_entries = BoundedVec::default();
			for entry in entries {
				migrated_entries
					.try_push(VaultFrameRevenue {
						frame_id: entry.frame_id,
						bitcoin_lock_fee_revenue: entry.bitcoin_lock_fee_revenue,
						bitcoin_lock_fee_coupon_value_used: entry
							.bitcoin_lock_fee_coupon_value_used,
						bitcoin_locks_created: entry.bitcoin_locks_created,
						bitcoin_locks_new_securitization: entry.bitcoin_locks_new_securitization,
						bitcoin_locks_released_securitization: entry
							.bitcoin_locks_released_securitization,
						bitcoin_locks_added_satoshis: entry.bitcoin_locks_added_satoshis,
						bitcoin_locks_released_satoshis: entry.bitcoin_locks_released_satoshis,
						securitization_activated: entry.securitization_activated,
						securitization_relockable: entry.securitization_relockable,
						securitization: entry.securitization,
						treasury_vault_earnings: entry.treasury_vault_earnings,
						treasury_total_earnings: entry.treasury_total_earnings,
						argonot_securitization: T::Balance::zero(),
						argonots_for_max_earnings: T::Balance::zero(),
						treasury_unrealized_earnings: T::Balance::zero(),
						treasury_vault_capital: entry.treasury_vault_capital,
						treasury_external_capital: entry.treasury_external_capital,
						uncollected_revenue: entry.uncollected_revenue,
					})
					.expect("source and destination revenue bounds match");
			}
			Some(migrated_entries)
		});

		T::DbWeight::get().reads_writes(vault_count, vault_count)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let (expected_vault_count, expected_revenue_count) =
			<(u64, u64)>::decode(&mut state.as_slice())
				.map_err(|_| TryRuntimeError::Other("could not decode earnings migration state"))?;
		let vault_count = RevenuePerFrameByVault::<T>::iter_keys().count() as u64;
		let mut revenue_count = 0u64;
		let mut new_fields_are_zero = true;
		for entries in RevenuePerFrameByVault::<T>::iter_values() {
			revenue_count.saturating_accrue(entries.len() as u64);
			new_fields_are_zero &= entries.iter().all(|entry| {
				entry.argonot_securitization.is_zero() &&
					entry.argonots_for_max_earnings.is_zero() &&
					entry.treasury_unrealized_earnings.is_zero()
			});
		}

		ensure!(
			vault_count == expected_vault_count && revenue_count == expected_revenue_count,
			TryRuntimeError::Other("vault revenue entries changed during earnings migration"),
		);
		ensure!(
			new_fields_are_zero,
			TryRuntimeError::Other("legacy vault revenue received nonzero earnings metadata"),
		);
		Ok(())
	}
}

pub type AddVaultBondEarningsHistoryMigration<T> = frame_support::migrations::VersionedMigration<
	17,
	18,
	AddVaultBondEarningsHistory<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::traits::OnRuntimeUpgrade;

	#[test]
	fn adds_empty_bond_earnings_metadata_to_legacy_vault_revenue() {
		new_test_ext().execute_with(|| {
			v17::RevenuePerFrameByVault::<Test>::insert(
				1,
				BoundedVec::truncate_from(vec![VaultFrameRevenueV17::<Test> {
					frame_id: 2,
					bitcoin_lock_fee_revenue: 3,
					bitcoin_lock_fee_coupon_value_used: 4,
					bitcoin_locks_created: 5,
					bitcoin_locks_new_securitization: 6,
					bitcoin_locks_released_securitization: 7,
					bitcoin_locks_added_satoshis: 8,
					bitcoin_locks_released_satoshis: 9,
					securitization_activated: 10,
					securitization_relockable: 11,
					securitization: 12,
					treasury_vault_earnings: 13,
					treasury_total_earnings: 14,
					treasury_vault_capital: 15,
					treasury_external_capital: 16,
					uncollected_revenue: 17,
				}]),
			);
			StorageVersion::new(17).put::<Pallet<Test>>();

			AddVaultBondEarningsHistoryMigration::<Test>::try_on_runtime_upgrade(true)
				.expect("runtime upgrade checks");

			let revenue = RevenuePerFrameByVault::<Test>::get(1);
			assert_eq!(revenue.len(), 1);
			assert_eq!(revenue[0].frame_id, 2);
			assert_eq!(revenue[0].bitcoin_lock_fee_revenue, 3);
			assert_eq!(revenue[0].treasury_total_earnings, 14);
			assert_eq!(revenue[0].treasury_vault_capital, 15);
			assert_eq!(revenue[0].uncollected_revenue, 17);
			assert_eq!(revenue[0].argonot_securitization, 0);
			assert_eq!(revenue[0].argonots_for_max_earnings, 0);
			assert_eq!(revenue[0].treasury_unrealized_earnings, 0);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 18);
		});
	}
}
