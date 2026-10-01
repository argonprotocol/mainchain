use crate::{
	ArgonotSecuritizationByVaultId, Config, Pallet, TotalVaultSecuritization,
	VaultFundsReleasingByHeight, VaultSecuritizationRanks, VaultsById,
};
use argon_primitives::{
	bitcoin::{BitcoinHeight, Satoshis},
	tick::Tick,
	vault::{SecuritizationScheduleEntry, Vault, VaultArgonotSecuritization, VaultTerms},
	AmountRankKey, VaultId,
};
use frame_support::{
	migrations::VersionedMigration, storage_alias, traits::UncheckedOnRuntimeUpgrade,
};
use pallet_bitcoin_locks::{Config as BitcoinLocksConfig, LockIdsByVaultId, LocksById};
use pallet_prelude::*;
use sp_runtime::{traits::SaturatedConversion, BoundedBTreeMap, BoundedBTreeSet, Permill};

#[cfg(feature = "try-runtime")]
use alloc::{collections::BTreeMap, vec::Vec};
use codec::{Decode, Encode, HasCompact};
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

mod old {
	use super::*;

	#[derive(Encode, Decode)]
	pub struct VaultTerms<Balance: HasCompact> {
		#[codec(compact)]
		pub bitcoin_annual_percent_rate: FixedU128,
		#[codec(compact)]
		pub bitcoin_base_fee: Balance,
		#[codec(compact)]
		pub treasury_profit_sharing: Permill,
	}

	#[derive(Encode, Decode)]
	pub struct Vault<T: Config> {
		pub operator_account_id: T::AccountId,
		pub delegate_account_id: Option<T::AccountId>,
		#[codec(compact)]
		pub securitization: T::Balance,
		#[codec(compact)]
		pub securitization_target: T::Balance,
		#[codec(compact)]
		pub securitization_locked: T::Balance,
		#[codec(compact)]
		pub flexible_securitization_locked: T::Balance,
		#[codec(compact)]
		pub reserved_securitization_space: T::Balance,
		#[codec(compact)]
		pub securitization_pending_activation: T::Balance,
		#[codec(compact)]
		pub securitized_satoshis: Satoshis,
		#[codec(compact)]
		pub total_satoshis: Satoshis,
		#[codec(compact)]
		pub ratio_adjusted_satoshis: Satoshis,
		#[codec(compact)]
		pub flexible_ratio_adjusted_satoshis: Satoshis,
		pub securitization_release_schedule:
			BoundedBTreeMap<BitcoinHeight, T::Balance, ConstU32<366>>,
		#[codec(compact)]
		pub securitization_ratio: FixedU128,
		pub is_closed: bool,
		pub terms: VaultTerms<T::Balance>,
		pub pending_terms: Option<(Tick, VaultTerms<T::Balance>)>,
		#[codec(compact)]
		pub opened_tick: Tick,
		pub operational_minimum_release_tick: Option<Tick>,
	}

	#[storage_alias]
	pub type VaultsById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, VaultId, Vault<T>, OptionQuery>;

	#[storage_alias]
	pub type VaultsReleasingOperationalMinimumByTick<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		Tick,
		BoundedBTreeSet<VaultId, <T as Config>::MaxVaults>,
		ValueQuery,
	>;

	#[derive(Encode, Decode)]
	pub struct VaultArgonotCommitment<Balance: HasCompact> {
		#[codec(compact)]
		pub committed_micronots: Balance,
		#[codec(compact)]
		pub encumbered_micronots: Balance,
	}

	#[storage_alias]
	pub type ArgonotCommitmentByVaultId<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		VaultId,
		VaultArgonotCommitment<<T as Config>::Balance>,
		OptionQuery,
	>;
}

/// Populate the ordered reward index and remove vault-wide profit-sharing terms.
pub struct IndexVaultSecuritizationAndRemoveProfitSharing<T>(core::marker::PhantomData<T>);

