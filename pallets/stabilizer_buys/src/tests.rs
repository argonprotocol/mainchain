use crate::{mock::*, *};
use argon_primitives::{AscendingRankKey, GatewayState, PriceIndexUpdated, RValueHighWaterMark};
use frame_support::{
	assert_noop, assert_ok,
	storage::{unhashed, with_storage_layer},
	traits::Hooks,
};

#[test]
fn cheap_bids_do_not_admit_the_excluded_matching_basis() {
	new_test_ext().execute_with(|| {
		for account in 10..20 {
			assert_ok!(Balances::mint_into(&account, 100_000_000));
			assert_ok!(record_purchase(&account, &purchase()));
		}
		start_frame(2);
		for account in 10..20 {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(50)
			));
		}
		start_frame(3);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		for block in 2..10 {
			StabilizerBuys::on_initialize(block);
		}

		for account in 10..19 {
			assert_eq!(Balances::free_balance(account), 105_000_000);
		}
		assert_eq!(Balances::free_balance(19), 100_000_000);
		assert_eq!(Budget::available_budget(), 55_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(AuctionAllocationQueue::<Test>::iter().next().is_none());
		assert!(PayoutQueue::<Test>::iter().next().is_none());
	});
}

#[test]
fn lowering_a_bid_moves_its_matching_basis_ahead_of_the_cutoff() {
	new_test_ext().execute_with(|| {
		for account in [1, 2] {
			assert_ok!(record_purchase(&account, &purchase()));
		}
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(2), 1, Percent::from_percent(60)));
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(2), 1, Percent::from_percent(49)));
		start_frame(3);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);

		assert_eq!(Balances::free_balance(2), 1_004_900_000);
		assert_eq!(Balances::free_balance(1), 1_004_000_000);
		assert_eq!(Budget::available_budget(), 91_100_000);
	});
}

#[test]
fn a_lone_bidder_is_paid_their_rate_on_only_the_included_matching_basis() {
	new_test_ext().execute_with(|| {
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		start_frame(3);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);

		assert_eq!(Balances::free_balance(1), 1_004_500_000);
		assert_eq!(Budget::available_budget(), 95_500_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
	});
}

#[test]
fn a_lone_full_rate_bid_receives_ninety_percent_of_its_matching_basis() {
	new_test_ext().execute_with(|| {
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::one()));
		start_frame(3);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);

		assert_eq!(Balances::free_balance(1), 1_009_000_000);
		assert_eq!(Budget::available_budget(), 91_000_000);
	});
}

#[test]
fn equal_rate_bids_prioritize_larger_matching_bases_and_partially_include_the_boundary() {
	new_test_ext().execute_with(|| {
		let mut smaller_purchase = purchase();
		smaller_purchase.usdc_spent = U256::from(95_000_000u64);
		assert_ok!(record_purchase(&1, &smaller_purchase));
		assert_ok!(record_purchase(&2, &purchase()));
		start_frame(2);
		for account in [1, 2] {
			assert_ok!(StabilizerBuys::bid(
				RuntimeOrigin::signed(account),
				1,
				Percent::from_percent(50)
			));
		}
		start_frame(3);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);

		assert_eq!(Balances::free_balance(2), 1_005_000_000);
		assert_eq!(Balances::free_balance(1), 1_001_750_000);
		assert_eq!(Budget::available_budget(), 93_250_000);
	});
}

#[test]
fn compact_quotes_preserve_target_market_and_usd_valuations() {
	new_test_ext().execute_with(|| {
		let mut index = price_index(1);
		index.argon_usd_target_price = FixedU128::from_rational(5, 4);
		let reference = PurchasePriceReference::from_price_index(&index).unwrap();
		assert_eq!(reference.r_value, FixedU128::from_rational(4, 5));
		assert_eq!(
			reference.microgons_at_target_per_btc,
			FixedU128::saturating_from_integer(48_000_000_000u128)
		);
		assert_eq!(reference.microgons_at_target_per_usdc, FixedU128::from_u32(800_000));
		assert_eq!(
			reference.microgons_at_target_per_btc.checked_div(&reference.r_value).unwrap(),
			FixedU128::saturating_from_integer(60_000_000_000u128)
		);
		assert_eq!(
			reference
				.microgons_at_target_per_btc
				.checked_mul(&reference.argon_usd_target_price)
				.unwrap(),
			FixedU128::saturating_from_integer(60_000_000_000u128)
		);
	});
}

#[test]
fn history_lookup_requires_a_fresh_observation_before_the_purchase() {
	new_test_ext().execute_with(|| {
		let _ = PriceHistoryByTick::<Test>::clear(u32::MAX, None);
		StabilizerBuys::on_price_index(&price_index(10));
		CurrentTick::set(30);
		assert!(StabilizerBuys::price_reference(9).is_none());
		assert!(StabilizerBuys::price_reference(20).is_some());
		assert!(StabilizerBuys::price_reference(21).is_none());
		assert!(StabilizerBuys::price_reference(31).is_none());

		let mut unavailable = price_index(11);
		unavailable.usdc_usd_price = None;
		StabilizerBuys::on_price_index(&unavailable);
		assert!(StabilizerBuys::price_reference(11)
			.unwrap()
			.1
			.microgons_at_target_per_usdc
			.is_zero());
		assert!(StabilizerBuys::price_reference(12)
			.unwrap()
			.1
			.microgons_at_target_per_usdc
			.is_zero());
		assert!(StabilizerBuys::price_reference(10).is_some());
		unavailable.tick = 13;
		unavailable.argon_usd_target_price = FixedU128::zero();
		StabilizerBuys::on_price_index(&unavailable);
		assert!(StabilizerBuys::price_reference(13).is_none());

		let mut future = price_index(31);
		future.usdc_usd_price = None;
		StabilizerBuys::on_price_index(&future);
		CurrentTick::set(31);
		assert!(StabilizerBuys::price_reference(31).is_none());
	});
}

#[test]
fn price_history_survives_sync_stalls_then_prunes_in_bounded_steps() {
	new_test_ext().execute_with(|| {
		for tick in 1..=800 {
			StabilizerBuys::on_price_index(&price_index(tick));
		}
		CurrentTick::set(800);
		StabilizerBuys::on_initialize(2);
		assert!(StabilizerBuys::price_reference(1).is_some());
		assert_eq!(PriceHistoryByTick::<Test>::iter_keys().count(), 800);

		LastGatewayState::set(Some(GatewayState {
			last_synced_source_block_time: Some(801_000),
			..Default::default()
		}));
		StabilizerBuys::on_initialize(3);
		assert_eq!(PriceHistoryByTick::<Test>::iter_keys().count(), 800);

		LastGatewayState::set(Some(GatewayState {
			last_synced_source_block_time: Some(50u64 * 1_000),
			..Default::default()
		}));
		StabilizerBuys::on_initialize(4);
		assert_eq!(PriceHistoryByTick::<Test>::iter_keys().count(), 784);
		assert!(PriceHistoryByTick::<Test>::contains_key(17u64.to_ordered()));
		for block in 5..=6 {
			StabilizerBuys::on_initialize(block);
		}
		assert!(!PriceHistoryByTick::<Test>::contains_key(39u64.to_ordered()));
		assert!(PriceHistoryByTick::<Test>::contains_key(40u64.to_ordered()));
		assert!(StabilizerBuys::price_reference(50).is_some());
		assert!(StabilizerBuys::price_reference(800).is_some());
	});
}

#[test]
fn repayment_keeps_history_until_its_window_is_fixed() {
	new_test_ext().execute_with(|| {
		let mut index = price_index(1);
		index.argon_usd_price = FixedU128::from_rational(6, 10);
		StabilizerBuys::on_price_index(&index);
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		assert_ok!(record_purchase(&1, &buy));
		LastGatewayState::set(Some(GatewayState {
			last_synced_source_block_time: Some(1_000u64 * 1_000),
			..Default::default()
		}));
		StabilizerBuys::on_initialize(2);
		assert!(PriceHistoryByTick::<Test>::contains_key(1u64.to_ordered()));
		assert_eq!(StabilizerBuys::oldest_pending_purchase_tick(), Some(1));

		CurrentFrame::set(31);
		FailRepayment::set(true);
		StabilizerBuys::on_initialize(3);
		assert_eq!(StabilizerBuys::oldest_pending_purchase_tick(), None);
		StabilizerBuys::on_initialize(4);
		assert!(!PriceHistoryByTick::<Test>::contains_key(1u64.to_ordered()));
		let repay =
			LiquidRepaysByAccountAndGatewayNonce::<Test>::get(1, 1u64.to_ordered()).unwrap();
		assert_eq!(
			repay.microgons_at_target_per_btc,
			FixedU128::saturating_from_integer(60_000_000_000u128)
		);
		assert_eq!(repay.purchase_r_value, FixedU128::from_rational(6, 10));
	});
}

