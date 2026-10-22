#[cfg(feature = "try-runtime")]
use alloc::collections::BTreeMap;
use argon_primitives::{treasury::TreasuryPositionProvider, OperationalAccountsHook};
#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
use frame_support::traits::OnRuntimeUpgrade;
use pallet_bitcoin_fissions::{
	migrations::v1, Config as FissionsConfig, FissionByOwnerAndId, FissionIdsByLockId,
	Pallet as FissionsPallet,
};
use pallet_bitcoin_locks::{
	migrations::v11, Config as LocksConfig, LockIdsByVaultId, LockedBitcoin, LocksById,
	Pallet as LocksPallet, SecuritizationHoldExpirationsByBitcoinHeight,
};
use pallet_prelude::*;
use pallet_vaults::{
	migrations::v19, Config as VaultsConfig, Pallet as VaultsPallet, VaultFundsReleasingByHeight,
	VaultsById,
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
		let vaults_version = StorageVersion::get::<VaultsPallet<T>>();
		let locks_version = StorageVersion::get::<LocksPallet<T>>();
		let fissions_version = StorageVersion::get::<FissionsPallet<T>>();
		if vaults_version != 19 || locks_version != 11 || fissions_version != 1 {
			log::info!(
				"Skipping cosigner guarantee migration: expected Vaults/Locks/Fissions 19/11/1, found {:?}/{:?}/{:?}",
				vaults_version, locks_version, fissions_version,
			);
			return T::DbWeight::get().reads(3);
		}

		let current_height = <T as VaultsConfig>::BitcoinBlockHeightChange::get().1;
		let mut accesses = 3u64;
		VaultsById::<T>::translate::<v19::Vault<T>, _>(|vault_id, old_vault| {
			let mut vault = old_vault.into_current();
			let previous_activation = vault.get_activated_securitization();
			accesses.saturating_accrue(1);

			for lock_id in LockIdsByVaultId::<T>::iter_key_prefix(vault_id) {
				let Some(old) = v11::LocksById::<T>::get(lock_id) else { continue };
				let expiration = old.securitization_hold_expiration_bitcoin_height;
				let mut lock: LockedBitcoin<T> = old.clone().into();
				let original = lock.cosigner_guarantee.clone();
				let old_activation = old.securitization_for_satoshis(lock.funded_satoshis);
				let remaining = &mut lock.cosigner_guarantee;
				let mut assigned_satoshis = 0u64;
				let mut assigned_microgons = <T as VaultsConfig>::Balance::zero();

				// Split Liquid guarantees at the original price, preserving rounding and holds.
				for id in FissionIdsByLockId::<T>::get(lock_id) {
					let Some(fission) = v1::FissionByOwnerAndId::<T>::get(&lock.owner_account, id)
					else {
						continue
					};
					assigned_satoshis.saturating_accrue(fission.satoshis);
					let total = old.securitization_for_satoshis(assigned_satoshis);
					let amount = total.saturating_sub(assigned_microgons);
					assigned_microgons = total;
					let Some(guarantee) = remaining.take_portion(fission.satoshis, amount) else {
						continue
					};
					FissionByOwnerAndId::<T>::insert(
						&lock.owner_account,
						id,
						fission.into_current(guarantee),
					);
					accesses.saturating_accrue(1);
				}

				// Keep the funding window, or shrink the remaining guarantee to confirmed Bitcoin.
				let pending = SecuritizationHoldExpirationsByBitcoinHeight::<T>::get(expiration)
					.contains(&lock_id);
				lock.securitization_hold_expiration_bitcoin_height = pending.then_some(expiration);
				let mut retired = <T as VaultsConfig>::Balance::zero();
				if !pending {
					let retained = old_activation.saturating_sub(assigned_microgons);
					retired = remaining.guaranteed_microgons.saturating_sub(retained);
					remaining.guaranteed_microgons = retained;
					remaining.basis.satoshis = remaining
						.basis
						.satoshis
						.min(lock.funded_satoshis.saturating_sub(lock.fissioned_satoshis));
					let mut to_retain = retained;
					remaining.fund_hold_extensions.retain(|_, amount| {
						*amount = (*amount).min(to_retain);
						to_retain.saturating_reduce(*amount);
						!amount.is_zero()
					});

					// Release the excess at its existing maturities.
					let mut to_skip = original.guaranteed_microgons.saturating_sub(retired);
					for (height, amount) in original.collateral_expirations(
						lock.vault_claim_height,
						original.guaranteed_microgons,
					) {
						let released = amount.saturating_sub(to_skip);
						to_skip.saturating_reduce(amount);
						if released.is_zero() {
							continue
						}
						let Some(entry) = vault.securitization_release_schedule.get_mut(&height)
						else {
							continue
						};
						entry.locked_commitments.saturating_reduce(released);
						if height <= current_height {
							continue
						}
						entry.relockable_commitments.saturating_accrue(released);

						// A full bucket defers the sweep, without changing the maturity.
						let mut sweep_height = height;
						while VaultFundsReleasingByHeight::<T>::mutate(sweep_height, |ids| {
							ids.try_insert(vault_id)
						})
						.is_err()
						{
							sweep_height = sweep_height.saturating_add(1);
							accesses.saturating_accrue(1);
						}
						accesses.saturating_accrue(1);
					}
				}

				// Update Vault and Treasury totals for the split guarantees.
				let activation = lock
					.lock_held_securitization()
					.activated_securitization
					.saturating_add(assigned_microgons);
				vault.securitization_locked.saturating_reduce(retired);
				vault.securitization_pending_activation = vault
					.securitization_pending_activation
					.saturating_add(old_activation)
					.saturating_sub(activation)
					.saturating_sub(retired);
				if lock.is_flexible {
					vault.flexible_securitization_locked = vault
						.flexible_securitization_locked
						.saturating_sub(old_activation)
						.saturating_add(activation);
				}

				if let Some(mut upstream) =
					<T as LocksConfig>::PositionProvider::upstream_position(&lock.owner_account)
						.filter(|position| position.vault_id == vault_id)
				{
					upstream.bitcoin_allocated_securitization.saturating_reduce(retired);
					upstream.bitcoin_securitization = upstream
						.bitcoin_securitization
						.saturating_sub(old_activation)
						.saturating_add(activation);
					<T as LocksConfig>::PositionProvider::set_upstream_position(
						&lock.owner_account,
						Some(upstream),
					);
				}
				LocksById::<T>::insert(lock_id, lock);
				accesses.saturating_accrue(5);
			}

			vault.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
			if previous_activation != vault.get_activated_securitization() {
				<T as VaultsConfig>::OperationalAccountsHook::vault_bitcoin_lock_funded(
					&vault.operator_account_id,
					vault.get_activated_securitization(),
				);
				accesses.saturating_accrue(3);
			}
			Some(vault)
		});

		pallet_bitcoin_locks::migrations::complete_migration::<T>();
		pallet_bitcoin_fissions::migrations::complete_migration::<T>();
		pallet_vaults::migrations::complete_migration::<T>();
		T::DbWeight::get().reads_writes(accesses.saturating_add(3), accesses.saturating_add(1))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let vaults_version = StorageVersion::get::<VaultsPallet<T>>();
		let locks_version = StorageVersion::get::<LocksPallet<T>>();
		let fissions_version = StorageVersion::get::<FissionsPallet<T>>();
		if vaults_version == 20 && locks_version == 12 && fissions_version == 2 {
			return Ok(Vec::new());
		}
		ensure!(vaults_version == 19, "expected runtime-160 Vault storage");
		ensure!(locks_version == 11, "expected runtime-160 Lock storage");
		ensure!(fissions_version == 1, "expected runtime-160 Fission storage");
		ensure!(
			frame_system::LastRuntimeUpgrade::<T>::get().is_some_and(|v| v.spec_version.0 == 160),
			"runtime 160 must precede this upgrade"
		);
		let vaults = v19::VaultsById::<T>::iter()
			.map(|(id, vault)| (id, vault.into_current()))
			.collect::<BTreeMap<_, _>>();
		let indexes = VaultFundsReleasingByHeight::<T>::iter().collect::<BTreeMap<_, _>>();
		ensure!(
			indexes.values().all(|ids| ids.iter().all(|id| vaults.contains_key(id))),
			"release index contains missing Vault"
		);
		let mut totals = BTreeMap::<
			VaultId,
			(
				u64,
				u64,
				<T as VaultsConfig>::Balance,
				<T as VaultsConfig>::Balance,
				<T as VaultsConfig>::Balance,
			),
		>::new();
		let mut upstream = BTreeMap::new();
		let mut upstream_totals = BTreeMap::<
			T::AccountId,
			(<T as VaultsConfig>::Balance, <T as VaultsConfig>::Balance),
		>::new();

		let mut pending = BTreeMap::new();
		for (height, ids) in SecuritizationHoldExpirationsByBitcoinHeight::<T>::iter() {
			for id in ids {
				ensure!(
					pending.insert(id, height).is_none(),
					"Lock has multiple funding expiry entries"
				);
			}
		}

		let mut fissions_by_lock = BTreeMap::<BitcoinLockId, Vec<_>>::new();
		let mut source_fissions = Vec::new();
		for (owner, id, fission) in old_fissions::FissionByOwnerAndId::<T>::iter() {
			source_fissions.push((owner.clone(), id, fission.clone()));
			fissions_by_lock.entry(fission.lock_id).or_default().push((owner, id, fission));
		}

		let mut source_locks = Vec::new();
		for (id, lock) in v11::LocksById::<T>::iter() {
			ensure!(
				lock.securitization_ratio == FixedU128::one(),
				"non-1x Bitcoin Lock requires conversion before runtime 161"
			);
			if let Some(height) = pending.get(&id) {
				ensure!(
					*height == lock.securitization_hold_expiration_bitcoin_height,
					"funding expiry index differs from stored Lock height"
				);
			}
			let vault = vaults.get(&lock.vault_id).ok_or("Lock Vault missing")?;
			ensure!(
				LockIdsByVaultId::<T>::contains_key(lock.vault_id, id),
				"Lock missing from Vault index"
			);
			let guarantee = LockedBitcoin::<T>::from(lock.clone()).cosigner_guarantee;
			for (height, amount) in guarantee
				.collateral_expirations(lock.vault_claim_height, guarantee.guaranteed_microgons)
			{
				if height > <T as VaultsConfig>::BitcoinBlockHeightChange::get().1 {
					ensure!(
						vault
							.securitization_release_schedule
							.get(&height)
							.is_some_and(|entry| entry.locked_commitments >= amount),
						"Lock maturity missing from Vault commitments"
					);
				}
			}
			let activation = lock.securitization_for_satoshis(lock.funded_satoshis);
			let (funded, covered, allocated, active, flexible) =
				totals.entry(lock.vault_id).or_default();
			funded.saturating_accrue(lock.funded_satoshis);
			covered.saturating_accrue(guarantee.securitized_satoshis(lock.funded_satoshis));
			allocated.saturating_accrue(guarantee.guaranteed_microgons);
			active.saturating_accrue(activation);
			if lock.is_flexible {
				flexible.saturating_accrue(activation);
			}
			let position =
				<T as LocksConfig>::PositionProvider::upstream_position(&lock.owner_account);
			if position.as_ref().is_some_and(|p| p.vault_id == lock.vault_id) {
				let (allocated, active) =
					upstream_totals.entry(lock.owner_account.clone()).or_default();
				allocated.saturating_accrue(guarantee.guaranteed_microgons);
				active.saturating_accrue(activation);
			}
			upstream.insert(lock.owner_account.clone(), position);
			let fissions = fissions_by_lock.remove(&id).unwrap_or_default();
			let indexed_ids = FissionIdsByLockId::<T>::get(id);
			ensure!(
				indexed_ids.len() == fissions.len(),
				"Fission index differs from source records"
			);

			let funded = lock.funded_satoshis.min(lock.securitization_basis.satoshis);
			let mut fissioned_satoshis = 0u64;
			for (owner, fission_id, fission) in &fissions {
				ensure!(*owner == lock.owner_account, "Fission owner differs from source Lock");
				ensure!(indexed_ids.contains(fission_id), "Fission missing from Lock index");
				ensure!(fission.satoshis > 0, "Fission has no satoshis");
				fissioned_satoshis = fissioned_satoshis
					.checked_add(fission.satoshis)
					.ok_or("Fission satoshis overflow")?;
				ensure!(fissioned_satoshis <= funded, "Fission exceeds funded Lock guarantee");
			}
			ensure!(
				fissioned_satoshis == lock.fissioned_satoshis,
				"Fission allocation differs from source Lock"
			);

			source_locks.push((id, lock));
		}
		ensure!(fissions_by_lock.is_empty(), "Fission source Lock missing");
		ensure!(
			LockIdsByVaultId::<T>::iter().count() == source_locks.len(),
			"Vault Lock index contains extra records"
		);
		for (id, vault) in &vaults {
			let (funded, covered, allocated, active, flexible) =
				totals.get(id).copied().unwrap_or_default();
			ensure!(
				(
					vault.total_satoshis,
					vault.securitized_satoshis,
					vault.securitization_locked,
					vault.get_activated_securitization(),
					vault.flexible_securitization_locked
				) == (funded, covered, allocated, active, flexible),
				"source Vault accounting differs from Locks"
			);
		}
		for (owner, position) in &upstream {
			if let Some(position) = position {
				let (allocated, active) = upstream_totals.get(owner).copied().unwrap_or_default();
				ensure!(
					(position.bitcoin_allocated_securitization, position.bitcoin_securitization) ==
						(allocated, active),
					"source upstream accounting differs from Locks"
				);
			}
		}
		let vaults = vaults.into_iter().collect::<Vec<_>>();
		let revenue = pallet_vaults::RevenuePerFrameByVault::<T>::iter()
			.map(|(id, r)| (id, r.encode()))
			.collect::<Vec<_>>();
		Ok((source_locks, source_fissions, pending, vaults, upstream, indexes, revenue).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		ensure!(
			StorageVersion::get::<VaultsPallet<T>>() == 20,
			"Vault storage version not updated"
		);
		ensure!(StorageVersion::get::<LocksPallet<T>>() == 12, "Lock storage version not updated");
		ensure!(
			StorageVersion::get::<FissionsPallet<T>>() == 2,
			"Fission storage version not updated"
		);
		// An empty snapshot means all three pallets were already migrated.
		if state.is_empty() {
			return Ok(());
		}

		let (locks, fissions, pending, vaults, mut upstream, mut indexes, revenue) = <(
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
		)>::decode(
			&mut state.as_slice(),
		)
		.map_err(|_| "invalid guarantee migration state")?;
		ensure!(LocksById::<T>::iter_keys().count() == locks.len(), "Bitcoin Lock count changed");
		ensure!(
			FissionByOwnerAndId::<T>::iter_keys().count() == fissions.len(),
			"Fission count changed"
		);
		let mut fissions_by_lock = BTreeMap::<BitcoinLockId, Vec<_>>::new();
		for (owner, id, fission) in fissions {
			fissions_by_lock.entry(fission.lock_id).or_default().push((owner, id, fission));
		}

		let mut expected_vaults = vaults.into_iter().collect::<BTreeMap<_, _>>();
		let mut locks = locks.into_iter().collect::<BTreeMap<_, _>>();
		let lock_ids =
			VaultsById::<T>::iter_keys().flat_map(LockIdsByVaultId::<T>::iter_key_prefix);
		for id in lock_ids {
			let old = locks.remove(&id).ok_or("unexpected migrated Lock")?;
			let lock = LocksById::<T>::get(id).ok_or("Bitcoin Lock missing after migration")?;
			let mut expected_terms: LockedBitcoin<T> = old.clone().into();
			expected_terms.cosigner_guarantee = lock.cosigner_guarantee.clone();
			expected_terms.securitization_hold_expiration_bitcoin_height =
				lock.securitization_hold_expiration_bitcoin_height;
			ensure!(
				lock.encode() == expected_terms.encode(),
				"Bitcoin Lock migration changed stored terms or funding"
			);

			let basis = BitcoinSecuritizationBasis {
				satoshis: old.securitization_basis.satoshis,
				microgons_at_target_per_btc: old.securitization_basis.microgons_at_target_per_btc,
				tick: old.securitization_tick,
			};
			let securitization = CosignerGuarantee {
				basis,
				guaranteed_microgons: old.securitization_coverage_microgons,
				fund_hold_extensions: old.fund_hold_extensions.clone(),
			};
			let mut source_fissions = fissions_by_lock.remove(&id).unwrap_or_default();
			source_fissions.sort_by_key(|(_, fission_id, _)| *fission_id);
			let mut guarantees = Vec::new();
			for (owner, fission_id, source) in source_fissions {
				let fission = FissionByOwnerAndId::<T>::get(owner, fission_id)
					.ok_or("Fission missing after migration")?;
				let satoshis = source.satoshis;
				ensure!(
					source.into_current(fission.cosigner_guarantee.clone()).encode() ==
						fission.encode(),
					"Fission migration changed stored terms"
				);
				guarantees.push((satoshis, fission.cosigner_guarantee));
			}
			let position = lock.total_securitization(id);
			ensure!(
				position ==
					BitcoinLockPosition {
						allocated_securitization: if pending.contains_key(&id) {
							old.securitization_coverage_microgons
						} else {
							old.securitization_for_satoshis(old.funded_satoshis)
						},
						activated_securitization: lock
							.lock_held_securitization()
							.activated_securitization
							.checked_add(&old.securitization_for_satoshis(old.fissioned_satoshis))
							.ok_or("migrated activated securitization overflows")?,
					},
				"migrated securitization does not match retained guarantees"
			);

			let capacity = if pending.contains_key(&id) {
				basis.satoshis
			} else {
				old.funded_satoshis.min(basis.satoshis)
			};
			let satoshis = capacity
				.checked_sub(old.fissioned_satoshis)
				.ok_or("Fissions exceed the Lock guarantee")?;
			ensure!(
				lock.cosigner_guarantee.basis.satoshis == satoshis,
				"Lock guarantee quantity changed"
			);
			ensure!(
				lock.securitization_hold_expiration_bitcoin_height == pending.get(&id).copied(),
				"initial funding deadline changed"
			);
			if let Some(deadline) = lock.securitization_hold_expiration_bitcoin_height {
				ensure!(
					SecuritizationHoldExpirationsByBitcoinHeight::<T>::get(deadline).contains(&id),
					"initial funding expiry not indexed"
				);
			}
			guarantees.push((satoshis, lock.cosigner_guarantee.clone()));

			let allocated = guarantees
				.iter()
				.fold(<T as VaultsConfig>::Balance::zero(), |total, (_, guarantee)| {
					total.saturating_add(guarantee.guaranteed_microgons)
				});
			let active = position.activated_securitization;
			let old_active = old.securitization_for_satoshis(old.funded_satoshis);
			let retired = old.securitization_coverage_microgons.saturating_sub(allocated);
			let vault = expected_vaults.get_mut(&old.vault_id).ok_or("Lock Vault missing")?;
			vault.securitization_locked.saturating_reduce(retired);
			vault.securitization_pending_activation = vault
				.securitization_pending_activation
				.saturating_add(old_active)
				.saturating_sub(active)
				.saturating_sub(retired);
			if old.is_flexible {
				vault.flexible_securitization_locked = vault
					.flexible_securitization_locked
					.saturating_sub(old_active)
					.saturating_add(active);
			}
			if let Some(Some(position)) = upstream.get_mut(&old.owner_account) {
				if position.vault_id == old.vault_id {
					position.bitcoin_allocated_securitization.saturating_reduce(retired);
					position.bitcoin_securitization = position
						.bitcoin_securitization
						.saturating_sub(old_active)
						.saturating_add(active);
				}
			}
			let mut owned_maturities =
				BTreeMap::<BitcoinHeight, <T as VaultsConfig>::Balance>::new();
			for (_, guarantee) in &guarantees {
				for (height, amount) in guarantee
					.collateral_expirations(old.vault_claim_height, guarantee.guaranteed_microgons)
				{
					owned_maturities.entry(height).or_default().saturating_accrue(amount);
				}
			}
			for (height, amount) in securitization
				.collateral_expirations(old.vault_claim_height, securitization.guaranteed_microgons)
			{
				let released =
					amount.saturating_sub(owned_maturities.remove(&height).unwrap_or_default());
				if released.is_zero() {
					continue
				}
				if let Some(entry) = vault.securitization_release_schedule.get_mut(&height) {
					entry.locked_commitments.saturating_reduce(released);
					if height > <T as VaultsConfig>::BitcoinBlockHeightChange::get().1 {
						entry.relockable_commitments.saturating_accrue(released);
						let mut sweep_height = height;
						while indexes
							.entry(sweep_height)
							.or_default()
							.try_insert(old.vault_id)
							.is_err()
						{
							sweep_height =
								sweep_height.checked_add(1).ok_or("release index overflows")?;
						}
					}
				}
			}

			// Verify each assigned guarantee amount and its hold extensions against the original
			// guarantee.
			let mut allocated_satoshis = 0;
			for (satoshis, guarantee) in guarantees {
				let allocated_microgons = old.securitization_for_satoshis(allocated_satoshis);
				allocated_satoshis =
					allocated_satoshis.checked_add(satoshis).ok_or("assigned satoshis overflow")?;
				let microgons = old
					.securitization_for_satoshis(allocated_satoshis)
					.checked_sub(&allocated_microgons)
					.ok_or("assigned securitization underflows")?;
				ensure!(
					guarantee.basis == BitcoinSecuritizationBasis { satoshis, ..basis } &&
						guarantee.guaranteed_microgons == microgons,
					"Assigned guarantee basis or securitization changed"
				);

				let mut extensions = old.fund_hold_extensions.clone();
				let mut microgons_to_skip = allocated_microgons;
				let mut microgons_to_assign = microgons;
				extensions.retain(|_, amount| {
					let assigned =
						amount.saturating_sub(microgons_to_skip).min(microgons_to_assign);
					microgons_to_skip = microgons_to_skip.saturating_sub(*amount);
					microgons_to_assign.saturating_reduce(assigned);
					*amount = assigned;
					!assigned.is_zero()
				});
				ensure!(
					guarantee.fund_hold_extensions == extensions,
					"Assigned guarantee release maturities changed"
				);
			}
		}
		ensure!(
			VaultsById::<T>::iter_keys().count() == expected_vaults.len(),
			"Vault count changed"
		);
		for (id, mut expected) in expected_vaults {
			expected.securitization_release_schedule.retain(|_, entry| !entry.is_empty());
			ensure!(
				VaultsById::<T>::get(id) == Some(expected),
				"migrated Vault accounting or maturities differ"
			);
		}
		for (owner, expected) in upstream {
			ensure!(
				<T as LocksConfig>::PositionProvider::upstream_position(&owner) == expected,
				"migrated Treasury upstream differs"
			);
		}
		ensure!(
			VaultFundsReleasingByHeight::<T>::iter().collect::<BTreeMap<_, _>>() == indexes,
			"migrated Vault release index differs"
		);
		ensure!(
			pallet_vaults::RevenuePerFrameByVault::<T>::iter()
				.map(|(id, r)| (id, r.encode()))
				.collect::<Vec<_>>() ==
				revenue,
			"migration changed frame revenue"
		);
		Ok(())
	}
}

#[cfg(all(test, not(feature = "runtime-benchmarks")))]
mod tests;
