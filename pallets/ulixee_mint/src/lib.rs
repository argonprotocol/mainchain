#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{traits::Zero, Saturating};
use sp_std::vec::Vec;

pub use pallet::*;
use ulx_primitives::{block_seal::BlockPayout, BlockRewardsEventHandler};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::{Mutate, MutateHold},
			StoredMap,
		},
		StorageMap as StorageMapT,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{AtLeast32BitUnsigned, CheckedDiv};

	use ulx_primitives::{BurnEventHandler, MintCirculationProvider};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Currency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Mutate<Self::AccountId, Balance = Self::Balance>;

		type UlixeeTokenStorage: frame_support::StorageMap<
			Self::AccountId,
			pallet_balances::AccountData<Self::Balance>,
			Query = pallet_balances::AccountData<Self::Balance>,
		>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ sp_std::fmt::Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		type BitcoinMintCirculation: MintCirculationProvider<Self::Balance>;
	}

	/// Last moved block of ulixee tokens
	#[pallet::storage]
	pub(super) type UlixeeAccountLastTransferBlock<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

	#[pallet::storage]
	pub(super) type MintedArgons<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {}

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		fn track_ulixees_transferred(sender: &T::AccountId) {
			UlixeeAccountLastTransferBlock::<T>::insert(
				sender,
				frame_system::Pallet::<T>::block_number(),
			);
		}

		pub fn track_block_mint(amount: T::Balance) {
			MintedArgons::<T>::mutate(|mint| *mint += amount);
		}

		pub fn on_argon_burn(amount: T::Balance) {
			let ulixee_mint = MintedArgons::<T>::get();
			let total_minted = ulixee_mint + T::BitcoinMintCirculation::get_mint_circulation();
			let prorata = (amount * ulixee_mint).checked_div(&total_minted);
			if let Some(milligons) = prorata {
				MintedArgons::<T>::mutate(|mint| *mint -= milligons);
			}
		}
	}

	impl<T: Config> MintCirculationProvider<T::Balance> for Pallet<T> {
		fn get_mint_circulation() -> T::Balance {
			MintedArgons::<T>::get()
		}
	}

	impl<T: Config> StoredMap<T::AccountId, pallet_balances::AccountData<T::Balance>> for Pallet<T> {
		fn get(k: &T::AccountId) -> pallet_balances::AccountData<T::Balance> {
			T::UlixeeTokenStorage::get(k)
		}
		fn insert(
			k: &T::AccountId,
			t: pallet_balances::AccountData<T::Balance>,
		) -> Result<(), DispatchError> {
			Self::track_ulixees_transferred(k);
			T::UlixeeTokenStorage::insert(k, t);
			Ok(())
		}
		fn remove(k: &T::AccountId) -> Result<(), DispatchError> {
			if T::UlixeeTokenStorage::contains_key(&k) {
				T::UlixeeTokenStorage::remove(k);
			}
			Ok(())
		}
		fn mutate<R>(
			k: &T::AccountId,
			f: impl FnOnce(&mut pallet_balances::AccountData<T::Balance>) -> R,
		) -> Result<R, DispatchError> {
			let result = T::UlixeeTokenStorage::mutate(k, f);
			Self::track_ulixees_transferred(k);
			Ok(result)
		}
		fn mutate_exists<R>(
			k: &T::AccountId,
			f: impl FnOnce(&mut Option<pallet_balances::AccountData<T::Balance>>) -> R,
		) -> Result<R, DispatchError> {
			T::UlixeeTokenStorage::try_mutate_exists(k, |maybe_value| {
				let r = f(maybe_value);
				Self::track_ulixees_transferred(k);
				Ok(r)
			})
		}
		fn try_mutate_exists<R, E: From<DispatchError>>(
			k: &T::AccountId,
			f: impl FnOnce(&mut Option<pallet_balances::AccountData<T::Balance>>) -> Result<R, E>,
		) -> Result<R, E> {
			T::UlixeeTokenStorage::try_mutate_exists(k, |maybe_value| {
				let r = f(maybe_value)?;
				Self::track_ulixees_transferred(k);
				Ok(r)
			})
		}
	}

	impl<T: Config> BurnEventHandler<T::Balance> for Pallet<T> {
		fn on_argon_burn(milligons: &T::Balance) -> sp_runtime::DispatchResult {
			Self::on_argon_burn(milligons.clone());
			Ok(())
		}
	}
}

impl<T: Config> BlockRewardsEventHandler<T::AccountId, T::Balance> for Pallet<T> {
	fn rewards_created(payout: &Vec<BlockPayout<T::AccountId, T::Balance>>) {
		let mut argons = T::Balance::zero();
		for reward in payout {
			argons = argons.saturating_add(reward.argons);
		}
		if argons != T::Balance::zero() {
			Self::track_block_mint(argons);
		}
	}
}
