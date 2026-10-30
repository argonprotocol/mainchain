use crate::{mock::*, *};
use argon_primitives::AscendingRankKey;
use frame_support::{
	assert_noop, assert_ok,
	storage::{unhashed, with_storage_layer},
	traits::Hooks,
};

#[test]
fn outer_failure_rolls_back_completed_reward_allocation_and_payment() {
	new_test_ext().execute_with(|| {
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		start_frame(3);
		CurrentTick::set(1_000 + RewardMaturityTicks::get());
		let issuance = Balances::total_issuance();

		assert_noop!(
			with_storage_layer(|| -> DispatchResult {
				StabilizerBuys::on_initialize(2);
				assert_eq!(Balances::free_balance(1), 1_005_000_000);
				assert_eq!(Balances::total_issuance(), issuance + 5_000_000);
				assert_eq!(Budget::available_budget(), 95_000_000);
				assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
				assert!(MaturingAuctions::<Test>::iter().next().is_none());
				assert!(PayoutQueue::<Test>::iter().next().is_none());
				Err(DispatchError::Other("wrapper failed"))
			}),
			DispatchError::Other("wrapper failed")
		);

		assert_eq!(Balances::free_balance(1), 900_000_000);
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			100_000_000
		);
		assert_eq!(Budget::available_budget(), 91_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining, 9_000_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(MaturingAuctions::<Test>::iter().count(), 1);
		assert_eq!(PayoutQueue::<Test>::iter().count(), 1);

		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 1_005_000_000);
		assert_eq!(Balances::total_issuance(), issuance + 5_000_000);
		assert_eq!(Budget::available_budget(), 95_000_000);
	});
}

#[test]
fn purchases_aggregate_by_recipient_and_admission_frame() {
	new_test_ext().execute_with(|| {
		start_frame(10);
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		assert!(Auctions::<Test>::get(1).is_none());
		let payout = PurchasesByAccount::<Test>::get(1, 10u64.to_ordered()).unwrap();
		assert_eq!(PurchasesByAccount::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(payout.principal, 200_000_000);
		assert_eq!(payout.captured_discount, 20_000_000);
		assert_eq!(Auctions::<Test>::get(10).unwrap().total_matching_basis, 20_000_000);

		start_frame(11);
		StabilizerBuys::on_initialize(2);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			200_000_000
		);
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		assert_eq!(PurchasesByAccount::<Test>::iter_prefix(1).count(), 2);
		assert_eq!(
			PurchasesByAccount::<Test>::get(1, 11u64.to_ordered())
				.unwrap()
				.captured_discount,
			10_000_000
		);
		assert_eq!(
			PurchasesByAccount::<Test>::get(1, 10u64.to_ordered())
				.unwrap()
				.captured_discount,
			20_000_000
		);
		assert_eq!(PayoutQueue::<Test>::iter().count(), 1);
	});
}

#[test]
fn matching_rate_is_frozen_for_the_entire_cohort() {
	new_test_ext().execute_with(|| {
		assert_ok!(StabilizerBuys::set_matching_rate(
			RuntimeOrigin::root(),
			Permill::from_percent(50)
		));
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		assert_ok!(StabilizerBuys::set_matching_rate(RuntimeOrigin::root(), Permill::one()));
		assert_ok!(StabilizerBuys::record_transferred_purchase(&2, &purchase()));
		assert_eq!(Auctions::<Test>::get(1).unwrap().total_matching_basis, 10_000_000);

		start_frame(2);
		let auction = Auctions::<Test>::get(1).unwrap();
		assert_eq!(auction.matching_rate, Permill::from_percent(50));
		assert_eq!(auction.budget, Some(9_000_000));
		assert_eq!(auction.remaining, 9_000_000);
		assert_ok!(StabilizerBuys::record_transferred_purchase(&2, &purchase()));
		assert_eq!(Auctions::<Test>::get(2).unwrap().matching_rate, Permill::one());
		assert_noop!(
			StabilizerBuys::set_matching_rate(RuntimeOrigin::signed(1), Permill::zero()),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn matching_rounds_after_account_aggregation_and_zero_rate_stays_frozen() {
	new_test_ext().execute_with(|| {
		assert_ok!(StabilizerBuys::set_matching_rate(
			RuntimeOrigin::root(),
			Permill::from_percent(50)
		));
		let mut purchase = purchase();
		purchase.principal = 90_000_001;
		for _ in 0..4 {
			assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase));
		}
		assert_eq!(
			PurchasesByAccount::<Test>::get(1, 1u64.to_ordered()).unwrap().captured_discount,
			4
		);
		assert_eq!(Auctions::<Test>::get(1).unwrap().total_matching_basis, 2);
		start_frame(2);
		assert_eq!(Auctions::<Test>::get(1).unwrap().budget, Some(1));
		assert_noop!(
			StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(49)),
			Error::<Test>::BidRoundsToZero
		);
		assert_eq!(
			PurchasesByAccount::<Test>::get(1, 1u64.to_ordered())
				.unwrap()
				.positive_bid_count,
			0
		);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::one()));
		assert_ok!(StabilizerBuys::set_matching_rate(RuntimeOrigin::root(), Permill::zero()));
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase));
		assert_ok!(StabilizerBuys::set_matching_rate(RuntimeOrigin::root(), Permill::one()));
		assert_ok!(StabilizerBuys::record_transferred_purchase(&2, &purchase));
		start_frame(3);
		assert_eq!(Auctions::<Test>::get(2).unwrap().total_matching_basis, 0);
		assert_eq!(Auctions::<Test>::get(2).unwrap().budget, Some(0));

		let issuance = Balances::total_issuance();
		CurrentTick::set(1_000 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::total_issuance(), issuance + 1);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			purchase.principal
		);
		assert_eq!(Budget::outstanding_rewards(), 0);
	});
}

