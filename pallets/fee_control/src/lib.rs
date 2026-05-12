#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate core;

mod check_fee_wrapper;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

pub use check_fee_wrapper::*;
use core::marker::PhantomData;
use frame_support::traits::OriginTrait;
pub use pallet::*;
use pallet_prelude::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use pallet_prelude::argon_primitives::{
		CallTxPoolKeyProvider, CallTxValidityProvider, CurrentTransactionFeeProvider,
		CurrentTransactionFeeProviderWeightInfo, FeelessCallTxPoolKeyProvider,
		TransactionSponsorProvider,
	};

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// Provides DoS protection for Feeless keys where you can supply a unique key to each type
		/// to prevent multiple instances from entering the tx pool
		type FeelessCallTxPoolKeyProviders: FeelessCallTxPoolKeyProvider<Self::RuntimeCall>;

		/// Provides call-level tx pool keys for requests that should be mutually exclusive in the
		/// pool even when they are not statically feeless or sponsor delegated.
		type CallTxPoolKeyProviders: CallTxPoolKeyProvider<Self::RuntimeCall, Self::AccountId>;

		/// Provides best-state-aware stale checks for calls that should be rejected before
		/// inclusion once another transaction has already satisfied the same logical request.
		type CallTxValidityProviders: CallTxValidityProvider<Self::RuntimeCall, Self::AccountId>;

		/// Provides transaction sponsors for transactions
		type TransactionSponsorProviders: TransactionSponsorProvider<
			Self::AccountId,
			Self::RuntimeCall,
			Self::Balance,
		>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {
		/// The requested tip + fee is higher than the maximum allowed by the sponsor
		SponsoredFeeTooHigh,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A transaction fee was skipped.
		FeeSkipped { origin: <T::RuntimeOrigin as OriginTrait>::PalletsOrigin },
		/// A transaction fee was delegated
		FeeDelegated {
			origin: <T::RuntimeOrigin as OriginTrait>::PalletsOrigin,
			from: T::AccountId,
			to: T::AccountId,
		},
	}

	pub struct CurrentTransactionFeeWeightAdapter<T>(PhantomData<T>);
	impl<T: Config> CurrentTransactionFeeProviderWeightInfo for CurrentTransactionFeeWeightAdapter<T> {
		fn current_transaction_fee() -> Weight {
			T::DbWeight::get().reads(1)
		}
	}

	impl<T> CurrentTransactionFeeProvider<T::Balance> for Pallet<T>
	where
		T: Config + pallet_transaction_payment::Config,
		<<<T as pallet_transaction_payment::Config>::OnChargeTransaction as pallet_transaction_payment::TxCreditHold<
			T,
		>>::Credit as frame_support::traits::SuppressedDrop>::Inner:
			frame_support::traits::Imbalance<T::Balance>,
	{
		type Weights = CurrentTransactionFeeWeightAdapter<T>;

		fn reimbursable_fee() -> Option<T::Balance> {
			let fee = pallet_transaction_payment::Pallet::<T>::remaining_txfee::<T::Balance>();
			(fee != T::Balance::default()).then_some(fee)
		}
	}
}
