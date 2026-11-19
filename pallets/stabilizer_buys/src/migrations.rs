use crate::{ApprovedPoolRevision, Config, Pallet};
use frame_support::{migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

pub struct InitializeApprovedPoolRevision<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InitializeApprovedPoolRevision<T> {
	fn on_runtime_upgrade() -> Weight {
		// An explicit revision lets the oracle distinguish this block from pre-upgrade state.
		ApprovedPoolRevision::<T>::put(0);
		T::DbWeight::get().writes(1)
	}
}

pub type InitializeApprovedPoolRevisionMigration<T> = VersionedMigration<
	1,
	2,
	InitializeApprovedPoolRevision<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
