use super::*;

#[test]
fn root_backfills_existing_vault_lot_metrics_without_moving_funds() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 3, None));
		let bond_lot_id = account_bond_lot_ids(2)[0];
		CurrentFrameId::set(10);
		let initial = BondLotEarningsMetrics {
			participated_frames: 0,
			last_frame_earnings_frame_id: None,
			last_frame_earnings: None,
			cumulative_earnings: 0,
		};
		let backfilled = BondLotEarningsMetrics {
			participated_frames: 2,
			last_frame_earnings_frame_id: Some(3),
			last_frame_earnings: Some(200_000),
			cumulative_earnings: 500_000,
		};
		let free_balance = Balances::free_balance(account(2));
		let total_issuance = Balances::total_issuance();
		let held_balance =
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2));

		assert_noop!(
			Treasury::backfill_bond_lot_earnings(
				origin(2),
				bond_lot_id,
				initial.clone(),
				backfilled.clone(),
			),
			sp_runtime::DispatchError::BadOrigin,
		);
		assert_ok!(Treasury::backfill_bond_lot_earnings(
			RuntimeOrigin::root(),
			bond_lot_id,
			initial.clone(),
			backfilled.clone(),
		));
		let lot = BondLotById::<Test>::get(bond_lot_id).unwrap();
		assert_eq!(lot.participated_frames, backfilled.participated_frames);
		assert_eq!(lot.last_frame_earnings_frame_id, backfilled.last_frame_earnings_frame_id);
		assert_eq!(lot.last_frame_earnings, backfilled.last_frame_earnings);
		assert_eq!(lot.cumulative_earnings, backfilled.cumulative_earnings);
		assert_eq!(Balances::free_balance(account(2)), free_balance);
		assert_eq!(Balances::total_issuance(), total_issuance);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2),),
			held_balance,
		);
		let event_count = System::events().len();
		assert_ok!(Treasury::backfill_bond_lot_earnings(
			RuntimeOrigin::root(),
			bond_lot_id,
			initial.clone(),
			backfilled.clone(),
		));
		assert_eq!(System::events().len(), event_count);
		assert_noop!(
			Treasury::backfill_bond_lot_earnings(
				RuntimeOrigin::root(),
				bond_lot_id,
				backfilled.clone(),
				BondLotEarningsMetrics { cumulative_earnings: 400_000, ..backfilled.clone() },
			),
			Error::<Test>::InvalidBondLotMetrics,
		);
		assert_noop!(
			Treasury::backfill_bond_lot_earnings(
				RuntimeOrigin::root(),
				bond_lot_id,
				backfilled.clone(),
				BondLotEarningsMetrics {
					last_frame_earnings_frame_id: Some(11),
					..backfilled.clone()
				},
			),
			Error::<Test>::InvalidBondLotMetrics,
		);
		assert_noop!(
			Treasury::backfill_bond_lot_earnings(
				RuntimeOrigin::root(),
				bond_lot_id,
				initial,
				BondLotEarningsMetrics { cumulative_earnings: 600_000, ..backfilled },
			),
			Error::<Test>::BondLotMetricsChanged,
		);
	});
}

#[test]
fn buy_bonds_uses_raw_vault_securitization() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);

		let mut securitization_funded_vault = test_vault(10, (4 * MICROGONS_PER_ARGON) as u64);
		securitization_funded_vault.securitization = 6 * MICROGONS_PER_ARGON;
		insert_vault(1, securitization_funded_vault);

		let mut bitcoin_funded_vault = test_vault(11, (6 * MICROGONS_PER_ARGON) as u64);
		bitcoin_funded_vault.securitization = 4 * MICROGONS_PER_ARGON;
		insert_vault(2, bitcoin_funded_vault);

		set_argons(2, 7 * MICROGONS_PER_ARGON);
		set_argons(3, 7 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 6, None));
		assert_eq!(VaultsById::get().get(&1).unwrap().committed_microgons, 6 * MICROGONS_PER_ARGON);
		assert_noop!(
			Treasury::buy_bonds(origin(2), 1, 1, None),
			Error::<Test>::InsufficientBondSpace
		);

		assert_ok!(Treasury::buy_bonds(origin(3), 2, 4, None));
		assert_eq!(VaultsById::get().get(&2).unwrap().committed_microgons, 4 * MICROGONS_PER_ARGON);
		assert_noop!(
			Treasury::buy_bonds(origin(3), 2, 1, None),
			Error::<Test>::InsufficientBondSpace
		);

		Treasury::lock_in_vault_capital(1);

		let frame = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(frame.total_active_bonds, 10);
		assert_eq!(
			frame
				.vault_securitization_positions
				.get(&1)
				.map(|vault| vault.active_bond_microgons),
			Some(6 * MICROGONS_PER_ARGON)
		);
		assert_eq!(
			frame
				.vault_securitization_positions
				.get(&2)
				.map(|vault| vault.active_bond_microgons),
			Some(4 * MICROGONS_PER_ARGON)
		);
	});
}

