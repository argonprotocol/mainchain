use crate::{
	Error, Event, HoldReason, LiquidityPool, LiquidityPoolCapital, PrebondedArgons,
	PrebondedByVaultId,
	mock::{LiquidityPools, *},
	pallet::{CapitalActive, CapitalRaising, InsertContributorResponse, VaultPoolsByFrame},
};
use argon_primitives::{OnNewSlot, vault::MiningBidPoolProvider};
use frame_support::{assert_err, assert_ok, traits::fungible::InspectHold};
use pallet_prelude::{argon_primitives::vault::VaultLiquidityPoolFrameEarnings, *};
use sp_core::bounded_vec;
use sp_runtime::Permill;

#[test]
fn it_can_add_pool_capital() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		let hold_reason = HoldReason::ContributedToLiquidityPool;

		set_argons(2, 500_000_000);

		assert_err!(
			LiquidityPools::bond_argons(RuntimeOrigin::signed(2), 1, 200_000_000),
			Error::<Test>::VaultNotAcceptingMiningBonds
		);

		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 50_000_000_000,
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);

		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(2), 1, 200_000_000));
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(3)
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
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(3), 1, 300_000_000));
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(3)
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
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(4), 1, 250_000_000));
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(3)
				.get(&1)
				.unwrap()
				.contributor_balances
				.clone()
				.into_inner(),
			vec![(3, 300_000_000), (4, 250_000_000), (2, 200_000_000)]
		);
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &4), 250_000_000);

		// now move the first bid
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(2), 1, 251_000_000));
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(3)
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
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);
		let hold_reason = HoldReason::ContributedToLiquidityPool;

		for i in 1..=10 {
			let amount = 200_000_000u128 + i as u128;
			set_argons(i, amount);
			assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(i), 1, amount));
			assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &i), amount);
		}

		set_argons(11, 300_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(11), 1, 300_000_000));
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &11), 300_000_000);

		// should have refunded the 1st contributor
		assert_eq!(Balances::balance_on_hold(&hold_reason.into(), &1), 0);
	});
}

