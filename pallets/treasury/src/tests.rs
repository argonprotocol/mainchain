use crate::{
	Error, Event, FunderStateByVaultAndAccount, FundersByVaultId, HoldReason, TreasuryCapital,
	TreasuryPool,
	mock::{Treasury, *},
	pallet::{CapitalActive, VaultPoolsByFrame},
};
use argon_primitives::{OperationalRewardKind, OperationalRewardPayout, OperationalRewardsPayer};
use frame_support::{assert_err, assert_ok, traits::fungible::InspectHold};
use pallet_prelude::{argon_primitives::vault::VaultTreasuryFrameEarnings, *};
use sp_runtime::Permill;

#[test]
fn test_can_add_pool_capital() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		let hold_reason = HoldReason::ContributedToTreasury;

		set_argons(1, 500_000_000);
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
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().target_principal,
			200_000_000
		);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal,
			200_000_000
		);
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().bonded_principal, 0);
		assert_eq!(FundersByVaultId::<Test>::get(1).len(), 1);

		// increase allocation
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
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().target_principal,
			300_000_000
		);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal,
			300_000_000
		);
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().bonded_principal, 0);
		assert_eq!(FundersByVaultId::<Test>::get(1).len(), 1);

		// can decrease allocation
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 100_000_000));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 100_000_000);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().target_principal,
			100_000_000
		);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal,
			100_000_000
		);
		assert_eq!(FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().bonded_principal, 0);

		// can't go below bonded
		FunderStateByVaultAndAccount::<Test>::mutate(1, 2, |funder_state_opt| {
			let funder_state = funder_state_opt.as_mut().unwrap();
			funder_state.bonded_principal = 80_000_000;
		});
		MinimumArgonsPerContributor::set(500);
		System::reset_events();
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 50_000_000));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 80_000_000);
		System::assert_has_event(
			Event::<Test>::RefundedTreasuryCapital {
				vault_id: 1,
				account_id: 2,
				amount: 20_000_000,
				frame_id: 1,
			}
			.into(),
		);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().target_principal,
			50_000_000
		);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().held_principal,
			80_000_000
		);
		assert_eq!(
			FunderStateByVaultAndAccount::<Test>::get(1, 2).unwrap().bonded_principal,
			80_000_000
		);

		FunderStateByVaultAndAccount::<Test>::mutate(1, 2, |funder_state_opt| {
			let funder_state = funder_state_opt.as_mut().unwrap();
			funder_state.bonded_principal = 0;
		});
		// can remove all allocation
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 0));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 0);
		assert!(FunderStateByVaultAndAccount::<Test>::get(1, 2).is_none());
		assert_eq!(FundersByVaultId::<Test>::get(1).len(), 0);
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
fn test_set_allocation_max_contributors_exceeded() {
	MaxTreasuryContributors::set(1);
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

		assert_err!(
			Treasury::set_allocation(RuntimeOrigin::signed(3), 1, 100),
			Error::<Test>::MaxContributorsExceeded
		);
		// Ensure no hold was created for the rejected account.
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &3), 0);
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
		assert!(FunderStateByVaultAndAccount::<Test>::get(1, 2).is_some());
		assert!(FunderStateByVaultAndAccount::<Test>::get(2, 2).is_some());
		assert_eq!(FundersByVaultId::<Test>::get(1).len(), 1);
		assert_eq!(FundersByVaultId::<Test>::get(2).len(), 1);

		// Removing vault 1 allocation should not affect vault 2.
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 0));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 100_000_000);
		assert!(FunderStateByVaultAndAccount::<Test>::get(1, 2).is_none());
		assert!(FunderStateByVaultAndAccount::<Test>::get(2, 2).is_some());
		assert_eq!(FundersByVaultId::<Test>::get(1).len(), 0);
		assert_eq!(FundersByVaultId::<Test>::get(2).len(), 1);
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

		set_argons(Treasury::get_bid_pool_account(), 1_000_000_000);

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
				TreasuryCapital { vault_id: 1, activated_capital: 500_000_000u128, frame_id: 2 },
				TreasuryCapital { vault_id: 2, activated_capital: 450_000_000u128, frame_id: 2 },
			],
			"sorted with biggest share first"
		);

		System::assert_last_event(
			Event::<Test>::NextBidPoolCapitalLocked {
				frame_id: 2,
				participating_vaults: 2,
				total_activated_capital: 950_000_000,
			}
			.into(),
		);
	});
}

