mod vault_securitized_satoshis;
pub use vault_securitized_satoshis::AddVaultSecuritizedSatoshis;

#[cfg(feature = "try-runtime")]
use alloc::collections::BTreeMap;
use argon_primitives::{treasury::TreasuryPositionProvider, OperationalAccountsHook};
#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
use frame_support::traits::OnRuntimeUpgrade;
use pallet_bitcoin_fissions::{
	migrations::{self as fissions_migrations, v1},
	Config as FissionsConfig, FissionByOwnerAndId, FissionIdsByLockId,
};
use pallet_bitcoin_locks::{
	migrations::{self as locks_migrations, v11},
	Config as LocksConfig, LockIdsByVaultId, LockedBitcoin, LocksById,
	SecuritizationHoldExpirationsByBitcoinHeight,
};
use pallet_prelude::*;
use pallet_vaults::{
	migrations::{self as vaults_migrations, v19},
	Config as VaultsConfig, VaultFundsReleasingByHeight, VaultsById,
};

#[cfg(feature = "try-runtime")]
use {
	argon_primitives::{
		bitcoin::{BitcoinHeight, BitcoinLockId, FissionId},
		treasury::BitcoinLockPosition,
		vault::{BitcoinSecuritizationBasis, CosignerGuarantee},
		VaultId,
	},
	pallet_bitcoin_fissions::migrations::v1 as old_fissions,
	sp_runtime::TryRuntimeError,
};

/// Convert runtime-160 Vaults, Locks and Fissions together, preserving securitization and release
/// dates.
pub struct CosignerGuaranteesMigration<T>(PhantomData<T>);

