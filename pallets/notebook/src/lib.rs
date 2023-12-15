#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

pub use pallet::*;
pub use ulx_notary_audit::VerifyError as NotebookVerifyError;
use ulx_primitives::{notary::NotaryProvider, notebook::NotebookNumber};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

const LOG_TARGET: &str = "runtime::notebook";

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use binary_merkle_tree::merkle_proof;
	use codec::alloc::string::ToString;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_core::{crypto::AccountId32, H256};
	use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
	use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

	use ulx_notary_audit::{notebook_verify, AccountHistoryLookupError, NotebookHistoryLookup};
	use ulx_primitives::{
		block_vote::VoteMinimum,
		localchain::{BestBlockVoteProofT, BlockVote, BlockVoteT},
		notary::{NotaryId, NotaryNotebookKeyDetails, NotaryNotebookVoteDetails, NotarySignature},
		notebook::{AccountOrigin, Notebook, NotebookHeader},
		tick::Tick,
		BlockVotingProvider, ChainTransferLookup, MerkleProof, NotebookEventHandler,
		NotebookProvider, NotebookVotes, TickProvider,
	};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId32, Hash = H256> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		type EventHandler: NotebookEventHandler;

		type NotaryProvider: NotaryProvider<<Self as frame_system::Config>::Block>;

		type ChainTransferLookup: ChainTransferLookup<Self::Nonce, Self::AccountId>;

		type BlockVotingProvider: BlockVotingProvider<Self::Block>;
		type TickProvider: TickProvider;
	}

	const MAX_NOTEBOOK_DETAILS_PER_NOTARY: u32 = 3;

	/// Double storage map of notary id + notebook # to the change root
	#[pallet::storage]
	pub(super) type NotebookChangedAccountsRootByNotary<T: Config> = StorageDoubleMap<
		Hasher1 = Blake2_128Concat,
		Hasher2 = Twox64Concat,
		Key1 = NotaryId,
		Key2 = NotebookNumber,
		Value = H256,
		QueryKind = OptionQuery,
	>;

	/// Storage map of account origin (notary_id, notebook, account_uid) to the last
	/// notebook containing this account in the changed accounts merkle root
	/// (NotebookChangedAccountsRootByNotary)
	#[pallet::storage]
	pub(super) type AccountOriginLastChangedNotebookByNotary<T: Config> = StorageDoubleMap<
		Hasher1 = Blake2_128Concat,
		Hasher2 = Blake2_128Concat,
		Key1 = NotaryId,
		Key2 = AccountOrigin,
		Value = NotebookNumber,
		QueryKind = OptionQuery,
	>;

	/// List of last few notebook details by notary. The bool is whether the notebook was received
	/// in the appropriate "tick"
	#[pallet::storage]
	pub(super) type LastNotebookDetailsByNotary<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		NotaryId,
		BoundedVec<(NotaryNotebookKeyDetails, bool), ConstU32<MAX_NOTEBOOK_DETAILS_PER_NOTARY>>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NotebookSubmitted { notary_id: NotaryId, notebook_number: NotebookNumber },
	}

	#[pallet::error]
	pub enum Error<T> {
		UnapprovedNotary,
		MissingNotebookNumber,
		NotebookTickAlreadyUsed,
		IncorrectBlockHeight,
		NoTickDigestFound,
		/// The secret or secret hash of the parent notebook do not match
		InvalidSecretProvided,
		CouldNotDecodeVote,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn submit(
			origin: OriginFor<T>,
			header: NotebookHeader,
			_hash: H256,
			// since signature verification is done in `validate_unsigned`
			// we can skip doing it here again.
			_signature: NotarySignature,
		) -> DispatchResult {
			ensure_none(origin)?;
			let notebook_number = header.notebook_number;
			let notary_id = header.notary_id;

			let mut notary_notebook_details = <LastNotebookDetailsByNotary<T>>::get(notary_id);

			if let Some((parent, _)) = notary_notebook_details.first() {
				ensure!(
					notebook_number == parent.notebook_number + 1,
					Error::<T>::MissingNotebookNumber
				);
				ensure!(parent.tick < header.tick, Error::<T>::NotebookTickAlreadyUsed);

				// check secret
				if let Some(secret) = header.parent_secret {
					let secret_hash = NotebookHeader::create_secret_hash(
						secret,
						parent.block_votes_root,
						parent.notebook_number,
					);
					ensure!(secret_hash == parent.secret_hash, Error::<T>::InvalidSecretProvided);
				}
			}

			T::EventHandler::notebook_submitted(&header)?;

			if notary_notebook_details.len() >= MAX_NOTEBOOK_DETAILS_PER_NOTARY as usize {
				notary_notebook_details.pop();
			}

			let _ = notary_notebook_details.try_insert(
				0,
				(
					NotaryNotebookKeyDetails {
						block_votes_root: header.block_votes_root,
						secret_hash: header.secret_hash,
						notebook_number,
						tick: header.tick,
					},
					T::TickProvider::current_tick() == header.tick,
				),
			);
			<LastNotebookDetailsByNotary<T>>::insert(notary_id, notary_notebook_details);

			<NotebookChangedAccountsRootByNotary<T>>::insert(
				notary_id,
				notebook_number,
				header.changed_accounts_root,
			);

			for account_origin in header.changed_account_origins.into_iter() {
				<AccountOriginLastChangedNotebookByNotary<T>>::insert(
					notary_id,
					account_origin,
					notebook_number,
				);
			}

			Self::deposit_event(Event::NotebookSubmitted {
				notary_id,
				notebook_number: header.notebook_number,
			});

			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit { header, hash, signature } = call {
				let mut last_notebook_number = None;
				if let Some((notebook, _)) =
					<LastNotebookDetailsByNotary<T>>::get(header.notary_id).first()
				{
					ensure!(
						header.notebook_number > notebook.notebook_number,
						InvalidTransaction::Stale
					);
					last_notebook_number = Some(notebook.notebook_number);
				}

				// make the sender provide the hash. We'll just reject it if it's bad
				ensure!(
					T::NotaryProvider::verify_signature(
						header.notary_id,
						// allow the signature to come from the latest finalized block
						header.finalized_block_number.into(),
						&hash,
						&signature
					),
					InvalidTransaction::BadProof
				);

				// verify the hash is valid
				let calculated_hash = header.hash();
				ensure!(calculated_hash == *hash, InvalidTransaction::BadProof);

				let mut tx_builder = ValidTransaction::with_tag_prefix("Notebook")
					.priority(TransactionPriority::MAX)
					.and_provides((header.notary_id, header.notebook_number))
					.longevity(500u32.into())
					.propagate(true);

				if header.notebook_number > 1 && last_notebook_number.is_some() {
					tx_builder =
						tx_builder.and_requires((header.notary_id, header.notebook_number - 1));
				}

				tx_builder.build()
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	pub struct LocalchainHistoryLookup<T: Config> {
		_phantom: PhantomData<T>,
	}
	impl<T: Config> LocalchainHistoryLookup<T> {
		pub fn new() -> Self {
			Self { _phantom: Default::default() }
		}
	}

	impl<T: Config> NotebookHistoryLookup for LocalchainHistoryLookup<T> {
		fn get_account_changes_root(
			&self,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
		) -> Result<H256, AccountHistoryLookupError> {
			<NotebookChangedAccountsRootByNotary<T>>::get(notary_id, notebook_number)
				.ok_or(AccountHistoryLookupError::RootNotFound)
		}

		fn get_last_changed_notebook(
			&self,
			notary_id: NotaryId,
			account_origin: AccountOrigin,
		) -> Result<NotebookNumber, AccountHistoryLookupError> {
			<AccountOriginLastChangedNotebookByNotary<T>>::get(notary_id, account_origin)
				.ok_or(AccountHistoryLookupError::LastChangeNotFound)
		}

		fn is_valid_transfer_to_localchain(
			&self,
			notary_id: NotaryId,
			account_id: &AccountId32,
			nonce: u32,
		) -> Result<bool, AccountHistoryLookupError> {
			if T::ChainTransferLookup::is_valid_transfer_to_localchain(
				notary_id,
				account_id,
				nonce.into(),
			) {
				Ok(true)
			} else {
				Err(AccountHistoryLookupError::InvalidTransferToLocalchain)
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn audit_notebook(
			_version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			header_hash: H256,
			block_vote_minimums: &BTreeMap<<T::Block as BlockT>::Hash, VoteMinimum>,
			bytes: &Vec<u8>,
		) -> Result<NotebookVotes, NotebookVerifyError> {
			if let Some((notebook, _)) = <LastNotebookDetailsByNotary<T>>::get(notary_id).first() {
				ensure!(
					notebook_number > notebook.notebook_number,
					NotebookVerifyError::NotebookTooOld
				);
			}

			let notebook = Notebook::decode(&mut bytes.as_ref()).map_err(|e| {
				log::warn!(
					target: LOG_TARGET,
					"Notebook audit failed to decode for notary {notary_id}, notebook {notebook_number}: {:?}", e.to_string()
				);
				NotebookVerifyError::DecodeError
			})?;

			ensure!(
				notebook.header.hash() == header_hash,
				NotebookVerifyError::InvalidNotebookHeaderHash
			);

			ensure!(
				T::NotaryProvider::verify_signature(
					notary_id,
					notebook_number.into(),
					&notebook.hash,
					&notebook.signature
				),
				NotebookVerifyError::InvalidNotarySignature
			);

			notebook_verify(&LocalchainHistoryLookup::<T>::new(), &notebook, block_vote_minimums)
				.map_err(|e| {
				log::info!(
					target: LOG_TARGET,
					"Notebook audit failed for notary {notary_id}, notebook {notebook_number}: {:?}", e.to_string()
				);
				e
			})?;

			let notebook_votes = NotebookVotes {
				raw_votes: notebook
					.notarizations
					.iter()
					.map(|notarization| {
						notarization.block_votes.iter().map(|vote| (vote.encode(), vote.power))
					})
					.flatten()
					.collect::<Vec<_>>(),
			};
			Ok(notebook_votes)
		}

		/// Decode the notebook submission into high level details
		pub fn decode_notebook_vote_details(
			call: &Call<T>,
		) -> Option<NotaryNotebookVoteDetails<<T::Block as BlockT>::Hash>> {
			if let Call::submit { header, hash, .. } = call {
				return Some(NotaryNotebookVoteDetails {
					notary_id: header.notary_id,
					notebook_number: header.notebook_number,
					version: header.version as u32,
					finalized_block_number: header.finalized_block_number,
					block_votes_root: header.block_votes_root,
					tick: header.tick,
					secret_hash: header.secret_hash,
					parent_secret: header.parent_secret,
					header_hash: hash.clone(),
					block_votes_count: header.block_votes_count,
					block_voting_power: header.block_voting_power,
					blocks_with_votes: header.blocks_with_votes.to_vec().clone(),
				})
			} else {
				None
			}
		}

		pub fn best_vote_proofs(
			notebook_votes: &BTreeMap<NotaryId, NotebookVotes>,
		) -> Result<
			BoundedVec<BestBlockVoteProofT<<T::Block as BlockT>::Hash>, ConstU32<2>>,
			Error<T>,
		> {
			let Some(parent_key) = T::BlockVotingProvider::parent_voting_key() else {
				return Ok(BoundedVec::new())
			};

			let block_number = <frame_system::Pallet<T>>::block_number();
			if block_number <= 3u32.into() {
				return Ok(BoundedVec::new())
			}

			let grandparent_vote_block_hash =
				<frame_system::Pallet<T>>::block_hash(block_number - 3u32.into());

			let mut best_votes = vec![];

			for (notary_id, votes) in notebook_votes {
				for (index, (vote_bytes, power)) in votes.raw_votes.iter().enumerate() {
					let nonce = BlockVote::calculate_vote_proof(
						power.clone(),
						vote_bytes.clone(),
						*notary_id,
						parent_key,
					);
					best_votes.push((nonce, notary_id, vote_bytes, index));
				}
			}
			best_votes.sort_by(|a, b| a.0.cmp(&b.0));

			let mut result = BoundedVec::new();
			for (vote_proof, notary_id, vote_bytes, index) in best_votes {
				let leafs = notebook_votes
					.get(notary_id)
					.expect("just came from iterating over this map")
					.raw_votes
					.iter()
					.map(|(vote_bytes, _)| vote_bytes)
					.collect::<Vec<_>>();

				let proof = merkle_proof::<BlakeTwo256, _, _>(&leafs, index);
				let vote = BlockVoteT::decode(&mut vote_bytes.as_ref())
					.map_err(|_| Error::<T>::CouldNotDecodeVote)?;

				if vote.grandparent_block_hash != grandparent_vote_block_hash {
					continue
				}

				let best_nonce = BestBlockVoteProofT {
					notary_id: *notary_id,
					vote_proof,
					block_vote: vote.clone(),
					source_notebook_proof: MerkleProof {
						proof: BoundedVec::truncate_from(proof.proof),
						leaf_index: proof.leaf_index as u32,
						number_of_leaves: proof.number_of_leaves as u32,
					},
				};
				if result.try_push(best_nonce).is_err() {
					break
				}
			}
			Ok(result)
		}
	}

	impl<T: Config> NotebookProvider for Pallet<T> {
		fn get_eligible_tick_votes_root(
			notary_id: NotaryId,
			tick: Tick,
		) -> Option<(H256, NotebookNumber)> {
			let history = LastNotebookDetailsByNotary::<T>::get(notary_id);
			for (entry, is_eligible) in history {
				if entry.tick == tick && is_eligible {
					return Some((entry.block_votes_root, entry.notebook_number))
				}
			}
			None
		}
	}
}
