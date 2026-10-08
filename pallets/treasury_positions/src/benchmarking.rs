use super::*;
use argon_primitives::MICROGONS_PER_ARGON;
use frame_benchmarking::v2::*;

// Whole bond/stake quantities and microgon liquidity at the certification and full-earnings levels.
const CERTIFICATION_AMOUNT: u128 = 2_500;
const FULL_EARNINGS_AMOUNT: u128 = 5_000;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn bond_principal() {
		let account = account::<T::AccountId>("position", 0, 0);
		PositionsByAccount::<T>::insert(&account, certified_position::<T>());
		#[block]
		{
			assert_eq!(
				Pallet::<T>::bond_principal(&account),
				(CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON).into()
			);
		}
	}

	#[benchmark]
	fn account_quantities() {
		let account = account::<T::AccountId>("position", 0, 0);
		let position = certified_position::<T>();
		PositionsByAccount::<T>::insert(&account, &position);
		#[block]
		{
			assert_eq!(Pallet::<T>::account_quantities(&account), position.quantities);
		}
	}

	#[benchmark]
	fn network_totals() {
		let quantities = certified_position::<T>().quantities;
		NetworkTotals::<T>::put(quantities);
		#[block]
		{
			assert_eq!(Pallet::<T>::network_totals(), quantities);
		}
	}

	#[benchmark]
	fn account_quantity_updated() {
		let account = account::<T::AccountId>("position", 0, 0);
		PositionsByAccount::<T>::insert(&account, certified_position::<T>());
		NetworkTotals::<T>::put(certified_position::<T>().quantities);
		#[block]
		{
			Pallet::<T>::account_quantity_updated(
				&account,
				PositionQuantity::FissionLiquidity,
				CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON,
				FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON,
			)
			.unwrap();
		}
		assert_eq!(
			NetworkTotals::<T>::get().fission_liquidity,
			(FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into()
		);
		assert_eq!(
			PositionsByAccount::<T>::get(&account).unwrap().quantities.fission_liquidity,
			(FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into()
		);
	}

	#[benchmark]
	fn upstream_position() {
		let account = account::<T::AccountId>("position", 0, 0);
		let position = certified_position::<T>();
		PositionsByAccount::<T>::insert(&account, &position);
		#[block]
		{
			assert_eq!(Pallet::<T>::upstream_position(&account), position.upstream);
		}
	}

	#[benchmark]
	fn set_upstream_position() {
		let account = account::<T::AccountId>("position", 0, 0);
		let position = certified_position::<T>();
		PositionsByAccount::<T>::insert(&account, &position);
		#[block]
		{
			Pallet::<T>::set_upstream_position(&account, position.upstream);
		}
		assert_eq!(PositionsByAccount::<T>::get(&account).unwrap(), position);
	}

	#[benchmark]
	fn bitcoin_position_updated() {
		let account = account::<T::AccountId>("position", 0, 0);
		let position = certified_position::<T>();
		PositionsByAccount::<T>::insert(&account, &position);
		#[block]
		{
			Pallet::<T>::bitcoin_position_updated(
				&account,
				1,
				BitcoinLockPosition {
					activated_securitization: (CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON).into(),
					allocated_securitization: (CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON).into(),
				},
				BitcoinLockPosition {
					activated_securitization: (FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into(),
					allocated_securitization: (FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into(),
				},
			)
			.unwrap();
		}
		assert_eq!(
			Pallet::<T>::upstream_position(&account).unwrap().bitcoin_securitization,
			(FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into()
		);
	}

	#[benchmark]
	fn bond_position_updated() {
		let account = account::<T::AccountId>("position", 0, 0);
		PositionsByAccount::<T>::insert(&account, certified_position::<T>());
		#[block]
		{
			Pallet::<T>::bond_position_updated(
				&account,
				1,
				(CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON).into(),
				(FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into(),
			)
			.unwrap();
		}
		assert_eq!(
			Pallet::<T>::bond_principal(&account),
			(FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into()
		);
		assert_eq!(
			Pallet::<T>::upstream_position(&account).unwrap().bond_principal,
			(FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into()
		);
	}
}

// Seed a populated account at certification levels; proof estimates still use MaxEncodedLen.
fn certified_position<T: Config>() -> Position<T::Balance> {
	Position {
		bond_principal: (CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON).into(),
		quantities: PositionQuantities {
			bonds: CERTIFICATION_AMOUNT,
			stakes: CERTIFICATION_AMOUNT,
			fission_liquidity: (CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON).into(),
		},
		upstream: Some(UpstreamPosition {
			vault_id: 1,
			bitcoin_securitization: (CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON).into(),
			bitcoin_allocated_securitization: (CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON).into(),
			bond_principal: (CERTIFICATION_AMOUNT * MICROGONS_PER_ARGON).into(),
		}),
	}
}