#[test]
fn repayment_requires_btc_and_r_capture_but_usdc_only_controls_reward_eligibility() {
	for (case, expects_repayment, expects_reward) in [
		("missing", false, false),
		("bitcoin", false, true),
		("argon", false, true),
		("liquidity", false, true),
		("usdc", true, false),
	] {
		new_test_ext().execute_with(|| {
			let mut index = price_index(1);
			match case {
				"bitcoin" => index.btc_usd_price = FixedU128::zero(),
				"argon" => index.argon_usd_price = FixedU128::zero(),
				"liquidity" => index.argon_time_weighted_average_liquidity = 0,
				"usdc" => index.usdc_usd_price = None,
				_ => {},
			}
			StabilizerBuys::on_price_index(&index);
			if case == "missing" {
				PriceHistoryByTick::<Test>::remove(1u64.to_ordered());
			}

			let mut buy = purchase();
			buy.repay_liquid_id = Some(42);
			assert_ok!(record_purchase(&1, &buy));
			assert_eq!(StabilizerBuys::has_pending_repayment(&1, 42), expects_repayment);
			assert_eq!(
				LiquidRepaysByAccountAndGatewayNonce::<Test>::contains_key(1, 1u64.to_ordered()),
				expects_repayment,
			);
			assert_eq!(
				BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered())
					.is_some_and(|frame_buys| frame_buys.captured_discount > 0),
				expects_reward,
			);
			let held = if expects_repayment || expects_reward { buy.principal } else { 0 };
			assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), held);
			assert_eq!(StabilizerBuys::oldest_pending_purchase_tick(), None);
		});
	}
}

#[test]
fn repayment_without_a_premium_skips_recovery_but_keeps_the_full_hold() {
	new_test_ext().execute_with(|| {
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		assert_ok!(record_purchase(&1, &buy));
		let repay =
			LiquidRepaysByAccountAndGatewayNonce::<Test>::get(1, 1u64.to_ordered()).unwrap();
		assert_eq!(repay.price_window.next_frame, None);
		assert_eq!(StabilizerBuys::oldest_pending_purchase_tick(), None);
		start_frame(2);
		start_frame(3);

		CurrentFrame::set(30);
		StabilizerBuys::on_initialize(2);
		assert!(StabilizerBuys::has_pending_repayment(&1, 42));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			buy.principal
		);

		CurrentFrame::set(31);
		StabilizerBuys::on_initialize(3);
		assert!(!StabilizerBuys::has_pending_repayment(&1, 42));
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
	});
}

#[test]
fn waiting_repayments_do_not_block_other_accounts_and_the_queue_wraps_after_removal() {
	new_test_ext().execute_with(|| {
		// Account 1 also has an earlier ordinary buy: its funds must wait with its repayment.
		CurrentFrame::set(0);
		StabilizerBuys::on_price_index(&price_index(0));
		assert_ok!(record_purchase(&1, &UniswapPurchase { tick: 0, ..purchase() }));
		start_frame(1);

		MaxPriceWindowSteps::set(1);
		for tick in 1..=10 {
			let mut index = price_index(tick);
			index.argon_usd_price = FixedU128::from_rational(6, 10);
			index.usdc_usd_price = None;
			StabilizerBuys::on_price_index(&index);
		}
		FrameHighs::mutate(|highs| {
			highs.insert(
				1,
				RValueHighWaterMark {
					r_value: FixedU128::from_rational(6, 10),
					first_tick: 0,
					end_tick: Some(20),
				},
			);
		});
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		for account in [1, 2] {
			assert_ok!(record_purchase(&account, &buy));
		}
		StabilizerBuys::on_price_index(&price_index(11));
		let ordinary = UniswapPurchase { tick: 11, ..purchase() };
		assert_ok!(record_purchase(&3, &ordinary));
		start_frame(2);
		start_frame(3);
		CurrentFrame::set(31);

		StabilizerBuys::on_initialize(2);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &3),
			ordinary.principal
		);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &3), 0);
		assert_eq!(Balances::free_balance(3), 1_009_000_000);
		for (account, held_principal) in [(1, buy.principal * 2), (2, buy.principal)] {
			assert!(StabilizerBuys::has_pending_repayment(&account, 42));
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &account),
				held_principal
			);
		}
		assert_eq!(PayoutStateByAccount::<Test>::get(1).payable_reward, 9_000_000);
		assert_eq!(Balances::free_balance(1), 800_000_000);

		MaxPriceWindowSteps::set(64);
		for block in 4..=6 {
			StabilizerBuys::on_initialize(block);
		}
		assert!(PayoutQueue::<Test>::iter().next().is_none());
		assert!(PayoutQueueCursor::<Test>::get().is_none());
		assert!(PendingPriceWindowCountByTick::<Test>::iter().next().is_none());
		for (account, expected_balance) in [(1, 1_009_000_000), (2, 1_000_000_000)] {
			assert!(!StabilizerBuys::has_pending_repayment(&account, 42));
			assert_eq!(Balances::free_balance(account), expected_balance);
		}
	});
}

#[test]
fn delayed_purchase_window_excludes_earlier_spikes_and_maturity_frame_through_retries() {
	for (recovery_price, last_price, expected_high) in [(80, 85, 85), (90, 110, 90)] {
		new_test_ext().execute_with(|| {
			MaxPriceWindowSteps::set(2);
			let mut highs = std::collections::BTreeMap::new();
			for (frame, tick, price) in
				[(1, 100, 200), (1, 110, 60), (1, 120, 70), (2, 210, recovery_price), (9, 900, 80)]
			{
				CurrentFrame::set(frame);
				CurrentTick::set(tick);
				let mut index = price_index(tick);
				index.argon_usd_price = FixedU128::from_rational(price, 100);
				if tick == 120 {
					index.usdc_usd_price = None;
				}
				StabilizerBuys::on_price_index(&index);
				highs
					.entry(frame)
					.and_modify(|high: &mut RValueHighWaterMark| {
						high.r_value = high.r_value.max(index.argon_usd_price);
					})
					.or_insert(RValueHighWaterMark {
						r_value: index.argon_usd_price,
						first_tick: tick,
						end_tick: Some((frame + 1) * 100),
					});
			}
			FrameHighs::set(highs);
			CurrentFrame::set(10);
			CurrentTick::set(1_000);
			let mut buy = purchase();
			buy.tick = 110;
			buy.repay_liquid_id = Some(42);
			assert_ok!(record_purchase(&1, &buy));
			assert_eq!(
				LiquidRepaysByAccountAndGatewayNonce::<Test>::get(1, 1u64.to_ordered())
					.unwrap()
					.price_window
					.next_frame,
				Some(1),
			);
			start_frame(11);
			start_frame(12);

			CurrentFrame::set(39);
			CurrentTick::set(3_900);
			let mut last = price_index(3_900);
			last.argon_usd_price = FixedU128::from_rational(last_price, 100);
			StabilizerBuys::on_price_index(&last);
			FrameHighs::mutate(|highs| {
				highs.insert(
					39,
					RValueHighWaterMark {
						r_value: last.argon_usd_price,
						first_tick: 3_900,
						end_tick: Some(4_000),
					},
				);
			});
			StabilizerBuys::on_initialize(2);
			assert_eq!(
				LiquidRepaysByAccountAndGatewayNonce::<Test>::get(1, 1u64.to_ordered())
					.unwrap()
					.price_window
					.r_value_high_water,
				FixedU128::zero()
			);

			CurrentFrame::set(40);
			CurrentTick::set(4_000);
			let mut excluded = price_index(4_000);
			excluded.argon_usd_price = FixedU128::from_u32(9);
			StabilizerBuys::on_price_index(&excluded);
			FrameHighs::mutate(|highs| {
				highs.insert(
					40,
					RValueHighWaterMark {
						r_value: excluded.argon_usd_price,
						first_tick: 4_000,
						end_tick: None,
					},
				);
			});
			LastGatewayState::set(Some(GatewayState {
				last_synced_source_block_time: Some(4_000_000),
				..Default::default()
			}));
			FailRepayment::set(true);
			StabilizerBuys::on_initialize(3);
			let partial =
				LiquidRepaysByAccountAndGatewayNonce::<Test>::get(1, 1u64.to_ordered()).unwrap();
			assert!(partial.price_window.next_frame.is_some());
			assert_eq!(partial.price_window.r_value_high_water, FixedU128::from_rational(7, 10));
			assert_eq!(StabilizerBuys::oldest_pending_purchase_tick(), Some(110));
			assert!(PriceHistoryByTick::<Test>::contains_key(110u64.to_ordered()));

			for block in 4..=7 {
				StabilizerBuys::on_initialize(block);
			}
			let fixed =
				LiquidRepaysByAccountAndGatewayNonce::<Test>::get(1, 1u64.to_ordered()).unwrap();
			assert_eq!(fixed.price_window.next_frame, None);
			assert_eq!(
				fixed.price_window.r_value_high_water,
				FixedU128::from_rational(expected_high, 100)
			);
			assert_eq!(StabilizerBuys::oldest_pending_purchase_tick(), None);
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
				buy.principal
			);

			CurrentFrame::set(45);
			CurrentTick::set(4_500);
			let mut late = price_index(4_500);
			late.argon_usd_price = FixedU128::from_u32(20);
			StabilizerBuys::on_price_index(&late);
			FrameHighs::mutate(|highs| {
				highs.insert(
					45,
					RValueHighWaterMark {
						r_value: late.argon_usd_price,
						first_tick: 4_500,
						end_tick: None,
					},
				);
			});
			StabilizerBuys::on_initialize(8);
			assert_eq!(
				LiquidRepaysByAccountAndGatewayNonce::<Test>::get(1, 1u64.to_ordered()).unwrap(),
				fixed
			);
			assert!(!PriceHistoryByTick::<Test>::contains_key(110u64.to_ordered()));

			FailRepayment::set(false);
			CurrentFrame::set(46);
			StabilizerBuys::on_initialize(9);
			assert!(!LiquidRepaysByAccountAndGatewayNonce::<Test>::contains_key(
				1,
				1u64.to_ordered()
			));
			assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		});
	}
}

