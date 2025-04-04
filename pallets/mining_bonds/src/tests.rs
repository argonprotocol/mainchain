use crate::{
	mock::{MiningBondFunds, *},
	pallet::{MiningBondFundsByCohort, NextVaultBidPoolCapital, OpenVaultBidPoolCapital},
	Error, Event, HoldReason, MiningBondFund, VaultBidPoolCapital,
};
use argon_primitives::{vault::MiningBidPoolProvider, OnNewSlot};
use frame_support::{assert_err, assert_ok, traits::fungible::InspectHold};
use sp_core::bounded_vec;
use sp_runtime::Permill;

#[test]
fn it_can_add_pool_capital() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let hold_reason = HoldReason::ContributedToBondFund;

		set_argons(2, 500_000_000);

		assert_err!(
			MiningBondFunds::add_capital(RuntimeOrigin::signed(2), 1, 200_000_000),
			Error::<Test>::VaultNotAcceptingMiningBonds
		);

		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 50_000_000_000,
				mining_bond_take: Permill::from_percent(10),
				is_closed: false,
			},
		);

		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(2), 1, 200_000_000));
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(2)
				.get(&1)
				.unwrap()
				.contributor_balances
				.clone()
				.into_inner(),
			vec![(2, 200_000_000)]
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 200_000_000);

		// ensure a second one sorts properly
		set_argons(3, 300_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(3), 1, 300_000_000));
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(2)
				.get(&1)
				.unwrap()
				.contributor_balances
				.clone()
				.into_inner(),
			vec![(3, 300_000_000), (2, 200_000_000)]
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &3), 300_000_000);

		// add a third in the middle
		set_argons(4, 250_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(4), 1, 250_000_000));
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(2)
				.get(&1)
				.unwrap()
				.contributor_balances
				.clone()
				.into_inner(),
			vec![(3, 300_000_000), (4, 250_000_000), (2, 200_000_000)]
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &4), 250_000_000);

		// now move the first bid
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(2), 1, 251_000_000));
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(2)
				.get(&1)
				.unwrap()
				.contributor_balances
				.clone()
				.into_inner(),
			vec![(3, 300_000_000), (2, 251_000_000), (4, 250_000_000)]
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &2), 251_000_000);
	});
}

#[test]
fn it_refunds_a_bounced_out_contributor() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 50_000_000_000,
				mining_bond_take: Permill::from_percent(10),
				is_closed: false,
			},
		);
		let hold_reason = HoldReason::ContributedToBondFund;

		for i in 1..=10 {
			let amount = 200_000_000u128 + i as u128;
			set_argons(i, amount);
			assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(i), 1, amount));
			assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &i), amount);
		}

		set_argons(11, 300_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(11), 1, 300_000_000));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &11), 300_000_000);

		// should have refunded the 1st contributor
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &1), 0);
	});
}

#[test]
fn it_can_lock_next_pool_capital() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000,
				mining_bond_take: Permill::from_percent(50),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000,
				mining_bond_take: Permill::from_percent(40),
				is_closed: false,
			},
		);

		set_argons(MiningBondFunds::get_bid_pool_account(), 1_000_000_000);

		set_argons(2, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(2), 1, 200_000_000));
		set_argons(3, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(3), 1, 300_000_000));

		set_argons(4, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(4), 2, 250_000_000));
		set_argons(5, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(5), 2, 200_000_000));

		MiningBondFunds::lock_next_bid_pool_capital(2);
		assert_eq!(
			OpenVaultBidPoolCapital::<Test>::get().into_inner(),
			vec![
				VaultBidPoolCapital {
					vault_id: 1,
					activated_capital: 500_000_000u128,
					cohort_id: 2
				},
				VaultBidPoolCapital {
					vault_id: 2,
					activated_capital: 450_000_000u128,
					cohort_id: 2
				},
			],
			"sorted with biggest share last"
		);

		System::assert_last_event(
			Event::<Test>::NextBidPoolCapitalLocked {
				cohort_id: 2,
				participating_vaults: 2,
				total_activated_capital: 950_000_000,
			}
			.into(),
		);
	});
}

