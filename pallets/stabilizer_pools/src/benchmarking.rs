use super::*;
#[cfg(test)]
use crate::mock::Test;
use crate::{
	mock::{
		config, pool_id, pool_manager, position_change_evidence, position_evidence,
		stabilizer_pool_price_source,
	},
	price_history::StabilizerPoolUsdQuotes,
	uniswap::{sqrt_price_at_uniswap_tick, MAX_REWARD_RANGES, REWARD_BAND_TOLERANCE},
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
		let _ = Pallet::<T>::start_stabilizer_pool_price_sampling(frame + 1, last_tick + 2);
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
			let _ = Pallet::<T>::start_stabilizer_pool_price_sampling(frame + 1, last_tick + 1);
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
		let previous = benchmark_checkpoint(caller.clone());
		Pallet::<T>::record_checkpoint(config().position_manager, U256::one(), previous.clone())?;
		// Exercise a replacement after the prior registration has become eligible.
		let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
		let range = StabilizerPoolLiquidityRange::from(&previous);
		StabilizerPoolRegistrations::<T>::insert(
			StabilizerPoolPositionId {
				position_manager: config().position_manager,
				token_id: U256::one(),
			},
			StabilizerPoolRegistration::Enrolled { eligible_from: current_frame.saturating_sub(1) },
		);
		seed_review_frame::<T>(current_frame.saturating_sub(1), &previous);
		RegisteredLiquidityByRange::<T>::mutate(range, |liquidity| {
			liquidity.current_liquidity = liquidity.next_frame_liquidity;
		});

		let proof = position_evidence(200, 2, beneficiary, u128::MAX, true, (-100, 100));
		let block = ProofBlock {
			hash: H256::from_low_u64_be(200),
			number: 200,
			timestamp_millis: 2_400_000,
		};

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

	#[benchmark]
	fn advance_liquidity_snapshots() -> Result<(), BenchmarkError> {
		let mut config = config();
		config.key.tick_spacing = 1;
		let pool_id = evm::pool_id(&config.key);
		Pallet::<T>::register_pool(RawOrigin::Root.into(), pool_manager(), config)?;
		// A pool rotation can leave the reviewed and newly closed frames in different markets.
		config.key.fee = config.key.fee.saturating_add(1);
		let reviewed_pool_id = evm::pool_id(&config.key);
		Pallet::<T>::register_pool(RawOrigin::Root.into(), pool_manager(), config)?;
		let mut pool = stabilizer_pool_price_source().pool;
		pool.pool_id = pool_id;
		let price = StabilizerPoolFramePrice {
			pool,
			sqrt_price_x96: sqrt_price_at_uniswap_tick(277_000).unwrap(),
			argon_usd_target_price: FixedU128::one(),
			usdc_usd_price: FixedU128::one(),
			argon_is_token0: false,
		};
		let mut reviewed_price = price;
		reviewed_price.pool.pool_id = reviewed_pool_id;
		let retained_frames = FrameId::from(T::MaxSampledPriceHistoryFrames::get());
		let current_frame = retained_frames.saturating_add(1);
		let reviewed_frame = current_frame.saturating_sub(2);
		let band = StabilizerPoolRewardBand::new(
			price.argon_usd_target_price,
			price.usdc_usd_price,
			price.argon_is_token0,
			1,
			T::RewardPriceBand::get(),
			REWARD_BAND_TOLERANCE,
		)
		.unwrap();
		let liquidity = U256::from(10u128.pow(24));
		let mut liquidity_by_range = BoundedBTreeMap::new();
		for (lower, upper) in band.reward_ranges().unwrap() {
			RegisteredLiquidityByRange::<T>::insert(
				StabilizerPoolLiquidityRange {
					pool_manager: pool.pool_manager,
					pool_id,
					lower_uniswap_tick: lower,
					upper_uniswap_tick: upper,
				},
				StabilizerPoolRangeLiquidity {
					frame_id: reviewed_frame,
					current_liquidity: liquidity,
					next_frame_liquidity: liquidity,
				},
			);
			liquidity_by_range.try_insert((lower, upper), liquidity).unwrap();
		}
		StabilizerPoolReviewFrame::<T>::put(StabilizerPoolFrameLiquidity {
			frame_id: reviewed_frame,
			liquidity_by_range,
		});
		StabilizerPoolPricesByFrame::<T>::mutate(|history| {
			for frame_id in 1..current_frame {
				history.try_insert(frame_id, reviewed_price).unwrap();
			}
		});
		let started_at = T::TimeProvider::now();
		StabilizerPoolFrameStartMillis::<T>::mutate(|starts| {
			for frame_id in 0..current_frame {
				let elapsed = current_frame.saturating_sub(frame_id).saturating_mul(60_000);
				starts.try_insert(frame_id, started_at.saturating_sub(elapsed)).unwrap();
			}
		});

		#[block]
		{
			Pallet::<T>::advance_liquidity_snapshots(current_frame, Some(price));
		}

		assert_ne!(
			FinalizedStabilizerPoolPrincipalByFrame::<T>::get()[&reviewed_frame],
			PositionAmounts::default()
		);
		assert!(!StabilizerPoolReviewFrame::<T>::get().unwrap().liquidity_by_range.is_empty());
		Ok(())
	}

	#[benchmark]
	fn report_position_change() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let config = config();
		let position = StabilizerPoolPositionId {
			position_manager: config.position_manager,
			token_id: U256::one(),
		};
		let frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		let reviewed_frame = frame_id.saturating_sub(1);
		let checkpoint = benchmark_checkpoint(caller);
		let liquidity = checkpoint.liquidity;
		PositionCheckpoints::<T>::insert(position.position_manager, position.token_id, &checkpoint);
		StabilizerPoolRegistrations::<T>::insert(
			position,
			StabilizerPoolRegistration::Enrolled { eligible_from: reviewed_frame },
		);
		RegisteredLiquidityByRange::<T>::insert(
			StabilizerPoolLiquidityRange::from(&checkpoint),
			StabilizerPoolRangeLiquidity {
				frame_id,
				current_liquidity: liquidity.into(),
				next_frame_liquidity: liquidity.into(),
			},
		);
		seed_review_frame::<T>(reviewed_frame, &checkpoint);
		let proof = position_change_evidence(200, 2, liquidity, true, (-100, 100));
		let observed_at = ProofBlock {
			hash: H256::from_low_u64_be(200),
			number: 200,
			timestamp_millis: 2_400_000,
		};

		// The Ethereum verifier's own benchmarks cover ancestry and authentication.
		#[block]
		{
			let Some(StabilizerPoolRegistration::Enrolled { eligible_from }) =
				StabilizerPoolRegistrations::<T>::get(position)
			else {
				return Err(BenchmarkError::Stop("fixture position must be enrolled"));
			};
			let previous =
				PositionCheckpoints::<T>::get(position.position_manager, position.token_id)
					.unwrap();
			evm::decode_position_change::<T>(position, &previous, &proof)?;
			Pallet::<T>::remove_reported_position(position, &previous, eligible_from, observed_at)?;
		}

		assert_eq!(
			StabilizerPoolRegistrations::<T>::get(position),
			Some(StabilizerPoolRegistration::Removed { reported_at: observed_at }),
		);
		assert!(!StabilizerPoolReviewFrame::<T>::get()
			.unwrap()
			.liquidity_by_range
			.contains_key(&(-100, 100)));
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, new_test_with_genesis::<Test>(|_| {}), Test);
}

