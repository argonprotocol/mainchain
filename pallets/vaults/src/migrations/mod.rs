use crate::{Config, Pallet};
use argon_primitives::{
	bitcoin::{BitcoinHeight, BitcoinLockId, Satoshis},
	tick::Tick,
	treasury::{
		CollectAmounts, CollectSource, TreasuryPositionProvider, TreasuryPositionProviderWeightInfo,
	},
	vault::{SecuritizationScheduleEntry, VaultTerms},
	VaultId,
};
use codec::{Decode, Encode};
use frame_support::{
	storage::with_storage_layer,
	storage_alias,
	traits::{
		fungible::{InspectHold, MutateHold},
		OnRuntimeUpgrade,
	},
};
use pallet_prelude::*;
use sp_runtime::BoundedBTreeMap;

type PositionWeights<T> = <<T as Config>::PositionProvider as TreasuryPositionProvider<
	<T as frame_system::Config>::AccountId,
	<T as Config>::Balance,
>>::Weights;

pub fn is_migration_pending<T: Config>() -> bool {
	StorageVersion::get::<Pallet<T>>() == 19
}

pub fn is_migration_complete<T: Config>() -> bool {
	StorageVersion::get::<Pallet<T>>() == 20
}

/// Record completion after the runtime has converted the Vault, Lock and Fission storage.
pub fn complete_migration<T: Config>() {
	StorageVersion::new(20).put::<Pallet<T>>();
}

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

#[storage_alias]
pub type PendingCosignByVaultId<T: Config> = StorageMap<
	Pallet<T>,
	Twox64Concat,
	VaultId,
	BoundedBTreeSet<BitcoinLockId, <T as Config>::MaxPendingCosignsPerVault>,
	ValueQuery,
>;

#[storage_alias]
pub type OrphanedUtxoAccountsByVaultId<T: Config> = StorageDoubleMap<
	Pallet<T>,
	Twox64Concat,
	VaultId,
	Blake2_128Concat,
	<T as frame_system::Config>::AccountId,
	u32,
	ValueQuery,
>;

/// Move issued vault revenue from operator holds to the shared Collect account.
pub struct CollectMigration<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for CollectMigration<T> {
	fn on_runtime_upgrade() -> Weight {
		if StorageVersion::get::<Pallet<T>>() != StorageVersion::new(20) {
			return T::DbWeight::get().reads(1)
		}
		let mut accounts = 0u64;
		let mut retired_entries = 0u64;
		let result = with_storage_layer(|| -> DispatchResult {
			T::PositionProvider::prepare_collect()?;
			let custody =
				T::PositionProvider::collect_account().ok_or(DispatchError::Unavailable)?;
			for (vault_id, vault) in crate::VaultsById::<T>::iter() {
				accounts.saturating_accrue(1);
				let recorded = crate::RevenuePerFrameByVault::<T>::get(vault_id)
					.iter()
					.fold(T::Balance::zero(), |sum, frame| {
						sum.saturating_add(frame.uncollected_revenue)
					});
				if recorded.is_zero() {
					continue
				}

				let account = &vault.operator_account_id;
				ensure!(
					crate::VaultIdByOperator::<T>::get(account) == Some(vault_id),
					crate::Error::<T>::VaultRevenueMismatch
				);
				let amount = T::Currency::balance_on_hold(
					&crate::HoldReason::PendingCollect.into(),
					account,
				);
				ensure!(amount == recorded, crate::Error::<T>::VaultRevenueMismatch);
				T::Currency::transfer_on_hold(
					&crate::HoldReason::PendingCollect.into(),
					account,
					&custody,
					amount,
					Precision::Exact,
					Restriction::Free,
					Fortitude::Force,
				)?;
				T::PositionProvider::record_funding(CollectAmounts::from_source(
					CollectSource::Vaults,
					amount,
				));
				T::PositionProvider::accrue(account, CollectSource::Vaults, amount)?;
			}
			retired_entries.saturating_accrue(
				PendingCosignByVaultId::<T>::clear(u32::MAX, None).unique.into(),
			);
			retired_entries.saturating_accrue(
				OrphanedUtxoAccountsByVaultId::<T>::clear(u32::MAX, None).unique.into(),
			);
			StorageVersion::new(21).put::<Pallet<T>>();
			Ok(())
		});
		if let Err(error) = result {
			log::error!("Vault Collect migration failed: {error:?}");
		}
		let account_weight =
			PositionWeights::<T>::record_funding().saturating_add(PositionWeights::<T>::accrue());
		T::DbWeight::get()
			.reads_writes(
				accounts.saturating_mul(8).saturating_add(retired_entries).saturating_add(5),
				accounts.saturating_mul(6).saturating_add(retired_entries).saturating_add(1),
			)
			.saturating_add(PositionWeights::<T>::prepare_collect())
			.saturating_add(account_weight.saturating_mul(accounts))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, HoldReason, RevenuePerFrameByVault, VaultsById};
	use frame_support::{
		assert_ok,
		traits::fungible::{Inspect, MutateHold},
	};

