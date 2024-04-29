#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;
use sp_std::convert::TryInto;

pub use pallet::*;
use ulx_primitives::ArgonCPI;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod tests;
pub mod weights;

#[cfg(test)]
mod mock;

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Clone,
	Copy,
	Ord,
	PartialOrd,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PriceIndex<Moment: Codec + Clone + MaxEncodedLen> {
	/// Bitcoin to usd price in cents
	#[codec(compact)]
	pub btc_usd_price: u64,
	/// Argon to usd price in cents
	#[codec(compact)]
	pub argon_usd_price: u64,
	/// Argon CPI calculated using consumer price index + argon price vs a base year
	pub argon_cpi: ArgonCPI,
	/// User created timestamp of submission
	#[codec(compact)]
	pub timestamp: Moment,
}

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::Time};
	use frame_system::pallet_prelude::*;
	use sp_arithmetic::traits::{
		AtLeast32BitUnsigned, CheckedDiv, Saturating, UniqueSaturatedInto,
	};
	use sp_std::{fmt::Debug, vec, vec::Vec};

	use ulx_primitives::{
		bitcoin::{Satoshis, SATOSHIS_PER_BITCOIN},
		ArgonPriceProvider, BitcoinPriceProvider,
	};

	use super::*;

	type PriceIndexOf<T> = PriceIndex<<<T as Config>::Time as Time>::Moment>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// ## Configuration
	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

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
		/// A time provider
		type Time: Time;

		/// The maximum number of oracle operators that can be authorized
		#[pallet::constant]
		type MaxDowntimeBeforeReset: Get<<Self::Time as Time>::Moment>;

		/// Oldest history entries to keep
		#[pallet::constant]
		type OldestHistoryToKeep: Get<<Self::Time as Time>::Moment>;

		/// Max entries to keep in history
		#[pallet::constant]
		type MaxHistoryToKeep: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a new price index is submitted
		NewIndex {
			price_index: PriceIndexOf<T>,
		},
		OperatorChanged {
			operator_id: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not authorized as an oracle operator
		NotAuthorizedOperator,
		/// Missing value
		MissingValue,
		/// Couldn't record history
		HistoryRecordingError,
		/// The submitted prices are too old
		PricesTooOld,
	}

	/// Stores the active price index
	#[pallet::storage]
	pub type Current<T: Config> = StorageValue<_, PriceIndexOf<T>>;

	/// Stores unprocessed values as they're submitted by operators
	#[pallet::storage]
	pub type History<T: Config> =
		StorageValue<_, BoundedVec<PriceIndexOf<T>, T::MaxHistoryToKeep>, ValueQuery>;

	/// The price index operator account
	#[pallet::storage]
	pub type Operator<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub operator: Option<T::AccountId>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(operator) = &self.operator {
				<Operator<T>>::put(operator);
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			T::DbWeight::get().reads_writes(3, 3)
		}

		fn on_finalize(_: BlockNumberFor<T>) {
			let Some(current) = Current::<T>::get() else {
				return;
			};
			let now = T::Time::now();
			if current.timestamp < now - T::MaxDowntimeBeforeReset::get() {
				Current::<T>::take();
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit the latest price index. Only valid for the configured operator account
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn submit(origin: OriginFor<T>, index: PriceIndexOf<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let operator = <Operator<T>>::get().ok_or(Error::<T>::NotAuthorizedOperator)?;
			ensure!(operator == who, Error::<T>::NotAuthorizedOperator);

			let oldest_age = T::Time::now() - T::OldestHistoryToKeep::get();

			ensure!(oldest_age < index.timestamp, Error::<T>::PricesTooOld);

			let mut should_use_as_current = true;
			if let Some(current) = <Current<T>>::get() {
				should_use_as_current = index.timestamp > current.timestamp;
			}
			if should_use_as_current {
				<Current<T>>::put(index.clone());
				Self::deposit_event(Event::<T>::NewIndex { price_index: index });
			}
			<History<T>>::try_mutate(|entry| {
				entry.retain(|a| a.timestamp >= oldest_age);
				let pos = entry
					.binary_search_by(|p| index.timestamp.cmp(&p.timestamp))
					.unwrap_or_else(|x| x);
				let max_length = T::MaxHistoryToKeep::get() as usize;

				if pos < max_length {
					if entry.len() >= max_length {
						entry.pop();
					}

					entry
						.try_insert(pos, index.clone())
						.map_err(|_| Error::<T>::HistoryRecordingError)?;
				}
				Ok::<(), Error<T>>(())
			})?;

			Ok(Pays::No.into())
		}

		/// Sets the operator account id (only executable by the Root account)
		///
		/// # Arguments
		/// * `account_id` - the account id of the operator
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::insert_oracle_operator())]
		pub fn set_operator(origin: OriginFor<T>, account_id: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;
			<Operator<T>>::put(account_id.clone());
			Self::deposit_event(Event::OperatorChanged { operator_id: account_id });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn calculate_argon_price_in_milligons(
			satoshis: Satoshis,
			price: &PriceIndexOf<T>,
		) -> Option<T::Balance> {
			let satoshis: T::Balance = satoshis.unique_saturated_into();
			let satoshis_per_bitcoin: T::Balance = SATOSHIS_PER_BITCOIN.unique_saturated_into();
			let milligons_per_argon: T::Balance = 1000u128.unique_saturated_into();

			let btc_usd_price: T::Balance = price.btc_usd_price.unique_saturated_into();
			let argon_usd_price: T::Balance = price.argon_usd_price.unique_saturated_into();

			let satoshi_cents: T::Balance =
				satoshis.saturating_mul(btc_usd_price).checked_div(&satoshis_per_bitcoin)?;

			let milligons = satoshi_cents
				.saturating_mul(milligons_per_argon)
				.checked_div(&argon_usd_price)?;
			Some(milligons)
		}

		fn get_current() -> Option<PriceIndexOf<T>> {
			let price = <Current<T>>::get()?;
			if price.timestamp < T::Time::now() - T::OldestHistoryToKeep::get() {
				return None;
			}
			Some(price)
		}
	}

	impl<T: Config> ArgonPriceProvider for Pallet<T> {
		fn get_argon_cpi_price() -> Option<ArgonCPI> {
			<Current<T>>::get().map(|a| a.argon_cpi)
		}
		fn get_latest_price_in_us_cents() -> Option<u64> {
			<Current<T>>::get().map(|a| a.argon_usd_price)
		}
	}

	impl<T: Config> BitcoinPriceProvider<T::Balance> for Pallet<T> {
		fn get_bitcoin_argon_prices(satoshis: Satoshis) -> Vec<T::Balance> {
			let oldest_valid_time = T::Time::now() - T::OldestHistoryToKeep::get();
			<History<T>>::get()
				.iter()
				.filter_map(|price| {
					if price.timestamp < oldest_valid_time {
						return None;
					}
					Self::calculate_argon_price_in_milligons(satoshis, price)
				})
				.collect::<Vec<_>>()
		}

		fn get_bitcoin_argon_price(satoshis: Satoshis) -> Option<T::Balance> {
			let price = Self::get_current()?;
			Self::calculate_argon_price_in_milligons(satoshis, &price)
		}

		fn get_latest_price_in_us_cents() -> Option<u64> {
			Self::get_current().map(|a| a.btc_usd_price)
		}
	}
}
