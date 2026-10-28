use argon_primitives::{
	providers::{
		BlockRewardAccountsProviderWeightInfo, BlockSealerProvider, BlockSealerProviderWeightInfo,
		MiningSlotProviderWeightInfo, OperationalAccountProvider,
		OperationalAccountProviderWeightInfo, PriceProviderWeightInfo, TickProvider,
		TickProviderWeightInfo,
	},
	PriceProvider,
};
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	// Actual extrinsics
	fn bid() -> Weight;
	fn bid_legacy() -> Weight;
	fn auction_bookkeeping(f: u32) -> Weight;
	fn bid_renewal() -> Weight;
	fn bid_with_max() -> Weight;
	fn bid_with_max_renewal() -> Weight;
	fn bid_max() -> Weight;
	fn bid_max_renewal() -> Weight;
	fn settle_bids(b: u32, m: u32) -> Weight;
	fn settle_bids_with_replacements(b: u32, m: u32) -> Weight;
	fn configure_mining_slot_delay() -> Weight;

	// Hooks with variance
	fn on_finalize_record_block_author() -> Weight;
	fn on_finalize_grandpa_rotation() -> Weight;
	fn start_new_frame(m: u32) -> Weight;

	// Frame adjustment operations
	fn on_finalize_frame_adjustments() -> Weight;

	// Event handlers
	fn block_seal_read_vote() -> Weight;
	fn provider_has_active_rewards_account_seat() -> Weight;
	fn provider_get_block_rewards_account() -> Weight;
	fn provider_is_compute_block_eligible_for_rewards() -> Weight;
}

type SealerInfoProviderWeights<T> = <<T as crate::Config>::SealerInfo as BlockSealerProvider<
	<T as frame_system::Config>::AccountId,
	<T as crate::Config>::MiningAuthorityId,
>>::Weights;
type TickProviderWeights<T> = <<T as crate::Config>::TickProvider as TickProvider<
	<T as frame_system::Config>::Block,
>>::Weights;
type PriceProviderWeights<T> =
	<<T as crate::Config>::PriceProvider as PriceProvider<<T as crate::Config>::Balance>>::Weights;
type OperationalAccountProviderWeights<T> =
	<<T as crate::Config>::OperationalAccountProvider as OperationalAccountProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	SealerInfoWeight = SealerInfoProviderWeights<T>,
	TickProviderWeight = TickProviderWeights<T>,
	PriceProviderWeight = PriceProviderWeights<T>,
	OperationalAccountProviderWeight = OperationalAccountProviderWeights<T>,
>(
	PhantomData<(
		T,
		Base,
		SealerInfoWeight,
		TickProviderWeight,
		PriceProviderWeight,
		OperationalAccountProviderWeight,
	)>,
);
impl<
		T,
		Base,
		SealerInfoWeight,
		TickProviderWeight,
		PriceProviderWeight,
		OperationalAccountProviderWeight,
	> WeightInfo
	for WithProviderWeights<
		T,
		Base,
		SealerInfoWeight,
		TickProviderWeight,
		PriceProviderWeight,
		OperationalAccountProviderWeight,
	>