#[test]
fn it_can_lock_next_pool_capital() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000,
				sharing_percent: Permill::from_percent(40),
				is_closed: false,
			},
		);

		set_argons(LiquidityPools::get_bid_pool_account(), 1_000_000_000);

		set_argons(2, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(2), 1, 200_000_000));
		set_argons(3, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(3), 1, 300_000_000));

		set_argons(4, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(4), 2, 250_000_000));
		set_argons(5, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(5), 2, 200_000_000));

		LiquidityPools::end_pool_capital_raise(2);
		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![
				LiquidityPoolCapital {
					vault_id: 1,
					activated_capital: 500_000_000u128,
					frame_id: 3
				},
				LiquidityPoolCapital {
					vault_id: 2,
					activated_capital: 450_000_000u128,
					frame_id: 3
				},
			],
			"sorted with biggest share last"
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
fn it_refunds_non_activated_funds_on_lock() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000,
				sharing_percent: Permill::from_percent(40),
				is_closed: false,
			},
		);

		set_argons(LiquidityPools::get_bid_pool_account(), 1_000_000_000);

		set_argons(2, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(2), 1, 220_000_000));
		set_argons(3, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(3), 1, 280_000_000));

		set_argons(4, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(4), 2, 250_000_000));
		set_argons(5, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(5), 2, 200_000_000));

		assert_eq!(
			CapitalRaising::<Test>::get().into_inner(),
			vec![
				LiquidityPoolCapital {
					vault_id: 1,
					activated_capital: 500_000_000u128,
					frame_id: 3
				},
				LiquidityPoolCapital {
					vault_id: 2,
					activated_capital: 450_000_000u128,
					frame_id: 3
				},
			],
			"sorted with biggest share last"
		);
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(3)
				.get(&1)
				.unwrap()
				.contributor_balances
				.clone()
				.into_inner(),
			vec![(3, 280_000_000), (2, 220_000_000)]
		);
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(3)
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

		LiquidityPools::end_pool_capital_raise(3);
		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![
				LiquidityPoolCapital {
					vault_id: 2,
					activated_capital: 400_000_000u128,
					frame_id: 3
				},
				LiquidityPoolCapital {
					vault_id: 1,
					activated_capital: 300_000_000u128,
					frame_id: 3
				},
			],
			"sorted with biggest share last"
		);

		let fund_capital = VaultPoolsByFrame::<Test>::get(3);
		assert_eq!(
			fund_capital.get(&1).unwrap().contributor_balances.clone().into_inner(),
			vec![(3, 280_000_000), (2, 20_000_000)]
		);
		assert_eq!(
			fund_capital.get(&2).unwrap().contributor_balances.clone().into_inner(),
			vec![(4, 250_000_000), (5, 150_000_000),]
		);

		// should return funds to the accounts in order
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &2),
			20_000_000
		);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &3),
			280_000_000
		);

		// vault 2 should return 50 to the 5th contributor
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &4),
			250_000_000
		);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &5),
			150_000_000
		);

		System::assert_has_event(
			Event::<Test>::RefundedLiquidityPoolCapital {
				vault_id: 1,
				amount: 200_000_000,
				account_id: 2,
				frame_id: 3,
			}
			.into(),
		);
		System::assert_has_event(
			Event::<Test>::RefundedLiquidityPoolCapital {
				vault_id: 2,
				amount: 50_000_000,
				account_id: 5,
				frame_id: 3,
			}
			.into(),
		);
		System::assert_last_event(
			Event::<Test>::NextBidPoolCapitalLocked {
				frame_id: 3,
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
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				activated: 5_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);
		insert_vault(
			2,
			TestVault {
				account_id: 11,
				activated: 5_000_000_000,
				sharing_percent: Permill::from_percent(60),
				is_closed: false,
			},
		);

		set_argons(LiquidityPools::get_bid_pool_account(), 1_000_000_002);

		set_argons(10, 100_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(10), 1, 100_000_000,));
		set_argons(2, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(2), 1, 120_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &2),
			120_000_000
		);
		set_argons(3, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(3), 1, 280_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &3),
			280_000_000
		);

		set_argons(4, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(4), 2, 250_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &4),
			250_000_000
		);
		set_argons(5, 500_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(5), 2, 250_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &5),
			250_000_000
		);

		let bond_funds = VaultPoolsByFrame::<Test>::get(3);
		assert_eq!(
			bond_funds.get(&1).unwrap().contributor_balances.clone().into_inner(),
			vec![(3, 280_000_000), (2, 120_000_000), (10, 100_000_000)]
		);
		assert_eq!(
			bond_funds.get(&2).unwrap().contributor_balances.clone().into_inner(),
			vec![(5, 250_000_000), (4, 250_000_000),]
		);
		assert_eq!(
			CapitalRaising::<Test>::get().into_inner(),
			vec![
				LiquidityPoolCapital {
					vault_id: 1,
					activated_capital: 500_000_000u128,
					frame_id: 3
				},
				LiquidityPoolCapital {
					vault_id: 2,
					activated_capital: 500_000_000u128,
					frame_id: 3
				},
			]
		);
		LiquidityPools::end_pool_capital_raise(3);
		assert_eq!(
			CapitalActive::<Test>::get().into_inner(),
			vec![
				LiquidityPoolCapital {
					vault_id: 1,
					activated_capital: 500_000_000u128,
					frame_id: 3
				},
				LiquidityPoolCapital {
					vault_id: 2,
					activated_capital: 500_000_000u128,
					frame_id: 3
				},
			],
			"sorted with biggest share last"
		);
		// Pretend we skipped forward and will now distribute the bid pool for 3
		LiquidityPools::distribute_bid_pool(3);
		let bond_funds = VaultPoolsByFrame::<Test>::get(3);
		System::assert_has_event(
			Event::<Test>::BidPoolDistributed {
				frame_id: 3,
				bid_pool_distributed: 800_000_001,
				bid_pool_shares: 2,
				bid_pool_burned: 200_000_001,
			}
			.into(),
		);

		let profits = LastVaultProfits::get();
		assert_eq!(profits.len(), 2, "should have 2 vault profits recorded");
		let vault_1_profits = profits.iter().find(|p| p.vault_id == 1).unwrap();
		assert_eq!(
			vault_1_profits,
			&VaultLiquidityPoolFrameEarnings {
				vault_id: 1,
				vault_operator_account_id: 10,
				earnings: 400_000_000 + 1, // 50% of the 800 + 1 extra penny
				earnings_for_vault: (400_000_000.0 * 0.5) as u128
					+ (100.0 / 500.0 * 400_000_000.0 * 0.5) as u128
					+ 1, // 50% of the 400 + 50% * 100 of 500 contributed argons * 400) + 1 extra penny
				capital_contributed_by_vault: 100_000_000,
				frame_id: 3,
				capital_contributed: 500_000_000,
			},
			"First vault gets half of the 400 for their side"
		);
		let vault_2_profits = profits.iter().find(|p| p.vault_id == 2).unwrap();
		assert_eq!(
			vault_2_profits,
			&VaultLiquidityPoolFrameEarnings {
				vault_id: 2,
				vault_operator_account_id: 11,
				earnings: 400_000_000,
				earnings_for_vault: (400_000_000.0 * 0.4) as u128,
				capital_contributed_by_vault: 0,
				frame_id: 3,
				capital_contributed: 500_000_000,
			},
			"Second vault gets 40% of the 400 for their side"
		);
		assert_eq!(Balances::free_balance(LiquidityPools::get_bid_pool_account()), 0);

		// fund 1 came first, so gets the extra penny
		let contributor_funds_balance_1 = 200_000_000;

		let vault_1_contributors = vec![
			(
				3,
				280_000_000 +
					Permill::from_rational(280, 500u64).mul_floor(contributor_funds_balance_1),
			),
			(
				2,
				120_000_000 +
					Permill::from_rational(120, 500u64).mul_floor(contributor_funds_balance_1),
			),
			(
				10,
				100_000_000 +
					Permill::from_rational(100, 500u64).mul_floor(contributor_funds_balance_1),
			),
		];
		assert_eq!(
			bond_funds.get(&1).unwrap().contributor_balances.clone().into_inner(),
			vault_1_contributors
		);
		// vault 1 = 200_000_000, contributors = 400_000_000, change = 1
		assert_eq!(bond_funds.get(&1).unwrap().distributed_profits, Some(400_000_001));

		for (account, amount) in vault_1_contributors {
			if account == 10 {
				// vault operator gets the full amount
				assert_eq!(
					Balances::free_balance(account),
					0,
					"vault operator must collect earnings"
				);
			} else {
				// contributors get their share on hold
				assert_eq!(
					Balances::balance_on_hold(
						&HoldReason::ContributedToLiquidityPool.into(),
						&account
					),
					amount,
					"contributor {} should have their funds on hold",
					account
				);
			}
		}
		let contributor_funds_balance_2 = 400_000_000 - (400_000_000.0 * 0.4) as u128;
		let vault_2_contributors = vec![
			(5, 250_000_000 + contributor_funds_balance_2 / 2),
			(4, 250_000_000 + contributor_funds_balance_2 / 2),
		];
		assert_eq!(
			bond_funds.get(&2).unwrap().contributor_balances.clone().into_inner(),
			vault_2_contributors
		);
		// 400 for vault 2, split as 40% to vault 2, 60% to contributors
		assert_eq!(bond_funds.get(&2).unwrap().distributed_profits, Some(400_000_000));
		for (account, amount) in vault_2_contributors {
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &account),
				amount
			);
		}
	});
}

