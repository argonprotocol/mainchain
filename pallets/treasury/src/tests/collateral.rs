use super::*;

#[test]
fn liquidate_bond_lot_rejects_when_it_would_drop_below_encumbered_backing() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 3, None));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 2, None));
		let bond_lot_ids = account_bond_lot_ids(2);
		assert_ok!(<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
			&account(2),
			4 * MICROGONS_PER_ARGON,
			None,
		));

		assert_err!(
			Treasury::liquidate_bond_lot(origin(2), bond_lot_ids[0]),
			Error::<Test>::ActiveBondAmountBelowEncumberedBacking,
		);
		assert_eq!(BondLotsByVault::<Test>::get(1).regular_bonds, 5);
		assert_eq!(
			BondLotById::<Test>::get(bond_lot_ids[0]).expect("bond lot").release_reason,
			None,
		);
	});
}

#[test]
fn burn_encumbered_bond_microgons_releases_fractional_slack_when_whole_bonds_still_cover_backing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
		assert_ok!(<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
			&account(2),
			5 * MICROGONS_PER_ARGON,
			None,
		));
		assert_ok!(Treasury::settle_bond_collateral_burn(
			&account(2),
			(3 * MICROGONS_PER_ARGON) / 2,
			&mut Weight::zero()
		));

		let bond_lot_id = account_bond_lot_ids(2)[0];
		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("bond lot");
		assert_eq!(bond_lot.bonds, 8);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2),),
			8 * MICROGONS_PER_ARGON,
		);
		assert_eq!(Treasury::encumbered_bond_microgons(&account(2)), (7 * MICROGONS_PER_ARGON) / 2,);
		System::assert_has_event(
			Event::<Test>::EncumberedBondMicrogonsBurned {
				account_id: account(2),
				burned_amount: (3 * MICROGONS_PER_ARGON) / 2,
				released_amount: MICROGONS_PER_ARGON / 2,
			}
			.into(),
		);
	});
}

#[test]
fn partial_and_multi_lot_burns_preserve_reserved_bond_space() {
	for (lots, burns) in [(vec![10], vec![5, 5]), (vec![5, 5], vec![10])] {
		new_test_ext().execute_with(|| {
			MinimumArgonsPerContributor::set(1);
			insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
			set_argons(10, 10 * MICROGONS_PER_ARGON);
			for bonds in lots {
				assert_ok!(Treasury::buy_bonds(origin(10), 1, bonds, None));
				let lot = account_bond_lot_ids(10)
					.into_iter()
					.find(|id| !BondLotById::<Test>::get(id).unwrap().is_flexible)
					.unwrap();
				assert_ok!(Treasury::set_bond_lot_flexible(origin(10), lot, true));
			}
			assert_ok!(Treasury::set_reserved_bond_space(origin(10), 1, 10));
			assert_ok!(Treasury::encumber_bond_microgons(
				&account(10),
				10 * MICROGONS_PER_ARGON,
				None,
			));

			let mut remaining = 10;
			for bonds in burns {
				assert_ok!(Treasury::settle_bond_collateral_burn(
					&account(10),
					u128::from(bonds) * MICROGONS_PER_ARGON,
					&mut Weight::zero()
				));
				remaining -= bonds;
				let vault = BondLotsByVault::<Test>::get(1);
				assert_eq!(vault.flexible_bonds, remaining);
				assert_eq!(vault.reserved_bond_space, 10);
				let live_bonds: Bonds = account_bond_lot_ids(10)
					.into_iter()
					.map(|id| BondLotById::<Test>::get(id).unwrap().bonds)
					.sum();
				assert_eq!(live_bonds, remaining);
			}
			assert!(account_bond_lot_ids(10).is_empty());
		});
	}
}

