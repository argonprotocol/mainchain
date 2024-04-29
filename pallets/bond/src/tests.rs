use frame_support::{
	assert_noop, assert_ok,
	traits::{
		fungible::{Inspect, InspectHold, Mutate},
		tokens::Preservation,
		OnInitialize,
	},
};
use ulx_primitives::bond::BondProvider;

use crate::{
	mock::{Bonds, *},
	pallet::{
		BondCompletions, BondFundExpirations, BondFunds, Bonds as BondsStorage, NextBondFundId,
	},
	Error, Event, HoldReason,
};

#[test]
fn it_can_offer_a_fund() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		assert_noop!(
			Bonds::offer_fund(RuntimeOrigin::signed(1), 10, 0, 100_000, 10),
			Error::<Test>::InsufficientFunds
		);

		set_argons(1, 100_010);

		assert_ok!(Bonds::offer_fund(RuntimeOrigin::signed(1), 10, 0, 100_000, 10));
		System::assert_last_event(
			Event::BondFundOffered {
				bond_fund_id: 1,
				expiration_block: 10,
				amount_offered: 100_000,
				offer_account_id: 1,
			}
			.into(),
		);

		Bonds::on_initialize(1);
		assert!(System::account_exists(&1));

		assert_eq!(Balances::reserved_balance(1), 100_000);
		assert_eq!(Balances::free_balance(1), 10);

		assert_eq!(NextBondFundId::<Test>::get(), Some(2u32));
		assert_eq!(BondFunds::<Test>::get(1).unwrap().offer_account_id, 1);
		assert_eq!(BondFundExpirations::<Test>::get(10).into_inner(), vec![1u32]);

		System::set_block_number(10);
		Bonds::on_initialize(10);
		System::assert_last_event(
			Event::BondFundExpired { offer_account_id: 1, bond_fund_id: 1 }.into(),
		);
		assert_eq!(BondFunds::<Test>::get(1), None);
		assert!(!BondFundExpirations::<Test>::contains_key(10));
	});
}

#[test]
fn it_can_extend_a_fund() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		set_argons(1, 2000);
		assert_ok!(Bonds::offer_fund(RuntimeOrigin::signed(1), 10, 0, 1000, 10));

		Bonds::on_initialize(1);
		assert!(System::account_exists(&1));
		assert_eq!(Balances::reserved_balance(1), 1000);

		assert_noop!(
			Bonds::extend_fund(RuntimeOrigin::signed(2), 1, 1010, 11),
			Error::<Test>::NoPermissions
		);

		assert_ok!(Bonds::extend_fund(RuntimeOrigin::signed(1), 1, 1010, 11));
		System::assert_last_event(
			Event::BondFundExtended { bond_fund_id: 1, expiration_block: 11, amount_offered: 1010 }
				.into(),
		);
		assert!(System::account_exists(&1));
		assert_eq!(Balances::reserved_balance(1), 1010);
		assert_eq!(BondFundExpirations::<Test>::get(10), vec![]);
		assert_eq!(BondFundExpirations::<Test>::get(11).into_inner(), vec![1u32]);
	});
}