#[test]
fn reward_and_principal_pay_automatically_at_maturity_and_never_before_it() {
	new_test_ext().execute_with(|| {
		let issuance = Balances::total_issuance();
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		start_frame(2);
		assert_eq!(Budget::available_budget(), 91_000_000);
		assert_ok!(Budget::return_budget(100_000_000));
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		CurrentTick::set(2_000);
		start_frame(3);
		let maturity = 2_000 + RewardMaturityTicks::get();
		assert_eq!(Auctions::<Test>::get(1).unwrap().matures_at_tick, Some(maturity));
		assert!(MaturingAuctions::<Test>::contains_key(AscendingRankKey::new(maturity, 1)));
		CurrentTick::set(maturity - 1);
		StabilizerBuys::on_initialize(2);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(Balances::free_balance(1), 900_000_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(Budget::available_budget(), 191_000_000);

		CurrentTick::set(maturity);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::total_issuance(), issuance + 5_000_000);
		assert_eq!(Balances::free_balance(1), 1_005_000_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Budget::available_budget(), 195_000_000);
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining, 0);
		assert!(MaturingAuctions::<Test>::iter().next().is_none());
		assert!(!BuysByAccount::<Test>::contains_key(1));
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::total_issuance(), issuance + 5_000_000);
	});
}

#[test]
fn bounded_allocation_ranks_rates_then_accounts_and_partially_funds_the_last_winner() {
	new_test_ext().execute_with(|| {
		unhashed::put(b"test_liquidity_buy_budget", &10_000_000u128);
		for account in [256, 255, 1] {
			if account != 1 {
				assert_ok!(Balances::mint_into(&account, 100_000_000));
			}
			assert_ok!(StabilizerBuys::record_transferred_purchase(&account, &purchase()));
		}
		start_frame(2);
		for (account, rate) in [(1, 70), (256, 50), (255, 50), (1, 60)] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(rate)
			));
		}
		start_frame(3);
		CurrentTick::set(1_000 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_eq!(Balances::free_balance(255), 105_000_000);
		assert_eq!(Balances::free_balance(256), 0);
		assert_eq!(BuysByAccount::<Test>::get(256).payable_reward, 4_000_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(Budget::available_budget(), 1_000_000);

		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(256), 104_000_000);
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert!(MaturingAuctions::<Test>::iter().next().is_none());
		assert_eq!(Budget::outstanding_rewards(), 0);
	});
}

#[test]
fn no_bid_returns_reservation_at_maturity_and_releases_principal() {
	new_test_ext().execute_with(|| {
		let providers = System::account(5).providers;
		assert_ok!(StabilizerBuys::record_transferred_purchase(&5, &purchase()));
		assert_eq!(Balances::free_balance(5), 0);
		// Balances drops its free-balance provider when the whole balance moves onto hold.
		assert_eq!(System::account(5).providers, providers);
		start_frame(2);
		CurrentTick::set(2_000);
		start_frame(3);
		let maturity = 2_000 + RewardMaturityTicks::get();
		CurrentTick::set(1_000 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(BuysByAccount::<Test>::get(5).next_payout_tick, Some(maturity));
		assert!(PayoutQueue::<Test>::contains_key(AscendingRankKey::new(maturity, 5)));
		CurrentTick::set(maturity - 1);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Budget::available_budget(), 91_000_000);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &5),
			100_000_000
		);
		CurrentTick::set(maturity);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Budget::available_budget(), 100_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Balances::free_balance(5), 100_000_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &5), 0);
		assert_eq!(System::account(5).providers, providers);
		assert!(!BuysByAccount::<Test>::contains_key(5));
		assert!(PayoutQueue::<Test>::iter().next().is_none());
		assert!(MaturingAuctions::<Test>::iter().next().is_none());
	});
}

