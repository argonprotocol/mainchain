#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::large_enum_variant)]

extern crate alloc;
pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use alloc::{collections::btree_map::BTreeMap, vec, vec::Vec};
	use argon_notary_audit::VerifyError;
	use argon_primitives::{
		digests::Digestset,
		fork_power::ForkPower,
		inherents::{BlockSealInherent, BlockSealInherentData, SealInherentError},
		localchain::{BestBlockVoteSeal, BlockVote, BlockVoteT},
		notary::NotaryNotebookRawVotes,
		notebook::NotebookNumber,
		tick::Tick,
		AuthorityProvider, BlockSealAuthoritySignature, BlockSealDigest, BlockSealEventHandler,
		BlockSealSpecProvider, BlockSealerInfo, BlockSealerProvider, BlockVotingKey,
		BlockVotingPower, MerkleProof, NotaryId, NotebookProvider, ParentVotingKeyDigest,
		TickProvider, VotingKey, VotingSchedule, PARENT_VOTING_KEY_DIGEST,
	};
	use binary_merkle_tree::{merkle_proof, verify_proof};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use log::info;
	use sp_core::{H256, U256};
	use sp_runtime::{
		traits::{BlakeTwo256, Block as BlockT, Verify},
		DigestItem, RuntimeAppPublic,
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The identifier type for an authority.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// Type that provides authorities
		type AuthorityProvider: AuthorityProvider<Self::AuthorityId, Self::Block, Self::AccountId>;
		/// Provide notebook lookups
		type NotebookProvider: NotebookProvider;

		/// Lookup seal specifications
		type BlockSealSpecProvider: BlockSealSpecProvider<Self::Block>;

		type TickProvider: TickProvider<Self::Block>;

		/// Emit events when a block seal is read
		type EventHandler: BlockSealEventHandler;

		type Digests: Get<Result<Digestset<VerifyError, Self::AccountId>, DispatchError>>;
	}

	#[pallet::storage]
	pub(super) type LastBlockSealerInfo<T: Config> =
		StorageValue<_, BlockSealerInfo<T::AccountId>, OptionQuery>;

	/// The calculated parent voting key for a block. Refers to the Notebook BlockVote Revealed
	/// Secret + VotesMerkleRoot of the parent block notebooks.
	#[pallet::storage]
	pub(super) type ParentVotingKey<T: Config> = StorageValue<_, Option<H256>, ValueQuery>;

	/// The calculated strength in the runtime so that it can be
	/// upgraded, but is used by the node to determine which fork to follow
	#[pallet::storage]
	pub(super) type BlockForkPower<T: Config> = StorageValue<_, ForkPower, ValueQuery>;

	/// The count of votes in the last 3 ticks
	#[pallet::storage]
	pub(super) type VotesInPast3Ticks<T> =
		StorageValue<_, BoundedVec<(Tick, u32), ConstU32<3>>, ValueQuery>;

	/// Ensures only a single inherent is applied
	#[pallet::storage]
	pub(super) type TempSealInherent<T: Config> = StorageValue<_, BlockSealInherent, OptionQuery>;

	type FindBlockVoteSealResult<T> = BoundedVec<
		BestBlockVoteSeal<
			<T as frame_system::Config>::AccountId,
			<T as pallet::Config>::AuthorityId,
		>,
		ConstU32<2>,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// The strength of the given seal did not match calculations
		InvalidVoteSealStrength,
		/// Vote not submitted by the right miner
		InvalidSubmitter,
		/// Could not decode the vote bytes
		UnableToDecodeVoteAccount,
		/// The block author is not a registered miner
		UnregisteredBlockAuthor,
		/// The merkle proof of vote inclusion in the notebook is invalid
		InvalidBlockVoteProof,
		/// No vote minimum found at grandparent height
		NoGrandparentVoteMinimum,
		/// Too many block seals submitted
		DuplicateBlockSealProvided,
		/// The block vote did not reach the minimum voting power at time of the grandparent block
		InsufficientVotingPower,
		/// No registered voting key found for the parent block
		ParentVotingKeyNotFound,
		/// The block vote was not for a valid block
		InvalidVoteGrandparentHash,
		/// The notebook for this vote was not eligible to vote
		IneligibleNotebookUsed,
		/// The lookup to verify a vote's authenticity is not available for the given block
		NoEligibleVotingRoot,
		/// Message was not signed by a registered miner
		InvalidAuthoritySignature,
		/// Could not decode the scale bytes of the votes
		CouldNotDecodeVote,
		/// Too many notebooks were submitted for the current tick. Should not be possible
		MaxNotebooksAtTickExceeded,
		/// No closest miner found for vote
		NoClosestMinerFoundForVote,
		/// The vote signature was invalid
		BlockVoteInvalidSignature,
		/// Invalid fork power parent
		InvalidForkPowerParent,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			T::DbWeight::get().reads_writes(2, 2)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			assert!(
				TempSealInherent::<T>::take().is_some(),
				"Block seal inherent must be included"
			);

			let voting_schedule = T::TickProvider::voting_schedule();

			// tick of the parent voting key
			let votes_tick = voting_schedule.eligible_votes_tick();

			let notebooks_at_tick =
				T::NotebookProvider::notebooks_at_tick(voting_schedule.notebook_tick());

			let parent_voting_keys = notebooks_at_tick
				.into_iter()
				.filter_map(|(notary_id, _, parent_secret)| {
					if let Some(parent_secret) = parent_secret {
						// NOTE: secret + eligibility is verified in the notebook provider
						if let Some((parent_vote_root, _)) =
							T::NotebookProvider::get_eligible_tick_votes_root(notary_id, votes_tick)
						{
							return Some(BlockVotingKey { parent_vote_root, parent_secret });
						}
					}
					None
				})
				.collect::<Vec<_>>();

			let mut parent_voting_key: Option<VotingKey> = None;
			if !parent_voting_keys.is_empty() {
				parent_voting_key = Some(BlockVotingKey::create_key(parent_voting_keys));
				<ParentVotingKey<T>>::put(parent_voting_key);
			}

			let included_digest = T::Digests::get().expect("Digests must be set");
			if let Some(included_digest) = included_digest.voting_key {
				assert_eq!(
					included_digest.parent_voting_key, parent_voting_key,
					"Calculated ParentVotingKey does not match the value in included digest."
				);
			} else {
				<frame_system::Pallet<T>>::deposit_log(DigestItem::Consensus(
					PARENT_VOTING_KEY_DIGEST,
					ParentVotingKeyDigest { parent_voting_key }.encode(),
				));
			}
		}
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn apply(origin: OriginFor<T>, seal: BlockSealInherent) -> DispatchResult {
			ensure_none(origin)?;
			info!(
				"Block seal inherent submitted -> {}",
				match seal {
					BlockSealInherent::Compute => "Compute",
					BlockSealInherent::Vote { .. } => "Vote",
				}
			);
			Self::apply_seal(seal).inspect_err(|e| {
				log::error!("Error applying block seal: {:?}", e);
			})?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn fork_power() -> ForkPower {
			BlockForkPower::<T>::get()
		}

		pub fn calculate_fork_power(
			seal: BlockSealDigest,
			voting_power: BlockVotingPower,
			notebooks: u32,
		) -> ForkPower {
			let mut fork_power = BlockForkPower::<T>::get();
			let compute = T::BlockSealSpecProvider::compute_difficulty();

			fork_power.add(voting_power, notebooks, seal, compute);
			fork_power
		}

		pub fn apply_seal(seal: BlockSealInherent) -> DispatchResult {
			ensure!(!TempSealInherent::<T>::exists(), Error::<T>::DuplicateBlockSealProvided);
			TempSealInherent::<T>::put(seal.clone());

			let digests = T::Digests::get()?;
			let block_author = digests.author;
			let notebooks = T::NotebookProvider::notebooks_in_block().len() as u32;
			let vote_digest = digests.block_vote;

			<VotesInPast3Ticks<T>>::mutate(|votes| {
				if votes.is_full() {
					votes.remove(0);
				}
				let notebook_tick = T::TickProvider::voting_schedule().notebook_tick();
				let _ = votes.try_push((notebook_tick, vote_digest.votes_count));
			});

			match seal {
				BlockSealInherent::Compute => {
					// NOTE: the compute nonce is checked in the node
					<LastBlockSealerInfo<T>>::put(BlockSealerInfo {
						block_vote_rewards_account: None,
						block_author_account_id: block_author,
					});
					let compute_difficulty = T::BlockSealSpecProvider::compute_difficulty();

					BlockForkPower::<T>::mutate(|fork| {
						fork.add_compute(vote_digest.voting_power, notebooks, compute_difficulty);
					});
				},
				BlockSealInherent::Vote {
					seal_strength,
					ref block_vote,
					notary_id,
					ref source_notebook_proof,
					source_notebook_number,
					ref miner_signature,
				} => {
					let current_tick = T::TickProvider::current_tick();
					let voting_schedule =
						VotingSchedule::when_evaluating_runtime_seals(current_tick);

					ensure!(voting_schedule.is_voting_started(), Error::<T>::NoEligibleVotingRoot);

					let parent_voting_key =
						<ParentVotingKey<T>>::get().ok_or(Error::<T>::ParentVotingKeyNotFound)?;
					ensure!(
						seal_strength == block_vote.get_seal_strength(notary_id, parent_voting_key),
						Error::<T>::InvalidVoteSealStrength
					);

					let block_vote_rewards_account = T::AccountId::decode(
						&mut block_vote.block_rewards_account_id.encode().as_slice(),
					)
					.map_err(|_| Error::<T>::UnableToDecodeVoteAccount)?;
					Self::verify_block_vote(
						seal_strength,
						block_vote,
						&block_author,
						&voting_schedule,
						miner_signature.clone(),
					)?;
					Self::verify_vote_source(
						notary_id,
						&voting_schedule,
						block_vote,
						source_notebook_proof,
						source_notebook_number,
					)?;
					<LastBlockSealerInfo<T>>::put(BlockSealerInfo {
						block_author_account_id: block_author,
						block_vote_rewards_account: Some(block_vote_rewards_account),
					});

					BlockForkPower::<T>::mutate(|fork| {
						fork.add_vote(vote_digest.voting_power, notebooks, seal_strength);
					});
				},
			}

			T::EventHandler::block_seal_read(&seal);
			Ok(())
		}

		pub fn verify_vote_source(
			notary_id: NotaryId,
			voting_schedule: &VotingSchedule,
			block_vote: &BlockVote,
			source_notebook_proof: &MerkleProof,
			source_notebook_number: NotebookNumber,
		) -> DispatchResult {
			let (notebook_votes_root, notebook_number) =
				T::NotebookProvider::get_eligible_tick_votes_root(
					notary_id,
					voting_schedule.eligible_votes_tick(),
				)
				.ok_or(Error::<T>::NoEligibleVotingRoot)?;
			ensure!(notebook_number == source_notebook_number, Error::<T>::IneligibleNotebookUsed);
			ensure!(
				verify_proof::<'_, BlakeTwo256, _, _>(
					&notebook_votes_root,
					source_notebook_proof.proof.clone(),
					source_notebook_proof.number_of_leaves as usize,
					source_notebook_proof.leaf_index as usize,
					&block_vote.encode(),
				),
				Error::<T>::InvalidBlockVoteProof
			);
			Ok(())
		}

		pub fn verify_block_vote(
			seal_strength: U256,
			block_vote: &BlockVote,
			block_author: &T::AccountId,
			voting_schedule: &VotingSchedule,
			signature: BlockSealAuthoritySignature,
		) -> DispatchResult {
			let grandpa_vote_minimum = T::BlockSealSpecProvider::grandparent_vote_minimum()
				.ok_or(Error::<T>::NoGrandparentVoteMinimum)?;

			ensure!(block_vote.power >= grandpa_vote_minimum, Error::<T>::InsufficientVotingPower);

			let grandpa_tick_block =
				T::TickProvider::block_at_tick(voting_schedule.grandparent_votes_tick())
					.ok_or(Error::<T>::InvalidVoteGrandparentHash)?;
			ensure!(
				grandpa_tick_block.as_ref() == block_vote.block_hash.as_bytes(),
				Error::<T>::InvalidVoteGrandparentHash
			);

			// check that the block author is one of the validators
			let authority_id = T::AuthorityProvider::get_authority(block_author.clone())
				.ok_or(Error::<T>::UnregisteredBlockAuthor)?;

			// ensure this miner is eligible to submit this tax proof
			let block_peer = T::AuthorityProvider::xor_closest_authority(seal_strength)
				.ok_or(Error::<T>::InvalidSubmitter)?;

			ensure!(block_peer.authority_id == authority_id, Error::<T>::InvalidSubmitter);

			let parent_hash = <frame_system::Pallet<T>>::parent_hash();

			let message = BlockVote::seal_signature_message(&parent_hash, seal_strength);
			let Ok(signature) = AuthoritySignature::<T>::decode(&mut signature.as_ref()) else {
				return Err(Error::<T>::InvalidAuthoritySignature.into());
			};
			ensure!(
				authority_id.verify(&message, &signature),
				Error::<T>::InvalidAuthoritySignature
			);
			ensure!(
				block_vote.signature.verify(&block_vote.hash()[..], &block_vote.account_id),
				Error::<T>::BlockVoteInvalidSignature
			);

			Ok(())
		}

		/// Returns true if there's a parent voting key and votes in the tick notebooks
		/// This API is used by the node to determine if it should build on a block
		pub fn has_eligible_votes() -> bool {
			if !<ParentVotingKey<T>>::exists() {
				return false;
			}
			let current_tick = T::TickProvider::current_tick();
			let voting_schedule = VotingSchedule::when_evaluating_runtime_votes(current_tick);
			let votes_tick = voting_schedule.eligible_votes_tick();
			let vote_history = <VotesInPast3Ticks<T>>::get();
			vote_history.iter().any(|(tick, votes)| *tick == votes_tick && *votes > 0)
		}

		/// This API will find block votes from the perspective of a new block creation activity
		/// calling into the runtime while trying to build the next block.
		///
		/// That means we're using the votes from the notebooks themselves
		pub fn find_vote_block_seals(
			notebook_votes: Vec<NotaryNotebookRawVotes>,
			with_better_strength: U256,
			expected_notebook_tick: Tick,
		) -> Result<FindBlockVoteSealResult<T>, Error<T>> {
			let Some(parent_key) = <ParentVotingKey<T>>::get() else {
				return Ok(BoundedVec::new());
			};

			// runtime tick will have the voting key for the parent
			let current_tick = T::TickProvider::current_tick();
			let voting_schedule = VotingSchedule::when_evaluating_runtime_votes(current_tick);
			if !voting_schedule.is_voting_started() {
				return Ok(BoundedVec::new());
			}

			ensure!(
				expected_notebook_tick == voting_schedule.notebook_tick(),
				Error::<T>::IneligibleNotebookUsed
			);

			let voted_for_block_at_tick = voting_schedule.grandparent_votes_tick();

			let Some(grandparent_tick_block) =
				T::TickProvider::block_at_tick(voted_for_block_at_tick)
			else {
				info!(
					"No eligible blocks to vote on in grandparent tick {:?}",
					voted_for_block_at_tick
				);
				return Ok(BoundedVec::new());
			};

			info!(
				"Finding votes for block at tick {} - {:?} (notebook tick={})",
				voted_for_block_at_tick, grandparent_tick_block, expected_notebook_tick
			);

			let mut best_votes = vec![];
			let mut leafs_by_notary = BTreeMap::new();

			for NotaryNotebookRawVotes { notebook_number, notary_id, raw_votes } in
				notebook_votes.into_iter()
			{
				// don't use locked notary votes!
				if T::NotebookProvider::is_notary_locked_at_tick(notary_id, expected_notebook_tick)
				{
					continue;
				}

				for (index, (vote_bytes, power)) in raw_votes.iter().enumerate() {
					leafs_by_notary
						.entry(notary_id)
						.or_insert_with(Vec::new)
						.push(vote_bytes.clone());

					let seal_strength = BlockVote::calculate_seal_strength(
						*power,
						vote_bytes.clone(),
						notary_id,
						parent_key,
					);
					if seal_strength >= with_better_strength {
						continue;
					}
					best_votes.push((seal_strength, notary_id, notebook_number, index));
				}
			}
			best_votes.sort_by(|a, b| a.0.cmp(&b.0));

			let mut result = BoundedVec::new();
			for (seal_strength, notary_id, source_notebook_number, index) in best_votes.into_iter()
			{
				let leafs = leafs_by_notary.get(&notary_id).expect("just created");

				let proof = merkle_proof::<BlakeTwo256, _, _>(leafs, index);

				let vote =
					BlockVoteT::<<T::Block as BlockT>::Hash>::decode(&mut leafs[index].as_slice())
						.map_err(|_| Error::<T>::CouldNotDecodeVote)?;

				if grandparent_tick_block != vote.block_hash {
					info!(
						"Cant use vote for grandparent tick {:?} - voted for {:?}",
						grandparent_tick_block, vote.block_hash
					);
					continue;
				}

				let closest_authority = T::AuthorityProvider::xor_closest_authority(seal_strength)
					.ok_or(Error::<T>::NoClosestMinerFoundForVote)?;
				let best_nonce = BestBlockVoteSeal {
					notary_id,
					seal_strength,
					block_vote_bytes: leafs[index].clone(),

					source_notebook_number,
					source_notebook_proof: MerkleProof {
						proof: BoundedVec::truncate_from(proof.proof),
						leaf_index: proof.leaf_index as u32,
						number_of_leaves: proof.number_of_leaves as u32,
					},
					closest_miner: (closest_authority.account_id, closest_authority.authority_id),
				};
				if result.try_push(best_nonce).is_err() {
					break;
				}
			}
			Ok(result)
		}
	}

	pub type AuthoritySignature<T> = <<T as Config>::AuthorityId as RuntimeAppPublic>::Signature;
	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = SealInherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier =
			argon_primitives::inherents::SEAL_INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call>
		where
			InherentData: BlockSealInherentData,
		{
			let seal = data
				.block_seal()
				.expect("Could not decode Block seal inherent data")
				.expect("Block seal inherent data must be provided");

			Some(Call::apply { seal })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let seal = match call {
				Call::apply { seal } => seal,
				_ => return Err(SealInherentError::MissingSeal),
			};
			let digest = data
				.digest()
				.expect("Could not decode Block seal digest data")
				.expect("Block seal digest data must be provided");

			ensure!(seal.matches(digest), SealInherentError::InvalidSeal);
			Ok(())
		}

		fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			Ok(Some(SealInherentError::MissingSeal))
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::apply { .. })
		}
	}

	impl<T: Config> BlockSealerProvider<T::AccountId> for Pallet<T> {
		fn get_sealer_info() -> BlockSealerInfo<T::AccountId> {
			<LastBlockSealerInfo<T>>::get().expect("BlockSealer must be set")
		}
	}

	impl<T: Config> Get<BlockSealInherent> for Pallet<T> {
		fn get() -> BlockSealInherent {
			<TempSealInherent<T>>::get().expect("Seal inherent must be set")
		}
	}
}
