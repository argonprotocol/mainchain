use frame_support::traits::OnTimestampSet;
use pallet_prelude::*;
use polkadot_sdk::sp_core::ed25519::Public;
use sp_core::crypto::AccountId32;

use argon_primitives::{
	MerkleProof, NotaryId, NotebookEventHandler,
	digests::{BLOCK_VOTES_DIGEST_ID, BlockVoteDigest},
	inherents::BlockSealInherent,
	localchain::BlockVote,
	notary::NotaryNotebookVoteDigestDetails,
	notebook::{NotebookHeader, NotebookNumber},
};

use crate::{
	Event, KEY_BLOCK_ROTATION,
	mock::{BlockSealSpec, System, *},
	pallet::{
		CurrentComputeKeyBlock, PastBlockVotes, PastComputeBlockTimes, PreviousBlockTimestamp,
	},
};

#[test]
fn it_will_adjust_minimum() {
	new_test_ext(1_000_000, 100).execute_with(|| {
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
		System::set_block_number(1);

		let start_vote_minimum = BlockSealSpec::vote_minimum();

		BlockSealSpec::update_vote_minimum(11, 2, 0);

		assert_eq!(BlockSealSpec::vote_minimum(), 901_000);
		assert_eq!(PastBlockVotes::<Test>::get(), vec![(11, 2, 0)]);
		System::assert_last_event(
			Event::VoteMinimumAdjusted {
				start_vote_minimum,
				actual_block_votes: 901,
				expected_block_votes: 1000,
				new_vote_minimum: 901_000,
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

		let start_vote_minimum = BlockSealSpec::vote_minimum();

		BlockSealSpec::update_vote_minimum(10, 2, 0);

		assert_eq!(BlockSealSpec::vote_minimum(), start_vote_minimum);
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
		let digest = BlockSealSpec::create_block_vote_digest(2, vec![book1.clone(), book2.clone()]);
		assert_eq!(digest, BlockVoteDigest { voting_power: 30_000, votes_count: 4 });
		// if locked, should not include!!

		LockedNotaries::set(vec![(2, 2)]);
		let digest = BlockSealSpec::create_block_vote_digest(2, vec![book1, book2]);
		assert_eq!(digest, BlockVoteDigest { voting_power: 20_000, votes_count: 1 });
	});
}

#[test]
fn it_checks_the_vote_digest() {
	new_test_ext(100, 10_000_000).execute_with(|| {
		CurrentTick::set(3);
		let mut book1 = create_default_notebook(1, 1, 2);
		book1.block_votes_count = 1;
		book1.block_voting_power = 20_000;
		let mut book2 = create_default_notebook(2, 1, 2);
		book2.block_votes_count = 3;
		book2.block_voting_power = 10_000;
		System::set_block_number(1);
		let digest_details = vec![
			NotaryNotebookVoteDigestDetails::from(&book1),
			NotaryNotebookVoteDigestDetails::from(&book2),
		];
		let digest = BlockSealSpec::create_block_vote_digest(2, digest_details);
		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, digest.encode())] },
		);
		BlockSealSpec::notebook_submitted(&book1);
		BlockSealSpec::notebook_submitted(&book2);

		BlockSealSpec::on_timestamp_set(2);
		BlockSealSpec::on_initialize(2);
		BlockSealSpec::on_finalize(2);

		///// Test with empty set
		System::set_block_number(2);
		System::initialize(
			&3,
			&System::parent_hash(),
			&Digest {
				logs: vec![DigestItem::PreRuntime(
					BLOCK_VOTES_DIGEST_ID,
					BlockSealSpec::create_block_vote_digest(2, Default::default()).encode(),
				)],
			},
		);

		BlockSealSpec::on_timestamp_set(3);
		BlockSealSpec::on_initialize(3);
		BlockSealSpec::on_finalize(3);
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
		BlockSealSpec::calculate_next_vote_minimum(u128::MAX - 500, 1000, 4000, 1, u128::MAX);
	assert_eq!(u128::MAX, actual, "Failed to overflow block_seal_spec");
}