#[test]
fn outer_failure_rolls_back_completed_reward_allocation_and_payment() {
	new_test_ext().execute_with(|| {
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		start_frame(3);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		let issuance = Balances::total_issuance();

		assert_noop!(
			with_storage_layer(|| -> DispatchResult {
				StabilizerBuys::on_initialize(2);
				assert_eq!(Balances::free_balance(1), 1_004_500_000);
				assert_eq!(Balances::total_issuance(), issuance + 4_500_000);
				assert_eq!(Budget::available_budget(), 95_500_000);
				assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
				assert!(AuctionAllocationQueue::<Test>::iter().next().is_none());
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
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining_budget, 9_000_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(AuctionAllocationQueue::<Test>::iter().count(), 1);
		assert_eq!(PayoutQueue::<Test>::iter().count(), 1);

		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 1_004_500_000);
		assert_eq!(Balances::total_issuance(), issuance + 4_500_000);
		assert_eq!(Budget::available_budget(), 95_500_000);
	});
}

#[test]
fn purchases_aggregate_by_recipient_and_admission_frame() {
	new_test_ext().execute_with(|| {
		start_frame(10);
		assert_ok!(record_purchase(&1, &purchase()));
		assert_ok!(record_purchase(&1, &purchase()));
		assert!(LiquidRepaysByAccountAndGatewayNonce::<Test>::iter().next().is_none());
		assert!(Auctions::<Test>::get(1).is_none());
		let frame_buys = BuysByAccountAndFrame::<Test>::get(1, 10u64.to_ordered()).unwrap();
		assert_eq!(BuysByAccountAndFrame::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(frame_buys.principal, 200_000_000);
		assert_eq!(frame_buys.captured_discount, 20_000_000);
		assert_eq!(Auctions::<Test>::get(10).unwrap().total_matching_basis, 20_000_000);

		start_frame(11);
		StabilizerBuys::on_initialize(2);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			200_000_000
		);
		assert_ok!(record_purchase(&1, &purchase()));
		assert_eq!(BuysByAccountAndFrame::<Test>::iter_prefix(1).count(), 2);
		assert_eq!(
			BuysByAccountAndFrame::<Test>::get(1, 11u64.to_ordered())
				.unwrap()
				.captured_discount,
			10_000_000
		);
		assert_eq!(
			BuysByAccountAndFrame::<Test>::get(1, 10u64.to_ordered())
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
		assert_ok!(record_purchase(&1, &purchase()));
		assert_ok!(StabilizerBuys::set_matching_rate(RuntimeOrigin::root(), Permill::one()));
		assert_ok!(record_purchase(&2, &purchase()));
		assert_eq!(Auctions::<Test>::get(1).unwrap().total_matching_basis, 10_000_000);

		start_frame(2);
		let auction = Auctions::<Test>::get(1).unwrap();
		assert_eq!(auction.matching_rate, Permill::from_percent(50));
		assert_eq!(auction.budget, Some(9_000_000));
		assert_eq!(auction.remaining_budget, 9_000_000);
		assert_ok!(record_purchase(&2, &purchase()));
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
			assert_ok!(record_purchase(&1, &purchase));
		}
		assert_eq!(
			BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered())
				.unwrap()
				.captured_discount,
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
			BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered())
				.unwrap()
				.positive_bid_count,
			0
		);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::one()));
		assert_ok!(StabilizerBuys::set_matching_rate(RuntimeOrigin::root(), Permill::zero()));
		assert_ok!(record_purchase(&1, &purchase));
		assert_ok!(StabilizerBuys::set_matching_rate(RuntimeOrigin::root(), Permill::one()));
		assert_ok!(record_purchase(&2, &purchase));
		start_frame(3);
		assert_eq!(Auctions::<Test>::get(2).unwrap().total_matching_basis, 0);
		assert_eq!(Auctions::<Test>::get(2).unwrap().budget, Some(0));

		let mut repay = purchase.clone();
		repay.repay_liquid_id = Some(42);
		RepaymentBurn::set(repay.principal);
		assert_ok!(record_purchase(&3, &repay));
		start_frame(4);
		start_frame(5);
		assert_eq!(Auctions::<Test>::get(3).unwrap().total_matching_basis, 1);
		assert_eq!(Auctions::<Test>::get(3).unwrap().budget, Some(0));

		let issuance = Balances::total_issuance();
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::total_issuance(), issuance + 1);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			purchase.principal
		);
		assert_eq!(Budget::outstanding_rewards(), 0);

		CurrentFrame::set(3 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(3);
		StabilizerBuys::on_initialize(4);
		assert!(!StabilizerBuys::has_pending_repayment(&3, 42));
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &3), 0);
		assert_eq!(Balances::total_issuance(), issuance + 1 - repay.principal);
	});
}

#[test]
fn reward_and_principal_pay_automatically_at_maturity_and_never_before_it() {
	new_test_ext().execute_with(|| {
		let issuance = Balances::total_issuance();
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_eq!(Budget::available_budget(), 91_000_000);
		assert_ok!(Budget::return_budget(100_000_000));
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		CurrentTick::set(2_000);
		start_frame(3);
		let maturity = 1 + PurchaseHoldFrames::get();
		assert_eq!(Auctions::<Test>::get(1).unwrap().matures_at_frame, maturity);
		assert!(AuctionAllocationQueue::<Test>::contains_key(AscendingRankKey::new(maturity, 1)));
		CurrentFrame::set(maturity - 1);
		StabilizerBuys::on_initialize(2);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(Balances::free_balance(1), 900_000_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(Budget::available_budget(), 191_000_000);

		CurrentFrame::set(maturity);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::total_issuance(), issuance + 4_500_000);
		assert_eq!(Balances::free_balance(1), 1_004_500_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Budget::available_budget(), 195_500_000);
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining_budget, 0);
		assert!(AuctionAllocationQueue::<Test>::iter().next().is_none());
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::total_issuance(), issuance + 4_500_000);
	});
}

