use argon_primitives::providers::{
	BlockSealSpecProviderWeightInfo, NotebookProvider, NotebookProviderWeightInfo,
};
use pallet_prelude::*;

/// Weight functions needed for this pallet.
pub trait WeightInfo {
	fn configure() -> Weight;
	fn on_initialize_with_digest() -> Weight;
	fn on_finalize_with_vote_adjustment(n: u32) -> Weight;
	fn notebook_submitted() -> Weight;
	fn provider_grandparent_vote_minimum() -> Weight;
	fn provider_compute_difficulty() -> Weight;
	fn provider_compute_key_block_hash() -> Weight;
}

type NotebookProviderWeights<T> =
	<<T as crate::Config>::NotebookProvider as NotebookProvider>::Weights;

pub struct WithProviderWeights<T, Base, NotebookProviderWeight = NotebookProviderWeights<T>>(
	PhantomData<(T, Base, NotebookProviderWeight)>,
);
impl<T, Base, NotebookProviderWeight> WeightInfo
	for WithProviderWeights<T, Base, NotebookProviderWeight>
where
	T: crate::Config,
	Base: WeightInfo,
	NotebookProviderWeight: NotebookProviderWeightInfo,
{
	fn configure() -> Weight {
		Base::configure()
	}

	fn on_initialize_with_digest() -> Weight {
		Base::on_initialize_with_digest()
	}

	fn on_finalize_with_vote_adjustment(n: u32) -> Weight {
		Base::on_finalize_with_vote_adjustment(n)
			.saturating_add(NotebookProviderWeight::vote_eligible_notebook_count(n))
			.saturating_add(
				NotebookProviderWeight::is_notary_locked_at_tick().saturating_mul(n.into()),
			)
	}

	fn notebook_submitted() -> Weight {
		Base::notebook_submitted()
	}

	fn provider_grandparent_vote_minimum() -> Weight {
		Base::provider_grandparent_vote_minimum()
	}

	fn provider_compute_difficulty() -> Weight {
		Base::provider_compute_difficulty()
	}

	fn provider_compute_key_block_hash() -> Weight {
		Base::provider_compute_key_block_hash()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> BlockSealSpecProviderWeightInfo for ProviderWeightAdapter<T> {
	fn grandparent_vote_minimum() -> Weight {
		<T as crate::Config>::WeightInfo::provider_grandparent_vote_minimum()
	}

	fn compute_difficulty() -> Weight {
		<T as crate::Config>::WeightInfo::provider_compute_difficulty()
	}

	fn compute_key_block_hash() -> Weight {
		<T as crate::Config>::WeightInfo::provider_compute_key_block_hash()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn configure() -> Weight {
		Weight::zero()
	}

	fn on_initialize_with_digest() -> Weight {
		Weight::zero()
	}

	fn on_finalize_with_vote_adjustment(_n: u32) -> Weight {
		Weight::zero()
	}

	fn notebook_submitted() -> Weight {
		Weight::zero()
	}

	fn provider_grandparent_vote_minimum() -> Weight {
		Weight::zero()
	}

	fn provider_compute_difficulty() -> Weight {
		Weight::zero()
	}

	fn provider_compute_key_block_hash() -> Weight {
		Weight::zero()
	}
}
