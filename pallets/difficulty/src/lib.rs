#![feature(slice_take)]
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::OnTimestampSet;
use sp_core::{Get, U256};
use sp_runtime::{traits::UniqueSaturatedInto, SaturatedConversion};
use sp_std::vec::Vec;

pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

type Difficulty = u128;

const MAX_ADJUST_UP: u128 = 4; // Represents 4x adjustment
const MAX_ADJUST_DOWN: u128 = 4; // Represents 1/4 adjustment
const MAX_DIFFICULTY: u128 = u128::MAX;
const MIN_DIFFICULTY: u128 = 1;

/// This pallet adjusts the difficulty after every timestamp is set. The difficulty is adjusted
/// based on the average time it took to mine the last `BlockChangePeriod` blocks. The difficulty
/// is adjusted up or down by a maximum of 4x or 1/4x respectively.
///
/// Difficulty is an average number of hashes that need to be checked in order mine a block.
///
/// To pass the difficulty test: `big endian(hash with nonce) <= U256::max_value / difficulty`.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::pallet_prelude::*;

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// The desired milliseconds per block
		type TargetBlockTime: Get<Self::Moment>;
		/// The frequency for changing difficulty
		#[pallet::constant]
		type BlockChangePeriod: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn timestamps)]
	pub(super) type PastBlockTimestamps<T: Config> =
		StorageValue<_, BoundedVec<T::Moment, T::BlockChangePeriod>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn difficulty)]
	/// The current difficulty of the chain. A nonce must hash to less than U256::Max -
	/// current difficulty. The larger the value, the harder to solve.
	pub(super) type CurrentDifficulty<T: Config> = StorageValue<_, Difficulty, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub initial_difficulty: u128,
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			<CurrentDifficulty<T>>::put(self.initial_difficulty);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		DifficultyAdjusted {
			observed_millis: u128,
			expected_millis: u128,
			start_difficulty: Difficulty,
			new_difficulty: Difficulty,
		},
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
	/// The easing calculation should reduce the difficulty of solving the nonce so that the amount
	/// of average tax revenue per block / active authorities in the system will close a block in 1
	/// minute.
	/// TODO: we need to know the average tax revenue over the trailing X blocks to be able to
	/// 	calculate this.
	pub fn calculate_easing(tax_amount: u128, signatures: u8) -> u128 {
		(tax_amount * 2) + (signatures.saturated_into::<u128>() * 1)
	}

	pub fn calculate_next_difficulty(
		current_difficulty: u128,
		target_time_per_block: u128,
		period: u128,
		actual_block_period_time: u128,
		min_difficulty: u128,
		max_difficulty: u128,
	) -> (Difficulty, u128) {
		let target_time: u128 = target_time_per_block * period;

		// Calculate the adjusted time span.
		let mut adjusted_timespan = match actual_block_period_time {
			x if x < target_time / MAX_ADJUST_DOWN => target_time / MAX_ADJUST_DOWN,
			x if x > target_time * MAX_ADJUST_UP => target_time * MAX_ADJUST_UP,
			x => x,
		};
		// don't divide by 0
		if adjusted_timespan == 0 {
			adjusted_timespan = 1;
		}
		// Compute the next difficulty based on the current difficulty and the ratio of target time
		// to adjusted timespan.
		let mut next_difficulty: u128 = (U256::from(current_difficulty)
			.saturating_mul(U256::from(target_time)) /
			U256::from(adjusted_timespan))
		.unique_saturated_into();

		next_difficulty = next_difficulty.min(max_difficulty).max(min_difficulty);
		(next_difficulty, target_time)
	}

	/// Calculate the observed block period by saturating the difference between each timestamp (as
	/// opposed to the net difference). This should weed out large outliers.
	pub fn observed_block_period(timestamps: Vec<T::Moment>) -> u128 {
		let mut period = 0;
		for i in 1..timestamps.len() {
			let previous: u128 = timestamps[i - 1].unique_saturated_into();
			let current: u128 = timestamps[i].unique_saturated_into();

			let delta = current.saturating_sub(previous);
			period += delta;
		}

		period
	}
}

impl<T: Config> OnTimestampSet<T::Moment> for Pallet<T> {
	fn on_timestamp_set(now: T::Moment) {
		let did_append =
			<PastBlockTimestamps<T>>::try_mutate(|timestamps| timestamps.try_push(now)).ok();

		// if we can still append, keep going
		if let Some(_) = did_append {
			return
		}

		let target_time =
			UniqueSaturatedInto::<u128>::unique_saturated_into(T::TargetBlockTime::get());
		let timestamps = <PastBlockTimestamps<T>>::get();

		let period: u128 = timestamps.len().unique_saturated_into();
		let actual_block_time = Self::observed_block_period(timestamps.into_inner());

		let start_difficulty = Self::difficulty();
		let (difficulty, expected_time) = Self::calculate_next_difficulty(
			start_difficulty,
			target_time,
			period,
			actual_block_time,
			MIN_DIFFICULTY,
			MAX_DIFFICULTY,
		);

		let _ = <PastBlockTimestamps<T>>::try_mutate(|timestamps| {
			timestamps.truncate(0);
			timestamps.try_insert(0, now)
		});
		<CurrentDifficulty<T>>::put(difficulty);

		Pallet::<T>::deposit_event(Event::<T>::DifficultyAdjusted {
			start_difficulty,
			new_difficulty: difficulty,
			expected_millis: expected_time,
			observed_millis: actual_block_time,
		});
	}
}
