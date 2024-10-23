#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use argon_primitives::TickProvider;
use frame_support::traits::OnTimestampSet;
pub use pallet::*;
use sp_runtime::traits::UniqueSaturatedInto;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

const MAX_RECENT_BLOCKS: u32 = 10;

/// This pallet tracks the current tick of the system
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use argon_primitives::{
		digests::TICK_DIGEST_ID,
		tick::{Tick, Ticker},
		TickDigest, TickProvider, VotingSchedule,
	};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Block as BlockT;

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
	pub(super) type RecentBlocksAtTicks<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<Tick, <T::Block as BlockT>::Hash, ConstU32<MAX_RECENT_BLOCKS>>,
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
			GenesisTicker::<T>::put(self.ticker);
		}
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_block_number: BlockNumberFor<T>) -> Weight {
			// kinda weird, but we don't know the current block hash
			let parent_tick = <CurrentTick<T>>::get();
			let proposed_tick = <frame_system::Pallet<T>>::digest()
				.logs
				.iter()
				.find_map(|a| a.pre_runtime_try_to::<TickDigest>(&TICK_DIGEST_ID))
				.expect("Tick digest must be set")
				.tick;

			let parent_hash = <frame_system::Pallet<T>>::parent_hash();
			if let Err(e) = RecentBlocksAtTicks::<T>::try_mutate(|map| {
				if map.contains_key(&parent_tick) {
					panic!("Block at tick already exists");
				}
				if map.len() >= MAX_RECENT_BLOCKS as usize {
					let first_key = *map.iter().next().expect("List is not empty").0;
					map.remove(&first_key);
				}
				map.try_insert(parent_tick, parent_hash).expect("Could not insert block");
				Ok::<(), Error<T>>(())
			}) {
				panic!("Could not add block to recent blocks at tick: {:?}", e);
			}

			if proposed_tick <= parent_tick {
				panic!("Proposed tick is less than or equal to current tick. Proposed: {:?}, Current: {:?}", proposed_tick, parent_tick);
			}

			<CurrentTick<T>>::put(proposed_tick);

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

		fn block_at_tick(tick: Tick) -> Option<<T::Block as BlockT>::Hash> {
			<RecentBlocksAtTicks<T>>::get().get(&tick).cloned()
		}

		fn voting_schedule() -> VotingSchedule {
			let current_tick = Self::current_tick();
			VotingSchedule::from_runtime_current_tick(current_tick)
		}
	}

	impl<T: Config> Get<Tick> for Pallet<T> {
		fn get() -> Tick {
			Self::current_tick()
		}
	}
}

impl<T: Config> OnTimestampSet<T::Moment> for Pallet<T> {
	// called from an inherent, so will be after on_initialize
	fn on_timestamp_set(now: T::Moment) {
		let timestamp = UniqueSaturatedInto::<u64>::unique_saturated_into(now);
		let tick_for_now = Self::ticker().tick_for_time(timestamp);
		let proposed_tick = <CurrentTick<T>>::get();
		// tick for current time must be >= the proposed tick
		if tick_for_now < proposed_tick {
			panic!("The proposed tick is in the future, which is not allowed. Digest tick={proposed_tick} vs Current tick={tick_for_now}");
		}
	}
}
