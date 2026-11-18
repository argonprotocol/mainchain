use crate::{Config, PositionChangeProof, PositionProof};
use argon_primitives::{
	treasury::{
		StabilizerPoolPositionProviderWeightInfo, StabilizerPoolRewardProvider,
		StabilizerPoolRewardProviderWeightInfo, TreasuryPositionProvider,
		TreasuryPositionProviderWeightInfo,
	},
	EthereumVerifyProvider, EthereumVerifyProviderWeightInfo, MiningFrameTransitionProvider,
	MiningFrameTransitionProviderWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;

pub trait WeightInfo {
	fn register_pool() -> Weight;
	fn report_position_change() -> Weight;
	fn advance_liquidity_snapshots() -> Weight;
	fn checkpoint_position() -> Weight;
	fn settle_account(positions: u32) -> Weight;
	fn read_price_index_context() -> Weight;
	fn record_stabilizer_pool_spot_price() -> Weight;
	fn start_stabilizer_pool_price_sampling() -> Weight;
}

impl WeightInfo for () {
	fn settle_account(_: u32) -> Weight {
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

type FrameWeights<T> =
	<<T as Config>::MiningFrameTransitionProvider as MiningFrameTransitionProvider>::Weights;

pub fn checkpoint_position<T: Config>(proof: &PositionProof) -> Weight {
	type ProofWeights<T> = <<T as Config>::EthereumVerifier as EthereumVerifyProvider>::Weights;
	let header_count =
		proof.subscriber.execution_block_proof.target_to_anchor_header_chain.len() as u32;

	<T as Config>::WeightInfo::checkpoint_position()
		.saturating_add(<T::TreasuryPositionProvider as TreasuryPositionProvider<
			T::AccountId,
			Balance,
		>>::Weights::ensure_account())
		.saturating_add(FrameWeights::<T>::get_current_frame_id())
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(2, header_count))
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(2, header_count))
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(1, header_count))
}

pub(crate) fn report_position_change<T: Config>(proof: &PositionChangeProof) -> Weight {
	type ProofWeights<T> = <<T as Config>::EthereumVerifier as EthereumVerifyProvider>::Weights;
	let witness = match proof {
		PositionChangeProof::Revision(witness) | PositionChangeProof::Subscription(witness) =>
			witness,
	};
	let header_count = witness.execution_block_proof.target_to_anchor_header_chain.len() as u32;
	<T as Config>::WeightInfo::report_position_change()
		.saturating_add(FrameWeights::<T>::get_current_frame_id())
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(1, header_count))
}

type RewardWeights<T> = <<T as Config>::RewardProvider as StabilizerPoolRewardProvider>::Weights;

pub struct PositionProviderWeights<T>(PhantomData<T>);
impl<T: Config> StabilizerPoolPositionProviderWeightInfo for PositionProviderWeights<T> {
	fn settle_account() -> Weight {
		let positions = T::MaxPositionsPerAccount::get();
		let frames = T::ClaimWindowFrames::get();
		FrameWeights::<T>::get_current_frame_id()
			.saturating_add(<T as Config>::WeightInfo::settle_account(positions))
			.saturating_add(
				RewardWeights::<T>::frame_reward_rate()
					.saturating_mul(u64::from(frames).saturating_mul(positions.into())),
			)
			.saturating_add(RewardWeights::<T>::claim_rewards(frames))
	}
}

pub(crate) fn liquidity_snapshot_weight<T: Config>() -> Weight {
	<T as Config>::WeightInfo::advance_liquidity_snapshots()
		.saturating_add(RewardWeights::<T>::finalize_frame_rewards())
}
