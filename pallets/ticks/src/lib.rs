#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

use argon_primitives::TickProvider;
use frame_support::traits::OnTimestampSet;
pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

const MAX_RECENT_BLOCKS: u64 = 10;

/// This pallet tracks the current tick of the system
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use argon_notary_audit::VerifyError;
	use argon_primitives::{
		Digestset, TickProvider, VotingSchedule,
		tick::{MAX_BLOCKS_PER_TICK, Ticker},
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + polkadot_sdk::frame_system::Config {
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		/// Loads the digest of the current block
		type Digests: Get<Result<Digestset<VerifyError, Self::AccountId>, DispatchError>>;
	}

	#[pallet::storage]
	pub(super) type PreviousTick<T: Config> = StorageValue<_, Tick, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn current_tick)]
	pub(super) type CurrentTick<T: Config> = StorageValue<_, Tick, ValueQuery>;

	#[pallet::storage]
	pub(super) type GenesisTick<T: Config> = StorageValue<_, Tick, ValueQuery>;

	#[pallet::storage]
	pub(super) type GenesisTicker<T: Config> = StorageValue<_, Ticker, ValueQuery>;

	/// Blocks from the last 100 ticks. Trimmed in on_initialize.
	/// NOTE: cannot include the current block hash until next block
	#[pallet::storage]
	pub(super) type RecentBlocksAtTicks<T: Config> = StorageMap<
		_,
		Twox64Concat,
		Tick,
		BoundedVec<<T::Block as BlockT>::Hash, ConstU32<MAX_BLOCKS_PER_TICK>>,
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
			PreviousTick::<T>::put(parent_tick);
			let digests = T::Digests::get().expect("Digests must be loadable");
			let proposed_tick = digests.tick.0;
			// if we're past the max recent blocks, remove the oldest
			if parent_tick > MAX_RECENT_BLOCKS {
				for tick in parent_tick..=proposed_tick {
					RecentBlocksAtTicks::<T>::take(tick.saturating_sub(MAX_RECENT_BLOCKS));
				}
			}
			let blocks_for_proposed = RecentBlocksAtTicks::<T>::get(proposed_tick);
			let blocks_at_proposed = blocks_for_proposed.len();
			if blocks_for_proposed.is_full() {
				panic!("No more blocks can be proposed at tick {:?}", proposed_tick);
			}

			if MAX_BLOCKS_PER_TICK as usize - blocks_at_proposed == 1 {
				let notebooks = digests.notebooks.notebooks.len();
				if notebooks == 0 {
					panic!("A fifth block per tick can only be proposed if there are notebooks");
				}
			}

			RecentBlocksAtTicks::<T>::mutate(parent_tick, |blocks| {
				blocks.try_push(<frame_system::Pallet<T>>::parent_hash())
			})
			.expect("Failed to push block hash to recent blocks at tick");

			if proposed_tick < parent_tick {
				panic!(
					"Proposed tick is less than or equal to current tick. Proposed: {:?}, Current: {:?}",
					proposed_tick, parent_tick
				);
			}

			<CurrentTick<T>>::put(proposed_tick);
			if <GenesisTick<T>>::get() == 0 {
				PreviousTick::<T>::put(proposed_tick);
				<GenesisTick<T>>::put(proposed_tick);
			}

			T::DbWeight::get().reads_writes(0, 1)
		}
	}

	impl<T: Config> TickProvider<T::Block> for Pallet<T> {
		fn previous_tick() -> Tick {
			<PreviousTick<T>>::get()
		}

		fn current_tick() -> Tick {
			<CurrentTick<T>>::get()
		}

		fn elapsed_ticks() -> Tick {
			Self::ticks_since_genesis()
		}

		fn voting_schedule() -> VotingSchedule {
			let current_tick = Self::current_tick();
			VotingSchedule::from_runtime_current_tick(current_tick)
		}

		fn ticker() -> Ticker {
			<GenesisTicker<T>>::get()
		}

		fn blocks_at_tick(tick: Tick) -> Vec<<T::Block as BlockT>::Hash> {
			<RecentBlocksAtTicks<T>>::get(tick).to_vec()
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn ticks_since_genesis() -> Tick {
			let genesis_tick = <GenesisTick<T>>::get();
			let current_tick = <CurrentTick<T>>::get();
			current_tick.saturating_sub(genesis_tick)
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
			panic!(
				"The proposed tick is in the future, which is not allowed. Digest tick={proposed_tick} vs Current tick={tick_for_now}"
			);
		}
	}
}