#[test]
fn bounded_allocation_ranks_bids_and_shares_the_funding_shortfall_proportionally() {
	new_test_ext().execute_with(|| {
		unhashed::put(b"test_liquidity_buy_budget", &9_000_000u128);
		for account in [256, 255, 1] {
			if account != 1 {
				assert_ok!(Balances::mint_into(&account, 100_000_000));
			}
			assert_ok!(record_purchase(&account, &purchase()));
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
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_eq!(Balances::free_balance(255), 101_666_666);
		assert_eq!(Balances::free_balance(256), 0);
		assert_eq!(PayoutStateByAccount::<Test>::get(256).payable_reward, 1_666_666);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(Budget::available_budget(), 0);

		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(256), 101_666_666);
		assert_eq!(Balances::free_balance(1), 1_001_400_000);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert!(AuctionAllocationQueue::<Test>::iter().next().is_none());
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Budget::available_budget(), 4_266_668);
	});
}

#[test]
fn default_full_request_earns_ordinary_rewards_and_releases_principal_at_maturity() {
	new_test_ext().execute_with(|| {
		let providers = System::account(5).providers;
		assert_ok!(record_purchase(&5, &purchase()));
		assert_eq!(Balances::free_balance(5), 0);
		// Balances drops its free-balance provider when the whole balance moves onto hold.
		assert_eq!(System::account(5).providers, providers);
		start_frame(2);
		CurrentTick::set(2_000);
		start_frame(3);
		let maturity = 1 + PurchaseHoldFrames::get();
		CurrentFrame::set(PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(PayoutStateByAccount::<Test>::get(5).next_payout_frame, Some(maturity));
		assert!(PayoutQueue::<Test>::contains_key(AscendingRankKey::new(maturity, 5)));
		CurrentFrame::set(maturity - 1);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Budget::available_budget(), 91_000_000);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &5),
			100_000_000
		);
		CurrentFrame::set(maturity);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Budget::available_budget(), 91_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Balances::free_balance(5), 109_000_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &5), 0);
		assert_eq!(System::account(5).providers, providers);
		assert!(!PayoutStateByAccount::<Test>::contains_key(5));
		assert!(PayoutQueue::<Test>::iter().next().is_none());
		assert!(AuctionAllocationQueue::<Test>::iter().next().is_none());
	});
}

#[test]
fn elapsed_ticks_and_delayed_processing_do_not_extend_the_admission_frame_hold() {
	new_test_ext().execute_with(|| {
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		CurrentTick::set(2_000);
		start_frame(3);
		CurrentTick::set(2_000 + 60 * 1_440);
		CurrentFrame::set(1 + PurchaseHoldFrames::get() + 5);
		assert_eq!(
			Auctions::<Test>::get(1).unwrap().matures_at_frame,
			1 + PurchaseHoldFrames::get()
		);
		assert_eq!(AuctionAllocationQueue::<Test>::iter().count(), 1);
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_004_500_000);
		CurrentFrame::set(1 + PurchaseHoldFrames::get() + 6);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 1_004_500_000);
		assert_eq!(Budget::available_budget(), 95_500_000);
		assert!(AuctionAllocationQueue::<Test>::iter().next().is_none());
	});
}

#[test]
fn failed_last_bid_return_rolls_back_that_allocation_and_retries_without_double_payment() {
	new_test_ext().execute_with(|| {
		for account in [1, 2, 3] {
			assert_ok!(record_purchase(&account, &purchase()));
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
		let maturity = 1 + PurchaseHoldFrames::get();
		CurrentFrame::set(maturity);
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
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining_budget, 17_000_000);
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining_matching_basis, 7_000_000);
		assert!(AuctionAllocationQueue::<Test>::contains_key(AscendingRankKey::new(
			maturity + 1,
			1
		)));

		FailReturn::set(false);
		StabilizerBuys::on_initialize(4);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		CurrentFrame::set(maturity + 1);
		StabilizerBuys::on_initialize(5);
		assert_eq!(Balances::free_balance(3), 1_003_500_000);
		assert_eq!(Balances::total_issuance(), issuance + 13_500_000);
		assert_eq!(Budget::available_budget(), 86_500_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(AuctionAllocationQueue::<Test>::iter().next().is_none());
	});
}

#[test]
fn failed_award_preserves_its_rank_and_funding_while_prior_awards_stay_paid() {
	new_test_ext().execute_with(|| {
		for account in [1, 2, 3] {
			assert_ok!(record_purchase(&account, &purchase()));
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
		let maturity = 1 + PurchaseHoldFrames::get();
		CurrentFrame::set(maturity);
		FailCommitAt::set(Some(10_000_000));
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_005_000_000);
		assert_eq!(Balances::free_balance(2), 1_000_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Budget::available_budget(), 73_000_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 2);
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining_budget, 22_000_000);
		assert_eq!(Auctions::<Test>::get(1).unwrap().remaining_matching_basis, 17_000_000);
		assert!(AuctionAllocationQueue::<Test>::contains_key(AscendingRankKey::new(
			maturity + 1,
			1
		)));
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(3), 1_000_000_000);

		FailCommitAt::set(None);
		CurrentFrame::set(maturity + 1);
		StabilizerBuys::on_initialize(4);
		for account in [1, 2] {
			assert_eq!(Balances::free_balance(account), 1_005_000_000);
		}
		assert_eq!(Balances::free_balance(3), 1_003_500_000);
		assert_eq!(Budget::available_budget(), 86_500_000);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
	});
}

#[test]
fn a_failed_auction_does_not_block_another_mature_auction() {
	new_test_ext().execute_with(|| {
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(60)));
		assert_ok!(record_purchase(&2, &purchase()));
		start_frame(3);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(2), 2, Percent::from_percent(50)));
		start_frame(4);
		let maturity = 2 + PurchaseHoldFrames::get();
		CurrentFrame::set(maturity);
		FailCommitAt::set(Some(5_400_000));
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_eq!(Balances::free_balance(2), 1_004_500_000);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert!(BidsByAuction::<Test>::iter_prefix(2).next().is_none());
		assert_eq!(Budget::available_budget(), 86_500_000);

		FailCommitAt::set(None);
		CurrentFrame::set(maturity + 1);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 1_005_400_000);
		assert_eq!(Budget::available_budget(), 90_100_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
	});
}

#[test]
fn failed_treasury_funding_still_allocates_default_burn_rewards_at_maturity() {
	new_test_ext().execute_with(|| {
		RepaymentBurn::set(40_000_000);
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		for account in [1, 2] {
			assert_ok!(record_purchase(&account, &buy));
		}
		FailReserve::set(true);
		start_frame(2);
		FailReserve::set(false);
		assert_noop!(
			StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::one()),
			Error::<Test>::BiddingClosed
		);
		assert_eq!(Budget::available_budget(), 100_000_000);
		assert_eq!(Auctions::<Test>::get(1).unwrap().budget, None);
		assert_eq!(
			BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered())
				.unwrap()
				.positive_bid_count,
			0
		);
		CurrentTick::set(2_000);
		start_frame(3);
		CurrentFrame::set(PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			100_000_000
		);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(3);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::free_balance(1), 962_000_000);
		assert_eq!(Balances::free_balance(2), 961_600_000);
		assert!(!StabilizerBuys::has_pending_repayment(&2, 42));
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(!StabilizerBuys::has_pending_repayment(&1, 42));
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
	});
}

#[test]
fn opting_out_preserves_principal_while_funded_bids_share_the_shortfall() {
	new_test_ext().execute_with(|| {
		unhashed::put(b"test_liquidity_buy_budget", &5_000_000u128);
		for account in [1, 2, 3] {
			assert_ok!(record_purchase(&account, &purchase()));
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
		CurrentFrame::set(PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);
		for account in [1, 2, 3] {
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &account),
				100_000_000
			);
		}
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(3);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::free_balance(1), 1_000_925_925);
		assert_eq!(Balances::free_balance(2), 1_001_111_111);
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
			assert_ok!(record_purchase(&1, &purchase));
		}
		for account in 10..1_011 {
			assert_ok!(Balances::mint_into(&account, 20_000));
			assert_ok!(record_purchase(&account, &purchase));
		}
		assert_eq!(BuysByAccountAndFrame::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(
			BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered()).unwrap().principal,
			10_010_000
		);
		assert_eq!(PayoutStateByAccount::<Test>::iter().count(), 1_002);
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
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		let issuance = Balances::total_issuance();
		StabilizerBuys::on_initialize(2);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1_000);
		assert_eq!(PayoutStateByAccount::<Test>::iter().count(), 1_000);
		for block in 3..=502 {
			StabilizerBuys::on_initialize(block);
		}
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert!(PayoutStateByAccount::<Test>::iter().next().is_none());
		assert!(AuctionAllocationQueue::<Test>::iter().next().is_none());
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Balances::total_issuance(), issuance + 900_900);
		assert_eq!(Balances::free_balance(1_010), 20_000);
	});
}

