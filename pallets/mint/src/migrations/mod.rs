use crate::{Config, Pallet};
use frame_support::{
	migrations::VersionedMigration, storage_alias, traits::UncheckedOnRuntimeUpgrade,
};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

mod v3 {
	use super::*;

	#[storage_alias]
	pub(super) type MintedMiningMicrogons<T: Config> =
		StorageValue<Pallet<T>, <T as Config>::Balance, ValueQuery>;
}

pub struct RemoveMiningMintCounter<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for RemoveMiningMintCounter<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(v3::MintedMiningMicrogons::<T>::exists().encode())
	}

	fn on_runtime_upgrade() -> Weight {
		v3::MintedMiningMicrogons::<T>::kill();
		T::DbWeight::get().writes(1)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let _was_present = bool::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("invalid mining counter migration state"))?;
		ensure!(
			!v3::MintedMiningMicrogons::<T>::exists(),
			TryRuntimeError::Other("mining mint counter was not removed"),
		);
		Ok(())
	}
}

pub type RemoveMiningMintCounterMigration<T> = VersionedMigration<
	3,
	4,
	RemoveMiningMintCounter<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use frame_support::traits::OnRuntimeUpgrade;

	#[test]
	fn v3_removes_the_redundant_mining_mint_counter() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(3).put::<Pallet<Test>>();
			v3::MintedMiningMicrogons::<Test>::put(123);
			#[cfg(feature = "try-runtime")]
			let upgrade_state = RemoveMiningMintCounter::<Test>::pre_upgrade().unwrap();

			RemoveMiningMintCounterMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			RemoveMiningMintCounter::<Test>::post_upgrade(upgrade_state).unwrap();

			assert!(!v3::MintedMiningMicrogons::<Test>::exists());
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), StorageVersion::new(4));
		});
	}
}
