use argon_primitives::providers::{BlockSealerProvider, BlockSealerProviderWeightInfo};
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn set_block_rewards_paused() -> Weight;
	fn on_initialize_with_rewards() -> Weight {
		Weight::zero()
	}
}

type BlockSealerProviderWeights<T> =
	<<T as crate::Config>::BlockSealerProvider as BlockSealerProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;

pub struct WithProviderWeights<T, Base, BlockSealerWeight = BlockSealerProviderWeights<T>>(
	PhantomData<(T, Base, BlockSealerWeight)>,
);
impl<T, Base, BlockSealerWeight> WeightInfo for WithProviderWeights<T, Base, BlockSealerWeight>
where
	T: crate::Config,
	Base: WeightInfo,
	BlockSealerWeight: BlockSealerProviderWeightInfo,
{
	fn set_block_rewards_paused() -> Weight {
		Base::set_block_rewards_paused()
	}

	fn on_initialize_with_rewards() -> Weight {
		Base::on_initialize_with_rewards()
			.saturating_add(BlockSealerWeight::is_block_vote_seal())
			.saturating_add(BlockSealerWeight::get_sealer_info())
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn set_block_rewards_paused() -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_rewards() -> Weight {
		Weight::zero()
	}
}
