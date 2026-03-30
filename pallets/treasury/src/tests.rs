use super::{
	Error, Event, FunderState, FunderStateByVaultAndAccount, FundersByVaultId, HoldReason,
	PendingUnlock, PendingUnlocksByFrame, TreasuryCapital,
	mock::{Treasury, *},
	pallet::{CapitalActive, TreasuryPool, VaultPoolsByFrame},
};
use argon_primitives::{
	OnNewSlot, OperationalRewardKind, OperationalRewardPayout, OperationalRewardsPayer,
};
use frame_support::{assert_err, assert_ok, traits::fungible::InspectHold};
use pallet_prelude::{argon_primitives::vault::VaultTreasuryFrameEarnings, *};
use sp_runtime::Permill;

fn has_pending_unlock_index(frame_id: FrameId, vault_id: VaultId, account_id: u64) -> bool {
	PendingUnlocksByFrame::<Test>::get(frame_id).into_iter().any(|pending_unlock| {
		pending_unlock.vault_id == vault_id && pending_unlock.account_id == account_id
	})
}

fn pending_unlock_state(vault_id: VaultId, account_id: u64) -> (Balance, Option<FrameId>) {
	let state = FunderStateByVaultAndAccount::<Test>::get(vault_id, account_id).unwrap();
	(state.pending_unlock_amount, state.pending_unlock_at_frame)
}

fn tracked_funders(vault_id: VaultId) -> Vec<(u64, Balance)> {
	FundersByVaultId::<Test>::get(vault_id).into_inner()
}

#[test]
fn test_set_allocation_updates_held_principal_and_schedules_delayed_unlocks() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		set_argons(2, 500_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 200_000_000));
		System::assert_last_event(
			Event::<Test>::VaultFunderAllocation {
				vault_id: 1,
				account_id: 2,
				amount: 200_000_000,
				previous_amount: None,
			}
			.into(),
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 200_000_000);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal,
			200_000_000
		);
		assert_eq!(pending_unlock_state(1, 2), (0, None));
		assert_eq!(FundersByVaultId::<Test>::get(1).len(), 1);

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 300_000_000));
		System::assert_last_event(
			Event::<Test>::VaultFunderAllocation {
				vault_id: 1,
				account_id: 2,
				amount: 300_000_000,
				previous_amount: Some(200_000_000),
			}
			.into(),
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 300_000_000);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal,
			300_000_000
		);

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100_000_000));
		System::assert_last_event(
			Event::<Test>::VaultFunderAllocation {
				vault_id: 1,
				account_id: 2,
				amount: 100_000_000,
				previous_amount: Some(300_000_000),
			}
			.into(),
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 300_000_000);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal,
			300_000_000
		);
		assert_eq!(pending_unlock_state(1, 2), (200_000_000, Some(11)));
		assert!(has_pending_unlock_index(11, 1, 2));
	});
}

#[test]
fn test_set_allocation_below_minimum_rejected_but_zero_allowed() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(500);

		set_argons(2, 10_000);
		assert_err!(
			Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 499),
			Error::<Test>::BelowMinimum
		);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 0));
	});
}

#[test]
fn test_set_allocation_tracks_top_active_funders_when_index_is_full() {
	MaxTreasuryContributors::set(1);
	MaxTrackedTreasuryFunders::set(1);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		set_argons(2, 1_000_000);
		set_argons(3, 1_000_000);

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 100);
		assert_eq!(FundersByVaultId::<Test>::get(1).len(), 1);
		assert_eq!(tracked_funders(1), vec![(2, 100)]);

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 1, 50));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &3), 50);
		assert_eq!(tracked_funders(1), vec![(2, 100)]);

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 1, 200));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &3), 200);
		assert_eq!(tracked_funders(1), vec![(3, 200)]);
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal, 100);
	});
}

#[test]
fn test_active_funder_index_keeps_operator_when_full() {
	MaxTreasuryContributors::set(2);
	MaxTrackedTreasuryFunders::set(2);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: 5_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(10, 1_000_000);
		set_argons(2, 1_000_000);
		set_argons(3, 1_000_000);

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(10), 1, 10));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 1, 90));

		let funders = tracked_funders(1);
		assert_eq!(funders, vec![(10, 10), (2, 100)]);
	});
}

#[test]
fn test_lock_in_sorts_tracked_funders_by_principal() {
	MaxTreasuryContributors::set(2);
	MaxTrackedTreasuryFunders::set(2);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: 100,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(2, 1_000_000);
		set_argons(3, 1_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 40));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 1, 90));

		Treasury::lock_in_vault_capital(2);

		let pool = VaultPoolsByFrame::<Test>::get(2).get(&1).cloned().unwrap();
		assert_eq!(pool.bond_holders[0], (3, 90));
		assert_eq!(pool.bond_holders[1], (2, 10));
	});
}

