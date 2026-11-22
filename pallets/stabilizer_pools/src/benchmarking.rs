use super::*;
#[cfg(test)]
use crate::mock::Test;
use crate::{
	mock::{
		config, continuation_evidence, pool_id, pool_manager, position_change_evidence,
		position_evidence, stabilizer_pool_price_source,
	},
	price_history::StabilizerPoolUsdQuotes,
	uniswap::{sqrt_price_at_uniswap_tick, MAX_REWARD_RANGES, REWARD_BAND_TOLERANCE},
};
use frame_system::RawOrigin;
use pallet_prelude::benchmarking::{benchmark_mining_clock, set_benchmark_mining_clock};
use polkadot_sdk::frame_benchmarking::v2::*;

/// Ethereum verification has its own weights. The fixture encodes the block number in the anchor.
pub struct BenchmarkEthereumVerifier;

impl EthereumVerifyProvider for BenchmarkEthereumVerifier {
	type Weights = ();

	fn verify_storage_at_block<'a>(
		block: &EthereumExecutionBlockProof,
		_: impl IntoIterator<Item = (H160, EthereumAccountStorageWitnessRef<'a>)>,
	) -> Result<ProofBlock, argon_primitives::EthereumVerifyError> {
		let hash = block.anchor_block_hash;
		let number = U256::from_big_endian(hash.as_bytes()).low_u64();
		Ok(ProofBlock { hash, number, timestamp_millis: number * 12_000 })
	}

	fn verify_receipt_logs<B: Get<u32>, L: Get<u32>>(
		_: &argon_primitives::EthereumReceiptLogProofBatch<B, L>,
	) -> Result<(), argon_primitives::EthereumVerifyError> {
		Err(argon_primitives::EthereumVerifyError::VerifierUnavailable)
	}

