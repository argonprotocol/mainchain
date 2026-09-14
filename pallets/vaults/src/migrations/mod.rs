use crate::{Config, Pallet};
use alloc::collections::BTreeMap;
use argon_primitives::{
	bitcoin::{BitcoinHeight, Satoshis},
	vault::{Vault, VaultTerms, MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES},
	VaultId,
};
use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Decode, Encode)]
struct VaultV17<T: Config> {
	operator_account_id: T::AccountId,
	delegate_account_id: Option<T::AccountId>,
	#[codec(compact)]
	securitization: T::Balance,
	#[codec(compact)]
	securitization_target: T::Balance,
	#[codec(compact)]
	securitization_locked: T::Balance,
	#[codec(compact)]
	flexible_securitization_locked: T::Balance,
	#[codec(compact)]
	reserved_securitization_space: T::Balance,
	#[codec(compact)]
	securitization_pending_activation: T::Balance,
	#[codec(compact)]
	securitized_satoshis: Satoshis,
	#[codec(compact)]
	ratio_adjusted_satoshis: Satoshis,
	#[codec(compact)]
	flexible_ratio_adjusted_satoshis: Satoshis,
	securitization_release_schedule: BoundedBTreeMap<
		BitcoinHeight,
		T::Balance,
		ConstU32<MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES>,
	>,
	#[codec(compact)]
	securitization_ratio: FixedU128,
	is_closed: bool,
	terms: VaultTerms<T::Balance>,
	pending_terms: Option<(Tick, VaultTerms<T::Balance>)>,
	#[codec(compact)]
	opened_tick: Tick,
	operational_minimum_release_tick: Option<Tick>,
}

mod v17 {
	use super::*;

	#[storage_alias]
	pub(super) type VaultsById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, VaultId, VaultV17<T>, OptionQuery>;
}

#[derive(Default)]
struct VaultBitcoinLockAccounting {
	total_satoshis: Satoshis,
	securitized_satoshis: Satoshis,
	ratio_adjusted_satoshis: Satoshis,
	flexible_ratio_adjusted_satoshis: Satoshis,
}

fn collect_bitcoin_lock_accounting<T>() -> (BTreeMap<VaultId, VaultBitcoinLockAccounting>, u64)
where
	T: Config + pallet_bitcoin_locks::Config<Balance = <T as Config>::Balance>,
{
	let mut accounting_by_vault = BTreeMap::<VaultId, VaultBitcoinLockAccounting>::new();
	let mut reads = 0u64;
	for (_, lock) in pallet_bitcoin_locks::LocksById::<T>::iter() {
		reads.saturating_accrue(1);
		let securitization = lock.get_securitization();
		let accounting = accounting_by_vault.entry(lock.vault_id).or_default();
		accounting.total_satoshis.saturating_accrue(lock.funded_satoshis);
		accounting
			.securitized_satoshis
			.saturating_accrue(securitization.securitized_satoshis(lock.funded_satoshis));
		let eligible_satoshis = securitization.eligible_satoshis(lock.funded_satoshis);
		accounting.ratio_adjusted_satoshis.saturating_accrue(eligible_satoshis);
		if lock.is_flexible {
			accounting.flexible_ratio_adjusted_satoshis.saturating_accrue(eligible_satoshis);
		}
	}
	(accounting_by_vault, reads)
}

/// Reconcile Vault Bitcoin accounting from the authoritative funded Locks.
pub struct ReconcileBitcoinLockAccounting<T>(core::marker::PhantomData<T>);