fn benchmark_checkpoint<AccountId>(beneficiary: AccountId) -> PositionCheckpoint<AccountId> {
	let block =
		ProofBlock { hash: H256::from_low_u64_be(100), number: 100, timestamp_millis: 1_200_000 };
	PositionCheckpoint {
		beneficiary,
		pool_manager: pool_manager(),
		pool_id: pool_id(),
		subscriber: config().subscriber,
		revision: U256::one(),
		tick_lower: -100,
		tick_upper: 100,
		liquidity: u128::MAX,
		maintained_since: block,
		proven_through: block,
	}
}

// Measure correction and checkpoint with a full snapshot to account for decoding and rewriting it.
fn seed_review_frame<T: Config>(frame_id: FrameId, checkpoint: &PositionCheckpoint<T::AccountId>) {
	let mut liquidity_by_range = BoundedBTreeMap::new();
	let ticks = (checkpoint.tick_lower, checkpoint.tick_upper);
	liquidity_by_range.try_insert(ticks, checkpoint.liquidity.into()).unwrap();
	for offset in 1..MAX_REWARD_RANGES {
		let offset = i32::try_from(offset).unwrap();
		liquidity_by_range
			.try_insert((ticks.0 + offset, ticks.1 + offset), U256::one())
			.unwrap();
	}
	StabilizerPoolReviewFrame::<T>::put(StabilizerPoolFrameLiquidity {
		frame_id,
		liquidity_by_range,
	});
	StabilizerPoolFrameStartMillis::<T>::mutate(|starts| {
		starts.try_insert(frame_id, 1_200_000).unwrap();
		starts.try_insert(frame_id.saturating_add(1), 1_800_000).unwrap();
	});
	let pool_source = stabilizer_pool_price_source();
	let price = StabilizerPoolFramePrice {
		pool: pool_source.pool,
		argon_is_token0: pool_source.argon_is_token0,
		sqrt_price_x96: sqrt_price_at_uniswap_tick(0).unwrap(),
		argon_usd_target_price: FixedU128::one(),
		usdc_usd_price: FixedU128::one(),
	};
	StabilizerPoolPricesByFrame::<T>::mutate(|history| {
		history.try_insert(frame_id, price).unwrap()
	});
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
		argon_is_token0: pool_source.argon_is_token0,
	};
	let _ = Pallet::<T>::start_stabilizer_pool_price_sampling(frame, first_tick);
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