#[test]
fn bond_purchases_cannot_reuse_securitization_under_exit_notice() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 6 * MICROGONS_PER_ARGON);
		set_argons(2, 5 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 6, None));
		let flexible_lot_id = account_bond_lot_ids(10)[0];
		assert_eq!(VaultsById::get().get(&1).unwrap().committed_microgons, 6 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), flexible_lot_id, true));
		VaultsById::mutate(|vaults| {
			vaults.get_mut(&1).unwrap().exit_notice_amount = 6 * MICROGONS_PER_ARGON;
		});

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 4, None));
		assert_noop!(
			Treasury::buy_bonds(origin(2), 1, 1, None),
			Error::<Test>::InsufficientBondSpace,
		);
		assert_noop!(
			Treasury::set_bond_lot_flexible(origin(10), flexible_lot_id, false),
			Error::<Test>::InsufficientBondSpace,
		);
		assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, 0);
	});
}

#[test]
fn buy_bonds_store_plain_and_bonus_terms_and_track_pool_participation() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);

		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, None));
		assert_ok!(Treasury::buy_bonds(
			origin(3),
			1,
			5,
			Some(bonus_approval(1, 3, Permill::from_percent(15), 1, 0, 1)),
		));

		let plain_bond_lot_ids = account_bond_lot_ids(2);
		assert_eq!(plain_bond_lot_ids.len(), 1);

		let bonus_bond_lot_ids = account_bond_lot_ids(3);
		assert_eq!(bonus_bond_lot_ids.len(), 1);

		let plain_bond_lot = BondLotById::<Test>::get(plain_bond_lot_ids[0]).expect("bond lot");
		assert_eq!(plain_bond_lot.owner, account(2));
		assert_eq!(
			plain_bond_lot.program,
			BondProgram::Vault {
				vault_id: 1,
				sharing_percent: Permill::zero(),
				bonus_percent: Permill::zero(),
			},
		);
		assert_eq!(plain_bond_lot.bonds, 5);
		assert_eq!(plain_bond_lot.created_frame_id, 1);

		let bonus_bond_lot = BondLotById::<Test>::get(bonus_bond_lot_ids[0]).expect("bond lot");
		assert_eq!(bonus_bond_lot.owner, account(3));
		assert_eq!(
			bonus_bond_lot.program,
			BondProgram::Vault {
				vault_id: 1,
				sharing_percent: Permill::zero(),
				bonus_percent: Permill::from_percent(15),
			},
		);
		assert_eq!(bonus_bond_lot.bonds, 5);
		assert_eq!(bonus_bond_lot.created_frame_id, 1);

		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2)),
			5 * MICROGONS_PER_ARGON,
		);
		assert_eq!(BondLotsByVault::<Test>::get(1).regular_bonds, 10);
		assert!(<Treasury as TreasuryPoolProvider<TestAccountId>>::has_vault_bond_participation(
			1,
			&account(2),
		));
		assert!(<Treasury as TreasuryPoolProvider<TestAccountId>>::has_vault_bond_participation(
			1,
			&account(3),
		));
	});
}

#[test]
fn bond_purchase_unreserves_reserved_bond_space_from_approval() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		let mut vault = test_vault(10, (10 * MICROGONS_PER_ARGON) as u64);
		vault.delegate_account_id = Some(account(3));
		insert_vault(1, vault);
		set_argons(2, 10 * MICROGONS_PER_ARGON);

		assert_noop!(
			Treasury::set_reserved_bond_space(origin(2), 1, 10),
			Error::<Test>::NoPermissions
		);
		assert_noop!(
			Treasury::set_reserved_bond_space(origin(3), 1, 10),
			Error::<Test>::NoPermissions
		);
		assert_noop!(
			Treasury::set_reserved_bond_space(origin(10), 1, 11),
			Error::<Test>::InsufficientBondSpace
		);
		assert_ok!(Treasury::set_reserved_bond_space(origin(10), 1, 10));

		assert_noop!(
			Treasury::buy_bonds(origin(2), 1, 10, None),
			Error::<Test>::InsufficientBondSpace
		);
		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			6,
			Some(bonus_approval(1, 2, Permill::from_percent(15), 1, 10, 1)),
		));
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.reserved_bond_space, 0);
		assert_eq!(vault_bonds.regular_bonds, 6);
	});
}

#[test]
fn bonus_approval_rejects_wrong_vault_account_expiry_and_signature() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);

		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			5,
			Some(bonus_approval(1, 2, Permill::from_percent(15), 1, 0, 1)),
		));

		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(2, 2, Permill::from_percent(15), 1, 0, 2)),
			),
			Error::<Test>::BonusApprovalWrongVault,
		);
		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(1, 3, Permill::from_percent(15), 1, 0, 2)),
			),
			Error::<Test>::BonusApprovalWrongAccount,
		);
		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(1, 2, Permill::from_percent(15), 0, 0, 2)),
			),
			Error::<Test>::BonusApprovalExpired,
		);

		let mut invalid_signature = bonus_approval(1, 3, Permill::from_percent(15), 1, 0, 2);
		invalid_signature.signature = Signature::Sr25519([1; 64].into());
		assert_err!(
			Treasury::buy_bonds(origin(3), 1, 5, Some(invalid_signature)),
			Error::<Test>::InvalidBonusApprovalSignature,
		);
	});
}

#[test]
fn bonus_approval_rejects_fields_changed_after_signing() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		let mut approval = bonus_approval(1, 2, Permill::from_percent(10), 1, 0, 1);
		approval.bonus_percent = Permill::from_percent(15);

		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 5, Some(approval)),
			Error::<Test>::InvalidBonusApprovalSignature,
		);

		let mut approval = bonus_approval(1, 2, Permill::from_percent(10), 1, 0, 1);
		approval.nonce = 2;

		assert_err!(
			Treasury::buy_bonds(origin(2), 1, 5, Some(approval)),
			Error::<Test>::InvalidBonusApprovalSignature,
		);
	});
}

