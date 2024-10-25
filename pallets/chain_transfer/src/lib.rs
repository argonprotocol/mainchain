#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use codec::{Decode, Encode};
use core::fmt::Debug;
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

pub use argon_notary_audit::VerifyError as NotebookVerifyError;
use argon_primitives::{notary::NotaryId, tick::Tick, TransferToLocalchainId};
pub use pallet::*;
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
			Incrementable,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::{crypto::AccountId32, H256};
	use sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned};

	use super::*;
	use argon_primitives::{
		notebook::{ChainTransfer, NotebookHeader},
		tick::Tick,
		BurnEventHandler, ChainTransferLookup, NotebookEventHandler, NotebookNumber,
		NotebookProvider,
	};

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

		type NotebookProvider: NotebookProvider;
		type NotebookTick: Get<Tick>;
		type EventHandler: BurnEventHandler<Self::Balance>;
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// How long a transfer should remain in storage before returning. NOTE: there is a 2 tick
		/// grace period where we will still allow a transfer
		#[pallet::constant]
		type TransferExpirationTicks: Get<u32>;

		/// How many transfers out can be queued per block
		#[pallet::constant]
		type MaxPendingTransfersOutPerBlock: Get<u32>;
	}

	#[pallet::storage]
	pub(super) type NextTransferId<T: Config> =
		StorageValue<_, TransferToLocalchainId, OptionQuery>;

	#[pallet::storage]
	pub(super) type PendingTransfersOut<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		TransferToLocalchainId,
		QueuedTransferOut<T::AccountId, T::Balance>,
		OptionQuery,
	>;

	#[pallet::storage]
	pub(super) type ExpiringTransfersOutByNotary<T: Config> = StorageDoubleMap<
		Hasher1 = Twox64Concat,
		Hasher2 = Twox64Concat,
		Key1 = NotaryId,
		Key2 = Tick,
		Value = BoundedVec<TransferToLocalchainId, T::MaxPendingTransfersOutPerBlock>,
		QueryKind = ValueQuery,
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
			transfer_id: TransferToLocalchainId,
			notary_id: NotaryId,
			expiration_tick: Tick,
		},
		TransferToLocalchainExpired {
			account_id: T::AccountId,
			transfer_id: TransferToLocalchainId,
			notary_id: NotaryId,
		},
		TransferIn {
			account_id: T::AccountId,
			amount: T::Balance,
			notary_id: NotaryId,
		},
		/// A transfer into the mainchain failed
		TransferInError {
			account_id: T::AccountId,
			amount: T::Balance,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			error: DispatchError,
		},
		/// An expired transfer to localchain failed to be refunded
		TransferToLocalchainRefundError {
			account_id: T::AccountId,
			transfer_id: TransferToLocalchainId,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			error: DispatchError,
		},
		/// A localchain transfer could not be cleaned up properly. Possible invalid transfer
		/// needing investigation.
		PossibleInvalidTransferAllowed {
			transfer_id: TransferToLocalchainId,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
		},
		/// Taxation failed
		TaxationError {
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			tax: T::Balance,
			error: DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		MaxBlockTransfersExceeded,
		/// Insufficient balance to create this transfer
		InsufficientFunds,
		/// Insufficient balance to fulfill a mainchain transfer
		InsufficientNotarizedFunds,
		/// The transfer was already submitted in a previous block
		InvalidOrDuplicatedLocalchainTransfer,
		/// A transfer was submitted in a previous block but the expiration block has passed
		NotebookIncludesExpiredLocalchainTransfer,
		/// The notary id is not registered
		InvalidNotaryUsedForTransfer,
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
			let transfer_id = Pallet::<T>::next_transfer_id()?;

			T::Currency::transfer(
				&who,
				&Self::notary_account_id(notary_id),
				amount,
				Preservation::Expendable,
			)?;

			let expiration_tick: Tick = T::NotebookTick::get() + T::TransferExpirationTicks::get();

			PendingTransfersOut::<T>::insert(
				transfer_id,
				QueuedTransferOut { account_id: who.clone(), amount, expiration_tick, notary_id },
			);
			ExpiringTransfersOutByNotary::<T>::try_append(notary_id, expiration_tick, transfer_id)
				.map_err(|_| Error::<T>::MaxBlockTransfersExceeded)?;

			Self::deposit_event(Event::TransferToLocalchain {
				account_id: who,
				amount,
				transfer_id,
				notary_id,
				expiration_tick,
			});
			Ok(())
		}
	}

	impl<T: Config> NotebookEventHandler for Pallet<T> {
		fn notebook_submitted(header: &NotebookHeader) {
			let notary_id = header.notary_id;

			// un-spendable notary account
			let notary_pallet_account_id = Self::notary_account_id(notary_id);
			for transfer in header.chain_transfers.iter() {
				match transfer {
					ChainTransfer::ToMainchain { account_id, amount } => {
						let amount = (*amount).into();
						if let Err(e) = Self::transfer_funds_to_mainchain(
							&notary_pallet_account_id,
							account_id,
							amount,
						) {
							Self::deposit_event(Event::TransferInError {
								notary_id,
								notebook_number: header.notebook_number,
								account_id: account_id.clone(),
								amount,
								error: e,
							});
						} else {
							Self::deposit_event(Event::TransferIn {
								notary_id,
								account_id: account_id.clone(),
								amount,
							});
						}
					},
					ChainTransfer::ToLocalchain { transfer_id } => {
						if let Some(transfer) = <PendingTransfersOut<T>>::take(transfer_id) {
							<ExpiringTransfersOutByNotary<T>>::mutate(
								transfer.notary_id,
								transfer.expiration_tick,
								|e| {
									if let Some(pos) = e.iter().position(|x| x == transfer_id) {
										e.remove(pos);
									}
								},
							);
						} else {
							Self::deposit_event(Event::PossibleInvalidTransferAllowed {
								transfer_id: *transfer_id,
								notebook_number: header.notebook_number,
								notary_id,
							});
						}
					},
				}
			}

			if header.tax > 0 {
				if let Err(e) = T::Currency::burn_from(
					&notary_pallet_account_id,
					header.tax.into(),
					Preservation::Preserve,
					Precision::Exact,
					Fortitude::Force,
				) {
					Self::deposit_event(Event::TaxationError {
						notary_id,
						notebook_number: header.notebook_number,
						tax: header.tax.into(),
						error: e,
					});
				}
				T::EventHandler::on_argon_burn(&header.tax.into());
			}

			let expiring = <ExpiringTransfersOutByNotary<T>>::take(notary_id, header.tick);
			for transfer_id in expiring.into_iter() {
				let Some(transfer) = <PendingTransfersOut<T>>::take(transfer_id) else { continue };
				match T::Currency::transfer(
					&Self::notary_account_id(transfer.notary_id),
					&transfer.account_id,
					transfer.amount,
					Preservation::Expendable,
				) {
					Ok(_) => {
						Self::deposit_event(Event::TransferToLocalchainExpired {
							account_id: transfer.account_id,
							transfer_id,
							notary_id: transfer.notary_id,
						});
					},
					Err(e) => {
						// can't panic here or chain will get stuck
						log::warn!(
							target: LOG_TARGET,
							"Failed to return pending Localchain transfer to account {:?} (amount={:?}): {:?}",
							&transfer.account_id,
							transfer.amount,
							e
						);
						Self::deposit_event(Event::TransferToLocalchainRefundError {
							account_id: transfer.account_id,
							notebook_number: header.notebook_number,
							transfer_id,
							notary_id: transfer.notary_id,
							error: e,
						});
					},
				}
			}
		}
	}

	impl<T: Config> ChainTransferLookup<T::AccountId, T::Balance> for Pallet<T> {
		fn is_valid_transfer_to_localchain(
			notary_id: NotaryId,
			transfer_id: TransferToLocalchainId,
			account_id: &T::AccountId,
			milligons: T::Balance,
			at_tick: Tick,
		) -> bool {
			let result = <PendingTransfersOut<T>>::get(transfer_id);
			if let Some(transfer) = result {
				return transfer.notary_id == notary_id &&
					transfer.amount == milligons &&
					transfer.account_id == *account_id &&
					transfer.expiration_tick >= at_tick;
			}

			false
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn transfer_funds_to_mainchain(
			notary_pallet_account_id: &T::AccountId,
			account_id: &T::AccountId,
			amount: T::Balance,
		) -> DispatchResult {
			ensure!(
				T::Currency::reducible_balance(
					notary_pallet_account_id,
					Preservation::Expendable,
					Fortitude::Force,
				) >= amount,
				Error::<T>::InsufficientNotarizedFunds
			);
			T::Currency::transfer(
				notary_pallet_account_id,
				account_id,
				amount,
				Preservation::Expendable,
			)?;
			Ok(())
		}

		pub fn notary_account_id(notary_id: NotaryId) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(notary_id)
		}
		fn next_transfer_id() -> Result<TransferToLocalchainId, Error<T>> {
			let transfer_id = NextTransferId::<T>::get().unwrap_or(1);
			let next_transfer_id = transfer_id.increment();
			NextTransferId::<T>::set(next_transfer_id);
			Ok(transfer_id)
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[codec(mel_bound(Balance: MaxEncodedLen, BlockNumber: MaxEncodedLen))]
pub struct QueuedTransferOut<AccountId, Balance> {
	pub account_id: AccountId,
	pub amount: Balance,
	pub expiration_tick: Tick,
	pub notary_id: NotaryId,
}
