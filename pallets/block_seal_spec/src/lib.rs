#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

extern crate core;

use frame_support::traits::OnTimestampSet;

pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

const MAX_ADJUST_UP: u128 = 4; // Represents 4x adjustment
const MAX_ADJUST_DOWN: u128 = 4; // Represents 1/4 adjustment
const MAX_COMPUTE_DIFFICULTY: u128 = u128::MAX;
const MIN_COMPUTE_DIFFICULTY: u128 = 4;
const MAX_TAX_MINIMUM: u128 = u128::MAX;
const MIN_TAX_MINIMUM: u128 = 500;

/// This pallet adjusts the BlockSeal Specification after every block for both voting and compute.
///
/// The VoteMinimum is the Minimum power of a BlockVote the network will accept in a Notebook. For
/// Compute, this means the number of leading zeros. For Tax, it's the milligons of Tax. Minimums
/// are only adjusted based on the votes in the last `BlockChangePeriod` blocks. The seal minimum is
/// adjusted up or down by a maximum of 4x or 1/4x respectively.
///
/// ComputeDifficulty is an average number of hashes that need to be checked in order mine a block.
/// - To pass the difficulty test: `big endian(hash with nonce) <= U256::max_value / difficulty`.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use alloc::{vec, vec::Vec};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_core::U256;
	use sp_runtime::{traits::UniqueSaturatedInto, DigestItem, Percent};

	use argon_primitives::{
		block_vote::VoteMinimum,
		digests::{BlockVoteDigest, BLOCK_VOTES_DIGEST_ID},
		inherents::BlockSealInherent,
		notary::NotaryNotebookVoteDigestDetails,
		notebook::{BlockVotingPower, NotebookHeader},
		tick::Tick,
		AuthorityProvider, BlockSealAuthorityId, BlockVotingProvider, ComputeDifficulty,
		NotebookEventHandler, NotebookProvider,
	};

	use super::*;
	use argon_primitives::TickProvider;
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The desired milliseconds per compute block
		type TargetComputeBlockPercent: Get<Percent>;

		type AuthorityProvider: AuthorityProvider<
			BlockSealAuthorityId,
			Self::Block,
			Self::AccountId,
		>;

		/// The maximum active notaries allowed
		#[pallet::constant]
		type MaxActiveNotaries: Get<u32>;

		type NotebookProvider: NotebookProvider;
		type TickProvider: TickProvider<Self::Block>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// The desired votes per block
		#[pallet::constant]
		type TargetBlockVotes: Get<u128>;
		/// The frequency for changing the minimum
		#[pallet::constant]
		type ChangePeriod: Get<u32>;

		type SealInherent: Get<BlockSealInherent>;
	}

	#[pallet::storage]
	#[pallet::getter(fn vote_minimum)]
	/// The current vote minimum of the chain. Block votes use this minimum to determine the
	/// minimum amount of tax or compute needed to create a vote. It is adjusted up or down to
	/// target a max number of votes
	pub(super) type CurrentVoteMinimum<T: Config> = StorageValue<_, VoteMinimum, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn compute_difficulty)]
	/// The current vote minimum of the chain. Block votes use this minimum to determine the
	/// minimum amount of tax or compute needed to create a vote. It is adjusted up or down to
	/// target a max number of votes
	pub(super) type CurrentComputeDifficulty<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	pub(super) type PastComputeBlockTimes<T: Config> =
		StorageValue<_, BoundedVec<u64, T::ChangePeriod>, ValueQuery>;
	#[pallet::storage]
	pub(super) type PreviousBlockTimestamp<T: Config> = StorageValue<_, T::Moment, OptionQuery>;

	#[pallet::storage]
	pub(super) type TempBlockTimestamp<T: Config> = StorageValue<_, T::Moment, OptionQuery>;

	const VOTE_MINIMUM_HISTORY_LEN: u32 = 3;
	/// Keeps the last 3 vote minimums. The first one applies to the current block.
	#[pallet::storage]
	pub(super) type VoteMinimumHistory<T: Config> =
		StorageValue<_, BoundedVec<VoteMinimum, ConstU32<VOTE_MINIMUM_HISTORY_LEN>>, ValueQuery>;

	/// Temporary store of any current tick notebooks included in this block (vs tick)
	#[pallet::storage]
	pub(super) type TempCurrentTickNotebooksInBlock<T: Config> = StorageValue<
		_,
		BoundedVec<NotaryNotebookVoteDigestDetails, T::MaxActiveNotaries>,
		ValueQuery,
	>;

	/// Temporary store the vote digest
	#[pallet::storage]
	pub(super) type TempBlockVoteDigest<T: Config> = StorageValue<_, BlockVoteDigest, OptionQuery>;

	#[pallet::storage]
	pub(super) type PastBlockVotes<T: Config> =
		StorageValue<_, BoundedVec<(Tick, u32, BlockVotingPower), T::ChangePeriod>, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub initial_compute_difficulty: ComputeDifficulty,
		pub initial_vote_minimum: VoteMinimum,
		#[serde(skip)]
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			<CurrentComputeDifficulty<T>>::put(self.initial_compute_difficulty);
			<CurrentVoteMinimum<T>>::put(self.initial_vote_minimum);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		VoteMinimumAdjusted {
			expected_block_votes: u128,
			actual_block_votes: u128,
			start_vote_minimum: VoteMinimum,
			new_vote_minimum: VoteMinimum,
		},
		ComputeDifficultyAdjusted {
			expected_block_time: u64,
			actual_block_time: u64,
			start_difficulty: ComputeDifficulty,
			new_difficulty: ComputeDifficulty,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The maximum number of notebooks at the current tick has been exceeded
		MaxNotebooksAtTickExceeded,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn configure(
			origin: OriginFor<T>,
			vote_minimum: Option<VoteMinimum>,
			compute_difficulty: Option<u128>,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(minimum) = vote_minimum {
				<CurrentVoteMinimum<T>>::put(minimum);
			}
			if let Some(difficulty) = compute_difficulty {
				<CurrentComputeDifficulty<T>>::put(difficulty);
			}
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let digest = <frame_system::Pallet<T>>::digest();
			for log in digest.logs.iter() {
				if let DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, data) = log {
					assert!(
						!<TempBlockVoteDigest<T>>::exists(),
						"Block vote digest can only be provided once!"
					);

					let decoded = BlockVoteDigest::decode(&mut data.as_ref());
					if let Ok(votes_digest) = decoded {
						<TempBlockVoteDigest<T>>::put(votes_digest.clone());
					}
				}
			}

			T::DbWeight::get().reads_writes(3, 3)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			let block_notebooks = <TempCurrentTickNotebooksInBlock<T>>::take();
			let tick = T::TickProvider::current_tick();

			let block_votes = Self::create_block_vote_digest(tick, block_notebooks.to_vec());

			if let Some(included_digest) = <TempBlockVoteDigest<T>>::take() {
				assert_eq!(
					included_digest, block_votes,
					"Calculated block vote digest does not match included digest"
				);
			}

			let now = <TempBlockTimestamp<T>>::take().expect("Timestamp must be set");
			Self::update_compute_difficulty(now);

			Self::update_vote_minimum(tick, block_votes.votes_count, block_votes.voting_power);

			<VoteMinimumHistory<T>>::mutate(|specs| {
				if specs.is_full() {
					specs.pop();
				}
				specs.try_insert(0, Self::vote_minimum())
			})
			.expect("VoteMinimumHistory is bounded");
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn update_vote_minimum(tick: Tick, total_votes: u32, total_voting_power: u128) {
			let did_append = <PastBlockVotes<T>>::try_mutate(|x| {
				if let Some(entry) = x.last_mut() {
					if entry.0 == tick {
						entry.1 = entry.1.saturating_add(total_votes);
						entry.2 = entry.2.saturating_add(total_voting_power);
						return Ok(());
					}
				}
				x.try_push((tick, total_votes, total_voting_power))
			})
			.ok();
			if did_append.is_some() {
				return;
			}

			let period_votes = <PastBlockVotes<T>>::get();
			let target_votes =
				UniqueSaturatedInto::<u128>::unique_saturated_into(T::TargetBlockVotes::get());

			let expected_block_votes = target_votes * period_votes.len() as u128;
			let actual_block_votes = period_votes
				.into_iter()
				.fold(0u128, |votes, (_, v, _)| votes.saturating_add(v.into()));

			let start_vote_minimum = Self::vote_minimum();
			let vote_minimum = Self::calculate_next_vote_minimum(
				start_vote_minimum,
				expected_block_votes,
				actual_block_votes,
				MIN_TAX_MINIMUM,
				MAX_TAX_MINIMUM,
			);

			let _ = <PastBlockVotes<T>>::try_mutate(|x| {
				x.truncate(0);
				x.try_insert(0, (tick, total_votes, total_voting_power))
			});
			if start_vote_minimum != vote_minimum {
				<CurrentVoteMinimum<T>>::put(vote_minimum);

				Pallet::<T>::deposit_event(Event::<T>::VoteMinimumAdjusted {
					start_vote_minimum,
					new_vote_minimum: vote_minimum,
					expected_block_votes,
					actual_block_votes,
				});
			}
		}

		pub(crate) fn update_compute_difficulty(now: T::Moment) {
			let previous_timestamp = <PreviousBlockTimestamp<T>>::take();
			<PreviousBlockTimestamp<T>>::put(now);

			if T::SealInherent::get() != BlockSealInherent::Compute {
				return;
			}

			let now: u64 = UniqueSaturatedInto::<u64>::unique_saturated_into(now);
			let previous: u64 = previous_timestamp
				.map(UniqueSaturatedInto::<u64>::unique_saturated_into)
				.unwrap_or(now);
			let block_period = now.saturating_sub(previous);

			let did_append =
				<PastComputeBlockTimes<T>>::try_mutate(|x| x.try_push(block_period)).ok();

			// if we can still append, keep going
			if did_append.is_some() {
				return;
			}

			let timestamps = <PastComputeBlockTimes<T>>::get();
			let tick_millis = T::TickProvider::ticker().tick_duration_millis;
			let target_time = T::TargetComputeBlockPercent::get().mul_floor(tick_millis);
			let expected_block_time = target_time * timestamps.len() as u64;
			let actual_block_time =
				timestamps.into_iter().fold(0u64, |sum, time| sum.saturating_add(time));

			let start_difficulty = Self::compute_difficulty();
			let difficulty = Self::calculate_next_difficulty(
				start_difficulty,
				expected_block_time,
				actual_block_time,
				MIN_COMPUTE_DIFFICULTY,
				MAX_COMPUTE_DIFFICULTY,
			);

			let _ = <PastComputeBlockTimes<T>>::try_mutate(|timestamps| {
				timestamps.truncate(0);
				timestamps.try_insert(0, now)
			});
			if start_difficulty != difficulty {
				<CurrentComputeDifficulty<T>>::put(difficulty);

				Pallet::<T>::deposit_event(Event::<T>::ComputeDifficultyAdjusted {
					start_difficulty,
					new_difficulty: difficulty,
					expected_block_time,
					actual_block_time,
				});
			}
		}

		pub fn create_block_vote_digest(
			tick: Tick,
			included_notebooks: Vec<NotaryNotebookVoteDigestDetails>,
		) -> BlockVoteDigest {
			let mut block_votes = BlockVoteDigest { voting_power: 0, votes_count: 0 };

			for header in included_notebooks {
				if header.tick != tick {
					continue;
				}
				if !T::NotebookProvider::is_notary_locked_at_tick(header.notary_id, header.tick) {
					block_votes.votes_count += header.block_votes_count;
					block_votes.voting_power += header.block_voting_power;
				}
			}

			block_votes
		}

		pub fn calculate_next_vote_minimum(
			current_vote_minimum: VoteMinimum,
			target_period_votes: u128,
			actual_period_votes: u128,
			min_vote_minimum: VoteMinimum,
			max_vote_minimum: VoteMinimum,
		) -> VoteMinimum {
			// Calculate the adjusted time span.
			let mut adjusted_votes = match actual_period_votes {
				x if x < target_period_votes / MAX_ADJUST_DOWN =>
					target_period_votes / MAX_ADJUST_DOWN,
				x if x > target_period_votes * MAX_ADJUST_UP => target_period_votes * MAX_ADJUST_UP,
				x => x,
			};
			// don't divide by 0
			if adjusted_votes == 0 {
				adjusted_votes = 1;
			}

			// Compute the next block_seal_spec based on the current
			// block_seal_spec and the ratio of target votes to adjusted votes.
			let mut next_vote_minimum: u128 = U256::from(current_vote_minimum)
				.saturating_mul(adjusted_votes.into())
				.checked_div(target_period_votes.into())
				.unwrap_or(0.into())
				.unique_saturated_into();

			next_vote_minimum = next_vote_minimum.clamp(min_vote_minimum, max_vote_minimum);
			next_vote_minimum
		}

		pub fn calculate_next_difficulty(
			current_difficulty: ComputeDifficulty,
			target_period_time: u64,
			actual_block_period_time: u64,
			min_difficulty: ComputeDifficulty,
			max_difficulty: ComputeDifficulty,
		) -> ComputeDifficulty {
			let target_period_time = target_period_time as u128;
			let actual_block_period_time = actual_block_period_time as u128;
			// Calculate the adjusted time span.
			let mut adjusted_timespan = match actual_block_period_time {
				x if x < target_period_time / MAX_ADJUST_DOWN =>
					target_period_time / MAX_ADJUST_DOWN,
				x if x > target_period_time * MAX_ADJUST_UP => target_period_time * MAX_ADJUST_UP,
				x => x,
			};
			// don't divide by 0
			if adjusted_timespan == 0 {
				adjusted_timespan = 1;
			}
			// Compute the next difficulty based on the current difficulty and the ratio of target
			// time to adjusted timespan.
			let mut next_difficulty: u128 = (U256::from(current_difficulty)
				.saturating_mul(U256::from(target_period_time)) /
				U256::from(adjusted_timespan))
			.unique_saturated_into();

			next_difficulty = next_difficulty.clamp(min_difficulty, max_difficulty);
			next_difficulty
		}
	}

	impl<T: Config> NotebookEventHandler for Pallet<T> {
		fn notebook_submitted(header: &NotebookHeader) {
			let current_tick = T::TickProvider::current_tick();
			if header.tick == current_tick {
				let digest_details = header.into();
				<TempCurrentTickNotebooksInBlock<T>>::try_mutate(|a| a.try_push(digest_details))
					.expect(
						"MaxActiveNotaries is a bound. If this is exceeded, something is wrong.",
					);
			}
		}
	}

	impl<T: Config> BlockVotingProvider<T::Block> for Pallet<T> {
		fn grandparent_vote_minimum() -> Option<VoteMinimum> {
			<VoteMinimumHistory<T>>::get().first().cloned()
		}
	}
}

impl<T: Config> OnTimestampSet<T::Moment> for Pallet<T> {
	fn on_timestamp_set(now: T::Moment) {
		TempBlockTimestamp::<T>::put(now);
	}
}
