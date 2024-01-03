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
	use log::info;
	use sp_core::{crypto::AccountId32, H256};
	use sp_runtime::traits::Block as BlockT;
	use sp_std::{collections::btree_map::BTreeMap, vec, vec::Vec};

	use ulx_notary_audit::{notebook_verify, AccountHistoryLookupError, NotebookHistoryLookup};
	use ulx_primitives::{
		block_vote::VoteMinimum,
		inherents::{NotebookInherentData, NotebookInherentError},
		notary::{NotaryId, NotaryNotebookKeyDetails, NotaryNotebookVoteDetails},
		notebook::{AccountOrigin, Notebook, NotebookHeader},
		tick::Tick,
		Balance, BlockVotingProvider, ChainTransferLookup, NotaryNotebookVotes,
		NotebookDigest as NotebookDigestT, NotebookEventHandler, NotebookProvider, NotebookSecret,
		SignedNotebookHeader, TickProvider, NOTEBOOKS_DIGEST_ID,
	};

	use super::*;

	type NotebookDigest = NotebookDigestT<NotebookVerifyError>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// This pallet is the entry point of notebooks to be submitted to the runtime. From here, there
	/// are multiple pallets that observe the notebooks and act on them. The main activity here is
	/// to confirm the inherent submitted accurately reflects the digest of notebooks, along with
	/// tracking for any notaries that failed audits and need to be locked.
	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId32, Hash = H256> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		type EventHandler: NotebookEventHandler;

		type NotaryProvider: NotaryProvider<<Self as frame_system::Config>::Block>;

		type ChainTransferLookup: ChainTransferLookup<Self::Nonce, Self::AccountId, Balance>;

		type BlockVotingProvider: BlockVotingProvider<Self::Block>;
		type TickProvider: TickProvider<Self::Block>;
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

	/// List of last few notebook details by notary. The bool is whether the notebook is eligible
	/// for votes (received at correct tick and audit passed)
	#[pallet::storage]
	pub(super) type LastNotebookDetailsByNotary<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		NotaryId,
		BoundedVec<(NotaryNotebookKeyDetails, bool), ConstU32<MAX_NOTEBOOK_DETAILS_PER_NOTARY>>,
		ValueQuery,
	>;

	/// The notebooks included in this block
	#[pallet::storage]
	pub(super) type BlockNotebooks<T: Config> = StorageValue<_, NotebookDigest, ValueQuery>;

	/// Temporary store a copy of the notebook digest in storage
	#[pallet::storage]
	pub(super) type TempNotebookDigest<T: Config> = StorageValue<_, NotebookDigest, OptionQuery>;

	/// Notaries locked for failing audits
	/// TODO: we need a mechanism to unlock a notary with "Fixes"
	#[pallet::storage]
	#[pallet::getter(fn notary_failed_audit_by_id)]
	pub(super) type NotariesLockedForFailedAudit<T: Config> =
		StorageMap<_, Blake2_128Concat, NotaryId, (NotebookNumber, Tick, NotebookVerifyError)>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NotebookSubmitted {
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
		},
		NotebookAuditFailure {
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			first_failure_reason: NotebookVerifyError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// This notebook has already been submitted
		DuplicateNotebookNumber,
		/// Notebooks received out of order
		MissingNotebookNumber,
		/// A notebook was already provided at this tick
		NotebookTickAlreadyUsed,
		/// The signature of the notebook is invalid
		InvalidNotebookSignature,
		/// The secret or secret hash of the parent notebook do not match
		InvalidSecretProvided,
		/// Could not decode the scale bytes of the notebook
		CouldNotDecodeNotebook,
		/// The notebook digest was included more than once
		DuplicateNotebookDigest,
		/// The notebook digest was not included
		MissingNotebookDigest,
		/// The notebook digest did not match the included notebooks
		InvalidNotebookDigest,
		/// Multiple inherents provided
		MultipleNotebookInherentsProvided,
		/// Unable to track the notebook change list
		InternalError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let digest = <frame_system::Pallet<T>>::digest();
			for log in digest.logs.iter() {
				if let Some(digest) = log.pre_runtime_try_to::<NotebookDigest>(&NOTEBOOKS_DIGEST_ID)
				{
					assert!(
						!<TempNotebookDigest<T>>::exists(),
						"Notebook digest can only be provided once!"
					);
					<TempNotebookDigest<T>>::put(digest);
				}
			}

			assert_ne!(<TempNotebookDigest<T>>::get(), None, "No valid notebook digest was found.");

			T::DbWeight::get().reads_writes(1, 1)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn submit(
			origin: OriginFor<T>,
			notebooks: Vec<SignedNotebookHeader>,
		) -> DispatchResult {
			ensure_none(origin)?;
			info!(
				target: LOG_TARGET,
				"Notebook inherent submitted with {} notebooks", notebooks.len()
			);

			// Take this value the first time. should reject a second time
			let notebook_digest = <TempNotebookDigest<T>>::take()
				.ok_or(Error::<T>::MultipleNotebookInherentsProvided)?;

			ensure!(
				notebook_digest.notebooks.len() == notebooks.len(),
				Error::<T>::InvalidNotebookDigest
			);

			let mut notebooks = notebooks;
			notebooks.sort_by(|a, b| a.header.notebook_number.cmp(&b.header.notebook_number));

			for SignedNotebookHeader { header, signature } in notebooks {
				let did_pass_audit = Self::check_audit_result(
					header.notary_id,
					header.notebook_number,
					header.tick,
					&notebook_digest,
				)?;

				Self::verify_notebook_order(&header)?;
				// make the sender provide the hash. We'll just reject it if it's bad
				ensure!(
					T::NotaryProvider::verify_signature(
						header.notary_id,
						// allow the signature to come from the latest finalized block
						header.finalized_block_number.into(),
						&header.hash(),
						&signature
					),
					Error::<T>::InvalidNotebookSignature
				);

				T::EventHandler::notebook_submitted(&header)?;
				Self::process_notebook(header, did_pass_audit)?;
			}

			<BlockNotebooks<T>>::put(notebook_digest);
			Ok(())
		}
	}
	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = NotebookInherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier =
			ulx_primitives::inherents::NOTEBOOKS_INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call>
		where
			InherentData: NotebookInherentData,
		{
			let notebooks = data
				.notebooks()
				.expect("Could not decode notebooks inherent data")
				.expect("Notebooks inherent data must be provided");

			Some(Call::submit { notebooks })
		}

		fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			return Ok(Some(NotebookInherentError::MissingInherent))
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::submit { .. })
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
			amount: Balance,
		) -> Result<bool, AccountHistoryLookupError> {
			if T::ChainTransferLookup::is_valid_transfer_to_localchain(
				notary_id,
				account_id,
				nonce.into(),
				amount,
			) {
				Ok(true)
			} else {
				Err(AccountHistoryLookupError::InvalidTransferToLocalchain)
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn verify_notebook_order(header: &NotebookHeader) -> Result<(), DispatchError> {
			let notebook_number = header.notebook_number;
			let notary_notebook_details = <LastNotebookDetailsByNotary<T>>::get(header.notary_id);

			if let Some((parent, _)) = notary_notebook_details.first() {
				ensure!(
					notebook_number != parent.notebook_number,
					Error::<T>::DuplicateNotebookNumber
				);
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
			} else {
				ensure!(notebook_number == 1, Error::<T>::MissingNotebookNumber);
			}
			Ok(())
		}

		pub(crate) fn check_audit_result(
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			tick: Tick,
			notebook_digest: &NotebookDigest,
		) -> Result<bool, DispatchError> {
			let digest = notebook_digest
				.notebooks
				.iter()
				.find(|n| {
					n.notary_id == notary_id &&
						n.notebook_number == notebook_number &&
						n.tick == tick
				})
				.ok_or(Error::<T>::InvalidNotebookDigest)?;

			if let Some(first_failure_reason) = &digest.audit_first_failure {
				Self::deposit_event(Event::<T>::NotebookAuditFailure {
					notary_id,
					notebook_number,
					first_failure_reason: first_failure_reason.clone(),
				});
				if !Self::notary_failed_audit_by_id(&notary_id).is_some() {
					<NotariesLockedForFailedAudit<T>>::insert(
						notary_id,
						(notebook_number, tick, first_failure_reason.clone()),
					);
				}
				return Ok(false)
			}

			Ok(true)
		}

		pub(crate) fn process_notebook(
			header: NotebookHeader,
			did_pass_audit: bool,
		) -> Result<(), DispatchError> {
			let notary_id = header.notary_id;
			let notebook_number = header.notebook_number;
			let current_tick = T::TickProvider::current_tick();

			<LastNotebookDetailsByNotary<T>>::try_mutate(notary_id, |x| {
				if x.len() >= MAX_NOTEBOOK_DETAILS_PER_NOTARY as usize {
					x.pop();
				}

				let is_vote_eligible = current_tick == header.tick && did_pass_audit;
				x.try_insert(
					0,
					(
						NotaryNotebookKeyDetails {
							block_votes_root: header.block_votes_root,
							parent_secret: header.parent_secret,
							secret_hash: header.secret_hash,
							notebook_number,
							tick: header.tick,
						},
						is_vote_eligible,
					),
				)
			})
			.map_err(|_| Error::<T>::InternalError)?;

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

		pub fn audit_notebook(
			_version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			header_hash: H256,
			block_vote_minimums: &BTreeMap<<T::Block as BlockT>::Hash, VoteMinimum>,
			bytes: &Vec<u8>,
		) -> Result<NotaryNotebookVotes, NotebookVerifyError> {
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
				info!(
					target: LOG_TARGET,
					"Notebook audit failed for notary {notary_id}, notebook {notebook_number}: {:?}", e.to_string()
				);
				e
			})?;

			let notebook_votes = NotaryNotebookVotes {
				notary_id,
				notebook_number,
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
		pub fn decode_signed_raw_notebook_header(
			header_data: Vec<u8>,
		) -> Result<NotaryNotebookVoteDetails<<T::Block as BlockT>::Hash>, DispatchError> {
			let header = NotebookHeader::decode(&mut header_data.as_ref())
				.map_err(|_| Error::<T>::CouldNotDecodeNotebook)?;

			return Ok(NotaryNotebookVoteDetails {
				notary_id: header.notary_id,
				notebook_number: header.notebook_number,
				version: header.version as u32,
				finalized_block_number: header.finalized_block_number,
				tick: header.tick,
				header_hash: header.hash(),
				block_votes_count: header.block_votes_count,
				block_voting_power: header.block_voting_power,
				blocks_with_votes: header.blocks_with_votes.to_vec().clone(),
			})
		}

		pub fn latest_notebook_by_notary() -> BTreeMap<NotaryId, (NotebookNumber, Tick)> {
			<LastNotebookDetailsByNotary<T>>::iter()
				.map(|(notary_id, details)| {
					let (details, _) = details.first().expect("just iterated over this");
					(notary_id, (details.notebook_number, details.tick))
				})
				.collect()
		}
	}

	impl<T: Config> NotebookProvider for Pallet<T> {
		fn get_eligible_tick_votes_root(
			notary_id: NotaryId,
			tick: Tick,
		) -> Option<(H256, NotebookNumber)> {
			if Self::is_notary_locked_at_tick(notary_id, tick) {
				return None;
			}

			let history = LastNotebookDetailsByNotary::<T>::get(notary_id);
			for (entry, is_eligible) in history {
				if entry.tick == tick && is_eligible {
					return Some((entry.block_votes_root, entry.notebook_number))
				}
			}
			None
		}

		fn notebooks_in_block() -> Vec<(NotaryId, NotebookNumber, Tick)> {
			BlockNotebooks::<T>::get()
				.notebooks
				.iter()
				.map(|n| (n.notary_id, n.notebook_number, n.tick))
				.collect()
		}

		fn notebooks_at_tick(
			tick: Tick,
		) -> Vec<(NotaryId, NotebookNumber, Option<NotebookSecret>)> {
			let mut notebooks = Vec::new();
			for (notary_id, details) in <LastNotebookDetailsByNotary<T>>::iter() {
				for (book, _) in details.iter() {
					if book.tick == tick {
						notebooks.push((notary_id, book.notebook_number, book.parent_secret));
					}
				}
			}
			notebooks
		}

		fn is_notary_locked_at_tick(notary_id: NotaryId, tick: Tick) -> bool {
			if let Some((_, locked_at_tick, _)) = Self::notary_failed_audit_by_id(&notary_id) {
				if locked_at_tick <= tick {
					return true;
				}
			}
			return false
		}
	}
}
