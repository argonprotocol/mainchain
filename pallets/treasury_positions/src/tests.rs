use super::*;
use crate::mock::*;
use alloc::collections::BTreeMap;
use frame_support::{assert_noop, traits::Hooks};

#[test]
fn manual_collect_preserves_failed_claims_and_never_auto_pays() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		ProvedThrough::set(BTreeMap::from([(9, [12, 9]), (10, [10, 5])]));
		CollectedThrough::set(&BTreeMap::from([(9, [11, 9]), (10, [9, 4])]));
		TreasuryPositions::ensure_account(&9);
		TreasuryPositions::ensure_account(&10);
		AfterFrameStart::set(false);
		TreasuryPositions::on_initialize(1);
		assert_eq!(Balances::balance(&9), 0);
		assert_eq!(Balances::balance(&10), 0);

		AfterFrameStart::set(true);
		TreasuryPositions::on_initialize(2);
		assert_eq!(CollectedThrough::get()[&10], [9, 4]);
		assert_eq!(Balances::balance(&10), 0);
		assert_noop!(
			TreasuryPositions::collect(RuntimeOrigin::signed(9)),
			sp_runtime::TokenError::BelowMinimum
		);
		assert_eq!(CollectedThrough::get()[&9], [11, 9]);
		assert_eq!(Balances::balance(&9), 0);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(10)));
		assert_eq!(CollectedThrough::get()[&10], [10, 5]);
		assert_eq!(Balances::balance(&10), 14);

		ProvedThrough::mutate(|proofs| {
			proofs.insert(9, [12, 10]);
		});
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_eq!(CollectedThrough::get()[&9], [12, 10]);
		assert_eq!(Balances::balance(&9), 14);
		let position = PositionsByAccount::<Test>::get(9).unwrap();
		assert_eq!(position.collected.stabilizer_pool_earnings, 14);
		assert_eq!(position.collect_accrued, CollectAmounts::default());

		ProvedThrough::mutate(|proofs| {
			proofs.insert(9, [13, 10]);
		});
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_eq!(CollectedThrough::get()[&9], [13, 10]);
		assert_eq!(Balances::balance(&9), 21);
		assert_eq!(
			PositionsByAccount::<Test>::get(9).unwrap().collected.stabilizer_pool_earnings,
			21
		);
	});
}

#[test]
fn newly_created_upstream_binds_bitcoin_only_for_the_actual_position_owner() {
	new_test_ext().execute_with(|| {
		assert_ok!(TreasuryPositions::bitcoin_position_updated(
			&8,
			3,
			BitcoinLockPosition::default(),
			BitcoinLockPosition { activated_securitization: 100, allocated_securitization: 200 }
		));
		assert_eq!(TreasuryPositions::upstream_position(&8), None);
		assert_ok!(TreasuryPositions::bitcoin_position_updated(
			&9,
			2,
			BitcoinLockPosition::default(),
			BitcoinLockPosition { activated_securitization: 100, allocated_securitization: 200 }
		));
		assert_eq!(TreasuryPositions::upstream_position(&9), None);
		assert_ok!(TreasuryPositions::bitcoin_position_updated(
			&9,
			3,
			BitcoinLockPosition::default(),
			BitcoinLockPosition { activated_securitization: 100, allocated_securitization: 200 }
		));
		assert_eq!(
			TreasuryPositions::upstream_position(&9),
			Some(UpstreamPosition {
				vault_id: 3,
				bitcoin_securitization: 100,
				bitcoin_allocated_securitization: 200,
				bond_principal: 0
			})
		);
		assert_ok!(TreasuryPositions::bitcoin_position_updated(
			&9,
			3,
			BitcoinLockPosition { activated_securitization: 100, allocated_securitization: 200 },
			BitcoinLockPosition::default()
		));
		assert_eq!(TreasuryPositions::upstream_position(&9).unwrap().bitcoin_securitization, 0);
	});
}

#[test]
fn account_principal_and_upstream_subset_follow_different_vaults() {
	new_test_ext().execute_with(|| {
		assert_ok!(TreasuryPositions::bond_position_updated(&1, 2, 0, 100));
		TreasuryPositions::set_upstream_position(
			&1,
			Some(UpstreamPosition { vault_id: 3, ..Default::default() }),
		);
		assert_ok!(TreasuryPositions::bond_position_updated(&1, 3, 0, 200));
		assert_ok!(TreasuryPositions::bond_position_updated(&1, 2, 100, 50));
		assert_eq!(TreasuryPositions::bond_principal(&1), 250);
		assert_eq!(TreasuryPositions::upstream_position(&1).unwrap().bond_principal, 200);
		assert_ok!(TreasuryPositions::bond_position_updated(&1, 3, 200, 0));
		assert_eq!(TreasuryPositions::bond_principal(&1), 50);
		assert_eq!(TreasuryPositions::upstream_position(&1).unwrap().bond_principal, 0);
	});
}