#[test]
fn bonus_approval_does_not_require_vault_wide_profit_sharing() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);
		set_argons(2, 20 * MICROGONS_PER_ARGON);

		let approval = bonus_approval(1, 2, Permill::from_percent(11), 1, 0, 1);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, Some(approval)));
		let lot_id = account_bond_lot_ids(2)[0];
		assert_eq!(
			BondLotById::<Test>::get(lot_id).unwrap().program,
			BondProgram::Vault {
				vault_id: 1,
				sharing_percent: Permill::zero(),
				bonus_percent: Permill::from_percent(11),
			},
		);
	});
}

#[test]
fn bonus_approval_allows_multiple_coupons_and_rejects_reuse() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);

		let vault = test_vault(10, (100 * MICROGONS_PER_ARGON) as u64);
		insert_vault(1, vault);

		set_argons(2, 20 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			5,
			Some(bonus_approval(1, 2, Permill::from_percent(15), 2, 0, 1)),
		));
		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			5,
			Some(bonus_approval(1, 2, Permill::from_percent(10), 2, 0, 2)),
		));
		assert_eq!(account_bond_lot_ids(2).len(), 2);

		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(1, 2, Permill::from_percent(10), 2, 0, 2)),
			),
			Error::<Test>::BonusApprovalAlreadyUsed,
		);
		assert_err!(
			Treasury::buy_bonds(
				origin(2),
				1,
				5,
				Some(bonus_approval(1, 2, Permill::from_percent(10), 2, 0, 1)),
			),
			Error::<Test>::BonusApprovalAlreadyUsed,
		);
	});
}

#[test]
fn liquidate_bond_lot_removes_it_from_future_frames_and_releases_on_maturity() {
	new_test_ext().execute_with(|| {
		LastOperationalBondTotal::set(None);
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 20 * MICROGONS_PER_ARGON);
		set_argons(3, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(3), 2, 1, None));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 3, None));
		let bond_lot_id = account_bond_lot_ids(2)[0];
		let other_bond_lot_id = account_bond_lot_ids(3)[0];
		assert!(BondLotIdsByVault::<Test>::contains_key(1, bond_lot_id));
		assert!(BondLotIdsByVault::<Test>::contains_key(2, other_bond_lot_id));
		assert_eq!(TotalArgonBondLots::<Test>::get(), 2);
		assert_eq!(LastOperationalBondTotal::get(), Some((account(2), 3 * MICROGONS_PER_ARGON)));

		set_target_securitization(10 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		let balance_before = Balances::balance(&account(2));

		assert_ok!(Treasury::liquidate_bond_lot(origin(2), bond_lot_id));
		assert!(BondLotIdsByVault::<Test>::contains_key(1, bond_lot_id));
		assert_eq!(TotalArgonBondLots::<Test>::get(), 2);
		assert_eq!(LastOperationalBondTotal::get(), Some((account(2), 0)));

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("releasing bond lot");
		assert_eq!(bond_lot.release_reason, Some(BondReleaseReason::UserLiquidation));
		assert_eq!(bond_lot.release_frame_id, Some(11));
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.regular_bonds, 0);
		assert_eq!(vault_bonds.flexible_bonds, 0);
		assert_eq!(PendingBondReleasesByFrame::<Test>::get(11), vec![bond_lot_id]);

		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		CurrentFrameId::set(2);
		collect_frame(1, &[2, 3]);
		assert_eq!(Balances::balance(&account(2)), balance_before + 1_500_000);
		assert_eq!(
			PositionsByAccount::<Test>::get(account(2)).unwrap().collected.bond_earnings,
			1_500_000
		);

		CurrentFrameId::set(11);
		Treasury::release_pending_bond_lots(11);

		assert!(BondLotById::<Test>::get(bond_lot_id).is_none());
		assert!(!BondLotIdsByVault::<Test>::contains_key(1, bond_lot_id));
		assert!(BondLotIdsByVault::<Test>::contains_key(2, other_bond_lot_id));
		assert_eq!(TotalArgonBondLots::<Test>::get(), 1);
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2)),
			0,
		);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 1, None));
	});
}

#[test]
fn active_account_vault_bond_amount_only_counts_vault_bonds() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));

		insert_vault(1, test_vault(10, (100 * MICROGONS_PER_ARGON) as u64));
		insert_vault(2, test_vault(11, (100 * MICROGONS_PER_ARGON) as u64));

		set_argons(2, 30 * MICROGONS_PER_ARGON);
		set_ownership(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 5, None));
		assert_ok!(Treasury::buy_bonds(origin(2), 2, 7, None));
		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 11));

		assert_eq!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::active_vault_bond_amount(
				1,
				&account(2),
			),
			5 * MICROGONS_PER_ARGON,
		);
		assert_eq!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::active_vault_bond_amount(
				2,
				&account(2),
			),
			7 * MICROGONS_PER_ARGON,
		);
		assert_eq!(
			<Treasury as TreasuryPoolProvider<TestAccountId>>::active_account_vault_bond_amount(
				&account(2),
			),
			12 * MICROGONS_PER_ARGON,
		);
	});
}

