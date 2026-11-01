use crate::{pallet::STORAGE_VERSION, *};

pub(crate) fn check<T: Config>() -> Result<(), sp_runtime::TryRuntimeError> {
	ensure!(
		StorageVersion::get::<Pallet<T>>() == STORAGE_VERSION,
		"Bitcoin Locks storage must be migrated before checking its state"
	);

	for (lock_id, lock) in LocksById::<T>::iter() {
		ensure!(
			LockIdsByOwnerAccount::<T>::contains_key(&lock.owner_account, lock_id),
			"Bitcoin Lock is missing its owner index"
		);
		ensure!(
			LockIdsByVaultId::<T>::contains_key(lock.vault_id, lock_id),
			"Bitcoin Lock is missing its vault index"
		);

		let funded_satoshis = lock
			.funding_utxos
			.values()
			.try_fold(0u64, |sum, amount| sum.checked_add(*amount))
			.ok_or("Bitcoin Lock funding UTXO sum overflows")?;
		ensure!(
			funded_satoshis == lock.funded_satoshis,
			"Bitcoin Lock funding UTXOs disagree with its funded satoshis"
		);
		ensure!(
			lock.fissioned_satoshis <= lock.funded_satoshis,
			"Bitcoin Lock Fissions exceed its funded satoshis"
		);
	}

	for (owner, lock_id, ()) in LockIdsByOwnerAccount::<T>::iter() {
		let lock = LocksById::<T>::get(lock_id)
			.ok_or("Bitcoin Lock owner index references a missing Lock")?;
		ensure!(lock.owner_account == owner, "Bitcoin Lock owner index references the wrong owner");
	}
	for (vault_id, lock_id, ()) in LockIdsByVaultId::<T>::iter() {
		let lock = LocksById::<T>::get(lock_id)
			.ok_or("Bitcoin Lock vault index references a missing Lock")?;
		ensure!(lock.vault_id == vault_id, "Bitcoin Lock vault index references the wrong vault");
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use argon_primitives::bitcoin::{CompressedBitcoinPubkey, SATOSHIS_PER_BITCOIN};
	use frame_support::traits::Hooks;

	#[test]
	fn try_state_detects_missing_and_stale_lock_indexes() {
		set_bitcoin_height(12);
		new_test_ext().execute_with(|| {
			<BitcoinLocks as frame_support::traits::OnGenesis>::on_genesis();
			set_argons(2, 2_000_000);
			assert_ok!(BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				CompressedBitcoinPubkey([1; 33]),
				None,
			));
			assert_ok!(BitcoinLocks::try_state(1));

			LockIdsByOwnerAccount::<Test>::remove(2, 1);
			assert_eq!(
				BitcoinLocks::try_state(1),
				Err(DispatchError::Other("Bitcoin Lock is missing its owner index")),
			);
			LockIdsByOwnerAccount::<Test>::insert(2, 1, ());
			LockIdsByVaultId::<Test>::insert(2, 1, ());
			assert_eq!(
				BitcoinLocks::try_state(1),
				Err(DispatchError::Other("Bitcoin Lock vault index references the wrong vault")),
			);
		});
	}

	#[test]
	fn try_state_reconciles_funded_and_fissioned_satoshis() {
		set_bitcoin_height(12);
		new_test_ext().execute_with(|| {
			<BitcoinLocks as frame_support::traits::OnGenesis>::on_genesis();
			set_argons(2, 2_000_000);
			assert_ok!(BitcoinLocks::create_receive_address(
				RuntimeOrigin::signed(2),
				1,
				SATOSHIS_PER_BITCOIN,
				CompressedBitcoinPubkey([1; 33]),
				None,
			));
			assert_ok!(funding_received(1, SATOSHIS_PER_BITCOIN));
			assert_ok!(BitcoinLocks::try_state(1));

			LocksById::<Test>::mutate(1, |lock| {
				lock.as_mut().unwrap().funded_satoshis += 1;
			});
			assert_eq!(
				BitcoinLocks::try_state(1),
				Err(DispatchError::Other(
					"Bitcoin Lock funding UTXOs disagree with its funded satoshis"
				)),
			);
			LocksById::<Test>::mutate(1, |lock| {
				let lock = lock.as_mut().unwrap();
				lock.funded_satoshis = SATOSHIS_PER_BITCOIN;
				lock.fissioned_satoshis = SATOSHIS_PER_BITCOIN + 1;
			});
			assert_eq!(
				BitcoinLocks::try_state(1),
				Err(DispatchError::Other("Bitcoin Lock Fissions exceed its funded satoshis")),
			);
		});
	}
}
