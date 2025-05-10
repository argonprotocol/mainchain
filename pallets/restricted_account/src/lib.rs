#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::boxed::Box;
use core::convert::TryInto;
use frame_support::{
	dispatch::{extract_actual_weight, GetDispatchInfo, PostDispatchInfo},
	traits::{InstanceFilter, IsSubType, OriginTrait},
};
pub use pallet::*;
use pallet_prelude::*;
use polkadot_sdk::frame_system::RawOrigin;
use sp_runtime::{
	impl_tx_ext_default,
	traits::{DispatchOriginOf, Implication, TransactionExtension, ValidateResult},
};
pub use weights::WeightInfo;

pub mod migrations;
#[cfg(test)]
mod tests;
pub mod weights;

#[cfg(test)]
mod mock;

/// Allow an account to be restricted to a set of calls that mimic the Proxy types. This is useful
/// for creating an account that operates as itself and pays its own fees, but has restricted pallet
/// access. (Proxy accounts are not allowed to pay fees).
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// ## Configuration
	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<
		RuntimeCall: IsSubType<Call<Self>>
		                 + Dispatchable<
			RuntimeOrigin = <Self as polkadot_sdk::frame_system::Config>::RuntimeOrigin,
			PostInfo = PostDispatchInfo,
		>,
	>
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// A kind of access; specified with the account and passed in to the `IsProxyable` filter.
		/// The instance filter determines whether a given call may be run under this type.
		///
		/// IMPORTANT: `Default` must be provided and MUST BE the *most permissive* value.
		type AccessTypes: Parameter
			+ Member
			+ Ord
			+ PartialOrd
			+ InstanceFilter<<Self as polkadot_sdk::frame_system::Config>::RuntimeCall>
			+ Default
			+ MaxEncodedLen;

		type MaxAccessTypes: Get<u32>;
	}

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		/// The account is already restricted.
		AccountAlreadyRestricted,
		/// This account is not the owner of the restriction
		AccountNotOwner,
	}

	/// The restricted access types for an account.
	#[pallet::storage]
	pub type AccountAccessList<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::AccessTypes, T::MaxAccessTypes>,
		OptionQuery,
	>;

	/// The owner of the account. This account is the only account that can remove the restriction.
	/// It also has an ability to dispatch calls as the restricted account.
	#[pallet::storage]
	pub type AccountOwner<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, T::AccountId, OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Restrict account access
		#[pallet::call_index(0)]
		pub fn register(
			origin: OriginFor<T>,
			owner_account: T::AccountId,
			access_types: BoundedVec<T::AccessTypes, T::MaxAccessTypes>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let existing_owner = AccountOwner::<T>::get(&who);
			ensure!(existing_owner.is_none(), Error::<T>::AccountAlreadyRestricted);
			AccountOwner::<T>::insert(&who, &owner_account);
			AccountAccessList::<T>::insert(&who, &access_types);
			Ok(())
		}

		/// Modify account access. Must be called by the owner account
		#[pallet::call_index(1)]
		pub fn modify_access(
			origin: OriginFor<T>,
			restricted_account: T::AccountId,
			access_types: BoundedVec<T::AccessTypes, T::MaxAccessTypes>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if let Some(owner) = AccountOwner::<T>::take(&restricted_account) {
				ensure!(who == owner, Error::<T>::AccountNotOwner);
				AccountAccessList::<T>::insert(&restricted_account, &access_types);
			}
			Ok(())
		}

		/// Remove account access restriction. Must be called by the owner account
		#[pallet::call_index(2)]
		pub fn deregister(
			origin: OriginFor<T>,
			restricted_account: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if let Some(owner) = AccountOwner::<T>::take(&restricted_account) {
				ensure!(who == owner, Error::<T>::AccountNotOwner);
				AccountAccessList::<T>::remove(&restricted_account);
			}
			Ok(())
		}

		/// Dispatch a restricted call as the restricted account. This api opens permissions to apis
		/// otherwise unavailable to the account
		#[pallet::call_index(3)]
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(
				T::WeightInfo::owner_dispatch()
					// AccountData for inner call origin accountdata.
					.saturating_add(T::DbWeight::get().reads_writes(1, 1))
					.saturating_add(dispatch_info.call_weight),
				dispatch_info.class,
			)
		})]
		#[allow(clippy::useless_conversion)]
		pub fn owner_dispatch(
			origin: OriginFor<T>,
			restricted_account: T::AccountId,
			call: Box<T::RuntimeCall>,
		) -> DispatchResultWithPostInfo {
			let mut origin = origin;
			let who = ensure_signed(origin.clone())?;
			let owner =
				AccountOwner::<T>::get(&restricted_account).ok_or(Error::<T>::AccountNotOwner)?;
			ensure!(owner == who, Error::<T>::AccountNotOwner);
			origin.set_caller_from(frame_system::RawOrigin::Signed(restricted_account));

			let info = call.get_dispatch_info();
			let result = call.dispatch(origin);
			// Always take into account the base weight of this call.
			let mut weight = T::WeightInfo::owner_dispatch()
				.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			// Add the real weight of the dispatch.
			weight = weight.saturating_add(extract_actual_weight(&result, &info));
			result
				.map_err(|mut err| {
					err.post_info = Some(weight).into();
					err
				})
				.map(|_| Some(weight).into())
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, DebugNoBound, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct CheckRestrictedAccess<T>(PhantomData<T>);

impl<T: Config> Default for CheckRestrictedAccess<T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<T: Config + Send + Sync> TransactionExtension<T::RuntimeCall> for CheckRestrictedAccess<T> {
	const IDENTIFIER: &'static str = "RestrictedAccess";
	type Implicit = ();
	type Val = ();
	type Pre = ();

	fn validate(
		&self,
		origin: DispatchOriginOf<T::RuntimeCall>,
		call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Implication,
		_source: TransactionSource,
	) -> ValidateResult<Self::Val, T::RuntimeCall> {
		if let Some(RawOrigin::Signed(account)) = origin.as_system_ref() {
			if let Some(access_list) = AccountAccessList::<T>::get(account) {
				for access_type in access_list {
					if !access_type.filter(call) {
						return Err(InvalidTransaction::BadSigner.into())
					}
				}
			}
		}
		Ok((ValidTransaction::default(), (), origin))
	}

	impl_tx_ext_default!(T::RuntimeCall; weight prepare);
}