#[test]
fn automatic_payout_preserves_future_groups_and_one_provider() {
	new_test_ext().execute_with(|| {
		let providers = System::account(1).providers;
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(record_purchase(&1, &purchase()));
		CurrentTick::set(2_000);
		start_frame(3);
		CurrentTick::set(3_000);
		start_frame(4);
		assert_eq!(System::account(1).providers, providers + 1);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);
		let payout_state = PayoutStateByAccount::<Test>::get(1);
		assert_eq!(BuysByAccountAndFrame::<Test>::iter_prefix(1).count(), 1);
		assert!(BuysByAccountAndFrame::<Test>::contains_key(1, 2u64.to_ordered()));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			100_000_000
		);
		assert_eq!(System::account(1).providers, providers + 1);
		assert_eq!(payout_state.next_payout_frame, Some(2 + PurchaseHoldFrames::get()));
		assert_eq!(PayoutQueue::<Test>::iter().count(), 1);
		CurrentFrame::set(2 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(3);
		assert_eq!(System::account(1).providers, providers);
		assert_eq!(Balances::free_balance(1), 1_018_000_000);
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
	});
}

#[test]
fn delayed_payouts_release_a_bounded_number_of_mature_buckets_per_visit() {
	new_test_ext().execute_with(|| {
		let providers = System::account(1).providers;
		let mut purchase = purchase();
		purchase.principal = 10_000;
		purchase.usdc_spent = U256::from(9_000u64);
		// A delayed account visit can leave mature buckets queued across many frames.
		for frame in 1..=40 {
			CurrentTick::set(1_000 + (frame - 1) * 6);
			start_frame(frame);
			if frame == 2 {
				assert_ok!(StabilizerBuys::bid(
					RuntimeOrigin::signed(1),
					1,
					Percent::from_percent(50)
				));
			} else if frame > 2 {
				assert_ok!(StabilizerBuys::bid(
					RuntimeOrigin::signed(1),
					frame - 1,
					Percent::zero()
				));
			}
			assert_ok!(record_purchase(&1, &purchase));
			System::reset_events();
		}
		assert!(Auctions::<Test>::get(1).unwrap().matures_at_frame <= CurrentFrame::get());
		assert_eq!(BuysByAccountAndFrame::<Test>::iter_prefix(1).count(), 40);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 400_000);
		assert_eq!(System::account(1).providers, providers + 1);
		assert_eq!(PayoutQueue::<Test>::iter().count(), 1);

		let maturity = Auctions::<Test>::get(16).unwrap().matures_at_frame;
		CurrentFrame::set(maturity);
		FailPaymentFor::set(Some(1));
		StabilizerBuys::on_initialize(2);
		assert_eq!(BuysByAccountAndFrame::<Test>::iter_prefix(1).count(), 40);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 400_000);
		assert_eq!(PayoutStateByAccount::<Test>::get(1).payable_reward, 450);
		assert_eq!(Budget::outstanding_rewards(), 450);
		assert_eq!(PayoutStateByAccount::<Test>::get(1).next_payout_frame, Some(maturity + 1));

		FailPaymentFor::set(None);
		CurrentFrame::set(maturity + 1);
		StabilizerBuys::on_initialize(3);
		assert_eq!(BuysByAccountAndFrame::<Test>::iter_prefix(1).count(), 40 - 16);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			400_000 - 160_000
		);
		assert_eq!(Balances::free_balance(1), 1_000_000_000 - 400_000 + 160_000 + 450);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(
			PayoutStateByAccount::<Test>::get(1).next_payout_frame,
			Some(Auctions::<Test>::get(17).unwrap().matures_at_frame)
		);
		assert_eq!(System::account(1).providers, providers + 1);

		CurrentFrame::set(Auctions::<Test>::get(40).unwrap().matures_at_frame);
		StabilizerBuys::on_initialize(4);
		assert_eq!(BuysByAccountAndFrame::<Test>::iter_prefix(1).count(), 40 - 32);
		assert_eq!(
			BuysByAccountAndFrame::<Test>::iter_prefix(1).next().unwrap().0,
			33u64.to_ordered()
		);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			400_000 - 320_000
		);
		assert!(
			PayoutStateByAccount::<Test>::get(1).next_payout_frame.unwrap() <= CurrentFrame::get()
		);
		assert_eq!(PayoutQueue::<Test>::iter().count(), 1);
	});
}

#[test]
fn reward_backlog_after_principal_payment_automatically_queues_the_new_reward() {
	new_test_ext().execute_with(|| {
		let providers = System::account(1).providers;
		for account in [1, 2, 3] {
			assert_ok!(record_purchase(&account, &purchase()));
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
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		let issuance = Balances::total_issuance();
		StabilizerBuys::on_initialize(2);
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_eq!(System::account(1).providers, providers);
		assert!(BidsByAuction::<Test>::contains_key(
			1,
			(Percent::from_percent(60), AmountRankKey::new(10_000_000, 1))
		));
		assert_eq!(Budget::outstanding_rewards(), 5_000_000);

		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 1_004_200_000);
		assert_eq!(Balances::free_balance(2), 1_005_000_000);
		assert_eq!(Balances::free_balance(3), 1_005_000_000);
		assert_eq!(Balances::total_issuance(), issuance + 14_200_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
		assert_eq!(System::account(1).providers, providers);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::total_issuance(), issuance + 14_200_000);
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
			assert_ok!(record_purchase(&account, &purchase()));
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
		let maturity = 1 + PurchaseHoldFrames::get();
		CurrentFrame::set(maturity);
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
		assert_eq!(PayoutStateByAccount::<Test>::get(5).payable_reward, 5_000_000);
		assert_eq!(PayoutStateByAccount::<Test>::get(5).next_payout_frame, Some(maturity + 1));
		assert_eq!(Balances::free_balance(255), 105_000_000);
		assert!(!PayoutStateByAccount::<Test>::contains_key(255));
		assert_eq!(Balances::free_balance(256), 0);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &256),
			100_000_000
		);

		FailPaymentFor::set(None);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(5), 0);
		assert_eq!(Balances::free_balance(256), 103_500_000);
		assert!(!PayoutStateByAccount::<Test>::contains_key(256));
		assert_eq!(Budget::outstanding_rewards(), 5_000_000);
		CurrentFrame::set(maturity + 1);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::total_issuance(), issuance + 13_500_000);
		assert_eq!(Balances::free_balance(5), 105_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(System::account(5).providers, providers);
		assert!(PayoutQueue::<Test>::iter().next().is_none());
		StabilizerBuys::on_initialize(5);
		assert_eq!(Balances::total_issuance(), issuance + 13_500_000);
	});
}

#[test]
fn one_automatic_payment_collects_all_mature_groups_for_an_account() {
	new_test_ext().execute_with(|| {
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		assert_ok!(record_purchase(&1, &purchase()));
		CurrentTick::set(2_000);
		start_frame(3);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 2, Percent::from_percent(60)));
		CurrentTick::set(3_000);
		start_frame(4);
		CurrentFrame::set(2 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 1_009_900_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
		assert!(PayoutQueue::<Test>::iter().next().is_none());
	});
}

#[test]
fn unavailable_prices_leave_delivered_principal_unencumbered() {
	new_test_ext().execute_with(|| {
		let _ = PriceHistoryByTick::<Test>::clear(u32::MAX, None);
		assert_ok!(record_purchase(&1, &purchase()));
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		assert!(Auctions::<Test>::get(1).is_none());
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
		assert!(PayoutQueue::<Test>::iter().next().is_none());
		System::assert_last_event(
			Event::<Test>::PurchaseIneligible {
				account: 1,
				purchase_tick: 1,
				reason: IneligibleReason::PricesUnavailable,
			}
			.into(),
		);
		let mut invalid = Prices::get().unwrap();
		invalid.argon_usd_target_price = FixedU128::zero();
		PriceHistoryByTick::<Test>::insert(
			1u64.to_ordered(),
			PriceObservation { frame: 1, r_value: invalid.r_value, prices: Some(invalid) },
		);
		assert_ok!(record_purchase(&1, &purchase()));
		assert!(Auctions::<Test>::get(1).is_none());

		let mut index = price_index(1);
		index.usdc_usd_price = None;
		StabilizerBuys::on_price_index(&index);
		assert_ok!(record_purchase(&1, &purchase()));
		assert!(Auctions::<Test>::get(1).is_none());
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		System::assert_last_event(
			Event::<Test>::PurchaseIneligible {
				account: 1,
				purchase_tick: 1,
				reason: IneligibleReason::PricesUnavailable,
			}
			.into(),
		);
	});
}

