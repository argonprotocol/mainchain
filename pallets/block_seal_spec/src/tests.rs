use codec::Encode;
use frame_support::{
	assert_ok,
	traits::{Len, OnFinalize, OnInitialize, OnTimestampSet},
};
use sp_core::{crypto::AccountId32, ed25519::Signature, H256};
use sp_runtime::{Digest, DigestItem};

use ulx_primitives::{
	digests::{BlockVoteDigest, BLOCK_VOTES_DIGEST_ID},
	inherents::BlockSealInherent,
	localchain::BlockVote,
	notary::NotaryNotebookVoteDigestDetails,
	notebook::{NotebookHeader, NotebookNumber},
	tick::Tick,
	DataDomain, DataTLD, MerkleProof, NotaryId, NotebookEventHandler,
};

use crate::{
	mock::{SealMinimums, System, *},
	pallet::{PastBlockVotes, PastComputeBlockTimes, PreviousBlockTimestamp},
	Event,
};

#[test]
fn it_will_adjust_minimum() {
	new_test_ext(1000, 100).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		assert_ok!(PastBlockVotes::<Test>::try_mutate(|a| {
			a.try_append(&mut vec![
				(1, 100, 0u128),
				(2, 100, 0u128),
				(3, 100, 0u128),
				(4, 100, 0u128),
				(5, 100, 0u128),
				(6, 100, 0u128),
				(7, 100, 0u128),
				(8, 100, 0u128),
				(9, 100, 0u128),
				(10, 1, 0u128),
			])
		}));
		System::set_block_number(2);

		let start_vote_minimum = SealMinimums::vote_minimum();

		SealMinimums::update_vote_minimum(11, 2, 0);

		assert_eq!(SealMinimums::vote_minimum(), 901);
		assert_eq!(PastBlockVotes::<Test>::get(), vec![(11, 2, 0)]);
		System::assert_last_event(
			Event::VoteMinimumAdjusted {
				start_vote_minimum,
				actual_block_votes: 901,
				expected_block_votes: 1000,
				new_vote_minimum: 901,
			}
			.into(),
		);
	});
}

#[test]
fn it_updates_tick_votes_if_not_changed() {
	new_test_ext(1000, 100).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		assert_ok!(PastBlockVotes::<Test>::try_mutate(|a| {
			a.try_append(&mut vec![
				(1, 100, 0u128),
				(2, 100, 0u128),
				(3, 100, 0u128),
				(4, 100, 0u128),
				(5, 100, 0u128),
				(6, 100, 0u128),
				(7, 100, 0u128),
				(8, 100, 0u128),
				(9, 100, 0u128),
				(10, 1, 0u128),
			])
		}));
		System::set_block_number(2);

		let start_vote_minimum = SealMinimums::vote_minimum();

		SealMinimums::update_vote_minimum(10, 2, 0);

		assert_eq!(SealMinimums::vote_minimum(), start_vote_minimum);
		assert_eq!(
			PastBlockVotes::<Test>::get(),
			vec![
				(1, 100, 0u128),
				(2, 100, 0u128),
				(3, 100, 0u128),
				(4, 100, 0u128),
				(5, 100, 0u128),
				(6, 100, 0u128),
				(7, 100, 0u128),
				(8, 100, 0u128),
				(9, 100, 0u128),
				(10, 3, 0u128),
			]
		);
	});
}

#[test]
fn it_creates_a_block_digest() {
	new_test_ext(500, 100).execute_with(|| {
		System::set_block_number(1);
		CurrentTick::set(2);
		let book1 = NotaryNotebookVoteDigestDetails {
			notary_id: 1,
			notebook_number: 1,
			tick: 2,
			block_votes_count: 1,
			block_voting_power: 20_000,
		};
		let book2 = NotaryNotebookVoteDigestDetails {
			notary_id: 2,
			notebook_number: 1,
			tick: 2,
			block_votes_count: 3,
			block_voting_power: 10_000,
		};
		let digest = SealMinimums::create_block_vote_digest(2, vec![book1.clone(), book2.clone()]);
		assert_eq!(digest, BlockVoteDigest { voting_power: 30_000, votes_count: 4 });
		// if locked, should not include!!

		LockedNotaries::set(vec![(2, 2)]);
		let digest = SealMinimums::create_block_vote_digest(2, vec![book1, book2]);
		assert_eq!(digest, BlockVoteDigest { voting_power: 20_000, votes_count: 1 });
	});
}

