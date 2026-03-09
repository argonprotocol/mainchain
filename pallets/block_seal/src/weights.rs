use argon_primitives::providers::{
	BlockSealSpecProvider, BlockSealSpecProviderWeightInfo, BlockSealerProviderWeightInfo,
	NotebookProvider, NotebookProviderWeightInfo,
};
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn apply() -> Weight;
	fn on_finalize_with_notebooks(n: u32) -> Weight;
	fn on_initialize_with_notebooks(notebook_count: u32) -> Weight;
	fn provider_get_sealer_info() -> Weight;
	fn provider_is_block_vote_seal() -> Weight;
}

type NotebookProviderWeights<T> =
	<<T as crate::Config>::NotebookProvider as NotebookProvider>::Weights;
type BlockSealSpecProviderWeights<T> =
	<<T as crate::Config>::BlockSealSpecProvider as BlockSealSpecProvider<
		<T as frame_system::Config>::Block,
	>>::Weights;

pub struct WithProviderWeights<
	T,
	Base,
	NotebookProviderWeight = NotebookProviderWeights<T>,
	BlockSealSpecProviderWeight = BlockSealSpecProviderWeights<T>,
>(PhantomData<(T, Base, NotebookProviderWeight, BlockSealSpecProviderWeight)>);
impl<T, Base, NotebookProviderWeight, BlockSealSpecProviderWeight> WeightInfo
	for WithProviderWeights<T, Base, NotebookProviderWeight, BlockSealSpecProviderWeight>
where
	T: crate::Config,
	Base: WeightInfo,
	NotebookProviderWeight: NotebookProviderWeightInfo,
	BlockSealSpecProviderWeight: BlockSealSpecProviderWeightInfo,
{
	fn apply() -> Weight {
		let compute_path_provider_weight = BlockSealSpecProviderWeight::compute_difficulty();
		let vote_path_provider_weight = BlockSealSpecProviderWeight::grandparent_vote_minimum()
			.saturating_add(NotebookProviderWeight::get_eligible_tick_votes_root());
		let branch_provider_weight = Weight::from_parts(
			compute_path_provider_weight
				.ref_time()
				.max(vote_path_provider_weight.ref_time()),
			compute_path_provider_weight
				.proof_size()
				.max(vote_path_provider_weight.proof_size()),
		);

		Base::apply()
			.saturating_add(NotebookProviderWeight::notebooks_in_block())
			.saturating_add(branch_provider_weight)
	}

	fn on_finalize_with_notebooks(n: u32) -> Weight {
		Base::on_finalize_with_notebooks(n)
			.saturating_add(NotebookProviderWeight::eligible_notebooks_for_vote(n))
			.saturating_add(
				NotebookProviderWeight::get_eligible_tick_votes_root().saturating_mul(n.into()),
			)
	}

	fn on_initialize_with_notebooks(notebook_count: u32) -> Weight {
		Base::on_initialize_with_notebooks(notebook_count)
			.saturating_add(NotebookProviderWeight::vote_eligible_notebook_count(notebook_count))
	}

	fn provider_get_sealer_info() -> Weight {
		Base::provider_get_sealer_info()
	}

	fn provider_is_block_vote_seal() -> Weight {
		Base::provider_is_block_vote_seal()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> BlockSealerProviderWeightInfo for ProviderWeightAdapter<T> {
	fn get_sealer_info() -> Weight {
		<T as crate::Config>::WeightInfo::provider_get_sealer_info()
	}

	fn is_block_vote_seal() -> Weight {
		<T as crate::Config>::WeightInfo::provider_is_block_vote_seal()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn apply() -> Weight {
		Weight::zero()
	}
	fn on_finalize_with_notebooks(_n: u32) -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_notebooks(_notebook_count: u32) -> Weight {
		Weight::zero()
	}

	fn provider_get_sealer_info() -> Weight {
		Weight::zero()
	}

	fn provider_is_block_vote_seal() -> Weight {
		Weight::zero()
	}
}
