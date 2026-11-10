use crate::{RValueHighWaterByFrame, RValueHighWaterMark};
use frame_support::traits::Hooks;
use pallet_prelude::*;
use polkadot_sdk::sp_runtime::ArithmeticError;
use std::collections::BTreeSet;

use crate::{
	mock::*, Error, Event, FissionByOwnerAndId, FissionIdsByLockId, FissionIdsByOwnerAndLiquid,
	LiquidTicks, LiquidTicksByOwnerAndId, NextFissionIdByOwner,
};
use argon_primitives::{
	amount_rank::AmountRankId,
	bitcoin::{LiquidRepayPrices, SATOSHIS_PER_BITCOIN},
	treasury::{NetworkPositionQuantities, PositionQuantities, TreasuryPositionProvider},
	BitcoinFissionsProvider, GatewayState, PriceIndex, PriceIndexUpdated,
};

#[test]
fn recovery_refunds_do_not_expand_the_bitcoin_repayment_and_only_actual_burn_is_retired() {
	for (high_r, expected_burn) in [(50, 3_376), (80, 2_637), (100, 2_500)] {
		new_test_ext().execute_with(|| {
			MockLocks::insert(1, lock(1, 100));
			let original_rate = 100 * u128::from(SATOSHIS_PER_BITCOIN);
			assert_ok!(BitcoinFissions::create(
				RuntimeOrigin::signed(1),
				0,
				77,
				1,
				100,
				original_rate,
				None
			));
			let prices = LiquidRepayPrices::new(
				FixedU128::saturating_from_integer(original_rate),
				FixedU128::from_rational(1, 2),
				FixedU128::from_rational(high_r, 100),
			)
			.unwrap();
			let burn = |amount| {
				Balances::burn_from(
					&1,
					amount,
					Preservation::Expendable,
					Precision::Exact,
					Fortitude::Force,
				)
				.map(|_| ())
			};

			if high_r == 100 {
				assert_noop!(
					with_storage_layer(|| BitcoinFissions::repay_liquid(
						&1,
						77,
						3_376,
						prices,
						|amount| {
							burn(amount)?;
							Err(DispatchError::Other("burn settlement failed"))
						},
					)),
					DispatchError::Other("burn settlement failed")
				);
			}

			assert_eq!(
				with_storage_layer(|| BitcoinFissions::repay_liquid(&1, 77, 3_376, prices, burn,))
					.unwrap(),
				expected_burn
			);
			assert_eq!(FissionByOwnerAndId::<Test>::get(1, 0).unwrap().satoshis, 75);
			assert_eq!(MockLocks::get(1).unwrap().fissioned_satoshis, 75);
			assert_eq!(Balances::free_balance(1), 20_000 - expected_burn);
			assert_eq!(MockFissionRedemptionBurns::get(), expected_burn);
		});
	}
}

#[test]
fn r_value_tracks_each_frames_high_and_rejects_invalid_observations() {
	new_test_ext().execute_with(|| {
		FissionFrame::set(1);
		FissionNewFrame::set(None);
		FissionTick::set(100);
		let mut index = PriceIndex {
			tick: 100,
			btc_usd_price: FixedU128::from_u32(60_000),
			argonot_usd_price: FixedU128::one(),
			argon_usd_price: FixedU128::from_rational(7, 10),
			argon_usd_target_price: FixedU128::one(),
			argon_time_weighted_average_liquidity: 1,
			usdc_usd_price: Some(FixedU128::one()),
			stabilizer_pool_sqrt_price_x96: None,
		};
		for price in [7, 9, 8] {
			index.argon_usd_price = FixedU128::from_rational(price, 10);
			BitcoinFissions::on_price_index(&index);
		}
		index.argon_usd_price = FixedU128::one();
		index.argon_time_weighted_average_liquidity = 0;
		BitcoinFissions::on_price_index(&index);
		index.argon_time_weighted_average_liquidity = 1;
		index.argon_usd_target_price = FixedU128::zero();
		BitcoinFissions::on_price_index(&index);
		index.argon_usd_target_price = FixedU128::one();
		index.tick = 99;
		BitcoinFissions::on_price_index(&index);
		assert_eq!(
			RValueHighWaterByFrame::<Test>::get(1u64.to_ordered()).unwrap().r_value,
			FixedU128::one()
		);
		FissionTick::set(200);
		FissionFrame::set(2);
		FissionNewFrame::set(Some(2));
		BitcoinFissions::on_initialize(2);
		assert_eq!(
			RValueHighWaterByFrame::<Test>::get(1u64.to_ordered()).unwrap().end_tick,
			Some(200)
		);
		assert!(RValueHighWaterByFrame::<Test>::get(2u64.to_ordered()).is_none());
		index.tick = 199;
		index.argon_usd_price = FixedU128::from_rational(11, 10);
		index.usdc_usd_price = None;
		BitcoinFissions::on_price_index(&index);
		let previous = RValueHighWaterByFrame::<Test>::get(1u64.to_ordered()).unwrap();
		assert_eq!(previous.r_value, FixedU128::one());
		assert_eq!(previous.first_tick, 99);
		assert_eq!(previous.end_tick, Some(200));
		assert_eq!(
			RValueHighWaterByFrame::<Test>::get(2u64.to_ordered()).unwrap(),
			RValueHighWaterMark {
				r_value: FixedU128::from_rational(11, 10),
				first_tick: 199,
				end_tick: None
			}
		);
		index.tick = 201;
		index.argon_usd_price = FixedU128::from_u32(2);
		BitcoinFissions::on_price_index(&index);
		assert_eq!(
			RValueHighWaterByFrame::<Test>::get(2u64.to_ordered()).unwrap().r_value,
			FixedU128::from_rational(11, 10)
		);
	});
}