#[test]
fn it_refunds_non_activated_funds_on_lock() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000,
				mining_bond_take: Permill::from_percent(50),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000,
				mining_bond_take: Permill::from_percent(40),
				is_closed: false,
			},
		);

		set_argons(MiningBondFunds::get_bid_pool_account(), 1_000_000_000);

		set_argons(2, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(2), 1, 220_000_000));
		set_argons(3, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(3), 1, 280_000_000));

		set_argons(4, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(4), 2, 250_000_000));
		set_argons(5, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(5), 2, 200_000_000));

		assert_eq!(
			NextVaultBidPoolCapital::<Test>::get().into_inner(),
			vec![
				VaultBidPoolCapital {
					vault_id: 1,
					activated_capital: 500_000_000u128,
					cohort_id: 2
				},
				VaultBidPoolCapital {
					vault_id: 2,
					activated_capital: 450_000_000u128,
					cohort_id: 2
				},
			],
			"sorted with biggest share last"
		);
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(2)
				.get(&1)
				.unwrap()
				.contributor_balances
				.clone()
				.into_inner(),
			vec![(3, 280_000_000), (2, 220_000_000)]
		);
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(2)
				.get(&2)
				.unwrap()
				.contributor_balances
				.clone()
				.into_inner(),
			vec![(4, 250_000_000), (5, 200_000_000),]
		);
		// now reduce activated
		VaultsById::mutate(|a| {
			a.get_mut(&1).unwrap().activated = 3_000_000_000;
			a.get_mut(&2).unwrap().activated = 4_000_000_000;
		});

		MiningBondFunds::lock_next_bid_pool_capital(2);
		assert_eq!(
			OpenVaultBidPoolCapital::<Test>::get().into_inner(),
			vec![
				VaultBidPoolCapital {
					vault_id: 2,
					activated_capital: 400_000_000u128,
					cohort_id: 2
				},
				VaultBidPoolCapital {
					vault_id: 1,
					activated_capital: 300_000_000u128,
					cohort_id: 2
				},
			],
			"sorted with biggest share last"
		);

		let cohort_capital = MiningBondFundsByCohort::<Test>::get(2);
		assert_eq!(
			cohort_capital.get(&1).unwrap().contributor_balances.clone().into_inner(),
			vec![(3, 280_000_000), (2, 20_000_000)]
		);
		assert_eq!(
			cohort_capital.get(&2).unwrap().contributor_balances.clone().into_inner(),
			vec![(4, 250_000_000), (5, 150_000_000),]
		);

		// should return funds to the accounts in order
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &2),
			20_000_000
		);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &3),
			280_000_000
		);

		// vault 2 should return 50 to the 5th contributor
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &4),
			250_000_000
		);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &5),
			150_000_000
		);

		System::assert_has_event(
			Event::<Test>::RefundedBondFundCapital {
				vault_id: 1,
				amount: 200_000_000,
				account_id: 2,
				cohort_id: 2,
			}
			.into(),
		);
		System::assert_has_event(
			Event::<Test>::RefundedBondFundCapital {
				vault_id: 2,
				amount: 50_000_000,
				account_id: 5,
				cohort_id: 2,
			}
			.into(),
		);
		System::assert_last_event(
			Event::<Test>::NextBidPoolCapitalLocked {
				cohort_id: 2,
				participating_vaults: 2,
				total_activated_capital: 700_000_000,
			}
			.into(),
		);
	});
}

#[test]
fn it_can_distribute_bid_pool_capital() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				activated: 5_000_000_000,
				mining_bond_take: Permill::from_percent(50),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 11,
				activated: 5_000_000_000,
				mining_bond_take: Permill::from_percent(40),
				is_closed: false,
			},
		);

		set_argons(MiningBondFunds::get_bid_pool_account(), 1_000_000_002);

		set_argons(2, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(2), 1, 220_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &2),
			220_000_000
		);
		set_argons(3, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(3), 1, 280_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &3),
			280_000_000
		);

		set_argons(4, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(4), 2, 250_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &4),
			250_000_000
		);
		set_argons(5, 500_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(5), 2, 250_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &5),
			250_000_000
		);

		let bond_funds = MiningBondFundsByCohort::<Test>::get(2);
		assert_eq!(
			bond_funds.get(&1).unwrap().contributor_balances.clone().into_inner(),
			vec![(3, 280_000_000), (2, 220_000_000),]
		);
		assert_eq!(
			bond_funds.get(&2).unwrap().contributor_balances.clone().into_inner(),
			vec![(5, 250_000_000), (4, 250_000_000),]
		);
		assert_eq!(
			NextVaultBidPoolCapital::<Test>::get().into_inner(),
			vec![
				VaultBidPoolCapital {
					vault_id: 1,
					activated_capital: 500_000_000u128,
					cohort_id: 2
				},
				VaultBidPoolCapital {
					vault_id: 2,
					activated_capital: 500_000_000u128,
					cohort_id: 2
				},
			]
		);
		MiningBondFunds::lock_next_bid_pool_capital(2);
		assert_eq!(
			OpenVaultBidPoolCapital::<Test>::get().into_inner(),
			vec![
				VaultBidPoolCapital {
					vault_id: 1,
					activated_capital: 500_000_000u128,
					cohort_id: 2
				},
				VaultBidPoolCapital {
					vault_id: 2,
					activated_capital: 500_000_000u128,
					cohort_id: 2
				},
			],
			"sorted with biggest share last"
		);
		MiningBondFunds::distribute_bid_pool(2);
		let bond_funds = MiningBondFundsByCohort::<Test>::get(2);
		System::assert_has_event(
			Event::<Test>::BidPoolDistributed {
				cohort_id: 2,
				bid_pool_distributed: 800_000_001,
				bid_pool_shares: 2,
				bid_pool_burned: 200_000_001,
			}
			.into(),
		);

		assert_eq!(
			Balances::free_balance(10),
			200_000_000,
			"First vault gets half of the 400 for their side"
		);
		assert_eq!(
			Balances::free_balance(11),
			(400_000_000.0 * 0.4) as u128,
			"Second vault gets 40% of the 400 for their side"
		);
		assert_eq!(Balances::free_balance(MiningBondFunds::get_bid_pool_account()), 0);

		// fund 1 came first, so gets the extra penny
		let contributor_funds_balance_1 = 200_000_000;
		let extra_microgon = 1;
		let vault_1_contributors = vec![
			(
				3,
				280_000_000 +
					extra_microgon + Permill::from_rational(280, 500u64)
					.mul_floor(contributor_funds_balance_1),
			),
			(
				2,
				220_000_000 +
					Permill::from_rational(220, 500u64).mul_floor(contributor_funds_balance_1),
			),
		];
		assert_eq!(
			bond_funds.get(&1).unwrap().contributor_balances.clone().into_inner(),
			vault_1_contributors
		);
		// vault 1 = 200_000_000, contributors = 400_000_000, change = 1
		assert_eq!(bond_funds.get(&1).unwrap().distributed_earnings, Some(400_000_001));

		for (account, amount) in vault_1_contributors {
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &account),
				amount
			);
		}
		let contributor_funds_balance_2 = 400_000_000 - (400_000_000.0 * 0.4) as u128;
		let vault_2_contributors = vec![
			// This one gets the extra penny floating around (first in order of contributions)
			(5, 250_000_000 + contributor_funds_balance_2 / 2),
			(4, 250_000_000 + contributor_funds_balance_2 / 2),
		];
		assert_eq!(
			bond_funds.get(&2).unwrap().contributor_balances.clone().into_inner(),
			vault_2_contributors
		);
		// 400 for vault 2, split as 40% to vault 2, 60% to contributors
		assert_eq!(bond_funds.get(&2).unwrap().distributed_earnings, Some(400_000_000));
		for (account, amount) in vault_2_contributors {
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &account),
				amount
			);
		}
	});
}

