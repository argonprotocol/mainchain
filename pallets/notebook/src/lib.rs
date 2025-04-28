#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use argon_notary_audit::VerifyError as NotebookVerifyError;
pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use alloc::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
	use sp_core::crypto::AccountId32;
	use tracing::{info, trace, warn};

	use super::*;
	use argon_notary_audit::{notebook_verify, AccountHistoryLookupError, NotebookHistoryLookup};
	use argon_primitives::{
		inherents::{NotebookInherentData, NotebookInherentError},
		notary::{
			NotaryNotebookAuditSummary, NotaryNotebookAuditSummaryDecoded,
			NotaryNotebookAuditSummaryDetails, NotaryNotebookDetails, NotaryNotebookKeyDetails,
			NotaryNotebookRawVotes, NotaryProvider, NotaryState,
		},
		notebook::{AccountOrigin, Notebook, NotebookHeader},
		AccountOriginUid, BlockSealSpecProvider, BlockVote, ChainTransfer, ChainTransferLookup,
		Digestset, NotebookDigest as NotebookDigestT, NotebookEventHandler, NotebookProvider,
		NotebookSecret, NotebookSecretHash, SignedNotebookHeader, TickProvider,
		TransferToLocalchainId,
	};

	type NotebookDigest = NotebookDigestT<NotebookVerifyError>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// This pallet is the entry point of notebooks to be submitted to the runtime. From here, there
	/// are multiple pallets that observe the notebooks and act on them. The main activity here is
	/// to confirm the inherent submitted accurately reflects the digest of notebooks, along with
	/// tracking for any notaries that failed audits and need to be locked.
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<AccountId = AccountId32, Hash = H256>
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		type EventHandler: NotebookEventHandler;

		type NotaryProvider: NotaryProvider<<Self as frame_system::Config>::Block, Self::AccountId>;

		type ChainTransferLookup: ChainTransferLookup<Self::AccountId, Balance>;

		type BlockSealSpecProvider: BlockSealSpecProvider<Self::Block>;
		type TickProvider: TickProvider<Self::Block>;

		type Digests: Get<Result<Digestset<NotebookVerifyError, Self::AccountId>, DispatchError>>;
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

	/// Check if the inherent was included
	#[pallet::storage]
	pub(super) type InherentIncluded<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Notaries locked for failing audits
	#[pallet::storage]
	#[pallet::getter(fn notary_failed_audit_by_id)]
	pub(super) type NotariesLockedForFailedAudit<T: Config> =
		StorageMap<_, Blake2_128Concat, NotaryId, (NotebookNumber, Tick, NotebookVerifyError)>;

	/// Notaries ready to start reprocessing at a given notebook number
	#[pallet::storage]
	pub(super) type LockedNotaryReadyForReprocess<T: Config> =
		StorageMap<_, Blake2_128Concat, NotaryId, NotebookNumber>;

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
			notebook_hash: H256,
			first_failure_reason: NotebookVerifyError,
		},
		NotebookReadyForReprocess {
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
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
		/// A notebook was submitted for a notary that failed audit, which is not allowed
		NotebookSubmittedForLockedNotary,
		/// Invalid reprocess notebook
		InvalidReprocessNotebook,
		/// Invalid notary operator
		InvalidNotaryOperator,
		/// Invalid notebook submission tick
		InvalidNotebookSubmissionTick,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			T::DbWeight::get().reads_writes(1, 1)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			// According to parity, the only way to ensure that a mandatory inherent is included
			// is by checking on block finalization that the inherent set a particular storage item:
			// https://github.com/paritytech/polkadot-sdk/issues/2841#issuecomment-1876040854
			assert!(
				InherentIncluded::<T>::take(),
				"Block invalid, missing inherent `notebooks::submit`"
			);
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
				notebooks = notebooks.len(),
				is_duplicated = InherentIncluded::<T>::get(),
				"Notebook inherent submitted",
			);

			ensure!(!InherentIncluded::<T>::get(), Error::<T>::MultipleNotebookInherentsProvided);

			InherentIncluded::<T>::put(true);

			// CRITICAL NOTE: very important to only have dispatch errors that are from the assembly
			// of the notebooks, not the notebooks themselves. Otherwise, we will stall here as
			// the node doesn't know.

			// Take this value the first time. should reject a second time
			let notebook_digest = T::Digests::get()?.notebooks;

			ensure!(
				notebook_digest.notebooks.len() == notebooks.len(),
				Error::<T>::InvalidNotebookDigest
			);

			let current_tick = T::TickProvider::current_tick();
			let mut notebooks = notebooks;
			notebooks.sort_by(|a, b| a.header.notebook_number.cmp(&b.header.notebook_number));

			for SignedNotebookHeader { header, signature } in notebooks {
				let notebook_number = header.notebook_number;
				let notary_id = header.notary_id;

				trace!(
					?notebook_number,
					?notary_id,
					locked = Self::is_notary_locked_at_tick(notary_id, header.tick),
					"Processing notebook",
				);

				ensure!(header.tick < current_tick, Error::<T>::InvalidNotebookSubmissionTick);
				ensure!(
					!Self::is_notary_locked_at_tick(notary_id, header.tick),
					Error::<T>::NotebookSubmittedForLockedNotary
				);
				if let Some(reprocess_notebook_number) =
					LockedNotaryReadyForReprocess::<T>::get(notary_id)
				{
					ensure!(
						reprocess_notebook_number == notebook_number,
						Error::<T>::InvalidReprocessNotebook
					);
				}

				let header_hash = header.hash();

				// Failure case(s): audit not in digest (created by node)
				let did_pass_audit = Self::check_audit_result(
					notary_id,
					notebook_number,
					header.tick,
					header_hash,
					&notebook_digest,
					header.parent_secret,
				)
				.inspect_err(|e| {
					warn!(?notebook_number, ?notary_id, ?e, "Notebook audit failed",);
				})?;

				// Failure cases: all based on notebooks not in order of runtime state; controllable
				// by node
				Self::verify_notebook_order(&header).inspect_err(|e| {
					warn!(
						?notebook_number,
						?notary_id,
						?e,
						"Notebook order verification failed for notary",
					);
				})?;

				// Failure case: invalid signature is not possible without bypassing audit
				ensure!(
					T::NotaryProvider::verify_signature(
						notary_id,
						// we validate signatures based on the latest tick
						header.tick,
						&header_hash,
						&signature
					),
					Error::<T>::InvalidNotebookSignature
				);

				if did_pass_audit {
					T::EventHandler::notebook_submitted(&header);

					Self::process_notebook(header);
				}
				trace!(
					?notary_id,
					?notebook_number,
					did_pass_audit,
					"Notebook processing complete",
				);
			}

			<BlockNotebooks<T>>::put(notebook_digest);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn unlock(origin: OriginFor<T>, notary_id: NotaryId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let notary_operator = T::NotaryProvider::notary_operator_account_id(notary_id)
				.ok_or(Error::<T>::InvalidNotaryOperator)?;
			ensure!(who == notary_operator, Error::<T>::InvalidNotaryOperator);
			if let Some((notebook_number, _, _)) =
				<NotariesLockedForFailedAudit<T>>::take(notary_id)
			{
				LockedNotaryReadyForReprocess::<T>::insert(notary_id, notebook_number);
				Self::deposit_event(Event::NotebookReadyForReprocess {
					notary_id,
					notebook_number,
				});
			}
			Ok(())
		}
	}
	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = NotebookInherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier =
			argon_primitives::inherents::NOTEBOOKS_INHERENT_IDENTIFIER;

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
			Ok(Some(NotebookInherentError::MissingInherent))
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::submit { .. })
		}
	}

	#[derive(DefaultNoBound)]
	pub struct LocalchainHistoryLookup<T: Config> {
		pub last_changed_notebooks:
			BTreeMap<(NotaryId, NotebookNumber, AccountOriginUid), NotebookNumber>,
		pub account_changes_root: BTreeMap<(NotaryId, NotebookNumber), H256>,
		pub used_transfers_to_localchain: BTreeSet<TransferToLocalchainId>,
		_marker: PhantomData<T>,
	}
	impl<T: Config> LocalchainHistoryLookup<T> {
		pub fn new() -> Self {
			Default::default()
		}

		pub fn add_audit_summary(&mut self, audit_summary: NotaryNotebookAuditSummaryDecoded) {
			let notary_id = audit_summary.notary_id;
			let notebook_number = audit_summary.notebook_number;
			let details = audit_summary.details;
			for id in details.used_transfers_to_localchain.iter() {
				self.used_transfers_to_localchain.insert(*id);
			}
			self.account_changes_root
				.insert((notary_id, notebook_number), details.changed_accounts_root);
			for account_origin in details.account_changelist.iter() {
				self.last_changed_notebooks.insert(
					(notary_id, account_origin.notebook_number, account_origin.account_uid),
					notebook_number,
				);
			}
		}
	}

	impl<T: Config> NotebookHistoryLookup for LocalchainHistoryLookup<T> {
		fn get_account_changes_root(
			&self,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
		) -> Result<H256, AccountHistoryLookupError> {
			if let Some(root) = self.account_changes_root.get(&(notary_id, notebook_number)) {
				return Ok(*root);
			}
			<NotebookChangedAccountsRootByNotary<T>>::get(notary_id, notebook_number)
				.ok_or(AccountHistoryLookupError::RootNotFound)
		}

		fn get_last_changed_notebook(
			&self,
			notary_id: NotaryId,
			account_origin: AccountOrigin,
		) -> Result<NotebookNumber, AccountHistoryLookupError> {
			if let Some(notebook) = self.last_changed_notebooks.get(&(
				notary_id,
				account_origin.notebook_number,
				account_origin.account_uid,
			)) {
				return Ok(*notebook);
			}
			<AccountOriginLastChangedNotebookByNotary<T>>::get(notary_id, account_origin)
				.ok_or(AccountHistoryLookupError::LastChangeNotFound)
		}

		fn is_valid_transfer_to_localchain(
			&self,
			notary_id: NotaryId,
			transfer_id: TransferToLocalchainId,
			account_id: &AccountId32,
			amount: Balance,
			for_notebook_tick: Tick,
		) -> Result<bool, AccountHistoryLookupError> {
			if self.used_transfers_to_localchain.contains(&transfer_id) {
				return Err(AccountHistoryLookupError::InvalidTransferToLocalchain);
			}
			if T::ChainTransferLookup::is_valid_transfer_to_localchain(
				notary_id,
				transfer_id,
				account_id,
				amount,
				for_notebook_tick,
			) {
				Ok(true)
			} else {
				Err(AccountHistoryLookupError::InvalidTransferToLocalchain)
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_state(notary_id: NotaryId) -> NotaryState<NotebookVerifyError> {
			if let Some((notebook_number, at_tick, failed_audit_reason)) =
				<NotariesLockedForFailedAudit<T>>::get(notary_id)
			{
				return NotaryState::Locked { notebook_number, at_tick, failed_audit_reason }
			}
			if let Some(reprocess_notebook_number) =
				LockedNotaryReadyForReprocess::<T>::get(notary_id)
			{
				return NotaryState::Reactivated { reprocess_notebook_number }
			}

			NotaryState::Active
		}
		/// Verify the notebook order is correct compared to the block state. This can fail as it is
		/// up to the node to do this correctly
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
			} else {
				ensure!(notebook_number == 1, Error::<T>::MissingNotebookNumber);
			}
			Ok(())
		}

		/// Look up the audit result in the given digest and check if the audit passed. If it did
		/// not pass, lock the notary and return false.
		///
		/// NOTE: this must ONLY fail for things that are the node's responsibility to check.
		pub(crate) fn check_audit_result(
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			tick: Tick,
			notebook_hash: H256,
			notebook_digest: &NotebookDigest,
			parent_secret: Option<NotebookSecret>,
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

			let mut verify_error = digest.audit_first_failure.clone();
			if verify_error.is_none() {
				let notary_notebook_details = <LastNotebookDetailsByNotary<T>>::get(notary_id);
				if let Some((parent, _)) = notary_notebook_details.first() {
					// check secret
					if let Some(secret) = parent_secret {
						let secret_hash = NotebookHeader::create_secret_hash(
							secret,
							parent.block_votes_root,
							parent.notebook_number,
						);
						if secret_hash != parent.secret_hash {
							verify_error = Some(NotebookVerifyError::InvalidSecretProvided);
						}
					}
				}
			}
			if let Some(first_failure_reason) = verify_error {
				Self::deposit_event(Event::<T>::NotebookAuditFailure {
					notary_id,
					notebook_number,
					notebook_hash,
					first_failure_reason: first_failure_reason.clone(),
				});
				if Self::notary_failed_audit_by_id(notary_id).is_none() {
					<NotariesLockedForFailedAudit<T>>::insert(
						notary_id,
						(notebook_number, tick, first_failure_reason.clone()),
					);
				}
				return Ok(false);
			}

			Ok(true)
		}

		pub(crate) fn process_notebook(header: NotebookHeader) {
			let notary_id = header.notary_id;
			let notebook_number = header.notebook_number;

			<LastNotebookDetailsByNotary<T>>::try_mutate(notary_id, |x| {
				if x.is_full() {
					x.pop();
				}

				let voting_schedule = T::TickProvider::voting_schedule();
				// we submit notebooks for the previous tick
				let is_vote_eligible = header.tick == voting_schedule.notebook_tick();
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
			.expect("we've pruned this list, so should not be possible to fail");

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
		}

		#[allow(clippy::too_many_arguments)]
		pub fn audit_notebook(
			_version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			notebook_tick: Tick,
			header_hash: H256,
			bytes: &Vec<u8>,
			raw_audit_dependency_summaries: Vec<NotaryNotebookAuditSummary>,
		) -> Result<NotaryNotebookRawVotes, NotebookVerifyError> {
			let mut history_lookup = LocalchainHistoryLookup::<T>::new();

			let mut audit_dependency_summaries: Vec<NotaryNotebookAuditSummaryDecoded> =
				raw_audit_dependency_summaries
					.into_iter()
					.map(|audit_summary| {
						audit_summary.try_into().map_err(|_| {
							warn!(

								"Notebook audit failed to decode for notary {notary_id}, notebook {notebook_number}"
							);
							NotebookVerifyError::DecodeError
						})
					})
					.collect::<Result<Vec<_>, _>>()?;

			audit_dependency_summaries.sort_by(|a, b| {
				let tick_cmp = a.tick.cmp(&b.tick);
				if tick_cmp != core::cmp::Ordering::Equal {
					return tick_cmp;
				}
				a.notebook_number.cmp(&b.notebook_number)
			});

			let mut parent_secret_hash = NotebookSecretHash::zero();
			let mut parent_block_votes_root = H256::zero();
			let previous_notebook = notebook_number.saturating_sub(1);
			let mut last_notebook_processed: NotebookNumber =
				<LastNotebookDetailsByNotary<T>>::get(notary_id)
					.first()
					.map(|(details, _)| {
						if details.notebook_number == previous_notebook {
							parent_secret_hash = details.secret_hash;
							parent_block_votes_root = details.block_votes_root;
						}
						details.notebook_number
					})
					.unwrap_or_default();

			for audit_summary in audit_dependency_summaries {
				ensure!(
					audit_summary.notebook_number == last_notebook_processed + 1,
					NotebookVerifyError::CatchupNotebooksMissing
				);
				if audit_summary.notebook_number == previous_notebook {
					parent_secret_hash = audit_summary.details.secret_hash;
					parent_block_votes_root = audit_summary.details.block_votes_root;
				}

				last_notebook_processed = audit_summary.notebook_number;
				history_lookup.add_audit_summary(audit_summary);
			}

			ensure!(
				notebook_number == last_notebook_processed + 1,
				NotebookVerifyError::CatchupNotebooksMissing
			);

			let notebook = Notebook::decode(&mut bytes.as_ref()).map_err(|e| {
				warn!(

					"Notebook audit failed to decode for notary {notary_id}, notebook {notebook_number}: {:?}", e.to_string()
				);
				NotebookVerifyError::DecodeError
			})?;

			ensure!(
				notebook.header.notebook_number == notebook_number,
				NotebookVerifyError::InvalidNotebookHeaderHash
			);
			ensure!(
				notebook.header.hash() == header_hash,
				NotebookVerifyError::InvalidNotebookHeaderHash
			);

			ensure!(
				T::NotaryProvider::verify_signature(
					notary_id,
					notebook_tick,
					&notebook.hash,
					&notebook.signature
				),
				NotebookVerifyError::InvalidNotarySignature
			);

			if let Some(secret) = notebook.header.parent_secret {
				let secret_hash = NotebookHeader::create_secret_hash(
					secret,
					parent_block_votes_root,
					notebook.header.notebook_number.saturating_sub(1),
				);
				ensure!(
					secret_hash == parent_secret_hash,
					NotebookVerifyError::InvalidSecretProvided
				);
			}

			let channel_hold_expiration_ticks =
				T::TickProvider::ticker().channel_hold_expiration_ticks;
			let notary_operator = T::NotaryProvider::notary_operator_account_id(notary_id)
				.ok_or(NotebookVerifyError::InvalidNotarySignature)?;

			notebook_verify(
				&history_lookup,
				&notebook,
				&notary_operator,
				channel_hold_expiration_ticks,
			)
			.inspect_err(|e| {
				info!(

					"Notebook audit failed for notary {notary_id}, notebook {notebook_number}: {:?}", e.to_string()
				);
			})?;

			let mut block_votes = BTreeMap::<(AccountId, u32), BlockVote>::new();
			for notarization in notebook.notarizations.iter() {
				for vote in notarization.block_votes.iter() {
					let key = (vote.account_id.clone(), vote.index);
					block_votes.insert(key, vote.clone());
				}
			}

			let block_votes =
				block_votes.values().map(|vote| (vote.encode(), vote.power)).collect::<Vec<_>>();
			Ok(NotaryNotebookRawVotes { notary_id, notebook_number, raw_votes: block_votes })
		}

		/// Decode the notebook submission into high level details
		pub fn decode_signed_raw_notebook_header(
			header_data: Vec<u8>,
		) -> Result<NotaryNotebookDetails<<T::Block as BlockT>::Hash>, DispatchError> {
			let header = NotebookHeader::decode(&mut header_data.as_ref())
				.map_err(|_| Error::<T>::CouldNotDecodeNotebook)?;

			let summary = NotaryNotebookAuditSummaryDetails {
				changed_accounts_root: header.changed_accounts_root,
				account_changelist: header.changed_account_origins.clone().to_vec(),
				used_transfers_to_localchain: header
					.chain_transfers
					.iter()
					.filter_map(|t| match t {
						ChainTransfer::ToLocalchain { transfer_id } => Some(*transfer_id),
						_ => None,
					})
					.collect(),
				secret_hash: header.secret_hash,
				block_votes_root: header.block_votes_root,
			};

			Ok(NotaryNotebookDetails {
				notary_id: header.notary_id,
				notebook_number: header.notebook_number,
				version: header.version as u32,
				tick: header.tick,
				header_hash: header.hash(),
				block_votes_count: header.block_votes_count,
				block_voting_power: header.block_voting_power,
				blocks_with_votes: header.blocks_with_votes.to_vec().clone(),
				raw_audit_summary: summary.encode(),
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
					return Some((entry.block_votes_root, entry.notebook_number));
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
			if let Some((_, locked_at_tick, _)) = Self::notary_failed_audit_by_id(notary_id) {
				if locked_at_tick <= tick {
					return true;
				}
			}
			false
		}
	}
}