#[test]
fn it_can_stop_offering_a_fund() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		set_argons(1, 100_000);
		assert_ok!(Bonds::offer_fund(RuntimeOrigin::signed(1), 10_000, 5, 50_000, 1440));
		assert_eq!(Balances::free_balance(1), 50_000);

		Bonds::on_initialize(1);

		set_argons(2, 10_000);
		assert_ok!(Bonds::lease(RuntimeOrigin::signed(2), 1, 10_000, 1440));
		assert_ok!(Bonds::lease(RuntimeOrigin::signed(2), 1, 20_000, 1440));
		assert_ok!(Bonds::end_fund(RuntimeOrigin::signed(1), 1));
		// 9 blocks * 10 milligons per block per argon (rented 1 argon)
		let fee1 = 5 + ((1f64 / 365f64) * 10_000f64 * 0.1f64) as u128; // 5 + 2
		let fee2 = 5 + ((1f64 / 365f64) * 20_000f64 * 0.1f64) as u128; // 5 + 5
		assert_eq!(Balances::free_balance(2), 10_000 - fee1 - fee2);

		// got back 20_000
		assert_eq!(Balances::free_balance(1), 50_000 + 20_000 + fee1 + fee2);
		assert_eq!(Balances::balance_on_hold(&HoldReason::EnterBondFund.into(), &1u64), 30_000);

		System::set_block_number(800);

		assert_ok!(Bonds::return_bond(RuntimeOrigin::signed(2), 1));
		assert_eq!(BondFunds::<Test>::get(1).unwrap().amount_bonded, 20_000);
		assert_eq!(BondFunds::<Test>::get(1).unwrap().amount_reserved, 20_000);
		assert_eq!(BondFundExpirations::<Test>::get(1440), vec![1]);

		let final_fee1 = 5 + ((799f64 / (365f64 * 1440f64)) * 10_000f64 * 0.1f64) as u128;
		System::assert_has_event(
			Event::BondFeeRefund {
				bonded_account_id: 2,
				bond_fund_reduction_for_payment: 0u32.into(),
				bond_fund_id: 1,
				final_fee: final_fee1,
				refund_amount: fee1 - final_fee1,
				bond_id: 1,
			}
			.into(),
		);
		assert_eq!(Balances::free_balance(2), 10_000 - final_fee1 - fee2);

		assert_eq!(Balances::free_balance(1), 100_000 - 20_000 + final_fee1 + fee2);

		System::set_block_number(1000);
		assert_ok!(Bonds::return_bond(RuntimeOrigin::signed(2), 2));
		let final_fee2 = 5 + ((999f64 / (365f64 * 1440f64)) * 20_000f64 * 0.1f64) as u128;

		System::assert_has_event(
			Event::BondFeeRefund {
				bonded_account_id: 2,
				bond_fund_reduction_for_payment: 0u32.into(),
				bond_fund_id: 1,
				final_fee: final_fee2,
				refund_amount: fee2 - final_fee2,
				bond_id: 2,
			}
			.into(),
		);
		assert!(!BondFunds::<Test>::contains_key(1));
		assert_eq!(BondFundExpirations::<Test>::get(1440), vec![]);

		assert_eq!(Balances::free_balance(2), 10_000 - final_fee1 - final_fee2);
		assert_eq!(Balances::free_balance(1), 100_000 + final_fee1 + final_fee2);
	});
}

#[test]
fn it_can_lease_a_bond() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(1, 1_000_000);
		assert_ok!(Bonds::offer_fund(RuntimeOrigin::signed(1), 1000, 25, 500_000, 2880));
		assert_eq!(Balances::free_balance(1), 500_000);

		Bonds::on_initialize(1);

		set_argons(2, 2_000);
		assert_ok!(Bonds::lease(RuntimeOrigin::signed(2), 1, 500_000, 2440));

		// fee is 9 milligons per block per argon (rented 5 argons)
		let fee = (25f64 + (0.01f64 * 500_000f64 * (2440f64 / (365f64 * 1440f64)))) as u128;
		System::assert_last_event(
			Event::BondLeased {
				bond_fund_id: 1,
				bond_id: 1,
				bonded_account_id: 2,
				amount: 500_000,
				completion_block: 2440,
				annual_percent_rate: 1000,
				total_fee: fee,
			}
			.into(),
		);
		assert_eq!(Balances::free_balance(2), 2_000 - fee);
		assert_eq!(Balances::free_balance(1), 500_000 + fee);

		System::set_block_number(2440);
		Bonds::on_initialize(2440);
		assert!(!BondsStorage::<Test>::contains_key(1));
		assert!(!BondCompletions::<Test>::contains_key(2440));
	});
}

