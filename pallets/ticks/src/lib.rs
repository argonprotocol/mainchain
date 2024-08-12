#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use frame_support::traits::OnTimestampSet;
use sp_runtime::traits::UniqueSaturatedInto;

use alloc::vec::Vec;
use argon_primitives::TickProvider;
pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

/// This pallet tracks the current tick of the system
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use argon_primitives::{
		digests::TICK_DIGEST_ID,
		tick::{Tick, Ticker},
		TickDigest, TickProvider,
	};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Block as BlockT;

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn current_tick)]
	pub(super) type CurrentTick<T: Config> = StorageValue<_, Tick, ValueQuery>;

	#[pallet::storage]
	pub(super) type GenesisTicker<T: Config> = StorageValue<_, Ticker, ValueQuery>;

	/// Blocks from the last 100 ticks. Trimmed in on_initialize.
	/// NOTE: cannot include the current block hash until next block
	#[pallet::storage]
	pub(super) type RecentBlocksAtTicks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		Tick,
		BoundedVec<<T::Block as BlockT>::Hash, ConstU32<100>>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub ticker: Ticker,

		#[serde(skip)]
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			GenesisTicker::<T>::put(self.ticker.clone());
		}
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_block_number: BlockNumberFor<T>) -> Weight {
			let previous_tick = <CurrentTick<T>>::get();
			let current_tick = <frame_system::Pallet<T>>::digest()
				.logs
				.iter()
				.find_map(|a| a.pre_runtime_try_to::<TickDigest>(&TICK_DIGEST_ID))
				.expect("Tick digest must be set")
				.tick;

			<CurrentTick<T>>::put(current_tick);

			if current_tick >= 100u32 {
				// prune old ticks
				RecentBlocksAtTicks::<T>::take(current_tick - 100u32);
			}

			// kinda weird, but we don't know the current block hash
			RecentBlocksAtTicks::<T>::mutate(previous_tick, |blocks| {
				blocks.try_push(<frame_system::Pallet<T>>::parent_hash())
			})
			.expect("Failed to push block hash to recent blocks. Too many blocks per tick is a valid panic reason.");

			T::DbWeight::get().reads_writes(0, 1)
		}
	}

	impl<T: Config> TickProvider<T::Block> for Pallet<T> {
		fn current_tick() -> Tick {
			<CurrentTick<T>>::get()
		}

		fn ticker() -> Ticker {
			<GenesisTicker<T>>::get()
		}

		fn blocks_at_tick(tick: Tick) -> Vec<<T::Block as BlockT>::Hash> {
			<RecentBlocksAtTicks<T>>::get(tick).to_vec()
		}
	}

	impl<T: Config> Get<Tick> for Pallet<T> {
		fn get() -> Tick {
			Self::current_tick()
		}
	}
}

impl<T: Config> OnTimestampSet<T::Moment> for Pallet<T> {
	fn on_timestamp_set(now: T::Moment) {
		let timestamp = UniqueSaturatedInto::<u64>::unique_saturated_into(now);
		let current_tick = Self::current_tick();
		let ticker = Self::ticker();
		let tick_by_time = ticker.tick_for_time(timestamp);
		// you can only submit this during the last 2 tick "times"
		if current_tick != tick_by_time && current_tick != tick_by_time.saturating_sub(1) {
			panic!("The tick digest is outside the allowed timestamp range to submit it. Digest tick={current_tick} vs Timestamp tick={tick_by_time}");
		}
	}
}
