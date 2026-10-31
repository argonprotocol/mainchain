use crate::{Config, PositionProof};
use argon_primitives::{EthereumVerifyProvider, EthereumVerifyProviderWeightInfo};
use pallet_ethereum_verifier::Pallet as EthereumVerifier;
use pallet_prelude::*;

pub trait WeightInfo {
	fn register_pool() -> Weight;
	fn checkpoint_position() -> Weight;
	fn read_price_index_context() -> Weight;
	fn record_frame_price() -> Weight;
	fn record_frame_price_after_gap() -> Weight;
}

impl WeightInfo for () {
	fn register_pool() -> Weight {
		Weight::zero()
	}
	fn checkpoint_position() -> Weight {
		Weight::zero()
	}
	fn read_price_index_context() -> Weight {
		Weight::zero()
	}
	fn record_frame_price() -> Weight {
		Weight::zero()
	}
	fn record_frame_price_after_gap() -> Weight {
		Weight::zero()
	}
}

pub fn checkpoint_position<T: Config>(proof: &PositionProof) -> Weight {
	type ProofWeights<T> = <EthereumVerifier<T> as EthereumVerifyProvider>::Weights;
	let header_count =
		proof.subscriber.execution_block_proof.target_to_anchor_header_chain.len() as u32;

	<T as Config>::WeightInfo::checkpoint_position()
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(2, header_count))
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(2, header_count))
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(1, header_count))
		// Metadata verification repeats only ancestry. Charge a full provider base conservatively.
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(0, header_count))
}