#[test]
fn test_standby_tracked_funder_recovers_when_active_funder_exits() {
	MaxTreasuryContributors::set(2);
	MaxTrackedTreasuryFunders::set(3);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: 1_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(2, 1_000_000);
		set_argons(3, 1_000_000);
		set_argons(4, 1_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 1, 90));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(4), 1, 80));
		assert_eq!(tracked_funders(1), vec![(2, 100), (3, 90), (4, 80)]);

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 0));
		Treasury::release_pending_unlocks(11);
		assert_eq!(tracked_funders(1), vec![(3, 90), (4, 80)]);

		Treasury::lock_in_vault_capital(12);
		let pool = VaultPoolsByFrame::<Test>::get(12).get(&1).cloned().unwrap();
		assert_eq!(pool.bond_holders[0], (3, 90));
		assert_eq!(pool.bond_holders[1], (4, 80));
	});
}

#[test]
fn test_set_allocation_multiple_vaults_independent_but_hold_adds() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		set_argons(2, 1_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 200_000_000));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 2, 100_000_000));

		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 300_000_000);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal,
			200_000_000
		);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(2, 2).unwrap().held_principal,
			100_000_000
		);
		assert_eq!(FundersByVaultId::<Test>::get(1).len(), 1);
		assert_eq!(FundersByVaultId::<Test>::get(2).len(), 1);
	});
}

#[test]
fn test_increase_cancels_pending_unlocks_before_adding_more_hold() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		set_argons(2, 1_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 40));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 100);
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal, 100);
		assert_eq!(pending_unlock_state(1, 2), (60, Some(11)));

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 70));
		System::assert_last_event(
			Event::<Test>::VaultFunderAllocation {
				vault_id: 1,
				account_id: 2,
				amount: 70,
				previous_amount: Some(40),
			}
			.into(),
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 100);
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal, 100);
		assert_eq!(pending_unlock_state(1, 2), (30, Some(11)));

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 130));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 130);
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal, 130);
		assert_eq!(pending_unlock_state(1, 2), (0, None));
		assert!(!has_pending_unlock_index(11, 1, 2));
		assert!(!PendingUnlocksByFrame::<Test>::contains_key(11));
	});
}

#[test]
fn test_reducing_again_reschedules_pending_unlock_to_latest_frame() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		set_argons(2, 1_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 80));
		assert_eq!(pending_unlock_state(1, 2), (20, Some(11)));
		assert!(has_pending_unlock_index(11, 1, 2));

		CurrentFrameId::set(2);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 50));

		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 100);
		assert_eq!(pending_unlock_state(1, 2), (50, Some(12)));
		assert!(!has_pending_unlock_index(11, 1, 2));
		assert!(has_pending_unlock_index(12, 1, 2));
	});
}

#[test]
fn test_lock_in_uses_full_principal_during_unlock_cooldown() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: 1_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(2, 1_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 500_000_000));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100_000_000));

		Treasury::lock_in_vault_capital(2);
		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![TreasuryCapital { vault_id: 1, activated_capital: 500_000_000u128, frame_id: 2 }],
		);
		assert_eq!(pending_unlock_state(1, 2), (400_000_000, Some(11)));
	});
}

#[test]
fn test_pending_unlock_releases_after_ten_frames() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		set_argons(2, 1_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 40));

		Treasury::release_pending_unlocks(10);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 100);
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal, 100);

		Treasury::release_pending_unlocks(11);
		System::assert_last_event(
			Event::<Test>::RefundedTreasuryCapital {
				frame_id: 11,
				vault_id: 1,
				amount: 60,
				account_id: 2,
			}
			.into(),
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 40);
		assert_eq!(pending_unlock_state(1, 2), (0, None));
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal, 40);
		assert!(PendingUnlocksByFrame::<Test>::get(11).is_empty());
	});
}

#[test]
fn test_rescheduled_pending_unlock_releases_only_at_latest_frame() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		set_argons(2, 1_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 80));

		CurrentFrameId::set(2);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 50));

		Treasury::release_pending_unlocks(11);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 100);
		assert_eq!(pending_unlock_state(1, 2), (50, Some(12)));

		Treasury::release_pending_unlocks(12);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 50);
		assert_eq!(pending_unlock_state(1, 2), (0, None));
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal, 50);
	});
}

#[test]
fn test_full_exit_releases_and_cleans_up_state() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		set_argons(2, 1_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 0));

		Treasury::release_pending_unlocks(11);

		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 0);
		assert!(FunderStateByVaultAndAccount::<Test>::get(1, 2).is_none());
		assert_eq!(FundersByVaultId::<Test>::get(1).len(), 0);
	});
}

