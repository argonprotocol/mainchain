#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::{RuntimeAppPublic, RuntimeDebug};
use sp_std::{fmt::Debug, prelude::*};

pub use pallet::*;
use ulx_primitives::{
	notary::{NotaryId, NotaryProvider},
	BlockSealAuthorityId, BlockSealAuthoritySignature,
};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
const LOG_TARGET: &str = "runtime::localchain::relay";

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
	use sp_io::hashing::blake2_256;
	use sp_runtime::{
		traits::{
			AccountIdConversion, AtLeast32BitUnsigned, MaybeDisplay, One, UniqueSaturatedInto,
		},
		Saturating,
	};
	use sp_std::cmp::min;
	use ulx_primitives::{
		block_seal::HistoricalBlockSealersLookup,
		digests::{FinalizedBlockNeededDigest, FINALIZED_BLOCK_DIGEST_ID},
		notary::{NotaryId, NotarySignature},
	};

	use ulx_primitives::notebook::{
		to_notebook_audit_signature_message, to_notebook_post_hash, ChainTransfer, Notebook,
	};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
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

		type NotaryProvider: NotaryProvider;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// An account id on the localchain
		type LocalchainAccountId: Parameter
			+ Member
			+ MaybeSerializeDeserialize
			+ Debug
			+ MaybeDisplay
			+ Ord
			+ MaxEncodedLen;

		/// How long a transfer should remain in storage before returning.
		#[pallet::constant]
		type TransferExpirationBlocks: Get<u32>;

		/// How many transfers out can be queued per block
		#[pallet::constant]
		type MaxPendingTransfersOutPerBlock: Get<u32>;

		/// How many transfers can be in a single notebook
		#[pallet::constant]
		type MaxNotebookTransfers: Get<u32>;

		/// How many auditors are expected to sign a notary block.
		#[pallet::constant]
		type RequiredNotebookAuditors: Get<u32>;

		/// Number of blocks to keep around for preventing notebook double-submit
		#[pallet::constant]
		type MaxNotebookBlocksToRemember: Get<u32>;
	}

	type NotebookOf<T> = Notebook<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
		<T as frame_system::Config>::Nonce,
		<T as Config>::MaxNotebookTransfers,
		<T as Config>::RequiredNotebookAuditors,
	>;

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

	#[pallet::storage]
	pub(super) type SubmittedNotebookBlocksByNotaryId<T: Config> = StorageMap<
		_,
		Twox64Concat,
		NotaryId,
		BoundedVec<BlockNumberFor<T>, T::MaxNotebookBlocksToRemember>,
		ValueQuery,
	>;

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
		},
		NotebookSubmitted {
			notary_id: NotaryId,
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
			notebook: NotebookOf<T>,
			// since signature verification is done in `validate_unsigned`
			// we can skip doing it here again.
			_signature: NotarySignature,
		) -> DispatchResult {
			ensure_none(origin)?;

			// already checked signature and validity of notary index in validate_unsigned
			let allowed_auditors = T::HistoricalBlockSealersLookup::get_active_block_sealers_of(
				notebook.pinned_to_block_number.into(),
			);

			let audit_signature_message =
				to_notebook_audit_signature_message(&notebook).using_encoded(blake2_256).into();

			Self::verify_notebook_audit_signatures(
				&notebook.auditors,
				&audit_signature_message,
				&allowed_auditors,
			)?;

			let notary_id = notebook.notary_id;
			let pinned_to_block_number = notebook.pinned_to_block_number;
			// un-spendable notary account
			let notary_pallet_account_id = Self::notary_account_id(notary_id);
			for transfer in notebook.transfers.into_iter() {
				match transfer {
					ChainTransfer::ToMainchain { account_id, amount } => {
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
							amount,
							Preservation::Expendable,
						)?;
						Self::deposit_event(Event::TransferIn { account_id, amount });
					},
					ChainTransfer::ToLocalchain { account_id, nonce } => {
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

			let pin: BlockNumberFor<T> = pinned_to_block_number.into();
			<SubmittedNotebookBlocksByNotaryId<T>>::try_mutate(notary_id, |blocks| {
				if blocks.len() >= T::MaxNotebookBlocksToRemember::get() as usize {
					blocks.remove(0);
				}
				// keep them sorted
				let pos = match blocks.binary_search(&pin) {
					Ok(pos) => pos,
					Err(pos) => pos,
				};
				blocks.try_insert(pos, pin).map_err(|_| Error::<T>::BadState)?;
				Ok::<_, Error<T>>(())
			})?;

			Self::deposit_event(Event::NotebookSubmitted {
				notary_id,
				pinned_to_block_number: pin,
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
			if let Call::submit_notebook { notebook, signature } = call {
				let current_finalized_block: u32 =
					<FinalizedBlockNumber<T>>::get().unique_saturated_into();

				// if the block is not finalized, we can't validate the notebook
				ensure!(
					notebook.pinned_to_block_number < current_finalized_block,
					InvalidTransaction::Future
				);

				let block_hash = to_notebook_post_hash(notebook).using_encoded(blake2_256).into();

				ensure!(
					T::NotaryProvider::verify_signature(
						notebook.notary_id,
						&block_hash,
						&signature
					),
					InvalidTransaction::BadProof
				);

				let blocks_until_finalized: u32 =
					1u32 + notebook.pinned_to_block_number - current_finalized_block;

				ValidTransaction::with_tag_prefix("Notebook")
					.priority(TransactionPriority::MAX)
					.and_provides((notebook.pinned_to_block_number, notebook.notary_id))
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
			auditors: &BoundedVec<
				(BlockSealAuthorityId, BlockSealAuthoritySignature),
				T::RequiredNotebookAuditors,
			>,
			signature_message: &H256,
			allowed_auditors: &Vec<BlockSealAuthorityId>,
		) -> DispatchResult {
			let required_auditors =
				min(T::RequiredNotebookAuditors::get() as usize, allowed_auditors.len());

			// check first so we can abort early
			ensure!(
				auditors.len() >= required_auditors,
				Error::<T>::InsufficientNotebookSignatures
			);

			let mut signatures = 0usize;
			for (auditor, signature) in auditors.iter() {
				let authority_index = allowed_auditors
					.iter()
					.position(|a| a == auditor)
					.ok_or(Error::<T>::InvalidNotebookAuditor)?;

				// must be in first X
				ensure!(
					authority_index < required_auditors,
					Error::<T>::InvalidNotebookAuditorIndex
				);

				ensure!(
					auditor.verify(&signature_message.as_ref(), signature),
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