#[test]
fn regular_lots_do_not_bump_smaller_purchases() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (102 * MICROGONS_PER_ARGON) as u64));

		set_argons(10, MICROGONS_PER_ARGON);
		set_argons(2, 101 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(10), 1, 1, None));
		for _ in 0..101 {
			assert_ok!(Treasury::buy_bonds(origin(2), 1, 1, None));
		}

		assert_eq!(BondLotsByVault::<Test>::get(1).regular_bonds, 102);
		let operator_lot_id = account_bond_lot_ids(10)[0];
		let operator_lot = BondLotById::<Test>::get(operator_lot_id).expect("operator lot");
		assert_eq!(operator_lot.release_reason, None);
		assert_eq!(account_bond_lot_ids(2).len(), 101);
		assert_eq!(BondLotsByVault::<Test>::get(1).flexible_bonds, 0);
	});
}

#[test]
fn regular_bonds_keep_reward_weight_after_securitization_drops() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));

		insert_vault(1, test_vault(10, (5 * MICROGONS_PER_ARGON) as u64));
		Treasury::lock_in_vault_capital(1);

		let frame = CurrentFrameVaultCapital::<Test>::get().expect("frame capital");
		assert_eq!(frame.total_active_bonds, 10);
		assert_eq!(
			frame.vault_securitization_positions.get(&1).unwrap().active_bond_microgons,
			10 * MICROGONS_PER_ARGON
		);
	});
}

#[test]
fn failed_release_retries_and_does_not_block_current_frame_releases() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(1);

		BondLotById::<Test>::insert(
			0,
			BondLot {
				owner: account(2),
				program: BondProgram::Vault {
					vault_id: 1,
					sharing_percent: Permill::zero(),
					bonus_percent: Permill::zero(),
				},
				bonds: 1,
				is_flexible: false,
				created_frame_id: 1,
				participated_frames: 0,
				last_frame_earnings_frame_id: None,
				last_frame_earnings: None,
				cumulative_earnings: 0,
				release_frame_id: Some(11),
				release_reason: Some(BondReleaseReason::UserLiquidation),
			},
		);
		BondLotIdsByAccount::<Test>::insert(account(2), 0, ());
		PendingBondReleasesByFrame::<Test>::insert(11, BoundedVec::truncate_from(vec![0]));

		set_argons(3, MICROGONS_PER_ARGON);
		assert_ok!(Treasury::create_hold::<Balances>(&account(3), MICROGONS_PER_ARGON));
		BondLotById::<Test>::insert(
			1,
			BondLot {
				owner: account(3),
				program: BondProgram::Vault {
					vault_id: 1,
					sharing_percent: Permill::zero(),
					bonus_percent: Permill::zero(),
				},
				bonds: 1,
				is_flexible: false,
				created_frame_id: 1,
				participated_frames: 0,
				last_frame_earnings_frame_id: None,
				last_frame_earnings: None,
				cumulative_earnings: 0,
				release_frame_id: Some(12),
				release_reason: Some(BondReleaseReason::UserLiquidation),
			},
		);
		BondLotIdsByAccount::<Test>::insert(account(3), 1, ());
		PendingBondReleasesByFrame::<Test>::insert(12, BoundedVec::truncate_from(vec![1]));

		Treasury::release_pending_bond_lots(11);
		assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), Some(11));
		assert!(BondLotById::<Test>::get(0).is_some());
		assert_eq!(PendingBondReleasesByFrame::<Test>::get(11), vec![0]);

		Treasury::release_pending_bond_lots(12);
		assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), Some(11));
		assert!(BondLotById::<Test>::get(0).is_some());
		assert!(BondLotById::<Test>::get(1).is_none());
		assert_eq!(PendingBondReleasesByFrame::<Test>::get(11), vec![0]);
		assert!(PendingBondReleasesByFrame::<Test>::get(12).is_empty());
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(3),),
			0,
		);

		set_argons(2, MICROGONS_PER_ARGON);
		assert_ok!(Treasury::create_hold::<Balances>(&account(2), MICROGONS_PER_ARGON));
		Treasury::release_pending_bond_lots(13);

		assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), None);
		assert!(BondLotById::<Test>::get(0).is_none());
		assert!(PendingBondReleasesByFrame::<Test>::get(11).is_empty());
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2),),
			0,
		);
	});
}

#[test]
fn release_succeeds_when_other_consumers_need_the_last_provider() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owner = account(2);
		set_ownership(&owner, MICROGONS_PER_ARGON);
		assert_ok!(Treasury::create_hold::<Ownership>(&owner, MICROGONS_PER_ARGON));
		assert_ok!(frame_system::Pallet::<Test>::inc_consumers(&owner));
		assert_eq!(System::providers(&owner), 1);
		assert_eq!(System::consumers(&owner), 2);

		BondLotById::<Test>::insert(
			0,
			BondLot {
				owner: owner.clone(),
				program: BondProgram::Argonot,
				bonds: 1,
				is_flexible: false,
				created_frame_id: 1,
				participated_frames: 0,
				last_frame_earnings_frame_id: None,
				last_frame_earnings: None,
				cumulative_earnings: 0,
				release_frame_id: Some(11),
				release_reason: Some(BondReleaseReason::UserLiquidation),
			},
		);
		BondLotIdsByAccount::<Test>::insert(&owner, 0, ());
		PendingBondReleasesByFrame::<Test>::insert(11, BoundedVec::truncate_from(vec![0]));

		Treasury::release_pending_bond_lots(11);

		assert_eq!(
			Ownership::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &owner,),
			0,
		);
		assert_eq!(System::providers(&owner), 1);
		assert_eq!(System::consumers(&owner), 1);
		assert!(!BondLotById::<Test>::contains_key(0));
		assert!(!BondLotIdsByAccount::<Test>::contains_key(&owner, 0));
		assert!(PendingBondReleasesByFrame::<Test>::get(11).is_empty());
		assert_eq!(PendingBondReleaseRetryCursor::<Test>::get(), None);
		System::assert_last_event(RuntimeEvent::Treasury(super::Event::BondLotReleased {
			frame_id: 11,
			program_id: BondProgramId::Argonot,
			bond_lot_id: 0,
			account_id: owner,
			bonds: 1,
		}));
	});
}

