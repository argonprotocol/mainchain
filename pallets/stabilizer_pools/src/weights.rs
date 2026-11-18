use crate::{uniswap::MAX_REWARD_RANGES, Config, PositionChangeProof, PositionProof};
use argon_primitives::{
	treasury::{StabilizerPoolClaimProvider, StabilizerPoolClaimProviderWeightInfo},
	EthereumVerifyProvider, EthereumVerifyProviderWeightInfo, MiningFrameTransitionProvider,
	MiningFrameTransitionProviderWeightInfo,
};
use pallet_prelude::*;

pub trait WeightInfo {
	fn register_pool() -> Weight;
	fn report_position_change() -> Weight;
	fn advance_liquidity_snapshots() -> Weight;
	fn checkpoint_position() -> Weight;
	fn value_position_claim() -> Weight;
	fn read_price_index_context() -> Weight;
	fn record_stabilizer_pool_spot_price() -> Weight;
	fn start_stabilizer_pool_price_sampling() -> Weight;
}

impl WeightInfo for () {
	fn value_position_claim() -> Weight {
		Weight::zero()
	}
	fn advance_liquidity_snapshots() -> Weight {
		Weight::zero()
	}
	fn report_position_change() -> Weight {
		Weight::zero()
	}
	fn register_pool() -> Weight {
		Weight::zero()
	}
	fn checkpoint_position() -> Weight {
		Weight::zero()
	}
	fn read_price_index_context() -> Weight {
		Weight::zero()
	}
	fn record_stabilizer_pool_spot_price() -> Weight {
		Weight::zero()
	}
	fn start_stabilizer_pool_price_sampling() -> Weight {
		Weight::zero()
	}
}

pub fn checkpoint_position<T: Config>(proof: &PositionProof) -> Weight {
	type ProofWeights<T> = <<T as Config>::EthereumVerifier as EthereumVerifyProvider>::Weights;
	type FrameWeights<T> =
		<<T as Config>::MiningFrameTransitionProvider as MiningFrameTransitionProvider>::Weights;
	let header_count =
		proof.subscriber.execution_block_proof.target_to_anchor_header_chain.len() as u32;

	<T as Config>::WeightInfo::checkpoint_position()
		// A replacement can also claim frames from the previous accepted proof.
		.saturating_add(position_claim_weight::<T>())
		.saturating_add(T::DbWeight::get().reads_writes(6, 4))
		.saturating_add(FrameWeights::<T>::get_current_frame_id())
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(2, header_count))
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(2, header_count))
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(1, header_count))
}

pub(crate) fn report_position_change<T: Config>(proof: &PositionChangeProof) -> Weight {
	type ProofWeights<T> = <<T as Config>::EthereumVerifier as EthereumVerifyProvider>::Weights;
	type FrameWeights<T> =
		<<T as Config>::MiningFrameTransitionProvider as MiningFrameTransitionProvider>::Weights;
	let witness = match proof {
		PositionChangeProof::Revision(witness) | PositionChangeProof::Subscription(witness) =>
			witness,
	};
	let header_count = witness.execution_block_proof.target_to_anchor_header_chain.len() as u32;
	<T as Config>::WeightInfo::report_position_change()
		.saturating_add(position_claim_weight::<T>())
		.saturating_add(FrameWeights::<T>::get_current_frame_id())
		.saturating_add(T::DbWeight::get().reads_writes(6, 3))
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(1, header_count))
}

fn position_claim_weight<T: Config>() -> Weight {
	if !T::ClaimProvider::ENABLED {
		return Weight::zero();
	}
	type ClaimWeights<T> = <<T as Config>::ClaimProvider as StabilizerPoolClaimProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;

	<T as Config>::WeightInfo::value_position_claim()
		.saturating_add(T::DbWeight::get().reads(3))
		.saturating_add(ClaimWeights::<T>::submit_claim(T::ClaimWindowFrames::get()))
}

pub(crate) fn liquidity_snapshot_weight<T: Config>() -> Weight {
	<T as Config>::WeightInfo::advance_liquidity_snapshots()
		.saturating_add(T::DbWeight::get().reads_writes(u64::from(MAX_REWARD_RANGES) + 7, 3))
}
