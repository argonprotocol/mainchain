use super::*;
use crate::{
	migrations::{AddCollectAmountsMigration, PositionV2},
	mock::*,
};
use alloc::collections::BTreeMap;
use frame_support::{
	assert_noop,
	storage::unhashed,
	traits::{Hooks, OnRuntimeUpgrade},
};
use polkadot_sdk::sp_io::TestExternalities;

#[test]
fn migrating_positions_preserves_principal_quantities_and_upstream() {
	polkadot_sdk::sp_io::TestExternalities::default().execute_with(|| {
		StorageVersion::new(2).put::<TreasuryPositions>();
		let quantities = PositionQuantities { bonds: 12, stakes: 2, fission_liquidity: 50 };
		let upstream = UpstreamPosition {
			vault_id: 3,
			bitcoin_securitization: 10,
			bitcoin_allocated_securitization: 45,
			bond_principal: 100,
		};
		let position_key = PositionsByAccount::<Test>::hashed_key_for(9);
		unhashed::put(
			&position_key,
			&PositionV2 { bond_principal: 300u128, quantities, upstream: Some(upstream) },
		);

		AddCollectAmountsMigration::<Test>::on_runtime_upgrade();
		assert_eq!(StorageVersion::get::<TreasuryPositions>(), StorageVersion::new(3));
		assert_eq!(
			PositionsByAccount::<Test>::get(9),
			Some(Position {
				bond_principal: 300,
				quantities,
				upstream: Some(upstream),
				collect_accrued: CollectAmounts::default(),
				collected: CollectAmounts::default(),
			})
		);

		PositionsByAccount::<Test>::mutate(9, |position| {
			position.as_mut().unwrap().collected.stabilizer_pool_earnings = 7;
		});
		AddCollectAmountsMigration::<Test>::on_runtime_upgrade();
		assert_eq!(
			PositionsByAccount::<Test>::get(9).unwrap().collected.stabilizer_pool_earnings,
			7
		);
	});
}

#[test]
fn collect_rolls_back_each_account_and_auto_collect_continues() {
	TestExternalities::default().execute_with(|| {
		System::set_block_number(1);
		ProvedThrough::set(BTreeMap::from([(9, [12, 9]), (10, [10, 5])]));
		CollectedThrough::set(&BTreeMap::from([(9, [11, 9]), (10, [9, 4])]));
		TreasuryPositions::ensure_account(&9);
		TreasuryPositions::ensure_account(&10);
		AfterFrameStart::set(false);
		TreasuryPositions::on_initialize(1);
		assert_eq!(Balances::balance(&9), 0);
		assert_eq!(Balances::balance(&10), 0);

		// Account 9's seven microgons cannot establish a balance. Account 10 receives one
		// combined payment of fourteen microgons from its two source positions.
		AfterFrameStart::set(true);
		TreasuryPositions::on_initialize(2);
		assert_eq!(CollectedThrough::get()[&9], [11, 9]);
		assert_eq!(Balances::balance(&9), 0);
		assert_eq!(CollectedThrough::get()[&10], [10, 5]);
		assert_eq!(Balances::balance(&10), 14);
		assert_eq!(
			PositionsByAccount::<Test>::get(10).unwrap().collected.stabilizer_pool_earnings,
			14
		);

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
	polkadot_sdk::sp_io::TestExternalities::default().execute_with(|| {
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
	polkadot_sdk::sp_io::TestExternalities::default().execute_with(|| {
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
	polkadot_sdk::sp_io::TestExternalities::default().execute_with(|| {
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
	TestExternalities::default().execute_with(|| {
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
			assert_noop!(
				TreasuryPositions::account_quantity_updated(
					&account,
					PositionQuantity::FissionLiquidity,
					0u128,
					1,
				),
				ArithmeticError::Overflow
			);
		}
	});
}

#[test]
fn vault_bitcoin_coverage_updates_only_the_network() {
	polkadot_sdk::sp_io::TestExternalities::default().execute_with(|| {
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
