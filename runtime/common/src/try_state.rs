use alloc::collections::BTreeMap;
use argon_primitives::{
	bitcoin::{BitcoinHeight, BitcoinLockId, Satoshis},
	treasury::BitcoinLockPosition,
	Balance, VaultId,
};
use pallet_prelude::*;
use polkadot_sdk::frame_support::{traits::GetStorageVersion, StorageNoopGuard};

/// Reconcile canonical Bitcoin records across pallets after their local state checks.
/// Pending releases and failed expirations retain their Lock backing until settlement succeeds.
pub fn check<T>() -> Result<(), sp_runtime::TryRuntimeError>
where
	T: pallet_bitcoin_locks::Config<Balance = Balance>
		+ pallet_bitcoin_fissions::Config<Balance = Balance>
		+ pallet_vaults::Config<Balance = Balance>
		+ pallet_treasury_positions::Config<Balance = Balance>,
{
	let _guard = StorageNoopGuard::default();
	ensure!(
		StorageVersion::get::<pallet_bitcoin_locks::Pallet<T>>() ==
			pallet_bitcoin_locks::Pallet::<T>::in_code_storage_version(),
		"Bitcoin backing checks require current Lock storage"
	);
	ensure!(
		StorageVersion::get::<pallet_bitcoin_fissions::Pallet<T>>() ==
			pallet_bitcoin_fissions::Pallet::<T>::in_code_storage_version(),
		"Bitcoin backing checks require current Fission storage"
	);
	ensure!(
		StorageVersion::get::<pallet_vaults::Pallet<T>>() ==
			pallet_vaults::Pallet::<T>::in_code_storage_version(),
		"Bitcoin backing checks require current Vault storage"
	);
	ensure!(
		StorageVersion::get::<pallet_treasury_positions::Pallet<T>>() ==
			pallet_treasury_positions::Pallet::<T>::in_code_storage_version(),
		"Bitcoin backing checks require current Treasury Positions storage"
	);

	let mut fissions_by_lock = BTreeMap::<BitcoinLockId, Vec<_>>::new();
	for (owner, _, fission) in pallet_bitcoin_fissions::FissionByOwnerAndId::<T>::iter() {
		fissions_by_lock.entry(fission.lock_id).or_default().push((owner, fission));
	}

	let bitcoin_height = <T as pallet_vaults::Config>::BitcoinBlockHeightChange::get().1;
	let mut backing_by_vault = BTreeMap::<VaultId, VaultBacking>::new();
	for (lock_id, lock) in pallet_bitcoin_locks::LocksById::<T>::iter() {
		let available_satoshis = lock
			.funded_satoshis
			.checked_sub(lock.fissioned_satoshis)
			.ok_or("Lock Fissions exceed its funded satoshis")?;
		let mut securitization = lock.lock_held_securitization();
		let mut covered_satoshis = lock.cosigner_guarantee.securitized_satoshis(available_satoshis);
		let mut fissioned_satoshis = 0u64;
		let fissions = fissions_by_lock.remove(&lock_id).unwrap_or_default();
		let backing = backing_by_vault.entry(lock.vault_id).or_default();
		for guarantee in core::iter::once(&lock.cosigner_guarantee)
			.chain(fissions.iter().map(|(_, fission)| &fission.cosigner_guarantee))
		{
			for (height, amount) in guarantee
				.collateral_expirations(lock.vault_claim_height, guarantee.guaranteed_microgons)
			{
				// Matured commitments can already have been swept while the Lock remains live.
				if height <= bitcoin_height {
					continue;
				}
				let committed = backing.release_commitments.entry(height).or_default();
				*committed =
					committed.checked_add(amount).ok_or("Vault guarantee commitments overflow")?;
			}
		}
		for (owner, fission) in fissions {
			ensure!(owner == lock.owner_account, "Fission owner differs from its source Lock");
			fissioned_satoshis = fissioned_satoshis
				.checked_add(fission.satoshis)
				.ok_or("Lock Fission satoshis overflow")?;
			covered_satoshis = covered_satoshis
				.checked_add(fission.cosigner_guarantee.securitized_satoshis(fission.satoshis))
				.ok_or("Lock covered satoshis overflow")?;
			// Migrated promises can exceed their apportioned guarantee. Reconcile the actual
			// guarantee rather than recomputing liability from today's redemption settings.
			securitization.allocated_securitization = securitization
				.allocated_securitization
				.checked_add(fission.cosigner_guarantee.guaranteed_microgons)
				.ok_or("Lock allocated securitization overflows")?;
			securitization.activated_securitization = securitization
				.activated_securitization
				.checked_add(fission.cosigner_guarantee.guaranteed_microgons)
				.ok_or("Lock activated securitization overflows")?;
		}
		ensure!(
			fissioned_satoshis == lock.fissioned_satoshis,
			"Lock Fission allocation disagrees with active Fissions"
		);

		backing.funded_satoshis = backing
			.funded_satoshis
			.checked_add(lock.funded_satoshis)
			.ok_or("Vault funded satoshis overflow")?;
		backing.securitized_satoshis = backing
			.securitized_satoshis
			.checked_add(covered_satoshis)
			.ok_or("Vault covered satoshis overflow")?;
		backing.securitization.allocated_securitization = backing
			.securitization
			.allocated_securitization
			.checked_add(securitization.allocated_securitization)
			.ok_or("Vault allocated securitization overflows")?;
		backing.securitization.activated_securitization = backing
			.securitization
			.activated_securitization
			.checked_add(securitization.activated_securitization)
			.ok_or("Vault activated securitization overflows")?;
		if lock.is_flexible {
			backing.flexible_securitization = backing
				.flexible_securitization
				.checked_add(securitization.activated_securitization)
				.ok_or("Vault flexible securitization overflows")?;
		}
	}
	ensure!(fissions_by_lock.is_empty(), "Active Fission references a missing Lock");

	let mut network_covered_satoshis = 0u128;
	for (vault_id, vault) in pallet_vaults::VaultsById::<T>::iter() {
		let mut backing = backing_by_vault.remove(&vault_id).unwrap_or_default();
		ensure!(
			vault.total_satoshis == backing.funded_satoshis,
			"Vault funded satoshis disagree with its Locks"
		);
		ensure!(
			vault.securitized_satoshis == backing.securitized_satoshis,
			"Vault covered satoshis disagree with its Locks and Fissions"
		);
		ensure!(
			vault.securitization_locked == backing.securitization.allocated_securitization,
			"Vault allocated securitization disagrees with its Locks and Fissions"
		);
		ensure!(
			vault.securitization_locked.checked_sub(vault.securitization_pending_activation) ==
				Some(backing.securitization.activated_securitization),
			"Vault activated securitization disagrees with its Locks and Fissions"
		);
		ensure!(
			vault.flexible_securitization_locked == backing.flexible_securitization,
			"Vault flexible securitization disagrees with its Locks and Fissions"
		);
		for (height, entry) in &vault.securitization_release_schedule {
			if *height <= bitcoin_height {
				continue;
			}
			ensure!(
				entry.locked_commitments ==
					backing.release_commitments.remove(height).unwrap_or_default(),
				"Vault release commitments disagree with its Lock and Fission guarantees"
			);
		}
		ensure!(
			backing.release_commitments.is_empty(),
			"Vault is missing a guarantee release commitment"
		);
		network_covered_satoshis = network_covered_satoshis
			.checked_add(u128::from(backing.securitized_satoshis))
			.ok_or("Network covered satoshis overflow")?;
	}
	ensure!(backing_by_vault.is_empty(), "Bitcoin Lock references a missing Vault");
	ensure!(
		pallet_treasury_positions::NetworkTotals::<T>::get().vault_securitized_satoshis ==
			network_covered_satoshis,
		"Treasury network Bitcoin coverage disagrees with Vault backing"
	);
	Ok(())
}