#[test]
fn test_failed_pending_unlock_is_retried_next_frame() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		FunderStateByVaultAndAccount::<Test>::insert(
			1,
			2,
			FunderState {
				held_principal: 100,
				pending_unlock_amount: 100,
				pending_unlock_at_frame: Some(11),
				lifetime_principal_last_basis_frame: 1,
				..Default::default()
			},
		);
		Treasury::refresh_funder_index(1, &2, 100);
		PendingUnlocksByFrame::<Test>::mutate(11, |pending_unlocks| {
			pending_unlocks.try_push(PendingUnlock { vault_id: 1, account_id: 2 }).unwrap();
		});

		Treasury::release_pending_unlocks(11);

		assert_eq!(pending_unlock_state(1, 2), (100, Some(11)));
		assert!(has_pending_unlock_index(11, 1, 2));

		set_argons(2, 100);
		assert_ok!(Treasury::create_hold(&2, 100));

		Treasury::release_pending_unlocks(12);

		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 0);
		assert!(FunderStateByVaultAndAccount::<Test>::get(1, 2).is_none());
		assert!(PendingUnlocksByFrame::<Test>::get(11).is_empty());
	});
}

#[test]
fn test_current_frame_unlocks_still_process_while_overdue_retry_exists() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		FunderStateByVaultAndAccount::<Test>::insert(
			1,
			2,
			FunderState {
				held_principal: 100,
				pending_unlock_amount: 100,
				pending_unlock_at_frame: Some(11),
				lifetime_principal_last_basis_frame: 1,
				..Default::default()
			},
		);
		Treasury::refresh_funder_index(1, &2, 100);
		PendingUnlocksByFrame::<Test>::mutate(11, |pending_unlocks| {
			pending_unlocks.try_push(PendingUnlock { vault_id: 1, account_id: 2 }).unwrap();
		});

		set_argons(3, 50);
		assert_ok!(Treasury::create_hold(&3, 50));
		FunderStateByVaultAndAccount::<Test>::insert(
			1,
			3,
			FunderState {
				held_principal: 50,
				pending_unlock_amount: 50,
				pending_unlock_at_frame: Some(12),
				lifetime_principal_last_basis_frame: 1,
				..Default::default()
			},
		);
		Treasury::refresh_funder_index(1, &3, 50);
		PendingUnlocksByFrame::<Test>::mutate(12, |pending_unlocks| {
			pending_unlocks.try_push(PendingUnlock { vault_id: 1, account_id: 3 }).unwrap();
		});

		Treasury::release_pending_unlocks(11);
		Treasury::release_pending_unlocks(12);

		assert_eq!(pending_unlock_state(1, 2), (100, Some(11)));
		assert!(has_pending_unlock_index(11, 1, 2));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &3), 0);
		assert!(FunderStateByVaultAndAccount::<Test>::get(1, 3).is_none());
		assert!(PendingUnlocksByFrame::<Test>::get(12).is_empty());
	});
}

#[test]
fn test_can_lock_next_pool_capital() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				securitized_satoshis: 5_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 1,
				securitized_satoshis: 5_000_000_000,
				sharing_percent: Permill::from_percent(40),
				is_closed: false,
			},
		);

		set_argons(2, 5_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 2_000_000_000));
		set_argons(3, 5_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 1, 3_000_000_000));
		set_argons(4, 5_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(4), 2, 2_500_000_000));
		set_argons(5, 5_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(5), 2, 2_000_000_000));

		Treasury::lock_in_vault_capital(2);
		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![
				TreasuryCapital { vault_id: 1, activated_capital: 5_000_000_000u128, frame_id: 2 },
				TreasuryCapital { vault_id: 2, activated_capital: 4_500_000_000u128, frame_id: 2 },
			],
			"sorted with biggest share first"
		);

		System::assert_last_event(
			Event::<Test>::NextBidPoolCapitalLocked {
				frame_id: 2,
				participating_vaults: 2,
				total_activated_capital: 9_500_000_000,
			}
			.into(),
		);
	});
}

#[test]
fn test_lock_in_respects_max_vaults_per_pool() {
	MaxVaultsPerPool::set(2);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				securitized_satoshis: 5_000_000_000,
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 1,
				securitized_satoshis: 5_000_000_000,
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);
		insert_vault(
			3,
			TestVault {
				account_id: 1,
				securitized_satoshis: 5_000_000_000,
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);

		set_argons(2, 10_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 5_000_000_000));
		set_argons(3, 10_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 2, 3_000_000_000));
		set_argons(4, 10_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(4), 3, 1_000_000_000));

		Treasury::lock_in_vault_capital(2);
		let active = CapitalActive::<Test>::get().into_inner();
		assert_eq!(active.len(), 2);
		assert_eq!(
			active,
			vec![
				TreasuryCapital { vault_id: 1, activated_capital: 5_000_000_000u128, frame_id: 2 },
				TreasuryCapital { vault_id: 2, activated_capital: 3_000_000_000u128, frame_id: 2 },
			],
		);
	});
}