#[test]
fn it_can_exit_auto_renew() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(0);
		insert_vault(
			1,
			TestVault {
				account_id: 10,
				activated: 5_000_000_000,
				sharing_percent: Permill::from_percent(50),
				is_closed: false,
			},
		);

		// last fund is 1
		set_argons(2, 200_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(2), 1, 200_000_000));
		set_argons(3, 220_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(3), 1, 220_000_000));

		LiquidityPools::on_frame_start(1);
		assert!(VaultPoolsByFrame::<Test>::contains_key(2));
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(2).get(&1).unwrap().contributor_balances.to_vec(),
			vec![(3, 220_000_000), (2, 200_000_000)]
		);
		// funds now wait on hold for 1 slot on_frame_start(2)
		// 3, 4. 5. 6, 7, 8, 9, 10, 11

		assert_ok!(LiquidityPools::unbond_argons(RuntimeOrigin::signed(2), 1, 2));
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(2).get(&1).unwrap().do_not_renew.to_vec(),
			vec![2]
		);

		// trigger rollover (will bump #2 to 2 forward)
		LiquidityPools::on_frame_start(10);
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(12).get(&1).unwrap(),
			&LiquidityPool {
				contributor_balances: bounded_vec![(3, 220_000_000)],
				do_not_renew: Default::default(),
				is_rolled_over: false,
				distributed_profits: None,
				vault_sharing_percent: Permill::from_percent(50),
			}
		);
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(2).get(&1).unwrap(),
			&LiquidityPool {
				contributor_balances: bounded_vec![(3, 220_000_000), (2, 200_000_000)],
				do_not_renew: bounded_vec![2],
				is_rolled_over: true,
				distributed_profits: None,
				vault_sharing_percent: Permill::from_percent(50),
			}
		);
		assert_err!(
			LiquidityPools::unbond_argons(RuntimeOrigin::signed(2), 1, 2),
			Error::<Test>::AlreadyRenewed
		);
		assert_eq!(
			CapitalRaising::<Test>::get().to_vec(),
			vec![LiquidityPoolCapital {
				vault_id: 1,
				activated_capital: 220_000_000,
				frame_id: 12,
			}]
		);

		// now trigger refund
		LiquidityPools::on_frame_start(12);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &2),
			0
		);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &3),
			220_000_000
		);
		assert_eq!(
			Balances::free_balance(2),
			200_000_000,
			"should have released the 200 to the exiting contributor"
		);
	});
}