impl<T> OnRuntimeUpgrade for CosignerGuaranteesMigration<T>
where
	T: VaultsConfig
		+ LocksConfig<Balance = <T as VaultsConfig>::Balance>
		+ FissionsConfig<Balance = <T as VaultsConfig>::Balance>,
{
	fn on_runtime_upgrade() -> Weight {
		let vaults_pending = vaults_migrations::is_migration_pending::<T>();
		let locks_pending = locks_migrations::is_migration_pending::<T>();
		let fissions_pending = fissions_migrations::is_migration_pending::<T>();

		if !vaults_pending || !locks_pending || !fissions_pending {
			log::info!("Skipping cosigner guarantee migration: not all three pallets are pending");
			return T::DbWeight::get().reads(3);
		}

		let current_bitcoin_height = <T as VaultsConfig>::BitcoinBlockHeightChange::get().1;
		let mut accesses = 3u64;
		VaultsById::<T>::translate::<v19::Vault<T>, _>(|vault_id, source_vault| {
			let mut migrated_vault = source_vault.into_current();
			let previous_vault_activation = migrated_vault.get_activated_securitization();
			accesses.saturating_accrue(1);

			for lock_id in LockIdsByVaultId::<T>::iter_key_prefix(vault_id) {
				let Some(source_lock) = v11::LocksById::<T>::get(lock_id) else { continue };

				let funding_expiration_height =
					source_lock.securitization_hold_expiration_bitcoin_height;
				let mut migrated_lock: LockedBitcoin<T> = source_lock.clone().into();
				let source_lock_guarantee = migrated_lock.cosigner_guarantee.clone();
				let source_activated_securitization =
					source_lock.securitization_for_satoshis(migrated_lock.funded_satoshis);
				let lock_guarantee = &mut migrated_lock.cosigner_guarantee;
				let mut assigned_fission_satoshis = 0u64;
				let mut assigned_fission_microgons = <T as VaultsConfig>::Balance::zero();

				// Split Liquid guarantees at the original price, preserving rounding and holds.
				for fission_id in FissionIdsByLockId::<T>::get(lock_id) {
					let Some(source_fission) =
						v1::FissionByOwnerAndId::<T>::get(&migrated_lock.owner_account, fission_id)
					else {
						continue
					};

					assigned_fission_satoshis.saturating_accrue(source_fission.satoshis);
					let cumulative_fission_microgons =
						source_lock.securitization_for_satoshis(assigned_fission_satoshis);
					let fission_guaranteed_microgons =
						cumulative_fission_microgons.saturating_sub(assigned_fission_microgons);
					assigned_fission_microgons = cumulative_fission_microgons;
					let Some(fission_guarantee) = lock_guarantee
						.take_portion(source_fission.satoshis, fission_guaranteed_microgons)
					else {
						continue
					};

					FissionByOwnerAndId::<T>::insert(
						&migrated_lock.owner_account,
						fission_id,
						source_fission.into_current(fission_guarantee),
					);
					accesses.saturating_accrue(1);
				}

				// Keep the funding window, or shrink the remaining guarantee to confirmed Bitcoin.
				let funding_expiration_pending =
					SecuritizationHoldExpirationsByBitcoinHeight::<T>::get(
						funding_expiration_height,
					)
					.contains(&lock_id);
				migrated_lock.securitization_hold_expiration_bitcoin_height =
					funding_expiration_pending.then_some(funding_expiration_height);
				let mut released_securitization = <T as VaultsConfig>::Balance::zero();
				if !funding_expiration_pending {
					let retained_securitization =
						source_activated_securitization.saturating_sub(assigned_fission_microgons);
					released_securitization =
						lock_guarantee.guaranteed_microgons.saturating_sub(retained_securitization);
					lock_guarantee.guaranteed_microgons = retained_securitization;
					lock_guarantee.basis.satoshis = lock_guarantee.basis.satoshis.min(
						migrated_lock
							.funded_satoshis
							.saturating_sub(migrated_lock.fissioned_satoshis),
					);

					let mut hold_microgons_to_retain = retained_securitization;
					lock_guarantee.fund_hold_extensions.retain(|_, hold_microgons| {
						*hold_microgons = (*hold_microgons).min(hold_microgons_to_retain);
						hold_microgons_to_retain.saturating_reduce(*hold_microgons);
						!hold_microgons.is_zero()
					});
				}

				if !released_securitization.is_zero() {
					// Release the excess at its existing maturities.
					let mut allocated_microgons_to_skip = source_lock_guarantee
						.guaranteed_microgons
						.saturating_sub(released_securitization);
					for (release_height, scheduled_microgons) in source_lock_guarantee
						.collateral_expirations(
							migrated_lock.vault_claim_height,
							source_lock_guarantee.guaranteed_microgons,
						) {
						let released_microgons =
							scheduled_microgons.saturating_sub(allocated_microgons_to_skip);
						allocated_microgons_to_skip.saturating_reduce(scheduled_microgons);
						if released_microgons.is_zero() {
							continue
						}

						let Some(release_schedule_entry) =
							migrated_vault.securitization_release_schedule.get_mut(&release_height)
						else {
							continue
						};
						release_schedule_entry
							.locked_commitments
							.saturating_reduce(released_microgons);
						if release_height <= current_bitcoin_height {
							continue
						}
						release_schedule_entry
							.relockable_commitments
							.saturating_accrue(released_microgons);

						// Preflight verifies that the source Vaults fit in a release bucket.
						VaultFundsReleasingByHeight::<T>::mutate(release_height, |ids| {
							let _ = ids.try_insert(vault_id);
						});
						accesses.saturating_accrue(1);
					}
				}

				// Update Vault and Treasury totals for the split guarantees.
				let migrated_activated_securitization = migrated_lock
					.lock_held_securitization()
					.activated_securitization
					.saturating_add(assigned_fission_microgons);
				migrated_vault.securitization_locked.saturating_reduce(released_securitization);
				migrated_vault.securitization_pending_activation = migrated_vault
					.securitization_pending_activation
					.saturating_add(source_activated_securitization)
					.saturating_sub(migrated_activated_securitization)
					.saturating_sub(released_securitization);
				if migrated_lock.is_flexible {
					migrated_vault.flexible_securitization_locked = migrated_vault
						.flexible_securitization_locked
						.saturating_sub(source_activated_securitization)
						.saturating_add(migrated_activated_securitization);
				}

				if let Some(mut upstream_position) =
					<T as LocksConfig>::PositionProvider::upstream_position(
						&migrated_lock.owner_account,
					)
					.filter(|position| position.vault_id == vault_id)
				{
					upstream_position
						.bitcoin_allocated_securitization
						.saturating_reduce(released_securitization);
					upstream_position.bitcoin_securitization = upstream_position
						.bitcoin_securitization
						.saturating_sub(source_activated_securitization)
						.saturating_add(migrated_activated_securitization);
					<T as LocksConfig>::PositionProvider::set_upstream_position(
						&migrated_lock.owner_account,
						Some(upstream_position),
					);
				}

				LocksById::<T>::insert(lock_id, migrated_lock);
				accesses.saturating_accrue(5);
			}

			migrated_vault
				.securitization_release_schedule
				.retain(|_, entry| !entry.is_empty());
			if previous_vault_activation != migrated_vault.get_activated_securitization() {
				<T as VaultsConfig>::OperationalAccountsHook::vault_bitcoin_lock_funded(
					&migrated_vault.operator_account_id,
					migrated_vault.get_activated_securitization(),
				);
				accesses.saturating_accrue(3);
			}

			Some(migrated_vault)
		});

		locks_migrations::complete_migration::<T>();
		fissions_migrations::complete_migration::<T>();
		vaults_migrations::complete_migration::<T>();

		T::DbWeight::get().reads_writes(accesses.saturating_add(3), accesses.saturating_add(1))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		if vaults_migrations::is_migration_complete::<T>() &&
			locks_migrations::is_migration_complete::<T>() &&
			fissions_migrations::is_migration_complete::<T>()
		{
			return Ok(Vec::new());
		}
		ensure!(vaults_migrations::is_migration_pending::<T>(), "expected source Vault storage");
		ensure!(locks_migrations::is_migration_pending::<T>(), "expected source Lock storage");
		ensure!(
			fissions_migrations::is_migration_pending::<T>(),
			"expected source Fission storage"
		);
		ensure!(
			frame_system::LastRuntimeUpgrade::<T>::get().is_some_and(|v| v.spec_version.0 == 160),
			"runtime 160 must precede this upgrade"
		);
		let source_vaults = v19::VaultsById::<T>::iter()
			.map(|(id, source_vault)| (id, source_vault.into_current()))
			.collect::<BTreeMap<_, _>>();
		ensure!(
			source_vaults.len() <= <T as VaultsConfig>::MaxVaults::get() as usize,
			"source Vaults exceed release index capacity"
		);
		let source_release_index =
			VaultFundsReleasingByHeight::<T>::iter().collect::<BTreeMap<_, _>>();
		ensure!(
			source_release_index
				.values()
				.all(|ids| ids.iter().all(|id| source_vaults.contains_key(id))),
			"release index contains missing Vault"
		);
		let mut source_vault_totals = BTreeMap::<
			VaultId,
			(
				u64,
				u64,
				<T as VaultsConfig>::Balance,
				<T as VaultsConfig>::Balance,
				<T as VaultsConfig>::Balance,
			),
		>::new();
		let mut source_upstream_positions = BTreeMap::new();
		let mut source_upstream_totals = BTreeMap::<
			T::AccountId,
			(<T as VaultsConfig>::Balance, <T as VaultsConfig>::Balance),
		>::new();

		let mut funding_expirations = BTreeMap::new();
		for (funding_expiration_height, lock_ids) in
			SecuritizationHoldExpirationsByBitcoinHeight::<T>::iter()
		{
			for lock_id in lock_ids {
				ensure!(
					funding_expirations.insert(lock_id, funding_expiration_height).is_none(),
					"Lock has multiple funding expiry entries"
				);
			}
		}

		let mut source_fissions_by_lock = BTreeMap::<BitcoinLockId, Vec<_>>::new();
		let mut source_fissions = Vec::new();
		for (owner, fission_id, source_fission) in old_fissions::FissionByOwnerAndId::<T>::iter() {
			source_fissions.push((owner.clone(), fission_id, source_fission.clone()));
			source_fissions_by_lock.entry(source_fission.lock_id).or_default().push((
				owner,
				fission_id,
				source_fission,
			));
		}

		let mut source_locks = Vec::new();
		for (lock_id, source_lock) in v11::LocksById::<T>::iter() {
			ensure!(
				source_lock.securitization_ratio == FixedU128::one(),
				"non-1x Bitcoin Lock requires conversion before runtime 161"
			);
			if let Some(funding_expiration_height) = funding_expirations.get(&lock_id) {
				ensure!(
					*funding_expiration_height ==
						source_lock.securitization_hold_expiration_bitcoin_height,
					"funding expiry index differs from stored Lock height"
				);
			}
			let source_vault =
				source_vaults.get(&source_lock.vault_id).ok_or("Lock Vault missing")?;
			ensure!(
				LockIdsByVaultId::<T>::contains_key(source_lock.vault_id, lock_id),
				"Lock missing from Vault index"
			);
			let source_lock_guarantee =
				LockedBitcoin::<T>::from(source_lock.clone()).cosigner_guarantee;
			for (release_height, scheduled_microgons) in source_lock_guarantee
				.collateral_expirations(
					source_lock.vault_claim_height,
					source_lock_guarantee.guaranteed_microgons,
				) {
				if release_height > <T as VaultsConfig>::BitcoinBlockHeightChange::get().1 {
					ensure!(
						source_vault
							.securitization_release_schedule
							.get(&release_height)
							.is_some_and(|release_schedule_entry| release_schedule_entry
								.locked_commitments >=
								scheduled_microgons),
						"Lock maturity missing from Vault commitments"
					);
				}
			}
			let source_activated_securitization =
				source_lock.securitization_for_satoshis(source_lock.funded_satoshis);
			let (
				funded_satoshis,
				securitized_satoshis,
				allocated_securitization,
				activated_securitization,
				flexible_securitization,
			) = source_vault_totals.entry(source_lock.vault_id).or_default();
			funded_satoshis.saturating_accrue(source_lock.funded_satoshis);
			securitized_satoshis.saturating_accrue(
				source_lock_guarantee.securitized_satoshis(source_lock.funded_satoshis),
			);
			allocated_securitization.saturating_accrue(source_lock_guarantee.guaranteed_microgons);
			activated_securitization.saturating_accrue(source_activated_securitization);
			if source_lock.is_flexible {
				flexible_securitization.saturating_accrue(source_activated_securitization);
			}
			let upstream_position =
				<T as LocksConfig>::PositionProvider::upstream_position(&source_lock.owner_account);
			if upstream_position.as_ref().is_some_and(|p| p.vault_id == source_lock.vault_id) {
				let (allocated_securitization, activated_securitization) =
					source_upstream_totals.entry(source_lock.owner_account.clone()).or_default();
				allocated_securitization
					.saturating_accrue(source_lock_guarantee.guaranteed_microgons);
				activated_securitization.saturating_accrue(source_activated_securitization);
			}
			source_upstream_positions.insert(source_lock.owner_account.clone(), upstream_position);
			let lock_source_fissions = source_fissions_by_lock.remove(&lock_id).unwrap_or_default();
			let indexed_fission_ids = FissionIdsByLockId::<T>::get(lock_id);
			ensure!(
				indexed_fission_ids.len() == lock_source_fissions.len(),
				"Fission index differs from source records"
			);

			let funded_satoshis =
				source_lock.funded_satoshis.min(source_lock.securitization_basis.satoshis);
			let mut fissioned_satoshis = 0u64;
			for (owner, fission_id, source_fission) in &lock_source_fissions {
				ensure!(
					*owner == source_lock.owner_account,
					"Fission owner differs from source Lock"
				);
				ensure!(
					indexed_fission_ids.contains(fission_id),
					"Fission missing from Lock index"
				);
				ensure!(source_fission.satoshis > 0, "Fission has no satoshis");
				fissioned_satoshis = fissioned_satoshis
					.checked_add(source_fission.satoshis)
					.ok_or("Fission satoshis overflow")?;
				ensure!(
					fissioned_satoshis <= funded_satoshis,
					"Fission exceeds funded Lock guarantee"
				);
			}
			ensure!(
				fissioned_satoshis == source_lock.fissioned_satoshis,
				"Fission allocation differs from source Lock"
			);

			source_locks.push((lock_id, source_lock));
		}
		ensure!(source_fissions_by_lock.is_empty(), "Fission source Lock missing");
		ensure!(
			LockIdsByVaultId::<T>::iter().count() == source_locks.len(),
			"Vault Lock index contains extra records"
		);
		for (vault_id, source_vault) in &source_vaults {
			let (
				funded_satoshis,
				securitized_satoshis,
				allocated_securitization,
				activated_securitization,
				flexible_securitization,
			) = source_vault_totals.get(vault_id).copied().unwrap_or_default();
			ensure!(
				(
					source_vault.total_satoshis,
					source_vault.securitized_satoshis,
					source_vault.securitization_locked,
					source_vault.get_activated_securitization(),
					source_vault.flexible_securitization_locked
				) == (
					funded_satoshis,
					securitized_satoshis,
					allocated_securitization,
					activated_securitization,
					flexible_securitization
				),
				"source Vault accounting differs from Locks"
			);
		}
		for (owner, upstream_position) in &source_upstream_positions {
			if let Some(upstream_position) = upstream_position {
				let (allocated_securitization, activated_securitization) =
					source_upstream_totals.get(owner).copied().unwrap_or_default();
				ensure!(
					(
						upstream_position.bitcoin_allocated_securitization,
						upstream_position.bitcoin_securitization
					) == (allocated_securitization, activated_securitization),
					"source upstream accounting differs from Locks"
				);
			}
		}
		let source_vaults = source_vaults.into_iter().collect::<Vec<_>>();
		let source_frame_revenue = pallet_vaults::RevenuePerFrameByVault::<T>::iter()
			.map(|(vault_id, frame_revenue)| (vault_id, frame_revenue.encode()))
			.collect::<Vec<_>>();
		Ok((
			source_locks,
			source_fissions,
			funding_expirations,
			source_vaults,
			source_upstream_positions,
			source_release_index,
			source_frame_revenue,
		)
			.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		ensure!(
			vaults_migrations::is_migration_complete::<T>(),
			"Vault storage version not updated"
		);
		ensure!(locks_migrations::is_migration_complete::<T>(), "Lock storage version not updated");
		ensure!(
			fissions_migrations::is_migration_complete::<T>(),
			"Fission storage version not updated"
		);
		// An empty snapshot means all three pallets were already migrated.
		if state.is_empty() {
			return Ok(());
		}

		let (
			source_locks,
			source_fissions,
			funding_expirations,
			source_vaults,
			mut expected_upstream_positions,
			mut expected_release_index,
			source_frame_revenue,
		) = <(
			Vec<(BitcoinLockId, v11::LockedBitcoin<T>)>,
			Vec<(T::AccountId, FissionId, v1::Fission<T>)>,
			BTreeMap<BitcoinLockId, BitcoinHeight>,
			Vec<(
				VaultId,
				argon_primitives::vault::Vault<T::AccountId, <T as VaultsConfig>::Balance>,
			)>,
			BTreeMap<
				T::AccountId,
				Option<argon_primitives::treasury::UpstreamPosition<<T as VaultsConfig>::Balance>>,
			>,
			BTreeMap<BitcoinHeight, BoundedBTreeSet<VaultId, <T as VaultsConfig>::MaxVaults>>,
			Vec<(VaultId, Vec<u8>)>,
		)>::decode(&mut state.as_slice())
		.map_err(|_| "invalid guarantee migration state")?;
		ensure!(
			LocksById::<T>::iter_keys().count() == source_locks.len(),
			"Bitcoin Lock count changed"
		);
		ensure!(
			FissionByOwnerAndId::<T>::iter_keys().count() == source_fissions.len(),
			"Fission count changed"
		);
		let mut source_fissions_by_lock = BTreeMap::<BitcoinLockId, Vec<_>>::new();
		for (owner, fission_id, source_fission) in source_fissions {
			source_fissions_by_lock.entry(source_fission.lock_id).or_default().push((
				owner,
				fission_id,
				source_fission,
			));
		}

		let mut expected_vaults = source_vaults.into_iter().collect::<BTreeMap<_, _>>();
		let mut source_locks = source_locks.into_iter().collect::<BTreeMap<_, _>>();
		let lock_ids =
			VaultsById::<T>::iter_keys().flat_map(LockIdsByVaultId::<T>::iter_key_prefix);
		for lock_id in lock_ids {
			let source_lock = source_locks.remove(&lock_id).ok_or("unexpected migrated Lock")?;
			let migrated_lock =
				LocksById::<T>::get(lock_id).ok_or("Bitcoin Lock missing after migration")?;
			let mut expected_lock: LockedBitcoin<T> = source_lock.clone().into();
			expected_lock.cosigner_guarantee = migrated_lock.cosigner_guarantee.clone();
			expected_lock.securitization_hold_expiration_bitcoin_height =
				migrated_lock.securitization_hold_expiration_bitcoin_height;
			ensure!(
				migrated_lock.encode() == expected_lock.encode(),
				"Bitcoin Lock migration changed stored terms or funding"
			);

			let source_securitization_basis = BitcoinSecuritizationBasis {
				satoshis: source_lock.securitization_basis.satoshis,
				microgons_at_target_per_btc: source_lock
					.securitization_basis
					.microgons_at_target_per_btc,
				tick: source_lock.securitization_tick,
			};
			let source_lock_guarantee = CosignerGuarantee {
				basis: source_securitization_basis,
				guaranteed_microgons: source_lock.securitization_coverage_microgons,
				fund_hold_extensions: source_lock.fund_hold_extensions.clone(),
			};
			let mut lock_source_fissions =
				source_fissions_by_lock.remove(&lock_id).unwrap_or_default();
			lock_source_fissions.sort_by_key(|(_, fission_id, _)| *fission_id);
			let mut migrated_guarantees = Vec::new();
			for (owner, fission_id, source_fission) in lock_source_fissions {
				let migrated_fission = FissionByOwnerAndId::<T>::get(owner, fission_id)
					.ok_or("Fission missing after migration")?;
				let guaranteed_satoshis = source_fission.satoshis;
				ensure!(
					source_fission
						.into_current(migrated_fission.cosigner_guarantee.clone())
						.encode() == migrated_fission.encode(),
					"Fission migration changed stored terms"
				);
				migrated_guarantees
					.push((guaranteed_satoshis, migrated_fission.cosigner_guarantee));
			}
			let securitization_position = migrated_lock.total_securitization(lock_id);
			ensure!(
				securitization_position ==
					BitcoinLockPosition {
						allocated_securitization: if funding_expirations.contains_key(&lock_id) {
							source_lock.securitization_coverage_microgons
						} else {
							source_lock.securitization_for_satoshis(source_lock.funded_satoshis)
						},
						activated_securitization: migrated_lock
							.lock_held_securitization()
							.activated_securitization
							.checked_add(
								&source_lock
									.securitization_for_satoshis(source_lock.fissioned_satoshis)
							)
							.ok_or("migrated activated securitization overflows")?,
					},
				"migrated securitization does not match retained guarantees"
			);

			let retained_guarantee_satoshis = if funding_expirations.contains_key(&lock_id) {
				source_securitization_basis.satoshis
			} else {
				source_lock.funded_satoshis.min(source_securitization_basis.satoshis)
			};
			let guaranteed_satoshis = retained_guarantee_satoshis
				.checked_sub(source_lock.fissioned_satoshis)
				.ok_or("Fissions exceed the Lock guarantee")?;
			ensure!(
				migrated_lock.cosigner_guarantee.basis.satoshis == guaranteed_satoshis,
				"Lock guarantee quantity changed"
			);
			ensure!(
				migrated_lock.securitization_hold_expiration_bitcoin_height ==
					funding_expirations.get(&lock_id).copied(),
				"initial funding deadline changed"
			);
			if let Some(funding_expiration_height) =
				migrated_lock.securitization_hold_expiration_bitcoin_height
			{
				ensure!(
					SecuritizationHoldExpirationsByBitcoinHeight::<T>::get(
						funding_expiration_height
					)
					.contains(&lock_id),
					"initial funding expiry not indexed"
				);
			}
			migrated_guarantees
				.push((guaranteed_satoshis, migrated_lock.cosigner_guarantee.clone()));

			let migrated_allocated_securitization = migrated_guarantees.iter().fold(
				<T as VaultsConfig>::Balance::zero(),
				|total, (_, migrated_guarantee)| {
					total.saturating_add(migrated_guarantee.guaranteed_microgons)
				},
			);
			let migrated_activated_securitization =
				securitization_position.activated_securitization;
			let source_activated_securitization =
				source_lock.securitization_for_satoshis(source_lock.funded_satoshis);
			let released_securitization = source_lock
				.securitization_coverage_microgons
				.saturating_sub(migrated_allocated_securitization);
			let expected_vault =
				expected_vaults.get_mut(&source_lock.vault_id).ok_or("Lock Vault missing")?;
			expected_vault.securitization_locked.saturating_reduce(released_securitization);
			expected_vault.securitization_pending_activation = expected_vault
				.securitization_pending_activation
				.saturating_add(source_activated_securitization)
				.saturating_sub(migrated_activated_securitization)
				.saturating_sub(released_securitization);
			if source_lock.is_flexible {
				expected_vault.flexible_securitization_locked = expected_vault
					.flexible_securitization_locked
					.saturating_sub(source_activated_securitization)
					.saturating_add(migrated_activated_securitization);
			}
			if let Some(Some(expected_upstream_position)) =
				expected_upstream_positions.get_mut(&source_lock.owner_account) &&
				expected_upstream_position.vault_id == source_lock.vault_id
			{
				expected_upstream_position
					.bitcoin_allocated_securitization
					.saturating_reduce(released_securitization);
				expected_upstream_position.bitcoin_securitization = expected_upstream_position
					.bitcoin_securitization
					.saturating_sub(source_activated_securitization)
					.saturating_add(migrated_activated_securitization);
			}
			let mut allocated_securitization_by_height =
				BTreeMap::<BitcoinHeight, <T as VaultsConfig>::Balance>::new();
			for (_, migrated_guarantee) in &migrated_guarantees {
				for (release_height, scheduled_microgons) in migrated_guarantee
					.collateral_expirations(
						source_lock.vault_claim_height,
						migrated_guarantee.guaranteed_microgons,
					) {
					allocated_securitization_by_height
						.entry(release_height)
						.or_default()
						.saturating_accrue(scheduled_microgons);
				}
			}
			for (release_height, scheduled_microgons) in source_lock_guarantee
				.collateral_expirations(
					source_lock.vault_claim_height,
					source_lock_guarantee.guaranteed_microgons,
				) {
				let released_microgons = scheduled_microgons.saturating_sub(
					allocated_securitization_by_height.remove(&release_height).unwrap_or_default(),
				);
				if released_microgons.is_zero() {
					continue
				}
				if let Some(release_schedule_entry) =
					expected_vault.securitization_release_schedule.get_mut(&release_height)
				{
					release_schedule_entry.locked_commitments.saturating_reduce(released_microgons);
					if release_height > <T as VaultsConfig>::BitcoinBlockHeightChange::get().1 {
						release_schedule_entry
							.relockable_commitments
							.saturating_accrue(released_microgons);
						expected_release_index
							.entry(release_height)
							.or_default()
							.try_insert(source_lock.vault_id)
							.map_err(|_| "unreachable: preflight verified all Vaults fit")?;
					}
				}
			}

			// Verify each assigned guarantee amount and its hold extensions against the original
			// guarantee.
			let mut assigned_satoshis = 0;
			for (guaranteed_satoshis, migrated_guarantee) in migrated_guarantees {
				let previously_assigned_microgons =
					source_lock.securitization_for_satoshis(assigned_satoshis);
				assigned_satoshis = assigned_satoshis
					.checked_add(guaranteed_satoshis)
					.ok_or("assigned satoshis overflow")?;
				let expected_guaranteed_microgons = source_lock
					.securitization_for_satoshis(assigned_satoshis)
					.checked_sub(&previously_assigned_microgons)
					.ok_or("assigned securitization underflows")?;
				ensure!(
					migrated_guarantee.basis ==
						BitcoinSecuritizationBasis {
							satoshis: guaranteed_satoshis,
							..source_securitization_basis
						} && migrated_guarantee.guaranteed_microgons == expected_guaranteed_microgons,
					"Assigned guarantee basis or securitization changed"
				);

				let mut expected_hold_extensions = source_lock.fund_hold_extensions.clone();
				let mut microgons_to_skip = previously_assigned_microgons;
				let mut microgons_to_assign = expected_guaranteed_microgons;
				expected_hold_extensions.retain(|_, amount| {
					let assigned =
						amount.saturating_sub(microgons_to_skip).min(microgons_to_assign);
					microgons_to_skip = microgons_to_skip.saturating_sub(*amount);
					microgons_to_assign.saturating_reduce(assigned);
					*amount = assigned;
					!assigned.is_zero()
				});
				ensure!(
					migrated_guarantee.fund_hold_extensions == expected_hold_extensions,
					"Assigned guarantee release maturities changed"
				);
			}
		}
		ensure!(
			VaultsById::<T>::iter_keys().count() == expected_vaults.len(),
			"Vault count changed"
		);
		for (vault_id, mut expected_vault) in expected_vaults {
			expected_vault
				.securitization_release_schedule
				.retain(|_, entry| !entry.is_empty());
			ensure!(
				VaultsById::<T>::get(vault_id) == Some(expected_vault),
				"migrated Vault accounting or maturities differ"
			);
		}
		for (owner, expected_upstream_position) in expected_upstream_positions {
			ensure!(
				<T as LocksConfig>::PositionProvider::upstream_position(&owner) ==
					expected_upstream_position,
				"migrated Treasury upstream differs"
			);
		}
		ensure!(
			VaultFundsReleasingByHeight::<T>::iter().collect::<BTreeMap<_, _>>() ==
				expected_release_index,
			"migrated Vault release index differs"
		);
		ensure!(
			pallet_vaults::RevenuePerFrameByVault::<T>::iter()
				.map(|(vault_id, frame_revenue)| (vault_id, frame_revenue.encode()))
				.collect::<Vec<_>>() ==
				source_frame_revenue,
			"migration changed frame revenue"
		);
		Ok(())
	}
}

#[cfg(all(test, not(feature = "runtime-benchmarks")))]
pub(crate) mod tests;