// assume that the current block_seal_spec is 100 and the target window time is 100
fn assert_next_minimum(start_minimum: u64, actual_votes: u64, next_minimum: u64) {
	let next_minimum: u128 = next_minimum.into();
	let actual = BlockSealSpec::calculate_next_vote_minimum(
		start_minimum.into(),
		100,
		actual_votes.into(),
		1,
		10_000,
	);
	assert_eq!(next_minimum, actual, "Failed for actual votes {actual_votes}");
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
		domains: Default::default(),
	}
}

////////// DIFFICULTY TESTS //////////////////////////////////////////////////////

#[test]
fn it_doesnt_adjust_difficulty_until_time() {
	new_test_ext(100, 1000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let start_difficulty = BlockSealSpec::compute_difficulty();

		BlockSealSpec::on_timestamp_set(1);
		BlockSealSpec::on_initialize(1);
		BlockSealSpec::on_finalize(1);

		assert_eq!(BlockSealSpec::compute_difficulty(), start_difficulty);
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
				account_id: AccountId32::new([0u8; 32]),
				index: 1,
				tick: 1,
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
			miner_nonce_score: Some(U256::one()),
		});

		assert_ok!(PastComputeBlockTimes::<Test>::try_mutate(|a| {
			a.try_append(&mut vec![100, 100, 100, 100, 100, 100, 100, 100, 100, 1])
		}));
		let start_difficulty = BlockSealSpec::compute_difficulty();

		BlockSealSpec::on_timestamp_set(1);
		BlockSealSpec::on_initialize(1);
		BlockSealSpec::on_finalize(1);

		assert_eq!(BlockSealSpec::compute_difficulty(), start_difficulty);
		assert_eq!(PastComputeBlockTimes::<Test>::get().len(), 10);
	});
}

#[test]
fn it_doesnt_adjust_difficulty_if_there_are_registered_miners() {
	new_test_ext(100, 1000).execute_with(|| {
		// Go past genesis block so events get deposited

		assert_ok!(PastComputeBlockTimes::<Test>::try_mutate(|a| {
			a.try_append(&mut vec![100, 100, 100, 100, 100, 100, 100, 100, 100, 1])
		}));
		AuthorityList::set(vec![(1, BlockSealAuthorityId::from(Public::from_raw([0; 32])))]);
		let start_difficulty = BlockSealSpec::compute_difficulty();
		for i in 1..100 {
			System::set_block_number(i);
			CurrentSeal::set(BlockSealInherent::Compute);

			BlockSealSpec::on_timestamp_set(i);
			BlockSealSpec::on_initialize(i);
			BlockSealSpec::on_finalize(i);

			assert_eq!(PastComputeBlockTimes::<Test>::get().len(), 10);
			// it should have put new times in, but not adjusted difficulty
			assert_eq!(BlockSealSpec::compute_difficulty(), start_difficulty);
			assert_ne!(
				PastComputeBlockTimes::<Test>::get(),
				vec![100, 100, 100, 100, 100, 100, 100, 100, 100, 1]
			);
		}
	});
}

#[test]
fn it_tracks_the_block_time_for_compute() {
	new_test_ext(100, 1000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		CurrentSeal::set(BlockSealInherent::Compute);

		let start_difficulty = BlockSealSpec::compute_difficulty();
		PreviousBlockTimestamp::<Test>::set(Some(500));

		BlockSealSpec::on_timestamp_set(1000);
		BlockSealSpec::on_initialize(1);
		BlockSealSpec::on_finalize(1);

		assert_eq!(BlockSealSpec::compute_difficulty(), start_difficulty);
		assert_eq!(PastComputeBlockTimes::<Test>::get().into_inner(), vec![500]);
	});
}

#[test]
fn it_will_adjust_difficulty() {
	HistoricalComputeBlocksForAverage::set(10);
	new_test_ext(100, 10_000_000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		// should weed out the outlier
		assert_ok!(PastComputeBlockTimes::<Test>::try_mutate(|a| {
			a.try_append(&mut vec![100, 100, 100, 100, 100, 100, 100, 100, 100, 1])
		}));
		System::set_block_number(10);

		let start_difficulty = BlockSealSpec::compute_difficulty();

		BlockSealSpec::on_timestamp_set(2);
		BlockSealSpec::on_initialize(10);
		BlockSealSpec::on_finalize(10);

		System::assert_last_event(
			Event::ComputeDifficultyAdjusted {
				start_difficulty,
				actual_block_time: 901, // (100s + 1)
				expected_block_time: 1000,
				new_difficulty: 11_098_779,
			}
			.into(),
		);
		assert_ne!(BlockSealSpec::compute_difficulty(), start_difficulty);
		assert_eq!(PastComputeBlockTimes::<Test>::get().len(), 1);
	});
}