#[test]
fn test_lock_in_respects_max_vaults_per_pool() {
	MaxVaultsPerPool::set(2); // Remove or comment out if not a storage item
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		// Three open vaults
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

		// Give each vault a different raised amount so ordering is deterministic.
		set_argons(2, 10_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 5_000_000_000)); // 500m per frame
		set_argons(3, 10_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 2, 3_000_000_000)); // 300m per frame
		set_argons(4, 10_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(4), 3, 1_000_000_000)); // 100m per frame

		Treasury::lock_in_vault_capital(2);
		let active = CapitalActive::<Test>::get().into_inner();
		assert_eq!(active.len(), 2, "must be truncated to MaxVaultsPerPool");
		assert_eq!(
			active,
			vec![
				TreasuryCapital { vault_id: 1, activated_capital: 500_000_000u128, frame_id: 2 },
				TreasuryCapital { vault_id: 2, activated_capital: 300_000_000u128, frame_id: 2 },
			],
			"should keep the top 2 vaults by activated capital"
		);
		System::assert_last_event(
			Event::<Test>::NextBidPoolCapitalLocked {
				frame_id: 2,
				participating_vaults: 2,
				total_activated_capital: 800_000_000,
			}
			.into(),
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
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 5_000_000_000)); // would be 500m per frame

		Treasury::lock_in_vault_capital(2);
		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![TreasuryCapital { vault_id: 1, activated_capital: 100_000_000u128, frame_id: 2 }],
			"activated capital must be capped by vault securitized satoshis"
		);
		System::assert_last_event(
			Event::<Test>::NextBidPoolCapitalLocked {
				frame_id: 2,
				participating_vaults: 1,
				total_activated_capital: 100_000_000,
			}
			.into(),
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

		let vault_satoshis = 77_777;
		set_vault_securitized_satoshis(1, vault_satoshis);

		set_argons(2, 10_000_000_000);
		assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 5_000_000_000));

		Treasury::lock_in_vault_capital(2);
		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![TreasuryCapital { vault_id: 1, activated_capital: 7_777u128, frame_id: 2 }],
			"activated capital must be capped by securitized satoshis"
		);
		System::assert_last_event(
			Event::<Test>::NextBidPoolCapitalLocked {
				frame_id: 2,
				participating_vaults: 1,
				total_activated_capital: 7_777,
			}
			.into(),
		);
	});
}

#[test]
fn test_treasury_pool_participated_only_on_first_operator_bond() {
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
		assert_eq!(take_treasury_pool_participated(), vec![(10, 10_000_000u128)]);

		Treasury::lock_in_vault_capital(3);
		assert!(take_treasury_pool_participated().is_empty());
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
		Treasury::pay_operational_rewards();

		assert_eq!(Balances::free_balance(42), 250_000_000);
		assert_eq!(Balances::free_balance(reserves_account), 750_000_000);
		assert_eq!(take_paid_operational_rewards(), vec![reward]);
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
		Treasury::pay_operational_rewards();

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

		Treasury::pay_operational_rewards();

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

#[test]
fn test_operational_rewards_with_no_funds_are_consumed_with_zero_payout() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		set_pending_operational_rewards(vec![]);
		let reserves_account = Treasury::get_treasury_reserves_account();
		set_argons(reserves_account, 0);
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
		set_pending_operational_rewards(vec![reward_a, reward_b]);

		Treasury::pay_operational_rewards();

		assert!(take_paid_operational_rewards().is_empty());
		assert!(pending_operational_rewards().is_empty());
		assert_eq!(Balances::free_balance(42), 0);
		assert_eq!(Balances::free_balance(43), 0);
		assert_eq!(Balances::free_balance(reserves_account), 0);
	});
}

