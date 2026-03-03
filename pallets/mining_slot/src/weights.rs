use argon_primitives::providers::{BlockSealerProvider, BlockSealerProviderWeightInfo};
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
}

type SealerInfoProviderWeights<T> = <<T as crate::Config>::SealerInfo as BlockSealerProvider<
	<T as frame_system::Config>::AccountId,
	<T as crate::Config>::MiningAuthorityId,
>>::Weights;

pub struct WithProviderWeights<T, Base, SealerInfoWeight = SealerInfoProviderWeights<T>>(
	PhantomData<(T, Base, SealerInfoWeight)>,
);
impl<T, Base, SealerInfoWeight> WeightInfo for WithProviderWeights<T, Base, SealerInfoWeight>
where
	T: crate::Config,
	Base: WeightInfo,
	SealerInfoWeight: BlockSealerProviderWeightInfo,
{
	fn bid() -> Weight {
		Base::bid()
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
		Base::start_new_frame(m)
	}

	fn on_finalize_frame_adjustments() -> Weight {
		Base::on_finalize_frame_adjustments()
	}

	fn block_seal_read_vote() -> Weight {
		Base::block_seal_read_vote()
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
}
