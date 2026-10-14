use crate::{AveragePricePerSeat, Config, MinersByCohort, NextFrameId, Pallet, SeatPriceStats};
use frame_support::{
	migrations::VersionedMigration, storage_alias, traits::UncheckedOnRuntimeUpgrade,
};
use pallet_prelude::*;

pub(crate) mod v10 {
	use super::*;

	#[storage_alias]
	pub type AveragePricePerSeat<T: Config> =
		StorageValue<Pallet<T>, BoundedVec<<T as Config>::Balance, ConstU32<10>>, ValueQuery>;
}

pub struct MigrateSeatPriceHistory<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateSeatPriceHistory<T> {
	fn on_runtime_upgrade() -> Weight {
		let current_frame = NextFrameId::<T>::get().saturating_sub(1);
		let observed_frames = (v10::AveragePricePerSeat::<T>::get().len() as FrameId)
			.min(T::FramesPerMiningTerm::get() as FrameId)
			.min(current_frame);
		let mut history = BoundedVec::new();
		for offset in 0..observed_frames {
			let cohort = MinersByCohort::<T>::get(current_frame - offset);
			let total_bids = cohort
				.iter()
				.fold(T::Balance::zero(), |sum, miner| sum.saturating_add(miner.bid));
			history
				.try_push(SeatPriceStats { seats: cohort.len() as u32, total_bids })
				.expect("Old price history fits in the same ten-frame bound");
		}
		AveragePricePerSeat::<T>::put(history);
		// VersionedMigration accounts for the storage-version read and write separately.
		T::DbWeight::get().reads_writes(observed_frames + 2, 1)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<alloc::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok((v10::AveragePricePerSeat::<T>::get().len() as FrameId)
			.min(T::FramesPerMiningTerm::get() as FrameId)
			.min(NextFrameId::<T>::get().saturating_sub(1))
			.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: alloc::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let observed_frames = FrameId::decode(&mut state.as_slice())
			.map_err(|_| "Invalid seat-price migration state")?;
		let history = AveragePricePerSeat::<T>::get();
		ensure!(history.len() as FrameId == observed_frames, "Seat-price history length changed");
		let current_frame = NextFrameId::<T>::get().saturating_sub(1);
		for (offset, auction) in history.iter().enumerate() {
			let cohort = MinersByCohort::<T>::get(current_frame - offset as FrameId);
			let total_bids = cohort
				.iter()
				.fold(T::Balance::zero(), |sum, miner| sum.saturating_add(miner.bid));
			ensure!(auction.seats == cohort.len() as u32, "Winning seat count was not preserved");
			ensure!(auction.total_bids == total_bids, "Winning bid total was not preserved");
		}
		Ok(())
	}
}

pub type MigrateSeatPriceHistoryMigration<T> = VersionedMigration<
	10,
	11,
	MigrateSeatPriceHistory<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