#[test]
fn r_value_history_waits_for_source_sync_and_keeps_thirty_reward_frames() {
	new_test_ext().execute_with(|| {
		LastGatewayState::set(None);
		for frame in 1..=50 {
			FissionFrame::set(frame);
			FissionNewFrame::set(Some(frame));
			FissionTick::set(frame * 100);
			BitcoinFissions::on_initialize(frame);
			RValueHighWaterByFrame::<Test>::insert(
				frame.to_ordered(),
				RValueHighWaterMark {
					r_value: FixedU128::from_rational(u128::from(frame), 100),
					first_tick: frame * 100,
					end_tick: None,
				},
			);
		}
		assert_eq!(RValueHighWaterByFrame::<Test>::iter_keys().count(), 50);
		FissionNewFrame::set(None);
		LastGatewayState::set(Some(GatewayState {
			last_synced_source_block_time: Some(6_000_000),
			..Default::default()
		}));
		BitcoinFissions::on_initialize(51);
		assert_eq!(RValueHighWaterByFrame::<Test>::iter_keys().count(), 50);

		LastGatewayState::set(Some(GatewayState {
			last_synced_source_block_time: Some(300u64 * 1_000),
			..Default::default()
		}));
		PendingPriceHistoryTick::set(Some(100));
		BitcoinFissions::on_initialize(52);
		assert!(RValueHighWaterByFrame::<Test>::get(1u64.to_ordered()).is_some());
		assert_eq!(BitcoinFissions::next_r_value_high_water(2).unwrap().0, 2);
		PendingPriceHistoryTick::set(None);
		BitcoinFissions::on_initialize(52);
		assert!(RValueHighWaterByFrame::<Test>::get(1u64.to_ordered()).is_none());
		assert!(RValueHighWaterByFrame::<Test>::get(2u64.to_ordered()).is_some());
		assert!(RValueHighWaterByFrame::<Test>::get(3u64.to_ordered()).is_some());
		LastGatewayState::set(Some(GatewayState {
			last_synced_source_block_time: Some(5_000u64 * 1_000),
			..Default::default()
		}));
		BitcoinFissions::on_initialize(53);
		assert_eq!(RValueHighWaterByFrame::<Test>::iter_keys().count(), 33);
		BitcoinFissions::on_initialize(54);
		assert_eq!(RValueHighWaterByFrame::<Test>::iter_keys().count(), 30);
		assert!(RValueHighWaterByFrame::<Test>::get(20u64.to_ordered()).is_none());
		assert!(RValueHighWaterByFrame::<Test>::get(21u64.to_ordered()).is_some());
	});
}

#[test]
fn create_reserves_zero_for_no_liquid_without_advancing_the_fission_id() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 0, 1, 40, 90, None),
			Error::<Test>::InvalidLiquidId
		);
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 1, 1, 40, 90, None));
		assert_eq!(FissionByOwnerAndId::<Test>::get(1, 0).unwrap().liquid_id, 1);
		assert_eq!(NextFissionIdByOwner::<Test>::get(1), 1);
	});
}