#[test]
fn bond_purchase_rolls_back_when_account_principal_would_overflow() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(1, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(2, 100 * MICROGONS_PER_ARGON);
		PositionsByAccount::<Test>::insert(
			account(2),
			pallet_treasury_positions::Position {
				bond_principal: Balance::MAX,
				upstream: None,
				..Default::default()
			},
		);
		assert_noop!(Treasury::buy_bonds(origin(2), 1, 100, None), ArithmeticError::Overflow);
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(
			Balances::balance_on_hold(
				&RuntimeHoldReason::Treasury(HoldReason::ContributedToTreasury),
				&account(2)
			),
			0
		);
	});
}

#[test]
fn flexible_bond_lots_are_not_capped_per_vault() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(1, (26 * MICROGONS_PER_ARGON) as u64));
		set_argons(1, 26 * MICROGONS_PER_ARGON);
		for _ in 0..26 {
			assert_ok!(Treasury::buy_bonds(origin(1), 1, 1, None));
		}
		let lot_ids = account_bond_lot_ids(1);
		for lot_id in &lot_ids {
			assert_ok!(Treasury::set_bond_lot_flexible(origin(1), *lot_id, true));
		}
		assert_eq!(BondLotsByVault::<Test>::get(1).flexible_bonds, 26);
		assert!(BondLotById::<Test>::get(lot_ids[25]).unwrap().is_flexible);
	});
}

#[test]
fn flexible_bonds_yield_their_vault_admission_capacity() {
	new_test_ext().execute_with(|| {
		UpstreamParticipationFactor::<Test>::put(Percent::from_percent(100));
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		Upstreams::mutate(|upstreams| {
			upstreams.insert(account(2), (account(2), 1));
		});
		AvailableSecuritizationSpace::mutate(|space| {
			space.insert(1, 0);
		});

		set_argons(10, 10 * MICROGONS_PER_ARGON);
		set_argons(2, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let operator_lot_id = account_bond_lot_ids(10)[0];
		// Exhausted space and flexible displacement do not reduce the nominal targets.
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::zero());
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), operator_lot_id, true));
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::zero());
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 10, None));

		assert_eq!(
			Treasury::upstream_participation(&account(2)),
			FixedU128::from_rational(1, 1_000)
		);
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.flexible_bonds, 10);
		assert_eq!(vault_bonds.regular_bonds, 10);

		assert_ok!(Treasury::liquidate_bond_lot(origin(10), operator_lot_id));

		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.flexible_bonds, 0);
		assert_eq!(vault_bonds.regular_bonds, 10);
	});
}

#[test]
fn reservation_uses_capacity_yielded_by_flexible_bonds() {
	new_test_ext().execute_with(|| {
		UpstreamParticipationFactor::<Test>::put(Percent::from_percent(100));
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		Upstreams::mutate(|upstreams| {
			upstreams.insert(account(2), (account(2), 1));
		});
		AvailableSecuritizationSpace::mutate(|space| {
			space.insert(1, 0);
		});
		set_argons(10, 10 * MICROGONS_PER_ARGON);
		set_argons(2, 10 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let operator_lot_id = account_bond_lot_ids(10)[0];
		// Admission changes do not grant participation credit.
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::zero());
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), operator_lot_id, true));
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::zero());
		assert_noop!(
			Treasury::set_reserved_bond_space(origin(10), 1, 11),
			Error::<Test>::InsufficientBondSpace
		);
		assert_ok!(Treasury::set_reserved_bond_space(origin(10), 1, 10));
		// Reserved space cannot be purchased without the upstream's approval.
		assert_eq!(Treasury::upstream_participation(&account(2)), FixedU128::zero());

		assert_ok!(Treasury::buy_bonds(
			origin(2),
			1,
			10,
			Some(bonus_approval(1, 2, Permill::zero(), 1, 10, System::block_number(),)),
		));
		assert_eq!(
			Treasury::upstream_participation(&account(2)),
			FixedU128::from_rational(1, 1_000)
		);
		let vault_bonds = BondLotsByVault::<Test>::get(1);
		assert_eq!(vault_bonds.flexible_bonds, 10);
		assert_eq!(vault_bonds.regular_bonds, 10);
		assert_eq!(vault_bonds.reserved_bond_space, 0);
		assert_noop!(
			Treasury::set_reserved_bond_space(origin(10), 1, 1),
			Error::<Test>::InsufficientBondSpace
		);
		assert_noop!(
			Treasury::set_bond_lot_flexible(origin(10), operator_lot_id, false),
			Error::<Test>::InsufficientBondSpace,
		);
		assert_ok!(Treasury::liquidate_bond_lot(origin(10), operator_lot_id));
	});
}