impl<T> UncheckedOnRuntimeUpgrade for IndexVaultSecuritizationAndRemoveProfitSharing<T>
where
	T: Config + BitcoinLocksConfig<Balance = <T as Config>::Balance>,
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let mut ranks = Vec::new();
		let mut terms = Vec::new();
		let mut total = 0u128;
		let current_height = <T as Config>::BitcoinBlockHeightChange::get().1;
		for (vault_id, vault) in old::VaultsById::<T>::iter() {
			let mut schedule = BTreeMap::<
				BitcoinHeight,
				SecuritizationScheduleEntry<<T as Config>::Balance>,
			>::new();
			for (height, amount) in &vault.securitization_release_schedule {
				schedule.entry(*height).or_default().relockable_commitments = *amount;
			}
			let pending_reduction =
				vault.securitization.saturating_sub(vault.securitization_target);
			if !pending_reduction.is_zero() {
				schedule.entry(current_height).or_default().argon_withdrawals = pending_reduction;
			}
			for lock_id in LockIdsByVaultId::<T>::iter_key_prefix(vault_id) {
				let Some(lock) = LocksById::<T>::get(lock_id) else { continue };
				let extension = lock.get_lock_extension();
				for (height, amount) in extension
					.collateral_expirations(lock.get_securitization().collateral_required())
				{
					if height > current_height {
						schedule
							.entry(height)
							.or_default()
							.locked_commitments
							.saturating_accrue(amount);
					}
				}
			}
			terms.push((
				vault_id,
				VaultTerms {
					bitcoin_annual_percent_rate: vault.terms.bitcoin_annual_percent_rate,
					bitcoin_base_fee: vault.terms.bitcoin_base_fee,
				},
				vault.pending_terms.map(|(tick, pending)| {
					(
						tick,
						VaultTerms {
							bitcoin_annual_percent_rate: pending.bitcoin_annual_percent_rate,
							bitcoin_base_fee: pending.bitcoin_base_fee,
						},
					)
				}),
				schedule,
				if vault.operational_minimum_release_tick.is_some() {
					<T as Config>::OperationalMinimumVaultSecuritization::get()
						.min(vault.securitization)
				} else {
					Zero::zero()
				},
			));
			if vault.is_closed || vault.securitization.is_zero() {
				continue;
			}
			let amount = vault.securitization.saturated_into::<u128>();
			total = total.saturating_add(amount);
			ranks.push(AmountRankKey::new(amount, vault_id));
		}
		ranks.sort_by(|left, right| {
			right
				.amount()
				.cmp(&left.amount())
				.then_with(|| left.holder_id().cmp(&right.holder_id()))
		});
		let argonots = old::ArgonotCommitmentByVaultId::<T>::iter()
			.map(|(vault_id, backing)| {
				(vault_id, backing.committed_micronots, backing.encumbered_micronots)
			})
			.collect::<Vec<_>>();
		Ok((ranks, total, terms, argonots).encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut total = <T as Config>::Balance::zero();
		let current_height = <T as Config>::BitcoinBlockHeightChange::get().1;
		let mut reads = 0u64;
		let mut writes = 1u64;
		let removed = old::VaultsReleasingOperationalMinimumByTick::<T>::drain().count() as u64;
		reads.saturating_accrue(removed);
		writes.saturating_accrue(removed);
		for (vault_id, backing) in old::ArgonotCommitmentByVaultId::<T>::drain() {
			ArgonotSecuritizationByVaultId::<T>::insert(
				vault_id,
				VaultArgonotSecuritization {
					held_micronots: backing.committed_micronots,
					committed_micronots: <T as Config>::Balance::zero(),
					encumbered_micronots: backing.encumbered_micronots,
				},
			);
			reads.saturating_accrue(1);
			writes.saturating_accrue(2);
		}
		VaultsById::<T>::translate::<old::Vault<T>, _>(|vault_id, vault| {
			reads.saturating_accrue(1);
			writes.saturating_accrue(1);
			if !vault.is_closed && !vault.securitization.is_zero() {
				VaultSecuritizationRanks::<T>::insert(
					AmountRankKey::new(vault.securitization.saturated_into::<u128>(), vault_id),
					(),
				);
				total.saturating_accrue(vault.securitization);
				writes.saturating_accrue(1);
			}
			let pending_reduction =
				vault.securitization.saturating_sub(vault.securitization_target);
			if !pending_reduction.is_zero() {
				VaultFundsReleasingByHeight::<T>::mutate(current_height.saturating_add(1), |ids| {
					ids.try_insert(vault_id).expect("all existing vaults fit the release index");
				});
				writes.saturating_accrue(1);
			}
			let mut securitization_release_schedule = BoundedBTreeMap::new();
			for (height, amount) in vault.securitization_release_schedule {
				securitization_release_schedule
					.try_insert(
						height,
						SecuritizationScheduleEntry {
							relockable_commitments: amount,
							..Default::default()
						},
					)
					.expect("existing releases fit the combined daily schedule");
			}
			let mut migrated = Vault {
				operator_account_id: vault.operator_account_id,
				delegate_account_id: vault.delegate_account_id,
				securitization: vault.securitization,
				securitization_target: vault.securitization_target,
				securitization_locked: vault.securitization_locked,
				flexible_securitization_locked: vault.flexible_securitization_locked,
				reserved_securitization_space: vault.reserved_securitization_space,
				securitization_pending_activation: vault.securitization_pending_activation,
				securitized_satoshis: vault.securitized_satoshis,
				total_satoshis: vault.total_satoshis,
				ratio_adjusted_satoshis: vault.ratio_adjusted_satoshis,
				flexible_ratio_adjusted_satoshis: vault.flexible_ratio_adjusted_satoshis,
				securitization_release_schedule,
				committed_microgons: if vault.operational_minimum_release_tick.is_some() {
					<T as Config>::OperationalMinimumVaultSecuritization::get()
						.min(vault.securitization)
				} else {
					Zero::zero()
				},
				securitization_ratio: vault.securitization_ratio,
				is_closed: vault.is_closed,
				terms: VaultTerms {
					bitcoin_annual_percent_rate: vault.terms.bitcoin_annual_percent_rate,
					bitcoin_base_fee: vault.terms.bitcoin_base_fee,
				},
				pending_terms: vault.pending_terms.map(|(tick, terms)| {
					(
						tick,
						VaultTerms {
							bitcoin_annual_percent_rate: terms.bitcoin_annual_percent_rate,
							bitcoin_base_fee: terms.bitcoin_base_fee,
						},
					)
				}),
				opened_tick: vault.opened_tick,
			};
			if !pending_reduction.is_zero() {
				// Existing reductions remain immediately due rather than starting a new notice.
				migrated
					.scheduled_release(current_height)
					.expect("one grandfathered notice fits")
					.argon_withdrawals = pending_reduction;
			}
			for lock_id in LockIdsByVaultId::<T>::iter_key_prefix(vault_id) {
				reads.saturating_accrue(2);
				let Some(lock) = LocksById::<T>::get(lock_id) else { continue };
				let extension = lock.get_lock_extension();
				for (height, amount) in extension
					.collateral_expirations(lock.get_securitization().collateral_required())
				{
					if height > current_height {
						migrated
							.scheduled_release(height)
							.expect("existing commitments fit the combined daily schedule")
							.locked_commitments
							.saturating_accrue(amount);
					}
				}
			}
			Some(migrated)
		});
		TotalVaultSecuritization::<T>::put(total);
		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let (expected_ranks, expected_total, expected_terms, expected_argonots) =
			<(
				Vec<AmountRankKey<VaultId>>,
				u128,
				Vec<(
					VaultId,
					VaultTerms<<T as Config>::Balance>,
					Option<(Tick, VaultTerms<<T as Config>::Balance>)>,
					BTreeMap<BitcoinHeight, SecuritizationScheduleEntry<<T as Config>::Balance>>,
					<T as Config>::Balance,
				)>,
				Vec<(VaultId, <T as Config>::Balance, <T as Config>::Balance)>,
			)>::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("invalid vault rank migration state"))?;
		ensure!(
			VaultSecuritizationRanks::<T>::iter_keys().collect::<Vec<_>>() == expected_ranks,
			TryRuntimeError::Other("vault securitization rank mismatch"),
		);
		ensure!(
			TotalVaultSecuritization::<T>::get().saturated_into::<u128>() == expected_total,
			TryRuntimeError::Other("vault securitization total mismatch"),
		);
		ensure!(
			VaultsById::<T>::iter().count() == expected_terms.len(),
			TryRuntimeError::Other("vault count changed during terms migration"),
		);
		for (vault_id, terms, pending_terms, schedule, committed_microgons) in expected_terms {
			let vault = VaultsById::<T>::get(vault_id)
				.ok_or(TryRuntimeError::Other("vault missing after terms migration"))?;
			ensure!(
				vault.terms == terms &&
					vault.pending_terms == pending_terms &&
					vault.committed_microgons == committed_microgons &&
					vault.securitization_release_schedule == schedule,
				TryRuntimeError::Other("vault terms changed during migration"),
			);
		}
		ensure!(
			old::VaultsReleasingOperationalMinimumByTick::<T>::iter_keys().next().is_none(),
			TryRuntimeError::Other("old operational minimum release queue remains after migration")
		);
		ensure!(
			old::ArgonotCommitmentByVaultId::<T>::iter_keys().next().is_none(),
			TryRuntimeError::Other("old Argonot backing remains after migration")
		);
		ensure!(
			ArgonotSecuritizationByVaultId::<T>::iter_keys().count() == expected_argonots.len(),
			TryRuntimeError::Other("Argonot backing count changed during migration")
		);
		for (vault_id, held_micronots, encumbered_micronots) in expected_argonots {
			ensure!(
				ArgonotSecuritizationByVaultId::<T>::get(vault_id) ==
					Some(VaultArgonotSecuritization {
						held_micronots,
						committed_micronots: <T as Config>::Balance::zero(),
						encumbered_micronots,
					}),
				TryRuntimeError::Other("Argonot backing changed during migration")
			);
		}
		Ok(())
	}
}

