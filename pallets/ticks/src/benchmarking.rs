#![cfg(feature = "runtime-benchmarks")]

use super::*;
use argon_notary_audit::VerifyError;
use argon_primitives::{
	NotebookAuditResult, TickProvider,
	digests::{BlockVoteDigest, NotebookDigest},
	tick::{MAX_BLOCKS_PER_TICK, Tick},
};
use codec::Decode;
use pallet_prelude::benchmarking::set_all_digests;
use polkadot_sdk::{
	frame_benchmarking::v2::*,
	frame_support::{BoundedVec, traits::Hooks},
};

const MAX_CLEANUP_RANGE: u32 = 1_024;

#[benchmarks(
	where
		T::AccountId: Decode,
		T::Hash: Decode,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn on_initialize_with_cleanup(c: Linear<0, MAX_CLEANUP_RANGE>) {
		let cleanup_range = c;
		let parent_tick = if cleanup_range == 0 {
			MAX_RECENT_BLOCKS
		} else {
			MAX_RECENT_BLOCKS.saturating_add(25)
		};
		let proposed_tick = if cleanup_range == 0 {
			parent_tick
		} else {
			parent_tick.saturating_add(cleanup_range as Tick).saturating_sub(1)
		};

		CurrentTick::<T>::put(parent_tick);
		PreviousTick::<T>::put(parent_tick.saturating_sub(1));
		GenesisTick::<T>::put(parent_tick.saturating_sub(1));

		seed_recent_blocks::<T>(parent_tick, proposed_tick, cleanup_range);
		set_all_digests::<T, VerifyError>(
			benchmark_author::<T>(),
			proposed_tick,
			BlockVoteDigest { voting_power: 1, votes_count: 1 },
			benchmark_notebooks_digest(proposed_tick),
		);

		#[block]
		{
			Pallet::<T>::on_initialize(1u32.into());
		}

		assert_eq!(CurrentTick::<T>::get(), proposed_tick);
		assert_eq!(PreviousTick::<T>::get(), parent_tick);

		if cleanup_range == 0 {
			assert_eq!(
				RecentBlocksAtTicks::<T>::get(parent_tick).len(),
				MAX_BLOCKS_PER_TICK as usize
			);
		} else {
			let oldest_removed_tick = parent_tick.saturating_sub(MAX_RECENT_BLOCKS);
			for offset in 0..cleanup_range {
				let removed_tick = oldest_removed_tick.saturating_add(Tick::from(offset));
				if removed_tick != parent_tick {
					assert!(!RecentBlocksAtTicks::<T>::contains_key(removed_tick));
				}
			}
			assert_eq!(RecentBlocksAtTicks::<T>::get(parent_tick).len(), 1);
			assert_eq!(
				RecentBlocksAtTicks::<T>::get(proposed_tick).len(),
				MAX_BLOCKS_PER_TICK as usize - 1,
			);
		}
	}

	#[benchmark]
	fn provider_previous_tick() {
		let previous_tick = 41;
		PreviousTick::<T>::put(previous_tick);

		#[block]
		{
			assert_eq!(<Pallet<T> as TickProvider<T::Block>>::previous_tick(), previous_tick);
		}
	}

	#[benchmark]
	fn provider_current_tick() {
		let current_tick = 42;
		CurrentTick::<T>::put(current_tick);

		#[block]
		{
			assert_eq!(<Pallet<T> as TickProvider<T::Block>>::current_tick(), current_tick);
		}
	}

	#[benchmark]
	fn provider_elapsed_ticks() {
		GenesisTick::<T>::put(10);
		CurrentTick::<T>::put(42);

		#[block]
		{
			assert_eq!(<Pallet<T> as TickProvider<T::Block>>::elapsed_ticks(), 32);
		}
	}

	#[benchmark]
	fn provider_ticker() {
		let ticker = benchmark_ticker();
		GenesisTicker::<T>::put(ticker);

		#[block]
		{
			assert_eq!(<Pallet<T> as TickProvider<T::Block>>::ticker(), ticker);
		}
	}

	#[benchmark]
	fn provider_blocks_at_tick() {
		let tick = 77;
		RecentBlocksAtTicks::<T>::insert(
			tick,
			BoundedVec::truncate_from(
				(0..MAX_BLOCKS_PER_TICK)
					.map(|index| benchmark_hash::<T>(index as u8 + 64))
					.collect::<Vec<_>>(),
			),
		);

		#[block]
		{
			assert_eq!(
				<Pallet<T> as TickProvider<T::Block>>::blocks_at_tick(tick).len(),
				MAX_BLOCKS_PER_TICK as usize,
			);
		}
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(500), crate::mock::Test);
}

fn seed_recent_blocks<T: Config>(parent_tick: Tick, proposed_tick: Tick, cleanup_range: u32)
where
	T::Hash: Decode,
{
	if cleanup_range == 0 {
		RecentBlocksAtTicks::<T>::insert(
			parent_tick,
			BoundedVec::truncate_from(
				(0..MAX_BLOCKS_PER_TICK.saturating_sub(1))
					.map(|i| benchmark_hash::<T>(i as u8 + 1))
					.collect::<Vec<_>>(),
			),
		);
		return;
	}

	for offset in 0..cleanup_range {
		let cleanup_tick =
			parent_tick.saturating_sub(MAX_RECENT_BLOCKS).saturating_add(Tick::from(offset));
		RecentBlocksAtTicks::<T>::insert(
			cleanup_tick,
			BoundedVec::truncate_from(vec![benchmark_hash::<T>(offset as u8 + 1)]),
		);
	}

	RecentBlocksAtTicks::<T>::insert(
		proposed_tick,
		BoundedVec::truncate_from(
			(0..MAX_BLOCKS_PER_TICK.saturating_sub(1))
				.map(|i| benchmark_hash::<T>(i as u8 + 33))
				.collect::<Vec<_>>(),
		),
	);
}

fn benchmark_author<T: Config>() -> T::AccountId
where
	T::AccountId: Decode,
{
	T::AccountId::decode(&mut &[0u8; 32][..]).expect("benchmark author account should decode")
}

fn benchmark_hash<T: Config>(seed: u8) -> T::Hash
where
	T::Hash: Decode,
{
	T::Hash::decode(&mut &[seed; 32][..]).expect("benchmark hash should decode")
}

fn benchmark_notebooks_digest(tick: Tick) -> NotebookDigest<VerifyError> {
	NotebookDigest {
		notebooks: BoundedVec::truncate_from(vec![NotebookAuditResult {
			tick,
			notary_id: 1,
			notebook_number: 1,
			audit_first_failure: None,
		}]),
	}
}

fn benchmark_ticker() -> argon_primitives::tick::Ticker {
	argon_primitives::tick::Ticker::new(1_000, 2)
}
