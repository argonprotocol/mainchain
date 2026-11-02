use crate::{pallet::STORAGE_VERSION, *};

pub(crate) fn check<T: Config>() -> Result<(), sp_runtime::TryRuntimeError> {
	ensure!(
		StorageVersion::get::<Pallet<T>>() == STORAGE_VERSION,
		"Treasury Positions storage version must match the runtime before checking state"
	);

	let mut bonds = 0u128;
	let mut stakes = 0u128;
	let mut fission_liquidity = T::Balance::zero();
	for position in PositionsByAccount::<T>::iter_values() {
		bonds = bonds
			.checked_add(position.quantities.bonds)
			.ok_or("Treasury Positions account bond quantities overflow")?;
		stakes = stakes
			.checked_add(position.quantities.stakes)
			.ok_or("Treasury Positions account stake quantities overflow")?;
		fission_liquidity = fission_liquidity
			.checked_add(&position.quantities.fission_liquidity)
			.ok_or("Treasury Positions account Fission liquidity overflows")?;

		if let Some(upstream) = position.upstream {
			ensure!(
				upstream.bond_principal <= position.bond_principal,
				"Treasury Positions upstream principal exceeds account principal"
			);
			ensure!(
				upstream.bitcoin_securitization <= upstream.bitcoin_allocated_securitization,
				"Treasury Positions activated upstream securitization exceeds its allocation"
			);
		}
	}

	// Vault coverage is maintained separately by canonical Vault updates, not account
	// positions.
	let totals = NetworkTotals::<T>::get().account_quantities;
	ensure!(
		bonds == totals.bonds,
		"Treasury Positions network bonds differ from account quantities"
	);
	ensure!(
		stakes == totals.stakes,
		"Treasury Positions network stakes differ from account quantities"
	);
	ensure!(
		fission_liquidity == totals.fission_liquidity,
		"Treasury Positions network Fission liquidity differs from account quantities"
	);
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use frame_support::traits::Hooks;

	#[test]
	fn try_state_reconciles_quantities_after_updates_and_releases() {
		polkadot_sdk::sp_io::TestExternalities::default().execute_with(|| {
			StorageVersion::new(2).put::<TreasuryPositions>();
			assert_ok!(TreasuryPositions::try_state(1));
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&1,
				PositionQuantity::Bonds,
				0u128,
				100
			));
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&2,
				PositionQuantity::Bonds,
				0u128,
				50
			));
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&1,
				PositionQuantity::Stakes,
				0u128,
				75
			));
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&2,
				PositionQuantity::FissionLiquidity,
				0u128,
				300
			));
			assert_ok!(TreasuryPositions::network_securitized_satoshis_updated(0, 1_000));
			assert_ok!(TreasuryPositions::bond_position_updated(&9, 3, 0, 200));
			assert_ok!(TreasuryPositions::bitcoin_position_updated(
				&9,
				3,
				BitcoinLockPosition::default(),
				BitcoinLockPosition {
					activated_securitization: 100,
					allocated_securitization: 200
				},
			));
			assert_ok!(TreasuryPositions::try_state(1));

			assert_ok!(TreasuryPositions::account_quantity_updated(
				&1,
				PositionQuantity::Bonds,
				100u128,
				25
			));
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&1,
				PositionQuantity::Stakes,
				75u128,
				0
			));
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&2,
				PositionQuantity::FissionLiquidity,
				300u128,
				0
			));
			assert_ok!(TreasuryPositions::bond_position_updated(&9, 3, 200, 0));
			assert_ok!(TreasuryPositions::try_state(2));
		});
	}

	#[test]
	fn try_state_rejects_drift_in_each_network_quantity() {
		polkadot_sdk::sp_io::TestExternalities::default().execute_with(|| {
			StorageVersion::new(2).put::<TreasuryPositions>();
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&1,
				PositionQuantity::Bonds,
				0u128,
				100
			));
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&1,
				PositionQuantity::Stakes,
				0u128,
				50
			));
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&1,
				PositionQuantity::FissionLiquidity,
				0u128,
				300
			));
			let totals = NetworkTotals::<Test>::get();

			NetworkTotals::<Test>::mutate(|totals| totals.account_quantities.bonds += 1);
			assert_eq!(
				TreasuryPositions::try_state(1),
				Err("Treasury Positions network bonds differ from account quantities".into())
			);
			NetworkTotals::<Test>::put(totals);
			NetworkTotals::<Test>::mutate(|totals| totals.account_quantities.stakes += 1);
			assert_eq!(
				TreasuryPositions::try_state(1),
				Err("Treasury Positions network stakes differ from account quantities".into())
			);
			NetworkTotals::<Test>::put(totals);
			NetworkTotals::<Test>::mutate(|totals| {
				totals.account_quantities.fission_liquidity += 1
			});
			assert_eq!(
				TreasuryPositions::try_state(1),
				Err("Treasury Positions network Fission liquidity differs from account quantities"
					.into())
			);
		});
	}

	#[test]
	fn try_state_rejects_impossible_upstream_subsets() {
		polkadot_sdk::sp_io::TestExternalities::default().execute_with(|| {
			StorageVersion::new(2).put::<TreasuryPositions>();
			assert_ok!(TreasuryPositions::bond_position_updated(&9, 3, 0, 100));
			PositionsByAccount::<Test>::mutate(9, |position| {
				position.as_mut().unwrap().upstream.as_mut().unwrap().bond_principal = 101;
			});
			assert_eq!(
				TreasuryPositions::try_state(1),
				Err("Treasury Positions upstream principal exceeds account principal".into())
			);

			assert_ok!(TreasuryPositions::operational_account_registered(&9));
			PositionsByAccount::<Test>::mutate(9, |position| {
				position.as_mut().unwrap().upstream.as_mut().unwrap().bitcoin_securitization = 1;
			});
			assert_eq!(
				TreasuryPositions::try_state(1),
				Err("Treasury Positions activated upstream securitization exceeds its allocation"
					.into())
			);
		});
	}

	#[test]
	fn try_state_rejects_old_storage_before_decoding_current_records() {
		polkadot_sdk::sp_io::TestExternalities::default().execute_with(|| {
			StorageVersion::new(1).put::<TreasuryPositions>();
			frame_support::storage::unhashed::put_raw(
				&NetworkTotals::<Test>::hashed_key(),
				&[0xff],
			);
			assert_eq!(
				TreasuryPositions::try_state(1),
				Err("Treasury Positions storage version must match the runtime before checking state"
					.into())
			);
		});
	}
}
