//! Benchmarking setup for pallet-block-seal-spec
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as BlockSealSpecPallet;
use argon_primitives::{
	ComputeDifficulty, NotebookEventHandler, TickProvider,
	digests::{BLOCK_VOTES_DIGEST_ID, BlockVoteDigest, NotebookDigest},
	inherents::BlockSealInherent,
	notebook::NotebookHeader,
	providers::BlockSealSpecProvider,
	tick::Tick,
};
use codec::{Decode, Encode};
use frame_system::RawOrigin;
use pallet_prelude::benchmarking::{
	BenchmarkNotebookProviderCallCounters, benchmark_notebook_provider_call_counters,
	reset_benchmark_notebook_provider_call_counters, reset_benchmark_notebook_provider_state,
	set_all_digests,
};
use polkadot_sdk::{
	frame_benchmarking::v2::*,
	frame_support::{
		BoundedVec,
		storage::storage_prefix,
		traits::{Hooks, OnTimestampSet},
	},
	sp_core::H256,
	sp_runtime::{DigestItem, traits::SaturatedConversion},
};

fn benchmark_notebook_header(tick: Tick) -> NotebookHeader {
	NotebookHeader {
		version: 1,
		notebook_number: 1,
		tick,
		tax: 0,
		notary_id: 1,
		chain_transfers: BoundedVec::default(),
		changed_accounts_root: H256::repeat_byte(1),
		changed_account_origins: BoundedVec::default(),
		block_votes_root: H256::repeat_byte(2),
		block_votes_count: 1,
		blocks_with_votes: BoundedVec::default(),
		block_voting_power: 1,
		secret_hash: H256::repeat_byte(3),
		parent_secret: Some(H256::repeat_byte(4)),
		domains: BoundedVec::default(),
	}
}

fn seed_block_seal_finalize_prereqs<T: Config>() {
	let temp_seal_key = storage_prefix(b"BlockSeal", b"TempSealInherent");
	sp_io::storage::set(&temp_seal_key, &BlockSealInherent::Compute.encode());
	reset_benchmark_notebook_provider_state();

	let author = T::AccountId::decode(&mut &[0u8; 32][..])
		.expect("benchmark account bytes must decode to account id");
	set_all_digests::<T, ()>(
		author,
		T::TickProvider::current_tick(),
		BlockVoteDigest { voting_power: 1, votes_count: 1 },
		NotebookDigest::<()> { notebooks: BoundedVec::default() },
	);
}

fn assert_provider_calls(expected: BenchmarkNotebookProviderCallCounters) {
	if cfg!(test) {
		assert_eq!(benchmark_notebook_provider_call_counters(), expected);
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn configure() {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let vote_minimum = 123u128;
		let compute_difficulty = 456u128;

		#[extrinsic_call]
		configure(RawOrigin::Root, Some(vote_minimum), Some(compute_difficulty));

		assert_eq!(CurrentVoteMinimum::<T>::get(), vote_minimum);
		assert_eq!(CurrentComputeDifficulty::<T>::get(), compute_difficulty);
		assert_provider_calls(Default::default());
	}

	#[benchmark]
	fn on_initialize_with_digest() {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let digest = BlockVoteDigest { voting_power: 100, votes_count: 10 };
		assert!(!TempBlockVoteDigest::<T>::exists());
		frame_system::Pallet::<T>::deposit_log(DigestItem::PreRuntime(
			BLOCK_VOTES_DIGEST_ID,
			digest.encode(),
		));

		#[block]
		{
			BlockSealSpecPallet::<T>::on_initialize(1u32.into());
		}

		assert_eq!(TempBlockVoteDigest::<T>::get(), Some(digest));
		assert_provider_calls(BenchmarkNotebookProviderCallCounters {
			vote_eligible_notebook_count: 1,
			..Default::default()
		});
	}

	#[benchmark]
	fn on_finalize_with_vote_adjustment(n: Linear<1, 25>) {
		seed_block_seal_finalize_prereqs::<T>();
		let notebook_tick = T::TickProvider::voting_schedule().notebook_tick();
		for i in 0..n {
			let mut header = benchmark_notebook_header(notebook_tick);
			header.notary_id = i.saturating_add(1);
			<BlockSealSpecPallet<T> as NotebookEventHandler>::notebook_submitted(&header);
		}

		PastComputeBlockTimes::<T>::mutate(|times| {
			times.clear();
			for i in 0..T::HistoricalComputeBlocksForAverage::get() {
				let _ = times.try_push(1_000u64.saturating_add(i as u64));
			}
		});
		PastBlockVotes::<T>::mutate(|votes| {
			votes.clear();
			for i in 0..T::HistoricalVoteBlocksForAverage::get() {
				let past_tick = notebook_tick.saturating_sub((i as Tick).saturating_add(1));
				let _ = votes.try_push((past_tick, 10, 100));
			}
		});
		let now: T::Moment = 2_000u64.saturated_into();
		<BlockSealSpecPallet<T> as OnTimestampSet<T::Moment>>::on_timestamp_set(now);
		reset_benchmark_notebook_provider_call_counters();

		#[block]
		{
			BlockSealSpecPallet::<T>::on_finalize(1u32.into());
		}

		assert!(PreviousBlockTimestamp::<T>::exists());
		assert!(!VoteMinimumHistory::<T>::get().is_empty());
		assert_provider_calls(BenchmarkNotebookProviderCallCounters {
			is_notary_locked_at_tick: n,
			..Default::default()
		});
	}

	#[benchmark]
	fn notebook_submitted() {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let notebook_tick = T::TickProvider::voting_schedule().notebook_tick();
		let header = benchmark_notebook_header(notebook_tick);

		#[block]
		{
			<BlockSealSpecPallet<T> as NotebookEventHandler>::notebook_submitted(&header);
		}

		assert_eq!(TempCurrentTickNotebooksInBlock::<T>::get().len(), 1);
		assert_provider_calls(Default::default());
	}

	#[benchmark]
	fn provider_grandparent_vote_minimum() {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let vote_minimum: VoteMinimum = 123u128;
		VoteMinimumHistory::<T>::put(BoundedVec::truncate_from(vec![vote_minimum]));

		#[block]
		{
			let result =
				<BlockSealSpecPallet<T> as BlockSealSpecProvider<T::Block>>::grandparent_vote_minimum();
			assert_eq!(result, Some(vote_minimum));
		}

		assert_provider_calls(Default::default());
	}

	#[benchmark]
	fn provider_compute_difficulty() {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let expected_difficulty: ComputeDifficulty = 456u128;
		CurrentComputeDifficulty::<T>::put(expected_difficulty);

		#[block]
		{
			let result =
				<BlockSealSpecPallet<T> as BlockSealSpecProvider<T::Block>>::compute_difficulty();
			assert_eq!(result, expected_difficulty);
		}

		assert_provider_calls(Default::default());
	}

	#[benchmark]
	fn provider_compute_key_block_hash() {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let expected_hash = frame_system::Pallet::<T>::parent_hash();
		CurrentComputeKeyBlock::<T>::put(expected_hash);

		#[block]
		{
			let result =
				<BlockSealSpecPallet<T> as BlockSealSpecProvider<T::Block>>::compute_key_block_hash(
				);
			assert_eq!(result, Some(expected_hash));
		}

		assert_provider_calls(Default::default());
	}

	impl_benchmark_test_suite!(
		BlockSealSpecPallet,
		crate::mock::new_test_ext(100, 200),
		crate::mock::Test
	);
}