#[test]
fn admission_keeps_pending_guards_past_u32_counts_and_schedules_without_an_older_auction() {
	new_test_ext().execute_with(|| {
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		let mut index = price_index(buy.tick);
		index.argon_usd_price = FixedU128::from_rational(1, 2);
		<StabilizerBuys as PriceIndexUpdated>::on_price_index(&index);
		assert_ok!(record_purchase(&1, &buy));

		// Exercise the former overflow boundary while keeping an older bucket to schedule.
		let count = u64::from(u32::MAX);
		PendingLiquidRepayCountByAccountAndLiquid::<Test>::insert(1, 42, count);
		PendingPriceWindowCountByTick::<Test>::insert(buy.tick.to_ordered(), count);
		BuysByAccountAndFrame::<Test>::mutate(1, 1u64.to_ordered(), |frame_buys| {
			frame_buys.as_mut().unwrap().pending_liquid_repays = count;
		});
		buy.gateway_activity_nonce = count + 1;
		assert_ok!(record_purchase(&1, &buy));
		assert_eq!(PendingLiquidRepayCountByAccountAndLiquid::<Test>::get(1, 42), count + 1);
		assert_eq!(PendingPriceWindowCountByTick::<Test>::get(buy.tick.to_ordered()), count + 1);
		assert_eq!(
			BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered())
				.unwrap()
				.pending_liquid_repays,
			count + 1
		);
		assert!(StabilizerBuys::has_pending_repayment(&1, 42));
		assert_eq!(StabilizerBuys::oldest_pending_purchase_tick(), Some(buy.tick));

		Auctions::<Test>::remove(1);
		CurrentFrame::set(2);
		buy.gateway_activity_nonce += 1;
		assert_ok!(record_purchase(&1, &buy));
		assert!(Auctions::<Test>::contains_key(2));
		assert_eq!(PayoutStateByAccount::<Test>::get(1).next_payout_frame, Some(31));
		assert_eq!(PayoutQueue::<Test>::iter_keys().count(), 1);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			3 * buy.principal
		);
	});
}

#[test]
fn failed_hold_rolls_back_admission_queue_and_provider_reference() {
	new_test_ext().execute_with(|| {
		let providers = System::account(4).providers;
		assert!(record_purchase(&4, &purchase()).is_err());

		let mut repay = purchase();
		repay.repay_liquid_id = Some(42);
		assert!(record_purchase(&4, &repay).is_err());
		assert!(!StabilizerBuys::has_pending_repayment(&4, 42));
		assert!(LiquidRepaysByAccountAndGatewayNonce::<Test>::iter_prefix(4).next().is_none());

		assert!(Auctions::<Test>::get(1).is_none());
		assert!(BidsByAuction::<Test>::iter().next().is_none());
		assert!(!PayoutStateByAccount::<Test>::contains_key(4));
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
		assert_ok!(record_purchase(&1, &purchase));
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
		assert_ok!(record_purchase(&4, &purchase));
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &4), 10_000);
		start_frame(2);
		start_frame(3);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(4), 10_720);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &4), 0);
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
		assert!(!PayoutStateByAccount::<Test>::contains_key(4));
	});
}

#[test]
fn four_positive_bids_allow_opt_out_but_never_restore_submission_count() {
	new_test_ext().execute_with(|| {
		assert_ok!(record_purchase(&1, &purchase()));
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
		let frame_buys = BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered()).unwrap();
		assert_eq!(frame_buys.positive_bid_count, 4);
		assert_eq!(frame_buys.bid_rate, Percent::zero());
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert_noop!(
			StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::one()),
			Error::<Test>::BidLimitReached
		);
		assert_eq!(Balances::free_balance(2), 1_000_000_000);
		assert!(!PayoutStateByAccount::<Test>::contains_key(2));
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
		let mut index = price_index(1);
		index.argon_usd_target_price = FixedU128::from_rational(101, 100);
		let prices = PurchasePriceReference::from_price_index(&index).unwrap();
		assert_eq!(pool.captured_discount(&p, prices), Ok(10_891_089));
		index.argon_usd_target_price =
			FixedU128::from_inner(index.argon_usd_target_price.into_inner() - 1);
		let prices = PurchasePriceReference::from_price_index(&index).unwrap();
		assert_eq!(pool.captured_discount(&p, prices), Ok(0));
	});
}

#[test]
fn discount_uses_usdc_value_and_rounds_down() {
	new_test_ext().execute_with(|| {
		let pool = Pool::get();
		let mut p = purchase();
		let mut index = price_index(1);
		index.usdc_usd_price = Some(FixedU128::from_rational(8, 10));
		let prices = PurchasePriceReference::from_price_index(&index).unwrap();
		assert_eq!(pool.captured_discount(&p, prices), Ok(28_000_000));
		p.principal = 1_000_001;
		p.usdc_spent = U256::one();
		index.usdc_usd_price = Some(FixedU128::from_inner(FixedU128::DIV + 1));
		let prices = PurchasePriceReference::from_price_index(&index).unwrap();
		assert_eq!(pool.captured_discount(&p, prices), Ok(999_999));
		p.usdc_spent = U256::MAX;
		index.argon_usd_target_price = FixedU128::from_inner(u128::MAX);
		index.argon_usd_price = index.argon_usd_target_price;
		index.usdc_usd_price = Some(FixedU128::from_inner(u128::MAX));
		let prices = PurchasePriceReference::from_price_index(&index).unwrap();
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
			assert_ok!(record_purchase(&1, &p));
			assert!(BuysByAccountAndFrame::<Test>::iter_prefix(1).next().is_none());
		}
	});
}

#[test]
fn zero_budget_still_drains_positive_bids_and_releases_every_principal() {
	new_test_ext().execute_with(|| {
		unhashed::put(b"test_liquidity_buy_budget", &0u128);
		for account in [1, 2, 3] {
			assert_ok!(record_purchase(&account, &purchase()));
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
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		let issuance = Balances::total_issuance();
		StabilizerBuys::on_initialize(2);
		assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 1);
		assert_eq!(Balances::free_balance(3), 900_000_000);
		StabilizerBuys::on_initialize(3);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert!(AuctionAllocationQueue::<Test>::iter().next().is_none());
		for account in [1, 2, 3] {
			assert_eq!(Balances::free_balance(account), 1_000_000_000);
		}
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(Budget::outstanding_rewards(), 0);
	});
}

#[test]
fn delayed_unreceivable_reward_is_retired_without_blocking_the_payout() {
	new_test_ext().execute_with(|| {
		let mut purchase = purchase();
		purchase.principal = 10_000;
		purchase.usdc_spent = U256::from(9_000u64);
		for account in [1, 2, 3] {
			assert_ok!(record_purchase(&account, &purchase));
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
		let maturity = 1 + PurchaseHoldFrames::get();
		CurrentFrame::set(maturity);
		let issuance = Balances::total_issuance();
		StabilizerBuys::on_initialize(2);
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
		assert_eq!(Balances::free_balance(1), 1_000_000_000);
		assert_ok!(Balances::transfer(&1, &4, Balances::free_balance(1), Preservation::Expendable));
		assert_eq!(System::account(1).providers, 0);

		StabilizerBuys::on_initialize(3);
		assert!(BuysByAccountAndFrame::<Test>::iter_prefix(1).next().is_none());
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
		assert_eq!(Balances::free_balance(1), 0);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert_eq!(Balances::total_issuance(), issuance + 1_000);
		assert!(BidsByAuction::<Test>::iter_prefix(1).next().is_none());
		assert!(PayoutQueue::<Test>::iter_keys().next().is_none());
		System::assert_has_event(
			Event::<Test>::PayoutCompleted { account: 1, principal: 0, reward: 0 }.into(),
		);

		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::free_balance(1), 0);
		assert_eq!(Balances::total_issuance(), issuance + 1_000);
	});
}