#[test]
fn create_stores_one_lock_allocation_with_an_opaque_liquid_id() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, lock(1, 100));

		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 10, 77, 1, 40, 90, None));

		let fission = FissionByOwnerAndId::<Test>::get(1, 10).expect("fission");
		assert_eq!(fission.liquid_id, 77);
		assert_eq!(fission.lock_id, 1);
		assert_eq!(fission.satoshis, 40);
		assert_eq!(fission.microgons_at_target_per_btc, 90);
		assert_eq!(fission.last_ratchet_price_tick, 1);
		assert_eq!(fission.liquidity_promised, 3_600);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 3_600);
		assert_eq!(fission.created_at_argon_block, 7);
		assert_eq!(fission.ratchet_number, 0);
		assert_eq!(fission.last_updated_argon_block, 7);
		assert_eq!(MockLocks::get(1).expect("lock").fissioned_satoshis, 40);
		assert!(FissionIdsByLockId::<Test>::get(1).contains(&10));
		assert_eq!(NextFissionIdByOwner::<Test>::get(1), 11);
		assert_eq!(MockMintRequests::get(), vec![(1, 10, 1, 3_600)]);
		assert_eq!(MockAccountBitcoinChanges::get(), vec![(1, 3_600, true)]);
		System::assert_last_event(
			Event::FissionCreated {
				account_id: 1,
				fission_id: 10,
				liquid_id: 77,
				lock_id: 1,
				satoshis: 40,
				microgons_at_target_per_btc: 90,
				liquidity_promised: 3_600,
			}
			.into(),
		);
	});
}

#[test]
fn liquid_creation_tick_limits_additions_without_requiring_equal_fission_prices() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		MockLocks::insert(2, lock(1, 100));
		FissionTimestamp::set(100_000);
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 20, 90, None));

		System::set_block_number(2);
		FissionTimestamp::set(100_999);
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 77, 1, 20, 80, None));
		assert_eq!(FissionByOwnerAndId::<Test>::get(1, 0).unwrap().microgons_at_target_per_btc, 90);
		assert_eq!(FissionByOwnerAndId::<Test>::get(1, 1).unwrap().microgons_at_target_per_btc, 80);
		assert!(!BitcoinFissions::is_liquid_repayment_valid(&1, 77, 100, 30));
		assert!(!BitcoinFissions::is_liquid_repayment_valid(&1, 77, 101, 30));
		assert!(BitcoinFissions::is_liquid_repayment_valid(&1, 77, 102, 30));

		FissionTimestamp::set(101_000);
		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 2, 77, 1, 20, 90, None),
			Error::<Test>::LiquidCreationWindowClosed
		);
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 2, 78, 2, 20, 90, None));
	});
}

#[test]
fn create_allows_gaps_but_rejects_ids_below_the_owner_floor() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		MockLocks::insert(2, lock(1, 100));

		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 10, 77, 1, 20, 90, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 20, 77, 2, 20, 90, None));

		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 15, 78, 1, 20, 90, None),
			Error::<Test>::FissionIdBelowMinimum
		);
		assert_eq!(NextFissionIdByOwner::<Test>::get(1), 21);
	});
}

#[test]
fn create_rolls_back_the_lock_index_when_allocation_fails() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 10));

		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 20, 90, None),
			Error::<Test>::InsufficientFundedSatoshis
		);

		assert_eq!(MockLocks::get(1).expect("lock").fissioned_satoshis, 0);
		assert!(FissionIdsByLockId::<Test>::get(1).is_empty());
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert!(MockMintRequests::get().is_empty());
		assert_eq!(NextFissionIdByOwner::<Test>::get(1), 0);
	});
}

#[test]
fn create_limits_active_fissions_per_lock() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));

		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 1, 1, 10, 90, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 2, 1, 10, 90, None));
		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 2, 3, 1, 10, 90, None),
			Error::<Test>::TooManyFissionsForLock
		);
	});
}

#[test]
fn pending_repayment_blocks_only_direct_close_of_the_same_owners_liquid() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		PendingLiquidRepayment::set(Some((1, 77)));
		assert_noop!(
			BitcoinFissions::close(RuntimeOrigin::signed(1), 0),
			Error::<Test>::LiquidRepaymentPending
		);

		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 78, 1, 10, 90, None));
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 1));

		PendingLiquidRepayment::set(Some((2, 77)));
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
	});
}

#[test]
fn close_removes_the_fission_and_preserves_its_mint_entitlement() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));

		RedemptionMicrogonsPerSatoshi::set(50);
		System::set_block_number(12);
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));

		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert_eq!(MockLocks::get(1).expect("lock").fissioned_satoshis, 0);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 0);
		assert!(FissionIdsByLockId::<Test>::get(1).is_empty());
		assert_eq!(Balances::free_balance(1), 18_000);
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600)]);
		assert_eq!(MockFissionRedemptionBurns::get(), 2_000);
		assert_eq!(MockAccountBitcoinChanges::get(), vec![(1, 3_600, true), (1, 3_600, false)]);
		System::assert_has_event(
			Event::FissionClosed { account_id: 1, fission_id: 0, redemption_amount: 2_000 }.into(),
		);
	});
}