#[test]
fn it_checks_the_vote_digest() {
	new_test_ext(100, 10_000_000).execute_with(|| {
		CurrentTick::set(2);
		let mut book1 = create_default_notebook(1, 1, 2);
		book1.block_votes_count = 1;
		book1.block_voting_power = 20_000;
		let mut book2 = create_default_notebook(2, 1, 2);
		book2.block_votes_count = 3;
		book2.block_voting_power = 10_000;
		System::set_block_number(2);
		let digest_details = vec![
			NotaryNotebookVoteDigestDetails::from(&book1),
			NotaryNotebookVoteDigestDetails::from(&book2),
		];
		let digest = SealMinimums::create_block_vote_digest(2, digest_details);
		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, digest.encode())] },
		);
		SealMinimums::notebook_submitted(&book1);
		SealMinimums::notebook_submitted(&book2);

		SealMinimums::on_timestamp_set(2);
		SealMinimums::on_initialize(2);
		SealMinimums::on_finalize(2);

		///// Test with empty set
		System::set_block_number(3);
		System::initialize(
			&3,
			&System::parent_hash(),
			&Digest {
				logs: vec![DigestItem::PreRuntime(
					BLOCK_VOTES_DIGEST_ID,
					SealMinimums::create_block_vote_digest(2, Default::default()).encode(),
				)],
			},
		);

		SealMinimums::on_timestamp_set(3);
		SealMinimums::on_initialize(3);
		SealMinimums::on_finalize(3);
	});
}

#[test]
fn it_calculates_next_vote_minimum() {
	// clamped
	assert_next_minimum(100, 0, 25);
	assert_next_minimum(25, 0, 6);
	assert_next_minimum(1, 0, 1);
	assert_next_minimum(100, 1, 25);
	assert_next_minimum(100, 25, 25);
	assert_next_minimum(100, 26, 26);
	assert_next_minimum(100, 50, 50);
	assert_next_minimum(100, 100, 100);
	assert_next_minimum(100, 200, 200);
	// clamped
	assert_next_minimum(100, 5_000, 400);
	assert_next_minimum(100, 10_000, 400);
}

#[test]
fn it_handles_overflowing_minimum() {
	new_test_ext(1, 0);
	let actual =
		SealMinimums::calculate_next_vote_minimum(u128::MAX - 500, 1000, 4000, 1, u128::MAX);
	assert_eq!(u128::MAX, actual, "Failed to overflow block_seal_spec");
}

// assume that the current block_seal_spec is 100 and the target window time is 100
fn assert_next_minimum(start_minimum: u64, actual_votes: u64, next_minimum: u64) {
	let next_minimum: u128 = next_minimum.into();
	let actual = SealMinimums::calculate_next_vote_minimum(
		start_minimum.into(),
		100,
		actual_votes.into(),
		1,
		10_000,
	);
	assert_eq!(next_minimum, actual, "Failed for actual votes {}", actual_votes);
}

fn create_default_notebook(
	notary_id: NotaryId,
	notebook_number: NotebookNumber,
	tick: Tick,
) -> NotebookHeader {
	NotebookHeader {
		version: 1,
		notary_id,
		notebook_number,
		tick,
		changed_accounts_root: Default::default(),
		chain_transfers: Default::default(),
		changed_account_origins: Default::default(),
		tax: 0,
		// Block Votes
		parent_secret: None,
		secret_hash: H256::from_slice(&[0u8; 32]),
		block_voting_power: 1,
		block_votes_root: H256::from_slice(&[0u8; 32]),
		block_votes_count: 1,
		blocks_with_votes: Default::default(),
		data_domains: Default::default(),
	}
}

////////// DIFFICULTY TESTS //////////////////////////////////////////////////////

#[test]
fn it_doesnt_adjust_difficulty_until_time() {
	new_test_ext(100, 1000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let start_difficulty = SealMinimums::compute_difficulty();

		SealMinimums::on_timestamp_set(1);
		SealMinimums::on_initialize(1);
		SealMinimums::on_finalize(1);

		assert_eq!(SealMinimums::compute_difficulty(), start_difficulty);
		assert_eq!(PastComputeBlockTimes::<Test>::get().len(), 1);
	});
}

