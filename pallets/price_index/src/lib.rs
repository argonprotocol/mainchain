#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use core::convert::TryInto;
use pallet_prelude::*;

use argon_primitives::ArgonCPI;
pub use pallet::*;
pub use weights::WeightInfo;

pub mod migrations;
#[cfg(test)]
mod tests;
pub mod weights;

#[cfg(test)]
mod mock;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
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
	/// Price of argon ownership tokens (argonot) in usd cents
	pub argonot_usd_price: FixedU128,
	/// Argon to usd price in cents
	#[codec(compact)]
	pub argon_usd_price: FixedU128,
	/// The target price for argon based on inflation since start
	pub argon_usd_target_price: FixedU128,
	/// The argon liquidity in the pool
	pub argon_time_weighted_average_liquidity: Balance,
	/// Tick of price index
	#[codec(compact)]
	pub tick: Tick,
}
impl PriceIndex {
	pub fn argon_cpi(&self) -> ArgonCPI {
		// if the difference is less than 0.001, treat it as normal market slippage/noise
		let fixed_i_argon_usd_price =
			FixedI128::from_inner(self.argon_usd_price.into_inner() as i128);
		let fixed_i_argon_usd_target_price =
			FixedI128::from_inner(self.argon_usd_target_price.into_inner() as i128);
		if (fixed_i_argon_usd_price - fixed_i_argon_usd_target_price).saturating_abs() <
			FixedI128::from_rational(1, 1000)
		{
			return ArgonCPI::zero();
		}
		let ratio = fixed_i_argon_usd_target_price
			.checked_div(&fixed_i_argon_usd_price)
			.unwrap_or(FixedI128::one());
		ratio - FixedI128::one()
	}

	pub fn redemption_r_value(&self) -> FixedU128 {
		self.argon_usd_price
			.checked_div(&self.argon_usd_target_price)
			.unwrap_or(FixedU128::one())
	}
}

#[frame_support::pallet]
pub mod pallet {
	use sp_arithmetic::FixedPointNumber;

	use argon_primitives::PriceProvider;

	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	/// ## Configuration
	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		type Currency: Inspect<Self::AccountId, Balance = Self::Balance>;

		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Member
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
			+ Debug
			+ Default
			+ From<u128>
			+ Into<u128>
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

	/// Stores the last valid price index
	#[pallet::storage]
	pub type LastValid<T: Config> = StorageValue<_, PriceIndex>;

	/// Tracks the average cpi data every 60 ticks
	#[pallet::storage]
	pub type HistoricArgonCPI<T> =
		StorageValue<_, BoundedVec<CpiMeasurementBucket, ConstU32<48>>, ValueQuery>;

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
			if let Some(current) = Current::<T>::get() {
				LastValid::<T>::put(current);
			}
			T::DbWeight::get().reads_writes(3, 2)
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
		#[pallet::weight((T::WeightInfo::submit(), DispatchClass::Operational))]
		#[pallet::feeless_if(|origin: &OriginFor<T>, _index: &PriceIndex| -> bool {
			let Ok(who) = ensure_signed(origin.clone()) else {
				return false;
			};
			Some(who) == Operator::<T>::get()
		})]
		pub fn submit(origin: OriginFor<T>, index: PriceIndex) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let operator = Operator::<T>::get().ok_or(Error::<T>::NotAuthorizedOperator)?;
			ensure!(operator == who, Error::<T>::NotAuthorizedOperator);

			let oldest_age = T::CurrentTick::get().saturating_sub(T::MaxPriceAgeInTicks::get());

			if index.tick < oldest_age {
				return Ok(());
			}

			let mut index = index;
			if let Some(current) = Current::<T>::get() {
				if index.tick <= current.tick {
					return Ok(());
				}
				Self::clamp_argon_prices(&current, &mut index);
				LastValid::<T>::put(current);
			}

			Current::<T>::put(index);

			// Get the current frame bucket (one per hour)
			const BUCKET_TICKS: Tick = 60;
			let tick_bucket_start = index.tick - (index.tick % BUCKET_TICKS);
			HistoricArgonCPI::<T>::mutate(|history| {
				for bucket in history.iter_mut() {
					if bucket.tick_range.0 == tick_bucket_start {
						bucket.record(index.argon_cpi());
						return;
					}
				}
				if history.is_full() {
					history.pop();
				}
				history
					.try_insert(
						0,
						CpiMeasurementBucket {
							tick_range: (tick_bucket_start, tick_bucket_start + BUCKET_TICKS),
							total_cpi: index.argon_cpi(),
							measurements_count: 1,
						},
					)
					.ok();
			});

			Self::deposit_event(Event::<T>::NewIndex);

			Ok(())
		}

		/// Sets the operator account id (only executable by the Root account)
		///
		/// # Arguments
		/// * `account_id` - the account id of the operator
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_operator())]
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

		pub fn has_new_price_index() -> bool {
			let Some(current) = <Current<T>>::get() else {
				return false;
			};
			let Some(last) = <LastValid<T>>::get() else {
				return true;
			};
			current.tick > last.tick
		}

		pub(crate) fn clamp_argon_prices(current: &PriceIndex, next: &mut PriceIndex) {
			let max_diff = T::MaxArgonChangePerTickAwayFromTarget::get() *
				FixedU128::saturating_from_integer(next.tick - current.tick);
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
				FixedU128::saturating_from_integer(next.tick - current.tick);
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
		fn get_argon_cpi() -> Option<ArgonCPI> {
			Self::get_current().map(|a| a.argon_cpi())
		}
		fn get_latest_argon_price_in_usd() -> Option<FixedU128> {
			Self::get_current().map(|a| a.argon_usd_price)
		}

		fn get_average_cpi_for_ticks(tick_range: (Tick, Tick)) -> ArgonCPI {
			let mut measurement = CpiMeasurementBucket::default();
			for bucket in HistoricArgonCPI::<T>::get().into_iter() {
				if bucket.tick_range.0 >= tick_range.0 && bucket.tick_range.1 <= tick_range.1 {
					measurement.accrue(bucket);
				}
			}
			measurement.average()
		}

		fn get_latest_btc_price_in_usd() -> Option<FixedU128> {
			Self::get_current().map(|a| a.btc_usd_price)
		}

		fn get_redemption_r_value() -> Option<FixedU128> {
			Self::get_current().map(|a| a.redemption_r_value())
		}

		fn get_circulation() -> T::Balance {
			T::Currency::total_issuance()
		}
	}
}

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Default,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct CpiMeasurementBucket {
	/// The tick range this frame represents
	pub tick_range: (Tick, Tick),
	/// The sum of all CPIs in this frame
	pub total_cpi: ArgonCPI,
	/// The count of measurements in this frame
	pub measurements_count: u32,
}

impl CpiMeasurementBucket {
	pub fn record(&mut self, cpi: ArgonCPI) {
		self.total_cpi.saturating_accrue(cpi);
		self.measurements_count.saturating_accrue(1);
	}

	pub fn accrue(&mut self, other: CpiMeasurementBucket) {
		self.total_cpi.saturating_accrue(other.total_cpi);
		self.measurements_count.saturating_accrue(other.measurements_count);
	}

	pub fn average(&self) -> ArgonCPI {
		if self.measurements_count == 0 {
			ArgonCPI::default()
		} else {
			self.total_cpi.div(FixedI128::from_u32(self.measurements_count))
		}
	}
}
