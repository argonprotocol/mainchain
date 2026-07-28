use super::*;
use argon_primitives::{PriceProvider, MICROGONS_PER_ARGON};
use frame_support::traits::Hooks;
use frame_system::RawOrigin;
use pallet_prelude::benchmarking::{
	set_benchmark_bitcoin_locks_runtime_state, BenchmarkBitcoinLocksRuntimeState,
};
use polkadot_sdk::frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn submit() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let index = benchmark_price_index(1);
		let ethereum = EthereumPriceIndex {
			ethereum_usd_price: FixedU128::from_u32(3_000),
			ethereum_gas_price_wei: 1_000_000_000,
			tick: index.tick,
		};
		Operator::<T>::put(caller.clone());

		#[extrinsic_call]
		submit(RawOrigin::Signed(caller.clone()), index, Some(ethereum));

		assert_eq!(Current::<T>::get(), Some(index));
		assert_eq!(CurrentEthereumPrice::<T>::get(), Some(ethereum));
		assert!(!HistoricEthereumPricesByFrame::<T>::get().is_empty());

		Ok(())
	}

	#[benchmark]
	fn set_operator() -> Result<(), BenchmarkError> {
		let operator: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		set_operator(RawOrigin::Root, operator.clone());

		assert_eq!(Operator::<T>::get(), Some(operator));

		Ok(())
	}

	#[benchmark]
	fn on_initialize() -> Result<(), BenchmarkError> {
		let index = benchmark_price_index(10);
		Current::<T>::put(index);
		CurrentFrameArgonotAverage::<T>::put(ArgonotAverageFrameAccumulator {
			frame_id: 1,
			total_microgons_per_argonot: T::Balance::from(2 * MICROGONS_PER_ARGON),
			sample_count: 2,
		});
		set_benchmark_bitcoin_locks_runtime_state(BenchmarkBitcoinLocksRuntimeState {
			current_frame_id: 2,
			current_tick: index.tick,
			did_start_new_frame: true,
		});

		#[block]
		{
			let _ = Pallet::<T>::on_initialize(1u32.into());
		}

		assert_eq!(LastValid::<T>::get(), Some(index));
		assert!(CurrentFrameArgonotAverage::<T>::get().is_some());
		assert!(!HistoricArgonotFloorByFrame::<T>::get().is_empty());

		Ok(())
	}

	#[benchmark]
	fn provider_get_lowest_microgons_per_argonot() -> Result<(), BenchmarkError> {
		let frame_id: FrameId = 2;
		let price = T::Balance::from(3 * MICROGONS_PER_ARGON);
		HistoricArgonotFloorByFrame::<T>::mutate(|history| {
			let _ = history.try_insert(frame_id, price);
		});

		#[block]
		{
			assert_eq!(
				<Pallet<T> as PriceProvider<T::Balance>>::get_lowest_microgons_per_argonot(1),
				Some(price)
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_get_average_microgons_per_argonot() -> Result<(), BenchmarkError> {
		let frame_id: FrameId = 2;
		let price = T::Balance::from(3 * MICROGONS_PER_ARGON);
		HistoricArgonotAverageByFrame::<T>::mutate(|history| {
			let _ = history.try_insert(frame_id, price);
		});

		#[block]
		{
			assert_eq!(
				<Pallet<T> as PriceProvider<T::Balance>>::get_average_microgons_per_argonot(
					frame_id
				),
				Some(price)
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_get_ethereum_prices() -> Result<(), BenchmarkError> {
		let tick = 10;
		Current::<T>::put(benchmark_price_index(tick));
		CurrentEthereumPrice::<T>::put(EthereumPriceIndex {
			ethereum_usd_price: FixedU128::from_u32(3_000),
			ethereum_gas_price_wei: 1_000_000_000,
			tick,
		});
		let mut average = EthereumPriceFrameAccumulator::default();
		average.record(FixedU128::from_u32(3_000), 1_000_000_000);
		HistoricEthereumPricesByFrame::<T>::mutate(|history| {
			let _ = history.try_insert(1, average);
		});
		set_benchmark_bitcoin_locks_runtime_state(BenchmarkBitcoinLocksRuntimeState {
			current_frame_id: 1,
			current_tick: tick,
			did_start_new_frame: false,
		});

		#[block]
		{
			assert!(
				<Pallet<T> as PriceProvider<T::Balance>>::get_average_ethereum_prices_in_microgons(
					10
				)
				.is_some()
			);
		}

		Ok(())
	}

	#[benchmark]
	fn provider_get_liquidity_change_needed() -> Result<(), BenchmarkError> {
		let mut index = benchmark_price_index(10);
		index.argon_usd_target_price = FixedU128::from_rational(101u128, 100u128);
		Current::<T>::put(index);

		#[block]
		{
			assert!(
				<Pallet<T> as PriceProvider<T::Balance>>::get_liquidity_change_needed().is_some()
			);
		}

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(Some(1)), crate::mock::Test);
}

fn benchmark_price_index(tick: Tick) -> PriceIndex {
	PriceIndex {
		tick,
		btc_usd_price: FixedU128::from_rational(6_200_000u128, 100u128),
		argon_usd_price: FixedU128::from_u32(1),
		argon_usd_target_price: FixedU128::from_u32(1),
		argonot_usd_price: FixedU128::from_u32(2),
		argon_time_weighted_average_liquidity: 100_000_000,
	}
}
