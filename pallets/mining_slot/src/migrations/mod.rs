use crate::{Config, FrameRewardTicksRemaining, FrameStartTicks, Pallet};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::{argon_primitives::TickProvider, *};

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

mod old_storage {
	use super::*;
	#[storage_alias]
	pub type DidStartNewCohort<T: Config> = StorageValue<Pallet<T>, bool, ValueQuery>;
}

impl<T: Config + pallet_treasury::Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		log::info!("Migrating mining slots");
		let modify_count = 2;
		old_storage::DidStartNewCohort::<T>::kill();

		let next_frame_id = Pallet::<T>::next_frame_id();
		let ticks_per_slot = Pallet::<T>::ticks_between_slots();
		let added_ticks = (next_frame_id - 1) * ticks_per_slot;
		let transition_tick = Pallet::<T>::frame_1_begins_tick() + added_ticks;
		let current_tick = T::TickProvider::current_tick();
		if current_tick > transition_tick {
			panic!(
				"In mining slot migration, current tick {} is > than calculated NextFrameTransitionTick {}, indicating inconsistent state. Aborting migration to prevent future issues.",
				current_tick, transition_tick
			);
		}
		let remaining_ticks = transition_tick - current_tick;
		assert!(remaining_ticks < ticks_per_slot, "Remaining ticks cant be more than a day");
		FrameRewardTicksRemaining::<T>::put(transition_tick - current_tick);
		FrameStartTicks::<T>::mutate(|a| {
			let _ = a.try_insert(next_frame_id - 1, transition_tick - ticks_per_slot);
		});
		log::info!(
			"Set NextFrameTransitionTick to {:?} for frame {:?}. Current tick is {}",
			remaining_ticks,
			next_frame_id,
			current_tick
		);

		T::DbWeight::get().reads_writes(modify_count as u64, modify_count as u64)
	}
}

pub type FrameChangeMigration<T> = frame_support::migrations::VersionedMigration<
	9,
	10,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