#[test]
fn test_operational_rewards_with_one_microgon_and_three_accounts_zeroes_everyone() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		set_pending_operational_rewards(vec![]);
		let reserves_account = Treasury::get_treasury_reserves_account();
		set_argons(reserves_account, 1);
		set_argons(42, 0);
		set_argons(43, 0);
		set_argons(44, 0);

		let reward_a = OperationalRewardPayout {
			operational_account: 99,
			payout_account: 42,
			reward_kind: OperationalRewardKind::Activation,
			amount: 1,
		};
		let reward_b = OperationalRewardPayout {
			operational_account: 100,
			payout_account: 43,
			reward_kind: OperationalRewardKind::Activation,
			amount: 1,
		};
		let reward_c = OperationalRewardPayout {
			operational_account: 101,
			payout_account: 44,
			reward_kind: OperationalRewardKind::Activation,
			amount: 1,
		};
		set_pending_operational_rewards(vec![reward_a, reward_b, reward_c]);

		Treasury::pay_operational_rewards();

		assert!(take_paid_operational_rewards().is_empty());
		assert!(pending_operational_rewards().is_empty());
		assert_eq!(Balances::free_balance(42), 0);
		assert_eq!(Balances::free_balance(43), 0);
		assert_eq!(Balances::free_balance(44), 0);
		assert_eq!(Balances::free_balance(reserves_account), 1);
	});
}

#[test]
fn test_pay_operational_rewards_processes_all_queued_when_funded() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		set_pending_operational_rewards(vec![]);
		let reserves_account = Treasury::get_treasury_reserves_account();
		set_argons(reserves_account, 1_000_000_000_000);

		let rewards = (0..=3u64)
			.map(|i| OperationalRewardPayout {
				operational_account: 1_000 + i,
				payout_account: 2_000 + i,
				reward_kind: OperationalRewardKind::Activation,
				amount: 1,
			})
			.collect::<Vec<_>>();
		set_pending_operational_rewards(rewards);

		Treasury::pay_operational_rewards();

		assert_eq!(take_paid_operational_rewards().len(), 4);
		assert!(pending_operational_rewards().is_empty());
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

	// Two vaults.
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

	// Bid pool.
	set_argons(Treasury::get_bid_pool_account(), 1_000_000_002);

	// Vault 1 allocations (operator + contributors).
	set_argons(10, 1_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(10), 1, 1_000_000_000));

	set_argons(2, 5_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(2), 1, 1_200_000_000));
	set_argons(3, 5_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(3), 1, 2_800_000_000));

	// Vault 2 allocations (contributors only).
	set_argons(4, 5_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(4), 2, 2_500_000_000));
	set_argons(5, 5_000_000_000);
	assert_ok!(Treasury::set_allocation(RuntimeOrigin::signed(5), 2, 2_500_000_000));

	// Lock in capital for next frame.
	const FRAME_ID: FrameId = 2;
	Treasury::lock_in_vault_capital(FRAME_ID);

	// Scenario constants.
	const BID_POOL_TOTAL: u128 = 1_000_000_002;
	const BID_POOL_DISTRIBUTED: u128 = 800_000_000;
	const BID_POOL_SHARES: u128 = 2;
	const PER_VAULT_GROSS_EARNINGS: u128 = BID_POOL_DISTRIBUTED / BID_POOL_SHARES; // 400m each

	// Vault 1
	const V1_ID: VaultId = 1;
	const V1_OPERATOR: u64 = 10;
	const V1_KEPT_PCT: u128 = 50;
	const V1_CAPITAL_OP: u128 = 100_000_000;
	const V1_CAPITAL_2: u128 = 120_000_000;
	const V1_CAPITAL_3: u128 = 280_000_000;
	const V1_CAPITAL_TOTAL: u128 = V1_CAPITAL_OP + V1_CAPITAL_2 + V1_CAPITAL_3;

	// Vault 2
	const V2_ID: VaultId = 2;
	const V2_OPERATOR: u64 = 11;
	const V2_KEPT_PCT: u128 = 40;
	const V2_CAPITAL_4: u128 = 250_000_000;
	const V2_CAPITAL_5: u128 = 250_000_000;
	const V2_CAPITAL_TOTAL: u128 = V2_CAPITAL_4 + V2_CAPITAL_5;

	// Expected earnings math.
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
		assert_eq!(profits.len(), 2, "should have 2 vault profits recorded");

		let v1 = profits.iter().find(|p| p.vault_id == s.v1_id).unwrap();
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

		let v2 = profits.iter().find(|p| p.vault_id == s.v2_id).unwrap();
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

		// Distributed earnings are recorded per vault.
		let pools = VaultPoolsByFrame::<Test>::get(s.frame_id);
		assert_eq!(
			pools.get(&s.v1_id).unwrap().distributed_earnings,
			Some(s.per_vault_gross_earnings)
		);
		assert_eq!(
			pools.get(&s.v2_id).unwrap().distributed_earnings,
			Some(s.per_vault_gross_earnings)
		);

		// 1 unit dust remains after reserves are siphoned. In mock runtimes using `u64` account
		// ids, the treasury reserves and bid pool accounts can collide due to truncation.
		let bid_pool_account = Treasury::get_bid_pool_account();
		let reserves_account = Treasury::get_treasury_reserves_account();
		let expected_reserves = s.bid_pool_total - s.bid_pool_distributed - 1;
		if bid_pool_account == reserves_account {
			assert_eq!(Balances::free_balance(bid_pool_account), expected_reserves + 1);
		} else {
			assert_eq!(Balances::free_balance(bid_pool_account), 1);
			assert_eq!(Balances::free_balance(reserves_account), expected_reserves);
		}
	});
}