#[test]
fn it_can_bond_from_self() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(2, 2_000);

		assert_noop!(
			Bonds::bond_self(RuntimeOrigin::signed(2), 2000, 100),
			Error::<Test>::TransactionWouldTakeAccountBelowMinimumBalance
		);
		// add existential deposit
		set_argons(2, 2_100);
		assert_ok!(Bonds::bond_self(RuntimeOrigin::signed(2), 2000, 100));
		System::assert_last_event(
			Event::BondedSelf {
				bond_id: 1,
				bonded_account_id: 2,
				amount: 2000,
				completion_block: 100,
			}
			.into(),
		);
		assert_eq!(BondsStorage::<Test>::get(1).unwrap().amount, 2000);
		assert_eq!(BondsStorage::<Test>::get(1).unwrap().bonded_account_id, 2);
		assert_eq!(BondsStorage::<Test>::get(1).unwrap().bond_fund_id, None);
		assert_eq!(BondCompletions::<Test>::get(100).into_inner(), vec![1]);
		// ensure no death
		assert!(System::account_exists(&2));

		assert_eq!(Balances::free_balance(2), 100);

		System::set_block_number(100);
		Bonds::on_initialize(100);
		assert!(!BondFunds::<Test>::contains_key(1));
		assert!(!BondCompletions::<Test>::contains_key(100));
		assert_eq!(Balances::free_balance(2), 2100);
		assert!(System::account_exists(&2));
	});
}

#[test]
fn it_can_recoup_funds_from_a_bond_fund_if_spent() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(2, 2_000);

		set_argons(1, 100_100);
		const BASE_FEE: Balance = 4u128;
		const APR: u32 = 2_115;

		assert_ok!(Bonds::offer_fund(RuntimeOrigin::signed(1), APR, BASE_FEE, 100_000, 1440));
		assert_eq!(Balances::free_balance(1), 100);

		Bonds::on_initialize(1);

		set_argons(2, 2_000);
		assert_ok!(Bonds::lease(RuntimeOrigin::signed(2), 1, 50_000, 1440));
		// fee is 9 milligons per block per argon (rented 5 argons)
		let fee: Balance = BASE_FEE + ((1f64 / 365f64) * 0.02115f64 * 50_000f64) as u128;
		System::assert_has_event(
			Event::BondLeased {
				bond_fund_id: 1,
				bond_id: 1,
				bonded_account_id: 2,
				amount: 50_000,
				completion_block: 1440,
				total_fee: fee,
				annual_percent_rate: APR,
			}
			.into(),
		);
		let bond = BondsStorage::<Test>::get(1);
		assert_eq!(bond.unwrap().fee, fee);
		assert_eq!(Balances::free_balance(2), 2_000 - fee);
		assert_eq!(Balances::free_balance(1), fee + 100);

		assert_ok!(Balances::transfer(&1, &10, 100 + fee, Preservation::Expendable));
		assert_eq!(Balances::free_balance(1), 0);
		assert!(System::account_exists(&1));

		System::set_block_number(500);
		Bonds::on_initialize(500);
		let original_fee = fee;
		let fee: Balance =
			BASE_FEE + ((495f64 / (365f64 * 1440f64)) * 0.02115f64 * 50_000f64) as u128;

		assert_ok!(Bonds::return_bond(RuntimeOrigin::signed(2), 1));
		assert!(System::account_exists(&1));

		assert_eq!(BondFunds::<Test>::get(1).unwrap().amount_bonded, 0);
		let recoverable_fee = original_fee - fee;
		System::assert_has_event(
			Event::BondFeeRefund {
				bond_fund_id: 1,
				bond_id: 1,
				bonded_account_id: 2,
				refund_amount: recoverable_fee,
				// had to pull in additional funds because it went below minimum balance
				bond_fund_reduction_for_payment: recoverable_fee + Balances::minimum_balance(),
				final_fee: fee,
			}
			.into(),
		);
		System::assert_has_event(Event::BondCompleted { bond_fund_id: Some(1), bond_id: 1 }.into());
		assert_eq!(
			BondFunds::<Test>::get(1).unwrap().amount_reserved,
			100_000 - recoverable_fee - Balances::minimum_balance()
		);
		assert_eq!(Balances::free_balance(1), Balances::minimum_balance());
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::EnterBondFund.into(), &1),
			100_000 - recoverable_fee - Balances::minimum_balance()
		);
		assert!(!BondsStorage::<Test>::contains_key(1));
		assert_eq!(BondCompletions::<Test>::get(103), vec![]);
	});
}

