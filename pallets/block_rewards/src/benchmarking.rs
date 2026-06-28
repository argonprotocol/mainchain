use super::*;
#[allow(unused)]
use crate::Pallet as BlockRewardsPallet;
use argon_primitives::TickProvider;
use frame_system::RawOrigin;
use pallet_prelude::benchmarking::{
	set_benchmark_notebook_provider_state, BenchmarkNotebookProviderState,
};
use polkadot_sdk::{frame_benchmarking::v2::*, frame_support::traits::Hooks};

#[benchmarks]
mod benchmarks {
	use super::*;

	// Both setter benchmarks exercise the non-default transition for their toggle.
	#[benchmark]
	fn set_block_rewards_paused() {
		assert!(!BlockRewardsPaused::<T>::get());

		#[extrinsic_call]
		set_block_rewards_paused(RawOrigin::Root, true);

		assert!(BlockRewardsPaused::<T>::get());
	}

	#[benchmark]
	fn set_block_voter_rewards_enabled() {
		assert!(!BlockVoterRewardsEnabled::<T>::get());

		#[extrinsic_call]
		set_block_voter_rewards_enabled(RawOrigin::Root, true);

		assert!(BlockVoterRewardsEnabled::<T>::get());
	}

	#[benchmark]
	fn on_initialize_with_rewards() {
		let block_number: BlockNumberFor<T> = 1u32.into();
		let notebook_tick = T::TickProvider::voting_schedule().notebook_tick();

		set_benchmark_notebook_provider_state(BenchmarkNotebookProviderState {
			notebooks_in_block: vec![(1, 1, notebook_tick)],
			..Default::default()
		});
		#[cfg(test)]
		{
			crate::mock::NotebookTick::set(notebook_tick);
			crate::mock::NotebooksInBlock::set(vec![(1, 1, notebook_tick)]);
		}

		frame_system::Pallet::<T>::set_block_number(block_number);
		BlockVoterRewardsEnabled::<T>::set(true);
		assert!(PayoutsByBlock::<T>::get(block_number).is_empty());

		// Reward payout happens in `on_finalize`, but the runtime charges that path from
		// `on_initialize` because `on_finalize` cannot return weight.
		#[block]
		{
			BlockRewardsPallet::<T>::on_finalize(block_number);
		}

		let payouts = PayoutsByBlock::<T>::get(block_number);
		assert_eq!(payouts.len(), 2);
	}

	impl_benchmark_test_suite!(BlockRewardsPallet, crate::mock::new_test_ext(), crate::mock::Test);
}
