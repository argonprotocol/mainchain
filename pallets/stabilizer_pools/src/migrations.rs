use crate::{Config, Pallet};
use argon_primitives::{block_seal::FrameId, PriceIndex};
use frame_support::{
	migrations::VersionedMigration, storage_alias, traits::UncheckedOnRuntimeUpgrade,
};
use pallet_prelude::*;

pub(crate) mod v1 {
	use super::*;

	#[storage_alias]
	pub type FrameStartPriceIndices<T: Config> =
		StorageValue<Pallet<T>, BoundedBTreeMap<FrameId, PriceIndex, ConstU32<33>>, ValueQuery>;
}

pub struct RemoveOpeningPriceIndices<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for RemoveOpeningPriceIndices<T> {
	fn on_runtime_upgrade() -> Weight {
		// An opening observation can't establish a representative price for a closed frame.
		v1::FrameStartPriceIndices::<T>::kill();
		T::DbWeight::get().writes(1)
	}
}

pub type RemoveOpeningPriceIndicesMigration<T> = VersionedMigration<
	1,
	2,
	RemoveOpeningPriceIndices<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
