#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_primitives::{
	bitcoin::{BitcoinLockId, LiquidRepayPrices, SATOSHIS_PER_BITCOIN},
	treasury::PositionQuantity,
	vault::{CosignerGuarantee, MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES},
	BitcoinFissionsProvider, PriceIndex, PriceIndexUpdated, MICROGONS_PER_ARGON,
};
use pallet_prelude::benchmarking::{
	set_benchmark_bitcoin_locks_runtime_state, BenchmarkBitcoinLocksRuntimeState,
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
	fn provider_has_repayment_window(f: Linear<1, 10>) -> Result<(), BenchmarkError> {
		let (owner, _, _) = repayment_fissions::<T>(f, true)?;
		LiquidTicksByOwnerAndId::<T>::insert(
			&owner,
			1,
			LiquidTicks { created_at_tick: Some(0), last_ratchet_tick: Some(0) },
		);
		let eligible;
		#[block]
		{
			eligible = <Pallet<T> as BitcoinFissionsProvider<
				T::AccountId,
				T::Balance,
			>>::is_liquid_repayment_valid(&owner, 1, 2, 30);
		}
		assert!(eligible);
		Ok(())
	}

	#[benchmark]
	fn provider_next_r_value_high_water() {
		RValueHighWaterByFrame::<T>::insert(
			40u64.to_ordered(),
			RValueHighWaterMark { r_value: FixedU128::one(), first_tick: 100, end_tick: Some(200) },
		);
		let high;
		#[block]
		{
			high = Pallet::<T>::next_r_value_high_water(40);
		}
		assert_eq!(high.unwrap().0, 40);
	}

	#[benchmark]
	fn record_r_value() {
		set_benchmark_bitcoin_locks_runtime_state(BenchmarkBitcoinLocksRuntimeState {
			current_frame_id: 40,
			current_tick: 100,
			did_start_new_frame: false,
		});
		let index = PriceIndex {
			tick: 71,
			btc_usd_price: FixedU128::from_u32(60_000),
			argonot_usd_price: FixedU128::one(),
			argon_usd_price: FixedU128::one(),
			argon_usd_target_price: FixedU128::one(),
			argon_time_weighted_average_liquidity: 1,
			usdc_usd_price: Some(FixedU128::one()),
			stabilizer_pool_sqrt_price_x96: None,
		};
		#[block]
		{
			Pallet::<T>::on_price_index(&index);
		}
		assert_eq!(
			RValueHighWaterByFrame::<T>::get(40u64.to_ordered()).unwrap().r_value,
			FixedU128::one()
		);
	}

	#[benchmark]
	fn prune_r_values(p: Linear<1, 16>) {
		set_benchmark_bitcoin_locks_runtime_state(BenchmarkBitcoinLocksRuntimeState {
			current_frame_id: 100,
			current_tick: 100,
			did_start_new_frame: true,
		});
		RValueHighWaterByFrame::<T>::insert(
			99u64.to_ordered(),
			RValueHighWaterMark { r_value: FixedU128::one(), first_tick: 0, end_tick: None },
		);
		for frame in 1..=u64::from(p) {
			RValueHighWaterByFrame::<T>::insert(
				frame.to_ordered(),
				RValueHighWaterMark {
					r_value: FixedU128::one(),
					first_tick: 0,
					end_tick: Some(frame),
				},
			);
		}
		#[block]
		{
			Pallet::<T>::prune_r_value_history();
		}
		assert_eq!(RValueHighWaterByFrame::<T>::iter_keys().count(), 1);
		assert_eq!(
			RValueHighWaterByFrame::<T>::get(99u64.to_ordered()).unwrap().end_tick,
			Some(100)
		);
	}

	#[benchmark]
	fn create() -> Result<(), BenchmarkError> {
		let account_id: T::AccountId = account("fission-owner", 0, 0);
		let timestamp = T::TimeProvider::now().as_millis().saturated_into::<u64>();
		let tick = T::TickProvider::ticker().tick_for_time(timestamp);
		LiquidTicksByOwnerAndId::<T>::insert(
			&account_id,
			1,
			LiquidTicks { created_at_tick: Some(tick), last_ratchet_tick: Some(tick) },
		);

		for n in 1..T::MaxFissionsPerLock::get() {
			FissionIdsByLockId::<T>::mutate(1, |ids| {
				ids.try_insert(u64::MAX - u64::from(n)).unwrap()
			});
		}
		for n in 1..T::MaxExpectedFissionsPerLiquid::get() {
			FissionIdsByOwnerAndLiquid::<T>::insert((&account_id, 1), u64::MAX - u64::from(n), ());
		}

		#[extrinsic_call]
		_(
			RawOrigin::Signed(account_id.clone()),
			0,
			1,
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
		LiquidTicksByOwnerAndId::<T>::insert(
			&account_id,
			1,
			LiquidTicks { created_at_tick: Some(0), last_ratchet_tick: Some(0) },
		);
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
				liquid_id: 1,
				lock_id: 1,
				satoshis: FISSION_SATOSHIS,
				microgons_at_target_per_btc: current_rate,
				last_ratchet_price_tick: 0,
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
		let account_id = funded_fission::<T>()?;
		#[extrinsic_call]
		_(RawOrigin::Signed(account_id.clone()), 0);
		assert!(!FissionByOwnerAndId::<T>::contains_key(&account_id, 0));
		Ok(())
	}

	#[benchmark]
	fn repay_liquid(f: Linear<1, 10>) -> Result<(), BenchmarkError> {
		let (owner, budget, prices) = repayment_fissions::<T>(f, false)?;
		#[block]
		{
			with_storage_layer(|| {
				<Pallet<T> as BitcoinFissionsProvider<T::AccountId, T::Balance>>::repay_liquid(
					&owner,
					1,
					budget.into(),
					prices,
					|_| Ok(()),
				)
			})?;
		}
		assert_eq!(FissionIdsByOwnerAndLiquid::<T>::iter_key_prefix((&owner, 1)).count(), 1);
		Ok(())
	}

	#[benchmark]
	fn repay_liquid_complete(f: Linear<1, 10>) -> Result<(), BenchmarkError> {
		let (owner, budget, prices) = repayment_fissions::<T>(f, true)?;

		#[block]
		{
			with_storage_layer(|| {
				<Pallet<T> as BitcoinFissionsProvider<T::AccountId, T::Balance>>::repay_liquid(
					&owner,
					1,
					budget.into(),
					prices,
					|_| Ok(()),
				)
			})?;
		}

		assert!(FissionIdsByOwnerAndLiquid::<T>::iter_key_prefix((&owner, 1)).next().is_none());
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
			FissionIdsByOwnerAndLiquid::<T>::insert((&account_id, fission_id + 1), fission_id, ());
			for n in 1..T::MaxExpectedFissionsPerLiquid::get() {
				FissionIdsByOwnerAndLiquid::<T>::insert(
					(&account_id, fission_id + 1),
					u64::MAX - u64::from(n),
					(),
				);
			}
			FissionByOwnerAndId::<T>::insert(
				&account_id,
				fission_id,
				Fission {
					liquid_id: fission_id + 1,
					lock_id,
					satoshis: FISSION_SATOSHIS,
					microgons_at_target_per_btc: T::Balance::from(MICROGONS_AT_TARGET_PER_BTC),
					last_ratchet_price_tick: 0,
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
					liquid_id: 1,
					lock_id,
					satoshis: FISSION_SATOSHIS,
					microgons_at_target_per_btc: T::Balance::from(
						MICROGONS_AT_TARGET_PER_BTC + 40 * fission_id as u128,
					),
					last_ratchet_price_tick: 0,
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

// Close and partial repayment both decode and partition the largest supported hold schedule.
fn funded_fission<T: Config>() -> Result<T::AccountId, BenchmarkError> {
	let account_id: T::AccountId = account("fission-owner", 0, 0);
	T::PositionProvider::account_quantity_updated(
		&account_id,
		PositionQuantity::FissionLiquidity,
		0u128,
		CERTIFICATION_LIQUIDITY,
	)?;
	let mut extensions = BoundedBTreeMap::new();
	let per_extension =
		CERTIFICATION_LIQUIDITY / u128::from(MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES);
	for height in 1..=MAX_SECURITIZATION_RELEASE_SCHEDULE_ENTRIES {
		extensions
			.try_insert(u64::from(height), per_extension.into())
			.map_err(|_| BenchmarkError::Stop("release schedule exceeded bound"))?;
	}
	let block_number = frame_system::Pallet::<T>::block_number();
	FissionByOwnerAndId::<T>::insert(
		&account_id,
		0,
		Fission {
			liquid_id: 1,
			lock_id: 1,
			satoshis: FISSION_SATOSHIS,
			microgons_at_target_per_btc: MICROGONS_AT_TARGET_PER_BTC.into(),
			last_ratchet_price_tick: 0,
			liquidity_promised: CERTIFICATION_LIQUIDITY.into(),
			created_at_argon_block: block_number,
			ratchet_number: 0,
			last_updated_argon_block: block_number,

			cosigner_guarantee: CosignerGuarantee {
				basis: BitcoinSecuritizationBasis {
					satoshis: FISSION_SATOSHIS,
					microgons_at_target_per_btc: MICROGONS_AT_TARGET_PER_BTC.into(),
					tick: 0,
				},
				guaranteed_microgons: CERTIFICATION_LIQUIDITY.into(),
				fund_hold_extensions: extensions,
			},
		},
	);
	FissionIdsByLockId::<T>::try_mutate(1, |fission_ids| {
		fission_ids
			.try_insert(0)
			.map(|_| ())
			.map_err(|_| BenchmarkError::Stop("failed to seed active Fission index"))
	})?;
	FissionIdsByOwnerAndLiquid::<T>::insert((&account_id, 1), 0, ());
	for n in 1..T::MaxFissionsPerLock::get() {
		FissionIdsByLockId::<T>::mutate(1, |ids| ids.try_insert(u64::MAX - u64::from(n)).unwrap());
	}
	for n in 1..T::MaxExpectedFissionsPerLiquid::get() {
		FissionIdsByOwnerAndLiquid::<T>::insert((&account_id, 1), u64::MAX - u64::from(n), ());
	}
	let owner_balance =
		T::Currency::minimum_balance().saturating_add(CERTIFICATION_LIQUIDITY.into());
	T::Currency::mint_into(&account_id, owner_balance)
		.map_err(|_| BenchmarkError::Stop("failed to seed Fission owner balance"))?;
	Ok(account_id)
}

fn repayment_fissions<T: Config>(
	fissions: u32,
	complete: bool,
) -> Result<(T::AccountId, u128, LiquidRepayPrices), BenchmarkError> {
	let owner = funded_fission::<T>()?;
	let _ = FissionIdsByOwnerAndLiquid::<T>::clear_prefix((&owner, 1), u32::MAX, None);
	let template = FissionByOwnerAndId::<T>::get(&owner, 0).unwrap();
	let mut total = 0u128;
	let mut budget = 0u128;
	for id in 0..u64::from(fissions) {
		let mut position = template.clone();
		position.lock_id = if id + 1 == u64::from(fissions) {
			u64::from(fissions)
		} else if id % 2 == 0 {
			id / 2 + 1
		} else {
			u64::from(fissions) - 1 - id / 2
		};
		if id + 1 == u64::from(fissions) {
			position.satoshis = u64::MAX;
			position.cosigner_guarantee.basis.satoshis = u64::MAX;
		}
		position.liquidity_promised = T::LockProvider::calculate_liquidity_promised(
			position.satoshis,
			position.microgons_at_target_per_btc,
		)
		.map_err(|_| BenchmarkError::Stop("quote setup failed"))?;
		position.cosigner_guarantee.guaranteed_microgons = position.liquidity_promised;
		let value: u128 = position.liquidity_promised.into();
		total += value;
		if !complete && id + 1 == u64::from(fissions) {
			budget += value / 2;
		} else {
			budget += value;
		}
		for n in 1..T::MaxFissionsPerLock::get() {
			FissionIdsByLockId::<T>::mutate(position.lock_id, |ids| {
				ids.try_insert(u64::MAX - u64::from(n)).unwrap()
			});
		}
		FissionIdsByLockId::<T>::mutate(position.lock_id, |ids| ids.try_insert(id).unwrap());
		FissionByOwnerAndId::<T>::insert(&owner, id, position);
		FissionIdsByOwnerAndLiquid::<T>::insert((&owner, 1), id, ());
	}
	T::PositionProvider::account_quantity_updated(
		&owner,
		PositionQuantity::FissionLiquidity,
		CERTIFICATION_LIQUIDITY,
		total,
	)?;
	// This profile commits at target so the setup budget covers exactly the quoted Bitcoin.
	let prices = LiquidRepayPrices::new(
		FixedU128::saturating_from_integer(MICROGONS_AT_TARGET_PER_BTC),
		FixedU128::one(),
		FixedU128::one(),
	)
	.unwrap();
	Ok((owner, budget, prices))
}