#[test]
fn delayed_processing_keeps_the_original_bidding_close_maturity() {
	new_test_ext().execute_with(|| {
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		CurrentTick::set(2_000);
		start_frame(3);
		CurrentTick::set(2_000 + 60 * 1_440);
		start_frame(4);
		assert_eq!(
			Auctions::<Test>::get(1).unwrap().matures_at_tick,
			Some(2_000 + RewardMaturityTicks::get())
		);
		assert_eq!(MaturingAuctions::<Test>::iter().count(), 1);
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_005_000_000);
		start_frame(5);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 1_005_000_000);
		assert_eq!(Budget::available_budget(), 95_000_000);
		assert!(MaturingAuctions::<Test>::iter().next().is_none());
	});
}

#[test]
fn failed_last_bid_return_rolls_back_that_allocation_and_retries_without_double_payment() {
	new_test_ext().execute_with(|| {
		for account in [1, 2, 3] {
			assert_ok!(StabilizerBuys::record_transferred_purchase(&account, &purchase()));
		}
		start_frame(2);
		for account in [1, 2, 3] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(50)
			));
		}
		start_frame(3);
		let maturity = 1_000 + RewardMaturityTicks::get();
		CurrentTick::set(maturity);
		let issuance = Balances::total_issuance();
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::total_issuance(), issuance + 10_000_000);
		FailReturn::set(true);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Budget::available_budget(), 73_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Balances::free_balance(3), 1_000_000_000);
		assert_eq!(Balances::total_issuance(), issuance + 10_000_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining, 17_000_000);
		assert!(MaturingAuctions::<Test>::contains_key(AscendingRankKey::new(maturity + 1, 1)));

		FailReturn::set(false);
		StabilizerBuys::on_initialize(4);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		CurrentTick::set(maturity + 1);
		StabilizerBuys::on_initialize(5);
		assert_eq!(Balances::free_balance(3), 1_005_000_000);
		assert_eq!(Balances::total_issuance(), issuance + 15_000_000);
		assert_eq!(Budget::available_budget(), 85_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(MaturingAuctions::<Test>::iter().next().is_none());
	});
}

#[test]
fn failed_award_preserves_its_rank_and_funding_while_prior_awards_stay_paid() {
	new_test_ext().execute_with(|| {
		for account in [1, 2, 3] {
			assert_ok!(StabilizerBuys::record_transferred_purchase(&account, &purchase()));
		}
		start_frame(2);
		for account in [1, 2, 3] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(50)
			));
		}
		start_frame(3);
		let maturity = 1_000 + RewardMaturityTicks::get();
		CurrentTick::set(maturity);
		FailCommitAt::set(Some(10_000_000));
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_005_000_000);
		assert_eq!(Balances::free_balance(2), 1_000_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Budget::available_budget(), 73_000_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 2);
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining, 22_000_000);
		assert!(MaturingAuctions::<Test>::contains_key(AscendingRankKey::new(maturity + 1, 1)));
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(3), 1_000_000_000);

		FailCommitAt::set(None);
		CurrentTick::set(maturity + 1);
		StabilizerBuys::on_initialize(4);
		for account in [1, 2, 3] {
			assert_eq!(Balances::free_balance(account), 1_005_000_000);
		}
		assert_eq!(Budget::available_budget(), 85_000_000);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
	});
}

#[test]
fn a_failed_auction_does_not_block_another_mature_auction() {
	new_test_ext().execute_with(|| {
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(60)));
		assert_ok!(StabilizerBuys::record_transferred_purchase(&2, &purchase()));
		start_frame(3);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(2), 2, Percent::from_percent(50)));
		start_frame(4);
		let maturity = 1_000 + RewardMaturityTicks::get();
		CurrentTick::set(maturity);
		FailCommitAt::set(Some(6_000_000));
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_eq!(Balances::free_balance(2), 1_005_000_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert!(BidsByAuction::<Test>::iter_prefix(2).next().is_none());
		assert_eq!(Budget::available_budget(), 86_000_000);

		FailCommitAt::set(None);
		CurrentTick::set(maturity + 1);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 1_006_000_000);
		assert_eq!(Budget::available_budget(), 89_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
	});
}

