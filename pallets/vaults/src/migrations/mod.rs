use crate::{Config, Pallet, VaultsById};
use argon_primitives::{
	bitcoin::{BitcoinHeight, Satoshis},
	tick::Tick,
	vault::{SecuritizationScheduleEntry, VaultTerms},
	VaultId,
};
use codec::{Decode, Encode};
use frame_support::{
	migrations::VersionedMigration, storage_alias, traits::UncheckedOnRuntimeUpgrade,
};
use pallet_prelude::*;
use sp_runtime::BoundedBTreeMap;

// At finalized mainnet block 957572 (runtime 159), all 146 existing Locks were 1x.
// Removing the ratio therefore changes no backing amounts or dependent accounting. The
// Bitcoin Locks preflight rejects non-1x Locks if that changes before this upgrade.
// Preserve schedules, release indexes, and operational/Treasury positions. Vault revenue
// entries record actual historical fees, earnings, and snapshots; they must not be rewritten.
/// Remove the ratio and ratio-adjusted counters without changing Vault accounting or schedules.
pub struct RemoveVaultSecuritizationRatio<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for RemoveVaultSecuritizationRatio<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut count = 0u64;
		VaultsById::<T>::translate::<v19::Vault<T>, _>(|_, vault| {
			count += 1;
			Some(vault.into_current())
		});
		T::DbWeight::get().reads_writes(count + 1, count)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		ensure!(
			frame_system::LastRuntimeUpgrade::<T>::get()
				.is_some_and(|version| version.spec_version.0 == 160),
			"runtime 160 must be applied before runtime 161"
		);
		let vaults = v19::VaultsById::<T>::iter()
			.map(|(id, vault)| (id, vault.into_current().encode()))
			.collect::<Vec<_>>();
		Ok(vaults.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let vaults = Vec::<(VaultId, Vec<u8>)>::decode(&mut state.as_slice())
			.map_err(|_| "invalid Vault migration state")?;
		ensure!(VaultsById::<T>::iter_keys().count() == vaults.len(), "Vault count changed");
		for (id, encoded) in vaults {
			ensure!(
				VaultsById::<T>::get(id).map(|vault| vault.encode()) == Some(encoded),
				"Vault migration changed accounting or schedules"
			);
		}
		Ok(())
	}
}

pub type RemoveVaultSecuritizationRatioMigration<T> = VersionedMigration<
	19,
	20,
	RemoveVaultSecuritizationRatio<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

/// Exact runtime 160 Vault layout; runtime 159 upgrades must finish before runtime 161.
pub mod v19 {
	use super::*;

	#[derive(Encode, Decode, CloneNoBound)]
	pub struct Vault<T: Config> {
		/// The account assigned to operate this vault
		pub operator_account_id: T::AccountId,
		/// Optional delegated hot account allowed to act on behalf of this vault.
		pub delegate_account_id: Option<T::AccountId>,
		/// The securitization in the vault
		#[codec(compact)]
		pub securitization: T::Balance,
		/// The target securitization to have in the vault (in case of reducing)
		#[codec(compact)]
		pub securitization_target: T::Balance,
		/// The securitization locked for bitcoin (at the ratio given)
		#[codec(compact)]
		pub securitization_locked: T::Balance,
		/// The funded flexible portion of `securitization_locked`.
		#[codec(compact)]
		pub flexible_securitization_locked: T::Balance,
		/// Vault securitization space reserved for future Bitcoin locks.
		#[codec(compact)]
		pub reserved_securitization_space: T::Balance,
		/// Securitization pending bitcoin funding confirmation (this is "out of" the
		/// securitization_locked, not in addition to)
		#[codec(compact)]
		pub securitization_pending_activation: T::Balance,
		/// Confirmed satoshis currently backed by activated securitization.
		#[codec(compact)]
		pub securitized_satoshis: Satoshis,
		/// Total confirmed satoshis currently held across this Vault's Locks.
		#[codec(compact)]
		pub total_satoshis: Satoshis,
		/// Funded satoshis adjusted by each Lock's securitization ratio.
		#[codec(compact)]
		pub ratio_adjusted_satoshis: Satoshis,
		/// The funded flexible portion of `ratio_adjusted_satoshis`.
		#[codec(compact)]
		pub flexible_ratio_adjusted_satoshis: Satoshis,
		/// Commitments and whole withdrawals grouped by their next Bitcoin day release boundary.
		pub securitization_release_schedule:
			BoundedBTreeMap<BitcoinHeight, SecuritizationScheduleEntry<T::Balance>, ConstU32<366>>,
		/// Microgons used in reward snapshots that require one-year withdrawal notice.
		#[codec(compact)]
		pub committed_microgons: T::Balance,
		/// The securitization ratio of "total securitization" to "available for locked bitcoin"
		#[codec(compact)]
		pub securitization_ratio: FixedU128,
		/// If the vault is closed, no new bitcoin locks can be issued
		pub is_closed: bool,
		/// The terms for locked bitcoin
		pub terms: VaultTerms<T::Balance>,
		/// The terms that are pending to be applied to this vault at the given tick
		pub pending_terms: Option<(Tick, VaultTerms<T::Balance>)>,
		/// A tick at which this vault is active
		#[codec(compact)]
		pub opened_tick: Tick,
	}

