#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::{fmt::Debug, prelude::*};

pub use pallet::*;
use ulx_primitives::{
	notary::{NotaryId, NotaryProvider},
	notebook::{AccountOriginUid, NotebookNumber},
	BlockSealAuthorityId,
};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
const LOG_TARGET: &str = "runtime::localchain::relay";

type CrossNotaryAccountOrigin = (NotaryId, NotebookNumber, AccountOriginUid);
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, Mutate},
			tokens::{Fortitude, Preservation},
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::{
		crypto::AccountId32,
		ed25519::{Public, Signature},
		H256,
	};

	use sp_runtime::{
		app_crypto::ed25519,
		traits::{AccountIdConversion, AtLeast32BitUnsigned, One, UniqueSaturatedInto},
		RuntimeAppPublic, Saturating,
	};
	use sp_std::cmp::min;

	use ulx_primitives::{
		block_seal::HistoricalBlockSealersLookup,
		digests::{FinalizedBlockNeededDigest, FINALIZED_BLOCK_DIGEST_ID},
		notary::{NotaryId, NotarySignature},
		notebook::{AuditedNotebook, ChainTransfer},
	};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config<AccountId = AccountId32> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		type Currency: Mutate<Self::AccountId, Balance = Self::Balance>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		type HistoricalBlockSealersLookup: HistoricalBlockSealersLookup<
			BlockNumberFor<Self>,
			BlockSealAuthorityId,
		>;

		type NotaryProvider: NotaryProvider<<Self as frame_system::Config>::Block>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// How long a transfer should remain in storage before returning.
		#[pallet::constant]
		type TransferExpirationBlocks: Get<u32>;

		/// How many transfers out can be queued per block
		#[pallet::constant]
		type MaxPendingTransfersOutPerBlock: Get<u32>;

		/// How many auditors are expected to sign a notary block.
		#[pallet::constant]
		type RequiredNotebookAuditors: Get<u32>;

		/// Number of blocks to keep around for preventing notebook double-submit
		#[pallet::constant]
		type MaxNotebookBlocksToRemember: Get<u32>;
	}

	#[pallet::storage]
	pub(super) type PendingTransfersOut<T: Config> = StorageDoubleMap<
		Hasher1 = Blake2_128Concat,
		Hasher2 = Twox64Concat,
		Key1 = T::AccountId,
		Key2 = T::Nonce,
		Value = QueuedTransferOut<T::Balance, BlockNumberFor<T>>,
		QueryKind = OptionQuery,
		MaxValues = ConstU32<1_000_000>,
	>;

	#[pallet::storage]
	pub(super) type ExpiringTransfersOut<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<(T::AccountId, T::Nonce), T::MaxPendingTransfersOutPerBlock>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub(super) type FinalizedBlockNumber<T: Config> =
		StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Double storage map of notary id to
	#[pallet::storage]
	pub(super) type NotebookChangedAccountsRootByNotary<T: Config> = StorageDoubleMap<
		Hasher1 = Blake2_128Concat,
		Hasher2 = Blake2_128Concat,
		Key1 = NotaryId,
		Key2 = NotebookNumber,
		Value = H256,
		QueryKind = OptionQuery,
	>;

	/// Storage map of cross-notary account origin (notary_id, notebook, account_uid) to the last
	/// notebook containing this account in the changed accounts merkle root
	/// (NotebookChangedAccountsRootByNotary)
	#[pallet::storage]
	pub(super) type AccountOriginLastChangedNotebook<T: Config> =
		StorageMap<_, Blake2_128Concat, CrossNotaryAccountOrigin, NotebookNumber, OptionQuery>;

	#[pallet::storage]
	pub(super) type LastNotebookNumberByNotary<T: Config> =
		StorageMap<_, Blake2_128Concat, NotaryId, (NotebookNumber, BlockNumberFor<T>), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		TransferToLocalchain {
			account_id: T::AccountId,
			amount: T::Balance,
			nonce: T::Nonce,
			notary_id: NotaryId,
			expiration_block: BlockNumberFor<T>,
		},
		TransferToLocalchainExpired {
			account_id: T::AccountId,
			nonce: T::Nonce,
			notary_id: NotaryId,
		},
		TransferIn {
			account_id: T::AccountId,
			amount: T::Balance,
			notary_id: NotaryId,
		},
		NotebookSubmitted {
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			pinned_to_block_number: BlockNumberFor<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		BadState,
		MaxBlockTransfersExceeded,
		InsufficientFunds,
		InvalidAccountNonce,
		UnapprovedNotary,
		MissingNotebookNumber,
		InvalidPinnedBlockNumber,
		InvalidNotebookSubmissionSignature,
		/// Auditor of a notary block was not a member of the validator set at the time the pinned
		/// finalized block was sealed.
		InvalidNotebookAuditor,
		/// The auditor was not in the first X authorities of the finalized block.
		InvalidNotebookAuditorIndex,
		InvalidNotebookAuditorSignature,
		InvalidNotebookHash,
		InsufficientNotebookSignatures,
		UnfinalizedBlock,
		InsufficientNotarizedFunds,
		TransferNotEligibleForCancellation,
		/// The transfer was already submitted in a previous block
		InvalidOrDuplicatedLocalchainTransfer,
		// A transfer was submitted in a previous block but the expiration block has passed
		NotebookIncludesExpiredLocalchainTransfer,
		InvalidNotaryUsedForTransfer,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
			for (id, mut data) in pre_runtime_digests.into_iter() {
				if id == FINALIZED_BLOCK_DIGEST_ID {
					let decoded =
						FinalizedBlockNeededDigest::<<T as frame_system::Config>::Block>::decode(
							&mut data,
						);
					if let Some(FinalizedBlockNeededDigest { number: block_number, hash: _ }) =
						decoded.ok()
					{
						if block_number > <FinalizedBlockNumber<T>>::get() {
							<FinalizedBlockNumber<T>>::put(block_number);
						}
					}
				}
			}

			let expiring = <ExpiringTransfersOut<T>>::take(block_number);
			for (account_id, nonce) in expiring.into_iter() {
				if let Some(transfer) = <PendingTransfersOut<T>>::take(account_id.clone(), nonce) {
					let _ = T::Currency::transfer(
						&Self::notary_account_id(transfer.notary_id),
						&account_id,
						transfer.amount,
						Preservation::Expendable,
					)
					.map_err(|e| {
						// can't panic here or chain will get stuck
						log::warn!(
							target: LOG_TARGET,
							"Failed to return pending Localchain transfer to account {:?} (amount={:?}): {:?}",
							&account_id,
							transfer.amount,
							e
						);
					});
					Self::deposit_event(Event::TransferToLocalchainExpired {
						account_id,
						nonce,
						notary_id: transfer.notary_id,
					});
				}
			}
			T::DbWeight::get().reads_writes(2, 1)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn submit_notebook(
			origin: OriginFor<T>,
			// This has already been checked at this point. It's provided as a spam reduction
			// feature to the unsigned transaction pool
			_notebook_hash: H256,
			notebook: AuditedNotebook,
			// since signature verification is done in `validate_unsigned`
			// we can skip doing it here again.
			_signature: NotarySignature,
		) -> DispatchResult {
			ensure_none(origin)?;
			let notebook_number = notebook.header.notebook_number;
			let notary_id = notebook.header.notary_id;
			let pinned_to_block_number: BlockNumberFor<T> =
				notebook.header.pinned_to_block_number.into();

			if let Some((last_notebook_number, last_block)) =
				LastNotebookNumberByNotary::<T>::get(notary_id)
			{
				ensure!(
					notebook_number == last_notebook_number + 1,
					Error::<T>::MissingNotebookNumber
				);
				ensure!(pinned_to_block_number >= last_block, Error::<T>::InvalidPinnedBlockNumber);
			}

			// already checked signature, notebook hash and validity of notary index in
			// validate_unsigned
			let allowed_auditors = T::HistoricalBlockSealersLookup::get_active_block_sealers_of(
				notebook.header.pinned_to_block_number.into(),
			);

			let audit_signature_message = notebook.header.hash();

			Self::verify_notebook_audit_signatures(
				&notebook.auditors,
				&audit_signature_message,
				&allowed_auditors,
			)?;

			// un-spendable notary account
			let notary_pallet_account_id = Self::notary_account_id(notary_id);
			for transfer in notebook.header.chain_transfers.into_iter() {
				match transfer {
					ChainTransfer::ToMainchain { account_id, amount } => {
						let amount = amount.into();

						ensure!(
							T::Currency::reducible_balance(
								&notary_pallet_account_id,
								Preservation::Expendable,
								Fortitude::Force,
							) >= amount,
							Error::<T>::InsufficientNotarizedFunds
						);
						T::Currency::transfer(
							&notary_pallet_account_id,
							&account_id,
							amount.clone(),
							Preservation::Expendable,
						)?;
						Self::deposit_event(Event::TransferIn { notary_id, account_id, amount });
					},
					ChainTransfer::ToLocalchain { account_id, nonce } => {
						let nonce = nonce.into();
						let account_id = account_id;
						let transfer = <PendingTransfersOut<T>>::take(&account_id, nonce)
							.ok_or(Error::<T>::InvalidOrDuplicatedLocalchainTransfer)?;
						ensure!(
							transfer.expiration_block > <frame_system::Pallet<T>>::block_number(),
							Error::<T>::NotebookIncludesExpiredLocalchainTransfer
						);
						ensure!(
							transfer.notary_id == notary_id,
							Error::<T>::InvalidNotaryUsedForTransfer
						);
						let _ =
							<ExpiringTransfersOut<T>>::try_mutate(transfer.expiration_block, |e| {
								if let Some(pos) =
									e.iter().position(|x| x.0 == account_id && x.1 == nonce)
								{
									e.remove(pos);
								}
								Ok::<_, Error<T>>(())
							});
					},
				}
			}

			<LastNotebookNumberByNotary<T>>::insert(
				notary_id,
				(notebook_number, pinned_to_block_number),
			);

			<NotebookChangedAccountsRootByNotary<T>>::insert(
				notary_id,
				notebook_number,
				notebook.header.changed_accounts_root,
			);

			for account_origin in notebook.header.changed_account_origins.into_iter() {
				<AccountOriginLastChangedNotebook<T>>::insert(
					(notary_id, account_origin.notebook_number, account_origin.account_uid),
					notebook_number,
				);
			}

			Self::deposit_event(Event::NotebookSubmitted {
				notary_id,
				notebook_number: notebook.header.notebook_number,
				pinned_to_block_number: pinned_to_block_number.into(),
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn send_to_localchain(
			origin: OriginFor<T>,
			#[pallet::compact] amount: T::Balance,
			notary_id: NotaryId,
			#[pallet::compact] nonce: T::Nonce,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				T::Currency::reducible_balance(&who, Preservation::Expendable, Fortitude::Force) >=
					amount,
				Error::<T>::InsufficientFunds,
			);
			ensure!(
				nonce ==
					<frame_system::Pallet<T>>::account_nonce(&who)
						.saturating_sub(T::Nonce::one()),
				Error::<T>::InvalidAccountNonce
			);

			T::Currency::transfer(
				&who,
				&Self::notary_account_id(notary_id),
				amount,
				Preservation::Expendable,
			)?;

			let expiration_block: BlockNumberFor<T> = <frame_system::Pallet<T>>::block_number() +
				T::TransferExpirationBlocks::get().into();

			PendingTransfersOut::<T>::insert(
				&who,
				nonce,
				QueuedTransferOut { amount, expiration_block, notary_id },
			);
			ExpiringTransfersOut::<T>::try_append(expiration_block, (&who, nonce))
				.map_err(|_| Error::<T>::MaxBlockTransfersExceeded)?;

			Self::deposit_event(Event::TransferToLocalchain {
				account_id: who,
				amount,
				nonce,
				notary_id,
				expiration_block,
			});
			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit_notebook { notebook_hash, notebook, signature } = call {
				let current_finalized_block: u32 =
					<FinalizedBlockNumber<T>>::get().unique_saturated_into();

				// if the block is not finalized, we can't validate the notebook
				ensure!(
					notebook.header.pinned_to_block_number <= current_finalized_block,
					InvalidTransaction::Future
				);

				if let Some((last_notebook, _)) =
					<LastNotebookNumberByNotary<T>>::get(notebook.header.notary_id)
				{
					ensure!(
						notebook.header.notebook_number > last_notebook,
						InvalidTransaction::Stale
					);
				}

				// make the sender provide the hash. We'll just reject it if it's bad
				ensure!(
					T::NotaryProvider::verify_signature(
						notebook.header.notary_id,
						notebook.header.pinned_to_block_number.into(),
						&notebook_hash,
						&signature
					),
					InvalidTransaction::BadProof
				);

				// verify the hash is valid
				let block_hash = notebook.hash();
				ensure!(block_hash == *notebook_hash, InvalidTransaction::BadProof);
				log::info!(
					target: LOG_TARGET,
					"Notebook from notary {} pinned to block={:?}, current_finalized_block={current_finalized_block}",
					notebook.header.notary_id,
					notebook.header.pinned_to_block_number
				);
				let blocks_until_finalized: u32 = 100u32 + current_finalized_block;

				ValidTransaction::with_tag_prefix("Notebook")
					.priority(TransactionPriority::MAX)
					.and_provides((
						notebook.header.notary_id,
						notebook.header.pinned_to_block_number,
					))
					.longevity(blocks_until_finalized.into())
					.propagate(true)
					.build()
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn notary_account_id(notary_id: NotaryId) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(notary_id)
		}

		pub fn verify_notebook_audit_signatures(
			auditors: &Vec<(Public, Signature)>,
			signature_message: &H256,
			allowed_auditors: &Vec<(u16, BlockSealAuthorityId)>,
		) -> DispatchResult {
			let required_auditors =
				min(T::RequiredNotebookAuditors::get() as usize, allowed_auditors.len());

			// check first so we can abort early
			ensure!(
				auditors.len() >= required_auditors,
				Error::<T>::InsufficientNotebookSignatures
			);

			let mut signatures = 0usize;
			for (auditor, signature) in auditors.into_iter() {
				let authority_index = allowed_auditors
					.iter()
					.position(|a| a.1.clone().into_inner() == *auditor)
					.ok_or(Error::<T>::InvalidNotebookAuditor)?;

				// must be in first X
				ensure!(
					authority_index < required_auditors,
					Error::<T>::InvalidNotebookAuditorIndex
				);

				let auditor = ed25519::AppPublic::from(*auditor);
				let signature = ed25519::AppSignature::from(signature.clone());

				ensure!(
					auditor.verify(&signature_message.as_ref(), &signature),
					Error::<T>::InvalidNotebookAuditorSignature
				);

				signatures += 1;
			}

			ensure!(signatures >= required_auditors, Error::<T>::InsufficientNotebookSignatures);

			Ok(())
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[codec(mel_bound(Balance: MaxEncodedLen, BlockNumber: MaxEncodedLen))]
pub struct QueuedTransferOut<Balance, BlockNumber> {
	pub amount: Balance,
	pub expiration_block: BlockNumber,
	pub notary_id: NotaryId,
}