#[test]
fn funding_failure_closes_bidding_and_keeps_the_full_principal_hold() {
	new_test_ext().execute_with(|| {
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		FailReserve::set(true);
		start_frame(2);
		FailReserve::set(false);
		assert_noop!(
			StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::one()),
			Error::<Test>::BiddingClosed
		);
		assert_eq!(Budget::available_budget(), 100_000_000);
		assert_eq!(Auctions::<Test>::get(1).unwrap().budget, None);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert_eq!(
			PurchasesByAccount::<Test>::get(1, 1u64.to_ordered())
				.unwrap()
				.positive_bid_count,
			0
		);
		CurrentTick::set(2_000);
		start_frame(3);
		CurrentTick::set(1_999 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			100_000_000
		);
		CurrentTick::set(2_000 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
	});
}

#[test]
fn opt_out_and_losing_accounts_receive_principal_at_full_maturity() {
	new_test_ext().execute_with(|| {
		unhashed::put(b"test_liquidity_buy_budget", &5_000_000u128);
		for account in [1, 2, 3] {
			assert_ok!(StabilizerBuys::record_transferred_purchase(&account, &purchase()));
		}
		start_frame(2);
		for (account, rate) in [(1, 50), (2, 60), (3, 70)] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(rate)
			));
		}
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(3), 1, Percent::zero()));
		start_frame(3);
		CurrentTick::set(999 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(2);
		for account in [1, 2, 3] {
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &account),
				100_000_000
			);
		}
		CurrentTick::set(1_000 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(3);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::free_balance(1), 1_004_500_000);
		assert_eq!(Balances::free_balance(2), 1_000_000_000);
		assert_eq!(Balances::free_balance(3), 1_000_000_000);
	});
}

#[test]
fn more_than_a_thousand_purchases_and_entrants_finish_with_bounded_automatic_work() {
	new_test_ext().execute_with(|| {
		let mut purchase = purchase();
		purchase.principal = 10_000;
		purchase.usdc_spent = U256::from(9_000u64);
		for _ in 0..1_001 {
			assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase));
		}
		for account in 10..1_011 {
			assert_ok!(Balances::mint_into(&account, 20_000));
			assert_ok!(StabilizerBuys::record_transferred_purchase(&account, &purchase));
		}
		assert_eq!(PurchasesByAccount::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(
			PurchasesByAccount::<Test>::get(1, 1u64.to_ordered()).unwrap().principal,
			10_010_000
		);
		assert_eq!(BuysByAccount::<Test>::iter().count(), 1_002);
		assert_eq!(Auctions::<Test>::get(1).unwrap().total_matching_basis, 2_002_000);
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		for account in 10..1_011 {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(50)
			));
		}
		start_frame(3);
		CurrentTick::set(1_000 + RewardMaturityTicks::get());
		let issuance = Balances::total_issuance();
		StabilizerBuys::on_initialize(2);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1_000);
		assert_eq!(BuysByAccount::<Test>::iter().count(), 1_000);
		for block in 3..=502 {
			StabilizerBuys::on_initialize(block);
		}
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert!(BuysByAccount::<Test>::iter().next().is_none());
		assert!(MaturingAuctions::<Test>::iter().next().is_none());
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Balances::total_issuance(), issuance + 1_001_000);
		assert_eq!(Balances::free_balance(1_010), 20_500);
	});
}

