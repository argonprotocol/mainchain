use crate::{Config, LockIdByScriptPubkey, Pallet, UtxoAddressByLockId, UtxoRefsByLockId};
use alloc::collections::BTreeMap;
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinLockId, Satoshis, UtxoAddress, UtxoRef,
	},
	BitcoinUtxoEvents,
};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

mod old_storage {
	use super::*;

	#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
	pub struct UtxoAddress {
		pub lock_id: BitcoinLockId,
		pub script_pubkey: BitcoinCosignScriptPubkey,
		#[codec(compact)]
		pub satoshis: Satoshis,
		#[codec(compact)]
		pub submitted_at_height: BitcoinHeight,
		#[codec(compact)]
		pub watch_for_spent_until_height: BitcoinHeight,
	}

	#[storage_alias]
	pub type LockedUtxos<T: Config> =
		StorageMap<Pallet<T>, Blake2_128Concat, UtxoRef, UtxoAddress, OptionQuery>;

	#[storage_alias]
	pub type LocksPendingFunding<T: Config> =
		StorageValue<Pallet<T>, BTreeMap<BitcoinLockId, UtxoAddress>, ValueQuery>;

	#[storage_alias]
	pub type ExpiredPendingFunding<T: Config> =
		StorageValue<Pallet<T>, BTreeMap<BitcoinLockId, UtxoAddress>, ValueQuery>;

	#[storage_alias]
	pub type CandidateUtxoRefsByUtxoId<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, BitcoinLockId, BTreeMap<UtxoRef, Satoshis>, ValueQuery>;

	impl From<UtxoAddress> for argon_primitives::bitcoin::UtxoAddress {
		fn from(value: UtxoAddress) -> Self {
			Self {
				lock_id: value.lock_id,
				script_pubkey: value.script_pubkey,
				submitted_at_height: value.submitted_at_height,
			}
		}
	}
}

pub struct MigrateUtxoTracking<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for MigrateUtxoTracking<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut weight = Weight::zero();

		for (_, watch) in old_storage::LocksPendingFunding::<T>::take() {
			let watch = UtxoAddress::from(watch);
			LockIdByScriptPubkey::<T>::insert(watch.script_pubkey, watch.lock_id);
			UtxoAddressByLockId::<T>::insert(watch.lock_id, watch);
			weight.saturating_accrue(T::DbWeight::get().writes(2));
		}
		for (_, watch) in old_storage::ExpiredPendingFunding::<T>::take() {
			let watch = UtxoAddress::from(watch);
			LockIdByScriptPubkey::<T>::insert(watch.script_pubkey, watch.lock_id);
			UtxoAddressByLockId::<T>::insert(watch.lock_id, watch);
			weight.saturating_accrue(T::DbWeight::get().writes(2));
		}

		let locked_utxos = old_storage::LockedUtxos::<T>::iter().collect::<Vec<_>>();
		for (utxo_ref, watch) in locked_utxos {
			UtxoRefsByLockId::<T>::try_mutate(watch.lock_id, |refs| {
				refs.try_insert(utxo_ref.clone()).map_err(|_| ())
			})
			.expect("known legacy UTXO refs fit in the new tracker");
			let watch = UtxoAddress::from(watch);
			LockIdByScriptPubkey::<T>::insert(watch.script_pubkey, watch.lock_id);
			UtxoAddressByLockId::<T>::insert(watch.lock_id, watch);
			old_storage::LockedUtxos::<T>::remove(&utxo_ref);
			weight.saturating_accrue(T::DbWeight::get().writes(3));
		}

		let candidate_utxos =
			old_storage::CandidateUtxoRefsByUtxoId::<T>::iter().collect::<Vec<_>>();
		for (lock_id, candidates) in candidate_utxos {
			for (utxo_ref, satoshis) in candidates {
				UtxoRefsByLockId::<T>::try_mutate(lock_id, |refs| {
					if refs.contains(&utxo_ref) {
						Ok(())
					} else {
						refs.try_insert(utxo_ref.clone()).map(|_| ())
					}
				})
				.expect("known legacy candidate UTXO refs fit in the new tracker");
				T::EventHandler::utxo_detected(lock_id, utxo_ref, satoshis, BitcoinHeight::MAX)
					.expect("legacy candidate UTXOs replay after Bitcoin Locks are migrated");
			}
			old_storage::CandidateUtxoRefsByUtxoId::<T>::remove(lock_id);
			weight.saturating_accrue(T::DbWeight::get().writes(2));
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<alloc::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
		Ok(alloc::vec::Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_: alloc::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		ensure!(
			old_storage::LockedUtxos::<T>::iter_keys().next().is_none() &&
				old_storage::LocksPendingFunding::<T>::get().is_empty() &&
				old_storage::ExpiredPendingFunding::<T>::get().is_empty() &&
				old_storage::CandidateUtxoRefsByUtxoId::<T>::iter_keys().next().is_none(),
			"old UTXO tracking storage must be empty",
		);
		Ok(())
	}
}