#[test]
fn flexible_bond_total_does_not_saturate_past_bonds_max() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (Bonds::MAX as u64) * MICROGONS_PER_ARGON as u64));
		set_argons(10, (Bonds::MAX as u128 + 1) * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, Bonds::MAX, None));
		let first_lot = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), first_lot, true));
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 1, None));
		let second_lot = account_bond_lot_ids(10)
			.into_iter()
			.find(|lot_id| BondLotById::<Test>::get(lot_id).is_some_and(|lot| lot.bonds == 1))
			.expect("one-bond lot");
		assert_err!(
			Treasury::set_bond_lot_flexible(origin(10), second_lot, true),
			ArithmeticError::Overflow,
		);
		assert_eq!(BondLotsByVault::<Test>::get(1).flexible_bonds, Bonds::MAX);
		assert_eq!(BondLotsByVault::<Test>::get(1).regular_bonds, 1);
		assert!(!BondLotById::<Test>::get(second_lot).unwrap().is_flexible);
	});
}

#[test]
fn flexible_lots_share_account_earnings_after_aggregate_displacement() {
	for (lots, regular, flexible_earnings, regular_earnings) in [
		(vec![10], 6, 2_000_000, 3_000_000),
		(vec![1, 1], 1, 2_500_000, 2_500_000),
		(vec![1, 1], 0, 5_000_000, 0),
	] {
		new_test_ext().execute_with(|| {
			CurrentFrameId::set(0);
			assert_ok!(TreasuryPositions::prepare_collect());
			MinimumArgonsPerContributor::set(1);
			let flexible = lots.iter().sum::<Bonds>();
			insert_vault(1, test_vault(10, u64::from(flexible) * MICROGONS_PER_ARGON as u64));
			set_argons(10, u128::from(flexible) * MICROGONS_PER_ARGON);
			for bonds in lots {
				assert_ok!(Treasury::buy_bonds(origin(10), 1, bonds, None));
			}
			for lot in account_bond_lot_ids(10) {
				assert_ok!(Treasury::set_bond_lot_flexible(origin(10), lot, true));
			}
			if regular > 0 {
				set_argons(2, u128::from(regular) * MICROGONS_PER_ARGON);
				assert_ok!(Treasury::buy_bonds(origin(2), 1, regular, None));
			}

			set_target_securitization(u128::from(flexible) * MICROGONS_PER_ARGON);
			CurrentFrameId::set(1);
			Treasury::lock_in_vault_capital(1);
			let frame = CurrentFrameVaultCapital::<Test>::get().unwrap();
			assert_eq!(frame.total_active_bonds, u128::from(flexible));
			assert_eq!(
				frame.vault_securitization_positions[&1].active_bond_microgons,
				u128::from(flexible) * MICROGONS_PER_ARGON
			);
			assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, regular);
			assert_eq!(
				TreasuryPositions::account_quantities(&account(10)).bonds,
				u128::from(flexible - regular)
			);

			assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
			collect_frame(1, &[10, 2]);
			assert_eq!(LastVaultProfits::get()[0].capital_contributed_by_vault, 0);
			assert_eq!(
				PositionsByAccount::<Test>::get(account(10)).unwrap().collected.bond_earnings,
				flexible_earnings
			);
			assert_eq!(
				PositionsByAccount::<Test>::get(account(2))
					.map(|p| p.collected.bond_earnings)
					.unwrap_or_default(),
				regular_earnings
			);
		});
	}
}

#[test]
fn fully_displaced_bonds_with_zero_target_do_not_stop_frame_transition() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let bond_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), bond_lot_id, true));

		// Vault close reports zero available securitization, displacing its flexible bonds.
		let mut closed_vault = test_vault(10, 0);
		closed_vault.is_closed = true;
		insert_vault(1, closed_vault);
		assert_ok!(Treasury::vault_securitization_changed(1, 0));
		TargetBitcoinPercent::<Test>::put(Percent::zero());
		MintedBitcoinMicrogons::set(0);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		let capital = CurrentFrameVaultCapital::<Test>::get().unwrap();
		assert_eq!(capital.total_active_bonds, 0);
		assert_eq!(capital.target_securitization, 0);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));

		Treasury::run_frame_transition(2);

		assert_eq!(Balances::balance(&account(10)), 0);
		assert_eq!(Balances::balance(&BidPoolAccountId::get()), 0);
		assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().frame_id, 2);
	});
}

#[test]
fn mid_frame_purchase_displaces_flexible_bonds_for_the_next_frame() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		MinimumArgonsPerContributor::set(1);
		CurrentFrameId::set(0);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 10 * MICROGONS_PER_ARGON);
		set_argons(2, 6 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 10, None));
		let flexible_lot_id = account_bond_lot_ids(10)[0];
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), flexible_lot_id, true));
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);

		assert_ok!(Treasury::buy_bonds(origin(2), 1, 6, None));
		assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, 6);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		collect_frame(1, &[10, 2]);

		CurrentFrameId::set(2);
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(2);
		Treasury::lock_in_vault_capital(2);
		assert_eq!(CurrentFrameVaultCapital::<Test>::get().unwrap().total_active_bonds, 10);
		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		collect_frame(2, &[10, 2]);
		assert_eq!(
			PositionsByAccount::<Test>::get(account(10)).unwrap().collected.bond_earnings,
			7_000_000
		);
		assert_eq!(
			PositionsByAccount::<Test>::get(account(2)).unwrap().collected.bond_earnings,
			3_000_000
		);
	});
}

