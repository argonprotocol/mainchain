#![feature(slice_take)]
#![cfg_attr(not(feature = "std"), no_std)]

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

/// This pallet adjusts the BlockVote Eligibility after every block.
///
/// The VoteMinimum is the Minimum power of a BlockVote the network will accept in a Notebook. For
/// Compute, this means the number of leading zeros. For Tax, it's the milligons of Tax. Minimums
/// are only adjusted based on the votes in the last `BlockChangePeriod` blocks. The seal minimum is
/// adjusted up or down by a maximum of 4x or 1/4x respectively.
///
/// Seal_Minimum is an average number of hashes that need to be checked in order mine a block.
///
/// To pass the seal_minimums test: `big endian(hash with nonce) <= U256::max_value /
/// seal_minimums`.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_core::{H256, U256};
	use sp_runtime::{traits::UniqueSaturatedInto, BoundedBTreeMap, DigestItem};
	use sp_std::vec;

	use ulx_primitives::{
		block_seal::{BlockVotingPower, VoteMinimum},
		digests::{
			BlockSealMinimumsDigest, BlockVoteDigest, NotaryNotebookDigest, SealSource,
			BLOCK_VOTES_DIGEST_ID, NEXT_SEAL_MINIMUMS_DIGEST_ID,
		},
		notebook::{BlockVotingKey, NotebookHeader},
		AuthorityProvider, BlockSealAuthorityId, BlockVotingProvider, ComputeDifficulty, NotaryId,
		NotebookEventHandler, NotebookProvider,
	};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The desired milliseconds per compute block
		type TargetComputeBlockTime: Get<Self::Moment>;

		type AuthorityProvider: AuthorityProvider<
			BlockSealAuthorityId,
			Self::Block,
			Self::AccountId,
		>;

		type NotebookProvider: NotebookProvider;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// The desired votes per block
		#[pallet::constant]
		type TargetBlockVotes: Get<u128>;
		/// The frequency for changing the minimum
		#[pallet::constant]
		type ChangePeriod: Get<u32>;

		type SealType: Get<SealSource>;
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

	/// The calculated parent voting key for a block. Refers to the Notebook BlockVote Revealed
	/// Secret + VotesMerkleRoot of the parent block notebooks.
	#[pallet::storage]
	#[pallet::getter(fn parent_voting_key)]
	pub(super) type ParentVotingKey<T: Config> = StorageValue<_, Option<H256>, ValueQuery>;

	const VOTE_MINIMUM_HISTORY_LEN: u32 = 3;
	/// Keeps the last 3 vote eligibilities. The first one applies to the current block.
	#[pallet::storage]
	pub(super) type VoteMinimumHistory<T: Config> =
		StorageValue<_, BoundedVec<VoteMinimum, ConstU32<VOTE_MINIMUM_HISTORY_LEN>>, ValueQuery>;

	/// Temporary store of the number of votes in the current block.
	#[pallet::storage]
	pub(super) type TempNotebooksByNotary<T: Config> =
		StorageValue<_, BoundedBTreeMap<NotaryId, NotebookHeader, ConstU32<50>>, ValueQuery>;

	/// Temporary store the vote digest
	#[pallet::storage]
	pub(super) type TempBlockVoteDigest<T: Config> = StorageValue<_, BlockVoteDigest, OptionQuery>;

	#[pallet::storage]
	pub(super) type TempSealMinimumsDigest<T: Config> =
		StorageValue<_, BlockSealMinimumsDigest, OptionQuery>;

	#[pallet::storage]
	pub(super) type PastBlockVotes<T: Config> =
		StorageValue<_, BoundedVec<(u32, BlockVotingPower), T::ChangePeriod>, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub initial_compute_difficulty: ComputeDifficulty,
		pub initial_vote_minimum: VoteMinimum,
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
	pub enum Error<T> {}

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
				match log {
					DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, data) => {
						assert!(
							!<TempBlockVoteDigest<T>>::exists(),
							"Block vote digest can only be provided once!"
						);

						let decoded = BlockVoteDigest::decode(&mut data.as_ref());
						if let Some(votes_digest) = decoded.ok() {
							<TempBlockVoteDigest<T>>::put(votes_digest.clone());
						}
					},
					DigestItem::Consensus(NEXT_SEAL_MINIMUMS_DIGEST_ID, data) => {
						assert!(
							!<TempSealMinimumsDigest<T>>::exists(),
							"Block seal minimums digest can only be provided once!"
						);

						let decoded = BlockSealMinimumsDigest::decode(&mut data.as_ref());
						if let Some(vote_eligibility) = decoded.ok() {
							<TempSealMinimumsDigest<T>>::put(vote_eligibility.clone());
						}
					},
					_ => {},
				};
			}

			T::DbWeight::get().reads_writes(3, 3)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			let notebooks_by_notary = <TempNotebooksByNotary<T>>::take();
			let block_votes = Self::create_block_vote_digest(notebooks_by_notary);

			if let Some(included_digest) = <TempBlockVoteDigest<T>>::take() {
				assert_eq!(
					included_digest, block_votes,
					"Calculated block vote digest does not match included digest"
				);
			}

			let now = <TempBlockTimestamp<T>>::take().expect("Timestamp must be set");
			Self::update_compute_difficulty(now);

			Self::update_vote_minimum(block_votes.votes_count, block_votes.voting_power);

			let next_seal_minimums = BlockSealMinimumsDigest {
				vote_minimum: Self::vote_minimum(),
				compute_difficulty: Self::compute_difficulty(),
			};
			<VoteMinimumHistory<T>>::mutate(|specs| {
				if specs.len() >= VOTE_MINIMUM_HISTORY_LEN as usize {
					specs.pop();
				}
				specs.try_insert(0, next_seal_minimums.vote_minimum)
			})
			.expect("VoteMinimumHistory is bounded");

			<ParentVotingKey<T>>::put(block_votes.parent_voting_key);

			if TempSealMinimumsDigest::<T>::exists() {
				let included_digest = <TempSealMinimumsDigest<T>>::take().unwrap();
				assert_eq!(
					included_digest, next_seal_minimums,
					"Calculated seal minimums do not match included digest"
				);
			} else {
				<frame_system::Pallet<T>>::deposit_log(DigestItem::Consensus(
					NEXT_SEAL_MINIMUMS_DIGEST_ID,
					next_seal_minimums.encode(),
				));
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn update_vote_minimum(total_votes: u32, total_voting_power: u128) {
			let did_append =
				<PastBlockVotes<T>>::try_mutate(|x| x.try_push((total_votes, total_voting_power)))
					.ok();
			if did_append.is_some() {
				return
			}

			let period_votes = <PastBlockVotes<T>>::get();
			let target_votes =
				UniqueSaturatedInto::<u128>::unique_saturated_into(T::TargetBlockVotes::get());

			let expected_block_votes = target_votes * period_votes.len() as u128;
			let actual_block_votes = period_votes
				.into_iter()
				.fold(0u128, |votes, (v, _)| votes.saturating_add(v.into()));

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
				x.try_insert(0, (total_votes, total_voting_power))
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

			if T::SealType::get() != SealSource::Compute {
				return
			}

			let now: u64 = UniqueSaturatedInto::<u64>::unique_saturated_into(now);
			let previous: u64 = previous_timestamp
				.map(UniqueSaturatedInto::<u64>::unique_saturated_into)
				.unwrap_or(now);
			let block_period = now.saturating_sub(previous);

			let did_append =
				<PastComputeBlockTimes<T>>::try_mutate(|x| x.try_push(block_period)).ok();

			// if we can still append, keep going
			if let Some(_) = did_append {
				return
			}

			let timestamps = <PastComputeBlockTimes<T>>::get();
			let target_time =
				UniqueSaturatedInto::<u64>::unique_saturated_into(T::TargetComputeBlockTime::get());
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
			notebooks_by_notary: BoundedBTreeMap<NotaryId, NotebookHeader, ConstU32<50>>,
		) -> BlockVoteDigest {
			let mut block_votes = BlockVoteDigest {
				parent_voting_key: None,
				notebook_numbers: Default::default(),
				voting_power: 0,
				votes_count: 0,
			};

			let current_block_number: u32 =
				<frame_system::Pallet<T>>::block_number().unique_saturated_into();
			let parent_block_number = current_block_number - 1;
			let mut parent_voting_keys = vec![];
			for (notary_id, header) in notebooks_by_notary.into_iter() {
				block_votes.votes_count += header.block_votes_count;
				block_votes.voting_power += header.block_voting_power;
				if let Some(parent_secret) = header.parent_secret {
					// NOTE: secret is verified in the notebook pallet
					if let Some((parent_vote_root, _)) =
						T::NotebookProvider::get_eligible_block_votes_root(
							notary_id,
							parent_block_number,
						) {
						parent_voting_keys.push(BlockVotingKey { parent_vote_root, parent_secret });
					}
				}
				let _ = block_votes.notebook_numbers.try_push(NotaryNotebookDigest {
					notary_id,
					notebook_number: header.notebook_number,
				});
			}
			if !parent_voting_keys.is_empty() {
				block_votes.parent_voting_key =
					Some(BlockVotingKey::create_key(parent_voting_keys));
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

			// Compute the next seal_minimums based on the current
			// seal_minimums and the ratio of target votes to adjusted votes.
			let mut next_vote_minimum: u128 = U256::from(current_vote_minimum)
				.saturating_mul(adjusted_votes.into())
				.checked_div(target_period_votes.into())
				.unwrap_or(0.into())
				.unique_saturated_into();

			next_vote_minimum = next_vote_minimum.min(max_vote_minimum).max(min_vote_minimum);
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

			next_difficulty = next_difficulty.min(max_difficulty).max(min_difficulty);
			next_difficulty
		}
	}

	impl<T: Config> NotebookEventHandler for Pallet<T> {
		fn notebook_submitted(header: &NotebookHeader) -> DispatchResult {
			let notary_id = header.notary_id;
			<TempNotebooksByNotary<T>>::try_mutate(|a| a.try_insert(notary_id, header.clone()))
				.expect(
				"TempNotebooksByNotary is bounded. This can't fail unless we have >50 notaries..",
			);

			Ok(())
		}
	}

	impl<T: Config> BlockVotingProvider<T::Block> for Pallet<T> {
		fn grandparent_vote_minimum() -> Option<VoteMinimum> {
			<VoteMinimumHistory<T>>::get().get(0).cloned()
		}

		fn parent_voting_key() -> Option<H256> {
			<ParentVotingKey<T>>::get()
		}
	}
}

impl<T: Config> OnTimestampSet<T::Moment> for Pallet<T> {
	fn on_timestamp_set(now: T::Moment) {
		TempBlockTimestamp::<T>::put(now);
	}
}
