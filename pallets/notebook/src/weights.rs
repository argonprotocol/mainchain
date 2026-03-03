use argon_primitives::providers::{
	ChainTransferLookup, ChainTransferLookupWeightInfo, NotebookProviderWeightInfo,
};
use pallet_prelude::*;

/// Weight functions needed for pallet_notebook.
pub trait WeightInfo {
	fn submit(n: u32) -> Weight;
	fn submit_with_account_origins(a: u32) -> Weight;
	fn submit_with_chain_transfers(t: u32) -> Weight;
	fn unlock() -> Weight;
	fn provider_notebooks_in_block() -> Weight;
	fn provider_vote_eligible_notebook_count(n: u32) -> Weight;
	fn provider_eligible_notebooks_for_vote(n: u32) -> Weight;
	fn provider_get_eligible_tick_votes_root() -> Weight;
	fn provider_is_notary_locked_at_tick() -> Weight;
}

type ChainTransferLookupWeights<T> =
	<<T as crate::Config>::ChainTransferLookup as ChainTransferLookup<
		<T as frame_system::Config>::AccountId,
		Balance,
	>>::Weights;

pub struct WithProviderWeights<T, Base, ChainTransferLookupWeight = ChainTransferLookupWeights<T>>(
	PhantomData<(T, Base, ChainTransferLookupWeight)>,
);
impl<T, Base, ChainTransferLookupWeight> WeightInfo
	for WithProviderWeights<T, Base, ChainTransferLookupWeight>
where
	T: crate::Config,
	Base: WeightInfo,
	ChainTransferLookupWeight: ChainTransferLookupWeightInfo,
{
	fn submit(n: u32) -> Weight {
		Base::submit(n)
	}

	fn submit_with_account_origins(a: u32) -> Weight {
		Base::submit_with_account_origins(a)
	}

	fn submit_with_chain_transfers(t: u32) -> Weight {
		Base::submit_with_chain_transfers(t).saturating_add(
			ChainTransferLookupWeight::is_valid_transfer_to_localchain().saturating_mul(t.into()),
		)
	}

	fn unlock() -> Weight {
		Base::unlock()
	}

	fn provider_notebooks_in_block() -> Weight {
		Base::provider_notebooks_in_block()
	}

	fn provider_vote_eligible_notebook_count(n: u32) -> Weight {
		Base::provider_vote_eligible_notebook_count(n)
	}

	fn provider_eligible_notebooks_for_vote(n: u32) -> Weight {
		Base::provider_eligible_notebooks_for_vote(n)
	}

	fn provider_get_eligible_tick_votes_root() -> Weight {
		Base::provider_get_eligible_tick_votes_root()
	}

	fn provider_is_notary_locked_at_tick() -> Weight {
		Base::provider_is_notary_locked_at_tick()
	}
}

pub struct ProviderWeightAdapter<T>(PhantomData<T>);
impl<T: crate::Config> NotebookProviderWeightInfo for ProviderWeightAdapter<T> {
	fn notebooks_in_block() -> Weight {
		T::WeightInfo::provider_notebooks_in_block()
	}

	fn vote_eligible_notebook_count(n: u32) -> Weight {
		T::WeightInfo::provider_vote_eligible_notebook_count(n)
	}

	fn eligible_notebooks_for_vote(n: u32) -> Weight {
		T::WeightInfo::provider_eligible_notebooks_for_vote(n)
	}

	fn get_eligible_tick_votes_root() -> Weight {
		T::WeightInfo::provider_get_eligible_tick_votes_root()
	}

	fn is_notary_locked_at_tick() -> Weight {
		T::WeightInfo::provider_is_notary_locked_at_tick()
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn submit(_n: u32) -> Weight {
		Weight::zero()
	}

	fn submit_with_account_origins(_a: u32) -> Weight {
		Weight::zero()
	}

	fn submit_with_chain_transfers(_t: u32) -> Weight {
		Weight::zero()
	}

	fn unlock() -> Weight {
		Weight::zero()
	}

	fn provider_notebooks_in_block() -> Weight {
		Weight::zero()
	}

	fn provider_vote_eligible_notebook_count(_n: u32) -> Weight {
		Weight::zero()
	}

	fn provider_eligible_notebooks_for_vote(_n: u32) -> Weight {
		Weight::zero()
	}

	fn provider_get_eligible_tick_votes_root() -> Weight {
		Weight::zero()
	}

	fn provider_is_notary_locked_at_tick() -> Weight {
		Weight::zero()
	}
}