#[test]
fn test_distribute_bid_pool_preserves_pool_accounts() {
	new_test_ext().execute_with(|| {
		let s = setup_distribute_scenario();

		Treasury::distribute_bid_pool(s.frame_id);

		let bid_pool_account = Treasury::get_bid_pool_account();
		let reserves_account = Treasury::get_treasury_reserves_account();
		assert!(System::providers(&bid_pool_account) > 0);
		assert!(System::providers(&reserves_account) > 0);
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

		let pre_op_10 = Balances::free_balance(s.v1_operator) +
			Balances::balance_on_hold(&hold_reason.into(), &s.v1_operator);
		let pre_op_11 = Balances::free_balance(s.v2_operator) +
			Balances::balance_on_hold(&hold_reason.into(), &s.v2_operator);

		Treasury::distribute_bid_pool(s.frame_id);

		let post_2 = Balances::free_balance(2) + Balances::balance_on_hold(&hold_reason.into(), &2);
		let post_3 = Balances::free_balance(3) + Balances::balance_on_hold(&hold_reason.into(), &3);
		let post_4 = Balances::free_balance(4) + Balances::balance_on_hold(&hold_reason.into(), &4);
		let post_5 = Balances::free_balance(5) + Balances::balance_on_hold(&hold_reason.into(), &5);

		let post_op_10 = Balances::free_balance(s.v1_operator) +
			Balances::balance_on_hold(&hold_reason.into(), &s.v1_operator);
		let post_op_11 = Balances::free_balance(s.v2_operator) +
			Balances::balance_on_hold(&hold_reason.into(), &s.v2_operator);

		assert_eq!(post_2, pre_2 + s.v1_f2_contrib);
		assert_eq!(post_3, pre_3 + s.v1_f3_contrib);
		assert_eq!(post_4, pre_4 + s.v2_f4_contrib);
		assert_eq!(post_5, pre_5 + s.v2_f5_contrib);

		// Operators are paid via the vault-provider path; balances here should be unchanged.
		assert_eq!(post_op_10, pre_op_10);
		assert_eq!(post_op_11, pre_op_11);
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

		assert_eq!(fs_op_pre.bonded_principal, s.v1_cap_op);
		assert_eq!(fs_2_pre.bonded_principal, s.v1_cap_2);
		assert_eq!(fs_3_pre.bonded_principal, s.v1_cap_3);
		assert_eq!(fs_4_pre.bonded_principal, s.v2_cap_4);
		assert_eq!(fs_5_pre.bonded_principal, s.v2_cap_5);

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

		// Operator does not auto-roll into principal.
		assert_eq!(fs_op_post.bonded_principal, fs_op_pre.bonded_principal);
		assert_eq!(fs_op_post.target_principal, fs_op_pre.target_principal);
		assert_eq!(fs_op_post.held_principal, fs_op_pre.held_principal);

		// Contributors auto-roll earnings into held + target; bonded is unchanged.
		assert_eq!(fs_2_post.bonded_principal, fs_2_pre.bonded_principal);
		assert_eq!(fs_2_post.target_principal, fs_2_pre.target_principal + s.v1_f2_contrib);
		assert_eq!(fs_2_post.held_principal, fs_2_pre.held_principal + s.v1_f2_contrib);

		assert_eq!(fs_3_post.bonded_principal, fs_3_pre.bonded_principal);
		assert_eq!(fs_3_post.target_principal, fs_3_pre.target_principal + s.v1_f3_contrib);
		assert_eq!(fs_3_post.held_principal, fs_3_pre.held_principal + s.v1_f3_contrib);

		assert_eq!(fs_4_post.bonded_principal, fs_4_pre.bonded_principal);
		assert_eq!(fs_4_post.target_principal, fs_4_pre.target_principal + s.v2_f4_contrib);
		assert_eq!(fs_4_post.held_principal, fs_4_pre.held_principal + s.v2_f4_contrib);

		assert_eq!(fs_5_post.bonded_principal, fs_5_pre.bonded_principal);
		assert_eq!(fs_5_post.target_principal, fs_5_pre.target_principal + s.v2_f5_contrib);
		assert_eq!(fs_5_post.held_principal, fs_5_pre.held_principal + s.v2_f5_contrib);
	});
}