#[test]
fn same_frame_flexibility_changes_preserve_frozen_earning_quantities() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(10, (10 * MICROGONS_PER_ARGON) as u64));
		set_argons(10, 3 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(10), 1, 3, None));
		let lot_id = account_bond_lot_ids(10)[0];
		set_target_securitization(10 * MICROGONS_PER_ARGON);
		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), lot_id, true));
		assert_ok!(Treasury::set_bond_lot_flexible(origin(10), lot_id, false));
		let lot = BondLotById::<Test>::get(lot_id).unwrap();
		assert!(!lot.is_flexible);

		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		collect_frame(1, &[10]);
		let vault_paid: Balance =
			LastVaultProfits::get().iter().map(|earnings| earnings.earnings_for_vault).sum();
		assert!(vault_paid > 0);
		assert_eq!(Treasury::available_budget(), 77 * MICROGONS_PER_ARGON - vault_paid - 1_500_000);
		assert_eq!(
			PositionsByAccount::<Test>::get(account(10)).unwrap().collected.bond_earnings,
			1500000
		);
	});
}

#[test]
fn account_quantities_track_operator_displacement_capital_releases_and_burns() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(1, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(1, 200 * MICROGONS_PER_ARGON);
		set_argons(2, 100 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(1), 1, 100, None));
		assert_ok!(Treasury::set_bond_lot_flexible(origin(1), account_bond_lot_ids(1)[0], true));
		assert_ok!(Treasury::buy_bonds(origin(2), 1, 60, None));
		assert_eq!(TreasuryPositions::bond_principal(&account(1)), 100 * MICROGONS_PER_ARGON);
		assert_eq!(TreasuryPositions::account_quantities(&account(1)).bonds, 40);
		assert_eq!(TreasuryPositions::account_quantities(&account(2)).bonds, 60);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.bonds, 100);
		// The operator buying regular bonds changes both of its contributions exactly once.
		assert_ok!(Treasury::buy_bonds(origin(1), 1, 20, None));
		assert_eq!(TreasuryPositions::account_quantities(&account(1)).bonds, 40);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.bonds, 100);
		// Regular bonds keep earning when capital drops; displaced flexible principal remains.
		insert_vault(1, test_vault(1, (70 * MICROGONS_PER_ARGON) as u64));
		assert_eq!(TreasuryPositions::account_quantities(&account(1)).bonds, 20);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.bonds, 80);
		assert_eq!(TreasuryPositions::bond_principal(&account(1)), 120 * MICROGONS_PER_ARGON);
		insert_vault(1, test_vault(1, (200 * MICROGONS_PER_ARGON) as u64));
		assert_eq!(TreasuryPositions::account_quantities(&account(1)).bonds, 120);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.bonds, 180);
		assert_ok!(Treasury::liquidate_bond_lot(origin(2), account_bond_lot_ids(2)[0]));
		assert_eq!(TreasuryPositions::network_totals().account_quantities.bonds, 120);
		assert_ok!(
			Treasury::encumber_bond_microgons(&account(1), 120 * MICROGONS_PER_ARGON, None,)
		);
		assert_ok!(with_storage_layer(|| Treasury::settle_bond_collateral_burn(
			&account(1),
			20 * MICROGONS_PER_ARGON,
			&mut Weight::zero()
		)));
		assert_eq!(TreasuryPositions::network_totals().account_quantities.bonds, 100);
		assert_ok!(with_storage_layer(|| Treasury::settle_bond_collateral_burn(
			&account(1),
			50 * MICROGONS_PER_ARGON,
			&mut Weight::zero()
		)));
		assert_eq!(TreasuryPositions::network_totals().account_quantities.bonds, 50);
		assert_eq!(TreasuryPositions::bond_principal(&account(1)), 50 * MICROGONS_PER_ARGON);
		assert_ok!(with_storage_layer(|| Treasury::settle_bond_collateral_burn(
			&account(1),
			50 * MICROGONS_PER_ARGON,
			&mut Weight::zero()
		)));
		assert_eq!(TreasuryPositions::network_totals(), NetworkPositionQuantities::default());
	});
}

#[test]
fn failed_operator_quantity_update_rolls_back_the_buyers_purchase_and_currency_hold() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		insert_vault(1, test_vault(1, (100 * MICROGONS_PER_ARGON) as u64));
		set_argons(1, 100 * MICROGONS_PER_ARGON);
		set_argons(2, 100 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_bonds(origin(1), 1, 100, None));
		assert_ok!(Treasury::set_bond_lot_flexible(origin(1), account_bond_lot_ids(1)[0], true));
		PositionsByAccount::<Test>::mutate(account(1), |position| {
			position.as_mut().unwrap().quantities.bonds = 0;
		});
		assert_noop!(Treasury::buy_bonds(origin(2), 1, 60, None), ArithmeticError::Underflow);
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(TreasuryPositions::network_totals().account_quantities.bonds, 100);
		assert_eq!(BondLotsByVault::<Test>::get(1).displaced_flexible_bonds, 0);
	});
}

#[test]
fn buy_argonot_bonds_allows_multiple_lots_per_account() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));
		set_ownership(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 5));
		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 4));

		let bond_lot_ids = account_bond_lot_ids(2);
		assert_eq!(bond_lot_ids.len(), 2);
		assert_eq!(argonot_bond_lots().len(), 2);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.stakes, 9);
		assert_eq!(TreasuryPositions::account_quantities(&account(2)).stakes, 9);
		assert_eq!(
			Ownership::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2),),
			9 * MICROGONS_PER_ARGON,
		);
	});
}

