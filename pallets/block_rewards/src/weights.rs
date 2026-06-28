use argon_primitives::{
	notary::{NotaryProvider, NotaryProviderWeightInfo},
	providers::{
		BlockRewardAccountsProvider, BlockRewardAccountsProviderWeightInfo, BlockSealerProvider,
		BlockSealerProviderWeightInfo, NotebookProvider, NotebookProviderWeightInfo, TickProvider,
		TickProviderWeightInfo,
	},
};
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn set_block_rewards_paused() -> Weight;
	fn set_block_voter_rewards_enabled() -> Weight;
	fn on_initialize_with_rewards() -> Weight {
		Weight::zero()
	}
}

type BlockSealerProviderWeights<T> =
	<<T as crate::Config>::BlockSealerProvider as BlockSealerProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;
type BlockRewardAccountsProviderWeights<T> =
	<<T as crate::Config>::BlockRewardAccountsProvider as BlockRewardAccountsProvider<
		<T as frame_system::Config>::AccountId,
	>>::Weights;
type NotaryProviderWeights<T> = <<T as crate::Config>::NotaryProvider as NotaryProvider<
	<T as frame_system::Config>::Block,
	<T as frame_system::Config>::AccountId,
>>::Weights;
type NotebookProviderWeights<T> =
	<<T as crate::Config>::NotebookProvider as NotebookProvider>::Weights;
type TickProviderWeights<T> = <<T as crate::Config>::TickProvider as TickProvider<
	<T as frame_system::Config>::Block,
>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	BlockSealerWeight = BlockSealerProviderWeights<T>,
	BlockRewardAccountsWeight = BlockRewardAccountsProviderWeights<T>,
	NotaryProviderWeight = NotaryProviderWeights<T>,
	NotebookProviderWeight = NotebookProviderWeights<T>,
	TickProviderWeight = TickProviderWeights<T>,
>(
	PhantomData<(
		T,
		Base,
		BlockSealerWeight,
		BlockRewardAccountsWeight,
		NotaryProviderWeight,
		NotebookProviderWeight,
		TickProviderWeight,
	)>,
);
impl<
		T,
		Base,
		BlockSealerWeight,
		BlockRewardAccountsWeight,
		NotaryProviderWeight,
		NotebookProviderWeight,
		TickProviderWeight,
	> WeightInfo
	for WithProviderWeights<
		T,
		Base,
		BlockSealerWeight,
		BlockRewardAccountsWeight,
		NotaryProviderWeight,
		NotebookProviderWeight,
		TickProviderWeight,
	>
where
	T: crate::Config,
	Base: WeightInfo,
	BlockSealerWeight: BlockSealerProviderWeightInfo,
	BlockRewardAccountsWeight: BlockRewardAccountsProviderWeightInfo,
	NotaryProviderWeight: NotaryProviderWeightInfo,
	NotebookProviderWeight: NotebookProviderWeightInfo,
	TickProviderWeight: TickProviderWeightInfo,
{
	fn set_block_rewards_paused() -> Weight {
		Base::set_block_rewards_paused()
	}

	fn set_block_voter_rewards_enabled() -> Weight {
		Base::set_block_voter_rewards_enabled()
	}

	fn on_initialize_with_rewards() -> Weight {
		Base::on_initialize_with_rewards()
			.saturating_add(BlockSealerWeight::is_block_vote_seal())
			.saturating_add(BlockRewardAccountsWeight::is_compute_block_eligible_for_rewards())
			.saturating_add(TickProviderWeight::elapsed_ticks())
			.saturating_add(BlockSealerWeight::get_sealer_info())
			.saturating_add(BlockRewardAccountsWeight::get_block_rewards_account())
			.saturating_add(NotaryProviderWeight::active_notaries())
			.saturating_add(NotebookProviderWeight::notebooks_in_block())
			.saturating_add(TickProviderWeight::current_tick())
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn set_block_rewards_paused() -> Weight {
		Weight::zero()
	}

	fn set_block_voter_rewards_enabled() -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_rewards() -> Weight {
		Weight::zero()
	}
}