#[test]
fn partial_repayment_retires_liability_and_splits_guarantee_holds() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		FissionByOwnerAndId::<Test>::mutate(1, 0, |fission| {
			let guarantee = &mut fission.as_mut().unwrap().cosigner_guarantee;
			guarantee.fund_hold_extensions.try_insert(200, 800).unwrap();
			guarantee.fund_hold_extensions.try_insert(300, 3_200).unwrap();
		});
		RedemptionMicrogonsPerSatoshi::set(50);
		System::set_block_number(12);
		let issuance = Balances::total_issuance();
		assert_ok!(repay_fission(&1, 0, 10));

		let fission = FissionByOwnerAndId::<Test>::get(1, 0).unwrap();
		assert_eq!((fission.satoshis, fission.liquidity_promised), (30, 2_700));
		assert_eq!(fission.cosigner_guarantee.basis.satoshis, 30);
		assert_eq!(fission.cosigner_guarantee.guaranteed_microgons, 3_000);
		assert_eq!(
			fission.cosigner_guarantee.fund_hold_extensions.into_inner(),
			[(300, 3_000)].into()
		);
		assert_eq!(fission.last_updated_argon_block, 12);
		assert_eq!(fission.last_ratchet_price_tick, 1);
		assert_eq!(MockLocks::get(1).unwrap().fissioned_satoshis, 30);
		assert!(FissionIdsByLockId::<Test>::get(1).contains(&0));
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 2_700);
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600)]);
		assert_eq!(MockFissionRedemptionBurns::get(), 500);
		assert_eq!(issuance - Balances::total_issuance(), 500);
		assert_eq!(MockAccountBitcoinChanges::get(), vec![(1, 3_600, true), (1, 900, false)]);
		System::assert_has_event(
			Event::FissionRepaid {
				account_id: 1,
				fission_id: 0,
				satoshis: 10,
				liquidity_repaid: 900,
				redemption_amount: 500,
			}
			.into(),
		);
	});
}

#[test]
fn final_repayment_consumes_liability_and_guarantee_rounding_remainders() {
	new_test_ext().execute_with(|| {
		let mut source = lock(1, 3);
		source.microgons_at_target_per_btc = 1;
		MockLocks::insert(1, source);
		LiquidityMultiplierPercent::set(190);
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 3, 1, None));
		for remaining in [4, 2] {
			assert_ok!(repay_fission(&1, 0, 1));
			let fission = FissionByOwnerAndId::<Test>::get(1, 0).unwrap();
			assert_eq!(fission.liquidity_promised, remaining);
			assert_eq!(fission.cosigner_guarantee.guaranteed_microgons, remaining);
		}
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert!(!FissionIdsByLockId::<Test>::contains_key(1));
		assert_eq!(MockLocks::get(1).unwrap().fissioned_satoshis, 0);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 0);
		assert_eq!(MockFissionRedemptionBurns::get(), 300);
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 5)]);
	});
}

#[test]
fn repayment_rejects_zero_excess_and_other_owners_without_mutation() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		assert_noop!(repay_fission(&1, 0, 0), Error::<Test>::RepaymentHasNoSatoshis);
		assert_noop!(repay_fission(&1, 0, 41), Error::<Test>::RepaymentExceedsFission);
		assert_noop!(repay_fission(&2, 0, 10), Error::<Test>::FissionNotFound);
	});
}

#[test]
fn partial_repayment_rolls_back_a_burn_when_position_accounting_fails_then_retries() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		pallet_treasury_positions::NetworkTotals::<Test>::mutate(|totals| {
			totals.account_quantities.fission_liquidity = 0;
		});
		assert_noop!(repay_fission(&1, 0, 10), ArithmeticError::Underflow);
		pallet_treasury_positions::NetworkTotals::<Test>::mutate(|totals| {
			totals.account_quantities.fission_liquidity = 3_600;
		});
		assert_ok!(repay_fission(&1, 0, 10));
		assert_eq!(Balances::free_balance(1), 19_000);
		assert_eq!(MockFissionRedemptionBurns::get(), 1_000);
		assert_eq!(MockLocks::get(1).unwrap().fissioned_satoshis, 30);
	});
}

#[test]
fn close_rolls_back_when_the_owner_cannot_burn_the_redemption() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), 1, 100));

		assert_noop!(
			BitcoinFissions::close(RuntimeOrigin::signed(1), 0),
			Error::<Test>::InsufficientFunds
		);

		assert_noop!(repay_fission(&1, 0, 10), Error::<Test>::InsufficientFunds);

		assert!(FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 3_600);
		assert_eq!(MockLocks::get(1).expect("lock").fissioned_satoshis, 40);
		assert!(FissionIdsByLockId::<Test>::get(1).contains(&0));
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600)]);
	});
}