#[test]
fn automatic_payout_preserves_future_groups_and_one_provider() {
	new_test_ext().execute_with(|| {
		let providers = System::account(1).providers;
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		CurrentTick::set(2_000);
		start_frame(3);
		CurrentTick::set(3_000);
		start_frame(4);
		assert_eq!(System::account(1).providers, providers + 1);
		CurrentTick::set(2_000 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(2);
		let buys = BuysByAccount::<Test>::get(1);
		assert_eq!(PurchasesByAccount::<Test>::iter_prefix(1).count(), 1);
		assert!(PurchasesByAccount::<Test>::contains_key(1, 2u64.to_ordered()));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			100_000_000
		);
		assert_eq!(System::account(1).providers, providers + 1);
		assert_eq!(buys.next_payout_tick, Some(3_000 + RewardMaturityTicks::get()));
		assert_eq!(PayoutQueue::<Test>::iter().count(), 1);
		CurrentTick::set(3_000 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(3);
		assert_eq!(System::account(1).providers, providers);
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert!(!BuysByAccount::<Test>::contains_key(1));
	});
}

#[test]
fn fast_frames_admit_without_a_cap_and_retry_bounded_principal_release() {
	new_test_ext().execute_with(|| {
		let providers = System::account(1).providers;
		let mut purchase = purchase();
		purchase.principal = 10_000;
		purchase.usdc_spent = U256::from(9_000u64);
		// Ten-block development frames can close within six network ticks, well before maturity.
		for frame in 1..=4_323 {
			CurrentTick::set(1_000 + (frame - 1) * 6);
			start_frame(frame);
			if frame == 2 {
				assert_ok!(StabilizerBuys::bid(
					RuntimeOrigin::signed(1),
					1,
					Percent::from_percent(50)
				));
			}
			assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase));
			System::reset_events();
		}
		assert!(Auctions::<Test>::get(1).unwrap().matures_at_tick.unwrap() > CurrentTick::get());
		assert_eq!(PurchasesByAccount::<Test>::iter_prefix(1).count(), 4_323);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			43_230_000
		);
		assert_eq!(System::account(1).providers, providers + 1);
		assert_eq!(PayoutQueue::<Test>::iter().count(), 1);

		let maturity = Auctions::<Test>::get(16).unwrap().matures_at_tick.unwrap();
		CurrentTick::set(maturity);
		FailPaymentFor::set(Some(1));
		StabilizerBuys::on_initialize(2);
		assert_eq!(PurchasesByAccount::<Test>::iter_prefix(1).count(), 4_323);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			43_230_000
		);
		assert_eq!(BuysByAccount::<Test>::get(1).payable_reward, 500);
		assert_eq!(Budget::outstanding_rewards(), 500);
		assert_eq!(BuysByAccount::<Test>::get(1).next_payout_tick, Some(maturity + 1));

		FailPaymentFor::set(None);
		CurrentTick::set(maturity + 1);
		StabilizerBuys::on_initialize(3);
		assert_eq!(PurchasesByAccount::<Test>::iter_prefix(1).count(), 4_323 - 16);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			43_230_000 - 160_000
		);
		assert_eq!(Balances::free_balance(1), 1_000_000_000 - 43_230_000 + 160_000 + 500);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(
			BuysByAccount::<Test>::get(1).next_payout_tick,
			Auctions::<Test>::get(17).unwrap().matures_at_tick
		);
		assert_eq!(System::account(1).providers, providers + 1);

		CurrentTick::set(Auctions::<Test>::get(40).unwrap().matures_at_tick.unwrap());
		StabilizerBuys::on_initialize(4);
		assert_eq!(PurchasesByAccount::<Test>::iter_prefix(1).count(), 4_323 - 32);
		assert_eq!(
			PurchasesByAccount::<Test>::iter_prefix(1).next().unwrap().0,
			33u64.to_ordered()
		);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			43_230_000 - 320_000
		);
		assert!(BuysByAccount::<Test>::get(1).next_payout_tick.unwrap() <= CurrentTick::get());
		assert_eq!(PayoutQueue::<Test>::iter().count(), 1);
	});
}

#[test]
fn reward_backlog_after_principal_payment_automatically_queues_the_new_reward() {
	new_test_ext().execute_with(|| {
		let providers = System::account(1).providers;
		for account in [1, 2, 3] {
			assert_ok!(StabilizerBuys::record_transferred_purchase(&account, &purchase()));
		}
		start_frame(2);
		for (account, rate) in [(1, 60), (2, 50), (3, 50)] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(rate)
			));
		}
		start_frame(3);
		CurrentTick::set(1_000 + RewardMaturityTicks::get());
		let issuance = Balances::total_issuance();
		StabilizerBuys::on_initialize(2);
		assert!(!BuysByAccount::<Test>::contains_key(1));
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_eq!(System::account(1).providers, providers);
		assert_eq!(BidsByAuction::<Test>::get(1, AscendingRankKey::new(60, 1)), Some(10_000_000));
		assert_eq!(Budget::outstanding_rewards(), 5_000_000);

		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 1_006_000_000);
		assert_eq!(Balances::free_balance(2), 1_005_000_000);
		assert_eq!(Balances::free_balance(3), 1_005_000_000);
		assert_eq!(Balances::total_issuance(), issuance + 16_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(!BuysByAccount::<Test>::contains_key(1));
		assert_eq!(System::account(1).providers, providers);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::total_issuance(), issuance + 16_000_000);
	});
}