pub type IndexVaultSecuritizationAndRemoveProfitSharingMigration<T> = VersionedMigration<
	18,
	19,
	IndexVaultSecuritizationAndRemoveProfitSharing<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		mock::{
			new_test_ext, set_argons, BitcoinLocks, CurrentFrameId, RuntimeOrigin, Test, Vaults,
		},
		tests::keys,
		VaultConfig,
	};
	use argon_primitives::{bitcoin::SATOSHIS_PER_BITCOIN, MICROGONS_PER_ARGON};
	use bitcoin::bip32::Xpub;
	use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};

	fn vault(operator: u64, securitization: u128, is_closed: bool) -> old::Vault<Test> {
		old::Vault {
			operator_account_id: operator,
			delegate_account_id: None,
			securitization,
			securitization_target: securitization,
			securitization_locked: 0,
			flexible_securitization_locked: 0,
			reserved_securitization_space: 0,
			securitized_satoshis: 0,
			ratio_adjusted_satoshis: 0,
			flexible_ratio_adjusted_satoshis: 0,
			terms: old::VaultTerms {
				bitcoin_annual_percent_rate: FixedU128::zero(),
				bitcoin_base_fee: 0,
				treasury_profit_sharing: Permill::from_percent(30),
			},
			securitization_ratio: FixedU128::one(),
			opened_tick: 1,
			securitization_release_schedule: Default::default(),
			is_closed,
			pending_terms: Some((
				3,
				old::VaultTerms {
					bitcoin_annual_percent_rate: FixedU128::from_rational(1, 10),
					bitcoin_base_fee: 7,
					treasury_profit_sharing: Permill::from_percent(40),
				},
			)),
			securitization_pending_activation: 0,
			total_satoshis: 0,
			operational_minimum_release_tick: None,
		}
	}

	#[test]
	fn seeds_only_open_vaults_and_the_full_reward_total() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(18).put::<Pallet<Test>>();
			let mut pending_exit = vault(1, 50_000, false);
			pending_exit.securitization_target = 40_000;
			old::VaultsById::<Test>::insert(1, pending_exit);
			let mut operational = vault(2, 70_000, false);
			operational.operational_minimum_release_tick = Some(40);
			old::VaultsById::<Test>::insert(2, operational);
			old::VaultsReleasingOperationalMinimumByTick::<Test>::mutate(40, |ids| {
				ids.try_insert(2).unwrap();
			});
			old::VaultsById::<Test>::insert(3, vault(3, 90_000, true));
			#[cfg(feature = "try-runtime")]
			let upgrade_state =
				IndexVaultSecuritizationAndRemoveProfitSharing::<Test>::pre_upgrade().unwrap();

			IndexVaultSecuritizationAndRemoveProfitSharingMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			IndexVaultSecuritizationAndRemoveProfitSharing::<Test>::post_upgrade(upgrade_state)
				.unwrap();

			let ranked_ids = VaultSecuritizationRanks::<Test>::iter_keys()
				.map(|rank| rank.holder_id())
				.collect::<Vec<_>>();
			assert_eq!(ranked_ids, vec![2, 1]);
			assert_eq!(TotalVaultSecuritization::<Test>::get(), 120_000);
			let migrated = VaultsById::<Test>::get(1).expect("migrated vault");
			assert_eq!(migrated.exit_notice_amount(), 10_000);
			assert_eq!(migrated.committed_microgons, 0);
			assert_eq!(
				VaultsById::<Test>::get(2).unwrap().committed_microgons,
				<Test as Config>::OperationalMinimumVaultSecuritization::get(),
			);
			assert!(old::VaultsReleasingOperationalMinimumByTick::<Test>::iter_keys()
				.next()
				.is_none());
			let next_height = <Test as Config>::BitcoinBlockHeightChange::get().1 + 1;
			assert!(VaultFundsReleasingByHeight::<Test>::get(next_height).contains(&1));
			assert_eq!(migrated.terms.bitcoin_base_fee, 0);
			assert_eq!(
				migrated.pending_terms.as_ref().map(|(_, terms)| terms.bitcoin_base_fee),
				Some(7)
			);
			assert_eq!(migrated.pending_terms.map(|(tick, _)| tick), Some(3));
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), StorageVersion::new(19));
		});
	}

	#[test]
	fn seeds_existing_bitcoin_maturities_without_releasing_or_rotating() {
		new_test_ext().execute_with(|| {
			let amount = 100_000 * MICROGONS_PER_ARGON;
			set_argons(1, amount);
			let xpub = keys();
			let owner_pubkey = Xpub::decode(&xpub.0).unwrap().public_key.serialize().into();
			assert_ok!(Vaults::create(
				RuntimeOrigin::signed(1),
				VaultConfig {
					terms: VaultTerms {
						bitcoin_annual_percent_rate: FixedU128::zero(),
						bitcoin_base_fee: 0
					},
					delegate_account_id: None,
					securitization: amount,
					bitcoin_xpubkey: xpub,
					securitization_ratio: FixedU128::one(),
				}
			));
			assert_ok!(BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				owner_pubkey,
				None
			));
			let lock = LocksById::<Test>::get(1).unwrap();
			let collateral = lock.get_securitization().collateral_required();
			let expiration = lock.get_lock_extension().expiration_day();
			LocksById::<Test>::mutate(1, |lock| {
				lock.as_mut()
					.unwrap()
					.fund_hold_extensions
					.try_insert(expiration + 144, collateral / 2)
					.unwrap();
			});
			let mut existing = vault(1, amount, false);
			existing.securitization_locked = collateral;
			existing.securitization_pending_activation = collateral;
			existing.securitization_target = amount - 7;
			existing.securitization_release_schedule.try_insert(expiration, 11).unwrap();
			let current_height = <Test as Config>::BitcoinBlockHeightChange::get().1;
			existing.securitization_release_schedule.try_insert(current_height, 13).unwrap();
			old::VaultsById::<Test>::insert(1, existing);
			old::ArgonotCommitmentByVaultId::<Test>::insert(
				1,
				old::VaultArgonotCommitment { committed_micronots: 100, encumbered_micronots: 30 },
			);
			StorageVersion::new(18).put::<Pallet<Test>>();
			#[cfg(feature = "try-runtime")]
			let state = IndexVaultSecuritizationAndRemoveProfitSharing::<Test>::pre_upgrade().unwrap();

			IndexVaultSecuritizationAndRemoveProfitSharingMigration::<Test>::on_runtime_upgrade();

			#[cfg(feature = "try-runtime")]
			IndexVaultSecuritizationAndRemoveProfitSharing::<Test>::post_upgrade(state).unwrap();
			let migrated = VaultsById::<Test>::get(1).unwrap();
			assert_eq!(
				migrated.securitization_release_schedule[&expiration],
				SecuritizationScheduleEntry {
					locked_commitments: collateral / 2,
					relockable_commitments: 11,
					argon_withdrawals: 0,
					argonot_withdrawals: 0,
				}
			);
			assert_eq!(
				migrated.securitization_release_schedule[&(expiration + 144)].locked_commitments,
				collateral / 2
			);
			assert_eq!(migrated.securitization, amount);
			assert_eq!(migrated.securitization_locked, collateral);
			assert_eq!(migrated.get_relock_capacity(), 24);
			assert_eq!(
				migrated.securitization_release_schedule[&current_height],
				SecuritizationScheduleEntry {
					locked_commitments: 0,
					relockable_commitments: 13,
					argon_withdrawals: 7,
					argonot_withdrawals: 0,
				}
			);
			assert_eq!(CurrentFrameId::get(), 1);
			assert_eq!(
				ArgonotSecuritizationByVaultId::<Test>::get(1),
				Some(VaultArgonotSecuritization {
					held_micronots: 100,
					committed_micronots: 0,
					encumbered_micronots: 30,
				})
			);
		});
	}
}