#[test]
fn close_uses_owner_scoped_lookup_and_removes_the_closed_fission() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));

		assert_noop!(
			BitcoinFissions::close(RuntimeOrigin::signed(2), 0),
			Error::<Test>::FissionNotFound
		);
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert_noop!(
			BitcoinFissions::close(RuntimeOrigin::signed(1), 0),
			Error::<Test>::FissionNotFound
		);
	});
}

#[test]
fn lock_cleanup_closes_only_active_fissions_and_preserves_pending_mints() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 78, 1, 10, 90, None));
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));

		System::set_block_number(12);
		assert_ok!(<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::close_for_lock(
			&1, 1, 0,
		));

		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600), (1, 1, 1, 900)]);
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 1));
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 0);
		assert!(FissionIdsByLockId::<Test>::get(1).is_empty());
		assert_eq!(
			MockAccountBitcoinChanges::get(),
			vec![(1, 3_600, true), (1, 900, true), (1, 3_600, false), (1, 900, false),]
		);
		System::assert_last_event(
			Event::FissionClosedByLock { account_id: 1, fission_id: 1, lock_id: 1 }.into(),
		);
	});
}

#[test]
fn terminal_redemption_records_the_full_vault_burn() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));

		assert_ok!(<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::close_for_lock(
			&1, 1, 4_000,
		));

		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600)]);
		assert_eq!(MockFissionRedemptionBurns::get(), 4_000);
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 0));
		assert!(FissionIdsByLockId::<Test>::get(1).is_empty());
	});
}

#[test]
fn terminal_settlement_uses_each_active_fission_basis() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		let bases =
			<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::get_lock_fission_guarantees(
				&1, 1,
			);
		assert_eq!(bases.len(), 1);
		assert_eq!(bases[0].0.satoshis, 40);
		assert_eq!(bases[0].0.microgons_at_target_per_btc, 90);
	});
}

#[test]
fn pending_repayment_blocks_both_ratchet_directions() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		PendingLiquidRepayment::set(Some((1, 77)));
		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 110, None),
			Error::<Test>::LiquidRepaymentPending
		);
		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 80, None),
			Error::<Test>::LiquidRepaymentPending
		);
		PendingLiquidRepayment::set(None);
		MockLastRatchetTick::set(2);
		assert_ok!(BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 80, None));
	});
}

#[test]
fn repayment_checks_ratchet_action_tick_instead_of_price_tick() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		FissionTick::set(1);
		FissionTimestamp::set(100_000);
		MockLastRatchetTick::set(2);
		assert_ok!(BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 80, None));

		let fission = FissionByOwnerAndId::<Test>::get(1, 0).unwrap();
		assert_eq!(fission.last_ratchet_price_tick, 2);
		assert_eq!(
			LiquidTicksByOwnerAndId::<Test>::get(1, 77),
			Some(LiquidTicks { created_at_tick: Some(0), last_ratchet_tick: Some(100) })
		);
		assert!(!BitcoinFissions::is_liquid_repayment_valid(&1, 77, 99, 30));
		assert!(!BitcoinFissions::is_liquid_repayment_valid(&1, 77, 100, 30));
		assert!(!BitcoinFissions::is_liquid_repayment_valid(&1, 77, 101, 30));
		assert!(BitcoinFissions::is_liquid_repayment_valid(&1, 77, 102, 30));

		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 77, 1, 10, 90, None),
			Error::<Test>::LiquidCreationWindowClosed
		);
		assert!(!BitcoinFissions::is_liquid_repayment_valid(&1, 77, 100, 30));
	});
}

#[test]
fn up_ratchet_updates_one_fission_when_threshold_and_lock_coverage_allow_it() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, MockLock { microgons_at_target_per_btc: 120, ..lock(1, 100) });
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));

		System::set_block_number(12);
		LiquidityMultiplierPercent::set(110);
		MockLastRatchetTick::set(2);
		assert_ok!(BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 110, None));

		let fission = FissionByOwnerAndId::<Test>::get(1, 0).expect("fission");
		assert_eq!(fission.microgons_at_target_per_btc, 110);
		assert_eq!(fission.last_ratchet_price_tick, 2);
		assert_eq!(fission.liquidity_promised, 4_840);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 4_840);
		assert_eq!(fission.ratchet_number, 1);
		assert_eq!(fission.last_updated_argon_block, 12);
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600), (1, 0, 1, 1_240)]);
		assert_eq!(MockAccountBitcoinChanges::get(), vec![(1, 3_600, true), (1, 1_240, true)]);
		System::assert_last_event(
			Event::FissionRatcheted {
				account_id: 1,
				fission_id: 0,
				ratchet_number: 1,
				microgons_at_target_per_btc: 110,
				liquidity_promised: 4_840,
				amount_minted: 1_240,
				amount_burned: 0,
			}
			.into(),
		);
	});
}