	fn latest_execution_block_number() -> Option<u64> {
		None
	}
	fn latest_execution_block_timestamp() -> Option<Moment> {
		None
	}
}

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

	// Exercise both owners and both registered ranges.
	#[benchmark(pov_mode = Measured)]
	fn checkpoint_position() -> Result<(), BenchmarkError> {
		set_current_frame(FrameId::from(T::MaxSampledPriceHistoryFrames::get()));
		let caller: T::AccountId = whitelisted_caller();
		let encoded = caller.encode();
		if encoded.len() != 32 {
			return Err(BenchmarkError::Stop("checkpoint beneficiaries require AccountId32"));
		}
		let beneficiary = H256::from_slice(&encoded);

		Pallet::<T>::register_pool(RawOrigin::Root.into(), pool_manager(), config())?;
		let previous_owner: T::AccountId = account("previous-owner", 0, 0);
		let previous = benchmark_checkpoint(previous_owner);
		let mut empty = previous.clone();
		empty.liquidity = 0;
		for token_id in 2..=T::MaxPositionsPerAccount::get() {
			Pallet::<T>::record_checkpoint(
				config().position_manager,
				token_id.into(),
				empty.clone(),
			)?;
		}
		Pallet::<T>::record_checkpoint(config().position_manager, U256::one(), previous.clone())?;
		// Fill both owner lists, leaving one recipient slot for the transferred NFT.
		empty.beneficiary = caller.clone();
		for index in 1..T::MaxPositionsPerAccount::get() {
			let token_id = T::MaxPositionsPerAccount::get().saturating_add(index);
			Pallet::<T>::record_checkpoint(
				config().position_manager,
				token_id.into(),
				empty.clone(),
			)?;
		}
		// Exercise owner and range replacement after the prior registration became eligible.
		let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
		let range = StabilizerPoolLiquidityRange::from(&previous);
		let source = StabilizerPoolPositionId {
			position_manager: config().position_manager,
			uniswap_nft_id: U256::one(),
		};
		seed_active_handoff::<T>(source, &previous, current_frame);
		let new_range = StabilizerPoolLiquidityRange {
			lower_uniswap_tick: -110,
			upper_uniswap_tick: 110,
			..range
		};
		RegisteredLiquidityByRange::<T>::insert(
			new_range,
			StabilizerPoolRangeLiquidity {
				frame_id: current_frame,
				current_liquidity: U256::one(),
				next_frame_liquidity: U256::one(),
			},
		);

		let proof = position_evidence(200, 2, beneficiary, u128::MAX, true, (-110, 110));
		#[extrinsic_call]
		_(RawOrigin::Signed(caller), pool_manager(), pool_id(), U256::one(), Box::new(proof));

		let checkpoint =
			PositionCheckpoints::<T>::get(config().position_manager, U256::one()).unwrap();
		assert_eq!(checkpoint.maintained_since_block.number, 200);
		assert_eq!(checkpoint.proven_through_block.number, 200);
		Ok(())
	}

	#[benchmark(pov_mode = Measured)]
	fn continue_enrollment() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let (source, proof) = prepare_continuation::<T>(&caller, false)?;
		#[extrinsic_call]
		_(RawOrigin::Signed(caller), source, U256::from(2), proof);
		assert_eq!(PendingBandChanges::<T>::get().len(), MAX_REWARD_RANGES as usize);
		Ok(())
	}

	#[benchmark(pov_mode = Measured)]
	fn continue_reported_enrollment() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let (source, proof) = prepare_continuation::<T>(&caller, true)?;
		#[extrinsic_call]
		continue_enrollment(RawOrigin::Signed(caller), source, U256::from(2), proof);
		assert_eq!(PendingBandChanges::<T>::get().len(), MAX_REWARD_RANGES as usize);
		Ok(())
	}

	#[benchmark]
	fn prove_position_earnings() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let position = setup_earning_position::<T>(caller, 1)?;

		#[block]
		{
			Pallet::<T>::prove_position_earnings(position)?;
		}
		let report = PositionContinuations::<T>::get(position).reported_enrollment.unwrap();
		assert_eq!(
			report.enrollment.next_claim_frame_id,
			FrameId::from(T::MaxSampledPriceHistoryFrames::get()) + 1
		);
		Ok(())
	}

	#[benchmark]
	fn settle_account(n: Linear<0, 5>) -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		for token_id in 1..=n {
			setup_earning_position::<T>(caller.clone(), token_id)?;
		}

		#[block]
		{
			Pallet::<T>::settle_account(&caller)?;
		}
		assert!(PositionIdsByAccount::<T>::get(&caller).is_empty());
		Ok(())
	}

	#[benchmark]
	fn advance_liquidity_snapshots() -> Result<(), BenchmarkError> {
		let mut config = config();
		config.key.tick_spacing = 1;
		let pool_id = evm::pool_id(&config.key);
		Pallet::<T>::register_pool(RawOrigin::Root.into(), pool_manager(), config)?;
		let mut pool = stabilizer_pool_price_source().pool;
		pool.pool_id = pool_id;
		let price = StabilizerPoolFramePrice {
			pool,
			sqrt_price_x96: sqrt_price_at_uniswap_tick(277_000).unwrap(),
			// Place both band edges across the maximum number of spacing-one intervals.
			argon_usd_target_price: FixedU128::from_rational(1_000_018, 1_000_000),
			usdc_usd_price: FixedU128::one(),
			argon_is_token0: false,
		};
		let retained_frames = FrameId::from(T::MaxSampledPriceHistoryFrames::get());
		let current_frame = retained_frames.saturating_add(1);
		let closed_frame_id = current_frame - 1;
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
		let ranges = band.reward_ranges().unwrap();
		assert_eq!(ranges.len(), 402);
		let mut pending = PendingBandLiquidity::new();
		for index in 0..MAX_REWARD_RANGES as usize {
			let (lower, upper) = ranges[index % ranges.len()];
			let replacement = ranges[(index + 1 + index / ranges.len()) % ranges.len()];
			let range = StabilizerPoolLiquidityRange {
				pool_manager: pool.pool_manager,
				pool_id,
				lower_uniswap_tick: lower,
				upper_uniswap_tick: upper,
			};
			pending
				.try_insert(PendingBandChange::new(range, replacement), liquidity)
				.unwrap();
		}
		PendingBandChanges::<T>::put(pending);
		for (lower, upper) in ranges {
			RegisteredLiquidityByRange::<T>::insert(
				StabilizerPoolLiquidityRange {
					pool_manager: pool.pool_manager,
					pool_id,
					lower_uniswap_tick: lower,
					upper_uniswap_tick: upper,
				},
				StabilizerPoolRangeLiquidity {
					frame_id: closed_frame_id,
					current_liquidity: liquidity,
					next_frame_liquidity: liquidity,
				},
			);
		}
		PendingFrameRewardValue::<T>::put(ClosedFrameRewardValue {
			frame_id: closed_frame_id - 1,
			eligible_value_microgons: 1,
		});
		StabilizerPoolRewardSharesByFrame::<T>::mutate(|history| {
			for frame_id in 0..closed_frame_id {
				history.try_insert(frame_id, T::UsdcRewardShare::get()).unwrap();
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

		assert_eq!(
			StabilizerPoolRewardSharesByFrame::<T>::get()[&closed_frame_id],
			T::UsdcRewardShare::get()
		);
		assert!(PendingBandChanges::<T>::get().is_empty());
		let frozen = PendingFrameRewardValue::<T>::get().unwrap();
		assert_eq!(frozen.frame_id, closed_frame_id);
		assert!(frozen.eligible_value_microgons > 0);
		Ok(())
	}

	#[benchmark(pov_mode = Measured)]
	fn report_position_change() -> Result<(), BenchmarkError> {
		set_current_frame(FrameId::from(T::MaxSampledPriceHistoryFrames::get()));
		let caller: T::AccountId = whitelisted_caller();
		let config = config();
		Pallet::<T>::register_pool(RawOrigin::Root.into(), pool_manager(), config)?;
		let position = StabilizerPoolPositionId {
			position_manager: config.position_manager,
			uniswap_nft_id: U256::one(),
		};
		let frame_id = T::MiningFrameTransitionProvider::get_current_frame_id();
		let checkpoint = benchmark_checkpoint(caller);
		let mut empty = checkpoint.clone();
		empty.liquidity = 0;
		for token_id in 2..=T::MaxPositionsPerAccount::get() {
			Pallet::<T>::record_checkpoint(
				config.position_manager,
				token_id.into(),
				empty.clone(),
			)?;
		}
		PositionIdsByAccount::<T>::mutate(&checkpoint.beneficiary, |positions| {
			positions.try_push(position).unwrap();
		});
		let liquidity = checkpoint.liquidity;
		PositionCheckpoints::<T>::insert(
			position.position_manager,
			position.uniswap_nft_id,
			&checkpoint,
		);
		StabilizerPoolRegistrations::<T>::insert(
			position,
			StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
				first_eligible_frame_id: frame_id + 1,
				next_claim_frame_id: frame_id.saturating_sub(1),
			}),
		);
		RegisteredLiquidityByRange::<T>::insert(
			StabilizerPoolLiquidityRange::from(&checkpoint),
			StabilizerPoolRangeLiquidity {
				frame_id,
				current_liquidity: U256::zero(),
				next_frame_liquidity: liquidity.into(),
			},
		);
		seed_active_handoff::<T>(position, &checkpoint, frame_id);
		let proof = position_change_evidence(
			200,
			2,
			liquidity,
			true,
			(checkpoint.tick_lower, checkpoint.tick_upper),
		);
		let observed_at = ProofBlock {
			hash: U256::from(200).to_big_endian().into(),
			number: 200,
			timestamp_millis: 2_400_000,
		};

		#[extrinsic_call]
		_(RawOrigin::Signed(whitelisted_caller()), position, Box::new(proof));

		assert_eq!(
			StabilizerPoolRegistrations::<T>::get(position),
			Some(StabilizerPoolRegistration::Removed { reported_at_block: observed_at }),
		);
		Ok(())
	}

	impl_benchmark_test_suite!(
		Pallet,
		{
			use sp_runtime::BuildStorage;
			polkadot_sdk::sp_io::TestExternalities::new(
				crate::mock::RuntimeGenesisConfig::default().build_storage().unwrap(),
			)
		},
		Test
	);
}