impl<T> UncheckedOnRuntimeUpgrade for ReconcileBitcoinLockAccounting<T>
where
	T: Config + pallet_bitcoin_locks::Config<Balance = <T as Config>::Balance>,
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(v17::VaultsById::<T>::iter_keys()
			.fold(0u64, |count, _| count.saturating_add(1))
			.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let (mut accounting_by_vault, mut reads) = collect_bitcoin_lock_accounting::<T>();
		let mut writes = 0u64;

		crate::VaultsById::<T>::translate::<VaultV17<T>, _>(|vault_id, vault| {
			reads.saturating_accrue(1);
			writes.saturating_accrue(1);
			let accounting = accounting_by_vault.remove(&vault_id).unwrap_or_default();
			Some(Vault {
				operator_account_id: vault.operator_account_id,
				delegate_account_id: vault.delegate_account_id,
				securitization: vault.securitization,
				securitization_target: vault.securitization_target,
				securitization_locked: vault.securitization_locked,
				flexible_securitization_locked: vault.flexible_securitization_locked,
				reserved_securitization_space: vault.reserved_securitization_space,
				securitization_pending_activation: vault.securitization_pending_activation,
				total_satoshis: accounting.total_satoshis,
				securitized_satoshis: accounting.securitized_satoshis,
				ratio_adjusted_satoshis: accounting.ratio_adjusted_satoshis,
				flexible_ratio_adjusted_satoshis: accounting.flexible_ratio_adjusted_satoshis,
				securitization_release_schedule: vault.securitization_release_schedule,
				securitization_ratio: vault.securitization_ratio,
				is_closed: vault.is_closed,
				terms: vault.terms,
				pending_terms: vault.pending_terms,
				opened_tick: vault.opened_tick,
				operational_minimum_release_tick: vault.operational_minimum_release_tick,
			})
		});

		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let expected_vault_count = u64::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode vault migration state"))?;
		let vault_count =
			crate::VaultsById::<T>::iter_keys().fold(0u64, |count, _| count.saturating_add(1));
		ensure!(
			vault_count == expected_vault_count,
			TryRuntimeError::Other("vault count changed during securitized satoshi reconciliation"),
		);

		let (mut accounting_by_vault, _) = collect_bitcoin_lock_accounting::<T>();
		for (vault_id, vault) in crate::VaultsById::<T>::iter() {
			let accounting = accounting_by_vault.remove(&vault_id).unwrap_or_default();
			ensure!(
				vault.total_satoshis == accounting.total_satoshis,
				TryRuntimeError::Other("vault total satoshis were not reconciled"),
			);
			ensure!(
				vault.securitized_satoshis == accounting.securitized_satoshis,
				TryRuntimeError::Other("vault securitized satoshis were not reconciled"),
			);
			ensure!(
				vault.ratio_adjusted_satoshis == accounting.ratio_adjusted_satoshis,
				TryRuntimeError::Other("vault ratio-adjusted satoshis were not reconciled"),
			);
			ensure!(
				vault.flexible_ratio_adjusted_satoshis ==
					accounting.flexible_ratio_adjusted_satoshis,
				TryRuntimeError::Other(
					"vault flexible ratio-adjusted satoshis were not reconciled"
				),
			);
		}
		ensure!(
			accounting_by_vault.is_empty(),
			TryRuntimeError::Other("migrated bitcoin lock references a missing vault"),
		);
		Ok(())
	}
}

pub type ReconcileBitcoinLockAccountingMigration<T> = frame_support::migrations::VersionedMigration<
	17,
	18,
	ReconcileBitcoinLockAccounting<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, VaultsById};
	use argon_primitives::{
		bitcoin::{BitcoinCosignScriptPubkey, CompressedBitcoinPubkey},
		vault::{BitcoinSecuritizationBasis, VaultTerms},
	};
	use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
	use pallet_bitcoin_locks::{LockedBitcoin, LocksById};
	use polkadot_sdk::sp_core::H256;

	fn vault(securitized_satoshis: u64) -> VaultV17<Test> {
		VaultV17 {
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
			v17::VaultsById::<Test>::insert(1, vault(99_000));
			LocksById::<Test>::insert(1, lock(1, 10_000, 12_000));
			let mut flexible_lock = lock(1, 10_000, 4_000);
			flexible_lock.securitization_ratio = FixedU128::from_rational(3, 2);
			flexible_lock.is_flexible = true;
			LocksById::<Test>::insert(2, flexible_lock);
			StorageVersion::new(17).put::<Pallet<Test>>();

			#[cfg(not(feature = "try-runtime"))]
			ReconcileBitcoinLockAccountingMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			ReconcileBitcoinLockAccountingMigration::<Test>::try_on_runtime_upgrade(true)
				.expect("runtime upgrade checks");

			let vault = VaultsById::<Test>::get(1).expect("vault");
			assert_eq!(vault.total_satoshis, 16_000);
			assert_eq!(vault.securitized_satoshis, 14_000);
			assert_eq!(vault.ratio_adjusted_satoshis, 16_000);
			assert_eq!(vault.flexible_ratio_adjusted_satoshis, 6_000);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), StorageVersion::new(18));
		});
	}
}
