use argon_primitives::providers::{
	BlockSealerProvider, BlockSealerProviderWeightInfo, MiningSlotProviderWeightInfo, TickProvider,
	TickProviderWeightInfo,
};
use core::marker::PhantomData;
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	// Actual extrinsics
	fn bid() -> Weight;
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
}

type SealerInfoProviderWeights<T> = <<T as crate::Config>::SealerInfo as BlockSealerProvider<
	<T as frame_system::Config>::AccountId,
	<T as crate::Config>::MiningAuthorityId,
>>::Weights;
type TickProviderWeights<T> = <<T as crate::Config>::TickProvider as TickProvider<
	<T as frame_system::Config>::Block,
>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	SealerInfoWeight = SealerInfoProviderWeights<T>,
	TickProviderWeight = TickProviderWeights<T>,
>(PhantomData<(T, Base, SealerInfoWeight, TickProviderWeight)>);
impl<T, Base, SealerInfoWeight, TickProviderWeight> WeightInfo
	for WithProviderWeights<T, Base, SealerInfoWeight, TickProviderWeight>
where
	T: crate::Config,
	Base: WeightInfo,
	SealerInfoWeight: BlockSealerProviderWeightInfo,
	TickProviderWeight: TickProviderWeightInfo,
{
	fn bid() -> Weight {
		Base::bid().saturating_add(TickProviderWeight::current_tick())
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
	}

	fn block_seal_read_vote() -> Weight {
		Base::block_seal_read_vote()
	}

	fn provider_has_active_rewards_account_seat() -> Weight {
		Base::provider_has_active_rewards_account_seat()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> MiningSlotProviderWeightInfo for ProviderWeightAdapter<T> {
	fn has_active_rewards_account_seat() -> Weight {
		<T as crate::Config>::WeightInfo::provider_has_active_rewards_account_seat()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
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
}