fn benchmark_checkpoint<AccountId>(beneficiary: AccountId) -> PositionCheckpoint<AccountId> {
	let block = ProofBlock {
		hash: U256::from(100).to_big_endian().into(),
		number: 100,
		timestamp_millis: 1_200_000,
	};
	PositionCheckpoint {
		beneficiary,
		pool_manager: pool_manager(),
		pool_id: pool_id(),
		subscriber: config().subscriber,
		revision: U256::one(),
		tick_lower: -277_380,
		tick_upper: -276_330,
		liquidity: u128::MAX,
		maintained_since_block: block,
		proven_through_block: block,
	}
}

fn set_current_frame(frame_id: FrameId) {
	let mut frame_state = benchmark_mining_clock();
	frame_state.current_frame_id = frame_id;
	set_benchmark_mining_clock(frame_state);
	#[cfg(test)]
	crate::mock::CurrentFrame::set(frame_id);
}

fn prepare_continuation<T: Config>(
	caller: &T::AccountId,
	report_first: bool,
) -> Result<(StabilizerPoolPositionId, Box<PositionContinuationProof>), BenchmarkError> {
	let frame = FrameId::from(T::MaxSampledPriceHistoryFrames::get()) + 1;
	set_current_frame(frame);
	let config = config();
	Pallet::<T>::register_pool(RawOrigin::Root.into(), pool_manager(), config)?;
	let source = StabilizerPoolPositionId {
		position_manager: config.position_manager,
		uniswap_nft_id: U256::one(),
	};
	let replacement = StabilizerPoolPositionId { uniswap_nft_id: U256::from(2), ..source };
	let mut before = benchmark_checkpoint(caller.clone());
	before.liquidity /= 2;
	Pallet::<T>::record_checkpoint(source.position_manager, source.uniswap_nft_id, before.clone())?;
	let range = StabilizerPoolLiquidityRange::from(&before);
	RegisteredLiquidityByRange::<T>::mutate(range, |registered| {
		registered.current_liquidity = registered.next_frame_liquidity;
	});
	StabilizerPoolRegistrations::<T>::insert(
		source,
		StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
			first_eligible_frame_id: 1,
			next_claim_frame_id: 1,
		}),
	);
	seed_position_history::<T>(source, &before, frame);
	StabilizerPoolFrameStartMillis::<T>::mutate(|starts| {
		for id in 1..=frame {
			let start = 1_800_000u64.saturating_sub((frame - id) * 60_000);
			starts.try_insert(id, start).unwrap();
		}
	});
	before.proven_through_block.number = 200;
	let mut after = before.clone();
	after.proven_through_block.number += 1;
	after.tick_lower += config.key.tick_spacing;
	after.liquidity += 1;
	let proof =
		continuation_evidence(source.uniswap_nft_id, &before, replacement.uniswap_nft_id, &after);

	// The production runtime reads Timestamp::Now; the test runtime supplies its own clock.
	let now = after.proven_through_block.number * 12_000;
	frame_support::storage::unhashed::put(
		&frame_support::storage::storage_prefix(b"Timestamp", b"Now"),
		&now,
	);
	#[cfg(test)]
	crate::mock::CurrentTime::set(now);
	seed_pending_changes::<T>(range, None);
	PositionIdsByAccount::<T>::mutate(caller, |positions| {
		for id in 0..T::MaxPositionsPerAccount::get() - 1 {
			positions
				.try_insert(
					0,
					StabilizerPoolPositionId { uniswap_nft_id: (10 + id).into(), ..source },
				)
				.unwrap();
		}
	});
	if report_first {
		let Some(StabilizerPoolRegistration::Enrolled(enrollment)) =
			StabilizerPoolRegistrations::<T>::get(source)
		else {
			return Err(BenchmarkError::Stop("source must be enrolled"));
		};
		let checkpoint =
			PositionCheckpoints::<T>::get(source.position_manager, source.uniswap_nft_id).unwrap();
		Pallet::<T>::remove_reported_position(
			source,
			&checkpoint,
			enrollment,
			ProofBlock {
				hash: U256::from(202).to_big_endian().into(),
				number: 202,
				timestamp_millis: 202 * 12_000,
			},
		)?;
	}

	Ok((source, Box::new(proof)))
}