#[test]
fn test_distribute_bid_pool_updates_lifetime_counters_for_contributors() {
	new_test_ext().execute_with(|| {
		let s = setup_distribute_scenario();

		// Choose a non-operator funder with deterministic state.
		let vault_id = s.v1_id;
		let account_id: u64 = 2;

		// Snapshot pre-state.
		let fs_pre =
			FunderStateByVaultAndAccount::<Test>::get(vault_id, account_id).expect("funder state");
		let held_pre = fs_pre.held_principal;
		let lifetime_earnings_pre = fs_pre.lifetime_compounded_earnings;

		// Make basis deterministic: accrue exactly 1 frame (1 -> 2) on pre-distribution held
		// principal.
		FunderStateByVaultAndAccount::<Test>::mutate(vault_id, account_id, |state| {
			let s = state.as_mut().unwrap();
			s.lifetime_principal_last_basis_frame = 1;
			s.lifetime_principal_deployed = 0;
		});

		Treasury::distribute_bid_pool(s.frame_id);

		let fs_post = FunderStateByVaultAndAccount::<Test>::get(vault_id, account_id)
			.expect("funder state post");

		// Lifetime earnings should accumulate exactly the pro-rata contributor earnings.
		assert_eq!(fs_post.lifetime_compounded_earnings, lifetime_earnings_pre + s.v1_f2_contrib);

		// Basis should accrue one frame of deployed principal using the *pre* distribution held
		// principal.
		assert_eq!(fs_post.lifetime_principal_deployed, held_pre);
		assert_eq!(fs_post.lifetime_principal_last_basis_frame, s.frame_id);
	});
}