#[test]
fn test_prebonded_argons() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 100_000_000,
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);

		assert_err!(
			LiquidityPools::vault_operator_prebond(RuntimeOrigin::signed(2), 1, 10_000_000),
			Error::<Test>::NotAVaultOperator
		);

		assert_err!(
			LiquidityPools::vault_operator_prebond(RuntimeOrigin::signed(1), 1, 10_000_000),
			DispatchError::Token(TokenError::FundsUnavailable)
		);

		set_argons(1, 200_000_000);
		assert_ok!(LiquidityPools::vault_operator_prebond(RuntimeOrigin::signed(1), 1, 10_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &1),
			100_000_000
		);
		assert_eq!(PrebondedByVaultId::<Test>::get(1).unwrap().starting_frame_id, 1);
		assert_eq!(PrebondedByVaultId::<Test>::get(1).unwrap().amount_unbonded, 100_000_000);
		System::assert_last_event(
			Event::<Test>::VaultOperatorPrebond {
				vault_id: 1,
				account_id: 1,
				amount_per_frame: 10_000_000,
			}
			.into(),
		);

		// if we add more funds, it should be additive
		assert_err!(
			LiquidityPools::vault_operator_prebond(RuntimeOrigin::signed(1), 1, 9_000_000),
			Error::<Test>::MaxAmountBelowMinimum //	"cant reduce the max amount"
		);
		assert_ok!(LiquidityPools::vault_operator_prebond(RuntimeOrigin::signed(1), 1, 15_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &1),
			150_000_000
		);
		assert_eq!(PrebondedByVaultId::<Test>::get(1).unwrap().amount_unbonded, 150_000_000);
		System::assert_last_event(
			Event::<Test>::VaultOperatorPrebond {
				vault_id: 1,
				account_id: 1,
				amount_per_frame: 15_000_000,
			}
			.into(),
		);

		CurrentFrameId::set(10);

		// now test that it properly accounts for already allocated funds
		for frame in 2..12 {
			VaultPoolsByFrame::<Test>::mutate(frame, |pools| {
				if !pools.contains_key(&1) {
					pools
						.try_insert(1, LiquidityPool::<Test>::new(1))
						.expect("Should be able to create a pool");
				}
				let pool = pools.get_mut(&1).unwrap();
				let amount = PrebondedByVaultId::<Test>::mutate(1, |a| {
					if let Some(a) = a { a.take_unbonded(frame, 5_000_000, 0) } else { 0 }
				});
				println!("Frame {}: Adding {} to pool 1", frame, amount);
				pool.contributor_balances.try_push((1, amount)).unwrap();
			});
		}
		// It should be able to add up the prebond and actual to know it's already full
		assert_err!(
			LiquidityPools::vault_operator_prebond(RuntimeOrigin::signed(1), 1, 15_000_000),
			Error::<Test>::MaxAmountBelowMinimum
		);
		// should have allocated 5_000_000 per frame
		assert_eq!(
			PrebondedByVaultId::<Test>::get(1).unwrap().amount_unbonded,
			150_000_000 - 5_000_000 * 10
		);
		assert_ok!(LiquidityPools::vault_operator_prebond(RuntimeOrigin::signed(1), 1, 16_000_000));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::ContributedToLiquidityPool.into(), &1),
			160_000_000
		);
	});
}