#[test]
fn failed_payment_rolls_back_its_account_without_blocking_bounded_payout_progress() {
	new_test_ext().execute_with(|| {
		let providers = System::account(5).providers;
		for account in [256, 255, 5] {
			if account != 5 {
				assert_ok!(Balances::mint_into(&account, 100_000_000));
			}
			assert_ok!(StabilizerBuys::record_transferred_purchase(&account, &purchase()));
		}
		start_frame(2);
		for account in [256, 255, 5] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(50)
			));
		}
		start_frame(3);
		let maturity = 1_000 + RewardMaturityTicks::get();
		CurrentTick::set(maturity);
		let issuance = Balances::total_issuance();
		FailPaymentFor::set(Some(5));
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::total_issuance(), issuance + 5_000_000);
		assert_eq!(Balances::free_balance(5), 0);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &5),
			100_000_000
		);
		assert_eq!(Budget::outstanding_rewards(), 5_000_000);
		assert_eq!(System::account(5).providers, providers);
		assert_eq!(BuysByAccount::<Test>::get(5).payable_reward, 5_000_000);
		assert_eq!(BuysByAccount::<Test>::get(5).next_payout_tick, Some(maturity + 1));
		assert_eq!(Balances::free_balance(255), 105_000_000);
		assert!(!BuysByAccount::<Test>::contains_key(255));
		assert_eq!(Balances::free_balance(256), 0);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &256),
			100_000_000
		);

		FailPaymentFor::set(None);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(5), 0);
		assert_eq!(Balances::free_balance(256), 105_000_000);
		assert!(!BuysByAccount::<Test>::contains_key(256));
		assert_eq!(Budget::outstanding_rewards(), 5_000_000);
		CurrentTick::set(maturity + 1);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::total_issuance(), issuance + 15_000_000);
		assert_eq!(Balances::free_balance(5), 105_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(System::account(5).providers, providers);
		assert!(PayoutQueue::<Test>::iter().next().is_none());
		StabilizerBuys::on_initialize(5);
		assert_eq!(Balances::total_issuance(), issuance + 15_000_000);
	});
}

#[test]
fn one_automatic_payment_collects_all_mature_groups_for_an_account() {
	new_test_ext().execute_with(|| {
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		CurrentTick::set(2_000);
		start_frame(3);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 2, Percent::from_percent(60)));
		CurrentTick::set(3_000);
		start_frame(4);
		CurrentTick::set(3_000 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_011_000_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(!BuysByAccount::<Test>::contains_key(1));
		assert!(PayoutQueue::<Test>::iter().next().is_none());
	});
}

#[test]
fn unavailable_prices_leave_delivered_principal_unencumbered() {
	new_test_ext().execute_with(|| {
		let _ = RecentPriceIndices::<Test>::clear(u32::MAX, None);
		let _ = HourlyPriceReferences::<Test>::clear(u32::MAX, None);
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		assert!(Auctions::<Test>::get(1).is_none());
		assert!(!BuysByAccount::<Test>::contains_key(1));
		assert!(PayoutQueue::<Test>::iter().next().is_none());
		System::assert_last_event(
			Event::<Test>::PurchaseIneligible {
				account: 1,
				purchase_tick: 1,
				reason: IneligibleReason::PricesUnavailable,
			}
			.into(),
		);
		HourlyPriceReferences::<Test>::insert(
			0u64.to_ordered(),
			HourlyPriceReference {
				minimum_argon_target_usd: FixedU128::zero(),
				usdc: Some((1, FixedU128::one())),
			},
		);
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		assert!(Auctions::<Test>::get(1).is_none());
	});
}

#[test]
fn failed_hold_rolls_back_admission_queue_and_provider_reference() {
	new_test_ext().execute_with(|| {
		let providers = System::account(4).providers;
		assert!(StabilizerBuys::record_transferred_purchase(&4, &purchase()).is_err());
		assert!(Auctions::<Test>::get(1).is_none());
		assert!(!BuysByAccount::<Test>::contains_key(4));
		assert!(PayoutQueue::<Test>::iter().next().is_none());
		assert_eq!(System::account(4).providers, providers);
	});
}

