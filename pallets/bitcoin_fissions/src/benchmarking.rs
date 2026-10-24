#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{
	bitcoin::{BitcoinLockId, SATOSHIS_PER_BITCOIN},
	treasury::PositionQuantity,
	vault::{CosignerGuarantee, MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES},
	BitcoinFissionsProvider, MICROGONS_PER_ARGON,
};
use polkadot_sdk::frame_benchmarking::v2::*;

const MAX_FISSIONS_PER_LOCK_BENCH: u32 = 50;
const CERTIFICATION_LIQUIDITY: u128 = 2_500 * MICROGONS_PER_ARGON;
const FULL_EARNINGS_LIQUIDITY: u128 = 5_000 * MICROGONS_PER_ARGON;
const MICROGONS_AT_TARGET_PER_BTC: u128 = 100_000 * MICROGONS_PER_ARGON;
const FISSION_SATOSHIS: u64 = SATOSHIS_PER_BITCOIN / 40;

#[benchmarks(
	where
		T::AccountId: codec::Decode,
)]
mod benchmarks {
	use super::*;
	use frame_system::RawOrigin;

	#[benchmark]
	fn create() -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("fission-owner", 0, 0);

		#[extrinsic_call]
		_(
			RawOrigin::Signed(account_id.clone()),
			0,
			0,
			1,
			FISSION_SATOSHIS,
			T::Balance::from(MICROGONS_AT_TARGET_PER_BTC),
			None,
		);

