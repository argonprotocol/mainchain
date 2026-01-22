#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate core;

mod check_fee_wrapper;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

pub use check_fee_wrapper::*;
use frame_support::traits::OriginTrait;
pub use pallet::*;
use pallet_prelude::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use pallet_prelude::argon_primitives::{
		FeelessCallTxPoolKeyProvider, TransactionSponsorProvider,
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

		/// Provides transaction sponsors for transactions
		type TransactionSponsorProviders: TransactionSponsorProvider<Self::AccountId, Self::RuntimeCall, Self::Balance>;
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
}