#[test]
fn upstream_snapshot_preserves_principal_and_failed_updates_are_atomic() {
	new_test_ext().execute_with(|| {
		assert_ok!(TreasuryPositions::bond_position_updated(&1, 2, 0, 100));
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&1,
			PositionQuantity::FissionLiquidity,
			0u128,
			300,
		));
		let upstream = UpstreamPosition {
			vault_id: 2,
			bitcoin_securitization: 100,
			bitcoin_allocated_securitization: 200,
			bond_principal: 100,
		};
		TreasuryPositions::set_upstream_position(&1, Some(upstream));
		assert_eq!(TreasuryPositions::bond_principal(&1), 100);
		assert_noop!(
			TreasuryPositions::bitcoin_position_updated(
				&1,
				2,
				BitcoinLockPosition {
					activated_securitization: 100,
					allocated_securitization: 201
				},
				BitcoinLockPosition { activated_securitization: 150, ..Default::default() }
			),
			ArithmeticError::Underflow
		);
		assert_noop!(
			TreasuryPositions::bond_position_updated(&1, 2, 101, 0),
			ArithmeticError::Underflow
		);
		assert_eq!(TreasuryPositions::upstream_position(&1), Some(upstream));
		assert_ok!(TreasuryPositions::bitcoin_position_updated(
			&1,
			2,
			BitcoinLockPosition { activated_securitization: 100, allocated_securitization: 200 },
			BitcoinLockPosition { activated_securitization: 150, allocated_securitization: 200 }
		));
		assert_eq!(TreasuryPositions::upstream_position(&1).unwrap().bitcoin_securitization, 150);
		TreasuryPositions::set_upstream_position(&1, None);
		assert_eq!(TreasuryPositions::bond_principal(&1), 100);
		assert_eq!(TreasuryPositions::account_quantities(&1).fission_liquidity, 300);
	});
}

#[test]
fn fission_liquidity_overflow_leaves_account_and_network_positions_unchanged() {
	new_test_ext().execute_with(|| {
		let full_account = 1;
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&full_account,
			PositionQuantity::FissionLiquidity,
			0u128,
			u128::MAX,
		));

		// The first update overflows the account. The second fits the account but overflows
		// the network total. Both must leave storage untouched.
		for account in [full_account, 2] {
			// The source caller owns rollback for its canonical change and this callback.
			assert_noop!(
				frame_support::storage::with_storage_layer(|| {
					TreasuryPositions::account_quantity_updated(
						&account,
						PositionQuantity::FissionLiquidity,
						0u128,
						1,
					)
				}),
				ArithmeticError::Overflow
			);
		}
	});
}

#[test]
fn vault_bitcoin_coverage_updates_only_the_network() {
	new_test_ext().execute_with(|| {
		assert_ok!(TreasuryPositions::network_securitized_satoshis_updated(0, 100));
		assert_ok!(TreasuryPositions::network_securitized_satoshis_updated(0, 50));
		assert_eq!(TreasuryPositions::network_totals().vault_securitized_satoshis, 150);
		assert_eq!(PositionsByAccount::<Test>::iter().count(), 0);
		assert_ok!(TreasuryPositions::network_securitized_satoshis_updated(100, 75));
		assert_eq!(TreasuryPositions::network_totals().vault_securitized_satoshis, 125);
		assert_noop!(
			TreasuryPositions::network_securitized_satoshis_updated(150, 0),
			ArithmeticError::Underflow
		);
		assert_eq!(TreasuryPositions::network_totals().vault_securitized_satoshis, 125);
		assert_eq!(PositionsByAccount::<Test>::iter().count(), 0);
	});
}

#[test]
fn indexes_preserve_frame_start_quantities_and_collect_once() {
	new_test_ext().execute_with(|| {
		CurrentFrame::set(0);
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::Bonds,
			0u128,
			4u128
		));
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::Stakes,
			0u128,
			2u128
		));
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::FissionLiquidity,
			0u128,
			1_000_000u128
		));
		let first = TreasuryPositions::network_totals().account_quantities;
		CurrentFrame::set(1);
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::Bonds,
			4u128,
			8u128
		));
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::FissionLiquidity,
			1_000_000u128,
			2_000_000u128
		));
		index_frame(
			1,
			first,
			CollectAmounts {
				bond_earnings: 12,
				stake_earnings: 10,
				bitcoin_earnings: 3,
				..Default::default()
			},
		);
		assert_eq!(Balances::balance(&9), 0);
		assert_eq!(PositionsByAccount::<Test>::get(9).unwrap().collect_accrued.total(), 0);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_eq!(Balances::balance(&9), 25);
		assert!(!EarningPositionChanges::<Test>::contains_key(9, 2));
		assert!(!EarningsIndexesAtFrameStart::<Test>::contains_key(2));

		CurrentFrame::set(2);
		let second = TreasuryPositions::network_totals().account_quantities;
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::Bonds,
			8u128,
			0u128
		));
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::FissionLiquidity,
			2_000_000u128,
			0u128
		));
		index_frame(
			2,
			second,
			CollectAmounts {
				bond_earnings: 24,
				stake_earnings: 10,
				bitcoin_earnings: 6,
				..Default::default()
			},
		);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_eq!(Balances::balance(&9), 65);
		let position = PositionsByAccount::<Test>::get(9).unwrap();
		assert_eq!(position.collected.bond_earnings, 36);
		assert_eq!(position.collected.stake_earnings, 20);
		assert_eq!(position.collected.bitcoin_earnings, 9);
		assert_eq!(position.earning_positions.bitcoin_liquidity.quantity, 0);
		assert_eq!(position.earning_positions.bonds.quantity, 0);
		assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 10);
	});
}

