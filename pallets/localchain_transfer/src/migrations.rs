use crate::{Config, Pallet};
#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
use frame_support::{
	storage::migration::move_pallet,
	traits::{
		OnRuntimeUpgrade, PalletInfoAccess, StorageVersion, STORAGE_VERSION_STORAGE_KEY_POSTFIX,
	},
	weights::Weight,
};
#[cfg(feature = "try-runtime")]
use frame_support::{storage::KeyPrefixIterator, traits::GetStorageVersion};
use pallet_prelude::*;
use sp_io::hashing::twox_128;

const OLD_PALLET_NAME: &str = "ChainTransfer";
const LOG_TARGET: &str = "runtime::localchain_transfer::migration";

pub struct RenameChainTransferPallet<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for RenameChainTransferPallet<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let new_pallet_name = <Pallet<T> as PalletInfoAccess>::name();

		if new_pallet_name == OLD_PALLET_NAME {
			return Ok((0u64, 0u64).encode());
		}

		let old_key_count = count_prefixed_keys(&twox_128(OLD_PALLET_NAME.as_bytes()));
		let new_pallet_prefix = twox_128(new_pallet_name.as_bytes());
		let new_non_version_key_count = count_non_version_prefixed_keys(&new_pallet_prefix);

		if old_key_count > 0 {
			frame_support::ensure!(
				new_non_version_key_count == 0,
				"new pallet prefix already contains migrated storage"
			);
		}

		Ok((old_key_count, new_non_version_key_count).encode())
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

		let old_pallet_prefix = twox_128(OLD_PALLET_NAME.as_bytes());
		let moved_keys = count_prefixed_keys(&old_pallet_prefix);
		if moved_keys == 0 {
			log::info!(
				target: LOG_TARGET,
				"no legacy {OLD_PALLET_NAME} storage found under the old pallet prefix",
			);
			return Weight::zero();
		}

		let new_pallet_prefix = twox_128(new_pallet_name.as_bytes());
		let new_non_version_key_count = count_non_version_prefixed_keys(&new_pallet_prefix);
		if new_non_version_key_count > 0 {
			log::error!(
				target: LOG_TARGET,
				"refusing to migrate {OLD_PALLET_NAME} storage because {new_pallet_name} already contains {new_non_version_key_count} non-version keys",
			);
			return Weight::zero();
		}

		log::info!(
			target: LOG_TARGET,
			"moving pallet storage prefix from {OLD_PALLET_NAME} to {new_pallet_name}",
		);

		move_pallet(OLD_PALLET_NAME.as_bytes(), new_pallet_name.as_bytes());
		StorageVersion::new(1).put::<Pallet<T>>();
		let migrated_key_count = count_prefixed_keys(&new_pallet_prefix);
		log::info!(
			target: LOG_TARGET,
			"migrated {moved_keys} legacy {OLD_PALLET_NAME} keys; {new_pallet_name} now has {migrated_key_count} total keys",
		);

		T::DbWeight::get().reads_writes(
			moved_keys.saturating_mul(3).saturating_add(2),
			moved_keys.saturating_mul(2).saturating_add(1),
		)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let (_old_key_count, _new_non_version_key_count_before) =
			<(u64, u64)>::decode(&mut &state[..])
				.map_err(|_| "invalid localchain migration state")?;
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
		let new_key_count_after = count_prefixed_keys(&new_pallet_prefix);
		log::info!(
			target: LOG_TARGET,
			"post-upgrade {new_pallet_name} key count: {new_key_count_after}",
		);
		frame_support::ensure!(
			<Pallet<T> as GetStorageVersion>::on_chain_storage_version() == StorageVersion::new(1),
			"wrong storage version after migration"
		);

		Ok(())
	}
}

pub type RenameChainTransferPalletMigration<T> = RenameChainTransferPallet<T>;

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

fn count_non_version_prefixed_keys(prefix: &[u8]) -> u64 {
	let mut key_count = 0u64;
	let storage_version_key = [prefix, &twox_128(STORAGE_VERSION_STORAGE_KEY_POSTFIX)].concat();
	let mut next_key = sp_io::storage::next_key(prefix);

	while let Some(key) = next_key {
		if !key.starts_with(prefix) {
			break;
		}

		if key != storage_version_key {
			key_count = key_count.saturating_add(1);
		}

		next_key = sp_io::storage::next_key(&key);
	}

	key_count
}