#[test]
fn test_lock_in_caps_by_securitized_satoshis_limit() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				securitized_satoshis: 1_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(2, 10_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 5_000_000_000));

		Treasury::lock_in_vault_capital(2);
		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![TreasuryCapital {
				vault_id: 1,
				activated_capital: 1_000_000_000u128,
				frame_id: 2
			}],
		);
	});
}

#[test]
fn test_lock_in_caps_by_explicit_securitized_satoshis_amount() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				securitized_satoshis: 1_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);
		set_vault_securitized_satoshis(1, 77_777);

		set_argons(2, 10_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 5_000_000_000));

		Treasury::lock_in_vault_capital(2);
		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![TreasuryCapital { vault_id: 1, activated_capital: 77_777u128, frame_id: 2 }],
		);
	});
}

#[test]
fn test_treasury_pool_participated_is_reported_when_operator_is_selected() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		reset_treasury_pool_participated();

		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: 1_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(10, 1_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(10), 1, 100_000_000));

		Treasury::lock_in_vault_capital(2);
		assert_eq!(take_treasury_pool_participated(), vec![(10, 100_000_000u128)]);

		Treasury::lock_in_vault_capital(3);
		assert_eq!(take_treasury_pool_participated(), vec![(10, 100_000_000u128)]);
	});
}

#[test]
fn test_full_exit_cleans_up_operator_state() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		reset_treasury_pool_participated();

		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: 1_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(10, 1_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(10), 1, 100_000_000));
		Treasury::lock_in_vault_capital(2);
		assert_eq!(take_treasury_pool_participated(), vec![(10, 100_000_000u128)]);

		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(10), 1, 0));
		Treasury::release_pending_unlocks(11);

		assert!(FunderStateByVaultAndAccount::<Test>::get(1, 10).is_none());
		assert!(FundersByVaultId::<Test>::get(1).is_empty());
	});
}

#[test]
fn test_pay_operational_rewards_from_treasury_reserves() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		set_pending_operational_rewards(vec![]);
		let reserves_account = Treasury::get_treasury_reserves_account();

		set_argons(reserves_account, 1_000_000_000);
		set_argons(42, 0);

		let reward = OperationalRewardPayout {
			operational_account: 99,
			payout_account: 42,
			reward_kind: OperationalRewardKind::Activation,
			amount: 250_000_000,
		};

		set_pending_operational_rewards(vec![reward.clone()]);
		Treasury::pay_operational_rewards(pending_operational_rewards());

		assert_eq!(Balances::free_balance(42), 250_000_000);
		assert_eq!(Balances::free_balance(reserves_account), 750_000_000);
		assert_eq!(take_paid_operational_rewards(), vec![reward]);
	});
}

#[test]
fn test_on_frame_start_runs_treasury_transition_and_pays_rewards() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: 1_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(10, 1_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(10), 1, 100_000_000));

		let reserves_account = Treasury::get_treasury_reserves_account();
		set_argons(reserves_account, 1_000_000_000);
		set_argons(42, 0);
		set_pending_operational_rewards(vec![OperationalRewardPayout {
			operational_account: 99,
			payout_account: 42,
			reward_kind: OperationalRewardKind::Activation,
			amount: 500_000_000,
		}]);

		<Treasury as OnNewSlot<u64>>::on_frame_start(2);

		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![TreasuryCapital { vault_id: 1, activated_capital: 100_000_000u128, frame_id: 2 }],
		);
		assert_eq!(pending_operational_rewards(), vec![]);
		assert_eq!(take_paid_operational_rewards().len(), 1);
		assert_eq!(Balances::free_balance(42), 500_000_000);
	});
}

#[test]
fn test_on_frame_start_defers_rewards_enqueued_during_transition_until_next_frame() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		reset_treasury_pool_participated();
		set_pending_operational_rewards(vec![]);

		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: 1_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(10, 1_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(10), 1, 100_000_000));

		let reserves_account = Treasury::get_treasury_reserves_account();
		set_argons(reserves_account, 1_000_000_000);
		set_argons(42, 0);
		queue_treasury_participation_rewards(vec![OperationalRewardPayout {
			operational_account: 99,
			payout_account: 42,
			reward_kind: OperationalRewardKind::Activation,
			amount: 500_000_000,
		}]);

		<Treasury as OnNewSlot<u64>>::on_frame_start(2);

		assert!(take_paid_operational_rewards().is_empty());
		assert_eq!(Balances::free_balance(42), 0);
		assert_eq!(pending_operational_rewards().len(), 1);
		CapitalActive::<Test>::kill();
		VaultPoolsByFrame::<Test>::remove(2);

		<Treasury as OnNewSlot<u64>>::on_frame_start(3);

		assert_eq!(take_paid_operational_rewards().len(), 1);
		assert_eq!(pending_operational_rewards(), vec![]);
		assert_eq!(Balances::free_balance(42), 500_000_000);
	});
}