#[test]
fn delayed_frames_keep_quantity_changes_and_bound_each_settlement() {
	new_test_ext().execute_with(|| {
		CurrentFrame::set(0);
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::Bonds,
			0u128,
			1u128
		));
		for frame in 1..=34 {
			CurrentFrame::set(frame);
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&9,
				PositionQuantity::Bonds,
				u128::from(frame),
				u128::from(frame + 2)
			));
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&9,
				PositionQuantity::Bonds,
				u128::from(frame + 2),
				u128::from(frame + 1)
			));
		}
		assert_eq!(EarningPositionChanges::<Test>::iter_prefix(9).count(), 34);
		for frame in 1..=33 {
			index_frame(
				frame,
				PositionQuantities { bonds: frame.into(), ..Default::default() },
				CollectAmounts { bond_earnings: u128::from(frame) * 10, ..Default::default() },
			);
		}
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::Bonds,
			35u128,
			36u128
		));
		assert_eq!(
			PositionsByAccount::<Test>::get(9).unwrap().earning_positions.next_change_frame,
			Some(3)
		);
		assert_eq!(EarningPositionChanges::<Test>::iter_prefix(9).count(), 33);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_eq!(Balances::balance(&9), 5_610); // One source refresh, then 32 Collect refreshes.
		assert_eq!(EarningPositionChanges::<Test>::iter_prefix(9).count(), 1);
		CurrentFrame::set(35);
		index_frame(
			34,
			PositionQuantities { bonds: 34, ..Default::default() },
			CollectAmounts { bond_earnings: 340, ..Default::default() },
		);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_eq!(Balances::balance(&9), 5_950); // 10 * sum(1..=34)
		assert_eq!(EarningPositionChanges::<Test>::iter_prefix(9).count(), 0);
		assert_eq!(EarningsIndexesAtFrameStart::<Test>::iter().count(), 0);
	});
}

#[test]
fn checkpoint_preserves_fractions_without_overflowing_scaled_products() {
	let mut checkpoint = argon_primitives::treasury::EarningsCheckpoint::default();
	assert_eq!(checkpoint.settle(1, FixedU128::from_rational(1, 2)), 0);
	assert_eq!(checkpoint.settle(1, FixedU128::one()), 1);
	assert_eq!(checkpoint.settle(1, FixedU128::one()), 0);
	assert_eq!(checkpoint.microgon_remainder, 0);

	checkpoint = Default::default();
	assert_eq!(checkpoint.settle(u128::MAX, FixedU128::from_rational(1, 2)), u128::MAX / 2);
	assert_eq!(checkpoint.settle(u128::MAX, FixedU128::one()), u128::MAX / 2 + 1);
	assert_eq!(checkpoint.microgon_remainder, 0);
}

#[test]
fn saturated_history_cannot_block_funded_earnings_or_wallet_collection() {
	new_test_ext().execute_with(|| {
		CurrentFrame::set(0);
		assert_ok!(TreasuryPositions::account_quantity_updated(
			&9,
			PositionQuantity::Bonds,
			0u128,
			2u128
		));
		let quantities = TreasuryPositions::network_totals().account_quantities;
		CollectTotals::<Test>::put(argon_primitives::treasury::CollectTotals {
			expired_vault_earnings: 0,
			funded: CollectAmounts { bond_earnings: u128::MAX, ..Default::default() },
			collected: CollectAmounts { bond_earnings: u128::MAX, ..Default::default() },
		});
		PositionsByAccount::<Test>::mutate(9, |position| {
			position.as_mut().unwrap().collected.bond_earnings = u128::MAX;
		});

		index_frame(1, quantities, CollectAmounts { bond_earnings: 10, ..Default::default() });
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_eq!(Balances::balance(&9), 10);
		assert_eq!(Balances::balance(&TreasuryPositions::account_id()), 10);
		assert_eq!(PositionsByAccount::<Test>::get(9).unwrap().collect_accrued.total(), 0);
		assert_eq!(CollectTotals::<Test>::get().funded.bond_earnings, u128::MAX);
		assert_eq!(CollectTotals::<Test>::get().collected.bond_earnings, u128::MAX);
		assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(9)));
		assert_eq!(Balances::balance(&9), 10);
	});
}