#[test]
fn buy_argonot_bonds_rejects_when_full_queue_lot_does_not_beat_floor() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxActiveArgonotBondLots::set(2);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));

		set_ownership(2, 20 * MICROGONS_PER_ARGON);
		set_ownership(3, 20 * MICROGONS_PER_ARGON);
		set_ownership(4, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 3));
		assert_ok!(Treasury::buy_argonot_bonds(origin(3), 5));

		assert_err!(
			Treasury::buy_argonot_bonds(origin(4), 3),
			Error::<Test>::ArgonotBondPurchaseBelowCutoff,
		);

		assert!(account_bond_lot_ids(4).is_empty());
		assert_eq!(argonot_bond_lots().len(), 2);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.stakes, 8);
	});
}

#[test]
fn buy_argonot_bonds_evicts_floor_and_schedules_release() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		MaxActiveArgonotBondLots::set(2);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));

		set_ownership(2, 20 * MICROGONS_PER_ARGON);
		set_ownership(3, 20 * MICROGONS_PER_ARGON);
		set_ownership(4, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 3));
		assert_ok!(Treasury::buy_argonot_bonds(origin(3), 5));
		let evicted_bond_lot_id = account_bond_lot_ids(2)[0];
		let retained_bond_lot_id = account_bond_lot_ids(3)[0];

		assert_ok!(Treasury::buy_argonot_bonds(origin(4), 6));
		let new_bond_lot_id = account_bond_lot_ids(4)[0];

		let evicted_bond_lot = BondLotById::<Test>::get(evicted_bond_lot_id).expect("bond lot");
		assert_eq!(evicted_bond_lot.release_reason, Some(BondReleaseReason::Bumped));
		assert_eq!(evicted_bond_lot.release_frame_id, Some(11));
		assert_eq!(argonot_bond_lots().len(), 2);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.stakes, 11);
		assert_eq!(TreasuryPositions::account_quantities(&account(2)).stakes, 0);
		assert_eq!(TreasuryPositions::account_quantities(&account(3)).stakes, 5);
		assert_eq!(TreasuryPositions::account_quantities(&account(4)).stakes, 6);
		assert_eq!(
			Ownership::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2),),
			3 * MICROGONS_PER_ARGON,
		);
		assert_eq!(
			argonot_bond_lots(),
			vec![
				BondLotSummary { bond_lot_id: retained_bond_lot_id, bonds: 5 },
				BondLotSummary { bond_lot_id: new_bond_lot_id, bonds: 6 },
			],
		);
	});
}

#[test]
fn buy_argonot_bonds_enforces_circulation_cap() {
	new_test_ext().execute_with(|| {
		MinimumArgonsPerContributor::set(1);
		set_ownership(2, 10 * MICROGONS_PER_ARGON);
		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 6));

		assert_err!(
			Treasury::buy_argonot_bonds(origin(2), 1),
			Error::<Test>::ArgonotBondPurchaseAboveCap,
		);

		assert_eq!(account_bond_lot_ids(2).len(), 1);
		assert_eq!(argonot_bond_lots().len(), 1);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.stakes, 6);
	});
}

#[test]
fn liquidate_argonot_bond_lot_removes_queue_entry_and_releases_ownership_on_maturity() {
	new_test_ext().execute_with(|| {
		CurrentFrameId::set(0);
		assert_ok!(TreasuryPositions::prepare_collect());
		MinimumArgonsPerContributor::set(1);
		MaxArgonotBondedPercentOfCirculation::set(Percent::from_percent(100));
		set_ownership(2, 20 * MICROGONS_PER_ARGON);

		assert_ok!(Treasury::buy_argonot_bonds(origin(2), 3));
		let bond_lot_id = account_bond_lot_ids(2)[0];

		CurrentFrameId::set(1);
		Treasury::lock_in_vault_capital(1);

		assert_ok!(Treasury::liquidate_bond_lot(origin(2), bond_lot_id));

		let bond_lot = BondLotById::<Test>::get(bond_lot_id).expect("releasing bond lot");
		assert_eq!(bond_lot.release_reason, Some(BondReleaseReason::UserLiquidation));
		assert_eq!(bond_lot.release_frame_id, Some(11));
		assert!(argonot_bond_lots().is_empty());
		assert_eq!(TreasuryPositions::network_totals().account_quantities.stakes, 0);
		assert_eq!(
			Ownership::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2),),
			3 * MICROGONS_PER_ARGON,
		);

		assert_ok!(Balances::mint_into(&BidPoolAccountId::get(), 100 * MICROGONS_PER_ARGON));
		CurrentFrameId::set(2);
		collect_frame(1, &[2]);
		assert_eq!(Balances::balance(&account(2)), 15_000_000);
		assert_eq!(
			PositionsByAccount::<Test>::get(account(2)).unwrap().collected.stake_earnings,
			15_000_000
		);

		CurrentFrameId::set(11);
		Treasury::release_pending_bond_lots(11);

		assert!(BondLotById::<Test>::get(bond_lot_id).is_none());
		assert!(account_bond_lot_ids(2).is_empty());
		assert_eq!(
			Ownership::balance_on_hold(&HoldReason::ContributedToTreasury.into(), &account(2),),
			0,
		);
	});
}
