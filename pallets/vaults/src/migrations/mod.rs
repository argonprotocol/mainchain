use crate::{Config, Pallet, VaultsById};
use alloc::collections::BTreeMap;
use argon_primitives::{bitcoin::Satoshis, VaultId};
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

/// Replace the former funded-satoshi total with the amount actually covered by each migrated
/// Bitcoin Lock's securitization.
pub struct ReconcileSecuritizedSatoshis<T>(core::marker::PhantomData<T>);

impl<T> UncheckedOnRuntimeUpgrade for ReconcileSecuritizedSatoshis<T>
where
	T: Config + pallet_bitcoin_locks::Config<Balance = <T as Config>::Balance>,
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(VaultsById::<T>::iter_keys()
			.fold(0u64, |count, _| count.saturating_add(1))
			.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut reads = 0u64;
		let mut writes = 0u64;
		let mut securitized_satoshis_by_vault = BTreeMap::<VaultId, Satoshis>::new();

		for (_, lock) in pallet_bitcoin_locks::LocksById::<T>::iter() {
			reads.saturating_accrue(1);
			securitized_satoshis_by_vault
				.entry(lock.vault_id)
				.or_default()
				.saturating_accrue(lock.funded_satoshis.min(lock.securitization_basis.satoshis));
		}

		for (vault_id, mut vault) in VaultsById::<T>::iter() {
			reads.saturating_accrue(1);
			writes.saturating_accrue(1);
			vault.securitized_satoshis =
				securitized_satoshis_by_vault.remove(&vault_id).unwrap_or_default();
			VaultsById::<T>::insert(vault_id, vault);
		}

		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let expected_vault_count = u64::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode vault migration state"))?;
		let vault_count =
			VaultsById::<T>::iter_keys().fold(0u64, |count, _| count.saturating_add(1));
		ensure!(
			vault_count == expected_vault_count,
			TryRuntimeError::Other("vault count changed during securitized satoshi reconciliation"),
		);

		let mut securitized_satoshis_by_vault = BTreeMap::<VaultId, Satoshis>::new();
		for (_, lock) in pallet_bitcoin_locks::LocksById::<T>::iter() {
			securitized_satoshis_by_vault
				.entry(lock.vault_id)
				.or_default()
				.saturating_accrue(lock.funded_satoshis.min(lock.securitization_basis.satoshis));
		}
		for (vault_id, vault) in VaultsById::<T>::iter() {
			ensure!(
				vault.securitized_satoshis ==
					securitized_satoshis_by_vault.remove(&vault_id).unwrap_or_default(),
				TryRuntimeError::Other("vault securitized satoshis were not reconciled"),
			);
		}
		ensure!(
			securitized_satoshis_by_vault.is_empty(),
			TryRuntimeError::Other("migrated bitcoin lock references a missing vault"),
		);
		Ok(())
	}
}

pub type ReconcileSecuritizedSatoshisMigration<T> = frame_support::migrations::VersionedMigration<
	17,
	18,
	ReconcileSecuritizedSatoshis<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, VaultsById};
	use argon_primitives::{
		bitcoin::{BitcoinCosignScriptPubkey, CompressedBitcoinPubkey},
		vault::{BitcoinSecuritizationBasis, Vault, VaultTerms},
	};
	use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
	use pallet_bitcoin_locks::{LockedBitcoin, LocksById};
	use polkadot_sdk::sp_core::H256;

	fn vault(securitized_satoshis: u64) -> Vault<u64, Balance> {
		Vault {
			operator_account_id: 1,
			delegate_account_id: None,
			securitization: 1_000,
			securitization_target: 1_000,
			securitization_locked: 100,
			flexible_securitization_locked: 0,
			reserved_securitization_space: 0,
			securitization_pending_activation: 0,
			securitized_satoshis,
			ratio_adjusted_satoshis: 77,
			flexible_ratio_adjusted_satoshis: 0,
			securitization_release_schedule: BoundedBTreeMap::new(),
			securitization_ratio: FixedU128::one(),
			is_closed: false,
			terms: VaultTerms {
				bitcoin_annual_percent_rate: FixedU128::zero(),
				bitcoin_base_fee: 0,
				treasury_profit_sharing: Permill::zero(),
			},
			pending_terms: None,
			opened_tick: 1,
			operational_minimum_release_tick: None,
		}
	}

	fn lock(vault_id: u32, basis_satoshis: u64, funded_satoshis: u64) -> LockedBitcoin<Test> {
		LockedBitcoin {
			vault_id,
			securitization_basis: BitcoinSecuritizationBasis {
				satoshis: basis_satoshis,
				microgons_at_target_per_btc: 1,
			},
			securitization_coverage_microgons: 1,
			securitization_tick: 1,
			funded_satoshis,
			funding_utxos: BoundedBTreeMap::new(),
			fissioned_satoshis: 0,
			owner_account: 2,
			securitization_ratio: FixedU128::one(),
			security_fees: 0,
			coupon_paid_fees: 0,
			vault_pubkey: CompressedBitcoinPubkey([1; 33]),
			vault_claim_pubkey: CompressedBitcoinPubkey([2; 33]),
			vault_xpub_sources: ([3; 4], 4, 5),
			owner_pubkey: CompressedBitcoinPubkey([6; 33]),
			vault_claim_height: 100,
			open_claim_height: 130,
			created_at_height: 1,
			securitization_hold_expiration_bitcoin_height: 10,
			utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
				wscript_hash: H256::repeat_byte(7),
			},
			is_flexible: false,
			fund_hold_extensions: BoundedBTreeMap::new(),
			created_at_argon_block: 1,
		}
	}

	#[test]
	fn caps_migrated_vault_satoshis_at_each_locks_securitization_basis() {
		new_test_ext().execute_with(|| {
			VaultsById::<Test>::insert(1, vault(99_000));
			LocksById::<Test>::insert(1, lock(1, 10_000, 12_000));
			LocksById::<Test>::insert(2, lock(1, 10_000, 4_000));
			StorageVersion::new(17).put::<Pallet<Test>>();

			#[cfg(not(feature = "try-runtime"))]
			ReconcileSecuritizedSatoshisMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			ReconcileSecuritizedSatoshisMigration::<Test>::try_on_runtime_upgrade(true)
				.expect("runtime upgrade checks");

			let vault = VaultsById::<Test>::get(1).expect("vault");
			assert_eq!(vault.securitized_satoshis, 14_000);
			assert_eq!(vault.ratio_adjusted_satoshis, 77);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), StorageVersion::new(18));
		});
	}
}
