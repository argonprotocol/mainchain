use crate::{pallet::STORAGE_VERSION, *};

pub(crate) fn check<T: Config>() -> Result<(), sp_runtime::TryRuntimeError> {
	ensure!(
		StorageVersion::get::<Pallet<T>>() == STORAGE_VERSION,
		"Treasury Positions storage version must match the runtime before checking state"
	);

	let checked_total = |amounts: CollectAmounts<T::Balance>| {
		amounts
			.bond_earnings
			.checked_add(&amounts.stake_earnings)
			.and_then(|total| total.checked_add(&amounts.bitcoin_earnings))
			.and_then(|total| total.checked_add(&amounts.stabilizer_pool_earnings))
			.and_then(|total| total.checked_add(&amounts.stabilizer_buy_earnings))
			.and_then(|total| total.checked_add(&amounts.liquid_repayment_earnings))
			.and_then(|total| total.checked_add(&amounts.vault_earnings))
			.and_then(|total| total.checked_add(&amounts.mining_earnings))
			.and_then(|total| total.checked_add(&amounts.mining_mints))
			.ok_or("Collect amount totals overflow")
	};
	let mut accrued = T::Balance::zero();
	let mut micronots = T::Balance::zero();
	let mining_index = MiningMintIndex::<T>::get().microgons_per_seat;
	let mut bonds = 0u128;
	let mut stakes = 0u128;
	let mut fission_liquidity = T::Balance::zero();
	for position in PositionsByAccount::<T>::iter_values() {
		let mining_mints = mining_index
			.checked_sub(&position.mining_mint_checkpoint)
			.and_then(|rate| rate.checked_mul(&u128::from(position.mining_seats).into()))
			.ok_or("Mining mint checkpoint or quantity is invalid")?;
		accrued = accrued
			.checked_add(&checked_total(position.collect_accrued)?)
			.and_then(|total| total.checked_add(&mining_mints))
			.ok_or("Collect account accrual overflows")?;
		micronots = micronots
			.checked_add(&position.collect_accrued.mining_micronots)
			.ok_or("Mining micronot accrual overflows")?;
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

	let funding = CollectTotals::<T>::get();
	let outstanding = checked_total(funding.funded)?
		.checked_sub(&checked_total(funding.collected)?)
		.and_then(|total| total.checked_sub(&funding.expired_vault_earnings))
		.ok_or("Collected amounts exceed Collect funding")?;
	let pending_backing = PendingCollectBacking::<T>::get().unwrap_or_default();
	let pending = checked_total(pending_backing)?;
	let liabilities = outstanding.checked_add(&pending).ok_or("Collect obligations overflow")?;
	ensure!(accrued <= liabilities, "Account accrual exceeds Collect backing");
	let outstanding_micronots = funding
		.funded
		.mining_micronots
		.checked_sub(&funding.collected.mining_micronots)
		.ok_or("Collected micronots exceed funding")?;
	let pending_micronots = PendingCollectBacking::<T>::get().unwrap_or_default().mining_micronots;
	ensure!(
		micronots <=
			outstanding_micronots
				.checked_add(&pending_micronots)
				.ok_or("Micronot obligations overflow")?,
		"Micronot accrual exceeds backing"
	);

	if !outstanding.is_zero() {
		let required = outstanding
			.checked_add(&T::Currency::minimum_balance())
			.ok_or("Collect backing overflows")?;
		ensure!(
			T::Currency::balance(&Pallet::<T>::account_id()) >= required,
			"Collect custody is underfunded"
		);
	}
	if !outstanding_micronots.is_zero() {
		let required = outstanding_micronots
			.checked_add(&T::OwnershipCurrency::minimum_balance())
			.ok_or("Micronot backing overflows")?;
		ensure!(
			T::OwnershipCurrency::balance(&Pallet::<T>::account_id()) >= required,
			"ARGNOT Collect custody is underfunded"
		);
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
		new_test_ext().execute_with(|| {
			STORAGE_VERSION.put::<TreasuryPositions>();
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
		new_test_ext().execute_with(|| {
			STORAGE_VERSION.put::<TreasuryPositions>();
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
		new_test_ext().execute_with(|| {
			STORAGE_VERSION.put::<TreasuryPositions>();
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
	fn try_state_checks_implicit_index_backing_and_counter_overflow() {
		new_test_ext().execute_with(|| {
			CurrentFrame::set(0);
			assert_ok!(TreasuryPositions::account_quantity_updated(
				&9,
				PositionQuantity::Bonds,
				0u128,
				1u128
			));
			index_frame(
				1,
				PositionQuantities { bonds: 1, ..Default::default() },
				CollectAmounts { bond_earnings: 10, ..Default::default() },
			);
			assert_eq!(
				PositionsByAccount::<Test>::get(9).unwrap().collect_accrued.total_argons(),
				0
			);
			assert_ok!(TreasuryPositions::try_state(1));

			assert_ok!(Balances::transfer(
				&TreasuryPositions::account_id(),
				&1,
				10,
				Preservation::Preserve
			));
			assert_eq!(
				TreasuryPositions::try_state(1),
				Err("Collect custody is underfunded".into())
			);

			CollectTotals::<Test>::mutate(|totals| {
				totals.funded.bond_earnings = u128::MAX;
				totals.funded.stake_earnings = 1;
			});
			assert_eq!(
				TreasuryPositions::try_state(1),
				Err("Collect amount totals overflow".into())
			);
		});
	}

	#[test]
	fn try_state_rejects_old_storage_before_decoding_current_records() {
		new_test_ext().execute_with(|| {
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