	#[storage_alias]
	pub type VaultsById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, VaultId, Vault<T>, OptionQuery>;

	impl<T: Config> Vault<T> {
		pub fn into_current(self) -> argon_primitives::vault::Vault<T::AccountId, T::Balance> {
			argon_primitives::vault::Vault {
				operator_account_id: self.operator_account_id,
				delegate_account_id: self.delegate_account_id,
				securitization: self.securitization,
				securitization_target: self.securitization_target,
				securitization_locked: self.securitization_locked,
				flexible_securitization_locked: self.flexible_securitization_locked,
				reserved_securitization_space: self.reserved_securitization_space,
				securitization_pending_activation: self.securitization_pending_activation,
				securitized_satoshis: self.securitized_satoshis,
				total_satoshis: self.total_satoshis,
				securitization_release_schedule: self.securitization_release_schedule,
				committed_microgons: self.committed_microgons,
				is_closed: self.is_closed,
				terms: self.terms,
				pending_terms: self.pending_terms,
				opened_tick: self.opened_tick,
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, RevenuePerFrameByVault, VaultFrameRevenue, VaultFundsReleasingByHeight};
	#[cfg(feature = "try-runtime")]
	use frame_support::assert_ok;
	use frame_support::traits::OnRuntimeUpgrade;
	use sp_runtime::bounded_btree_map;

	fn stored_vault() -> v19::Vault<Test> {
		v19::Vault {
			operator_account_id: 1,
			delegate_account_id: Some(2),
			securitization: 1_000,
			securitization_target: 950,
			securitization_locked: 111,
			flexible_securitization_locked: 8,
			reserved_securitization_space: 50,
			securitization_pending_activation: 103,
			securitized_satoshis: 8,
			total_satoshis: 15,
			ratio_adjusted_satoshis: 8,
			flexible_ratio_adjusted_satoshis: 8,
			securitization_release_schedule: bounded_btree_map! {
				432 => SecuritizationScheduleEntry { locked_commitments: 111, relockable_commitments: 9, argon_withdrawals: 50, argonot_withdrawals: 7 }
			},
			committed_microgons: 500,
			securitization_ratio: FixedU128::from_u32(2),
			is_closed: true,
			terms: VaultTerms {
				bitcoin_annual_percent_rate: FixedU128::from_rational(1, 10),
				bitcoin_base_fee: 7,
			},
			pending_terms: Some((
				23,
				VaultTerms { bitcoin_annual_percent_rate: FixedU128::zero(), bitcoin_base_fee: 11 },
			)),
			opened_tick: 3,
		}
	}

	#[test]
	fn migration_preserves_accounting_schedules_and_revenue_and_runs_once() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(19).put::<Pallet<Test>>();
			frame_system::LastRuntimeUpgrade::<Test>::put(frame_system::LastRuntimeUpgradeInfo {
				spec_version: 160.into(),
				spec_name: "argon".into(),
			});
			let original = stored_vault();
			v19::VaultsById::<Test>::insert(1, &original);
			VaultFundsReleasingByHeight::<Test>::mutate(432, |ids| {
				ids.try_insert(1).unwrap();
			});
			let mut revenue = VaultFrameRevenue::<Test>::new(3);
			revenue.bitcoin_lock_fee_revenue = 17;
			revenue.uncollected_revenue = 12;
			revenue.securitization_activated = 16;
			revenue.treasury_vault_earnings = 7;
			RevenuePerFrameByVault::<Test>::mutate(1, |entries| {
				entries.try_push(revenue).unwrap();
			});
			let revenue_before = RevenuePerFrameByVault::<Test>::get(1).encode();
			#[cfg(feature = "try-runtime")]
			let state = RemoveVaultSecuritizationRatio::<Test>::pre_upgrade().unwrap();

			RemoveVaultSecuritizationRatioMigration::<Test>::on_runtime_upgrade();

			#[cfg(feature = "try-runtime")]
			assert_ok!(RemoveVaultSecuritizationRatio::<Test>::post_upgrade(state));
			let vault = VaultsById::<Test>::get(1).unwrap();
			assert_eq!((vault.securitization, vault.securitization_target), (1_000, 950));
			assert_eq!(vault.securitization_locked, 111);
			assert_eq!(vault.flexible_securitization_locked, 8);
			assert_eq!(vault.securitization_pending_activation, 103);
			assert_eq!(vault.reserved_securitization_space, 50);
			assert_eq!((vault.securitized_satoshis, vault.total_satoshis), (8, 15));
			assert_eq!(vault.committed_microgons, 500);
			assert_eq!(
				vault.securitization_release_schedule,
				original.securitization_release_schedule
			);
			assert_eq!(vault.terms, original.terms);
			assert_eq!(vault.pending_terms, original.pending_terms);
			assert_eq!(vault.delegate_account_id, Some(2));
			assert!(vault.is_closed);
			assert_eq!(VaultFundsReleasingByHeight::<Test>::get(432).into_inner(), [1].into());
			assert_eq!(RevenuePerFrameByVault::<Test>::get(1).encode(), revenue_before);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), StorageVersion::new(20));
			RemoveVaultSecuritizationRatioMigration::<Test>::on_runtime_upgrade();
			assert_eq!(VaultsById::<Test>::get(1).unwrap().encode(), vault.encode());
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn pre_upgrade_requires_runtime_160() {
		new_test_ext().execute_with(|| {
			frame_system::LastRuntimeUpgrade::<Test>::put(frame_system::LastRuntimeUpgradeInfo {
				spec_version: 159.into(),
				spec_name: "argon".into(),
			});
			assert!(RemoveVaultSecuritizationRatio::<Test>::pre_upgrade().is_err());
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn post_upgrade_detects_changed_schedules_and_missing_vaults() {
		new_test_ext().execute_with(|| {
			frame_system::LastRuntimeUpgrade::<Test>::put(frame_system::LastRuntimeUpgradeInfo {
				spec_version: 160.into(),
				spec_name: "argon".into(),
			});
			v19::VaultsById::<Test>::insert(1, stored_vault());
			let state = RemoveVaultSecuritizationRatio::<Test>::pre_upgrade().unwrap();
			RemoveVaultSecuritizationRatio::<Test>::on_runtime_upgrade();
			VaultsById::<Test>::mutate(1, |vault| {
				vault.as_mut().unwrap().securitization_release_schedule.clear()
			});
			assert!(RemoveVaultSecuritizationRatio::<Test>::post_upgrade(state.clone()).is_err());
			VaultsById::<Test>::remove(1);
			assert!(RemoveVaultSecuritizationRatio::<Test>::post_upgrade(state).is_err());
		});
	}
}
