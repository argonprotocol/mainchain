use crate::{Config, LocksByUtxoId, UtxoIdsByVaultId};
use frame_support::{traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

pub struct PopulateVaultUtxoIndex<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for PopulateVaultUtxoIndex<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;
		let count = LocksByUtxoId::<T>::iter().count() as u64;
		Ok(count.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut count = 0u64;
		log::info!("Populating UtxoIdsByVaultId index");

		for (utxo_id, lock) in LocksByUtxoId::<T>::iter() {
			UtxoIdsByVaultId::<T>::insert(lock.vault_id, utxo_id, ());
			count += 1;
		}

		T::DbWeight::get().reads_writes(count, count)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;
		use sp_core::Decode;

		let expected_count = u64::decode(&mut &state[..])
			.map_err(|_| sp_runtime::TryRuntimeError::Other("Failed to decode count"))?;

		let index_count = UtxoIdsByVaultId::<T>::iter().count() as u64;
		ensure!(expected_count == index_count, "UtxoIdsByVaultId count mismatch");

		for (utxo_id, lock) in LocksByUtxoId::<T>::iter() {
			ensure!(
				UtxoIdsByVaultId::<T>::contains_key(lock.vault_id, utxo_id),
				"Missing vault index entry"
			);
		}

		Ok(())
	}
}

pub type PopulateVaultUtxoIndexMigration<T> = frame_support::migrations::VersionedMigration<
	6,
	7,
	PopulateVaultUtxoIndex<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{Test, new_test_ext};
	use argon_primitives::bitcoin::{BitcoinCosignScriptPubkey, CompressedBitcoinPubkey};
	use frame_support::assert_ok;
	use sp_core::H256;

	#[test]
	fn populates_vault_utxo_index() {
		new_test_ext().execute_with(|| {
			let lock_v1 = crate::LockedBitcoin::<Test> {
				vault_id: 1,
				liquidity_promised: 100,
				locked_market_rate: 100,
				owner_account: 1,
				securitization_ratio: FixedU128::one(),
				security_fees: 10u128,
				coupon_paid_fees: 0u128,
				satoshis: 1000,
				utxo_satoshis: Some(1000),
				vault_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_claim_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_xpub_sources: Default::default(),
				owner_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_claim_height: 1,
				open_claim_height: 1,
				created_at_height: 1,
				utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
					wscript_hash: H256::from([0u8; 32]),
				},
				is_funded: true,
				fund_hold_extensions: Default::default(),
				created_at_argon_block: 1,
			};
			let lock_v2 = crate::LockedBitcoin::<Test> { vault_id: 2, ..lock_v1.clone() };
			let lock_v1b = crate::LockedBitcoin::<Test> { vault_id: 1, ..lock_v1.clone() };

			LocksByUtxoId::<Test>::insert(1, lock_v1);
			LocksByUtxoId::<Test>::insert(2, lock_v2);
			LocksByUtxoId::<Test>::insert(3, lock_v1b);

			let bytes = PopulateVaultUtxoIndex::<Test>::pre_upgrade().unwrap();
			let weight = PopulateVaultUtxoIndex::<Test>::on_runtime_upgrade();
			assert_ok!(PopulateVaultUtxoIndex::<Test>::post_upgrade(bytes));

			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 3));

			assert!(UtxoIdsByVaultId::<Test>::contains_key(1, 1));
			assert!(UtxoIdsByVaultId::<Test>::contains_key(2, 2));
			assert!(UtxoIdsByVaultId::<Test>::contains_key(1, 3));
			assert!(!UtxoIdsByVaultId::<Test>::contains_key(2, 1));
		});
	}
}
