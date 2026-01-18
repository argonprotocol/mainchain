use crate::{Config, migrations::old_storage::LockedUtxoExpirationsByBlock};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use crate::{Config, Pallet};
	use argon_primitives::bitcoin::BitcoinHeight;
	use frame_support::storage_alias;
	use pallet_prelude::{argon_primitives::bitcoin::UtxoRef, *};

	#[storage_alias]
	pub type LockedUtxoExpirationsByBlock<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		BitcoinHeight,
		BoundedVec<UtxoRef, <T as Config>::MaxPendingConfirmationUtxos>,
		ValueQuery,
	>;
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok(vec![])
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		log::info!("Migrating Bitcoin utxos");
		let res = LockedUtxoExpirationsByBlock::<T>::drain();
		let count = res.count() as u64;

		T::DbWeight::get().reads_writes(count, count)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		ensure!(
			LockedUtxoExpirationsByBlock::<T>::iter_keys().count() == 0,
			"expirations should have been removed",
		);

		Ok(())
	}
}

pub type BitcoinUtxosMigrate<T> = frame_support::migrations::VersionedMigration<
	0,
	1,
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