#[test]
fn test_try_pay_operational_reward_pays_immediately_when_funded() {
	new_test_ext().execute_with(|| {
		let reserves_account = Treasury::get_treasury_reserves_account();
		set_argons(reserves_account, 1_000_000);
		set_argons(42, 0);

		let reward = OperationalRewardPayout {
			operational_account: 99,
			payout_account: 42,
			reward_kind: OperationalRewardKind::Activation,
			amount: 250_000,
		};

		let paid = <Treasury as OperationalRewardsPayer<u64, Balance>>::try_pay_reward(&reward);
		assert!(paid);
		assert_eq!(Balances::free_balance(42), 250_000);
	});
}

#[test]
fn test_try_pay_operational_reward_skips_when_insufficient() {
	new_test_ext().execute_with(|| {
		let reserves_account = Treasury::get_treasury_reserves_account();
		set_argons(reserves_account, 10);
		set_argons(42, 0);

		let reward = OperationalRewardPayout {
			operational_account: 99,
			payout_account: 42,
			reward_kind: OperationalRewardKind::Activation,
			amount: 250,
		};

		let paid = <Treasury as OperationalRewardsPayer<u64, Balance>>::try_pay_reward(&reward);
		assert!(!paid);
		assert_eq!(Balances::free_balance(42), 0);
	});
}

#[test]
fn test_operational_rewards_are_partially_paid_when_insufficient_funds() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		set_pending_operational_rewards(vec![]);
		let reserves_account = Treasury::get_treasury_reserves_account();

		set_argons(reserves_account, 50_000_000);

		let reward = OperationalRewardPayout {
			operational_account: 99,
			payout_account: 42,
			reward_kind: OperationalRewardKind::Activation,
			amount: 250_000_000,
		};

		set_pending_operational_rewards(vec![reward.clone()]);
		Treasury::pay_operational_rewards(pending_operational_rewards());

		let mut expected_paid = reward.clone();
		expected_paid.amount = 49_999_999;
		assert_eq!(take_paid_operational_rewards(), vec![expected_paid]);
		assert!(pending_operational_rewards().is_empty());
		assert_eq!(Balances::free_balance(42), 49_999_999);
		assert_eq!(Balances::free_balance(reserves_account), 1);
	});
}

#[test]
fn test_operational_rewards_prorata_uses_all_queued_accounts() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		set_pending_operational_rewards(vec![]);
		let reserves_account = Treasury::get_treasury_reserves_account();
		set_argons(reserves_account, 50);
		set_argons(42, 0);
		set_argons(43, 0);

		let reward_a = OperationalRewardPayout {
			operational_account: 99,
			payout_account: 42,
			reward_kind: OperationalRewardKind::Activation,
			amount: 80,
		};
		let reward_b = OperationalRewardPayout {
			operational_account: 100,
			payout_account: 43,
			reward_kind: OperationalRewardKind::Activation,
			amount: 20,
		};
		set_pending_operational_rewards(vec![reward_a.clone(), reward_b.clone()]);

		Treasury::pay_operational_rewards(pending_operational_rewards());

		let mut paid_a = reward_a.clone();
		paid_a.amount = 39;
		let mut paid_b = reward_b.clone();
		paid_b.amount = 9;
		assert_eq!(take_paid_operational_rewards(), vec![paid_a, paid_b]);
		assert!(pending_operational_rewards().is_empty());
		assert_eq!(Balances::free_balance(42), 39);
		assert_eq!(Balances::free_balance(43), 9);
		assert_eq!(Balances::free_balance(reserves_account), 2);
	});
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct DistributeScenario {
	frame_id: FrameId,
	bid_pool_total: u128,
	bid_pool_distributed: u128,
	bid_pool_shares: u128,
	per_vault_gross_earnings: u128,

	v1_id: VaultId,
	v1_operator: u64,
	v1_kept_pct: u128,
	v1_cap_op: u128,
	v1_cap_2: u128,
	v1_cap_3: u128,
	v1_cap_total: u128,

	v2_id: VaultId,
	v2_operator: u64,
	v2_kept_pct: u128,
	v2_cap_4: u128,
	v2_cap_5: u128,
	v2_cap_total: u128,

	v1_vault_kept: u128,
	v1_contrib_pool: u128,
	v1_op_contrib: u128,
	v1_f2_contrib: u128,
	v1_f3_contrib: u128,

	v2_vault_kept: u128,
	v2_contrib_pool: u128,
	v2_f4_contrib: u128,
	v2_f5_contrib: u128,

	expected_v1_operator_delta: u128,
	expected_v2_operator_delta: u128,
}

