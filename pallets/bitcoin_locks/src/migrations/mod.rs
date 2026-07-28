use argon_primitives::bitcoin::{
	BitcoinCosignScriptPubkey, BitcoinHeight, CompressedBitcoinPubkey, Satoshis, UtxoId,
	XPubChildNumber, XPubFingerprint,
};
use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

use crate::{Config, LockedBitcoin, LocksByUtxoId, Pallet};

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::{ensure, traits::StorageVersion};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Decode, Encode)]
struct LockedBitcoinV8<T: Config> {
	#[codec(compact)]
	vault_id: VaultId,
	#[codec(compact)]
	liquidity_promised: T::Balance,
	#[codec(compact)]
	locked_target_price: T::Balance,
	owner_account: T::AccountId,
	securitization_ratio: FixedU128,
	#[codec(compact)]
	security_fees: T::Balance,
	#[codec(compact)]
	coupon_paid_fees: T::Balance,
	#[codec(compact)]
	satoshis: Satoshis,
	utxo_satoshis: Option<Satoshis>,
	vault_pubkey: CompressedBitcoinPubkey,
	vault_claim_pubkey: CompressedBitcoinPubkey,
	vault_xpub_sources: (XPubFingerprint, XPubChildNumber, XPubChildNumber),
	owner_pubkey: CompressedBitcoinPubkey,
	#[codec(compact)]
	vault_claim_height: BitcoinHeight,
	#[codec(compact)]
	open_claim_height: BitcoinHeight,
	#[codec(compact)]
	created_at_height: BitcoinHeight,
	utxo_script_pubkey: BitcoinCosignScriptPubkey,
	is_funded: bool,
	fund_hold_extensions: BoundedBTreeMap<BitcoinHeight, T::Balance, ConstU32<366>>,
	#[codec(compact)]
	created_at_argon_block: BlockNumberFor<T>,
}

mod v8 {
	use super::*;

	#[storage_alias]
	pub(super) type LocksByUtxoId<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, UtxoId, LockedBitcoinV8<T>, OptionQuery>;
}

pub struct AddBitcoinBackfillFlag<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for AddBitcoinBackfillFlag<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut lock_count = 0u64;
		LocksByUtxoId::<T>::translate::<LockedBitcoinV8<T>, _>(|_, lock| {
			lock_count = lock_count.saturating_add(1);
			Some(LockedBitcoin {
				vault_id: lock.vault_id,
				liquidity_promised: lock.liquidity_promised,
				locked_target_price: lock.locked_target_price,
				owner_account: lock.owner_account,
				securitization_ratio: lock.securitization_ratio,
				security_fees: lock.security_fees,
				coupon_paid_fees: lock.coupon_paid_fees,
				satoshis: lock.satoshis,
				utxo_satoshis: lock.utxo_satoshis,
				vault_pubkey: lock.vault_pubkey,
				vault_claim_pubkey: lock.vault_claim_pubkey,
				vault_xpub_sources: lock.vault_xpub_sources,
				owner_pubkey: lock.owner_pubkey,
				vault_claim_height: lock.vault_claim_height,
				open_claim_height: lock.open_claim_height,
				created_at_height: lock.created_at_height,
				utxo_script_pubkey: lock.utxo_script_pubkey,
				is_funded: lock.is_funded,
				is_backfill: false,
				fund_hold_extensions: lock.fund_hold_extensions,
				created_at_argon_block: lock.created_at_argon_block,
			})
		});

		T::DbWeight::get().reads_writes(lock_count, lock_count)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		ensure!(
			StorageVersion::get::<Pallet<T>>() == 8,
			TryRuntimeError::Other("bitcoin locks storage version must be 8 before migration"),
		);

		let lock_count =
			v8::LocksByUtxoId::<T>::iter().fold(0u64, |count, _| count.saturating_add(1));
		Ok(lock_count.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		ensure!(
			StorageVersion::get::<Pallet<T>>() == 9,
			TryRuntimeError::Other("bitcoin locks storage version was not updated"),
		);

		let expected_lock_count = u64::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode bitcoin lock count"))?;
		let mut migrated_lock_count = 0u64;
		for (_, lock) in LocksByUtxoId::<T>::iter() {
			migrated_lock_count = migrated_lock_count.saturating_add(1);
			ensure!(
				!lock.is_backfill,
				TryRuntimeError::Other("migrated bitcoin lock was marked as backfill"),
			);
		}
		ensure!(
			migrated_lock_count == expected_lock_count,
			TryRuntimeError::Other("bitcoin lock count changed during migration"),
		);

		Ok(())
	}
}

pub type AddBitcoinBackfillFlagMigration<T> = frame_support::migrations::VersionedMigration<
	8,
	9,
	AddBitcoinBackfillFlag<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(all(feature = "try-runtime", test))]
mod test {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::traits::OnRuntimeUpgrade;

	#[test]
	fn preserves_existing_locks_and_adds_the_backfill_flag() {
		new_test_ext().execute_with(|| {
			v8::LocksByUtxoId::<Test>::insert(
				1,
				LockedBitcoinV8 {
					vault_id: 7,
					liquidity_promised: 100,
					locked_target_price: 110,
					owner_account: 2,
					securitization_ratio: FixedU128::from_rational(3, 2),
					security_fees: 3,
					coupon_paid_fees: 4,
					satoshis: 5,
					utxo_satoshis: Some(6),
					vault_pubkey: CompressedBitcoinPubkey([7; 33]),
					vault_claim_pubkey: CompressedBitcoinPubkey([8; 33]),
					vault_xpub_sources: ([9; 4], 10, 11),
					owner_pubkey: CompressedBitcoinPubkey([12; 33]),
					vault_claim_height: 13,
					open_claim_height: 14,
					created_at_height: 15,
					utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
						wscript_hash: H256::from([16; 32]),
					},
					is_funded: true,
					fund_hold_extensions: BoundedBTreeMap::default(),
					created_at_argon_block: 17,
				},
			);
			StorageVersion::new(8).put::<Pallet<Test>>();

			let state = AddBitcoinBackfillFlagMigration::<Test>::pre_upgrade().unwrap();
			AddBitcoinBackfillFlagMigration::<Test>::on_runtime_upgrade();
			AddBitcoinBackfillFlagMigration::<Test>::post_upgrade(state).unwrap();

			assert_eq!(
				LocksByUtxoId::<Test>::get(1),
				Some(LockedBitcoin {
					vault_id: 7,
					liquidity_promised: 100,
					locked_target_price: 110,
					owner_account: 2,
					securitization_ratio: FixedU128::from_rational(3, 2),
					security_fees: 3,
					coupon_paid_fees: 4,
					satoshis: 5,
					utxo_satoshis: Some(6),
					vault_pubkey: CompressedBitcoinPubkey([7; 33]),
					vault_claim_pubkey: CompressedBitcoinPubkey([8; 33]),
					vault_xpub_sources: ([9; 4], 10, 11),
					owner_pubkey: CompressedBitcoinPubkey([12; 33]),
					vault_claim_height: 13,
					open_claim_height: 14,
					created_at_height: 15,
					utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
						wscript_hash: H256::from([16; 32]),
					},
					is_funded: true,
					is_backfill: false,
					fund_hold_extensions: BoundedBTreeMap::default(),
					created_at_argon_block: 17,
				}),
			);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), 9);
		});
	}
}