#[test]
fn it_can_extend_a_bond_from_self() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(2, 2_100);
		assert_ok!(Bonds::bond_self(RuntimeOrigin::signed(2), 2000, 100));
		assert_eq!(BondsStorage::<Test>::get(1).unwrap().amount, 2000);
		assert_eq!(BondCompletions::<Test>::get(100).into_inner(), vec![1]);
		assert_eq!(Balances::free_balance(2), 100);

		System::set_block_number(99);
		assert_ok!(Bonds::extend_bond(RuntimeOrigin::signed(2), 1, 2000, 200));
		assert_eq!(Balances::free_balance(2), 100);
		assert_eq!(BondCompletions::<Test>::get(100).into_inner(), Vec::<u64>::new());
		assert_eq!(BondCompletions::<Test>::get(200).into_inner(), vec![1]);

		assert_noop!(
			Bonds::extend_bond(RuntimeOrigin::signed(2), 1, 3000, 200),
			Error::<Test>::InsufficientFunds
		);
		set_argons(2, 1_000);
		assert_ok!(Bonds::extend_bond(RuntimeOrigin::signed(2), 1, 3000, 201));
		assert_eq!(Balances::free_balance(2), 0);
		assert_eq!(Balances::reserved_balance(2), 3000);
		assert_eq!(BondCompletions::<Test>::get(200).into_inner(), Vec::<u64>::new());
		assert_eq!(BondCompletions::<Test>::get(201).into_inner(), vec![1]);
	});
}

#[test]
fn it_can_calculate_apr() {
	new_test_ext().execute_with(|| {
		assert_eq!(Bonds::calculate_fees(1000, 0, 1000, 1440, 1440 * 365), 0);
		assert_eq!(Bonds::calculate_fees(1000, 0, 100, 1440 * 365, 1440 * 365), 1);
		assert_eq!(Bonds::calculate_fees(1000, 0, 99, 1440 * 365, 1440 * 365), 0);
		assert_eq!(Bonds::calculate_fees(1000, 0, 365000, 1440 * 365, 1440 * 365), 3650);
		assert_eq!(Bonds::calculate_fees(1000, 0, 365000, 1440, 1440 * 365), 10);
		// minimum argons for a day that will charge anything
		assert_eq!(Bonds::calculate_fees(1000, 0, 36500, 1440, 1440 * 365), 1);
	})
}

#[test]
fn it_can_send_minimum_balance_transfers() {
	new_test_ext().execute_with(|| {
		set_argons(1, 1060);
		assert_ok!(Balances::transfer(&1, &2, 1000, Preservation::Preserve));
		assert_ok!(Balances::transfer(&1, &2, 50, Preservation::Preserve));
		assert_eq!(Balances::free_balance(1), 10);
		assert_ok!(Balances::transfer(&1, &2, 4, Preservation::Expendable));
		// dusted! will remove anything below ED
		assert_eq!(Balances::free_balance(1), 0);
	})
}
#[test]
fn it_can_extend_a_bond_from_lease() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		Bonds::on_initialize(1);

		set_argons(1, 500_100);
		const BASE_FEE: Balance = 4u128;
		const APR: u32 = 10000u32; // 1 percent apr for the year

		assert_ok!(Bonds::offer_fund(RuntimeOrigin::signed(1), APR, BASE_FEE, 500_000, 5000));
		assert_eq!(Balances::free_balance(1), 100);
		set_argons(2, 3_000);
		assert_ok!(Bonds::lease(RuntimeOrigin::signed(2), 1, 200_000, 1440));
		assert_eq!(BondsStorage::<Test>::get(1).unwrap().amount, 200_000);
		assert_eq!(BondCompletions::<Test>::get(1440).into_inner(), vec![1]);

		let fee: Balance = BASE_FEE + (200_000u128 / 10u128 / 365);
		assert_eq!(BondsStorage::<Test>::get(1).unwrap().fee, fee);
		assert_eq!(Balances::free_balance(2), 3_000 - fee);
		assert_eq!(Balances::free_balance(1), 100 + fee);

		// extend the amount
		assert_ok!(Bonds::extend_bond(RuntimeOrigin::signed(2), 1, 200_000u128, 2880));
		let fee: Balance = BASE_FEE + (2 * 200_000u128 / 10u128 / 365);
		assert_eq!(Balances::free_balance(2), 3000 - fee);
		assert_eq!(BondCompletions::<Test>::get(1440).into_inner(), Vec::<u64>::new());
		assert_eq!(BondCompletions::<Test>::get(2880).into_inner(), vec![1]);

		Balances::transfer(&2, &10, 2800, Preservation::Preserve).unwrap();

		assert_noop!(
			Bonds::extend_bond(RuntimeOrigin::signed(2), 1, 400_000u128, 2880),
			Error::<Test>::InsufficientFunds
		);
		Balances::transfer(&10, &2, 2800, Preservation::Expendable).unwrap();
		assert_ok!(Bonds::extend_bond(RuntimeOrigin::signed(2), 1, 200_000u128, 3000));
		let fee = BASE_FEE + (3000 * 200_000u128 / 10u128 / 365 / 1440);
		assert_eq!(Balances::free_balance(2), 3000 - fee);
		assert_eq!(Balances::free_balance(1), 100 + fee);
		assert_eq!(BondCompletions::<Test>::get(2880).into_inner(), Vec::<u64>::new());
		assert_eq!(BondCompletions::<Test>::get(3000).into_inner(), vec![1]);
	});
}

