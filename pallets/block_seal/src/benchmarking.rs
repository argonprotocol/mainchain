//! Benchmarking setup for pallet-block-seal
#![cfg(feature = "runtime-benchmarks")]

extern crate alloc;

use super::*;
use crate::Pallet as BlockSealPallet;
use alloc::collections::BTreeMap;
use argon_primitives::{
	AuthorityProvider, BlockSealerInfo, MerkleProof, NotebookAuditResult, TickProvider,
	digests::{BlockVoteDigest, NotebookDigest},
	inherents::BlockSealInherent,
	localchain::BlockVote,
	providers::BlockSealerProvider,
	tick::Tick,
};
use binary_merkle_tree::{merkle_proof, merkle_root};
use codec::Encode;
use frame_system::RawOrigin;
use pallet_prelude::benchmarking::{
	BenchmarkNotebookProviderCallCounters, BenchmarkNotebookProviderState,
	benchmark_notebook_provider_call_counters, benchmark_notebook_provider_state,
	reset_benchmark_notebook_provider_call_counters, reset_benchmark_notebook_provider_state,
	set_all_digests, set_benchmark_notebook_provider_state, synthetic_benchmark_parent_secret,
	synthetic_benchmark_votes_root,
};
#[cfg(not(test))]
use polkadot_sdk::sp_core::crypto::KeyTypeId;
#[cfg(test)]
use polkadot_sdk::sp_core::{Pair, ed25519::Pair as Ed25519Pair};
use polkadot_sdk::{
	frame_benchmarking::v2::*,
	frame_support::{BoundedVec, traits::Hooks},
	sp_runtime::traits::BlakeTwo256,
};

#[cfg(not(test))]
const BENCH_VOTE_KEY_TYPE: KeyTypeId = KeyTypeId(*b"bvte");

fn ensure_required_digests<T: Config>(
	author: T::AccountId,
	notebook_count: u32,
	include_vote_tick_entries: bool,
) {
	let voting_schedule = T::TickProvider::voting_schedule();
	let notebook_tick = voting_schedule.notebook_tick();
	let votes_tick = voting_schedule.eligible_votes_tick();

	let mut notebooks_in_block = Vec::new();
	let mut notebooks_for_vote = Vec::new();
	let mut eligible_vote_roots = BTreeMap::new();
	let mut notebooks = (0..notebook_count)
		.map(|i| {
			let notary_id = i.saturating_add(1);
			notebooks_in_block.push((notary_id, 2, notebook_tick));
			notebooks_for_vote.push((
				notary_id,
				2,
				Some(synthetic_benchmark_parent_secret(notary_id)),
			));
			NotebookAuditResult {
				notary_id,
				notebook_number: 2,
				tick: notebook_tick,
				audit_first_failure: None,
			}
		})
		.collect::<Vec<_>>();
	if include_vote_tick_entries {
		notebooks.extend((0..notebook_count).map(|i| {
			let notary_id = i.saturating_add(1);
			notebooks_in_block.push((notary_id, 1, votes_tick));
			eligible_vote_roots.insert(
				(notary_id, votes_tick),
				(synthetic_benchmark_votes_root(notary_id, votes_tick), 1),
			);
			NotebookAuditResult {
				notary_id,
				notebook_number: 1,
				tick: votes_tick,
				audit_first_failure: None,
			}
		}));
	}
	let mut notebooks_by_tick = BTreeMap::new();
	notebooks_by_tick.insert(notebook_tick, notebooks_for_vote);
	set_benchmark_notebook_provider_state(BenchmarkNotebookProviderState {
		notebooks_in_block,
		notebooks_by_tick,
		eligible_vote_roots,
		notary_locked_from_tick: BTreeMap::new(),
		call_counters: Default::default(),
	});

	set_all_digests::<T, ()>(
		author,
		T::TickProvider::current_tick(),
		BlockVoteDigest { voting_power: 1, votes_count: 1 },
		NotebookDigest::<()> { notebooks: BoundedVec::truncate_from(notebooks) },
	);
}

fn signed_non_default_proxy_vote(tick: Tick) -> BlockVote {
	#[cfg(test)]
	let (public, signature) = {
		let pair = Ed25519Pair::from_seed(&[7u8; 32]);
		let public = pair.public();
		let account_id = argon_primitives::AccountId::new(public.0);
		let block_vote = BlockVote {
			account_id: account_id.clone(),
			block_hash: H256::zero(),
			index: 1,
			power: u128::MAX,
			signature: sp_runtime::MultiSignature::Ed25519([0; 64].into()),
			block_rewards_account_id: account_id,
			tick,
		};
		let signature = pair.sign(block_vote.hash().as_ref());
		(public, signature)
	};

	#[cfg(not(test))]
	let (public, signature) = {
		let public = sp_io::crypto::ed25519_generate(
			BENCH_VOTE_KEY_TYPE,
			Some(b"//block-seal-bench-vote".to_vec()),
		);
		let account_id = argon_primitives::AccountId::new(public.0);
		let block_vote = BlockVote {
			account_id: account_id.clone(),
			block_hash: H256::zero(),
			index: 1,
			power: u128::MAX,
			signature: sp_runtime::MultiSignature::Ed25519([0; 64].into()),
			block_rewards_account_id: account_id,
			tick,
		};
		let signature =
			sp_io::crypto::ed25519_sign(BENCH_VOTE_KEY_TYPE, &public, block_vote.hash().as_ref())
				.expect("benchmark signing key should exist");
		(public, signature)
	};

	let account_id = argon_primitives::AccountId::new(public.0);
	BlockVote {
		account_id: account_id.clone(),
		block_hash: H256::zero(),
		index: 1,
		power: u128::MAX,
		signature: signature.into(),
		block_rewards_account_id: account_id,
		tick,
	}
}

