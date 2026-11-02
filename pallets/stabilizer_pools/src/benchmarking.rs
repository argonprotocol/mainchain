use super::*;
#[cfg(test)]
use crate::mock::Test;
use crate::mock::{config, evidence, pool_id, pool_manager};
use frame_system::RawOrigin;
use polkadot_sdk::frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn read_price_index_context() -> Result<(), BenchmarkError> {
		if T::MiningFrameTransitionProvider::is_new_frame_started().is_some() {
			return Err(BenchmarkError::Stop("measure the fallback frame lookup"));
		}

		#[block]
		{
			let tick = T::CurrentTick::get();
			let frame = T::MiningFrameTransitionProvider::is_new_frame_started()
				.unwrap_or_else(T::MiningFrameTransitionProvider::get_current_frame_id);
			core::hint::black_box((tick, frame));
		}
		Ok(())
	}

	#[benchmark]
	fn record_frame_price() -> Result<(), BenchmarkError> {
		let (retained_frames, index) = full_frame_history::<T>()?;
		let frame_id = retained_frames + 1;

		#[block]
		{
			Pallet::<T>::record_frame_price(frame_id, &index);
		}

		let history = FrameStartPriceIndices::<T>::get();
		assert_eq!(history.len(), retained_frames as usize);
		assert!(!history.contains_key(&1));
		assert_eq!(history.get(&frame_id), Some(&index));
		Ok(())
	}

	#[benchmark]
	fn record_frame_price_after_gap() -> Result<(), BenchmarkError> {
		let (retained_frames, index) = full_frame_history::<T>()?;
		let frame_id = retained_frames * 2 + 1;

		#[block]
		{
			Pallet::<T>::record_frame_price(frame_id, &index);
		}

		let history = FrameStartPriceIndices::<T>::get();
		assert_eq!(history.len(), 1);
		assert_eq!(history.get(&frame_id), Some(&index));
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

		let proof = Box::new(evidence::<T>(200, 1, beneficiary, u128::MAX, true));

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
		assert_eq!(checkpoint.maintained_since.number, 100);
		assert_eq!(checkpoint.proven_through.number, 200);
		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, new_test_with_genesis::<Test>(|_| {}), Test);
}

fn full_frame_history<T: Config>() -> Result<(FrameId, PriceIndex), BenchmarkError> {
	let retained_frames = u64::from(T::MaxFramePriceHistoryFrames::get());
	if retained_frames == 0 {
		return Err(BenchmarkError::Stop("frame history is disabled"));
	}

	let index = PriceIndex {
		btc_usd_price: FixedU128::from_inner(u128::MAX),
		argonot_usd_price: FixedU128::from_inner(u128::MAX),
		argon_usd_price: FixedU128::from_inner(u128::MAX),
		argon_usd_target_price: FixedU128::from_inner(u128::MAX),
		argon_time_weighted_average_liquidity: u128::MAX,
		tick: Tick::MAX,
		usdc_usd_price: Some(FixedU128::from_inner(u128::MAX)),
		stabilizer_pool_sqrt_price_x96: Some(U256::one() << 96),
	};

	let mut history = BoundedBTreeMap::<FrameId, PriceIndex, T::MaxFramePriceHistoryFrames>::new();
	for frame in 1..=retained_frames {
		let observation = PriceIndex { tick: Tick::MAX - (retained_frames - frame), ..index };
		history
			.try_insert(frame, observation)
			.map_err(|_| BenchmarkError::Stop("frame history benchmark overflow"))?;
	}

	FrameStartPriceIndices::<T>::put(history);
	Ok((retained_frames, index))
}