#[test]
fn it_adjusts_difficulty_in_new_algorithm() {
	new_test_ext(100, 3_674_680).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		TargetComputeBlockPercent::set(Percent::from_percent(49));
		HistoricalComputeBlocksForAverage::set(360);
		TickDuration::set(60_000);

		let mut entries: Vec<u64> = vec![
			27_604, 2_450, 2_565, 13_103, 9_183, 13_897, 45_231, 24_726, 14_102, 3_850, 3_726,
			6_120, 7_700, 20_000, 7_869, 13_628, 17_743, 61_102, 14_819, 35_899, 4_506, 3_934,
			27_810, 4_417, 1_762, 7_806, 18_558, 5_830, 13_863, 39_317, 15_617, 22_454, 17_467,
			5_624, 14_983, 20_500, 23_704, 61_255, 7_510, 2_067, 5_997, 12_396, 5_107, 25_122,
			14_201, 46_971, 4_250, 39_174, 15_779, 5_506, 16_273, 10_368, 7_933, 21_094, 5_996,
			20_272, 32_909, 2_209, 17_266, 40_703, 4_331, 2_407, 4_601, 2_525, 46_304, 7_088,
			3_179, 9_345, 40_818, 16_436, 15_601, 13_997, 13_943, 3_589, 9_123, 4_164, 16_739,
			9_273, 5_578, 11_754, 13_484, 22_034, 24_026, 4_876, 5_166, 7_115, 3_082, 14_721,
			8_726, 17_122, 2_585, 14_568, 24_447, 16_630, 12_678, 7_262, 11_457, 28_843, 1_548,
			10_201, 7_566, 6_919, 35_205, 13_474, 25_269, 2_806, 16_400, 4_780, 43_495, 12_853,
			9_070, 8_413, 41_677, 19_107, 4_485, 37_622, 1_558, 2_612, 26_092, 28_911, 3_538,
			17_026, 39_644, 6_151, 10_329, 1_924, 18_092, 13_653, 2_025, 1_718, 6_285, 8_813,
			11_839, 15_212, 23_489, 29_346, 20_274, 10_406, 14_876, 45_410, 6_682, 9_307, 16_877,
			27_214, 60_781, 13_152, 45_548, 20_149, 40_029, 10_956, 19_547, 18_390, 1_860, 9_437,
			34_513, 14_553, 11_144, 38_999, 2_461, 18_672, 16_756, 23_387, 20_065, 8_875, 1_623,
			2_921, 16_023, 10_220, 19_542, 15_941, 6_574, 24_519, 14_144, 3_020, 36_257, 19_911,
			2_034, 24_271, 2_213, 31_699, 13_983, 17_974, 3_406, 7_893, 15_942, 7_702, 53_454,
			29_653, 29_552, 7_165, 11_344, 14_687, 28_065, 41_071, 17_075, 61_200, 59_298, 16_160,
			2_413, 42_578, 21_119, 38_058, 60_208, 110_289, 10_168, 28_441, 30_614, 180_575,
			59_201, 61_055, 2_270, 11_688, 45_345, 61_209, 12_547, 46_650, 61_182, 31_437, 27_734,
			5_861, 4_716, 48_641, 28_395, 31_769, 2_247, 21_980, 3_145, 33_802, 6_594, 52_578,
			60_221, 3_319, 55_900, 61_159, 12_133, 16_287, 16_925, 8_471, 5_347, 61_184, 8_187,
			51_017, 60_214, 58_858, 21_198, 40_336, 119_356, 6_706, 52_514, 61_187, 33_451, 25_736,
			145_192, 35_402, 87_995, 32_390, 78_078, 8_882, 20_144, 13_287, 3_110, 56_056, 4_241,
			26_439, 29_541, 59_193, 2_519, 23_905, 33_754, 60_188, 119_669, 7_411, 33_801, 9_799,
			9_248, 60_133, 12_677, 46_527, 120_394, 120_386, 42_036, 18_145, 4_685, 54_505, 22_771,
			38_145, 58_526, 97_042, 23_327, 6_880, 54_250, 59_220, 13_427, 46_790, 14_491, 45_709,
			59_203, 14_601, 10_399, 35_230, 60_163, 60_208, 299_010, 181_616, 59_230, 375_786,
			43_679, 60_214, 60_231, 60_202, 60_224, 60_171, 22_820, 2_962, 34_416, 119_384, 60_660,
			119_942, 59_201, 180_663, 299_948, 64_109, 56_225, 539_467, 59_655, 119_399, 153_325,
			27_262, 60_188, 60_290, 239_854, 77_513, 42_891, 67_792, 50_612, 120_406, 720_668,
			60_119, 60_199, 59_483, 420_217, 60_192, 239_735, 120_442, 59_226, 239_540, 601_285,
			179_629, 7_584, 52_654, 31_112, 29_110, 44_725, 15_584, 59_157, 61_206, 238_820,
			60_158, 239_850, 300_052,
		];
		let count = entries.len() as u32;
		// should weed out the outlier
		assert_ok!(PastComputeBlockTimes::<Test>::try_mutate(|a| { a.try_append(&mut entries) }));
		System::set_block_number(10);

		let start_difficulty = BlockSealSpec::compute_difficulty();

		BlockSealSpec::on_timestamp_set(2);
		BlockSealSpec::on_initialize(10);
		BlockSealSpec::on_finalize(10);

		System::assert_last_event(
			Event::ComputeDifficultyAdjusted {
				start_difficulty,
				actual_block_time: 15293870,
				expected_block_time: TargetComputeBlockPercent::get()
					.mul_ceil(60_000 * count as u64),
				new_difficulty: 2543032, // should be less
			}
			.into(),
		);
		assert_ne!(BlockSealSpec::compute_difficulty(), start_difficulty);
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
	let actual = BlockSealSpec::calculate_next_difficulty(u128::MAX - 500, 1000, 0, 1, u128::MAX);
	assert_eq!(u128::MAX, actual, "Failed to overflow difficulty");
}

