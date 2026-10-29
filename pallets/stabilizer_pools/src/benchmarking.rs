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
