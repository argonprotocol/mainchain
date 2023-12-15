#![feature(slice_take)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate core;

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
		digests::{BlockVoteDigest, BLOCK_VOTES_DIGEST_ID},
		inherents::{BlockSealInherent, BlockSealInherentData, InherentError},
		localchain::BlockVote,
		notebook::NotebookNumber,
		AuthorityProvider, BlockSealAuthoritySignature, BlockSealerInfo, BlockSealerProvider,
		BlockVotingProvider, MerkleProof, NotaryId, NotebookProvider, AUTHOR_DIGEST_ID,
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
	}

	#[pallet::storage]
	pub(super) type LastBlockSealerInfo<T: Config> =
		StorageValue<_, BlockSealerInfo<T::AccountId>, OptionQuery>;

	/// Author of current block (temporary storage).
	#[pallet::storage]
	#[pallet::getter(fn author)]
	pub(super) type TempAuthor<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	pub(super) type TempBlockVoteDigest<T: Config> = StorageValue<_, BlockVoteDigest, OptionQuery>;

	/// Ensures only a single inherent is applied
	#[pallet::storage]
	pub(super) type TempSealInherent<T: Config> = StorageValue<_, BlockSealInherent, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		InvalidVoteProof,
		InvalidSubmitter,
		UnableToDecodeVoteAccount,
		UnregisteredBlockAuthor,
		InvalidBlockVoteProof,
		NoGrandparentVoteMinimum,
		DuplicateBlockSealProvided,
		InsufficientVotingPower,
		ParentVotingKeyNotFound,
		InvalidVoteGrandparentHash,
		BlockVoteDigestMissing,
		IneligibleNotebookUsed,
		NoEligibleVotingRoot,
		InvalidAuthoritySupplied,
		InvalidAuthoritySignature,
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
				if id == BLOCK_VOTES_DIGEST_ID {
					// Duplicated logic with block_vote pallet, so we don't do extra validation
					if let Some(digest) = BlockVoteDigest::decode(&mut data).ok() {
						<TempBlockVoteDigest<T>>::put(digest);
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

			T::DbWeight::get().reads_writes(3, 3)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			assert!(
				TempSealInherent::<T>::take().is_some(),
				"Block seal inherent must be included"
			);
			// ensure we never go to trie with these values.
			TempAuthor::<T>::kill();
			TempBlockVoteDigest::<T>::kill();
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn apply(origin: OriginFor<T>, seal: BlockSealInherent) -> DispatchResult {
			ensure_none(origin)?;

			ensure!(!TempSealInherent::<T>::exists(), Error::<T>::DuplicateBlockSealProvided);
			TempSealInherent::<T>::put(seal.clone());

			let block_vote_digest =
				<TempBlockVoteDigest<T>>::get().expect("already unwrapped, should not be possible");
			let block_author =
				<TempAuthor<T>>::get().expect("already unwrapped, should not be possible");

			let mut miner_rewards_account = block_author.clone();

			if let Some(rewards_account) =
				T::AuthorityProvider::get_rewards_account(block_author.clone())
			{
				miner_rewards_account = rewards_account;
			}

			match seal {
				BlockSealInherent::Compute => {
					// NOTE: the compute nonce is checked in the node
					<LastBlockSealerInfo<T>>::put(BlockSealerInfo {
						block_vote_rewards_account: miner_rewards_account.clone(),
						miner_rewards_account,
						notaries_included: block_vote_digest.tick_notebooks,
					});
				},
				BlockSealInherent::Vote {
					vote_proof,
					block_vote,
					notary_id,
					source_notebook_proof,
					source_notebook_number,
					miner_signature,
				} => {
					let current_block_number = <frame_system::Pallet<T>>::block_number();

					// there won't be a grandparent block to vote for until block 2, and those votes
					// don't count until block 4
					ensure!(current_block_number >= 4u32.into(), Error::<T>::NoEligibleVotingRoot);

					let parent_voting_key = T::BlockVotingProvider::parent_voting_key()
						.ok_or(Error::<T>::ParentVotingKeyNotFound)?;
					ensure!(
						vote_proof == block_vote.vote_proof(notary_id, parent_voting_key),
						Error::<T>::InvalidVoteProof
					);

					let grandparent_block_number = current_block_number - 2u32.into();
					let block_vote_account_id =
						T::AccountId::decode(&mut block_vote.account_id.encode().as_slice())
							.map_err(|_| Error::<T>::UnableToDecodeVoteAccount)?;
					Self::verify_block_vote(
						vote_proof,
						&block_vote,
						&block_author,
						grandparent_block_number,
						miner_signature,
					)?;
					Self::verify_vote_source(
						notary_id,
						grandparent_block_number.unique_saturated_into(),
						&block_vote,
						source_notebook_proof,
						source_notebook_number,
					)?;
					<LastBlockSealerInfo<T>>::put(BlockSealerInfo {
						miner_rewards_account,
						block_vote_rewards_account: block_vote_account_id.clone(),
						notaries_included: block_vote_digest.tick_notebooks,
					});
				},
			}

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
				T::NotebookProvider::get_eligible_tick_votes_root(notary_id, block_number)
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
			vote_proof: U256,
			block_vote: &BlockVote,
			block_author: &T::AccountId,
			grandparent_block_number: BlockNumberFor<T>,
			signature: BlockSealAuthoritySignature,
		) -> DispatchResult {
			let grandpa_vote_minimum = T::BlockVotingProvider::grandparent_vote_minimum()
				.ok_or(Error::<T>::NoGrandparentVoteMinimum)?;

			ensure!(block_vote.power >= grandpa_vote_minimum, Error::<T>::InsufficientVotingPower);

			let eligible_vote_block_hash =
				<frame_system::Pallet<T>>::block_hash(grandparent_block_number - 2u32.into());
			ensure!(
				eligible_vote_block_hash.as_ref() == block_vote.grandparent_block_hash.as_bytes(),
				Error::<T>::InvalidVoteGrandparentHash
			);

			// check that the block author is one of the validators
			let authority_id = T::AuthorityProvider::get_authority(block_author.clone())
				.ok_or(Error::<T>::UnregisteredBlockAuthor)?;

			// ensure this miner is eligible to submit this tax proof
			let block_peer = T::AuthorityProvider::xor_closest_authority(vote_proof)
				.ok_or(Error::<T>::InvalidSubmitter)?;

			ensure!(block_peer.authority_id == authority_id, Error::<T>::InvalidSubmitter);

			let parent_hash = <frame_system::Pallet<T>>::parent_hash();

			let message = BlockVote::vote_proof_signature_message(&parent_hash, vote_proof);
			let Ok(signature) = AuthoritySignature::<T>::decode(&mut signature.as_ref()) else {
				return Err(Error::<T>::InvalidAuthoritySignature.into())
			};
			ensure!(
				authority_id.verify(&message, &signature),
				Error::<T>::InvalidAuthoritySignature
			);

			// TODO: verify channel pass authority
			// let channel_pass_hash = channel_pass.hash();

			Ok(())
		}
	}

	pub type AuthoritySignature<T> =
		<<T as crate::pallet::Config>::AuthorityId as RuntimeAppPublic>::Signature;
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

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let seal = match call {
				Call::apply { seal } => seal,
				_ => return Err(InherentError::MissingSeal),
			};
			let digest = data
				.digest()
				.expect("Could not decode Block seal digest data")
				.expect("Block seal digest data must be provided");

			ensure!(seal.matches(digest), InherentError::InvalidSeal);
			Ok(())
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
			<LastBlockSealerInfo<T>>::get().expect("BlockSealer must be set")
		}
	}

	impl<T: Config> Get<BlockSealInherent> for Pallet<T> {
		fn get() -> BlockSealInherent {
			<TempSealInherent<T>>::get().expect("Seal inherent must be set")
		}
	}
}
