use crate::{
	pallet::STORAGE_VERSION, Config, FissionByOwnerAndId, FissionIdsByLockId, NextFissionIdByOwner,
	Pallet,
};
use alloc::collections::{BTreeMap, BTreeSet};
use argon_primitives::{bitcoin::BitcoinLockId, treasury::TreasuryPositionProvider};
use pallet_prelude::*;

pub(crate) fn check<T: Config>() -> Result<(), sp_runtime::TryRuntimeError> {
	ensure!(
		StorageVersion::get::<Pallet<T>>() == STORAGE_VERSION,
		"Bitcoin Fissions storage must be migrated before checking its state"
	);

	let mut ids_by_lock = BTreeMap::<BitcoinLockId, BTreeSet<_>>::new();
	let mut liquidity_by_owner = BTreeMap::<T::AccountId, T::Balance>::new();
	let mut total_liquidity = T::Balance::zero();
	for (owner, fission_id, fission) in FissionByOwnerAndId::<T>::iter() {
		ensure!(fission.satoshis > 0, "Active Fission has no satoshis");
		ensure!(
			fission.cosigner_guarantee.basis.satoshis <= fission.satoshis,
			"Fission guarantee covers more satoshis than its allocation"
		);
		let mut held_microgons = T::Balance::zero();
		for amount in fission.cosigner_guarantee.fund_hold_extensions.values() {
			ensure!(!amount.is_zero(), "Fission guarantee contains an empty release hold");
			held_microgons = held_microgons
				.checked_add(amount)
				.ok_or("Fission guarantee release holds overflow")?;
		}
		ensure!(
			held_microgons <= fission.cosigner_guarantee.guaranteed_microgons,
			"Fission release holds exceed its paid guarantee"
		);
		ensure!(
			fission_id < NextFissionIdByOwner::<T>::get(&owner),
			"Active Fission ID is not below its owner's next ID"
		);
		ensure!(
			ids_by_lock.entry(fission.lock_id).or_default().insert(fission_id),
			"Source Lock has duplicate owner-scoped Fission IDs"
		);
		let liquidity = liquidity_by_owner.entry(owner).or_default();
		*liquidity = liquidity
			.checked_add(&fission.liquidity_promised)
			.ok_or("Account Fission liquidity overflows")?;
		total_liquidity = total_liquidity
			.checked_add(&fission.liquidity_promised)
			.ok_or("Network Fission liquidity overflows")?;
	}

	for (lock_id, ids) in FissionIdsByLockId::<T>::iter() {
		let expected = ids_by_lock.remove(&lock_id).unwrap_or_default();
		ensure!(!ids.is_empty(), "Empty Fission Lock index remains in storage");
		ensure!(
			ids.iter().copied().eq(expected.into_iter()),
			"Fission Lock index disagrees with its active records"
		);
	}
	ensure!(ids_by_lock.is_empty(), "Active Fission is missing its Lock index");

	for (owner, liquidity) in liquidity_by_owner {
		ensure!(
			T::PositionProvider::account_quantities(&owner).fission_liquidity == liquidity,
			"Account Fission liquidity disagrees with Treasury Positions"
		);
	}
	ensure!(
		T::PositionProvider::network_totals().account_quantities.fission_liquidity ==
			total_liquidity,
		"Network Fission liquidity disagrees with active Fissions"
	);
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use frame_support::traits::Hooks;
	use pallet_treasury_positions::{NetworkTotals, PositionsByAccount};

	#[test]
	fn accepts_owner_scoped_ids_and_closed_fission_mint_entitlements() {
		new_test_ext().execute_with(|| {
			STORAGE_VERSION.put::<BitcoinFissions>();
			MockLocks::insert(1, lock(1, 100));
			MockLocks::insert(2, lock(2, 100));
			assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 20, 90, None));
			assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(2), 0, 77, 2, 20, 90, None));
			assert_ok!(BitcoinFissions::try_state(1));

			assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
			assert_eq!(MockMintRequests::get().len(), 2);
			assert_ok!(BitcoinFissions::try_state(1));
		});
	}

	#[test]
	fn rejects_missing_and_stale_lock_indexes_and_reused_owner_ids() {
		new_test_ext().execute_with(|| {
			STORAGE_VERSION.put::<BitcoinFissions>();
			MockLocks::insert(1, lock(1, 100));
			assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 20, 90, None));
			let ids = FissionIdsByLockId::<Test>::take(1);
			assert_eq!(
				BitcoinFissions::try_state(1),
				Err(DispatchError::Other("Active Fission is missing its Lock index"))
			);
			FissionIdsByLockId::<Test>::insert(1, ids);
			FissionIdsByLockId::<Test>::mutate(1, |ids| {
				ids.try_insert(1).unwrap();
			});
			assert_eq!(
				BitcoinFissions::try_state(1),
				Err(DispatchError::Other("Fission Lock index disagrees with its active records"))
			);
			FissionIdsByLockId::<Test>::mutate(1, |ids| {
				ids.remove(&1);
			});
			NextFissionIdByOwner::<Test>::insert(1, 0);
			assert_eq!(
				BitcoinFissions::try_state(1),
				Err(DispatchError::Other("Active Fission ID is not below its owner's next ID"))
			);
		});
	}

	#[test]
	fn rejects_account_and_network_liquidity_drift() {
		new_test_ext().execute_with(|| {
			STORAGE_VERSION.put::<BitcoinFissions>();
			MockLocks::insert(1, lock(1, 100));
			assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 20, 90, None));
			NetworkTotals::<Test>::mutate(|totals| {
				totals.account_quantities.fission_liquidity += 1
			});
			assert_eq!(
				BitcoinFissions::try_state(1),
				Err(DispatchError::Other(
					"Network Fission liquidity disagrees with active Fissions"
				))
			);
			NetworkTotals::<Test>::mutate(|totals| {
				totals.account_quantities.fission_liquidity -= 1
			});
			PositionsByAccount::<Test>::mutate(1, |position| {
				position.as_mut().unwrap().quantities.fission_liquidity += 1;
			});
			assert_eq!(
				BitcoinFissions::try_state(1),
				Err(DispatchError::Other(
					"Account Fission liquidity disagrees with Treasury Positions"
				))
			);
		});
	}

	#[test]
	fn checks_guarantee_coverage_and_owned_release_holds() {
		new_test_ext().execute_with(|| {
			STORAGE_VERSION.put::<BitcoinFissions>();
			MockLocks::insert(1, lock(1, 100));
			assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 20, 90, None));
			FissionByOwnerAndId::<Test>::mutate(1, 0, |fission| {
				let guarantee = &mut fission.as_mut().unwrap().cosigner_guarantee;
				guarantee
					.fund_hold_extensions
					.try_insert(144, guarantee.guaranteed_microgons)
					.unwrap();
			});
			assert_ok!(BitcoinFissions::try_state(1));

			FissionByOwnerAndId::<Test>::mutate(1, 0, |fission| {
				fission
					.as_mut()
					.unwrap()
					.cosigner_guarantee
					.fund_hold_extensions
					.try_insert(288, 1)
					.unwrap();
			});
			assert_eq!(
				BitcoinFissions::try_state(1),
				Err("Fission release holds exceed its paid guarantee".into())
			);

			FissionByOwnerAndId::<Test>::mutate(1, 0, |fission| {
				fission
					.as_mut()
					.unwrap()
					.cosigner_guarantee
					.fund_hold_extensions
					.try_insert(288, 0)
					.unwrap();
			});
			assert_eq!(
				BitcoinFissions::try_state(1),
				Err("Fission guarantee contains an empty release hold".into())
			);

			FissionByOwnerAndId::<Test>::mutate(1, 0, |fission| {
				let fission = fission.as_mut().unwrap();
				fission.cosigner_guarantee.fund_hold_extensions.remove(&288);
				fission.cosigner_guarantee.basis.satoshis = fission.satoshis + 1;
			});
			assert_eq!(
				BitcoinFissions::try_state(1),
				Err("Fission guarantee covers more satoshis than its allocation".into())
			);
		});
	}

	#[test]
	fn rejects_old_storage_before_decoding_current_records() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(1).put::<BitcoinFissions>();
			frame_support::storage::unhashed::put_raw(
				&FissionByOwnerAndId::<Test>::hashed_key_for(1, 0),
				&[0xff],
			);
			assert_eq!(
				BitcoinFissions::try_state(1),
				Err(DispatchError::Other(
					"Bitcoin Fissions storage must be migrated before checking its state"
				))
			);
		});
	}
}