	#[test]
	fn held_revenue_moves_once_and_a_mismatch_preserves_all_old_balances() {
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(2);
			System::set_block_number(1);
			assert_ok!(TreasuryPositions::prepare_collect());
			for account in [1, 2] {
				set_argons(account, 100_000);
				assert_ok!(Vaults::create(RuntimeOrigin::signed(account), default_vault()));
				assert_ok!(Balances::hold(&HoldReason::PendingCollect.into(), &account, 100));
				assert_ok!(Vaults::mutate_frame_revenue(account as u32, 1, |frame| {
					frame.uncollected_revenue = 100;
					Ok(())
				}));
			}
			let last_vault = VaultsById::<Test>::iter_keys().last().unwrap();
			let mut closed_vault = VaultsById::<Test>::get(1).unwrap();
			closed_vault.is_closed = true;
			VaultsById::<Test>::insert(3, closed_vault);
			RevenuePerFrameByVault::<Test>::mutate(last_vault, |frames| {
				frames[0].uncollected_revenue = 101;
			});
			StorageVersion::new(20).put::<Vaults>();
			PendingCosignByVaultId::<Test>::mutate(1, |ids| {
				ids.try_insert(9).unwrap();
			});
			OrphanedUtxoAccountsByVaultId::<Test>::insert(1, 2, 1);
			let issuance = Balances::total_issuance();

			CollectMigration::<Test>::on_runtime_upgrade();
			assert_eq!(StorageVersion::get::<Vaults>(), StorageVersion::new(20));
			assert!(PendingCosignByVaultId::<Test>::contains_key(1));
			assert!(OrphanedUtxoAccountsByVaultId::<Test>::contains_key(1, 2));
			for account in [1, 2] {
				assert_eq!(
					Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &account),
					100
				);
				assert_eq!(vault_earnings(account), 0);
			}
			assert_eq!(
				Balances::balance(&TreasuryPositions::account_id()),
				ExistentialDeposit::get()
			);

			RevenuePerFrameByVault::<Test>::mutate(last_vault, |frames| {
				frames[0].uncollected_revenue = 100;
			});
			CollectMigration::<Test>::on_runtime_upgrade();
			CollectMigration::<Test>::on_runtime_upgrade();
			assert_eq!(StorageVersion::get::<Vaults>(), StorageVersion::new(21));
			assert!(!PendingCosignByVaultId::<Test>::contains_key(1));
			assert!(!OrphanedUtxoAccountsByVaultId::<Test>::contains_key(1, 2));
			assert_eq!(Balances::total_issuance(), issuance);
			for account in [1, 2] {
				assert_eq!(
					Balances::balance_on_hold(&HoldReason::PendingCollect.into(), &account),
					0
				);
				assert_eq!(vault_earnings(account), 100);
				assert_eq!(RevenuePerFrameByVault::<Test>::get(account as u32)[0].frame_id, 1);
				let wallet = Balances::balance(&account);
				assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(account)));
				assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(account)));
				assert_eq!(Balances::balance(&account), wallet + 100);
			}
			assert_eq!(Balances::total_issuance(), issuance);
		});
	}
}