#[test]
fn down_ratchet_burns_and_requeues_the_replacement_liability() {
	new_test_ext().execute_with(|| {
		System::set_block_number(7);
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));

		System::set_block_number(12);
		MockLastRatchetTick::set(2);
		assert_ok!(BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 80, None));

		let fission = FissionByOwnerAndId::<Test>::get(1, 0).expect("fission");
		assert_eq!(fission.microgons_at_target_per_btc, 80);
		assert_eq!(fission.last_ratchet_price_tick, 2);
		assert_eq!(fission.liquidity_promised, 3_200);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 3_200);
		assert_eq!(fission.ratchet_number, 1);
		assert_eq!(fission.last_updated_argon_block, 12);
		assert_eq!(MockMintRequests::get(), vec![(1, 0, 1, 3_600), (1, 0, 1, 3_200)]);
		assert_eq!(MockFissionRedemptionBurns::get(), 3_200);
		assert_eq!(MockAccountBitcoinChanges::get(), vec![(1, 3_600, true), (1, 400, false)]);
		assert_eq!(Balances::free_balance(1), 16_800);
		System::assert_last_event(
			Event::FissionRatcheted {
				account_id: 1,
				fission_id: 0,
				ratchet_number: 1,
				microgons_at_target_per_btc: 80,
				liquidity_promised: 3_200,
				amount_minted: 3_200,
				amount_burned: 3_200,
			}
			.into(),
		);
	});
}

#[test]
fn registration_provider_reads_maintained_fission_liquidity() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 78, 1, 10, 90, None));
		MockLocks::insert(2, lock(2, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(2), 0, 78, 2, 10, 90, None));
		assert_eq!(TreasuryPositions::account_quantities(&2).fission_liquidity, 900);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.fission_liquidity, 5_400);

		assert_eq!(
			<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::get_account_fission_liquidity(
				&1
			),
			4_500
		);

		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
		assert_eq!(
			<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::get_account_fission_liquidity(
				&1
			),
			900
		);
		assert_eq!(TreasuryPositions::account_quantities(&2).fission_liquidity, 900);
		assert_eq!(TreasuryPositions::network_totals().account_quantities.fission_liquidity, 1_800);
	});
}

#[test]
fn create_rolls_back_lock_allocation_and_mint_request_when_liquidity_overflows() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		pallet_treasury_positions::PositionsByAccount::<Test>::insert(
			1,
			pallet_treasury_positions::Position {
				quantities: PositionQuantities {
					fission_liquidity: u128::MAX,
					..Default::default()
				},
				..Default::default()
			},
		);
		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None),
			sp_runtime::ArithmeticError::Overflow
		);
		assert_eq!(MockLocks::get(1).unwrap().fissioned_satoshis, 0);
		assert!(MockMintRequests::get().is_empty());
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, u128::MAX);
	});
}

#[test]
fn terminal_cleanup_rolls_back_all_fissions_in_the_callers_storage_layer() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 78, 1, 10, 90, None));
		// The first removal can succeed, but the second detects the inconsistent account total.
		pallet_treasury_positions::PositionsByAccount::<Test>::mutate(1, |position| {
			position.as_mut().unwrap().quantities.fission_liquidity = 3_600;
		});
		assert_noop!(
			with_storage_layer(|| {
				<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::close_for_lock(&1, 1, 0)
			}),
			sp_runtime::ArithmeticError::Underflow
		);
		assert_eq!(FissionIdsByLockId::<Test>::get(1).len(), 2);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 3_600);
	});
}

#[test]
fn ratchet_rejects_a_removed_fission_or_an_ineligible_change_without_writes() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		let original = FissionByOwnerAndId::<Test>::get(1, 0).expect("fission");

		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 95, None),
			Error::<Test>::NoRatchetingAvailable
		);
		MockLastRatchetTick::set(2);
		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 110, None),
			Error::<Test>::NoRatchetingAvailable
		);
		assert_eq!(FissionByOwnerAndId::<Test>::get(1, 0), Some(original));

		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 0));
		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 80, None),
			Error::<Test>::FissionNotFound
		);
	});
}