#[test]
fn test_liquidity_pool_struct() {
	MaxLiquidityPoolContributors::set(2);
	new_test_ext().execute_with(|| {
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 50_000_000_000,
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);

		let mut pool = LiquidityPool::<Test>::new(1);
		assert_eq!(pool.vault_sharing_percent, Permill::from_percent(10));
		assert!(pool.can_add_contributor(&1));
		assert_eq!(
			pool.try_insert_contributor(1, 50).ok(),
			Some(InsertContributorResponse { hold_amount: 50, needs_refund: None })
		);
		assert_eq!(pool.contributor_balances.len(), 1);
		assert_eq!(
			pool.try_insert_contributor(2, 500).ok(),
			Some(InsertContributorResponse { hold_amount: 500, needs_refund: None })
		);
		assert_eq!(pool.contributor_balances.len(), 2);
		assert_eq!(
			pool.try_insert_contributor(3, 1000).ok(),
			Some(InsertContributorResponse { hold_amount: 1000, needs_refund: Some((1, 50)) }),
			"should remove the first contributor"
		);
		assert_eq!(pool.contributor_balances.to_vec(), vec![(3, 1000), (2, 500)]);
		assert!(pool.can_add_contributor(&2), "should be able to move contributor 2 to the front");
		assert_eq!(
			pool.try_insert_contributor(2, 2000).ok(),
			Some(InsertContributorResponse { hold_amount: 2000 - 500, needs_refund: None }),
			"should update the second contributor"
		);
		assert_eq!(pool.contributor_balances.to_vec(), vec![(2, 2000), (3, 1000),]);
	});
}

#[test]
fn test_capital_raise_with_prebonded() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000, // activates 500_000_000 per frame
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);

		set_argons(1, 5_000_000_000);
		assert_ok!(LiquidityPools::vault_operator_prebond(
			RuntimeOrigin::signed(1),
			1,
			500_000_000
		));

		LiquidityPools::end_pool_capital_raise(1);

		let frame_1_pools = VaultPoolsByFrame::<Test>::get(1);
		let vault_1_frame_1 = frame_1_pools.get(&1).unwrap();

		assert_eq!(
			vault_1_frame_1.contributor_balances.to_vec(),
			vec![(1, 500_000_000)],
			"should add if space"
		);
		assert_eq!(
			CapitalActive::<Test>::get().to_vec(),
			vec![LiquidityPoolCapital { vault_id: 1, activated_capital: 500_000_000, frame_id: 1 }]
		);
	});
}