// A changed-band handoff keeps its capped contribution pending until the sampled price is known.
fn seed_pending_changes<T: Config>(
	range: StabilizerPoolLiquidityRange,
	existing: Option<(TickRange, U256)>,
) {
	let mut pending = PendingBandLiquidity::new();
	let spacing = config().key.tick_spacing;
	for offset in 1..MAX_REWARD_RANGES {
		let ticks = (range.lower_uniswap_tick, range.upper_uniswap_tick + offset as i32 * spacing);
		pending.try_insert(PendingBandChange::new(range, ticks), U256::one()).unwrap();
	}
	if let Some((ticks, amount)) = existing {
		pending.try_insert(PendingBandChange::new(range, ticks), amount).unwrap();
	}
	PendingBandChanges::<T>::put(pending);
}

// Seed the capped contribution that removal must deduct from the open frame.
fn seed_active_handoff<T: Config>(
	source: StabilizerPoolPositionId,
	head: &PositionCheckpoint<T::AccountId>,
	frame: FrameId,
) {
	let retired = seed_position_history::<T>(source, head, frame + 1);
	seed_pending_changes::<T>(
		StabilizerPoolLiquidityRange::from(&retired),
		Some(((head.tick_lower, head.tick_upper), head.liquidity.into())),
	);
}