#[test]
fn it_can_exit_auto_renew() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				activated: 5_000_000_000,
				mining_bond_take: Permill::from_percent(50),
				is_closed: false,
			},
		);

		// last cohort is 1
		set_argons(2, 200_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(2), 1, 200_000_000));
		set_argons(3, 220_000_000);
		assert_ok!(MiningBondFunds::add_capital(RuntimeOrigin::signed(3), 1, 220_000_000));

		MiningBondFunds::on_new_cohort(1);
		assert!(MiningBondFundsByCohort::<Test>::contains_key(2));
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(2)
				.get(&1)
				.unwrap()
				.contributor_balances
				.to_vec(),
			vec![(3, 220_000_000), (2, 200_000_000)]
		);
		// funds now wait on hold for 1 slot on_new_cohort(2)
		// 3, 4. 5. 6, 7, 8, 9, 10, 11

		assert_ok!(MiningBondFunds::end_renewal(RuntimeOrigin::signed(2), 1, 2));
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(2).get(&1).unwrap().do_not_renew.to_vec(),
			vec![2]
		);

		// trigger rollover (will bump #2 to 2 forward)
		MiningBondFunds::on_new_cohort(10);
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(12).get(&1).unwrap(),
			&MiningBondFund {
				contributor_balances: bounded_vec![(3, 220_000_000)],
				do_not_renew: Default::default(),
				is_rolled_over: false,
				distributed_earnings: None,
				vault_percent_take: Permill::from_percent(50),
			}
		);
		assert_eq!(
			MiningBondFundsByCohort::<Test>::get(2).get(&1).unwrap(),
			&MiningBondFund {
				contributor_balances: bounded_vec![(3, 220_000_000), (2, 200_000_000)],
				do_not_renew: bounded_vec![2],
				is_rolled_over: true,
				distributed_earnings: None,
				vault_percent_take: Permill::from_percent(50),
			}
		);
		assert_err!(
			MiningBondFunds::end_renewal(RuntimeOrigin::signed(2), 1, 2),
			Error::<Test>::AlreadyRenewed
		);
		assert_eq!(
			NextVaultBidPoolCapital::<Test>::get().to_vec(),
			vec![VaultBidPoolCapital {
				vault_id: 1,
				activated_capital: 220_000_000,
				cohort_id: 12,
			}]
		);

		// now trigger refund
		MiningBondFunds::on_new_cohort(12);
		assert_eq!(Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &2), 0);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToBondFund.into(), &3),
			220_000_000
		);
		assert_eq!(
			Balances::free_balance(2),
			200_000_000,
			"should have released the 200 to the exiting contributor"
		);
	});
}
