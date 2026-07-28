use crate::{pallet::Pallet as OperationalAccountsPallet, Config};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[storage_alias]
type EncryptedServerByDownstreamAccount<T: Config> = StorageMap<
	OperationalAccountsPallet<T>,
	Blake2_128Concat,
	<T as frame_system::Config>::AccountId,
	Vec<u8>,
>;

pub struct RemoveEncryptedServerStorage<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for RemoveEncryptedServerStorage<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok(Vec::new())
	}

	fn on_runtime_upgrade() -> Weight {
		let removed_count = EncryptedServerByDownstreamAccount::<T>::drain().count() as u64;
		T::DbWeight::get().reads_writes(removed_count, removed_count)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
		ensure!(
			EncryptedServerByDownstreamAccount::<T>::iter_keys().next().is_none(),
			TryRuntimeError::Other("encrypted server storage was not removed"),
		);
		Ok(())
	}
}

pub type RemoveLegacyEncryptedServerStorageMigration<T> =
	frame_support::migrations::VersionedMigration<
		2,
		3,
		RemoveEncryptedServerStorage<T>,
		OperationalAccountsPallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;

#[cfg(test)]
mod test {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::traits::OnRuntimeUpgrade;
	use sp_runtime::AccountId32;

	#[test]
	fn removes_legacy_encrypted_server_storage() {
		new_test_ext().execute_with(|| {
			let downstream_account = AccountId32::new([21; 32]);
			frame_support::traits::StorageVersion::new(2).put::<OperationalAccountsPallet<Test>>();
			EncryptedServerByDownstreamAccount::<Test>::insert(&downstream_account, vec![7u8; 32]);

			RemoveLegacyEncryptedServerStorageMigration::<Test>::on_runtime_upgrade();

			assert!(!EncryptedServerByDownstreamAccount::<Test>::contains_key(downstream_account));
			assert_eq!(
				frame_support::traits::StorageVersion::get::<OperationalAccountsPallet<Test>>(),
				frame_support::traits::StorageVersion::new(3)
			);
		});
	}
}
