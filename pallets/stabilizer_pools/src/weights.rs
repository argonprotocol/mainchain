use crate::{Config, PositionChangeProof, PositionContinuationProof, PositionProof};
use argon_primitives::{
	treasury::{
		EarningsSettlementProviderWeightInfo, StabilizerPoolRewardProvider,
		StabilizerPoolRewardProviderWeightInfo, TreasuryPositionProvider,
		TreasuryPositionProviderWeightInfo,
	},
	EthereumExecutionBlockProof, EthereumVerifyProvider, EthereumVerifyProviderWeightInfo,
	MiningFrameTransitionProvider, MiningFrameTransitionProviderWeightInfo, PriceProvider,
	PriceProviderWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;

pub trait WeightInfo {
	fn register_pool() -> Weight;
	fn report_position_change() -> Weight;
	fn advance_liquidity_snapshots() -> Weight;
	fn checkpoint_position() -> Weight;
	fn continue_enrollment() -> Weight;
	fn continue_reported_enrollment() -> Weight;
	fn prove_position_earnings() -> Weight;
	fn settle_account(positions: u32) -> Weight;
	fn read_price_index_context() -> Weight;
	fn record_stabilizer_pool_spot_price() -> Weight;
	fn start_stabilizer_pool_price_sampling() -> Weight;
}

impl WeightInfo for () {
	fn prove_position_earnings() -> Weight {
		Weight::zero()
	}
	fn continue_reported_enrollment() -> Weight {
		Weight::zero()
	}
	fn continue_enrollment() -> Weight {
		Weight::zero()
	}
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
	<T as Config>::WeightInfo::checkpoint_position()
		.saturating_add(prove_position_earnings::<T>().saturating_mul(2))
		.saturating_add(<T::TreasuryPositionProvider as TreasuryPositionProvider<
			T::AccountId,
			Balance,
		>>::Weights::ensure_account())
		.saturating_add(FrameWeights::<T>::get_current_frame_id())
		// Covers the two current-price lookups for the market and target quotes.
		.saturating_add(
			<T::PriceProvider as PriceProvider<Balance>>::Weights::get_redemption_prices(),
		)
		.saturating_add(position_proof_weight::<T>(&proof.subscriber.execution_block_proof))
}

pub(crate) fn continue_enrollment<T: Config>(proof: &PositionContinuationProof) -> Weight {
	type ProofWeights<T> = <<T as Config>::EthereumVerifier as EthereumVerifyProvider>::Weights;
	<T as Config>::WeightInfo::continue_enrollment()
		.max(<T as Config>::WeightInfo::continue_reported_enrollment())
		.saturating_add(prove_position_earnings::<T>())
		// Covers the two current-price lookups for the market and target quotes.
		.saturating_add(
			<T::PriceProvider as PriceProvider<Balance>>::Weights::get_redemption_prices(),
		)
		.saturating_add(FrameWeights::<T>::get_current_frame_id())
		.saturating_add(position_proof_weight::<T>(&proof.before.execution_block_proof))
		.saturating_add(position_proof_weight::<T>(&proof.after.execution_block_proof))
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(1, 0))
}

pub(crate) fn report_position_change<T: Config>(proof: &PositionChangeProof) -> Weight {
	type ProofWeights<T> = <<T as Config>::EthereumVerifier as EthereumVerifyProvider>::Weights;
	let witness = match proof {
		PositionChangeProof::Revision(witness) | PositionChangeProof::Subscription(witness) =>
			witness,
	};
	let header_count = witness.execution_block_proof.target_to_anchor_header_chain.len() as u32;
	<T as Config>::WeightInfo::report_position_change()
		.saturating_add(prove_position_earnings::<T>())
		.saturating_add(FrameWeights::<T>::get_current_frame_id())
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(1, header_count))
}

type RewardWeights<T> = <<T as Config>::RewardProvider as StabilizerPoolRewardProvider>::Weights;

pub struct PositionProviderWeights<T>(PhantomData<T>);
impl<T: Config> EarningsSettlementProviderWeightInfo for PositionProviderWeights<T> {
	fn settle_account() -> Weight {
		let positions = T::MaxPositionsPerAccount::get();
		<T as Config>::WeightInfo::settle_account(positions)
			.saturating_add(FrameWeights::<T>::get_current_frame_id())
			.saturating_add(earnings_provider_weight::<T>().saturating_mul(positions.into()))
	}
}

pub(crate) fn prove_position_earnings<T: Config>() -> Weight {
	<T as Config>::WeightInfo::prove_position_earnings()
		.saturating_add(earnings_provider_weight::<T>())
}

fn earnings_provider_weight<T: Config>() -> Weight {
	let frames = T::ClaimWindowFrames::get();
	FrameWeights::<T>::get_current_frame_id()
		.saturating_add(RewardWeights::<T>::frame_reward_rate().saturating_mul(frames.into()))
		.saturating_add(RewardWeights::<T>::claim_rewards(frames))
		.saturating_add(<T::TreasuryPositionProvider as TreasuryPositionProvider<
			T::AccountId,
			Balance,
		>>::Weights::accrue())
}

pub(crate) fn liquidity_snapshot_weight<T: Config>() -> Weight {
	<T as Config>::WeightInfo::advance_liquidity_snapshots()
		.saturating_add(RewardWeights::<T>::finalize_frame_rewards())
}

fn position_proof_weight<T: Config>(block: &EthereumExecutionBlockProof) -> Weight {
	type ProofWeights<T> = <<T as Config>::EthereumVerifier as EthereumVerifyProvider>::Weights;
	let headers = block.target_to_anchor_header_chain.len() as u32;
	ProofWeights::<T>::verify_account_storage_proof(2, headers)
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(2, 0))
		.saturating_add(ProofWeights::<T>::verify_account_storage_proof(1, 0))
}
