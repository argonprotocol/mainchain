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

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use binary_merkle_tree::verify_proof;
	use frame_support::{pallet_prelude::*, traits::FindAuthor};
	use frame_system::pallet_prelude::*;
	use sp_core::U256;
	use sp_runtime::{
		traits::{BlakeTwo256, UniqueSaturatedInto},
		ConsensusEngineId, RuntimeAppPublic,
	};
	use sp_std::vec::Vec;

	use ulx_primitives::{
		block_seal::VoteSource,
		digests::{
			BlockVoteDigest, BlockVoteSource, BLOCK_VOTES_DIGEST_ID, COMPUTE_AUTHORITY_DIGEST_ID,
		},
		inherents::{BlockSealInherent, BlockSealInherentData, InherentError},
		localchain::BlockVote,
		notebook::NotebookNumber,
		AuthorityProvider, BlockSealAuthorityId, BlockSealDigest, BlockSealerInfo,
		BlockSealerProvider, BlockVotingProvider, MerkleProof, NotaryId, NotebookProvider,
		AUTHOR_DIGEST_ID, BLOCK_SEAL_DIGEST_ID,
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
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// Type that provides authorities
		type AuthorityProvider: AuthorityProvider<Self::AuthorityId, Self::Block, Self::AccountId>;
		/// Provide notebook lookups
		type NotebookProvider: NotebookProvider;

		/// Lookup previous block votes specifications
		type BlockVotingProvider: BlockVotingProvider<Self::Block>;
	}

	/// Author of previous block
	#[pallet::storage]
	pub(super) type LastBlockSealer<T: Config> =
		StorageValue<_, BlockSealerInfo<T::AccountId>, OptionQuery>;
	#[pallet::storage]
	pub(super) type TempBlockSealerInfo<T: Config> =
		StorageValue<_, BlockSealerInfo<T::AccountId>, OptionQuery>;

	/// Author of current block (temporary storage).
	#[pallet::storage]
	#[pallet::getter(fn author)]
	pub(super) type TempAuthor<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Decodes the block seal from the digest.
	#[pallet::storage]
	pub(super) type TempSealDigest<T: Config> = StorageValue<_, BlockSealDigest, OptionQuery>;
	#[pallet::storage]
	pub(super) type TempBlockVoteDigest<T: Config> = StorageValue<_, BlockVoteDigest, OptionQuery>;
	/// Temporary storage of the compute authority supplied during proof of compute phase
	#[pallet::storage]
	pub(super) type TempComputeAuthorityDigest<T: Config> =
		StorageValue<_, BlockSealAuthorityId, OptionQuery>;

	/// Ensures only a single inherent is applied
	#[pallet::storage]
	pub(super) type HasSealInherent<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		InvalidBlockVoteSource,
		InvalidNonce,
		InvalidComputePuzzleNonce,
		InvalidSubmitter,
		UnregisteredBlockAuthor,
		InvalidBlockVoteProof,
		NoHistoricalVoteEligibility,
		DuplicateSealSpecification,
		InvalidPower,
		InvalidParentVotingKey,
		InvalidBlockHistoryVote,
		BlockSealDigestMismatch,
		BlockVoteDigestMissing,
		InvalidContinuationAuthor,
		InvalidContinuation,
		InvalidBlockSealUsed,
		IneligibleNotebookUsed,
		NoEligibleVotingRoot,
		InvalidAuthoritySupplied,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_logs =
				digest.logs.iter().filter_map(|a| a.as_pre_runtime()).collect::<Vec<_>>();
			for (id, mut data) in pre_runtime_logs {
				if id == AUTHOR_DIGEST_ID && <TempAuthor<T>>::get() == None {
					assert!(!<TempAuthor<T>>::exists(), "Author digest can only be provided once!");
					let decoded = T::AccountId::decode(&mut data);
					if let Some(account_id) = decoded.ok() {
						<TempAuthor<T>>::put(&account_id);
					}
				}
				if id == BLOCK_SEAL_DIGEST_ID {
					assert!(
						!<TempSealDigest<T>>::exists(),
						"Seal digest can only be provided once!"
					);
					let decoded = BlockSealDigest::decode(&mut data);
					if let Some(seal) = decoded.ok() {
						<TempSealDigest<T>>::put(seal);
					}
				}
				if id == BLOCK_VOTES_DIGEST_ID {
					// Duplicated logic with block_vote pallet, so we don't do extra validation
					if let Some(digest) = BlockVoteDigest::decode(&mut data).ok() {
						<TempBlockVoteDigest<T>>::put(digest);
					}
				}
				if id == COMPUTE_AUTHORITY_DIGEST_ID {
					assert!(
						!<TempComputeAuthorityDigest<T>>::exists(),
						"Compute authority digest can only be provided once!"
					);
					let decoded = BlockSealAuthorityId::decode(&mut data);
					if let Some(authority_id) = decoded.ok() {
						<TempComputeAuthorityDigest<T>>::put(authority_id);
					}
				}
			}

			assert_ne!(
				<TempAuthor<T>>::get(),
				None,
				"No valid account id provided for block author."
			);
			assert_ne!(
				<TempBlockVoteDigest<T>>::get(),
				None,
				"The block vote digest was not provided"
			);
			assert_ne!(<TempSealDigest<T>>::get(), None, "The seal digest was not provided");

			T::DbWeight::get().reads_writes(3, 3)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			assert!(HasSealInherent::<T>::take(), "Block seal inherent must be included");
			let sealer = <TempBlockSealerInfo<T>>::take().expect("BlockSealer must be set");
			<LastBlockSealer<T>>::put(sealer);
			// ensure we never go to trie with these values.
			TempSealDigest::<T>::kill();
			TempAuthor::<T>::kill();
			TempBlockVoteDigest::<T>::kill();
			TempComputeAuthorityDigest::<T>::kill();
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn apply(origin: OriginFor<T>, seal: BlockSealInherent) -> DispatchResult {
			ensure_none(origin)?;

			ensure!(!HasSealInherent::<T>::exists(), Error::<T>::DuplicateSealSpecification);
			HasSealInherent::<T>::put(true);

			let seal_digest =
				<TempSealDigest<T>>::get().expect("already unwrapped, should not be possible");
			let block_vote_digest =
				<TempBlockVoteDigest<T>>::get().expect("already unwrapped, should not be possible");
			let block_author =
				<TempAuthor<T>>::get().expect("already unwrapped, should not be possible");

			let miner_rewards_account =
				T::AuthorityProvider::get_rewards_account(block_author.clone()).expect(
					"Block author must have a rewards account configured in the authority provider",
				);

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			if matches!(seal, BlockSealInherent::Continuation) {
				ensure!(U256::MAX == seal_digest.nonce, Error::<T>::BlockSealDigestMismatch);
				let last_seal = <LastBlockSealer<T>>::get()
					.or_else(|| {
						let Some((_, _, _, account_id)) = T::AuthorityProvider::miner_zero() else {
							return None
						};
						let Some(rewards_account) =
							T::AuthorityProvider::get_rewards_account(account_id)
						else {
							return None
						};
						Some(BlockSealerInfo {
							miner_rewards_account: rewards_account.clone(),
							block_vote_rewards_account: rewards_account.clone(),
							notaries_included: 0,
						})
					})
					.ok_or(Error::<T>::InvalidContinuationAuthor)?;

				ensure!(
					last_seal.miner_rewards_account == miner_rewards_account,
					Error::<T>::InvalidContinuationAuthor
				);
				ensure!(
					block_vote_digest.votes_count < 5 || current_block_number < 3u32.into(),
					Error::<T>::InvalidContinuation
				);

				return Ok(())
			}

			ensure!(current_block_number >= 3u32.into(), Error::<T>::InvalidBlockSealUsed);

			let (nonce, block_vote, notary_id, source_notebook_proof, source_notebook_number) =
				match seal {
					BlockSealInherent::ClosestNonce {
						nonce,
						block_vote,
						notary_id,
						source_notebook_proof,
						source_notebook_number,
					} => (
						nonce,
						block_vote,
						notary_id,
						source_notebook_proof,
						source_notebook_number,
					),
					_ => unreachable!(),
				};
			ensure!(nonce == seal_digest.nonce, Error::<T>::BlockSealDigestMismatch);
			let parent_voting_key = T::BlockVotingProvider::parent_voting_key()
				.ok_or(Error::<T>::InvalidParentVotingKey)?;
			ensure!(
				nonce == block_vote.calculate_block_nonce(notary_id, parent_voting_key),
				Error::<T>::InvalidNonce
			);

			let grandparent_block_number = current_block_number - 2u32.into();
			let block_vote_account_id =
				T::AccountId::decode(&mut block_vote.account_id.encode().as_slice())
					.map_err(|_| Error::<T>::InvalidSubmitter)?;
			Self::verify_block_vote(
				&block_vote,
				&block_author,
				&block_vote_account_id,
				grandparent_block_number,
			)?;
			Self::verify_vote_source(
				notary_id,
				grandparent_block_number.unique_saturated_into(),
				&block_vote,
				source_notebook_proof,
				source_notebook_number,
			)?;

			<TempBlockSealerInfo<T>>::put(BlockSealerInfo {
				miner_rewards_account,
				block_vote_rewards_account: block_vote_account_id.clone(),
				notaries_included: block_vote_digest.notebook_numbers.len() as u32,
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn verify_vote_source(
			notary_id: NotaryId,
			block_number: u32,
			block_vote: &BlockVote,
			source_notebook_proof: MerkleProof,
			source_notebook_number: NotebookNumber,
		) -> DispatchResult {
			let (notebook_votes_root, notebook_number) =
				T::NotebookProvider::get_eligible_block_votes_root(notary_id, block_number)
					.ok_or(Error::<T>::NoEligibleVotingRoot)?;
			ensure!(notebook_number == source_notebook_number, Error::<T>::IneligibleNotebookUsed);
			ensure!(
				verify_proof::<'_, BlakeTwo256, _, _>(
					&notebook_votes_root,
					source_notebook_proof.proof,
					source_notebook_proof.number_of_leaves as usize,
					source_notebook_proof.leaf_index as usize,
					&block_vote.encode(),
				),
				Error::<T>::InvalidBlockVoteProof
			);
			Ok(())
		}

		pub fn verify_block_vote(
			block_vote: &BlockVote,
			block_author: &T::AccountId,
			block_vote_account_id: &T::AccountId,
			grandparent_block_number: BlockNumberFor<T>,
		) -> DispatchResult {
			let grandpa_vote_eligibility = T::BlockVotingProvider::grandpa_vote_eligibility()
				.ok_or(Error::<T>::NoHistoricalVoteEligibility)?;

			ensure!(block_vote.power >= grandpa_vote_eligibility.minimum, Error::<T>::InvalidPower);

			let grandparent_hash =
				<frame_system::Pallet<T>>::block_hash(grandparent_block_number.clone());
			let eligible_vote_block =
				<frame_system::Pallet<T>>::block_hash(grandparent_block_number - 1u32.into());
			ensure!(
				eligible_vote_block.as_ref() == block_vote.block_hash.as_bytes(),
				Error::<T>::InvalidBlockHistoryVote
			);

			match &block_vote.vote_source {
				VoteSource::Tax { channel_pass: _ } => {
					ensure!(
						grandpa_vote_eligibility.allowed_sources == BlockVoteSource::Tax,
						Error::<T>::InvalidBlockVoteSource
					);
					ensure!(
						TempComputeAuthorityDigest::<T>::get().is_none(),
						Error::<T>::InvalidAuthoritySupplied
					);
					// check that the block author is one of the validators
					let authority_id = T::AuthorityProvider::get_authority(block_author.clone())
						.ok_or(Error::<T>::UnregisteredBlockAuthor)?;

					// ensure this miner is eligible to submit this tax proof
					let block_peer =
						T::AuthorityProvider::block_peer(&grandparent_hash, &block_vote.account_id)
							.ok_or(Error::<T>::InvalidSubmitter)?;

					ensure!(block_peer.authority_id == authority_id, Error::<T>::InvalidSubmitter);

					// TODO: verify channel pass authority
					// let channel_pass_hash = channel_pass.hash();
				},
				VoteSource::Compute { .. } => {
					ensure!(
						grandpa_vote_eligibility.allowed_sources == BlockVoteSource::Compute,
						Error::<T>::InvalidBlockVoteSource
					);
					ensure!(
						TempComputeAuthorityDigest::<T>::get().is_some(),
						Error::<T>::InvalidAuthoritySupplied
					);

					ensure!(block_author == block_vote_account_id, Error::<T>::InvalidSubmitter);

					let puzzle_nonce = &block_vote.calculate_puzzle_nonce();
					let power = BlockVote::calculate_compute_power(puzzle_nonce);
					ensure!(power == block_vote.power, Error::<T>::InvalidPower);
				},
			};

			Ok(())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier =
			ulx_primitives::inherents::INHERENT_IDENTIFIER;

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

		fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			return Ok(Some(InherentError::MissingSeal))
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
				return Some(account_id)
			}

			for (id, mut data) in digests.into_iter() {
				if id == AUTHOR_DIGEST_ID {
					let decoded = T::AccountId::decode(&mut data);
					if let Some(account_id) = decoded.ok() {
						<TempAuthor<T>>::put(&account_id);
						return Some(account_id)
					}
				}
			}

			None
		}
	}

	impl<T: Config> BlockSealerProvider<T::AccountId> for Pallet<T> {
		fn get_sealer_info() -> BlockSealerInfo<T::AccountId> {
			<TempBlockSealerInfo<T>>::get().expect("BlockSealer must be set")
		}
	}
}
