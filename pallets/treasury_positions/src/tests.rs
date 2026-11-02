use super::*;
use crate::mock::*;

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