#[test]
fn burn_encumbered_bond_microgons_keeps_fractional_remainder_held_until_it_is_released() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, None));
		assert_ok!(<Treasury as TreasuryPoolProvider<TestAccountId>>::encumber_bond_microgons(
			&account(2),
			5 * MICROGONS_PER_ARGON,
			None,
		));
		assert_ok!(Treasury::settle_bond_collateral_burn(
			&account(2),
			(9 * MICROGONS_PER_ARGON) / 2,
			&mut Weight::zero()
		));

		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.regular_bonds, 0);
		assert_eq!(vault_bonds.flexible_bonds, 0);
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2),),
			MICROGONS_PER_ARGON / 2,
		);
		assert_eq!(Treasury::encumbered_bond_microgons(&account(2)), MICROGONS_PER_ARGON / 2);

		assert_err!(
			Treasury::settle_bond_collateral_release(
				&account(2),
				(MICROGONS_PER_ARGON / 2) + 1,
				&mut Weight::zero()
			),
			Error::<Test>::ActiveBondAmountBelowEncumberedBacking,
		);
		assert_ok!(Treasury::settle_bond_collateral_release(
			&account(2),
			MICROGONS_PER_ARGON / 2,
			&mut Weight::zero()
		));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2),),
			0,
		);
		assert_eq!(Treasury::encumbered_bond_microgons(&account(2)), 0);
	});
}

#[test]
fn failed_bond_burn_rolls_back_in_the_callers_storage_layer() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(1, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 100 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
		assert_ok!(Treasury::encumber_bond_microgons(&account(2), 10 * MICROGONS_PER_ARGON, None,));

		// Make the aggregate inconsistent so replacement fails after the held balance is burned.
		PositionsByAccount::<Test>::mutate(account(2), |position| {
			position.as_mut().unwrap().bond_principal = 0;
		});
		assert_noop!(
			with_storage_layer(|| {
				Treasury::settle_bond_collateral_burn(
					&account(2),
					MICROGONS_PER_ARGON,
					&mut Weight::zero(),
				)
			}),
			ArithmeticError::Underflow
		);
		assert_eq!(BondLotById::<Test>::get(account_bond_lot_ids(2)[0]).unwrap().bonds, 10);
		assert_eq!(Treasury::encumbered_bond_microgons(&account(2)), 10 * MICROGONS_PER_ARGON);
	});
}

#[test]
fn collateral_requests_combine_without_releasing_or_double_committing_the_hold() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(1, 100 * MICROGONS_PER_ARGON as u64));
		set_argons(2, 100 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));
		let owner = account(2);
		assert_ok!(Treasury::encumber_bond_microgons(&owner, 10 * MICROGONS_PER_ARGON, None,));
		let issuance = Balances::total_issuance();

		assert_ok!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::burn_encumbered_bond_microgons(
				&owner,
				3 * MICROGONS_PER_ARGON,
			)
		);
		assert_ok!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::release_encumbered_bond_microgons(
				&owner,
				4 * MICROGONS_PER_ARGON,
			)
		);
		assert_eq!(BondCollateralSettlementQueue::<Test>::iter_keys().count(), 1);
		assert_eq!(Treasury::encumbered_bond_microgons(&owner), 10 * MICROGONS_PER_ARGON);
		assert_eq!(Balances::total_issuance(), issuance);
		assert_noop!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::release_encumbered_bond_microgons(
				&owner,
				4 * MICROGONS_PER_ARGON,
			),
			Error::<Test>::ActiveBondAmountBelowEncumberedBacking,
		);
		assert_noop!(
			Treasury::liquidate_bond_lot(origin(2), account_bond_lot_ids(2)[0]),
			Error::<Test>::ActiveBondAmountBelowEncumberedBacking,
		);

		Treasury::on_initialize(System::block_number() + 1);
		assert!(!PendingBondCollateralByAccount::<Test>::contains_key(&owner));
		assert_eq!(BondCollateralSettlementQueue::<Test>::iter_keys().count(), 0);
		assert_eq!(Treasury::encumbered_bond_microgons(&owner), 3 * MICROGONS_PER_ARGON);
		assert_eq!(TreasuryPositions::bond_principal(&owner), 7 * MICROGONS_PER_ARGON);
		assert_eq!(Balances::total_issuance(), issuance - 3 * MICROGONS_PER_ARGON);
	});
}