pub type MigrateUtxoTrackingMigration<T> = frame_support::migrations::VersionedMigration<
	2,
	3,
	MigrateUtxoTracking<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use argon_primitives::bitcoin::H256Le;
	use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};

	#[test]
	fn migrates_every_old_output_into_neutral_tracking() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(2).put::<Pallet<Test>>();
			let first_old_watch = old_storage::UtxoAddress {
				lock_id: 1,
				script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
					wscript_hash: sp_core::H256::repeat_byte(1),
				},
				satoshis: 100,
				submitted_at_height: 1,
				watch_for_spent_until_height: 50,
			};
			let second_old_watch = old_storage::UtxoAddress {
				lock_id: 2,
				script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
					wscript_hash: sp_core::H256::repeat_byte(2),
				},
				satoshis: 200,
				submitted_at_height: 2,
				watch_for_spent_until_height: 60,
			};
			let first_watch = UtxoAddress::from(first_old_watch.clone());
			let second_watch = UtxoAddress::from(second_old_watch.clone());
			let first_funding_ref = UtxoRef { txid: H256Le([1; 32]), output_index: 0 };
			let second_funding_ref = UtxoRef { txid: H256Le([2; 32]), output_index: 0 };
			let first_candidate_ref = UtxoRef { txid: H256Le([3; 32]), output_index: 0 };
			let second_candidate_ref = UtxoRef { txid: H256Le([4; 32]), output_index: 0 };
			old_storage::LockedUtxos::<Test>::insert(&first_funding_ref, first_old_watch);
			old_storage::LockedUtxos::<Test>::insert(&second_funding_ref, second_old_watch);
			old_storage::CandidateUtxoRefsByUtxoId::<Test>::insert(
				1,
				BTreeMap::from([(first_candidate_ref.clone(), 90)]),
			);
			old_storage::CandidateUtxoRefsByUtxoId::<Test>::insert(
				2,
				BTreeMap::from([(second_candidate_ref.clone(), 190)]),
			);

			MigrateUtxoTrackingMigration::<Test>::on_runtime_upgrade();

			assert_eq!(UtxoAddressByLockId::<Test>::get(1), Some(first_watch));
			assert_eq!(UtxoAddressByLockId::<Test>::get(2), Some(second_watch));
			let first_refs = UtxoRefsByLockId::<Test>::get(1);
			assert!(first_refs.contains(&first_funding_ref));
			assert!(first_refs.contains(&first_candidate_ref));
			let second_refs = UtxoRefsByLockId::<Test>::get(2);
			assert!(second_refs.contains(&second_funding_ref));
			assert!(second_refs.contains(&second_candidate_ref));
			assert!(old_storage::LockedUtxos::<Test>::iter_keys().next().is_none());
			assert!(old_storage::CandidateUtxoRefsByUtxoId::<Test>::iter_keys().next().is_none());
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), StorageVersion::new(3));
		});
	}
}
