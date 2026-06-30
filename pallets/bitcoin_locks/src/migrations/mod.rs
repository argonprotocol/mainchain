use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

use crate::{Config, LocksByUtxoId, Pallet as BitcoinLocksPallet, UtxoIdsByOwnerAccount};

#[cfg(feature = "try-runtime")]
use frame_support::{ensure, traits::StorageVersion};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

pub struct PopulateOwnerUtxoIndex<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for PopulateOwnerUtxoIndex<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<alloc::vec::Vec<u8>, TryRuntimeError> {
		Ok(LocksByUtxoId::<T>::iter()
			.map(|(utxo_id, lock)| (utxo_id, lock.owner_account))
			.collect::<alloc::vec::Vec<_>>()
			.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut lock_count = 0u64;

		for (utxo_id, lock) in LocksByUtxoId::<T>::iter() {
			UtxoIdsByOwnerAccount::<T>::insert(lock.owner_account, utxo_id, ());
			lock_count = lock_count.saturating_add(1);
		}

		T::DbWeight::get().reads_writes(lock_count, lock_count)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: alloc::vec::Vec<u8>) -> Result<(), TryRuntimeError> {
		let prior_locks =
			alloc::vec::Vec::<(argon_primitives::bitcoin::UtxoId, T::AccountId)>::decode(
				&mut &state[..],
			)
			.map_err(|_| TryRuntimeError::Other("failed to decode pre-upgrade state"))?;

		for (utxo_id, owner_account) in prior_locks {
			ensure!(
				UtxoIdsByOwnerAccount::<T>::contains_key(&owner_account, utxo_id),
				TryRuntimeError::Other("owner utxo index missing lock"),
			);
		}
		ensure!(
			StorageVersion::get::<BitcoinLocksPallet<T>>() == 8,
			TryRuntimeError::Other("bitcoin locks storage version was not updated"),
		);

		Ok(())
	}
}

pub type PopulateOwnerUtxoIndexMigration<T> = frame_support::migrations::VersionedMigration<
	7,
	8,
	PopulateOwnerUtxoIndex<T>,
	BitcoinLocksPallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod test {
	use super::*;
	use crate::{
		mock::{new_test_ext, Test},
		LockedBitcoin,
	};
	use argon_primitives::bitcoin::{BitcoinCosignScriptPubkey, CompressedBitcoinPubkey};
	use frame_support::traits::OnRuntimeUpgrade;
	use sp_core::H256;
	use sp_runtime::FixedU128;

	fn sample_lock(
		owner_account: u64,
		vault_id: u32,
		liquidity_promised: u128,
		is_funded: bool,
	) -> LockedBitcoin<Test> {
		LockedBitcoin {
			vault_id,
			liquidity_promised,
			locked_target_price: liquidity_promised,
			owner_account,
			securitization_ratio: FixedU128::one(),
			security_fees: 0,
			coupon_paid_fees: 0,
			satoshis: 1,
			utxo_satoshis: is_funded.then_some(1),
			vault_pubkey: CompressedBitcoinPubkey([0; 33]),
			vault_claim_pubkey: CompressedBitcoinPubkey([1; 33]),
			vault_xpub_sources: ([0; 4], 0, 1),
			owner_pubkey: CompressedBitcoinPubkey([2; 33]),
			vault_claim_height: 1,
			open_claim_height: 2,
			created_at_height: 0,
			utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
				wscript_hash: H256::from([0; 32]),
			},
			is_funded,
			fund_hold_extensions: BoundedBTreeMap::default(),
			created_at_argon_block: 0,
		}
	}

	#[test]
	fn populates_owner_utxo_index_from_existing_locks() {
		new_test_ext().execute_with(|| {
			frame_support::traits::StorageVersion::new(7).put::<BitcoinLocksPallet<Test>>();
			LocksByUtxoId::<Test>::insert(1, sample_lock(7, 1, 250, false));
			LocksByUtxoId::<Test>::insert(2, sample_lock(7, 1, 500, true));
			LocksByUtxoId::<Test>::insert(3, sample_lock(9, 2, 750, true));

			#[cfg(feature = "try-runtime")]
			let state = PopulateOwnerUtxoIndexMigration::<Test>::pre_upgrade().expect("pre-upgrade");
			PopulateOwnerUtxoIndexMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			PopulateOwnerUtxoIndexMigration::<Test>::post_upgrade(state).expect("post-upgrade");

			assert!(UtxoIdsByOwnerAccount::<Test>::contains_key(7, 1));
			assert!(UtxoIdsByOwnerAccount::<Test>::contains_key(7, 2));
			assert!(UtxoIdsByOwnerAccount::<Test>::contains_key(9, 3));
			assert_eq!(frame_support::traits::StorageVersion::get::<BitcoinLocksPallet<Test>>(), 8,);
		});
	}
}
