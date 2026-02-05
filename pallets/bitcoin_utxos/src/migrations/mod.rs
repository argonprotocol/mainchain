use crate::Config;
use frame_support::{
	storage::{migration::move_prefix, storage_prefix},
	traits::UncheckedOnRuntimeUpgrade,
	weights::Weight,
};
use pallet_prelude::*;

pub struct RenamePendingConfirmationInner<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for RenamePendingConfirmationInner<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok(vec![])
	}

	fn on_runtime_upgrade() -> Weight {
		log::info!("Renaming pending funding storage");
		move_prefix(
			&storage_prefix(b"BitcoinUtxos", b"UtxosPendingConfirmation"),
			&storage_prefix(b"BitcoinUtxos", b"LocksPendingFunding"),
		);
		move_prefix(
			&storage_prefix(b"BitcoinUtxos", b"UtxoIdToRef"),
			&storage_prefix(b"BitcoinUtxos", b"UtxoIdToFundingUtxoRef"),
		);
		T::DbWeight::get().reads_writes(2, 2)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		Ok(())
	}
}

pub type RenamePendingConfirmation<T> = frame_support::migrations::VersionedMigration<
	1,
	2,
	RenamePendingConfirmationInner<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