#[test]
fn test_capital_raise_with_prebonded_when_no_space() {
	MaxLiquidityPoolContributors::set(2);
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		CurrentFrameId::set(1);
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 5_000_000_000, // activates 500_000_000 per frame
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);

		set_argons(2, 1_000_000_000);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(2), 1, 500_000_000));
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(3).get(&1).unwrap().contributor_balances.to_vec(),
			vec![(2, 500_000_000)]
		);

		set_argons(1, 5_000_000_000);
		assert_ok!(LiquidityPools::vault_operator_prebond(
			RuntimeOrigin::signed(1),
			1,
			500_000_000
		));

		LiquidityPools::end_pool_capital_raise(3);
		// should not be able to add the prebonded funds as there is no space

		let frame_3_pools = VaultPoolsByFrame::<Test>::get(3);
		let vault_1_frame_3 = frame_3_pools.get(&1).unwrap();
		assert_eq!(
			vault_1_frame_3.contributor_balances.to_vec(),
			vec![(2, 500_000_000)],
			"should not add if no space"
		);
		assert_eq!(
			CapitalActive::<Test>::get().to_vec(),
			vec![LiquidityPoolCapital { vault_id: 1, activated_capital: 500_000_000, frame_id: 3 }]
		);

		// now try if we have just a small amount that can be filled
		CurrentFrameId::set(2);
		assert_ok!(LiquidityPools::bond_argons(RuntimeOrigin::signed(2), 1, 400_000_000));
		assert_eq!(
			VaultPoolsByFrame::<Test>::get(4).get(&1).unwrap().contributor_balances.to_vec(),
			vec![(2, 400_000_000)]
		);

		LiquidityPools::end_pool_capital_raise(4);
		// should not be able to add the prebonded funds as there is no space

		let frame_4_pools = VaultPoolsByFrame::<Test>::get(4);
		let vault_1_frame_4 = frame_4_pools.get(&1).unwrap();
		assert_eq!(
			vault_1_frame_4.contributor_balances.to_vec(),
			vec![(2, 400_000_000), (1, 100_000_000)],
			"should have the prebonded funds added as there is space now"
		);
		assert_eq!(
			CapitalActive::<Test>::get().to_vec(),
			vec![LiquidityPoolCapital { vault_id: 1, activated_capital: 500_000_000, frame_id: 4 }]
		);
	});
}

#[test]
fn test_prebonded_argons_struct() {
	new_test_ext().execute_with(|| {
		insert_vault(
			1,
			TestVault {
				account_id: 1,
				activated: 5_000_000,
				sharing_percent: Permill::from_percent(10),
				is_closed: false,
			},
		);
		CurrentFrameId::set(1);
		let mut prebonded = PrebondedArgons::<Test>::new(1, 1, 5_000_000, 500_000);
		assert_eq!(prebonded.starting_frame_id, 1);

		assert_eq!(prebonded.take_unbonded(2, 1_000_000, 0), 500_000);
		assert_eq!(prebonded.starting_frame_id, 1);

		assert_eq!(prebonded.take_unbonded(2, 1_000_000, 500_000), 0, "already maxed out");
		assert_eq!(prebonded.take_unbonded(12, 1_000_000, 500_000), 0, "next pass also maxed out");

		assert_eq!(prebonded.take_unbonded(3, 300_000, 0), 300_000, "takes max available");
		assert_eq!(
			prebonded.take_unbonded(13, 300_000, 300_000),
			200_000,
			"caps out at 500k on next pass"
		);
	});
}