#[test]
fn pending_burn_keeps_the_next_frames_terms_until_it_is_processed() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(0);
		insert_vault(1, test_vault(1, 100 * MICROGONS_PER_ARGON as u64));
		set_argons(2, 200 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 100, None));
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		let owner = account(2);
		assert_ok!(Treasury::encumber_bond_microgons(&owner, 40 * MICROGONS_PER_ARGON, None,));
		assert_ok!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::burn_encumbered_bond_microgons(
				&owner,
				40 * MICROGONS_PER_ARGON,
			)
		);

		CurrentFrameId::set(2);
		Treasury::run_frame_transition(2);
		assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().total_active_bonds, 100);
		Treasury::on_initialize(System::block_number() + 1);
		let lot = BondLotById::<Test>::get(account_bond_lot_ids(2)[0]).unwrap();
		assert_eq!(lot.bonds, 60);
		assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().total_active_bonds, 100);

		CurrentFrameId::set(3);
		Treasury::run_frame_transition(3);
		assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().total_active_bonds, 60);
	});
}

#[test]
fn failed_collateral_settlement_rolls_back_and_retries_without_blocking_other_owners() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		crate::mock::MaxBondCollateralSettlementsPerBlock::set(1);
		insert_vault(1, test_vault(1, 100 * MICROGONS_PER_ARGON as u64));
		for id in [2, 3] {
			set_argons(id, 100 * MICROGONS_PER_ARGON);
			assert_ok!(Treasury::buy_bonds(origin(id), 1, 10, None));
			assert_ok!(Treasury::encumber_bond_microgons(
				&account(id),
				10 * MICROGONS_PER_ARGON,
				None,
			));
			assert_ok!(
				<Treasury as TreasuryPoolProvider<TestAccountId>>::burn_encumbered_bond_microgons(
					&account(id),
					5 * MICROGONS_PER_ARGON,
				)
			);
		}
		let first = BondCollateralSettlementQueue::<Test>::iter_keys().next().unwrap().holder_id();
		let other = if first == account(2) { account(3) } else { account(2) };
		PositionsByAccount::<Test>::mutate(&first, |position| {
			position.as_mut().unwrap().bond_principal = 0;
		});
		let issuance = Balances::total_issuance();
		let now = System::block_number() + 1;

		Treasury::on_initialize(now);
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(Treasury::encumbered_bond_microgons(&first), 10 * MICROGONS_PER_ARGON);
		assert!(PendingBondCollateralByAccount::<Test>::contains_key(&first));
		Treasury::on_initialize(now + 1);
		assert!(!PendingBondCollateralByAccount::<Test>::contains_key(&other));
		assert_eq!(TreasuryPositions::bond_principal(&other), 5 * MICROGONS_PER_ARGON);

		PositionsByAccount::<Test>::mutate(&first, |position| {
			position.as_mut().unwrap().bond_principal = 10 * MICROGONS_PER_ARGON;
		});
		Treasury::on_initialize(now + 2);
		assert_eq!(BondCollateralSettlementQueue::<Test>::iter_keys().count(), 0);
		assert_eq!(TreasuryPositions::bond_principal(&first), 5 * MICROGONS_PER_ARGON);
		assert_eq!(Balances::total_issuance(), issuance - 10 * MICROGONS_PER_ARGON);
	});
}

