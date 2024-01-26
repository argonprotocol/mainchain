#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::{fmt::Debug, prelude::*};

pub use pallet::*;
pub use ulx_notary_audit::VerifyError as NotebookVerifyError;
use ulx_primitives::notary::{NotaryId, NotaryProvider};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
const LOG_TARGET: &str = "runtime::chain_transfer";

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, Mutate},
			tokens::{Fortitude, Precision, Preservation},
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::{crypto::AccountId32, H256};
	use sp_runtime::{
		traits::{AccountIdConversion, AtLeast32BitUnsigned, One},
		Saturating,
	};

	use ulx_primitives::{
		notebook::{ChainTransfer, NotebookHeader},
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

		type NotaryProvider: NotaryProvider<<Self as frame_system::Config>::Block>;

		type NotebookProvider: NotebookProvider;
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// How long a transfer should remain in storage before returning.
		#[pallet::constant]
		type TransferExpirationBlocks: Get<u32>;

		/// How many transfers out can be queued per block
		#[pallet::constant]
		type MaxPendingTransfersOutPerBlock: Get<u32>;
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
	pub(super) type TransfersUsedInBlockNotebooks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<(T::AccountId, T::Nonce), T::MaxPendingTransfersOutPerBlock>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		TransferToLocalchain {
			account_id: T::AccountId,
			amount: T::Balance,
			account_nonce: T::Nonce,
			notary_id: NotaryId,
			expiration_block: BlockNumberFor<T>,
		},
		TransferToLocalchainExpired {
			account_id: T::AccountId,
			account_nonce: T::Nonce,
			notary_id: NotaryId,
		},
		TransferIn {
			account_id: T::AccountId,
			amount: T::Balance,
			notary_id: NotaryId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		MaxBlockTransfersExceeded,
		/// Insufficient balance to create this transfer
		InsufficientFunds,
		/// The account nonce used for this transfer is no longer valid
		InvalidAccountNonce,
		/// Insufficient balance to fulfill a mainchain transfer
		InsufficientNotarizedFunds,
		/// The transfer was already submitted in a previous block
		InvalidOrDuplicatedLocalchainTransfer,
		/// A transfer was submitted in a previous block but the expiration block has passed
		NotebookIncludesExpiredLocalchainTransfer,
		/// The notary id is not registered
		InvalidNotaryUsedForTransfer,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let expiring = <ExpiringTransfersOut<T>>::take(block_number);
			for (account_id, account_nonce) in expiring.into_iter() {
				if let Some(transfer) =
					<PendingTransfersOut<T>>::take(account_id.clone(), account_nonce)
				{
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
						account_nonce,
						notary_id: transfer.notary_id,
					});
				}
			}
			T::DbWeight::get().reads_writes(2, 1)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			// nothing to do
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn send_to_localchain(
			origin: OriginFor<T>,
			#[pallet::compact] amount: T::Balance,
			notary_id: NotaryId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				T::Currency::reducible_balance(&who, Preservation::Expendable, Fortitude::Force) >=
					amount,
				Error::<T>::InsufficientFunds,
			);

			// the nonce is incremented pre-dispatch. we want the nonce for the transaction
			let account_nonce =
				<frame_system::Pallet<T>>::account_nonce(&who).saturating_sub(T::Nonce::one());

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
				account_nonce,
				QueuedTransferOut { amount, expiration_block, notary_id },
			);
			ExpiringTransfersOut::<T>::try_append(expiration_block, (&who, account_nonce))
				.map_err(|_| Error::<T>::MaxBlockTransfersExceeded)?;

			Self::deposit_event(Event::TransferToLocalchain {
				account_id: who,
				amount,
				account_nonce,
				notary_id,
				expiration_block,
			});
			Ok(())
		}
	}

	impl<T: Config> NotebookEventHandler for Pallet<T> {
		fn notebook_submitted(header: &NotebookHeader) -> sp_runtime::DispatchResult {
			let notary_id = header.notary_id;

			let is_locked = T::NotebookProvider::is_notary_locked_at_tick(notary_id, header.tick);

			// un-spendable notary account
			let notary_pallet_account_id = Self::notary_account_id(notary_id);
			for transfer in header.chain_transfers.iter() {
				match transfer {
					ChainTransfer::ToMainchain { account_id, amount } => {
						if is_locked {
							continue;
						}
						let amount = amount.clone().into();
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
						Self::deposit_event(Event::TransferIn {
							notary_id,
							account_id: account_id.clone(),
							amount,
						});
					},
					ChainTransfer::ToLocalchain { account_id, account_nonce: nonce } => {
						let nonce = nonce.clone().into();
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
									e.iter().position(|x| x.0 == *account_id && x.1 == nonce)
								{
									e.remove(pos);
								}
								Ok::<_, Error<T>>(())
							});
					},
				}
			}

			if header.tax > 0 && !is_locked {
				T::Currency::burn_from(
					&notary_pallet_account_id,
					header.tax.into(),
					Precision::Exact,
					Fortitude::Force,
				)?;
			}

			Ok(())
		}
	}

	impl<T: Config> ChainTransferLookup<T::Nonce, T::AccountId, T::Balance> for Pallet<T> {
		fn is_valid_transfer_to_localchain(
			notary_id: NotaryId,
			account_id: &T::AccountId,
			nonce: T::Nonce,
			milligons: T::Balance,
		) -> bool {
			let result = <PendingTransfersOut<T>>::get(account_id, nonce);
			if let Some(transfer) = result {
				return transfer.notary_id == notary_id && transfer.amount == milligons;
			}

			false
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn notary_account_id(notary_id: NotaryId) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(notary_id)
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
