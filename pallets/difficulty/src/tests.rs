use frame_support::{assert_ok, traits::OnTimestampSet};
use sp_runtime::SaturatedConversion;

use crate::{mock::*, Event, PastBlockTimestamps};

#[test]
fn it_doesnt_adjust_difficulty_until_time() {
	new_test_ext(1000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let start_difficulty = DifficultyModule::difficulty();

		DifficultyModule::on_timestamp_set(1);

		assert_eq!(DifficultyModule::difficulty(), start_difficulty);
		assert_eq!(DifficultyModule::timestamps().len(), 1);
	});
}

#[test]
fn it_will_adjust_difficulty() {
	new_test_ext(10_000_000).execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		assert_ok!(PastBlockTimestamps::<Test>::try_mutate(|a| {
			a.try_append(&mut vec![
				1_000_000_u64,
				1_000_100_u64,
				1_000_200_u64,
				1_000_300_u64,
				1_000_400_u64,
				1_000_500_u64,
				1_000_600_u64,
				1_000_700_u64,
				1_000_800_u64,
				1_000_801_u64,
			])
		}));
		System::set_block_number(2);

		let start_difficulty = DifficultyModule::difficulty();

		DifficultyModule::on_timestamp_set(1_000_802_u64);

		System::assert_last_event(
			Event::DifficultyAdjusted {
				start_difficulty,
				observed_millis: 801,
				expected_millis: 1000,
				new_difficulty: 12_484_394,
			}
			.into(),
		);
		assert_ne!(DifficultyModule::difficulty(), start_difficulty);
		assert_eq!(DifficultyModule::timestamps().len(), 1);
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
	new_test_ext(1);
	let (actual, _) =
		DifficultyModule::calculate_next_difficulty(u128::MAX - 500, 1, 1000, 0, 1, u128::MAX);
	assert_eq!(u128::MAX, actual, "Failed to overflow difficulty");
}

// assume that the current difficulty is 100 and the target window time is 100
fn assert_next_difficulty(start_difficulty: u64, time_observed: u64, next_difficulty: u64) {
	let time_observed = time_observed.saturated_into::<u128>();
	let next_difficulty: u128 = next_difficulty.into();
	let (actual, _) = DifficultyModule::calculate_next_difficulty(
		start_difficulty.into(),
		1,
		100,
		time_observed,
		1,
		10000,
	);
	assert_eq!(next_difficulty, actual, "Failed for time_observed {}", time_observed);
}