where
	T: crate::Config,
	Base: WeightInfo,
	SealerInfoWeight: BlockSealerProviderWeightInfo,
	TickProviderWeight: TickProviderWeightInfo,
	PriceProviderWeight: PriceProviderWeightInfo,
	OperationalAccountProviderWeight: OperationalAccountProviderWeightInfo,
{
	fn bid_with_max() -> Weight {
		Base::bid_with_max()
			.max(Base::bid_with_max_renewal())
			.saturating_add(OperationalAccountProviderWeight::is_eligible())
	}
	fn bid_max() -> Weight {
		Base::bid_max()
			.max(Base::bid_max_renewal())
			.saturating_add(OperationalAccountProviderWeight::is_eligible())
			.saturating_add(TickProviderWeight::current_tick())
	}
	fn settle_bids(b: u32, m: u32) -> Weight {
		// The bookkeeping benchmark already includes finalization of an empty batch.
		if b == 0 {
			return Weight::zero();
		}
		Base::settle_bids(b, m).max(Base::settle_bids_with_replacements(b, m))
	}

	fn settle_bids_with_replacements(b: u32, m: u32) -> Weight {
		Base::settle_bids_with_replacements(b, m)
	}

	fn bid_legacy() -> Weight {
		Base::bid_legacy()
	}
	fn auction_bookkeeping(f: u32) -> Weight {
		Base::auction_bookkeeping(f)
			// Bootstrap checks bidding start and derives frame one's deadline from ticks.
			.saturating_add(TickProviderWeight::current_tick().saturating_mul(2))
			.saturating_add(TickProviderWeight::elapsed_ticks().saturating_mul(2))
	}

	fn bid_renewal() -> Weight {
		Base::bid_renewal()
	}

	fn bid_with_max_renewal() -> Weight {
		Base::bid_with_max_renewal()
	}

	fn bid_max_renewal() -> Weight {
		Base::bid_max_renewal()
	}

	fn bid() -> Weight {
		Base::bid()
			.max(Base::bid_legacy())
			.max(Base::bid_renewal())
			.max(Base::bid_with_max())
			.max(Base::bid_with_max_renewal())
			.saturating_add(TickProviderWeight::current_tick())
			.saturating_add(OperationalAccountProviderWeight::is_eligible())
	}

	fn configure_mining_slot_delay() -> Weight {
		Base::configure_mining_slot_delay()
	}

	fn on_finalize_record_block_author() -> Weight {
		Base::on_finalize_record_block_author().saturating_add(SealerInfoWeight::get_sealer_info())
	}

	fn on_finalize_grandpa_rotation() -> Weight {
		Base::on_finalize_grandpa_rotation()
	}

	fn start_new_frame(m: u32) -> Weight {
		Base::start_new_frame(m).saturating_add(TickProviderWeight::current_tick())
	}

	fn on_finalize_frame_adjustments() -> Weight {
		Base::on_finalize_frame_adjustments()
			.saturating_add(PriceProviderWeight::get_average_microgons_per_argonot())
	}

	fn block_seal_read_vote() -> Weight {
		Base::block_seal_read_vote()
	}

	fn provider_has_active_rewards_account_seat() -> Weight {
		Base::provider_has_active_rewards_account_seat()
	}

	fn provider_get_block_rewards_account() -> Weight {
		Base::provider_get_block_rewards_account()
	}

	fn provider_is_compute_block_eligible_for_rewards() -> Weight {
		Base::provider_is_compute_block_eligible_for_rewards()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> MiningSlotProviderWeightInfo for ProviderWeightAdapter<T> {
	fn has_active_rewards_account_seat() -> Weight {
		T::WeightInfo::provider_has_active_rewards_account_seat()
	}
}

impl<T: crate::Config> BlockRewardAccountsProviderWeightInfo for ProviderWeightAdapter<T> {
	fn get_block_rewards_account() -> Weight {
		T::WeightInfo::provider_get_block_rewards_account()
	}

	fn is_compute_block_eligible_for_rewards() -> Weight {
		T::WeightInfo::provider_is_compute_block_eligible_for_rewards()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn bid_with_max() -> Weight {
		Weight::zero()
	}
	fn bid_max() -> Weight {
		Weight::zero()
	}
	fn settle_bids(_: u32, _: u32) -> Weight {
		Weight::zero()
	}

	fn settle_bids_with_replacements(_: u32, _: u32) -> Weight {
		Weight::zero()
	}

	fn bid_renewal() -> Weight {
		Weight::zero()
	}

	fn bid_with_max_renewal() -> Weight {
		Weight::zero()
	}

	fn bid_max_renewal() -> Weight {
		Weight::zero()
	}

	fn bid_legacy() -> Weight {
		Weight::zero()
	}
	fn auction_bookkeeping(_: u32) -> Weight {
		Weight::zero()
	}

	fn bid() -> Weight {
		Weight::zero()
	}

	fn configure_mining_slot_delay() -> Weight {
		Weight::zero()
	}

	fn on_finalize_grandpa_rotation() -> Weight {
		Weight::zero()
	}

	fn on_finalize_record_block_author() -> Weight {
		Weight::zero()
	}

	fn start_new_frame(_m: u32) -> Weight {
		Weight::zero()
	}

	fn on_finalize_frame_adjustments() -> Weight {
		Weight::zero()
	}

	fn block_seal_read_vote() -> Weight {
		Weight::zero()
	}

	fn provider_has_active_rewards_account_seat() -> Weight {
		Weight::zero()
	}

	fn provider_get_block_rewards_account() -> Weight {
		Weight::zero()
	}

	fn provider_is_compute_block_eligible_for_rewards() -> Weight {
		Weight::zero()
	}
}
