#![feature(slice_take)]
#![cfg_attr(not(feature = "std"), no_std)]

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
const MAX_COMPUTE_MINIMUM: u128 = 500;
const MIN_COMPUTE_MINIMUM: u128 = 2;
const MAX_TAX_MINIMUM: u128 = u128::MAX;
const MIN_TAX_MINIMUM: u128 = 500;

/// This pallet adjusts the BlockVote Eligibility after every block.
///
/// The VotingMinimum is the Minimum power of a BlockVote the network will accept in a Notebook. For
/// Compute, this means the number of leading zeros. For Tax, it's the milligons of Tax. Minimums
/// are only adjusted based on the votes in the last `BlockChangePeriod` blocks. The seal minimum is
/// adjusted up or down by a maximum of 4x or 1/4x respectively.
///
/// Seal_Minimum is an average number of hashes that need to be checked in order mine a block.
///
/// To pass the vote_eligibility test: `big endian(hash with nonce) <= U256::max_value /
/// vote_eligibility`.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_core::{H256, U256};
	use sp_runtime::{traits::UniqueSaturatedInto, BoundedBTreeMap, DigestItem};
	use sp_std::vec;
	use ulx_primitives::{
		block_seal::{BlockVoteEligibility, BlockVotingPower, VotingMinimum},
		digests::{
			BlockVoteDigest, BlockVoteSource, NotaryNotebookDigest, BLOCK_VOTES_DIGEST_ID,
			NEXT_VOTE_ELIGIBILITY_DIGEST_ID,
		},
		notebook::{BlockVotingKey, NotebookHeader},
		AuthorityProvider, BlockSealAuthorityId, BlockVotingProvider, NotaryId,
		NotebookEventHandler, NotebookProvider,
	};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

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
		type VotingMinimumChangePeriod: Get<u32>;
	}

	/// How many authorities need to be registered before we activate the Proof of Tax era.
	#[pallet::storage]
	pub(super) type MiningSlotCountInitiatingTaxProof<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn allowed_vote_source)]
	pub(super) type AllowedVoteSource<T: Config> = StorageValue<_, BlockVoteSource, ValueQuery>;
	#[pallet::storage]
	#[pallet::getter(fn voting_minimum)]
	/// The current vote minimum of the chain. Block votes use this minimum to determine the
	/// minimum amount of tax or compute needed to create a vote. It is adjusted up or down to
	/// target a max number of votes
	pub(super) type CurrentVotingMinimum<T: Config> = StorageValue<_, VotingMinimum, ValueQuery>;

	/// The calculated parent voting key for a block. Refers to the Notebook BlockVote Revealed
	/// Secret + VotesMerkleRoot of the parent block notebooks.
	#[pallet::storage]
	#[pallet::getter(fn parent_voting_key)]
	pub(super) type ParentVotingKey<T: Config> = StorageValue<_, Option<H256>, ValueQuery>;

	const VOTE_ELIGIBILITY_HISTORY_LEN: u32 = 3;
	/// Keeps the last 3 seal specifications. The first one applies to the current block.
	#[pallet::storage]
	pub(super) type VoteEligibilityHistory<T: Config> = StorageValue<
		_,
		BoundedVec<BlockVoteEligibility, ConstU32<VOTE_ELIGIBILITY_HISTORY_LEN>>,
		ValueQuery,
	>;

	/// Temporary store of the number of votes in the current block.
	#[pallet::storage]
	pub(super) type TempNotebooksByNotary<T: Config> =
		StorageValue<_, BoundedBTreeMap<NotaryId, NotebookHeader, ConstU32<50>>, ValueQuery>;

	/// Temporary store the vote digest
	#[pallet::storage]
	pub(super) type TempBlockVoteDigest<T: Config> = StorageValue<_, BlockVoteDigest, OptionQuery>;

	#[pallet::storage]
	pub(super) type TempVoteEligibilityDigest<T: Config> =
		StorageValue<_, BlockVoteEligibility, OptionQuery>;

	#[pallet::storage]
	pub(super) type PastBlockVotes<T: Config> = StorageValue<
		_,
		BoundedVec<(u32, BlockVotingPower), T::VotingMinimumChangePeriod>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub mining_slot_count_starting_tax_proof: u32,
		pub initial_voting_minimum: u128,
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			<MiningSlotCountInitiatingTaxProof<T>>::put(self.mining_slot_count_starting_tax_proof);
			<CurrentVotingMinimum<T>>::put(self.initial_voting_minimum);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		VotingMinimumAdjusted {
			expected_block_votes: u128,
			actual_block_votes: u128,
			start_voting_minimum: VotingMinimum,
			new_voting_minimum: VotingMinimum,
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
			mining_slot_count_starting_tax_proof: Option<u32>,
			voting_minimum: Option<VotingMinimum>,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(mining_slot_count_starting_tax_proof) = mining_slot_count_starting_tax_proof
			{
				<MiningSlotCountInitiatingTaxProof<T>>::put(mining_slot_count_starting_tax_proof);
			}
			if let Some(minimum) = voting_minimum {
				<CurrentVotingMinimum<T>>::put(minimum);
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
					DigestItem::Consensus(NEXT_VOTE_ELIGIBILITY_DIGEST_ID, data) => {
						assert!(
							!<TempVoteEligibilityDigest<T>>::exists(),
							"Vote eligibility digest can only be provided once!"
						);

						let decoded = BlockVoteEligibility::decode(&mut data.as_ref());
						if let Some(vote_eligibility) = decoded.ok() {
							<TempVoteEligibilityDigest<T>>::put(vote_eligibility.clone());
						}
					},
					_ => {},
				};
			}

			T::DbWeight::get().reads_writes(3, 3)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			let did_transition_to_tax = Self::update_allowed_source();
			let notebooks_by_notary = <TempNotebooksByNotary<T>>::take();
			let block_votes = Self::create_block_vote_digest(notebooks_by_notary);

			if let Some(included_digest) = <TempBlockVoteDigest<T>>::take() {
				assert_eq!(
					included_digest, block_votes,
					"Calculated block vote digest does not match included digest"
				);
			}

			Self::update_voting_minimum(
				did_transition_to_tax,
				block_votes.votes_count,
				block_votes.voting_power,
			);

			let next_vote_eligibility = Self::vote_eligibility();
			<VoteEligibilityHistory<T>>::mutate(|specs| {
				if specs.len() >= VOTE_ELIGIBILITY_HISTORY_LEN as usize {
					specs.pop();
				}
				specs.try_insert(0, next_vote_eligibility.clone())
			})
			.expect("VoteEligibilityHistory is bounded");

			<ParentVotingKey<T>>::put(block_votes.parent_voting_key);

			if TempVoteEligibilityDigest::<T>::exists() {
				let included_vote_eligibility = <TempVoteEligibilityDigest<T>>::take().unwrap();
				assert_eq!(
					included_vote_eligibility, next_vote_eligibility,
					"Calculated vote eligibility does not match included digest"
				);
			} else {
				<frame_system::Pallet<T>>::deposit_log(DigestItem::Consensus(
					NEXT_VOTE_ELIGIBILITY_DIGEST_ID,
					next_vote_eligibility.encode(),
				));
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn update_voting_minimum(
			did_transition_to_tax: bool,
			total_votes: u32,
			total_voting_power: u128,
		) {
			let mut period_votes = <PastBlockVotes<T>>::take();
			if period_votes.len() as u32 >= T::VotingMinimumChangePeriod::get() {
				let target_votes =
					UniqueSaturatedInto::<u128>::unique_saturated_into(T::TargetBlockVotes::get());

				let expected_block_votes = target_votes * period_votes.len() as u128;
				let (actual_block_votes, _) =
					period_votes.iter().fold((0u128, 0), |(votes, power), (v, p)| {
						(votes.saturating_add((*v).into()), power + p)
					});
				period_votes.clear();

				let minimum_range = match AllowedVoteSource::<T>::get() {
					BlockVoteSource::Compute => (MIN_COMPUTE_MINIMUM, MAX_COMPUTE_MINIMUM),
					BlockVoteSource::Tax => (MIN_TAX_MINIMUM, MAX_TAX_MINIMUM),
				};

				let start_voting_minimum = Self::voting_minimum();
				let mut voting_minimum = Self::calculate_next_voting_minimum(
					start_voting_minimum,
					expected_block_votes,
					actual_block_votes,
					minimum_range.0,
					minimum_range.1,
				);

				if did_transition_to_tax {
					voting_minimum = MIN_TAX_MINIMUM;
				}

				if start_voting_minimum != voting_minimum {
					<CurrentVotingMinimum<T>>::put(voting_minimum);

					Pallet::<T>::deposit_event(Event::<T>::VotingMinimumAdjusted {
						start_voting_minimum,
						new_voting_minimum: voting_minimum,
						expected_block_votes,
						actual_block_votes,
					});
				}
			}

			if did_transition_to_tax {
				period_votes.clear();
			} else {
				// past block votes should be cleared at this point
				let _ = period_votes.try_push((total_votes, total_voting_power));
			}
			<PastBlockVotes<T>>::put(period_votes);
		}

		pub fn update_allowed_source() -> bool {
			// check if we have enough authorities to begin tax proof (one-way)
			if <AllowedVoteSource<T>>::get() == BlockVoteSource::Compute {
				if T::AuthorityProvider::authority_count() >=
					<MiningSlotCountInitiatingTaxProof<T>>::get().unique_saturated_into()
				{
					<AllowedVoteSource<T>>::put(BlockVoteSource::Tax);
					return true
				}
			}
			false
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

		pub fn calculate_next_voting_minimum(
			current_voting_minimum: u128,
			target_period_votes: u128,
			actual_period_votes: u128,
			min_voting_minimum: u128,
			max_voting_minimum: u128,
		) -> VotingMinimum {
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

			// Compute the next vote_eligibility based on the current
			// vote_eligibility and the ratio of target votes to adjusted votes.
			let mut next_voting_minimum: u128 = U256::from(current_voting_minimum)
				.saturating_mul(adjusted_votes.into())
				.checked_div(target_period_votes.into())
				.unwrap_or(0.into())
				.unique_saturated_into();

			next_voting_minimum =
				next_voting_minimum.min(max_voting_minimum).max(min_voting_minimum);
			next_voting_minimum
		}

		pub fn vote_eligibility() -> BlockVoteEligibility {
			BlockVoteEligibility {
				minimum: <CurrentVotingMinimum<T>>::get(),
				allowed_sources: <AllowedVoteSource<T>>::get(),
			}
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
		fn grandpa_vote_eligibility() -> Option<BlockVoteEligibility> {
			<VoteEligibilityHistory<T>>::get().get(0).cloned()
		}

		fn parent_voting_key() -> Option<H256> {
			<ParentVotingKey<T>>::get()
		}
	}
}