fn assert_provider_calls(expected: BenchmarkNotebookProviderCallCounters) {
	if cfg!(test) {
		let actual = benchmark_notebook_provider_call_counters();
		if actual != BenchmarkNotebookProviderCallCounters::default() ||
			expected == BenchmarkNotebookProviderCallCounters::default()
		{
			assert_eq!(actual, expected);
		}
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn apply() {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		#[cfg(test)]
		crate::mock::CurrentTick::set(1);
		let seal = if cfg!(test) {
			BlockSealInherent::Compute
		} else {
			let voting_schedule = T::TickProvider::voting_schedule();
			let notary_id = 1u32;
			let parent_voting_key = H256::repeat_byte(7);
			let block_vote = signed_non_default_proxy_vote(voting_schedule.eligible_votes_tick());
			let block_vote_bytes = block_vote.encode();
			let block_vote_root = merkle_root::<BlakeTwo256, _>(vec![block_vote_bytes.clone()]);
			let block_vote_proof =
				merkle_proof::<BlakeTwo256, _, _>(vec![block_vote_bytes], 0).proof;
			let seal_proof = block_vote.get_seal_proof(notary_id, parent_voting_key);
			let (winning_authority, _, _) =
				T::AuthorityProvider::get_winning_managed_authority(seal_proof, None, None)
					.expect("benchmark requires at least one active mining authority");
			let ref_block_number =
				<frame_system::Pallet<T>>::block_number().saturating_sub(1u32.into());
			let miner_nonce_score = T::AuthorityProvider::get_authority_score(
				seal_proof,
				&winning_authority.authority_id,
				&winning_authority.account_id,
				ref_block_number,
			)
			.expect("benchmark authority score must be available");
			ensure_required_digests::<T>(winning_authority.account_id, 1, true);
			let mut notebook_provider_state = benchmark_notebook_provider_state();
			notebook_provider_state
				.eligible_vote_roots
				.insert((notary_id, voting_schedule.eligible_votes_tick()), (block_vote_root, 1));
			set_benchmark_notebook_provider_state(notebook_provider_state);
			ParentVotingKey::<T>::put(Some(parent_voting_key));
			let current_tick = T::TickProvider::current_tick();
			LastTickWithVoteSeal::<T>::put(current_tick.saturating_add(1));
			BlockSealInherent::Vote {
				seal_strength: block_vote.get_seal_strength(notary_id, parent_voting_key),
				block_vote,
				notary_id,
				source_notebook_proof: MerkleProof {
					proof: BoundedVec::truncate_from(block_vote_proof),
					number_of_leaves: 1,
					leaf_index: 0,
				},
				source_notebook_number: 1,
				miner_nonce_score: Some(miner_nonce_score),
			}
		};

		#[extrinsic_call]
		apply(RawOrigin::None, seal);

		assert!(TempSealInherent::<T>::exists());
		if cfg!(test) {
			assert_provider_calls(BenchmarkNotebookProviderCallCounters {
				notebooks_in_block: 1,
				..Default::default()
			});
		} else {
			assert_provider_calls(BenchmarkNotebookProviderCallCounters {
				notebooks_in_block: 1,
				get_eligible_tick_votes_root: 1,
				..Default::default()
			});
		}
	}

	#[benchmark]
	fn on_initialize_with_notebooks(n: Linear<1, 100>) {
		reset_benchmark_notebook_provider_state();
		let author: T::AccountId = account("block-author", 0, 0);
		ensure_required_digests::<T>(author, n, false);
		reset_benchmark_notebook_provider_call_counters();
		#[block]
		{
			BlockSealPallet::<T>::on_initialize(1u32.into());
		}
		assert_provider_calls(BenchmarkNotebookProviderCallCounters {
			vote_eligible_notebook_count: 1,
			..Default::default()
		});
	}

	#[benchmark]
	fn on_finalize_with_notebooks(n: Linear<1, 100>) {
		reset_benchmark_notebook_provider_state();
		let author: T::AccountId = account("block-author", 0, 0);
		ensure_required_digests::<T>(author, n, true);
		TempSealInherent::<T>::put(BlockSealInherent::Compute);
		reset_benchmark_notebook_provider_call_counters();

		#[block]
		{
			BlockSealPallet::<T>::on_finalize(1u32.into());
		}

		assert!(!TempSealInherent::<T>::exists());
		assert_provider_calls(BenchmarkNotebookProviderCallCounters {
			eligible_notebooks_for_vote: 1,
			get_eligible_tick_votes_root: n,
			..Default::default()
		});
	}

	#[benchmark]
	fn provider_get_sealer_info() {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		let author: T::AccountId = account("block-sealer", 0, 0);
		LastBlockSealerInfo::<T>::put(BlockSealerInfo {
			block_author_account_id: author.clone(),
			block_vote_rewards_account: None,
			block_seal_authority: None,
		});

		#[block]
		{
			let sealer_info =
				<BlockSealPallet<T> as BlockSealerProvider<T::AccountId>>::get_sealer_info();
			assert_eq!(sealer_info.block_author_account_id, author);
		}

		assert_provider_calls(Default::default());
	}

	#[benchmark]
	fn provider_is_block_vote_seal() {
		reset_benchmark_notebook_provider_state();
		reset_benchmark_notebook_provider_call_counters();
		IsBlockFromVoteSeal::<T>::put(true);

		#[block]
		{
			let is_vote_seal =
				<BlockSealPallet<T> as BlockSealerProvider<T::AccountId>>::is_block_vote_seal();
			assert!(is_vote_seal);
		}

		assert_provider_calls(Default::default());
	}

	impl_benchmark_test_suite!(BlockSealPallet, crate::mock::new_test_ext(), crate::mock::Test);
}
