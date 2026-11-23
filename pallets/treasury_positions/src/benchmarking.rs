use super::*;
use crate::pallet::STORAGE_VERSION;
use argon_primitives::MICROGONS_PER_ARGON;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

// Whole bond/stake quantities and microgon liquidity at the certification and full-earnings levels.
const CERTIFICATION_AMOUNT: u128 = 2_500;
const FULL_EARNINGS_AMOUNT: u128 = 5_000;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn allocate_mining_mint() {
		STORAGE_VERSION.put::<Pallet<T>>();
		#[block]
		{
			assert!(Pallet::<T>::allocate_mining_mint(1, 1_000_000u128.into(), 1440).unwrap());
		}
		assert_eq!(MiningMintIndex::<T>::get().last_processed_frame, Some(1));
	}

	#[benchmark]
	fn mining_seats_changed() {
		let owner: T::AccountId = account("mining-beneficiary", 0, 0);
		let mut position = certified_position::<T>();
		position.mining_seats = 144;
		PositionsByAccount::<T>::insert(&owner, position);
		MiningMintIndex::<T>::put(MiningMintIndexState {
			microgons_per_seat: 1_000_000u128.into(),
			last_processed_frame: Some(1),
		});
		#[block]
		{
			Pallet::<T>::mining_seats_changed(&owner, 1, 1).unwrap();
		}
		assert_eq!(
			PositionsByAccount::<T>::get(owner).unwrap().collect_accrued.mining_mints,
			144_000_000u128.into()
		);
	}

	#[benchmark]
	fn prepare_collect() {
		STORAGE_VERSION.put::<Pallet<T>>();
		let amounts =
			CollectAmounts { bond_earnings: T::Currency::minimum_balance(), ..Default::default() };
		PendingCollectBacking::<T>::put(amounts);
		T::Currency::mint_into(&Pallet::<T>::account_id(), T::Currency::minimum_balance()).unwrap();
		#[block]
		{
			Pallet::<T>::prepare_collect().unwrap();
		}
		assert!(!PendingCollectBacking::<T>::exists());
	}

	#[benchmark]
	fn indexed_frame() {
		EarningsIndexes::<T>::put(EarningsIndexState {
			last_processed_frame: Some(1),
			..Default::default()
		});
		#[block]
		{
			assert_eq!(Pallet::<T>::indexed_frame(), Some(1));
		}
	}

	#[benchmark]
	fn allocate_frame_earnings() {
		STORAGE_VERSION.put::<Pallet<T>>();
		let amounts = CollectAmounts {
			bond_earnings: 100u128.into(),
			stake_earnings: 100u128.into(),
			bitcoin_earnings: 100u128.into(),
			..Default::default()
		};
		let quantities =
			PositionQuantities { bonds: 10, stakes: 10, fission_liquidity: 1_000_000u128.into() };
		T::Currency::mint_into(
			&Pallet::<T>::account_id(),
			T::Currency::minimum_balance() + amounts.total_argons(),
		)
		.unwrap();
		EarningsIndexesAtFrameStart::<T>::insert(
			2,
			FrameEarningsIndexes { values: None, pending_account_count: 1 },
		);
		#[block]
		{
			Pallet::<T>::allocate_frame_earnings(1, &quantities, amounts).unwrap();
		}
		assert_eq!(
			EarningsIndexesAtFrameStart::<T>::get(2).unwrap().values,
			Some(EarningsIndexes::<T>::get().values)
		);
	}

	#[benchmark]
	fn settle_earning_changes(c: Linear<0, 32>) {
		let owner: T::AccountId = account("index-owner", 0, 0);
		let mut position = certified_position::<T>();
		position.earning_positions.bonds.quantity = 10;
		position.earning_positions.stakes.quantity = 10;
		position.earning_positions.bitcoin_liquidity.quantity = 1_000_000;
		if c > 0 {
			position.earning_positions.next_change_frame = Some(2);
			position.earning_positions.last_change_frame = Some(u64::from(c) + 1);
		}
		for offset in 0..c {
			let frame = u64::from(offset) + 2;
			EarningPositionChanges::<T>::insert(
				&owner,
				frame,
				EarningPositionChange {
					quantities: position.quantities,
					next_frame: if offset + 1 < c { Some(frame + 1) } else { None },
				},
			);
			EarningsIndexesAtFrameStart::<T>::insert(
				frame,
				FrameEarningsIndexes {
					values: Some(EarningsIndexValues {
						bonds: FixedU128::from_u32(offset + 1),
						stakes: FixedU128::from_u32(offset + 1),
						bitcoin_liquidity: FixedU128::from_u32(offset + 1),
					}),
					pending_account_count: 1,
				},
			);
		}
		PositionsByAccount::<T>::insert(&owner, position);
		EarningsIndexes::<T>::put(EarningsIndexState {
			values: EarningsIndexValues {
				bonds: FixedU128::from_u32(c + 1),
				stakes: FixedU128::from_u32(c + 1),
				bitcoin_liquidity: FixedU128::from_u32(c + 1),
			},
			last_processed_frame: Some(u64::from(c) + 1),
			..Default::default()
		});
		#[block]
		{
			Pallet::<T>::settle_account_indexes(&owner).unwrap();
		}
		assert!(PositionsByAccount::<T>::get(&owner)
			.unwrap()
			.earning_positions
			.next_change_frame
			.is_none());
	}

	#[benchmark]
	fn ensure_account() {
		let owner: T::AccountId = account("new-pool-beneficiary", 0, 0);
		#[block]
		{
			Pallet::<T>::ensure_account(&owner);
		}
		assert!(PositionsByAccount::<T>::contains_key(owner));
	}

	#[benchmark]
	fn collect() {
		let caller: T::AccountId = account("collect-recipient", 0, 0);
		let earnings: T::Balance = (FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into();
		let mut position = certified_position::<T>();
		position.collect_accrued = CollectAmounts {
			bond_earnings: earnings,
			stake_earnings: earnings,
			bitcoin_earnings: earnings,
			stabilizer_pool_earnings: earnings,
			stabilizer_buy_earnings: earnings,
			liquid_repayment_earnings: earnings,
			vault_earnings: earnings,
			mining_earnings: earnings,
			mining_mints: earnings,
			mining_micronots: earnings,
		};
		let amounts = position.collect_accrued;
		PositionsByAccount::<T>::insert(&caller, position);
		T::Currency::mint_into(&Pallet::<T>::account_id(), T::Currency::minimum_balance()).unwrap();
		PendingCollectBacking::<T>::put(amounts);
		STORAGE_VERSION.put::<Pallet<T>>();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()));

		let position = PositionsByAccount::<T>::get(&caller).unwrap();
		assert_eq!(position.collect_accrued, CollectAmounts::default());
		assert_eq!(position.collected, amounts);
	}

	#[benchmark]
	fn record_funding() {
		let amounts = CollectAmounts { vault_earnings: 1_000_000u128.into(), ..Default::default() };
		#[block]
		{
			Pallet::<T>::record_funding(amounts);
		}
		assert_eq!(CollectTotals::<T>::get().funded, amounts);
	}

	#[benchmark]
	fn expire_vault_earnings() {
		let owner: T::AccountId = account("vault-expiry", 0, 0);
		let amount: T::Balance = 1_000_000u128.into();
		STORAGE_VERSION.put::<Pallet<T>>();
		Pallet::<T>::fund(CollectSource::Vaults, amount).unwrap();
		Pallet::<T>::accrue(&owner, CollectSource::Vaults, amount).unwrap();
		PendingCollectBacking::<T>::put(CollectAmounts {
			bond_earnings: amount,
			..Default::default()
		});
		#[block]
		{
			Pallet::<T>::expire_vault_earnings(&owner, amount).unwrap();
		}
		assert_eq!(
			PositionsByAccount::<T>::get(owner).unwrap().collect_accrued.vault_earnings,
			0u128.into()
		);
		assert_eq!(CollectTotals::<T>::get().expired_vault_earnings, amount);
	}

	#[benchmark]
	fn fund() {
		T::Currency::mint_into(&Pallet::<T>::account_id(), T::Currency::minimum_balance()).unwrap();
		let amount = (FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into();
		STORAGE_VERSION.put::<Pallet<T>>();
		PendingCollectBacking::<T>::put(CollectAmounts {
			stabilizer_pool_earnings: amount,
			..Default::default()
		});
		#[block]
		{
			Pallet::<T>::fund(argon_primitives::treasury::CollectSource::StabilizerPools, amount)
				.unwrap();
		}
		assert_eq!(CollectTotals::<T>::get().funded.stabilizer_pool_earnings, amount + amount);
	}

	#[benchmark]
	fn accrue() {
		let account: T::AccountId = account("pool-beneficiary", 0, 0);
		let amount = (FULL_EARNINGS_AMOUNT * MICROGONS_PER_ARGON).into();
		STORAGE_VERSION.put::<Pallet<T>>();
		let amounts = CollectAmounts { stabilizer_pool_earnings: amount, ..Default::default() };
		PendingCollectBacking::<T>::put(amounts);
		CollectTotals::<T>::put(argon_primitives::treasury::CollectTotals {
			expired_vault_earnings: 0u128.into(),
			funded: amounts,
			collected: Default::default(),
		});
		T::Currency::mint_into(&Pallet::<T>::account_id(), amount + T::Currency::minimum_balance())
			.unwrap();
		#[block]
		{
			Pallet::<T>::accrue(
				&account,
				argon_primitives::treasury::CollectSource::StabilizerPools,
				amount,
			)
			.unwrap();
		}
		assert_eq!(
			PositionsByAccount::<T>::get(account)
				.unwrap()
				.collect_accrued
				.stabilizer_pool_earnings,
			amount
		);
	}

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
		let quantities = NetworkPositionQuantities {
			account_quantities: certified_position::<T>().quantities,
			vault_securitized_satoshis: CERTIFICATION_AMOUNT,
		};
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
		NetworkTotals::<T>::put(NetworkPositionQuantities {
			account_quantities: certified_position::<T>().quantities,
			vault_securitized_satoshis: CERTIFICATION_AMOUNT,
		});
		#[block]
		{
			Pallet::<T>::account_quantity_updated(
				&account,
				PositionQuantity::Bonds,
				CERTIFICATION_AMOUNT,
				FULL_EARNINGS_AMOUNT,
			)
			.unwrap();
		}
		assert_eq!(NetworkTotals::<T>::get().account_quantities.bonds, FULL_EARNINGS_AMOUNT);
		assert_eq!(
			PositionsByAccount::<T>::get(&account).unwrap().quantities.bonds,
			FULL_EARNINGS_AMOUNT
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
		..Default::default()
	}
}