fn seed_position_history<T: Config>(
	position: StabilizerPoolPositionId,
	head: &PositionCheckpoint<T::AccountId>,
	replacement_start_frame_id: FrameId,
) -> PositionCheckpoint<T::AccountId> {
	let mut continuation = PositionContinuation::<T::MaxSampledPriceHistoryFrames>::default();
	let count = FrameId::from(T::MaxSampledPriceHistoryFrames::get());
	let mut old = head.clone();
	for end in replacement_start_frame_id.saturating_sub(count - 1)..=replacement_start_frame_id {
		let retired = StabilizerPoolPositionId { uniswap_nft_id: (100 + end).into(), ..position };
		let shift = config().key.tick_spacing * (1 + (end % 2) as i32);
		old.tick_lower = head.tick_lower + shift;
		old.tick_upper = head.tick_upper + shift;
		PositionCheckpoints::<T>::insert(retired.position_manager, retired.uniswap_nft_id, &old);
		continuation
			.earning_periods
			.try_push(PositionEarningPeriod {
				retired_position_id: retired,
				first_eligible_frame_id: end.saturating_sub(1),
				replacement_start_frame_id: end,
				excluded_frame_id: None,
			})
			.unwrap();
	}
	PositionContinuations::<T>::insert(position, continuation);
	old
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

/// Seed the same worst-case finalized history for one-position proofs and account settlement.
fn setup_earning_position<T: Config>(
	caller: T::AccountId,
	token_id: u32,
) -> Result<StabilizerPoolPositionId, BenchmarkError> {
	let retained_frames = FrameId::from(T::MaxSampledPriceHistoryFrames::get());
	let current_frame = retained_frames + 2;
	let first_frame = current_frame - FrameId::from(T::ClaimWindowFrames::get()) - 1;
	let mut checkpoint = benchmark_checkpoint(caller.clone());
	let config = config();
	if !Pools::<T>::contains_key(pool_manager(), pool_id()) {
		Pallet::<T>::register_pool(RawOrigin::Root.into(), pool_manager(), config)?;
	}

	let price = StabilizerPoolFramePrice {
		pool: stabilizer_pool_price_source().pool,
		sqrt_price_x96: sqrt_price_at_uniswap_tick(-276_800).unwrap(),
		argon_usd_target_price: FixedU128::one(),
		usdc_usd_price: FixedU128::one(),
		argon_is_token0: true,
	};
	let band = StabilizerPoolRewardBand::new(
		price.argon_usd_target_price,
		price.usdc_usd_price,
		price.argon_is_token0,
		config.key.tick_spacing,
		T::RewardPriceBand::get(),
		REWARD_BAND_TOLERANCE,
	)
	.unwrap();
	let ranges = band.reward_ranges().unwrap();
	(checkpoint.tick_lower, checkpoint.tick_upper) = ranges[0];
	checkpoint.maintained_since_block.timestamp_millis = 0;
	checkpoint.proven_through_block.timestamp_millis = current_frame.saturating_mul(60_000);

	StabilizerPoolFrameStartMillis::<T>::mutate(|starts| {
		for frame in current_frame - retained_frames..=current_frame {
			starts.try_insert(frame, frame.saturating_mul(60_000)).unwrap();
		}
	});
	StabilizerPoolPricesByFrame::<T>::mutate(|prices| {
		for frame in current_frame - retained_frames..current_frame {
			prices.try_insert(frame, price).unwrap();
		}
	});
	StabilizerPoolRewardSharesByFrame::<T>::mutate(|history| {
		for frame in current_frame - retained_frames..current_frame {
			history.try_insert(frame, T::UsdcRewardShare::get()).unwrap();
			T::RewardProvider::finalize_frame_rewards(frame, 1);
		}
	});
	set_current_frame(current_frame);
	Pallet::<T>::record_checkpoint(config.position_manager, token_id.into(), checkpoint.clone())?;
	let position = StabilizerPoolPositionId {
		position_manager: config.position_manager,
		uniswap_nft_id: token_id.into(),
	};
	let mut continuation = PositionContinuation::<T::MaxSampledPriceHistoryFrames>::default();
	for index in 0..retained_frames {
		let retired = StabilizerPoolPositionId {
			uniswap_nft_id: (100 + u64::from(token_id) * retained_frames + index).into(),
			..position
		};
		let mut old = checkpoint.clone();
		(old.tick_lower, old.tick_upper) = ranges[1 + index as usize % (ranges.len() - 1)];
		PositionCheckpoints::<T>::insert(retired.position_manager, retired.uniswap_nft_id, old);
		continuation
			.earning_periods
			.try_push(PositionEarningPeriod {
				retired_position_id: retired,
				first_eligible_frame_id: 0,
				replacement_start_frame_id: index + 2,
				excluded_frame_id: None,
			})
			.unwrap();
	}
	continuation.reported_enrollment = Some(ReportedPositionEnrollment {
		enrollment: StabilizerPoolEnrollment {
			first_eligible_frame_id: first_frame,
			next_claim_frame_id: first_frame,
		},
		recovery_deadline_frame_id: current_frame,
		reported_frame_id: current_frame.saturating_sub(1),
	});
	PositionContinuations::<T>::insert(position, continuation);
	StabilizerPoolRegistrations::<T>::insert(
		position,
		StabilizerPoolRegistration::Removed { reported_at_block: checkpoint.proven_through_block },
	);

	let continuation = PositionContinuations::<T>::get(position);
	let enrollment = continuation.reported_enrollment.unwrap().enrollment;
	let (_, claims) = Pallet::<T>::calculate_position_earnings(
		&checkpoint,
		&enrollment,
		&continuation,
		current_frame,
	)?;
	assert_eq!(claims.len(), T::ClaimWindowFrames::get() as usize);
	Ok(position)
}
