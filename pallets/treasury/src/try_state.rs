use crate::*;
use alloc::collections::{BTreeMap, BTreeSet};
use sp_runtime::TryRuntimeError;

pub(crate) fn check<T: Config>() -> Result<(), TryRuntimeError> {
	let mut lots_by_owner = BTreeMap::<T::AccountId, BTreeSet<_>>::new();
	for (lot_id, lot) in BondLotById::<T>::iter() {
		let ids = lots_by_owner.entry(lot.owner).or_default();
		ids.insert(lot_id);
	}

	for (owner, lot_id, ()) in BondLotIdsByAccount::<T>::iter() {
		let ids = lots_by_owner
			.get_mut(&owner)
			.ok_or("Bond lot owner index contains an unknown owner")?;
		ensure!(ids.remove(&lot_id), "Bond lot owner index disagrees with its records");
	}
	ensure!(lots_by_owner.values().all(BTreeSet::is_empty), "Bond lot is missing its owner index");

	let mut principal = BTreeMap::new();
	let mut active_by_owner = BTreeMap::<T::AccountId, T::Balance>::new();
	let mut holds = BTreeMap::<T::AccountId, T::Balance>::new();
	let mut available = BTreeSet::new();
	for (id, lot) in BondLotById::<T>::iter() {
		let BondProgram::Vault { vault_id, .. } = lot.program else { continue };
		let amount: T::Balance = u128::from(lot.bonds)
			.saturating_mul(argon_primitives::MICROGONS_PER_ARGON)
			.into();
		let required = holds.entry(lot.owner.clone()).or_default();
		*required = required.checked_add(&amount).ok_or("Bond hold totals overflow")?;
		if lot.release_reason.is_some() {
			continue
		}
		let active = active_by_owner.entry(lot.owner.clone()).or_default();
		*active = active.checked_add(&amount).ok_or("Active bond principal overflows")?;
		let total = principal.entry((lot.owner.clone(), vault_id)).or_insert(0u128);
		*total = total.checked_add(lot.bonds.into()).ok_or("Bond principal totals overflow")?;
		if amount >
			EncumberedBondLotsByAccount::<T>::get(&lot.owner)
				.iter()
				.find(|entry| entry.bond_lot_id == Some(id))
				.map(|entry| entry.reserved_microgons)
				.unwrap_or_default()
		{
			available.insert((lot.owner, id));
		}
	}
	for (owner, vault, amount) in ActiveBondsByAccountAndVault::<T>::iter() {
		ensure!(
			principal.remove(&(owner, vault)) == Some(amount),
			"Active bond principal disagrees with lots"
		);
	}
	ensure!(principal.is_empty(), "Active bonds are missing their principal total");
	for (owner, id, ()) in AvailableBondCollateralLots::<T>::iter() {
		ensure!(available.remove(&(owner, id)), "Available collateral index disagrees with lots");
	}
	ensure!(available.is_empty(), "Unreserved bond principal is missing its collateral index");

	let mut reserved = BTreeMap::<T::AccountId, T::Balance>::new();
	for (owner, backing) in EncumberedBondLotsByAccount::<T>::iter() {
		let mut ids = BTreeSet::new();
		for entry in backing {
			let id = entry.bond_lot_id;
			let amount = entry.reserved_microgons;
			ensure!(ids.insert(id), "Duplicate collateral lot");
			ensure!(!amount.is_zero(), "Empty collateral reservation");
			let lot = id.and_then(BondLotById::<T>::get);
			let mut principal = T::Balance::zero();
			if let Some(lot) = lot {
				ensure!(lot.owner == owner, "Collateral lot belongs to another account");
				ensure!(lot.release_reason.is_none(), "Releasing lot remains encumbered");
				ensure!(
					matches!(lot.program, BondProgram::Vault { .. }),
					"ARGONOT lot used as ARGON backing"
				);
				principal = u128::from(lot.bonds)
					.saturating_mul(argon_primitives::MICROGONS_PER_ARGON)
					.into();
			}
			let required = holds.entry(owner.clone()).or_default();
			*required = required
				.checked_add(&amount.saturating_sub(principal))
				.ok_or("Collateral holds overflow")?;
			let total = reserved.entry(owner.clone()).or_default();
			*total = total.checked_add(&amount).ok_or("Collateral reservations overflow")?;
		}
	}

	for (owner, amount) in EncumberedBondMicrogonsByAccount::<T>::iter() {
		let selected = reserved.remove(&owner).unwrap_or_default();
		ensure!(selected <= amount, "Selected lots exceed account encumbrance");
		if selected < amount {
			let backing = EncumberedBondLotsByAccount::<T>::get(&owner);
			ensure!(
				backing.iter().all(|entry| entry.bond_lot_id.is_none()),
				"Partial concrete selection disagrees with account encumbrance"
			);
			let active = active_by_owner.get(&owner).copied().unwrap_or_default();
			let covered = active
				.checked_add(&selected)
				.ok_or("Unassigned collateral coverage overflows")?;
			ensure!(
				amount <= covered,
				"Unassigned collateral exceeds active principal and fractional backing"
			);
		}
	}
	ensure!(reserved.is_empty(), "Collateral reservations have no account total");
	for (owner, required) in holds {
		ensure!(
			T::Currency::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &owner) >=
				required,
			"Native hold does not cover bonds and exact collateral"
		);
	}

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
		MinimumArgonsPerContributor, RuntimeOrigin, Test, TestAccountId, Treasury,
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
			assert_ok!(Treasury::encumber_bond_microgons(
				&account(2),
				10 * MICROGONS_PER_ARGON,
				None,
			));
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
	fn many_bond_and_stake_lots_keep_indexes_consistent_through_liquidation() {
		new_test_ext().execute_with(|| {
			MinimumArgonsPerContributor::set(1);
			MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));
			CurrentFrameId::set(1);
			insert_vault(1, test_vault(10, (200 * MICROGONS_PER_ARGON) as u64));
			set_argons(2, 200 * MICROGONS_PER_ARGON);
			set_ownership(2, 200 * MICROGONS_PER_ARGON);

			for _ in 0..50 {
				assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(account(2)), 1, 1, None));
				assert_ok!(Treasury::buy_argonot_bonds(RuntimeOrigin::signed(account(2)), 1));
			}
			assert_ok!(Treasury::try_state(1));

			assert_ok!(Treasury::liquidate_bond_lot(RuntimeOrigin::signed(account(2)), 0));
			assert_ok!(Treasury::buy_bonds(RuntimeOrigin::signed(account(2)), 1, 7, None));
			assert_eq!(BondLotIdsByAccount::<Test>::iter_key_prefix(account(2)).count(), 101);
			assert_ok!(Treasury::try_state(1));
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