#[derive(Default)]
struct VaultBacking {
	funded_satoshis: Satoshis,
	securitized_satoshis: Satoshis,
	securitization: BitcoinLockPosition<Balance>,
	flexible_securitization: Balance,
	release_commitments: BTreeMap<BitcoinHeight, Balance>,
}

#[cfg(all(test, not(feature = "runtime-benchmarks")))]
mod tests {
	use super::*;
	use crate::migrations::tests::{new_test_ext, source_state};
	use argon_primitives::treasury::PositionQuantities;
	use argon_runtime::{BitcoinFissions, Runtime};
	use frame_support::{assert_ok, traits::Hooks};
	use pallet_bitcoin_fissions::{FissionByOwnerAndId, NextFissionIdByOwner};
	use pallet_bitcoin_locks::LocksById;
	use pallet_treasury_positions::{NetworkTotals, PositionsByAccount};
	use pallet_vaults::VaultsById;
	use polkadot_sdk::sp_io;
	use sp_runtime::AccountId32;

	const LOCK_OWNER: AccountId32 = AccountId32::new([2; 32]);

	fn migrate_backing() {
		source_state();
		let quantities = PositionQuantities { fission_liquidity: 6, ..Default::default() };
		PositionsByAccount::<Runtime>::mutate(LOCK_OWNER, |position| {
			position.as_mut().unwrap().quantities = quantities;
		});
		sp_io::storage::set(&NetworkTotals::<Runtime>::hashed_key(), &quantities.encode());
		StorageVersion::new(1).put::<pallet_treasury_positions::Pallet<Runtime>>();
		NextFissionIdByOwner::<Runtime>::insert(LOCK_OWNER, 3);
		assert_ok!(argon_runtime::Executive::try_runtime_upgrade(
			polkadot_sdk::frame_try_runtime::UpgradeCheckSelect::PreAndPost,
		));
	}

