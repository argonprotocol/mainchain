use crate::{
	types::{
		ExecutionBlockHash, ExecutionHeaderAnchor, ExecutionHeaderAnchorScanKey, ReceiptsRoot,
	},
	Config, ExecutionHeaderAnchors, ExecutionHeaderAnchorsByBlockNumber, Pallet,
};
#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
#[cfg(feature = "try-runtime")]
use polkadot_sdk::sp_runtime::TryRuntimeError;
use polkadot_sdk::{
	frame_support::{
		migrations::VersionedMigration, pallet_prelude::*, storage_alias,
		traits::UncheckedOnRuntimeUpgrade, weights::Weight, Identity,
	},
	sp_core::H256,
};
use scale_info::TypeInfo;

mod v0 {
	use super::*;

	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Copy,
		PartialEq,
		Eq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct ExecutionHeaderAnchor {
		#[codec(compact)]
		pub block_number: u64,
		#[codec(compact)]
		pub timestamp_millis: u64,
		pub block_hash: ExecutionBlockHash,
		pub parent_hash: ExecutionBlockHash,
		pub receipts_root: ReceiptsRoot,
	}

	impl ExecutionHeaderAnchor {
		pub fn into_current(self) -> super::ExecutionHeaderAnchor {
			super::ExecutionHeaderAnchor {
				block_number: self.block_number,
				timestamp_millis: self.timestamp_millis,
				block_hash: self.block_hash,
				parent_hash: self.parent_hash,
				state_root: H256::zero(),
				receipts_root: self.receipts_root,
			}
		}
	}

	#[storage_alias]
	pub type ExecutionHeaderAnchors<T: Config> =
		StorageMap<Pallet<T>, Identity, ExecutionBlockHash, ExecutionHeaderAnchor, OptionQuery>;

	#[storage_alias]
	pub type ExecutionHeaderAnchorsByBlockNumber<T: Config> = StorageMap<
		Pallet<T>,
		Identity,
		ExecutionHeaderAnchorScanKey,
		ExecutionHeaderAnchor,
		OptionQuery,
	>;
}

pub struct MigrateExecutionHeaderAnchorsV0ToV1<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateExecutionHeaderAnchorsV0ToV1<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		Ok((
			ExecutionHeaderAnchors::<T>::iter_keys().count() as u64,
			ExecutionHeaderAnchorsByBlockNumber::<T>::iter_keys().count() as u64,
		)
			.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut hash_anchor_count = 0u64;
		ExecutionHeaderAnchors::<T>::translate::<v0::ExecutionHeaderAnchor, _>(|_, anchor| {
			hash_anchor_count = hash_anchor_count.saturating_add(1);
			Some(anchor.into_current())
		});

		let mut scan_anchor_count = 0u64;
		ExecutionHeaderAnchorsByBlockNumber::<T>::translate::<v0::ExecutionHeaderAnchor, _>(
			|_, anchor| {
				scan_anchor_count = scan_anchor_count.saturating_add(1);
				Some(anchor.into_current())
			},
		);

		let reads = hash_anchor_count.saturating_add(scan_anchor_count);
		let writes = reads;
		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		use polkadot_sdk::frame_support::ensure;

		let (expected_hash_anchor_count, expected_scan_anchor_count) =
			<(u64, u64)>::decode(&mut &state[..]).map_err(|_| {
				TryRuntimeError::Other("failed to decode verifier migration counts")
			})?;

		ensure!(
			ExecutionHeaderAnchors::<T>::iter_values().count() as u64 == expected_hash_anchor_count,
			"execution header anchor hash count changed during migration"
		);
		ensure!(
			ExecutionHeaderAnchorsByBlockNumber::<T>::iter_values().count() as u64 ==
				expected_scan_anchor_count,
			"execution header anchor scan count changed during migration"
		);
		ensure!(
			ExecutionHeaderAnchors::<T>::iter_values()
				.all(|anchor| anchor.state_root == H256::zero()),
			"legacy execution header anchors were not updated with a default state root"
		);
		ensure!(
			ExecutionHeaderAnchorsByBlockNumber::<T>::iter_values()
				.all(|anchor| anchor.state_root == H256::zero()),
			"legacy execution header scan anchors were not updated with a default state root"
		);

		Ok(())
	}
}

pub type MigrateExecutionHeaderAnchorsV0ToV1Migration<T> = VersionedMigration<
	0,
	1,
	MigrateExecutionHeaderAnchorsV0ToV1<T>,
	Pallet<T>,
	<T as polkadot_sdk::frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod test {
	use super::{v0, MigrateExecutionHeaderAnchorsV0ToV1Migration};
	use crate::{
		mock::{new_tester, Test},
		ExecutionHeaderAnchors, ExecutionHeaderAnchorsByBlockNumber,
	};
	use polkadot_sdk::{
		frame_support::traits::{GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
		sp_core::H256,
	};

	#[test]
	fn migrates_legacy_execution_header_anchors() {
		new_tester().execute_with(|| {
			let legacy_anchor = v0::ExecutionHeaderAnchor {
				block_number: 42,
				timestamp_millis: 777_000,
				block_hash: H256::repeat_byte(0x11),
				parent_hash: H256::repeat_byte(0x22),
				receipts_root: H256::repeat_byte(0x33),
			};
			let scan_key = legacy_anchor.block_number.to_be_bytes();

			v0::ExecutionHeaderAnchors::<Test>::insert(legacy_anchor.block_hash, legacy_anchor);
			v0::ExecutionHeaderAnchorsByBlockNumber::<Test>::insert(scan_key, legacy_anchor);
			StorageVersion::new(0).put::<crate::Pallet<Test>>();

			MigrateExecutionHeaderAnchorsV0ToV1Migration::<Test>::on_runtime_upgrade();

			let migrated_anchor =
				ExecutionHeaderAnchors::<Test>::get(legacy_anchor.block_hash).expect("anchor");
			assert_eq!(migrated_anchor.block_number, legacy_anchor.block_number);
			assert_eq!(migrated_anchor.timestamp_millis, legacy_anchor.timestamp_millis);
			assert_eq!(migrated_anchor.block_hash, legacy_anchor.block_hash);
			assert_eq!(migrated_anchor.parent_hash, legacy_anchor.parent_hash);
			assert_eq!(migrated_anchor.receipts_root, legacy_anchor.receipts_root);
			assert_eq!(migrated_anchor.state_root, H256::zero());
			assert_eq!(
				ExecutionHeaderAnchorsByBlockNumber::<Test>::get(scan_key),
				Some(migrated_anchor),
			);
			assert_eq!(
				<crate::Pallet<Test> as GetStorageVersion>::on_chain_storage_version(),
				StorageVersion::new(1),
			);
		});
	}
}
