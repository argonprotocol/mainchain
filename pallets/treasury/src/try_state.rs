use crate::{
	BondCollateralSettlementQueue, BondLotById, BondLotIdsByAccount, Config,
	EncumberedBondMicrogonsByAccount, PendingBondCollateralByAccount,
};
use alloc::collections::{BTreeMap, BTreeSet};
use pallet_prelude::*;
use sp_runtime::TryRuntimeError;

pub(crate) fn check<T: Config>() -> Result<(), TryRuntimeError> {
	let mut lots_by_owner = BTreeMap::<T::AccountId, BTreeSet<_>>::new();
	for (lot_id, lot) in BondLotById::<T>::iter() {
		let ids = lots_by_owner.entry(lot.owner).or_default();
		ids.insert(lot_id);
		ensure!(
			ids.len() <= T::MaxExpectedBondLotsPerAccount::get() as usize,
			"Owner bond/stake lot count exceeds scan benchmark coverage"
		);
	}

	for (owner, lot_id, ()) in BondLotIdsByAccount::<T>::iter() {
		let ids = lots_by_owner
			.get_mut(&owner)
			.ok_or("Bond lot owner index contains an unknown owner")?;
		ensure!(ids.remove(&lot_id), "Bond lot owner index disagrees with its records");
	}
	ensure!(lots_by_owner.values().all(BTreeSet::is_empty), "Bond lot is missing its owner index");

	let mut queued_accounts = BTreeSet::new();
	for key in BondCollateralSettlementQueue::<T>::iter_keys() {
		ensure!(queued_accounts.insert(key.holder_id()), "Duplicate bond collateral queue entry");
	}
	for (account, pending) in PendingBondCollateralByAccount::<T>::iter() {
		ensure!(
			queued_accounts.remove(&account),
			"Pending bond collateral is missing its queue entry"
		);
		let total = pending
			.burn_microgons
			.checked_add(&pending.release_microgons)
			.ok_or("Pending bond collateral overflows")?;
		ensure!(!total.is_zero(), "Empty bond collateral settlement remains queued");
		ensure!(
			total <= EncumberedBondMicrogonsByAccount::<T>::get(&account),
			"Pending bond collateral exceeds its encumbrance"
		);
	}
	ensure!(queued_accounts.is_empty(), "Bond collateral queue contains an unknown account");
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{
		account_id_from_seed as account, insert_vault, new_test_ext, set_argons, set_ownership,
		test_vault, CurrentFrameId, MaxArgonotBondedPercentOfCirculation,
		MaxExpectedBondLotsPerAccount, MinimumArgonsPerContributor, RuntimeOrigin, Test,
		TestAccountId, Treasury,
	};
	use argon_primitives::{TreasuryPoolProvider, MICROGONS_PER_ARGON};
	use frame_support::{assert_ok, traits::Hooks};

	#[test]
	fn pending_collateral_requires_one_queue_entry_and_sufficient_encumbrance() {
		new_test_ext().execute_with(|| {
			MinimumArgonsPerContributor::set(1);
			insert_vault(1, test_vault(1, 100 * MICROGONS_PER_ARGON as u64));
			set_argons(2, 100 * MICROGONS_PER_ARGON);
			assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(account(2)), 1, 10, None));
			assert_ok!(Treasury::encumber_bond_microgons(&account(2), 10 * MICROGONS_PER_ARGON));
			assert_ok!(Treasury::burn_encumbered_bond_microgons(
				&account(2),
				4 * MICROGONS_PER_ARGON
			));
			assert_ok!(check::<Test>());

			let key = BondCollateralSettlementQueue::<Test>::iter_keys().next().unwrap();
			BondCollateralSettlementQueue::<Test>::remove(&key);
			assert!(check::<Test>().is_err());
			BondCollateralSettlementQueue::<Test>::insert(key, ());
			PendingBondCollateralByAccount::<Test>::mutate(account(2), |pending| {
				pending.as_mut().unwrap().burn_microgons = 11 * MICROGONS_PER_ARGON;
			});
			assert!(check::<Test>().is_err());
		});
	}

	#[test]
	fn owner_scan_expectation_counts_both_assets_and_pending_releases_without_capping_purchases() {
		new_test_ext().execute_with(|| {
			MinimumArgonsPerContributor::set(1);
			MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));
			CurrentFrameId::set(1);
			insert_vault(1, test_vault(10, (200 * MICROGONS_PER_ARGON) as u64));
			set_argons(2, 200 * MICROGONS_PER_ARGON);
			set_ownership(2, 200 * MICROGONS_PER_ARGON);

			assert_eq!(MaxExpectedBondLotsPerAccount::get(), 100);
			for _ in 0..50 {
				assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(account(2)), 1, 1, None));
				assert_ok!(Treasury::buy_argonot_bonds(RuntimeOrigin::signed(account(2)), 1));
			}
			assert_ok!(Treasury::try_state(1));

			assert_ok!(Treasury::liquidate_bond_lot(RuntimeOrigin::signed(account(2)), 0));
			assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(account(2)), 1, 7, None));
			assert_eq!(BondLotIdsByAccount::<Test>::iter_key_prefix(account(2)).count(), 101);
			assert!(Treasury::try_state(1).is_err());
			assert_eq!(
				<Treasury as TreasuryPoolProvider<TestAccountId>>::active_vault_bond_amount(
					1,
					&account(2),
				),
				56 * MICROGONS_PER_ARGON,
			);

			Treasury::release_pending_bond_lots(11);
			assert_eq!(BondLotIdsByAccount::<Test>::iter_key_prefix(account(2)).count(), 100);
			assert_ok!(Treasury::try_state(1));
		});
	}

	#[test]
	fn owner_scan_state_check_rejects_missing_and_incorrect_index_entries() {
		new_test_ext().execute_with(|| {
			MinimumArgonsPerContributor::set(1);
			insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
			set_argons(2, 10 * MICROGONS_PER_ARGON);
			assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(account(2)), 1, 1, None));
			assert_ok!(Treasury::try_state(1));

			BondLotIdsByAccount::<Test>::remove(account(2), 0);
			assert!(Treasury::try_state(1).is_err());
			BondLotIdsByAccount::<Test>::insert(account(2), 0, ());
			BondLotIdsByAccount::<Test>::insert(account(3), 0, ());
			assert!(Treasury::try_state(1).is_err());
			BondLotIdsByAccount::<Test>::remove(account(3), 0);
			BondLotIdsByAccount::<Test>::insert(account(2), 1, ());
			assert!(Treasury::try_state(1).is_err());
		});
	}
}