#[test]
fn sub_minimum_purchase_remains_spendable_instead_of_creating_an_unreleasable_hold() {
	new_test_ext().execute_with(|| {
		let mut purchase = purchase();
		purchase.principal = 1_000;
		purchase.usdc_spent = U256::from(920u64);
		purchase.sqrt_price_after_x96 =
			purchase.sqrt_price_before_x96 + purchase.sqrt_price_before_x96 / U256::from(100u64);
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase));
		System::assert_last_event(
			Event::<Test>::PurchaseIneligible {
				account: 1,
				purchase_tick: 1,
				reason: IneligibleReason::PrincipalBelowMinimum,
			}
			.into(),
		);
		assert_ok!(Balances::transfer(&1, &2, Balances::free_balance(1), Preservation::Expendable));
		purchase.principal = 10_000;
		purchase.usdc_spent = U256::from(9_200u64);
		assert_ok!(StabilizerBuys::record_transferred_purchase(&4, &purchase));
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &4), 10_000);
		start_frame(2);
		start_frame(3);
		CurrentTick::set(1_000 + RewardMaturityTicks::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(4), 10_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &4), 0);
		assert!(!BuysByAccount::<Test>::contains_key(1));
		assert!(!BuysByAccount::<Test>::contains_key(4));
	});
}

#[test]
fn four_positive_bids_allow_opt_out_but_never_restore_submission_count() {
	new_test_ext().execute_with(|| {
		assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &purchase()));
		assert_noop!(
			StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::one()),
			Error::<Test>::BiddingClosed
		);
		start_frame(2);
		assert_noop!(
			StabilizerBuys::bid(RuntimeOrigin::signed(2), 1, Percent::one()),
			Error::<Test>::EntryNotFound
		);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		assert_noop!(
			StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)),
			Error::<Test>::BidUnchanged
		);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::zero()));
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		for rate in [40, 30, 20] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(1),
				1,
				Percent::from_percent(rate)
			));
			assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		}
		assert_noop!(
			StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(10)),
			Error::<Test>::BidLimitReached
		);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::zero()));
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::zero()));
		let payout = PurchasesByAccount::<Test>::get(1, 1u64.to_ordered()).unwrap();
		assert_eq!(payout.positive_bid_count, 4);
		assert_eq!(payout.bid_rate, Percent::zero());
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert_noop!(
			StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::one()),
			Error::<Test>::BidLimitReached
		);
		assert_eq!(Balances::free_balance(2), 1_000_000_000);
		assert!(!BuysByAccount::<Test>::contains_key(2));
	});
}

#[test]
fn whole_purchase_discount_uses_actual_spend_and_accepts_crossing_target() {
	new_test_ext().execute_with(|| {
		let pool = Pool::get();
		let prices = Prices::get().unwrap();
		let mut p = purchase();
		assert_eq!(pool.captured_discount(&p, prices), Ok(10_000_000));
		p.usdc_spent = U256::from(90_000_001u64);
		assert_eq!(pool.captured_discount(&p, prices), Ok(9_999_999));
		p.usdc_spent = U256::from(101_000_000u64);
		assert_eq!(pool.captured_discount(&p, prices), Ok(0));
		p.usdc_spent = U256::from(50_000_000u64);
		p.sqrt_price_before_x96 = U256::one() << 75;
		assert_eq!(pool.captured_discount(&p, prices), Ok(50_000_000));
	});
}

#[test]
fn inverse_token_order_and_inclusive_cent_gate_use_raw_token_decimals() {
	new_test_ext().execute_with(|| {
		let mut pool = Pool::get();
		pool.argon_is_token0 = false;
		let mut p = purchase();
		// ARGON token1: at one dollar sqrt(raw ARGON / raw USDC) is 10^6 * Q96.
		p.sqrt_price_before_x96 = (U256::one() << 96) * U256::from(1_000_000u64);
		p.sqrt_price_after_x96 = p.sqrt_price_before_x96 - U256::one();
		let mut prices = Prices::get().unwrap();
		prices.argon_target_usd = FixedU128::from_rational(101, 100);
		assert_eq!(pool.captured_discount(&p, prices), Ok(10_891_089));
		prices.argon_target_usd = FixedU128::from_inner(prices.argon_target_usd.into_inner() - 1);
		assert_eq!(pool.captured_discount(&p, prices), Ok(0));
	});
}

