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
	use codec::alloc::string::ToString;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_api::BlockT;
	use sp_core::{crypto::AccountId32, H256};
	use sp_io::hashing::blake2_256;
	use sp_runtime::traits::UniqueSaturatedInto;
	use sp_std::collections::btree_map::BTreeMap;

	use ulx_notary_audit::{notebook_verify, AccountHistoryLookupError, NotebookHistoryLookup};
	use ulx_primitives::{
		block_seal::BlockVoteEligibility,
		notary::{NotaryId, NotaryNotebookKeyDetails, NotaryNotebookVoteDetails, NotarySignature},
		notebook::{AccountOrigin, Notebook, NotebookHeader},
		ChainTransferLookup, NotebookEventHandler, NotebookProvider,
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

	#[pallet::storage]
	pub(super) type LastNotebookDetailsByNotary<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		NotaryId,
		BoundedVec<NotaryNotebookKeyDetails, ConstU32<MAX_NOTEBOOK_DETAILS_PER_NOTARY>>,
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
		IncorrectBlockHeight,
		/// The secret or secret hash of the parent notebook do not match
		InvalidSecretProvided,
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
			let current_block_number = frame_system::Pallet::<T>::block_number();
			let block_u32: u32 =
				UniqueSaturatedInto::<u32>::unique_saturated_into(current_block_number);

			ensure!(header.block_number == (block_u32 - 1u32), Error::<T>::IncorrectBlockHeight);

			let mut notary_notebook_details = <LastNotebookDetailsByNotary<T>>::get(notary_id);

			if let Some(notebook) = notary_notebook_details.first() {
				ensure!(
					notebook_number == notebook.notebook_number + 1,
					Error::<T>::MissingNotebookNumber
				);

				// check secret
				if let Some(secret) = header.parent_secret {
					ensure!(
						blake2_256(&secret[..]) == notebook.secret_hash.as_bytes(),
						Error::<T>::InvalidSecretProvided
					);
				}
			}

			T::EventHandler::notebook_submitted(&header)?;

			if notary_notebook_details.len() >= MAX_NOTEBOOK_DETAILS_PER_NOTARY as usize {
				notary_notebook_details.pop();
			}

			let _ = notary_notebook_details.try_insert(
				0,
				NotaryNotebookKeyDetails {
					block_votes_root: header.block_votes_root,
					secret_hash: header.secret_hash,
					notebook_number,
					block_number: block_u32,
				},
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
				let current_block_number: u32 =
					frame_system::Pallet::<T>::block_number().unique_saturated_into();

				// if the block is in the future, we can't include it yet in this fork
				if header.block_number > current_block_number {
					return Err(InvalidTransaction::Future.into())
				}

				// if older than 1 block mark it stale to remove from the pool
				if header.block_number < current_block_number - 1 {
					return Err(InvalidTransaction::Stale.into())
				}

				if let Some(notebook) =
					<LastNotebookDetailsByNotary<T>>::get(header.notary_id).first()
				{
					ensure!(
						header.notebook_number > notebook.notebook_number,
						InvalidTransaction::Stale
					);
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
					.longevity(1u32.into())
					.propagate(true);

				if header.notebook_number > 1 {
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
			block_vote_eligibility: &BTreeMap<<T::Block as BlockT>::Hash, BlockVoteEligibility>,
			bytes: &Vec<u8>,
		) -> Result<bool, NotebookVerifyError> {
			if let Some(notebook) = <LastNotebookDetailsByNotary<T>>::get(notary_id).first() {
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

			let is_valid = notebook_verify(
				&LocalchainHistoryLookup::<T>::new(),
				&notebook,
				block_vote_eligibility,
			)
			.map_err(|e| {
				log::info!(
					target: LOG_TARGET,
					"Notebook audit failed for notary {notary_id}, notebook {notebook_number}: {:?}", e.to_string()
				);
				e
			})?;
			Ok(is_valid)
		}

		/// Decode the notebook submission into high level details
		pub fn decode_notebook_vote_details(
			call: &Call<T>,
		) -> Option<NotaryNotebookVoteDetails<<T::Block as BlockT>::Hash>> {
			if let Call::submit { header, hash, .. } = call {
				let key_details = NotaryNotebookKeyDetails {
					notebook_number: header.notebook_number,
					block_votes_root: header.block_votes_root,
					block_number: header.block_number,
					secret_hash: header.secret_hash,
				};
				return Some(NotaryNotebookVoteDetails {
					notary_id: header.notary_id,
					notebook_number: header.notebook_number,
					version: header.version as u32,
					finalized_block_number: header.finalized_block_number,
					key_details,
					header_hash: hash.clone(),
					block_votes: header.block_votes_count,
					block_voting_power: header.block_voting_power,
					blocks_with_votes: header.blocks_with_votes.to_vec().clone(),
					best_nonces: header
						.best_block_nonces
						.iter()
						.map(|(voting_key, best_nonce)| {
							(voting_key.clone(), header.notary_id, best_nonce.clone())
						})
						.collect::<Vec<_>>(),
				})
			} else {
				None
			}
		}
	}

	impl<T: Config> NotebookProvider for Pallet<T> {
		fn get_eligible_block_votes_root(
			notary_id: NotaryId,
			block_number: u32,
		) -> Option<(H256, NotebookNumber)> {
			let history = LastNotebookDetailsByNotary::<T>::get(notary_id);
			for entry in history {
				if entry.block_number == block_number {
					return Some((entry.block_votes_root, entry.notebook_number))
				}
			}
			None
		}
	}
}
