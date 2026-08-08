use crate::{Config, OperationalAccounts, Pallet};
use argon_primitives::{BitcoinLocksProvider, BitcoinLocksProviderWeightInfo};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

pub struct ReconcileAccountBitcoinAmounts<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for ReconcileAccountBitcoinAmounts<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let expected_amounts = OperationalAccounts::<T>::iter()
			.map(|(owner, account)| {
				let amount = T::BitcoinLocksProvider::get_account_funded_bitcoin_amount(
					&account.vault_account,
				);
				(owner, amount)
			})
			.collect::<Vec<_>>();

		Ok(expected_amounts.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut weight = Weight::zero();
		let mut account_reads = 0u64;
		let mut account_writes = 0u64;

		for (owner, mut account) in OperationalAccounts::<T>::iter() {
			account_reads.saturating_accrue(1);
			let funded_amount =
				T::BitcoinLocksProvider::get_account_funded_bitcoin_amount(&account.vault_account);
			weight.saturating_accrue(<T::BitcoinLocksProvider as BitcoinLocksProvider<
				T::AccountId,
				T::Balance,
			>>::Weights::get_account_funded_bitcoin_amount());

			if account.account_bitcoin_amount == funded_amount {
				continue;
			}

			account.account_bitcoin_amount = funded_amount;
			OperationalAccounts::<T>::insert(owner, account);
			account_writes.saturating_accrue(1);
		}

		weight.saturating_add(T::DbWeight::get().reads_writes(account_reads, account_writes))
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let expected_amounts = <Vec<(T::AccountId, T::Balance)>>::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode bitcoin reconciliation state"))?;
		for (owner, expected_amount) in expected_amounts {
			let account = OperationalAccounts::<T>::get(owner).ok_or(TryRuntimeError::Other(
				"operational account was removed during bitcoin reconciliation",
			))?;
			ensure!(
				account.account_bitcoin_amount == expected_amount,
				TryRuntimeError::Other("operational account bitcoin amount was not reconciled"),
			);
		}

		Ok(())
	}
}

pub type ReconcileAccountBitcoinAmountsMigration<T> = frame_support::migrations::VersionedMigration<
	3,
	4,
	ReconcileAccountBitcoinAmounts<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, OpaqueEncryptionPubkey, OperationalAccount};
	use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};

	#[test]
	fn reconciles_each_account_to_its_linked_vault_funded_total() {
		new_test_ext().execute_with(|| {
			let owner = TestAccountId::new([1; 32]);
			let vault_account = TestAccountId::new([2; 32]);
			let funded_total = 1_999_999_932;
			record_funded_bitcoin_amount(&vault_account, funded_total);
			crate::OperationalAccounts::<Test>::insert(
				&owner,
				OperationalAccount::<Test> {
					vault_account,
					mining_account: TestAccountId::new([3; 32]),
					encryption_pubkey: OpaqueEncryptionPubkey([0; 32]),
					upstream_account: None,
					name: None,
					last_name_change_tick: None,
					uniswap_argon_transfers_in_amount: 0,
					account_bitcoin_amount: 0,
					account_vault_bond_amount: 0,
					vault_created: false,
					vault_bitcoin_accrual: 0,
					vault_bitcoin_applied_total: 0,
					mining_seat_accrual: 0,
					mining_seat_applied_total: 0,
					operational_certifications_count: 0,
					available_access_codes: 0,
					rewards_earned_count: 0,
					rewards_earned_amount: 0,
					rewards_collected_amount: 0,
					is_operationally_certified: false,
				},
			);
			StorageVersion::new(3).put::<Pallet<Test>>();

			#[cfg(feature = "try-runtime")]
			let state = ReconcileAccountBitcoinAmountsMigration::<Test>::pre_upgrade()
				.expect("pre-upgrade checks");
			ReconcileAccountBitcoinAmountsMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			ReconcileAccountBitcoinAmountsMigration::<Test>::post_upgrade(state)
				.expect("post-upgrade checks");

			assert_eq!(
				crate::OperationalAccounts::<Test>::get(&owner)
					.expect("operational account")
					.account_bitcoin_amount,
				funded_total
			);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 4);
		});
	}
}
