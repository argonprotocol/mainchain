use alloc::vec::Vec;
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

use crate::{pallet::Pallet as CrosschainTransferPallet, Config};

#[cfg(feature = "try-runtime")]
use frame_support::{ensure, traits::StorageVersion};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[storage_alias]
type RecentArgonTransfersByAccount<T: Config> = StorageMap<
	CrosschainTransferPallet<T>,
	Blake2_128Concat,
	<T as frame_system::Config>::AccountId,
	u32,
	ValueQuery,
>;

#[storage_alias]
type InboundTransfersExpiringAt<T: Config> = StorageMap<
	CrosschainTransferPallet<T>,
	Twox64Concat,
	Tick,
	Vec<<T as frame_system::Config>::AccountId>,
	ValueQuery,
>;

#[storage_alias]
type LastTransferExpiryCleanupTick<T: Config> =
	StorageValue<CrosschainTransferPallet<T>, Tick, ValueQuery>;

pub struct CleanupRecentTransferState<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for CleanupRecentTransferState<T> {
	fn on_runtime_upgrade() -> Weight {
		let recent_accounts = RecentArgonTransfersByAccount::<T>::drain().count() as u64;
		let expiry_ticks = InboundTransfersExpiringAt::<T>::drain().count() as u64;

		LastTransferExpiryCleanupTick::<T>::kill();

		T::DbWeight::get().reads_writes(
			recent_accounts.saturating_add(expiry_ticks),
			recent_accounts.saturating_add(expiry_ticks).saturating_add(1),
		)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
		ensure!(
			RecentArgonTransfersByAccount::<T>::iter_keys().count() as u64 == 0,
			TryRuntimeError::Other("recent transfer accounts were not cleared"),
		);
		ensure!(
			InboundTransfersExpiringAt::<T>::iter_keys().count() as u64 == 0,
			TryRuntimeError::Other("transfer expiry index was not cleared"),
		);
		ensure!(
			!LastTransferExpiryCleanupTick::<T>::exists(),
			TryRuntimeError::Other("cleanup tick was not cleared"),
		);
		ensure!(
			StorageVersion::get::<CrosschainTransferPallet<T>>() == 2,
			TryRuntimeError::Other("crosschain transfer storage version was not updated"),
		);

		Ok(())
	}
}

pub type CleanupRecentTransferStateMigration<T> = frame_support::migrations::VersionedMigration<
	1,
	2,
	CleanupRecentTransferState<T>,
	CrosschainTransferPallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod test {
	use super::*;
	use crate::mock::{account, new_test_ext, Test};
	use frame_support::traits::OnRuntimeUpgrade;

	#[test]
	fn clears_legacy_recent_transfer_storage() {
		new_test_ext().execute_with(|| {
			let first_account = account(1);
			let second_account = account(2);
			let expiry_tick = 9;

			frame_support::traits::StorageVersion::new(1).put::<CrosschainTransferPallet<Test>>();
			RecentArgonTransfersByAccount::<Test>::insert(&first_account, 1);
			RecentArgonTransfersByAccount::<Test>::insert(&second_account, 2);
			InboundTransfersExpiringAt::<Test>::insert(
				expiry_tick,
				vec![first_account.clone(), second_account.clone()],
			);
			LastTransferExpiryCleanupTick::<Test>::put(expiry_tick.saturating_sub(1));

			#[cfg(feature = "try-runtime")]
			let state = CleanupRecentTransferStateMigration::<Test>::pre_upgrade().expect("pre-upgrade");
			CleanupRecentTransferStateMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			CleanupRecentTransferStateMigration::<Test>::post_upgrade(state).expect("post-upgrade");

			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&first_account), 0);
			assert_eq!(RecentArgonTransfersByAccount::<Test>::get(&second_account), 0);
			assert!(InboundTransfersExpiringAt::<Test>::get(expiry_tick).is_empty());
			assert!(!LastTransferExpiryCleanupTick::<Test>::exists());
			assert_eq!(
				frame_support::traits::StorageVersion::get::<CrosschainTransferPallet<Test>>(),
				2,
			);
		});
	}
}