		assert!(FissionByOwnerAndId::<T>::contains_key(&account_id, 0));
		assert!(FissionIdsByLockId::<T>::get(1).contains(&0));
		assert_eq!(
			FissionByOwnerAndId::<T>::get(&account_id, 0).unwrap().liquidity_promised,
			T::Balance::from(CERTIFICATION_LIQUIDITY)
		);
		Ok(())
	}

	#[benchmark]
	fn ratchet() -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("fission-owner", 0, 0);
		let block_number = frame_system::Pallet::<T>::block_number();
		let current_rate = T::Balance::from(MICROGONS_AT_TARGET_PER_BTC * 2);
		T::PositionProvider::account_quantity_updated(
			&account_id,
			PositionQuantity::FissionLiquidity,
			0u128,
			FULL_EARNINGS_LIQUIDITY,
		)?;
		FissionByOwnerAndId::<T>::insert(
			&account_id,
			0,
			Fission {
				liquid_id: 0,
				lock_id: 1,
				satoshis: FISSION_SATOSHIS,
				microgons_at_target_per_btc: current_rate,
				last_ratchet_tick: 0,
				liquidity_promised: T::Balance::from(FULL_EARNINGS_LIQUIDITY),
				created_at_argon_block: block_number,
				ratchet_number: 0,
				last_updated_argon_block: block_number,

				cosigner_guarantee: CosignerGuarantee {
					basis: BitcoinSecuritizationBasis {
						satoshis: FISSION_SATOSHIS,
						microgons_at_target_per_btc: current_rate,
						tick: 0,
					},
					guaranteed_microgons: T::Balance::from(FULL_EARNINGS_LIQUIDITY),
					fund_hold_extensions: Default::default(),
				},
			},
		);
		let owner_balance = T::Currency::minimum_balance()
			.saturating_add(T::Balance::from(CERTIFICATION_LIQUIDITY));
		T::Currency::mint_into(&account_id, owner_balance)
			.map_err(|_| BenchmarkError::Stop("failed to seed Fission owner balance"))?;

		#[extrinsic_call]
		_(
			RawOrigin::Signed(account_id.clone()),
			0,
			T::Balance::from(MICROGONS_AT_TARGET_PER_BTC),
			None,
		);

		assert_eq!(
			FissionByOwnerAndId::<T>::get(&account_id, 0)
				.expect("benchmark Fission exists")
				.ratchet_number,
			1
		);
		assert_eq!(
			FissionByOwnerAndId::<T>::get(&account_id, 0).unwrap().liquidity_promised,
			T::Balance::from(CERTIFICATION_LIQUIDITY)
		);
		Ok(())
	}

	#[benchmark]
	fn close() -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("fission-owner", 0, 0);
		T::PositionProvider::account_quantity_updated(
			&account_id,
			PositionQuantity::FissionLiquidity,
			0u128,
			CERTIFICATION_LIQUIDITY,
		)?;
		let block_number = frame_system::Pallet::<T>::block_number();
		FissionByOwnerAndId::<T>::insert(
			&account_id,
			0,
			Fission {
				liquid_id: 0,
				lock_id: 1,
				satoshis: FISSION_SATOSHIS,
				microgons_at_target_per_btc: T::Balance::from(MICROGONS_AT_TARGET_PER_BTC),
				last_ratchet_tick: 0,
				liquidity_promised: T::Balance::from(CERTIFICATION_LIQUIDITY),
				created_at_argon_block: block_number,
				ratchet_number: 0,
				last_updated_argon_block: block_number,

				cosigner_guarantee: CosignerGuarantee {
					basis: BitcoinSecuritizationBasis {
						satoshis: FISSION_SATOSHIS,
						microgons_at_target_per_btc: T::Balance::from(MICROGONS_AT_TARGET_PER_BTC),
						tick: 0,
					},
					guaranteed_microgons: T::Balance::from(CERTIFICATION_LIQUIDITY),
					fund_hold_extensions: Default::default(),
				},
			},
		);
		FissionIdsByLockId::<T>::try_mutate(1, |fission_ids| {
			fission_ids
				.try_insert(0)
				.map(|_| ())
				.map_err(|_| BenchmarkError::Stop("failed to seed active Fission index"))
		})?;
		let owner_balance = T::Currency::minimum_balance()
			.saturating_add(T::Balance::from(CERTIFICATION_LIQUIDITY));
		T::Currency::mint_into(&account_id, owner_balance)
			.map_err(|_| BenchmarkError::Stop("failed to seed Fission owner balance"))?;

		#[extrinsic_call]
		_(RawOrigin::Signed(account_id.clone()), 0);

		assert!(!FissionByOwnerAndId::<T>::contains_key(&account_id, 0));
		Ok(())
	}

	#[benchmark]
	fn lock_spent(l: Linear<0, MAX_FISSIONS_PER_LOCK_BENCH>) -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("fission-owner", 0, 0);
		T::PositionProvider::account_quantity_updated(
			&account_id,
			PositionQuantity::FissionLiquidity,
			0u128,
			CERTIFICATION_LIQUIDITY.saturating_mul(l as u128),
		)?;
		let lock_id: BitcoinLockId = 1;
		let block_number = frame_system::Pallet::<T>::block_number();

		for fission_id in 0..l as u64 {
			FissionIdsByLockId::<T>::try_mutate(lock_id, |fission_ids| {
				fission_ids
					.try_insert(fission_id)
					.map(|_| ())
					.map_err(|_| BenchmarkError::Stop("active Fission benchmark capacity exceeded"))
			})?;
			FissionByOwnerAndId::<T>::insert(
				&account_id,
				fission_id,
				Fission {
					liquid_id: 0,
					lock_id,
					satoshis: FISSION_SATOSHIS,
					microgons_at_target_per_btc: T::Balance::from(MICROGONS_AT_TARGET_PER_BTC),
					last_ratchet_tick: 0,
					liquidity_promised: T::Balance::from(CERTIFICATION_LIQUIDITY),
					created_at_argon_block: block_number,
					ratchet_number: 0,
					last_updated_argon_block: block_number,

					cosigner_guarantee: CosignerGuarantee {
						basis: BitcoinSecuritizationBasis {
							satoshis: FISSION_SATOSHIS,
							microgons_at_target_per_btc: T::Balance::from(
								MICROGONS_AT_TARGET_PER_BTC,
							),
							tick: 0,
						},
						guaranteed_microgons: T::Balance::from(CERTIFICATION_LIQUIDITY),
						fund_hold_extensions: Default::default(),
					},
				},
			);
		}

		#[block]
		{
			<Pallet<T> as BitcoinFissionsProvider<T::AccountId, T::Balance>>::close_for_lock(
				&account_id,
				lock_id,
				T::Balance::from(CERTIFICATION_LIQUIDITY.saturating_mul(l as u128)),
			)?;
		}

		assert!(FissionIdsByLockId::<T>::get(lock_id).is_empty());
		assert!((0..l as u64)
			.all(|fission_id| !FissionByOwnerAndId::<T>::contains_key(&account_id, fission_id)));
		Ok(())
	}

	#[benchmark]
	fn provider_get_account_fission_liquidity() -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("fission-owner", 0, 0);
		T::PositionProvider::account_quantity_updated(
			&account_id,
			PositionQuantity::FissionLiquidity,
			0u128,
			CERTIFICATION_LIQUIDITY,
		)?;
		// Provider work is benchmarked and charged separately. Runtime benchmarks use a
		// no-op provider; pallet tests can use the real maintained quantity.
		let expected = T::PositionProvider::account_quantities(&account_id).fission_liquidity;
		let liquidity;

		#[block]
		{
			liquidity = <Pallet<T> as BitcoinFissionsProvider<
				T::AccountId,
				T::Balance,
			>>::get_account_fission_liquidity(&account_id);
		}

		assert_eq!(liquidity, expected);
		Ok(())
	}

	#[benchmark]
	fn provider_get_lock_fission_guarantees() -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("fission-owner", 0, 0);
		let block_number = frame_system::Pallet::<T>::block_number();
		let lock_id = 1;
		let mut release_extensions = BoundedBTreeMap::new();
		for day in 1..=MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES {
			release_extensions
				.try_insert(u64::from(day) * 144, T::Balance::from(1u128))
				.expect("bounded by the maturity limit");
		}
		let mut fission_ids = BoundedBTreeSet::new();
		for fission_id in 0..T::MaxFissionsPerLock::get() as u64 {
			fission_ids.try_insert(fission_id).expect("bounded by MaxFissionsPerLock");
			FissionByOwnerAndId::<T>::insert(
				&account_id,
				fission_id,
				Fission {
					liquid_id: 0,
					lock_id,
					satoshis: FISSION_SATOSHIS,
					microgons_at_target_per_btc: T::Balance::from(
						MICROGONS_AT_TARGET_PER_BTC + 40 * fission_id as u128,
					),
					last_ratchet_tick: 0,
					liquidity_promised: T::Balance::from(
						CERTIFICATION_LIQUIDITY + fission_id as u128,
					),
					created_at_argon_block: block_number,
					ratchet_number: 0,
					last_updated_argon_block: block_number,

					cosigner_guarantee: CosignerGuarantee {
						basis: BitcoinSecuritizationBasis {
							satoshis: FISSION_SATOSHIS,
							microgons_at_target_per_btc: T::Balance::from(
								MICROGONS_AT_TARGET_PER_BTC,
							),
							tick: 0,
						},
						guaranteed_microgons: T::Balance::from(CERTIFICATION_LIQUIDITY),
						fund_hold_extensions: release_extensions.clone(),
					},
				},
			);
		}
		FissionIdsByLockId::<T>::insert(lock_id, fission_ids);
		let guarantees;

		#[block]
		{
			guarantees = <Pallet<T> as BitcoinFissionsProvider<
				T::AccountId,
				T::Balance,
			>>::get_lock_fission_guarantees(&account_id, lock_id);
		}

		assert_eq!(guarantees.len(), T::MaxFissionsPerLock::get() as usize);
		Ok(())
	}
}