#[test]
fn ten_reserved_lots_can_be_topped_up_and_fully_burned() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 100 * MICROGONS_PER_ARGON);
		TotalArgonBondLots::<Test>::put(15_000);
		for _ in 0..11 {
			assert_ok!(Treasury::buy_bonds(origin(2), 1, 1, None));
		}
		let owner = account(2);
		assert_noop!(
			Treasury::encumber_bond_microgons(&owner, 11 * MICROGONS_PER_ARGON, None,),
			Error::<Test>::ActiveBondAmountBelowEncumberedBacking
		);
		assert_eq!(crate::EncumberedBondLotsByAccount::<Test>::get(&owner).len(), 0);
		let available =
			crate::AvailableBondCollateralLots::<Test>::iter_key_prefix(&owner).collect::<Vec<_>>();
		let unselected = available[0];
		for id in &available[1..] {
			assert_ok!(Treasury::encumber_bond_microgons(
				&owner,
				MICROGONS_PER_ARGON / 2,
				Some(&[*id])
			));
		}
		assert_noop!(
			Treasury::encumber_bond_microgons(&owner, 1, Some(&[unselected])),
			Error::<Test>::TooManyBondCollateralLots
		);
		assert_ok!(Treasury::encumber_bond_microgons(&owner, 5 * MICROGONS_PER_ARGON, None));
		assert_noop!(
			Treasury::encumber_bond_microgons(&owner, MICROGONS_PER_ARGON, None),
			Error::<Test>::ActiveBondAmountBelowEncumberedBacking
		);
		assert_eq!(crate::EncumberedBondLotsByAccount::<Test>::get(&owner).len(), 10);
		assert_ok!(Treasury::burn_encumbered_bond_microgons(&owner, 10 * MICROGONS_PER_ARGON));

		Treasury::on_initialize(System::block_number() + 1);
		assert!(!PendingBondCollateralByAccount::<Test>::contains_key(&owner));
		assert_eq!(Treasury::encumbered_bond_microgons(&owner), 0);
		assert_eq!(BondLotIdsByAccount::<Test>::iter_prefix(&owner).count(), 1);
		assert_ok!(Treasury::try_state(1));
		Treasury::on_initialize(System::block_number() + 2);
		assert!(!PendingBondCollateralByAccount::<Test>::contains_key(&owner));
		assert_eq!(Treasury::encumbered_bond_microgons(&owner), 0);
		assert_ok!(Treasury::try_state(1));
	});
}

#[test]
fn explicit_collateral_ids_choose_backing_and_reject_invalid_or_duplicate_lots() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 100 * MICROGONS_PER_ARGON);
		set_argons(3, 100 * MICROGONS_PER_ARGON);
		for _ in 0..11 {
			assert_ok!(Treasury::buy_bonds(origin(2), 1, 1, None));
		}
		assert_ok!(Treasury::buy_bonds(origin(3), 1, 1, None));
		let owner = account(2);
		let selected = crate::AvailableBondCollateralLots::<Test>::iter_key_prefix(&owner)
			.nth(10)
			.unwrap();
		let other_owner = account_bond_lot_ids(3)[0];
		assert_noop!(
			Treasury::encumber_bond_microgons(&owner, 1, Some(&[selected, selected])),
			Error::<Test>::DuplicateBondCollateralLot
		);
		assert_noop!(
			Treasury::encumber_bond_microgons(&owner, 1, Some(&[other_owner])),
			Error::<Test>::NotBondLotOwner
		);
		assert_noop!(
			Treasury::encumber_bond_microgons(&owner, 1, Some(&[u64::MAX])),
			Error::<Test>::BondLotNotFound
		);
		let too_many = (0..11).collect::<Vec<_>>();
		assert_noop!(
			Treasury::encumber_bond_microgons(&owner, 1, Some(&too_many)),
			Error::<Test>::TooManyBondCollateralLots
		);
		assert_ok!(Treasury::encumber_bond_microgons(
			&owner,
			MICROGONS_PER_ARGON,
			Some(&[selected])
		));
		assert_eq!(
			crate::EncumberedBondLotsByAccount::<Test>::get(&owner).as_slice(),
			&[crate::BondCollateralLot {
				bond_lot_id: Some(selected),
				reserved_microgons: MICROGONS_PER_ARGON,
			}]
		);
		assert_ok!(Treasury::burn_encumbered_bond_microgons(&owner, MICROGONS_PER_ARGON / 2));
		assert_ok!(Treasury::release_encumbered_bond_microgons(&owner, MICROGONS_PER_ARGON / 2));
		Treasury::on_initialize(System::block_number() + 1);
		assert!(!BondLotById::<Test>::contains_key(selected));
		assert!(crate::EncumberedBondLotsByAccount::<Test>::get(&owner).is_empty());
		assert_eq!(Treasury::encumbered_bond_microgons(&owner), 0);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &owner),
			10 * MICROGONS_PER_ARGON
		);
		assert_ok!(Treasury::try_state(2));
	});
}