	#[test]
	fn reconciles_real_migrated_backing_with_promises_above_paid_guarantees() {
		new_test_ext().execute_with(|| {
			migrate_backing();
			let fission = FissionByOwnerAndId::<Runtime>::get(LOCK_OWNER, 0).unwrap();
			assert_eq!(fission.cosigner_guarantee.guaranteed_microgons, 1);
			assert_eq!(fission.liquidity_promised, 2);
			assert_ok!(BitcoinFissions::try_state(1));
			assert_ok!(check::<Runtime>());
		});
	}

	#[test]
	fn detects_wrong_fission_owners_and_missing_source_locks() {
		new_test_ext().execute_with(|| {
			migrate_backing();
			let other_owner = AccountId32::new([8; 32]);
			let fission = FissionByOwnerAndId::<Runtime>::take(LOCK_OWNER, 0).unwrap();
			FissionByOwnerAndId::<Runtime>::insert(&other_owner, 0, fission.clone());
			assert_eq!(
				check::<Runtime>(),
				Err("Fission owner differs from its source Lock".into())
			);

			FissionByOwnerAndId::<Runtime>::remove(other_owner, 0);
			FissionByOwnerAndId::<Runtime>::insert(LOCK_OWNER, 0, fission);
			FissionByOwnerAndId::<Runtime>::mutate(LOCK_OWNER, 0, |fission| {
				fission.as_mut().unwrap().lock_id = 99;
			});
			// Keep the remaining source allocation consistent so the orphan is the failing
			// boundary.
			LocksById::<Runtime>::mutate(1, |lock| {
				lock.as_mut().unwrap().fissioned_satoshis -= 2;
			});
			assert_eq!(check::<Runtime>(), Err("Active Fission references a missing Lock".into()));
		});
	}

	#[test]
	fn detects_lock_allocation_drift() {
		new_test_ext().execute_with(|| {
			migrate_backing();
			LocksById::<Runtime>::mutate(1, |lock| {
				lock.as_mut().unwrap().fissioned_satoshis += 1;
			});
			assert_eq!(
				check::<Runtime>(),
				Err("Lock Fission allocation disagrees with active Fissions".into())
			);
		});
	}

	#[test]
	fn detects_vault_backing_drift() {
		for field in 0..5 {
			new_test_ext().execute_with(|| {
				migrate_backing();
				let error = match field {
					0 => "Vault funded satoshis disagree with its Locks",
					1 => "Vault covered satoshis disagree with its Locks and Fissions",
					2 => "Vault allocated securitization disagrees with its Locks and Fissions",
					3 => "Vault activated securitization disagrees with its Locks and Fissions",
					_ => "Vault flexible securitization disagrees with its Locks and Fissions",
				};
				VaultsById::<Runtime>::mutate(1, |vault| {
					let vault = vault.as_mut().unwrap();
					match field {
						0 => vault.total_satoshis += 1,
						1 => vault.securitized_satoshis += 1,
						2 => vault.securitization_locked += 1,
						3 => vault.securitization_pending_activation += 1,
						_ => vault.flexible_securitization_locked += 1,
					}
				});
				assert_eq!(check::<Runtime>(), Err(error.into()));
			});
		}
	}

	#[test]
	fn detects_network_bitcoin_coverage_drift() {
		new_test_ext().execute_with(|| {
			migrate_backing();
			NetworkTotals::<Runtime>::mutate(|totals| totals.vault_securitized_satoshis += 1);
			assert_eq!(
				check::<Runtime>(),
				Err("Treasury network Bitcoin coverage disagrees with Vault backing".into())
			);
		});
	}

	#[test]
	fn detects_guarantee_release_commitment_drift() {
		new_test_ext().execute_with(|| {
			migrate_backing();
			VaultsById::<Runtime>::mutate(1, |vault| {
				vault
					.as_mut()
					.unwrap()
					.securitization_release_schedule
					.get_mut(&576)
					.unwrap()
					.locked_commitments += 1;
			});
			assert_eq!(
				check::<Runtime>(),
				Err("Vault release commitments disagree with its Lock and Fission guarantees"
					.into())
			);
		});
	}
}
