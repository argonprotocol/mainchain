#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_arithmetic::{FixedI128, FixedU128};
use sp_core::RuntimeDebug;
use sp_runtime::traits::{CheckedDiv, One, Zero};
use sp_std::convert::TryInto;

pub use pallet::*;
use ulx_primitives::{tick::Tick, ArgonCPI};
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
pub struct PriceIndex {
	/// Bitcoin to usd price in cents
	#[codec(compact)]
	pub btc_usd_price: FixedU128,
	/// Argon to usd price in cents
	#[codec(compact)]
	pub argon_usd_price: FixedU128,
	/// The target price for argon based on inflation since start
	pub argon_usd_target_price: FixedU128,
	/// Tick of price index
	#[codec(compact)]
	pub tick: Tick,
}
impl PriceIndex {
	pub fn argon_cpi(&self) -> ArgonCPI {
		let ratio = self
			.argon_usd_target_price
			.checked_div(&self.argon_usd_price)
			.unwrap_or(FixedU128::zero());
		ArgonCPI::from_inner(ratio.into_inner() as i128) - FixedI128::one()
	}
}

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_arithmetic::{traits::AtLeast32BitUnsigned, FixedPointNumber};
	use sp_std::{fmt::Debug, vec};

	use ulx_primitives::PriceProvider;

	use super::*;

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

		/// The maximum number of ticks to preserve a price index
		#[pallet::constant]
		type MaxDowntimeTicksBeforeReset: Get<Tick>;

		/// The oldest history to keep
		#[pallet::constant]
		type MaxPriceAgeInTicks: Get<Tick>;

		/// The current tick
		type CurrentTick: Get<Tick>;

		/// The max price difference dropping below target or raising above target per tick. There's
		/// no corresponding constant for time to recovery to target
		#[pallet::constant]
		type MaxArgonChangePerTickAwayFromTarget: Get<FixedU128>;
		#[pallet::constant]
		type MaxArgonTargetChangePerTick: Get<FixedU128>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a new price index is submitted
		NewIndex,
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
		/// The submitted prices are too old
		PricesTooOld,
		/// Change in argon price is too large
		MaxPriceChangePerTickExceeded,
	}

	/// Stores the active price index
	#[pallet::storage]
	pub type Current<T: Config> = StorageValue<_, PriceIndex>;

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
			let current_tick = T::CurrentTick::get();
			if current.tick < current_tick.saturating_sub(T::MaxDowntimeTicksBeforeReset::get()) {
				Current::<T>::take();
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit the latest price index. Only valid for the configured operator account
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Operational))]
		pub fn submit(origin: OriginFor<T>, index: PriceIndex) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let operator = <Operator<T>>::get().ok_or(Error::<T>::NotAuthorizedOperator)?;
			ensure!(operator == who, Error::<T>::NotAuthorizedOperator);

			let oldest_age = T::CurrentTick::get().saturating_sub(T::MaxPriceAgeInTicks::get());

			if index.tick < oldest_age {
				return Ok(Pays::No.into())
			}

			let mut index = index;
			if let Some(current) = <Current<T>>::get() {
				if index.tick <= current.tick {
					return Ok(Pays::No.into())
				}
				Self::clamp_argon_prices(&current, &mut index);
			}

			<Current<T>>::put(index);
			Self::deposit_event(Event::<T>::NewIndex);

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
		fn get_current() -> Option<PriceIndex> {
			let price = <Current<T>>::get()?;
			if price.tick < T::CurrentTick::get().saturating_sub(T::MaxPriceAgeInTicks::get()) {
				return None;
			}
			Some(price)
		}

		pub(crate) fn clamp_argon_prices(current: &PriceIndex, next: &mut PriceIndex) {
			let max_diff = T::MaxArgonChangePerTickAwayFromTarget::get() *
				FixedU128::from_u32(next.tick - current.tick);
			let argon_cpi = next.argon_cpi();
			if next.argon_usd_price <= current.argon_usd_price {
				// If the argon cpi is negative, then we're in inflation. We will allow price to
				// come back to target without restraint.
				//
				// However, if it's positive (deflation), for security, we are going to limit
				// the allowed price change per tick.
				if argon_cpi.is_positive() {
					let diff = current.argon_usd_price - next.argon_usd_price;
					if diff > max_diff {
						next.argon_usd_price = current.argon_usd_price - max_diff;
					}
				}
			} else {
				// if the price is increasing, we will allow it to go up without restraint only
				// when we are in a deflationary period
				if argon_cpi.is_negative() {
					let diff = next.argon_usd_price - current.argon_usd_price;
					if diff > max_diff {
						next.argon_usd_price = current.argon_usd_price + max_diff;
					}
				}
			}

			// clamp change for target price
			let max_target_diff = T::MaxArgonTargetChangePerTick::get() *
				FixedU128::from_u32(next.tick - current.tick);
			if current.argon_usd_target_price > next.argon_usd_target_price {
				let diff = current.argon_usd_target_price - next.argon_usd_target_price;
				if diff > max_target_diff {
					next.argon_usd_target_price = current.argon_usd_target_price - max_target_diff;
				}
			} else {
				let diff = next.argon_usd_target_price - current.argon_usd_target_price;
				if diff > max_target_diff {
					next.argon_usd_target_price = current.argon_usd_target_price + max_target_diff;
				}
			}
		}
	}

	impl<T: Config> PriceProvider<T::Balance> for Pallet<T> {
		fn get_argon_cpi_price() -> Option<ArgonCPI> {
			Self::get_current().map(|a| a.argon_cpi())
		}
		fn get_latest_argon_price_in_us_cents() -> Option<FixedU128> {
			Self::get_current().map(|a| a.argon_usd_price)
		}

		fn get_latest_btc_price_in_us_cents() -> Option<FixedU128> {
			Self::get_current().map(|a| a.btc_usd_price)
		}
	}
}