fn pct(amount: u128, percent: u128) -> u128 {
	amount * percent / 100
}

fn pro_rata(pool: u128, part: u128, total: u128) -> u128 {
	pool * part / total
}

fn setup_distribute_scenario() -> DistributeScenario {
	System::set_block_number(1);
	CurrentFrameId::set(1);

	insert_vault(
		1,
		TestVault {
			account_id: 10,
			securitized_satoshis: 5_000_000_000,
			sharing_percent: Permill::from_percent(50),
			is_closed: false,
		},
	);
	insert_vault(
		2,
		TestVault {
			account_id: 11,
			securitized_satoshis: 5_000_000_000,
			sharing_percent: Permill::from_percent(60),
			is_closed: false,
		},
	);

	set_argons(Treasury::get_bid_pool_account(), 1_000_000_002);

	set_argons(10, 1_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(10), 1, 1_000_000_000));
	set_argons(2, 5_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 1_200_000_000));
	set_argons(3, 5_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 1, 2_800_000_000));

	set_argons(4, 5_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(4), 2, 2_500_000_000));
	set_argons(5, 5_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(5), 2, 2_500_000_000));

	const FRAME_ID: FrameId = 2;
	Treasury::lock_in_vault_capital(FRAME_ID);

	const BID_POOL_TOTAL: u128 = 1_000_000_002;
	const BID_POOL_DISTRIBUTED: u128 = 800_000_000;
	const BID_POOL_SHARES: u128 = 2;
	const PER_VAULT_GROSS_EARNINGS: u128 = BID_POOL_DISTRIBUTED / BID_POOL_SHARES;

	const V1_ID: VaultId = 1;
	const V1_OPERATOR: u64 = 10;
	const V1_KEPT_PCT: u128 = 50;
	const V1_CAPITAL_OP: u128 = 1_000_000_000;
	const V1_CAPITAL_2: u128 = 1_200_000_000;
	const V1_CAPITAL_3: u128 = 2_800_000_000;
	const V1_CAPITAL_TOTAL: u128 = V1_CAPITAL_OP + V1_CAPITAL_2 + V1_CAPITAL_3;

	const V2_ID: VaultId = 2;
	const V2_OPERATOR: u64 = 11;
	const V2_KEPT_PCT: u128 = 40;
	const V2_CAPITAL_4: u128 = 2_500_000_000;
	const V2_CAPITAL_5: u128 = 2_500_000_000;
	const V2_CAPITAL_TOTAL: u128 = V2_CAPITAL_4 + V2_CAPITAL_5;

	let v1_vault_kept = pct(PER_VAULT_GROSS_EARNINGS, V1_KEPT_PCT);
	let v1_contrib_pool = PER_VAULT_GROSS_EARNINGS - v1_vault_kept;
	let v1_op_contrib = pro_rata(v1_contrib_pool, V1_CAPITAL_OP, V1_CAPITAL_TOTAL);
	let v1_f2_contrib = pro_rata(v1_contrib_pool, V1_CAPITAL_2, V1_CAPITAL_TOTAL);
	let v1_f3_contrib = pro_rata(v1_contrib_pool, V1_CAPITAL_3, V1_CAPITAL_TOTAL);

	let v2_vault_kept = pct(PER_VAULT_GROSS_EARNINGS, V2_KEPT_PCT);
	let v2_contrib_pool = PER_VAULT_GROSS_EARNINGS - v2_vault_kept;
	let v2_f4_contrib = pro_rata(v2_contrib_pool, V2_CAPITAL_4, V2_CAPITAL_TOTAL);
	let v2_f5_contrib = pro_rata(v2_contrib_pool, V2_CAPITAL_5, V2_CAPITAL_TOTAL);

	let expected_v1_operator_delta = v1_vault_kept + v1_op_contrib;
	let expected_v2_operator_delta = v2_vault_kept;

	DistributeScenario {
		frame_id: FRAME_ID,
		bid_pool_total: BID_POOL_TOTAL,
		bid_pool_distributed: BID_POOL_DISTRIBUTED,
		bid_pool_shares: BID_POOL_SHARES,
		per_vault_gross_earnings: PER_VAULT_GROSS_EARNINGS,
		v1_id: V1_ID,
		v1_operator: V1_OPERATOR,
		v1_kept_pct: V1_KEPT_PCT,
		v1_cap_op: V1_CAPITAL_OP,
		v1_cap_2: V1_CAPITAL_2,
		v1_cap_3: V1_CAPITAL_3,
		v1_cap_total: V1_CAPITAL_TOTAL,
		v2_id: V2_ID,
		v2_operator: V2_OPERATOR,
		v2_kept_pct: V2_KEPT_PCT,
		v2_cap_4: V2_CAPITAL_4,
		v2_cap_5: V2_CAPITAL_5,
		v2_cap_total: V2_CAPITAL_TOTAL,
		v1_vault_kept,
		v1_contrib_pool,
		v1_op_contrib,
		v1_f2_contrib,
		v1_f3_contrib,
		v2_vault_kept,
		v2_contrib_pool,
		v2_f4_contrib,
		v2_f5_contrib,
		expected_v1_operator_delta,
		expected_v2_operator_delta,
	}
}