#[test]
fn repayment_remainder_can_use_its_reward_while_a_later_purchase_is_still_held() {
	new_test_ext().execute_with(|| {
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), 1, 200_000_000));
		RepaymentBurn::set(99_999_999);
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		assert_ok!(record_purchase(&1, &buy));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(3);
		CurrentFrame::set(1 + PurchaseHoldFrames::get());
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 7_000_000);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			100_000_000
		);
		assert!(!BuysByAccountAndFrame::<Test>::contains_key(1, 1u64.to_ordered()));
		assert!(BuysByAccountAndFrame::<Test>::contains_key(1, 2u64.to_ordered()));
		assert_eq!(PayoutStateByAccount::<Test>::get(1).payable_reward, 0);
	});
}

#[test]
fn repayment_holds_principal_through_maturity_and_retries_a_failed_burn_atomically() {
	new_test_ext().execute_with(|| {
		RepaymentBurn::set(40_000_000);
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		let issuance = Balances::total_issuance();
		assert_ok!(record_purchase(&1, &buy));
		let frame_buys = BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered()).unwrap();
		assert_eq!(
			(frame_buys.principal, frame_buys.pending_liquid_repays, frame_buys.captured_discount),
			(0, 1, 10_000_000)
		);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			100_000_000
		);
		assert_eq!(Balances::total_issuance(), issuance);
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		start_frame(3);
		CurrentTick::set(u64::MAX);
		CurrentFrame::set(30);
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(Balances::free_balance(1), 900_000_000);

		CurrentFrame::set(31);
		FailRepayment::set(true);
		StabilizerBuys::on_initialize(3);
		assert!(System::events().iter().any(|record| matches!(
			record.event,
			RuntimeEvent::StabilizerBuys(Event::PayoutFailed { account: 1, .. })
		)));
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			100_000_000
		);
		assert_eq!(
			BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered())
				.unwrap()
				.pending_liquid_repays,
			1
		);
		assert!(LiquidRepaysByAccountAndGatewayNonce::<Test>::contains_key(1, 1u64.to_ordered()));
		assert!(StabilizerBuys::has_pending_repayment(&1, 42));

		FailRepayment::set(false);
		CurrentFrame::set(32);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::free_balance(1), 965_500_000);
		assert_eq!(Balances::total_issuance(), issuance - 40_000_000 + 5_500_000);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		assert!(!LiquidRepaysByAccountAndGatewayNonce::<Test>::contains_key(1, 1u64.to_ordered()));
		assert!(!PayoutStateByAccount::<Test>::contains_key(1));
		assert!(!StabilizerBuys::has_pending_repayment(&1, 42));
		StabilizerBuys::on_initialize(5);
		assert_eq!(Balances::total_issuance(), issuance - 40_000_000 + 5_500_000);
	});
}

#[test]
fn multiple_repayments_share_the_auction_result_and_burn_unreceivable_bonus() {
	for (spend_free_balance, expected_bonus) in [(false, 3_000), (true, 0)] {
		new_test_ext().execute_with(|| {
			assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), 1, 100_100_000));
			unhashed::put(b"test_liquidity_buy_budget", &7_500u128);
			RepaymentBurn::set(40_000_000);
			for nonce in [1u64, 2] {
				let mut buy = purchase();
				buy.repay_liquid_id = Some(42);
				buy.gateway_activity_nonce = nonce;
				if nonce == 2 {
					buy.principal = 100_000;
					buy.usdc_spent = U256::from(90_000u64);
				}
				assert_ok!(record_purchase(&1, &buy));
			}
			assert_eq!(PayoutQueue::<Test>::iter_keys().count(), 1);
			assert!(StabilizerBuys::has_pending_repayment(&1, 42));
			assert!(!StabilizerBuys::has_pending_repayment(&2, 42));
			assert!(!StabilizerBuys::has_pending_repayment(&1, 43));
			start_frame(2);
			assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(60)));
			start_frame(3);

			CurrentFrame::set(31);
			StabilizerBuys::on_initialize(2);
			assert_eq!(Balances::free_balance(1), 61_204_500);
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
				100_000
			);
			assert_eq!(LiquidRepaysByAccountAndGatewayNonce::<Test>::iter_prefix(1).count(), 1);
			assert!(StabilizerBuys::has_pending_repayment(&1, 42));
			assert_eq!(
				BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered())
					.unwrap()
					.pending_liquid_repays,
				1
			);
			assert_eq!(Budget::outstanding_rewards(), 0);
			assert_eq!(Budget::available_budget(), 3_000);

			if spend_free_balance {
				assert_ok!(Balances::transfer(
					&1,
					&4,
					Balances::free_balance(1),
					Preservation::Expendable
				));
			}
			let free_before_second = Balances::free_balance(1);
			let issuance = Balances::total_issuance();
			System::reset_events();
			RepaymentBurn::set(100_000);
			StabilizerBuys::on_initialize(3);

			assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
			assert_eq!(Balances::free_balance(1), free_before_second + expected_bonus);
			assert_eq!(Balances::total_issuance(), issuance - 100_000 + expected_bonus);
			let awarded: Balance = System::events()
				.iter()
				.filter_map(|record| match record.event {
					RuntimeEvent::StabilizerBuys(Event::LiquidRepaymentRewardAwarded {
						amount,
						..
					}) => Some(amount),
					_ => None,
				})
				.sum();
			assert_eq!(awarded, expected_bonus);
			assert!(!PayoutStateByAccount::<Test>::contains_key(1));
			assert!(LiquidRepaysByAccountAndGatewayNonce::<Test>::iter_prefix(1).next().is_none());
			assert!(!StabilizerBuys::has_pending_repayment(&1, 42));
			assert!(PayoutQueue::<Test>::iter_keys().next().is_none());
			StabilizerBuys::on_initialize(4);
			assert_eq!(Balances::total_issuance(), issuance - 100_000 + expected_bonus);
		});
	}
}

#[test]
fn repayment_commitment_stays_held_even_when_the_purchase_is_reward_ineligible() {
	new_test_ext().execute_with(|| {
		ApprovedPool::<Test>::kill();
		RepaymentBurn::set(40_000_000);
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		assert_ok!(record_purchase(&1, &buy));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			100_000_000
		);
		assert_eq!(
			BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered())
				.unwrap()
				.captured_discount,
			0
		);
		assert!(LiquidRepaysByAccountAndGatewayNonce::<Test>::get(1, 1u64.to_ordered())
			.unwrap()
			.eligible_reward_percent
			.is_zero());
		// A qualifying buy in the same cohort must not make this repayment reward-eligible.
		ApprovedPool::<Test>::put(Pool::get());
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(60)));
		start_frame(3);
		CurrentFrame::set(31);
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::free_balance(1), 965_400_000);
	});
}

#[test]
fn repayment_package_allocation_scales_treasury_funding_proportionally_and_keeps_burn_independent()
{
	for (budget, bid_rate, ordinary_one, ordinary_two, bonus_one, bonus_two) in [
		(100_000_000, Some(60), 6_000_000, 4_800_000, 1_200_000, 960_000),
		(7_000_000, Some(60), 2_333_333, 1_866_666, 1_200_000, 960_000),
		(6_000_000, Some(60), 2_000_000, 1_600_000, 1_200_000, 960_000),
		(0, Some(60), 0, 0, 1_200_000, 960_000),
		(0, None, 0, 0, 2_000_000, 1_600_000),
		(0, Some(0), 0, 0, 0, 0),
	] {
		new_test_ext().execute_with(|| {
			unhashed::put(b"test_liquidity_buy_budget", &budget);
			RepaymentBurn::set(40_000_000);
			let mut buy = purchase();
			buy.repay_liquid_id = Some(42);
			for account in [1, 2] {
				assert_ok!(record_purchase(&account, &buy));
			}
			start_frame(2);
			if let Some(rate) = bid_rate {
				for account in [1, 2] {
					assert_ok!(StabilizerBuys::bid(
						RuntimeOrigin::signed(account),
						1,
						Percent::from_percent(rate)
					));
				}
			} else {
				assert_eq!(BidsByAuction::<Test>::iter_prefix(1).count(), 2);
			}
			start_frame(3);
			let issuance = Balances::total_issuance();
			CurrentFrame::set(31);
			StabilizerBuys::on_initialize(2);
			StabilizerBuys::on_initialize(3);

			assert_eq!(Balances::free_balance(1), 960_000_000 + ordinary_one + bonus_one);
			assert_eq!(Balances::free_balance(2), 960_000_000 + ordinary_two + bonus_two);
			assert_eq!(
				Balances::total_issuance(),
				issuance - 80_000_000 + ordinary_one + ordinary_two + bonus_one + bonus_two
			);
			assert_eq!(Budget::available_budget(), budget - ordinary_one - ordinary_two);
			assert_eq!(Budget::outstanding_rewards(), 0);
			assert!(LiquidRepaysByAccountAndGatewayNonce::<Test>::iter().next().is_none());
			assert!(PayoutQueue::<Test>::iter().next().is_none());
			StabilizerBuys::on_initialize(4);
			assert_eq!(Balances::free_balance(1), 960_000_000 + ordinary_one + bonus_one);
		});
	}
}