#[test]
fn discount_uses_usdc_value_and_rounds_down() {
	new_test_ext().execute_with(|| {
		let pool = Pool::get();
		let mut p = purchase();
		let mut prices = Prices::get().unwrap();
		prices.usdc_usd = FixedU128::from_rational(8, 10);
		assert_eq!(pool.captured_discount(&p, prices), Ok(28_000_000));
		p.principal = 1_000_001;
		p.usdc_spent = U256::one();
		prices.usdc_usd = FixedU128::from_inner(FixedU128::DIV + 1);
		assert_eq!(pool.captured_discount(&p, prices), Ok(999_999));
		p.usdc_spent = U256::MAX;
		prices.argon_target_usd = FixedU128::from_inner(u128::MAX);
		prices.usdc_usd = FixedU128::from_inner(u128::MAX);
		assert_eq!(pool.captured_discount(&p, prices), Ok(0));
	});
}

#[test]
fn pool_identity_and_price_direction_are_required() {
	new_test_ext().execute_with(|| {
		let pool = Pool::get();
		let prices = Prices::get().unwrap();
		for p in [
			UniswapPurchase { chain_id: 2, ..purchase() },
			UniswapPurchase { pool_manager: H160::repeat_byte(4), ..purchase() },
			UniswapPurchase { pool_id: H256::repeat_byte(4), ..purchase() },
			UniswapPurchase { usdc_token: H160::repeat_byte(4), ..purchase() },
			UniswapPurchase { sqrt_price_after_x96: U256::one() << 76, ..purchase() },
			UniswapPurchase { sqrt_price_after_x96: U256::one() << 75, ..purchase() },
		] {
			assert_eq!(pool.captured_discount(&p, prices), Ok(0));
			assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &p));
			assert!(PurchasesByAccount::<Test>::iter_prefix(1).next().is_none());
		}
	});
}

#[test]
fn zero_budget_still_drains_positive_bids_and_releases_every_principal() {
	new_test_ext().execute_with(|| {
		unhashed::put(b"test_liquidity_buy_budget", &0u128);
		for account in [1, 2, 3] {
			assert_ok!(StabilizerBuys::record_transferred_purchase(&account, &purchase()));
		}
		start_frame(2);
		assert_eq!(Auctions::<Test>::get(1).unwrap().budget, Some(0));
		for account in [1, 2, 3] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(50)
			));
		}
		start_frame(3);
		CurrentTick::set(1_000 + RewardMaturityTicks::get());
		let issuance = Balances::total_issuance();
		StabilizerBuys::on_initialize(2);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(Balances::free_balance(3), 900_000_000);
		StabilizerBuys::on_initialize(3);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert!(MaturingAuctions::<Test>::iter().next().is_none());
		for account in [1, 2, 3] {
			assert_eq!(Balances::free_balance(account), 1_000_000_000);
		}
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(Budget::outstanding_rewards(), 0);
	});
}

#[test]
fn delayed_small_reward_survives_an_empty_account_until_it_can_receive_payment() {
	new_test_ext().execute_with(|| {
		let mut purchase = purchase();
		purchase.principal = 10_000;
		purchase.usdc_spent = U256::from(9_000u64);
		for account in [1, 2, 3] {
			assert_ok!(StabilizerBuys::record_transferred_purchase(&account, &purchase));
		}
		start_frame(2);
		for (account, rate) in [(1, 60), (2, 50), (3, 50)] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(rate)
			));
		}
		start_frame(3);
		let maturity = 1_000 + RewardMaturityTicks::get();
		CurrentTick::set(maturity);
		let issuance = Balances::total_issuance();
		StabilizerBuys::on_initialize(2);
		assert!(!BuysByAccount::<Test>::contains_key(1));
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_ok!(Balances::transfer(&1, &4, Balances::free_balance(1), Preservation::Expendable));
		assert_eq!(System::account(1).providers, 0);

		StabilizerBuys::on_initialize(3);
		let buys = BuysByAccount::<Test>::get(1);
		assert!(PurchasesByAccount::<Test>::iter_prefix(1).next().is_none());
		assert_eq!(buys.payable_reward, 600);
		assert_eq!(buys.next_payout_tick, Some(maturity + 1));
		assert_eq!(Balances::free_balance(1), 0);
		assert_eq!(Budget::outstanding_rewards(), 600);
		assert_eq!(Balances::total_issuance(), issuance + 1_000);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());

		assert_ok!(Balances::transfer(&4, &1, 10_000, Preservation::Expendable));
		CurrentTick::set(maturity + 1);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::free_balance(1), 10_600);
		assert_eq!(Balances::total_issuance(), issuance + 1_600);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(!BuysByAccount::<Test>::contains_key(1));
		StabilizerBuys::on_initialize(5);
		assert_eq!(Balances::free_balance(1), 10_600);
	});
}
