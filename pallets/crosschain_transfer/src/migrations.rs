use frame_support::{traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

use crate::{
	Config, NextCouncilRotationFrameByDestinationChain, Pallet as CrosschainTransferPallet,
	SourceChain,
};

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::{ensure, traits::StorageVersion};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

pub struct PrepareScheduledCouncilRotation<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for PrepareScheduledCouncilRotation<T> {
	fn on_runtime_upgrade() -> Weight {
		let destination_chain = SourceChain::Ethereum;

		NextCouncilRotationFrameByDestinationChain::<T>::insert(
			destination_chain,
			T::CurrentFrameId::get(),
		);

		T::DbWeight::get().writes(1)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
		ensure!(
			StorageVersion::get::<CrosschainTransferPallet<T>>() == 3,
			TryRuntimeError::Other("crosschain transfer storage version was not updated"),
		);

		ensure!(
			NextCouncilRotationFrameByDestinationChain::<T>::contains_key(SourceChain::Ethereum),
			TryRuntimeError::Other("next council rotation frame was not initialized"),
		);

		Ok(())
	}
}

pub type PrepareScheduledCouncilRotationMigration<T> =
	frame_support::migrations::VersionedMigration<
		2,
		3,
		PrepareScheduledCouncilRotation<T>,
		CrosschainTransferPallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;