#[test]
fn test_distribute_bid_pool_emits_event_and_records_profits() {
	new_test_ext().execute_with(|| {
		let s = setup_distribute_scenario();

		System::reset_events();
		Treasury::distribute_bid_pool(s.frame_id);

		System::assert_has_event(
			Event::<Test>::BidPoolDistributed {
				frame_id: s.frame_id,
				bid_pool_distributed: s.bid_pool_distributed,
				bid_pool_shares: s.bid_pool_shares as u32,
				treasury_reserves: s.bid_pool_total - s.bid_pool_distributed - 1,
			}
			.into(),
		);

		let profits = LastVaultProfits::get();
		assert_eq!(profits.len(), 2);

		let v1 = profits.iter().find(|profit| profit.vault_id == s.v1_id).unwrap();
		assert_eq!(
			v1,
			&VaultTreasuryFrameEarnings {
				vault_id: s.v1_id,
				vault_operator_account_id: s.v1_operator,
				earnings: s.per_vault_gross_earnings,
				earnings_for_vault: s.expected_v1_operator_delta,
				capital_contributed_by_vault: s.v1_cap_op,
				frame_id: s.frame_id,
				capital_contributed: s.v1_cap_total,
			}
		);

		let v2 = profits.iter().find(|profit| profit.vault_id == s.v2_id).unwrap();
		assert_eq!(
			v2,
			&VaultTreasuryFrameEarnings {
				vault_id: s.v2_id,
				vault_operator_account_id: s.v2_operator,
				earnings: s.per_vault_gross_earnings,
				earnings_for_vault: s.expected_v2_operator_delta,
				capital_contributed_by_vault: 0,
				frame_id: s.frame_id,
				capital_contributed: s.v2_cap_total,
			}
		);

		let pools = VaultPoolsByFrame::<Test>::get(s.frame_id);
		assert_eq!(
			pools.get(&s.v1_id).unwrap().distributed_earnings,
			Some(s.per_vault_gross_earnings)
		);
		assert_eq!(
			pools.get(&s.v2_id).unwrap().distributed_earnings,
			Some(s.per_vault_gross_earnings)
		);
	});
}

#[test]
fn test_distribute_bid_pool_updates_contributor_balances() {
	new_test_ext().execute_with(|| {
		let s = setup_distribute_scenario();
		let hold_reason = HoldReason::ContributedToTreasury;

		let pre_2 = Balances::free_balance(2) + Balances::balance_on_hold(&hold_reason.into(), &2);
		let pre_3 = Balances::free_balance(3) + Balances::balance_on_hold(&hold_reason.into(), &3);
		let pre_4 = Balances::free_balance(4) + Balances::balance_on_hold(&hold_reason.into(), &4);
		let pre_5 = Balances::free_balance(5) + Balances::balance_on_hold(&hold_reason.into(), &5);

		Treasury::distribute_bid_pool(s.frame_id);

		let post_2 = Balances::free_balance(2) + Balances::balance_on_hold(&hold_reason.into(), &2);
		let post_3 = Balances::free_balance(3) + Balances::balance_on_hold(&hold_reason.into(), &3);
		let post_4 = Balances::free_balance(4) + Balances::balance_on_hold(&hold_reason.into(), &4);
		let post_5 = Balances::free_balance(5) + Balances::balance_on_hold(&hold_reason.into(), &5);

		assert_eq!(post_2, pre_2 + s.v1_f2_contrib);
		assert_eq!(post_3, pre_3 + s.v1_f3_contrib);
		assert_eq!(post_4, pre_4 + s.v2_f4_contrib);
		assert_eq!(post_5, pre_5 + s.v2_f5_contrib);
	});
}

