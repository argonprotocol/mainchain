#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::large_enum_variant)]

extern crate alloc;
pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

const LOG_TARGET: &str = "runtime::block_seal";
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use alloc::{collections::btree_map::BTreeMap, vec, vec::Vec};

	use binary_merkle_tree::{merkle_proof, verify_proof};
	use frame_support::{pallet_prelude::*, traits::FindAuthor};
	use frame_system::pallet_prelude::*;
	use log::info;
	use sp_core::{H256, U256};
	use sp_runtime::{
		traits::{BlakeTwo256, Block as BlockT, Verify},
		ConsensusEngineId, DigestItem, RuntimeAppPublic,
	};

	use argon_primitives::{
		inherents::{BlockSealInherent, BlockSealInherentData, SealInherentError},
		localchain::{BestBlockVoteSeal, BlockVote, BlockVoteT},
		notebook::NotebookNumber,
		tick::Tick,
		AuthorityProvider, BlockSealAuthoritySignature, BlockSealEventHandler, BlockSealerInfo,
		BlockSealerProvider, BlockVotingKey, BlockVotingProvider, DataDomainProvider, MerkleProof,
		NotaryId, NotaryNotebookVotes, NotebookProvider, ParentVotingKeyDigest, TickProvider,
		VotingKey, AUTHOR_DIGEST_ID, ESCROW_CLAWBACK_TICKS, PARENT_VOTING_KEY_DIGEST,
	};

	use super::*;

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

		/// Lookup previous block votes specifications
		type BlockVotingProvider: BlockVotingProvider<Self::Block>;

		type TickProvider: TickProvider<Self::Block>;

		type DataDomainProvider: DataDomainProvider<Self::AccountId>;

		/// Emit events when a block seal is read
		type EventHandler: BlockSealEventHandler;
	}

	#[pallet::storage]
	pub(super) type LastBlockSealerInfo<T: Config> =
		StorageValue<_, BlockSealerInfo<T::AccountId>, OptionQuery>;

	/// The calculated parent voting key for a block. Refers to the Notebook BlockVote Revealed
	/// Secret + VotesMerkleRoot of the parent block notebooks.
	#[pallet::storage]
	pub(super) type ParentVotingKey<T: Config> = StorageValue<_, Option<H256>, ValueQuery>;

	/// Author of current block (temporary storage).
	#[pallet::storage]
	#[pallet::getter(fn author)]
	pub(super) type TempAuthor<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Ensures only a single inherent is applied
	#[pallet::storage]
	pub(super) type TempSealInherent<T: Config> = StorageValue<_, BlockSealInherent, OptionQuery>;

	/// Temporarily track the parent voting key digest
	#[pallet::storage]
	pub(super) type TempVotingKeyDigest<T: Config> =
		StorageValue<_, ParentVotingKeyDigest, OptionQuery>;

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
		/// The data domain was not registered
		UnregisteredDataDomain,
		/// The data domain account is mismatched with the block reward seeker
		InvalidDataDomainAccount,
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
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let digest = <frame_system::Pallet<T>>::digest();
			let is_author_preset = <TempAuthor<T>>::exists();
			for log in digest.logs.into_iter() {
				if let Some(parent_voting_key_digest) =
					log.consensus_try_to::<ParentVotingKeyDigest>(&PARENT_VOTING_KEY_DIGEST)
				{
					assert!(
						!<TempVotingKeyDigest<T>>::exists(),
						"ParentVotingKey digest can only be provided once!"
					);
					<TempVotingKeyDigest<T>>::put(parent_voting_key_digest);
				}

				if let Some(account_id) = log.pre_runtime_try_to::<T::AccountId>(&AUTHOR_DIGEST_ID)
				{
					if !is_author_preset {
						assert!(
							!<TempAuthor<T>>::exists(),
							"ParentVotingKey digest can only be provided once!"
						);
					}

					<TempAuthor<T>>::put(account_id);
				}
			}

			assert_ne!(
				<TempAuthor<T>>::get(),
				None,
				"No valid account id provided for block author."
			);

			T::DbWeight::get().reads_writes(2, 2)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			assert!(
				TempSealInherent::<T>::take().is_some(),
				"Block seal inherent must be included"
			);
			// ensure we never go to trie with these values.
			TempAuthor::<T>::kill();

			let current_tick = T::TickProvider::current_tick();

			// tick of the parent voting key
			let votes_tick = current_tick.saturating_sub(1);

			let notebooks_at_tick = T::NotebookProvider::notebooks_at_tick(current_tick);

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

			if let Some(included_digest) = TempVotingKeyDigest::<T>::take() {
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
				target: LOG_TARGET,
				"Block seal inherent submitted {:?}", seal
			);
			Self::apply_seal(seal).map_err(|e| {
				log::error!(target: LOG_TARGET, "Error applying block seal: {:?}", e);
				e
			})?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn apply_seal(seal: BlockSealInherent) -> DispatchResult {
			ensure!(!TempSealInherent::<T>::exists(), Error::<T>::DuplicateBlockSealProvided);
			TempSealInherent::<T>::put(seal.clone());

			let block_author =
				<TempAuthor<T>>::get().expect("already unwrapped, should not be possible");

			match seal {
				BlockSealInherent::Compute => {
					// NOTE: the compute nonce is checked in the node
					<LastBlockSealerInfo<T>>::put(BlockSealerInfo {
						block_vote_rewards_account: None,
						block_author_account_id: block_author,
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

					// there won't be a grandparent block to vote for until block 2, and those votes
					// don't count until tick 4
					ensure!(current_tick >= 4u32, Error::<T>::NoEligibleVotingRoot);

					let parent_voting_key =
						<ParentVotingKey<T>>::get().ok_or(Error::<T>::ParentVotingKeyNotFound)?;
					ensure!(
						seal_strength == block_vote.get_seal_strength(notary_id, parent_voting_key),
						Error::<T>::InvalidVoteSealStrength
					);

					let votes_from_tick = current_tick - 2u32;
					let block_vote_rewards_account = T::AccountId::decode(
						&mut block_vote.block_rewards_account_id.encode().as_slice(),
					)
					.map_err(|_| Error::<T>::UnableToDecodeVoteAccount)?;
					Self::verify_block_vote(
						seal_strength,
						block_vote,
						&block_author,
						votes_from_tick,
						miner_signature.clone(),
					)?;
					Self::verify_vote_source(
						notary_id,
						votes_from_tick,
						block_vote,
						source_notebook_proof,
						source_notebook_number,
					)?;
					<LastBlockSealerInfo<T>>::put(BlockSealerInfo {
						block_author_account_id: block_author,
						block_vote_rewards_account: Some(block_vote_rewards_account),
					});
				},
			}
			T::EventHandler::block_seal_read(&seal);
			Ok(())
		}
		pub fn verify_vote_source(
			notary_id: NotaryId,
			votes_from_tick: Tick,
			block_vote: &BlockVote,
			source_notebook_proof: &MerkleProof,
			source_notebook_number: NotebookNumber,
		) -> DispatchResult {
			let (notebook_votes_root, notebook_number) =
				T::NotebookProvider::get_eligible_tick_votes_root(notary_id, votes_from_tick)
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
			votes_from_tick: Tick,
			signature: BlockSealAuthoritySignature,
		) -> DispatchResult {
			let grandpa_vote_minimum = T::BlockVotingProvider::grandparent_vote_minimum()
				.ok_or(Error::<T>::NoGrandparentVoteMinimum)?;

			ensure!(block_vote.power >= grandpa_vote_minimum, Error::<T>::InsufficientVotingPower);

			let voted_on_blocks =
				T::TickProvider::blocks_at_tick(votes_from_tick.saturating_sub(2));
			ensure!(
				voted_on_blocks.iter().any(|a| a.as_ref() == block_vote.block_hash.as_bytes()),
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
			let data_domain_hash = &block_vote.data_domain_hash;
			let data_domain_account =
				T::AccountId::decode(&mut block_vote.data_domain_account.encode().as_slice())
					.map_err(|_| Error::<T>::UnableToDecodeVoteAccount)?;
			let last_tick =
				votes_from_tick.saturating_sub(T::TickProvider::ticker().escrow_expiration_ticks);
			ensure!(
				T::DataDomainProvider::is_registered_payment_account(
					data_domain_hash,
					&data_domain_account,
					(last_tick.saturating_sub(ESCROW_CLAWBACK_TICKS), last_tick)
				),
				Error::<T>::InvalidDataDomainAccount
			);
			ensure!(
				block_vote.signature.verify(&block_vote.hash()[..], &block_vote.account_id),
				Error::<T>::BlockVoteInvalidSignature
			);

			Ok(())
		}

		/// This API will find block votes from the perspective of a new block creation activity
		/// calling into the runtime while trying to build the next block. This means the current
		/// tick is expected to be +1 of the runtime tick.
		pub fn find_vote_block_seals(
			notebook_votes: Vec<NotaryNotebookVotes>,
			with_better_strength: U256,
		) -> Result<FindBlockVoteSealResult<T>, Error<T>> {
			let Some(parent_key) = <ParentVotingKey<T>>::get() else {
				return Ok(BoundedVec::new());
			};

			// runtime tick will have the voting key for the parent
			let runtime_tick = T::TickProvider::current_tick();
			if runtime_tick <= 4u32 {
				return Ok(BoundedVec::new());
			}

			let votes_in_tick = runtime_tick - 1u32;
			let voted_for_blocks_at_tick = votes_in_tick - 2u32;

			// This API is called when trying to assemble a block. Current tick will be the "parent"
			// of the new block, so votes come from 1 tick previous. They will be votes for the
			// previous tick - aka, - 2 ticks from the current (parent) tick.
			let grandparent_tick_blocks = T::TickProvider::blocks_at_tick(voted_for_blocks_at_tick);
			info!(target: LOG_TARGET,
				"eligible votes at tick {} - {:?} (runtime tick={})",
				voted_for_blocks_at_tick, grandparent_tick_blocks, runtime_tick
			);

			let mut best_votes = vec![];
			let mut leafs_by_notary = BTreeMap::new();

			for NotaryNotebookVotes { notebook_number, notary_id, raw_votes } in
				notebook_votes.into_iter()
			{
				// don't use locked notary votes!
				if T::NotebookProvider::is_notary_locked_at_tick(notary_id, votes_in_tick) {
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

				if !grandparent_tick_blocks.contains(&vote.block_hash) {
					info!(target: LOG_TARGET, "cant use vote for grandparent tick {:?}", vote.block_hash);
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

	impl<T: Config> FindAuthor<T::AccountId> for Pallet<T> {
		fn find_author<'a, I>(digests: I) -> Option<T::AccountId>
		where
			I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
		{
			// if this is called after initialize, we're fine, but it might not be
			if let Some(account_id) = <TempAuthor<T>>::get() {
				return Some(account_id);
			}

			for (id, mut data) in digests.into_iter() {
				if id == AUTHOR_DIGEST_ID {
					let decoded = T::AccountId::decode(&mut data);
					if let Ok(account_id) = decoded {
						<TempAuthor<T>>::put(&account_id);
						return Some(account_id);
					}
				}
			}

			None
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