#[test]
fn test_distribute_bid_pool_lifetime_counters_accrue_across_two_frames() {
	new_test_ext().execute_with(|| {
		let s = setup_distribute_scenario();

		// Pick a non-operator funder.
		let vault_id = s.v1_id;
		let account_id: u64 = 2;

		// Pre-state before first distribution (frame 2).
		let fs0 =
			FunderStateByVaultAndAccount::<Test>::get(vault_id, account_id).expect("funder state");
		let held0 = fs0.held_principal;
		let lifetime_earnings0 = fs0.lifetime_compounded_earnings;

		// Force deterministic basis accrual for the first step: accrue exactly 1 frame (1 -> 2).
		FunderStateByVaultAndAccount::<Test>::mutate(vault_id, account_id, |state| {
			let s = state.as_mut().unwrap();
			s.lifetime_principal_last_basis_frame = 1;
			s.lifetime_principal_deployed = 0;
		});

		Treasury::distribute_bid_pool(2);

		let fs1 = FunderStateByVaultAndAccount::<Test>::get(vault_id, account_id)
			.expect("funder state after frame 2");
		let held1 = fs1.held_principal;
		let lifetime_earnings1 = fs1.lifetime_compounded_earnings;
		let deployed1 = fs1.lifetime_principal_deployed;

		// After frame 2: basis is exactly one frame of the pre-distribution held principal.
		assert_eq!(deployed1, held0);
		assert_eq!(fs1.lifetime_principal_last_basis_frame, 2);
		assert!(lifetime_earnings1 >= lifetime_earnings0);

		// Prepare frame 3: top up bid pool and lock-in for next frame.
		System::set_block_number(2);
		CurrentFrameId::set(2);
		set_argons(Treasury::get_bid_pool_account(), 1_000_000_002);
		Treasury::lock_in_vault_capital(3);

		// Capture held just before the frame 3 distribution (this is the basis for 2 -> 3 accrual).
		let fs_before3 = FunderStateByVaultAndAccount::<Test>::get(vault_id, account_id)
			.expect("funder state before frame 3");
		let held_before3 = fs_before3.held_principal;
		let deployed_before3 = fs_before3.lifetime_principal_deployed;
		assert_eq!(deployed_before3, deployed1);
		assert_eq!(fs_before3.lifetime_principal_last_basis_frame, 2);

		Treasury::distribute_bid_pool(3);

		let fs2 = FunderStateByVaultAndAccount::<Test>::get(vault_id, account_id)
			.expect("funder state after frame 3");
		let held2 = fs2.held_principal;
		let lifetime_earnings2 = fs2.lifetime_compounded_earnings;
		let deployed2 = fs2.lifetime_principal_deployed;

		// Earnings should monotonically increase.
		assert!(lifetime_earnings2 >= lifetime_earnings1);

		// If earnings are auto-rolled for contributors, held increases by the same delta as
		// lifetime earnings for the step.
		let step_earnings_delta = lifetime_earnings2.saturating_sub(lifetime_earnings1);
		assert_eq!(held2.saturating_sub(held_before3), step_earnings_delta);

		// After frame 3: deployed basis should have added exactly one frame of held_before3 (2 ->
		// 3).
		assert_eq!(deployed2, held0 + held_before3);
		assert_eq!(fs2.lifetime_principal_last_basis_frame, 3);

		// Sanity: held should have grown (or stayed) across steps.
		assert!(held1 >= held0);
		assert!(held2 >= held_before3);
	});
}

#[test]
fn test_bond_holder_sort_order() {
	MaxTreasuryContributors::set(2);
	new_test_ext().execute_with(|| {
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				securitized_satoshis: 50_000_000_000,
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);

		let mut pool = TreasuryPool::<Test>::new(1);
		assert_eq!(pool.vault_sharing_percent, Permill::from_percent(10));

		// Insert two contributors.
		pool.try_insert_bond_holder(1, 50, Some(&1)).unwrap();
		pool.try_insert_bond_holder(2, 500, Some(&1)).unwrap();
		assert_eq!(pool.bond_holders.len(), 2);

		// Inserting a larger contributor should evict the smallest non-operator.
		pool.try_insert_bond_holder(3, 1000, Some(&1)).unwrap();
		assert_eq!(
			pool.bond_holders.to_vec(),
			vec![(3, 1000), (1, 50)],
			"Won't evict the vault operator"
		);
	});
}