#[test]
fn down_ratchet_rolls_back_when_the_owner_cannot_burn_the_replacement_liability() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None));
		let original = FissionByOwnerAndId::<Test>::get(1, 0).expect("fission");
		let original_mint_requests = MockMintRequests::get();
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), 1, 100));
		MockLastRatchetTick::set(2);

		assert_noop!(
			BitcoinFissions::ratchet(RuntimeOrigin::signed(1), 0, 80, None),
			Error::<Test>::InsufficientFunds
		);

		assert_eq!(FissionByOwnerAndId::<Test>::get(1, 0), Some(original));
		assert_eq!(MockMintRequests::get(), original_mint_requests);
		assert_eq!(Balances::free_balance(1), 100);
	});
}

#[test]
fn creation_rolls_back_when_the_network_fission_quantity_overflows() {
	new_test_ext().execute_with(|| {
		MockLocks::insert(1, lock(1, 100));
		pallet_treasury_positions::NetworkTotals::<Test>::put(NetworkPositionQuantities {
			account_quantities: PositionQuantities {
				fission_liquidity: u128::MAX,
				..Default::default()
			},
			..Default::default()
		});
		assert_noop!(
			BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 40, 90, None),
			sp_runtime::ArithmeticError::Overflow
		);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 0);
		assert_eq!(MockLocks::get(1).unwrap().fissioned_satoshis, 0);
		assert!(MockMintRequests::get().is_empty());
	});
}

#[test]
fn liquid_repayment_prioritizes_expiring_locks_and_preserves_unspent_budget() {
	new_test_ext().execute_with(|| {
		let rate = 100 * u128::from(SATOSHIS_PER_BITCOIN);
		let prices = LiquidRepayPrices::new(
			FixedU128::saturating_from_integer(rate),
			FixedU128::one(),
			FixedU128::one(),
		)
		.unwrap();
		MockLocks::insert(1, MockLock { microgons_at_target_per_btc: rate, ..lock(1, 100) });
		MockLocks::insert(2, MockLock { microgons_at_target_per_btc: rate, ..lock(1, 100) });
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 2, 10, rate, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 77, 1, 10, rate, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 2, 88, 1, 5, rate, None));
		let mut burns = Vec::new();
		assert_eq!(
			with_storage_layer(|| {
				<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::repay_liquid(
					&1,
					77,
					1_250,
					prices,
					|amount| {
						burns.push(amount);
						Balances::burn_from(
							&1,
							amount,
							Preservation::Expendable,
							Precision::Exact,
							Fortitude::Force,
						)
						.map(|_| ())
					},
				)
			})
			.unwrap(),
			1_200
		);
		assert_eq!(burns, vec![1_200]);
		assert!(!FissionByOwnerAndId::<Test>::contains_key(1, 1));
		let remainder = FissionByOwnerAndId::<Test>::get(1, 0).unwrap();
		assert_eq!((remainder.satoshis, remainder.liquidity_promised), (8, 8 * rate));
		assert_eq!(FissionByOwnerAndId::<Test>::get(1, 2).unwrap().satoshis, 5);
		assert_eq!(
			FissionIdsByOwnerAndLiquid::<Test>::iter_key_prefix((1, 77)).collect::<BTreeSet<_>>(),
			[0].into()
		);
		assert_eq!(Balances::free_balance(1), 18_800);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 13 * rate);
	});
}

#[test]
fn liquid_repayment_rolls_back_failures_and_retries_without_leftover_state() {
	new_test_ext().execute_with(|| {
		let rate = 100 * u128::from(SATOSHIS_PER_BITCOIN);
		let prices = LiquidRepayPrices::new(
			FixedU128::saturating_from_integer(rate),
			FixedU128::one(),
			FixedU128::one(),
		)
		.unwrap();
		MockLocks::insert(1, MockLock { microgons_at_target_per_btc: rate, ..lock(1, 100) });
		MockLocks::insert(2, MockLock { microgons_at_target_per_btc: rate, ..lock(1, 100) });
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 10, rate, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 77, 2, 10, rate, None));

		let burn = |amount| {
			Balances::burn_from(
				&1,
				amount,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)
			.map(|_| ())
		};
		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), 1, 1_000));
		assert_noop!(
			with_storage_layer(|| {
				<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::repay_liquid(
					&1, 77, 2_000, prices, burn,
				)
			}),
			TokenError::FundsUnavailable
		);

		assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), 1, 3_000));
		assert_noop!(
			with_storage_layer(|| -> DispatchResult {
				assert_eq!(
					<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::repay_liquid(
						&1, 77, 2_000, prices, burn,
					)?,
					2_000
				);
				Err(DispatchError::Other("later step failed"))
			}),
			DispatchError::Other("later step failed")
		);

		assert_eq!(
			with_storage_layer(|| {
				<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::repay_liquid(
					&1, 77, 2_000, prices, burn,
				)
			})
			.unwrap(),
			2_000
		);
		assert_eq!(Balances::free_balance(1), 1_000);
		assert_eq!(MockFissionRedemptionBurns::get(), 2_000);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 0);
		assert!(FissionByOwnerAndId::<Test>::iter_prefix(1).next().is_none());
		assert!(FissionIdsByOwnerAndLiquid::<Test>::iter_key_prefix((1, 77)).next().is_none());
		for lock_id in [1, 2] {
			assert_eq!(MockLocks::get(lock_id).unwrap().fissioned_satoshis, 0);
			assert!(!FissionIdsByLockId::<Test>::contains_key(lock_id));
		}
	});
}