#[test]
fn it_changes_key_block_appropriately() {
	new_test_ext(100, 1000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		BlockSealSpec::on_timestamp_set(1);
		BlockSealSpec::on_initialize(1);
		BlockSealSpec::on_finalize(1);
		assert_eq!(CurrentComputeKeyBlock::<Test>::get(), Some(System::block_hash(0)));

		System::initialize(&2, &System::parent_hash(), &Default::default());
		BlockSealSpec::on_timestamp_set(2);
		BlockSealSpec::on_initialize(2);
		BlockSealSpec::on_finalize(2);
		assert_eq!(CurrentComputeKeyBlock::<Test>::get(), Some(System::block_hash(0)));

		let next_rotation = KEY_BLOCK_ROTATION.into();
		System::set_block_number(next_rotation - 1);
		System::initialize(&next_rotation, &System::parent_hash(), &Default::default());
		BlockSealSpec::on_timestamp_set(next_rotation);
		BlockSealSpec::on_initialize(next_rotation);
		BlockSealSpec::on_finalize(next_rotation);
		assert_eq!(CurrentComputeKeyBlock::<Test>::get(), Some(System::block_hash(0)));

		System::initialize(&(next_rotation + 1), &System::parent_hash(), &Default::default());
		BlockSealSpec::on_timestamp_set(next_rotation + 1);
		BlockSealSpec::on_initialize(next_rotation + 1);
		BlockSealSpec::on_finalize(next_rotation + 1);
		assert_eq!(CurrentComputeKeyBlock::<Test>::get(), Some(System::block_hash(next_rotation)));
	});
}

// assume that the current difficulty is 100 and the target window time is 100
fn assert_next_difficulty(start_difficulty: u64, time_observed: u64, next_difficulty: u64) {
	let next_difficulty: u128 = next_difficulty.into();
	let actual = BlockSealSpec::calculate_next_difficulty(
		start_difficulty.into(),
		100,
		time_observed,
		1,
		10000,
	);
	assert_eq!(next_difficulty, actual, "Failed for time_observed {time_observed}");
}
