use crate::{Config, Pallet};
use frame_support::{
	migrations::VersionedMigration,
	storage::migration::move_pallet,
	traits::{PalletInfoAccess, UncheckedOnRuntimeUpgrade},
	weights::Weight,
};
#[cfg(feature = "try-runtime")]
use frame_support::{
	storage::KeyPrefixIterator,
	traits::{GetStorageVersion, StorageVersion, STORAGE_VERSION_STORAGE_KEY_POSTFIX},
};
use pallet_prelude::*;
use sp_io::hashing::twox_128;

const OLD_PALLET_NAME: &str = "ChainTransfer";
const LOG_TARGET: &str = "runtime::localchain_transfer::migration";

pub struct RenameChainTransferPallet<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for RenameChainTransferPallet<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let new_pallet_name = <Pallet<T> as PalletInfoAccess>::name();

		if new_pallet_name == OLD_PALLET_NAME {
			return Ok(Vec::new());
		}

		let new_pallet_prefix = twox_128(new_pallet_name.as_bytes());
		let storage_version_key = twox_128(STORAGE_VERSION_STORAGE_KEY_POSTFIX);
		let mut new_pallet_prefix_iter =
			KeyPrefixIterator::new(new_pallet_prefix.to_vec(), new_pallet_prefix.to_vec(), |key| {
				Ok(key.to_vec())
			});

		frame_support::ensure!(
			new_pallet_prefix_iter.all(|key| key == storage_version_key),
			"new pallet prefix already contains non-version storage"
		);

		Ok(Vec::new())
	}

	fn on_runtime_upgrade() -> Weight {
		let new_pallet_name = <Pallet<T> as PalletInfoAccess>::name();

		if new_pallet_name == OLD_PALLET_NAME {
			log::warn!(
				target: LOG_TARGET,
				"skipping localchain transfer rename migration because the new and old pallet names match",
			);
			return Weight::zero();
		}

		log::info!(
			target: LOG_TARGET,
			"moving pallet storage prefix from {OLD_PALLET_NAME} to {new_pallet_name}",
		);

		let moved_keys = count_prefixed_keys(&twox_128(OLD_PALLET_NAME.as_bytes()));
		move_pallet(OLD_PALLET_NAME.as_bytes(), new_pallet_name.as_bytes());

		T::DbWeight::get().reads_writes(
			moved_keys.saturating_mul(3).saturating_add(1),
			moved_keys.saturating_mul(2),
		)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let new_pallet_name = <Pallet<T> as PalletInfoAccess>::name();

		if new_pallet_name == OLD_PALLET_NAME {
			return Ok(());
		}

		let old_pallet_prefix = twox_128(OLD_PALLET_NAME.as_bytes());
		let old_pallet_prefix_iter =
			KeyPrefixIterator::new(old_pallet_prefix.to_vec(), old_pallet_prefix.to_vec(), |_| {
				Ok(())
			});
		frame_support::ensure!(
			old_pallet_prefix_iter.count() == 0,
			"old pallet prefix still contains storage"
		);

		let new_pallet_prefix = twox_128(new_pallet_name.as_bytes());
		let new_pallet_prefix_iter =
			KeyPrefixIterator::new(new_pallet_prefix.to_vec(), new_pallet_prefix.to_vec(), |_| {
				Ok(())
			});
		frame_support::ensure!(
			new_pallet_prefix_iter.count() >= 1,
			"new pallet prefix is missing migrated storage"
		);
		frame_support::ensure!(
			<Pallet<T> as GetStorageVersion>::on_chain_storage_version() == StorageVersion::new(1),
			"wrong storage version after migration"
		);

		Ok(())
	}
}

pub type RenameChainTransferPalletMigration<T> = VersionedMigration<
	0,
	1,
	RenameChainTransferPallet<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

fn count_prefixed_keys(prefix: &[u8]) -> u64 {
	let mut key_count = 0u64;
	let mut next_key = sp_io::storage::next_key(prefix);

	while let Some(key) = next_key {
		if !key.starts_with(prefix) {
			break;
		}

		key_count = key_count.saturating_add(1);
		next_key = sp_io::storage::next_key(&key);
	}

	key_count
}
