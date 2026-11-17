use super::*;
#[cfg(test)]
use crate::mock::Test;
use crate::{
	mock::{config, evidence, pool_id, pool_manager, stabilizer_pool_price_source},
	price_history::StabilizerPoolUsdQuotes,
	uniswap::sqrt_price_at_uniswap_tick,
};
use frame_system::RawOrigin;
use polkadot_sdk::frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn read_price_index_context() {
		#[block]
		{
			core::hint::black_box(T::CurrentTick::get());
		}
	}

	#[benchmark]
	fn record_stabilizer_pool_spot_price() -> Result<(), BenchmarkError> {
		let frame = 1;
		let (last_tick, price) = fill_price_buckets::<T>(frame)?;

		#[block]
		{
			Pallet::<T>::record_stabilizer_pool_spot_price(
				last_tick + 1,
				price.sqrt_price_x96,
				Some(StabilizerPoolUsdQuotes {
					argon_usd_target_price: price.argon_usd_target_price,
					usdc_usd_price: price.usdc_usd_price,
				}),
				stabilizer_pool_price_source(),
			);
		}

		// Closing this delayed frame needs the observation that merged the full buckets.
		Pallet::<T>::start_stabilizer_pool_price_sampling(frame + 1, last_tick + 2);
		assert_eq!(StabilizerPoolPricesByFrame::<T>::get().get(&frame), Some(&price));
		Ok(())
	}

	#[benchmark]
	fn start_stabilizer_pool_price_sampling() -> Result<(), BenchmarkError> {
		let retained_frames = FrameId::from(T::MaxSampledPriceHistoryFrames::get());
		if retained_frames == 0 {
			return Err(BenchmarkError::Stop("sampled price history is disabled"));
		}
		let frame = retained_frames + 1;
		let (last_tick, price) = fill_price_buckets::<T>(frame)?;
		StabilizerPoolPricesByFrame::<T>::mutate(|history| {
			for frame in 1..=retained_frames {
				history.try_insert(frame, price).unwrap();
			}
		});

		#[block]
		{
			core::hint::black_box(T::CurrentTick::get());
			Pallet::<T>::start_stabilizer_pool_price_sampling(frame + 1, last_tick + 1);
		}

		let history = StabilizerPoolPricesByFrame::<T>::get();
		assert_eq!(history.get(&frame), Some(&price));
		assert!(!history.contains_key(&1));
		Ok(())
	}

	#[benchmark]
	fn register_pool() {
		#[extrinsic_call]
		_(RawOrigin::Root, pool_manager(), config());
		assert_eq!(Pools::<T>::get(pool_manager(), pool_id()), Some(config()));
	}

	#[benchmark]
	fn checkpoint_position() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let encoded = caller.encode();
		if encoded.len() != 32 {
			return Err(BenchmarkError::Stop("checkpoint beneficiaries require AccountId32"));
		}
		let beneficiary = H256::from_slice(&encoded);

		Pallet::<T>::register_pool(RawOrigin::Root.into(), pool_manager(), config())?;
		let first = evidence::<T>(100, 1, beneficiary, u128::MAX, true);
		Pallet::<T>::checkpoint_position(
			RawOrigin::Signed(caller.clone()).into(),
			pool_manager(),
			pool_id(),
			U256::one(),
			Box::new(first),
		)?;
		// Exercise a replacement after the prior registration has become eligible.
		let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
		let previous =
			PositionCheckpoints::<T>::get(config().position_manager, U256::one()).unwrap();
		let range = StabilizerPoolLiquidityRange::from(&previous);
		EligibleFromFrameByPosition::<T>::insert(
			StabilizerPoolPositionId {
				position_manager: config().position_manager,
				token_id: U256::one(),
			},
			current_frame,
		);
		RegisteredLiquidityByRange::<T>::mutate(range, |liquidity| {
			liquidity.current_liquidity = liquidity.next_frame_liquidity;
		});

		let proof = Box::new(evidence::<T>(200, 2, beneficiary, u128::MAX, true));

		let header = EthereumVerifier::<T>::verified_execution_header(
			&proof.subscriber.execution_block_proof,
		)
		.map_err(|_| BenchmarkError::Stop("fixture execution header"))?;

		// Proof providers have their own measured weights. Measure the local decode, authorization,
		// storage transition and event here instead of counting those providers twice.
		#[block]
		{
			let config = Pools::<T>::get(pool_manager(), pool_id()).unwrap();

			assert_eq!(
				proof.subscriber.execution_block_proof,
				proof.position_manager.execution_block_proof
			);
			assert_eq!(
				proof.subscriber.execution_block_proof,
				proof.pool_manager.execution_block_proof
			);

			let position = evm::decode_position::<T>(&config, pool_id(), U256::one(), &proof)?;
			assert_eq!(caller.encode().as_slice(), position.argon_account_id.as_bytes());

			let block = ProofBlock {
				hash: header.block_hash,
				number: header.block_number,
				timestamp_millis: header.timestamp_millis,
			};

			Pallet::<T>::record_checkpoint(
				config.position_manager,
				U256::one(),
				PositionCheckpoint {
					beneficiary: caller,
					pool_manager: pool_manager(),
					pool_id: pool_id(),
					subscriber: config.subscriber,
					revision: position.revision,
					tick_lower: position.tick_lower,
					tick_upper: position.tick_upper,
					liquidity: position.liquidity,
					maintained_since: block,
					proven_through: block,
				},
			)?;
		}

		let checkpoint =
			PositionCheckpoints::<T>::get(config().position_manager, U256::one()).unwrap();
		assert_eq!(checkpoint.maintained_since.number, 200);
		assert_eq!(checkpoint.proven_through.number, 200);
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, new_test_with_genesis::<Test>(|_| {}), Test);
}

fn fill_price_buckets<T: Config>(
	frame: FrameId,
) -> Result<(Tick, StabilizerPoolFramePrice), BenchmarkError> {
	let bucket_count = Tick::from(T::MaxPriceBucketsPerFrame::get());
	if bucket_count == 0 {
		return Err(BenchmarkError::Stop("price sampling is disabled"));
	}
	let bucket_ticks = T::TicksPerFrame::get().div_ceil(bucket_count).max(1);
	let first_tick = T::CurrentTick::get();
	let first_tick_after_full_buckets = first_tick + bucket_ticks * bucket_count;
	let pool_source = stabilizer_pool_price_source();
	let price = StabilizerPoolFramePrice {
		sqrt_price_x96: sqrt_price_at_uniswap_tick(524_287).unwrap(),
		argon_usd_target_price: FixedU128::one(),
		usdc_usd_price: FixedU128::one(),
		pool: pool_source.pool,
	};
	Pallet::<T>::start_stabilizer_pool_price_sampling(frame, first_tick);
	let usd_quotes = StabilizerPoolUsdQuotes {
		argon_usd_target_price: price.argon_usd_target_price,
		usdc_usd_price: price.usdc_usd_price,
	};
	for tick in first_tick..=first_tick_after_full_buckets {
		Pallet::<T>::record_stabilizer_pool_spot_price(
			tick,
			price.sqrt_price_x96,
			Some(usd_quotes),
			pool_source,
		);
	}
	Ok((first_tick_after_full_buckets, price))
}