#[test]
fn liquid_index_allows_more_than_the_expected_count_and_cleans_up() {
	new_test_ext().execute_with(|| {
		let rate = 100 * u128::from(SATOSHIS_PER_BITCOIN);
		let prices = LiquidRepayPrices::new(
			FixedU128::saturating_from_integer(rate),
			FixedU128::one(),
			FixedU128::one(),
		)
		.unwrap();
		MockLocks::insert(1, MockLock { microgons_at_target_per_btc: rate, ..lock(1, 100) });
		MockLocks::insert(2, MockLock { microgons_at_target_per_btc: rate, ..lock(1, 100) });
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 10, rate, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 77, 2, 10, rate, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 2, 77, 2, 10, rate, None));
		assert_eq!(NextFissionIdByOwner::<Test>::get(1), 3);
		assert_ok!(<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::close_for_lock(
			&1, 1, 0
		));
		assert_eq!(
			FissionIdsByOwnerAndLiquid::<Test>::iter_key_prefix((1, 77)).collect::<BTreeSet<_>>(),
			[1, 2].into()
		);
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 1));
		assert_ok!(BitcoinFissions::close(RuntimeOrigin::signed(1), 2));
		assert!(FissionIdsByOwnerAndLiquid::<Test>::iter_key_prefix((1, 77)).next().is_none());
		assert_eq!(
			with_storage_layer(|| {
				<BitcoinFissions as BitcoinFissionsProvider<u64, u128>>::repay_liquid(
					&2,
					77,
					100,
					prices,
					|_| panic!("absent Liquid must not burn"),
				)
			})
			.unwrap(),
			0
		);
	});
}

#[test]
fn repayment_scans_all_fissions_above_the_expected_count() {
	new_test_ext().execute_with(|| {
		let rate = 100 * u128::from(SATOSHIS_PER_BITCOIN);
		let prices = LiquidRepayPrices::new(
			FixedU128::saturating_from_integer(rate),
			FixedU128::one(),
			FixedU128::one(),
		)
		.unwrap();
		MockLocks::insert(1, MockLock { microgons_at_target_per_btc: rate, ..lock(1, 100) });
		MockLocks::insert(2, MockLock { microgons_at_target_per_btc: rate, ..lock(1, 100) });
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 10, rate, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 77, 1, 10, rate, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 2, 77, 2, 10, rate, None));

		assert_ok!(with_storage_layer(|| BitcoinFissions::repay_liquid(
			&1,
			77,
			10_000,
			prices,
			|_| Ok(())
		)));
		assert_eq!(FissionByOwnerAndId::<Test>::iter_prefix(1).count(), 0);
		assert_eq!(FissionIdsByOwnerAndLiquid::<Test>::iter_key_prefix((1, 77)).count(), 0);
	});
}

#[test]
fn repayment_window_rejects_the_entire_liquid_when_one_source_lock_is_too_close_to_claim() {
	new_test_ext().execute_with(|| {
		assert!(!BitcoinFissions::is_liquid_repayment_valid(&1, 77, 0, 30));
		MockLocks::insert(1, lock(1, 100));
		MockLocks::insert(2, lock(1, 100));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 0, 77, 1, 20, 90, None));
		assert_ok!(BitcoinFissions::create(RuntimeOrigin::signed(1), 1, 77, 2, 20, 90, None));
		assert!(BitcoinFissions::is_liquid_repayment_valid(&1, 77, 2, 30));
		assert!(!BitcoinFissions::is_liquid_repayment_valid(&2, 77, 2, 30));

		RepaymentWindowUnavailableForLock::set(Some(2));
		assert!(!BitcoinFissions::is_liquid_repayment_valid(&1, 77, 2, 30));
		assert_eq!(FissionIdsByOwnerAndLiquid::<Test>::iter_key_prefix((1, 77)).count(), 2);
	});
}