#[test]
fn it_doesnt_adjust_difficulty_if_tax_block() {
	new_test_ext(100, 1000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		CurrentSeal::set(BlockSealInherent::Vote {
			notary_id: 1,
			block_vote: BlockVote {
				block_hash: System::block_hash(System::block_number().saturating_sub(4)),
				data_domain_account: AccountId32::new([0u8; 32]),
				data_domain_hash: DataDomain::new("test", DataTLD::Automotive).hash(),
				account_id: AccountId32::new([0u8; 32]),
				index: 1,
				power: 500,
				signature: sp_core::sr25519::Signature::from_raw([0u8; 64]).into(),
				block_rewards_account_id: AccountId32::new([0u8; 32]),
			},
			seal_strength: 1.into(),
			source_notebook_proof: MerkleProof {
				proof: Default::default(),
				number_of_leaves: 1,
				leaf_index: 0,
			},
			source_notebook_number: 1,
			miner_signature: Signature::from_raw([0u8; 64]).into(),
		});

		assert_ok!(PastComputeBlockTimes::<Test>::try_mutate(|a| {
			a.try_append(&mut vec![100, 100, 100, 100, 100, 100, 100, 100, 100, 1])
		}));
		let start_difficulty = SealMinimums::compute_difficulty();

		SealMinimums::on_timestamp_set(1);
		SealMinimums::on_initialize(1);
		SealMinimums::on_finalize(1);

		assert_eq!(SealMinimums::compute_difficulty(), start_difficulty);
		assert_eq!(PastComputeBlockTimes::<Test>::get().len(), 10);
	});
}

#[test]
fn it_tracks_the_block_time_for_compute() {
	new_test_ext(100, 1000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		CurrentSeal::set(BlockSealInherent::Compute);

		let start_difficulty = SealMinimums::compute_difficulty();
		PreviousBlockTimestamp::<Test>::set(Some(500));

		SealMinimums::on_timestamp_set(1000);
		SealMinimums::on_initialize(1);
		SealMinimums::on_finalize(1);

		assert_eq!(SealMinimums::compute_difficulty(), start_difficulty);
		assert_eq!(PastComputeBlockTimes::<Test>::get().into_inner(), vec![500]);
	});
}

#[test]
fn it_will_adjust_difficulty() {
	new_test_ext(100, 10_000_000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		assert_ok!(PastComputeBlockTimes::<Test>::try_mutate(|a| {
			a.try_append(&mut vec![100, 100, 100, 100, 100, 100, 100, 100, 100, 1])
		}));
		System::set_block_number(2);

		let start_difficulty = SealMinimums::compute_difficulty();

		SealMinimums::on_timestamp_set(2);
		SealMinimums::on_initialize(1);
		SealMinimums::on_finalize(1);

		System::assert_last_event(
			Event::ComputeDifficultyAdjusted {
				start_difficulty,
				actual_block_time: 901,
				expected_block_time: 1000,
				new_difficulty: 11_098_779,
			}
			.into(),
		);
		assert_ne!(SealMinimums::compute_difficulty(), start_difficulty);
		assert_eq!(PastComputeBlockTimes::<Test>::get().len(), 1);
	});
}

#[test]
fn it_calculates_next_difficulty() {
	// clamped
	assert_next_difficulty(100, 0, 400);
	assert_next_difficulty(100, 1, 400);
	assert_next_difficulty(100, 25, 400);
	assert_next_difficulty(100, 26, 384);
	assert_next_difficulty(100, 50, 200);
	assert_next_difficulty(100, 100, 100);
	assert_next_difficulty(100, 200, 50);
	// clamped
	assert_next_difficulty(100, 5_000, 25);
	assert_next_difficulty(100, 10_000, 25);
}

#[test]
fn it_handles_overflowing_difficulty() {
	new_test_ext(0, 1);
	let actual = SealMinimums::calculate_next_difficulty(u128::MAX - 500, 1000, 0, 1, u128::MAX);
	assert_eq!(u128::MAX, actual, "Failed to overflow difficulty");
}

// assume that the current difficulty is 100 and the target window time is 100
fn assert_next_difficulty(start_difficulty: u64, time_observed: u64, next_difficulty: u64) {
	let next_difficulty: u128 = next_difficulty.into();
	let actual = SealMinimums::calculate_next_difficulty(
		start_difficulty.into(),
		100,
		time_observed,
		1,
		10000,
	);
	assert_eq!(next_difficulty, actual, "Failed for time_observed {}", time_observed);
}