#[test]
fn it_can_lock_and_unlock_bonds() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(5);

		set_argons(2, 2_100);
		assert_ok!(Bonds::bond_self(RuntimeOrigin::signed(2), 2000, 100));
		assert_ok!(<Bonds as BondProvider>::lock_bond(1u64));
		System::assert_last_event(Event::BondLocked { bond_id: 1, bonded_account_id: 2 }.into());
		assert_noop!(
			Bonds::extend_bond(RuntimeOrigin::signed(2), 1, 2000, 95),
			Error::<Test>::BondLockedCannotModify
		);
		// can extend locked!
		assert_ok!(Bonds::extend_bond(RuntimeOrigin::signed(2), 1, 2000, 101),);
		assert_noop!(
			Bonds::return_bond(RuntimeOrigin::signed(2), 1),
			Error::<Test>::BondLockedCannotModify
		);
		assert_ok!(<Bonds as BondProvider>::unlock_bond(1u64));
		System::assert_last_event(Event::BondUnlocked { bond_id: 1, bonded_account_id: 2 }.into());
		assert_ok!(Bonds::return_bond(RuntimeOrigin::signed(2), 1));
	});
}

#[test]
fn it_can_burn_a_bond_from_a_fund() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		Bonds::on_initialize(1);

		set_argons(1, 500_100);
		const BASE_FEE: Balance = 4u128;
		const APR: u32 = 10000u32; // 1 percent apr for the year

		assert_ok!(Bonds::offer_fund(RuntimeOrigin::signed(1), APR, BASE_FEE, 500_000, 5000));
		assert_eq!(Balances::free_balance(1), 100);
		set_argons(2, 3_000);
		assert_ok!(Bonds::lease(RuntimeOrigin::signed(2), 1, 200_000, 1440));
		assert_eq!(BondsStorage::<Test>::get(1).unwrap().amount, 200_000);
		assert_eq!(BondCompletions::<Test>::get(1440).into_inner(), vec![1]);

		let fee: Balance = BASE_FEE + (200_000u128 / 10u128 / 365);
		assert_eq!(BondsStorage::<Test>::get(1).unwrap().fee, fee);
		assert_eq!(Balances::free_balance(2), 3_000 - fee);
		assert_eq!(Balances::free_balance(1), 100 + fee);

		// burn the bond
		assert_ok!(Bonds::burn_bond(1));
		System::assert_last_event(
			Event::<Test>::BondBurned { bond_id: 1, bond_fund_id: Some(1), amount: 200_000 }.into(),
		);
		assert_eq!(Balances::reserved_balance(1), 500_000 - 200_000);
		assert_eq!(Balances::free_balance(2), 3000 - fee);
		let bond_fund = BondFunds::<Test>::get(1).unwrap();
		assert_eq!(bond_fund.amount_bonded, 0);
		assert_eq!(bond_fund.amount_reserved, 500_000 - 200_000);

		assert!(BondCompletions::<Test>::get(1440).into_inner().is_empty());
		assert!(BondCompletions::<Test>::get(2880).into_inner().is_empty());
	});
}