#[test]
fn test_distribute_bid_pool_updates_funder_state_principal_for_contributors() {
	new_test_ext().execute_with(|| {
		let s = setup_distribute_scenario();

		let fs_op_pre = FunderStateByVaultAndAccount::<Test>::get(s.v1_id, s.v1_operator)
			.expect("operator state");
		let fs_2_pre =
			FunderStateByVaultAndAccount::<Test>::get(s.v1_id, 2).expect("funder 2 state");
		let fs_3_pre =
			FunderStateByVaultAndAccount::<Test>::get(s.v1_id, 3).expect("funder 3 state");
		let fs_4_pre =
			FunderStateByVaultAndAccount::<Test>::get(s.v2_id, 4).expect("funder 4 state");
		let fs_5_pre =
			FunderStateByVaultAndAccount::<Test>::get(s.v2_id, 5).expect("funder 5 state");

		assert_eq!(fs_op_pre.held_principal, s.v1_cap_op);
		assert_eq!(fs_2_pre.held_principal, s.v1_cap_2);
		assert_eq!(fs_3_pre.held_principal, s.v1_cap_3);
		assert_eq!(fs_4_pre.held_principal, s.v2_cap_4);
		assert_eq!(fs_5_pre.held_principal, s.v2_cap_5);

		Treasury::distribute_bid_pool(s.frame_id);

		let fs_op_post = FunderStateByVaultAndAccount::<Test>::get(s.v1_id, s.v1_operator)
			.expect("operator state post");
		let fs_2_post =
			FunderStateByVaultAndAccount::<Test>::get(s.v1_id, 2).expect("funder 2 state post");
		let fs_3_post =
			FunderStateByVaultAndAccount::<Test>::get(s.v1_id, 3).expect("funder 3 state post");
		let fs_4_post =
			FunderStateByVaultAndAccount::<Test>::get(s.v2_id, 4).expect("funder 4 state post");
		let fs_5_post =
			FunderStateByVaultAndAccount::<Test>::get(s.v2_id, 5).expect("funder 5 state post");

		assert_eq!(fs_op_post.held_principal, fs_op_pre.held_principal);
		assert_eq!(fs_2_post.held_principal, fs_2_pre.held_principal + s.v1_f2_contrib);
		assert_eq!(fs_3_post.held_principal, fs_3_pre.held_principal + s.v1_f3_contrib);
		assert_eq!(fs_4_post.held_principal, fs_4_pre.held_principal + s.v2_f4_contrib);
		assert_eq!(fs_5_post.held_principal, fs_5_pre.held_principal + s.v2_f5_contrib);
	});
}

#[test]
fn test_distribute_bid_pool_updates_lifetime_counters_for_contributors() {
	new_test_ext().execute_with(|| {
		let s = setup_distribute_scenario();
		let vault_id = s.v1_id;
		let account_id: u64 = 2;

		let fs_pre =
			FunderStateByVaultAndAccount::<Test>::get(vault_id, account_id).expect("funder state");
		let held_principal_pre = fs_pre.held_principal;
		let lifetime_earnings_pre = fs_pre.lifetime_compounded_earnings;

		FunderStateByVaultAndAccount::<Test>::mutate(vault_id, account_id, |state| {
			let funder_state = state.as_mut().unwrap();
			funder_state.lifetime_principal_last_basis_frame = 1;
			funder_state.lifetime_principal_deployed = 0;
		});

		Treasury::distribute_bid_pool(s.frame_id);

		let fs_post = FunderStateByVaultAndAccount::<Test>::get(vault_id, account_id)
			.expect("funder state post");
		assert_eq!(fs_post.lifetime_compounded_earnings, lifetime_earnings_pre + s.v1_f2_contrib);
		assert_eq!(fs_post.lifetime_principal_deployed, held_principal_pre);
		assert_eq!(fs_post.lifetime_principal_last_basis_frame, s.frame_id);
	});
}

#[test]
fn test_distribute_bid_pool_pays_exited_contributor_as_free_balance() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		MinimumArgonsPerContributor::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		insert_vault(
			1,
			TestVault {
				account_id: 10,
				securitized_satoshis: 1_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		set_argons(2, 1_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100));
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 0));

		let mut pool = TreasuryPool::<Test>::new(1);
		pool.try_insert_bond_holder(2, 100, Some(&10)).unwrap();
		let mut pools = BoundedBTreeMap::new();
		pools.try_insert(1, pool).unwrap();
		VaultPoolsByFrame::<Test>::insert(10, pools);
		let active_capital: BoundedVec<TreasuryCapital<Test>, MaxVaultsPerPool> =
			vec![TreasuryCapital { vault_id: 1, activated_capital: 100u128, frame_id: 10 }]
				.try_into()
				.unwrap();
		CapitalActive::<Test>::put(active_capital);
		set_argons(Treasury::get_bid_pool_account(), 1_000);

		Treasury::release_pending_unlocks(11);
		assert!(FunderStateByVaultAndAccount::<Test>::get(1, 2).is_none());
		assert!(FundersByVaultId::<Test>::get(1).is_empty());
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 0);

		let pre_free_balance = Balances::free_balance(2);
		Treasury::distribute_bid_pool(10);

		assert!(FunderStateByVaultAndAccount::<Test>::get(1, 2).is_none());
		assert!(FundersByVaultId::<Test>::get(1).is_empty());
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 0);
		assert_eq!(Balances::free_balance(2), pre_free_balance + 400);
	});
}