#[test]
fn lowering_a_default_request_wins_burn_allocation_without_treasury_funding() {
	new_test_ext().execute_with(|| {
		unhashed::put(b"test_liquidity_buy_budget", &0u128);
		RepaymentBurn::set(40_000_000);
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		let issuance = Balances::total_issuance();
		for account in 10..20 {
			assert_ok!(Balances::mint_into(&account, buy.principal));
			assert_ok!(record_purchase(&account, &buy));
		}
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(19), 1, Percent::from_percent(50)));
		start_frame(3);
		CurrentFrame::set(31);
		for block in 2..=12 {
			StabilizerBuys::on_initialize(block);
		}

		for account in 10..18 {
			assert_eq!(Balances::free_balance(account), 62_000_000);
		}
		assert_eq!(Balances::free_balance(18), 60_000_000);
		assert_eq!(Balances::free_balance(19), 61_000_000);
		assert_eq!(Balances::total_issuance(), issuance + 1_000_000_000 - 400_000_000 + 17_000_000);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(BidsByAuction::<Test>::iter().next().is_none());
		assert!(LiquidRepaysByAccountAndGatewayNonce::<Test>::iter().next().is_none());
		assert!(PayoutQueue::<Test>::iter().next().is_none());
	});
}

#[test]
fn final_liquid_payout_waits_for_allocation_and_retries_the_whole_payment_atomically() {
	new_test_ext().execute_with(|| {
		RepaymentBurn::set(40_000_000);
		unhashed::put(b"test_liquidity_buy_budget", &7_500u128);
		let mut buy = purchase();
		buy.repay_liquid_id = Some(42);
		assert_ok!(record_purchase(&1, &buy));
		assert_ok!(record_purchase(&1, &purchase()));
		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(60)));
		start_frame(3);
		AuctionAllocationQueue::<Test>::remove(AscendingRankKey::new(31, 1));
		AuctionAllocationQueue::<Test>::insert(AscendingRankKey::new(32, 1), ());
		let issuance = Balances::total_issuance();

		CurrentFrame::set(31);
		StabilizerBuys::on_initialize(2);
		let frame_buys = BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered()).unwrap();
		assert_eq!(frame_buys.pending_liquid_repays, 1);
		assert_eq!(frame_buys.liquid_repayment_reward_rate, None);
		assert_eq!(Balances::free_balance(1), 800_000_000);
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			200_000_000
		);
		assert!(StabilizerBuys::has_pending_repayment(&1, 42));

		FailPaymentFor::set(Some(1));
		CurrentFrame::set(32);
		StabilizerBuys::on_initialize(3);
		assert_eq!(Balances::free_balance(1), 800_000_000);
		assert_eq!(Balances::total_issuance(), issuance);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
			200_000_000
		);
		assert!(StabilizerBuys::has_pending_repayment(&1, 42));
		assert_eq!(PayoutStateByAccount::<Test>::get(1).payable_reward, 4_500);
		assert_eq!(Budget::outstanding_rewards(), 4_500);
		assert_eq!(Budget::available_budget(), 3_000);

		FailPaymentFor::set(None);
		CurrentFrame::set(33);
		StabilizerBuys::on_initialize(4);
		assert_eq!(Balances::free_balance(1), 961_204_500);
		assert_eq!(Balances::total_issuance(), issuance - 40_000_000 + 1_204_500);
		assert_eq!(Budget::outstanding_rewards(), 0);
		assert!(PayoutQueue::<Test>::iter().next().is_none());
		StabilizerBuys::on_initialize(5);
		assert_eq!(Balances::free_balance(1), 961_204_500);
	});
}

#[test]
fn full_liquid_repayment_retries_failed_burns_and_preserves_receivable_small_rewards() {
	for (principal, budget, bid, ordinary_reward, paid_ordinary_reward, paid_bonus) in [
		(100_000_000, 100_000_000, Some(60), 5_400_000, 5_400_000, 3_000_000),
		(100_000_000, 7_500, Some(60), 4_500, 4_500, 3_000_000),
		// Individually unreceivable dust need not be combined to establish an empty account.
		(150_000, 10_000, Some(60), 6_000, 0, 0),
		(100_000_000, 100_000_000, None, 9_000_000, 9_000_000, 5_000_000),
		(100_000_000, 4_500_000, None, 4_500_000, 4_500_000, 5_000_000),
		(100_000_000, 0, None, 0, 0, 5_000_000),
	] {
		new_test_ext().execute_with(|| {
			assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), 1, principal));
			RepaymentBurn::set(principal);
			unhashed::put(b"test_liquidity_buy_budget", &budget);
			let mut buy = purchase();
			buy.repay_liquid_id = Some(42);
			buy.principal = principal;
			buy.usdc_spent = U256::from(principal * 9 / 10);
			assert_ok!(record_purchase(&1, &buy));
			start_frame(2);
			if let Some(rate) = bid {
				assert_ok!(StabilizerBuys::bid(
					RuntimeOrigin::signed(1),
					1,
					Percent::from_percent(rate)
				));
			}
			start_frame(3);
			let issuance = Balances::total_issuance();

			FailRepayment::set(true);
			CurrentFrame::set(31);
			StabilizerBuys::on_initialize(2);
			assert_eq!(Balances::total_issuance(), issuance);
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1),
				principal
			);
			assert!(StabilizerBuys::has_pending_repayment(&1, 42));
			assert_eq!(Budget::outstanding_rewards(), ordinary_reward);

			FailRepayment::set(false);
			CurrentFrame::set(32);
			StabilizerBuys::on_initialize(3);
			let received = paid_ordinary_reward + paid_bonus;
			assert_eq!(Balances::free_balance(1), received);
			assert_eq!(Balances::total_issuance(), issuance - principal + received);
			assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
			assert_eq!(Budget::available_budget(), budget - ordinary_reward);
			assert_eq!(Budget::outstanding_rewards(), 0);
			assert!(!StabilizerBuys::has_pending_repayment(&1, 42));
			assert!(!PayoutStateByAccount::<Test>::contains_key(1));
			StabilizerBuys::on_initialize(5);
			assert_eq!(Balances::total_issuance(), issuance - principal + received);
		});
	}
}

#[test]
fn insufficient_bitcoin_window_keeps_the_buy_ordinary_and_reward_eligible() {
	new_test_ext().execute_with(|| {
		HasRepaymentWindow::set(false);
		let mut purchase = purchase();
		purchase.repay_liquid_id = Some(1);
		assert_ok!(record_purchase(&1, &purchase));
		System::assert_has_event(
			Event::<Test>::LiquidRepaymentInvalid {
				account: 1,
				gateway_activity_nonce: purchase.gateway_activity_nonce,
				liquid_id: 1,
			}
			.into(),
		);
		assert!(!StabilizerBuys::has_pending_repayment(&1, 1));
		assert!(LiquidRepaysByAccountAndGatewayNonce::<Test>::iter_prefix(1).next().is_none());
		assert!(PendingPriceWindowCountByTick::<Test>::iter_keys().next().is_none());
		let frame_buys = BuysByAccountAndFrame::<Test>::get(1, 1u64.to_ordered()).unwrap();
		assert_eq!(frame_buys.principal, purchase.principal);
		assert_eq!(frame_buys.pending_liquid_repays, 0);
		assert_eq!(frame_buys.captured_discount, 10_000_000);

		start_frame(2);
		assert_ok!(StabilizerBuys::bid(RuntimeOrigin::signed(1), 1, Percent::from_percent(50)));
		start_frame(3);
		CurrentFrame::set(31);
		StabilizerBuys::on_initialize(2);
		assert_eq!(Balances::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &1), 0);
		assert_eq!(Balances::free_balance(1), 1_004_500_000);
	});
}
