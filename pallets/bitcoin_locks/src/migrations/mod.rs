use crate::{pallet::LocksByUtxoId, Config, LockedBitcoin, OwedUtxoAggrieved};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use super::*;
	use crate::Config;
	use argon_primitives::{
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, CompressedBitcoinPubkey, Satoshis, UtxoId,
			XPubChildNumber, XPubFingerprint,
		},
		ObligationId, VaultId,
	};
	use frame_support::storage_alias;

	#[derive(Decode, Encode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct LockedBitcoin<T: Config> {
		#[codec(compact)]
		pub obligation_id: ObligationId,
		#[codec(compact)]
		pub vault_id: VaultId,
		pub lock_price: T::Balance,
		pub owner_account: T::AccountId,
		#[codec(compact)]
		pub satoshis: Satoshis,
		pub vault_pubkey: CompressedBitcoinPubkey,
		pub vault_claim_pubkey: CompressedBitcoinPubkey,
		/// The vault xpub sources. First is the cosign number, second is the claim number
		pub vault_xpub_sources: (XPubFingerprint, XPubChildNumber, XPubChildNumber),
		pub owner_pubkey: CompressedBitcoinPubkey,
		#[codec(compact)]
		pub vault_claim_height: BitcoinHeight,
		#[codec(compact)]
		pub open_claim_height: BitcoinHeight,
		#[codec(compact)]
		pub created_at_height: BitcoinHeight,
		pub utxo_script_pubkey: BitcoinCosignScriptPubkey,
		pub is_verified: bool,
	}

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub locked_utxos: Vec<(UtxoId, LockedBitcoin<T>)>,
		#[allow(clippy::type_complexity)]
		pub owed_utxos: Vec<(
			UtxoId,
			(
				<T as frame_system::Config>::AccountId,
				VaultId,
				<T as Config>::Balance,
				LockedBitcoin<T>,
			),
		)>,
	}

	#[storage_alias]
	pub(super) type OwedUtxoAggrieved<T: Config> = StorageMap<
		crate::Pallet<T>,
		Twox64Concat,
		UtxoId,
		(<T as frame_system::Config>::AccountId, VaultId, <T as Config>::Balance, LockedBitcoin<T>),
		OptionQuery,
	>;

	#[storage_alias]
	pub(super) type LocksByUtxoId<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, UtxoId, LockedBitcoin<T>, OptionQuery>;
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Access the old value using the `storage_alias` type
		let owed_utxos = old_storage::OwedUtxoAggrieved::<T>::iter().collect::<Vec<_>>();
		let locked_utxos = old_storage::LocksByUtxoId::<T>::iter().collect::<Vec<_>>();

		Ok(<old_storage::Model<T>>::encode(&old_storage::Model { locked_utxos, owed_utxos }))
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating Bitcoin locks");
		LocksByUtxoId::<T>::translate_values::<old_storage::LockedBitcoin<T>, _>(|a| {
			count += 1;
			Some(LockedBitcoin {
				obligation_id: a.obligation_id,
				vault_id: a.vault_id,
				lock_price: a.lock_price,
				owner_account: a.owner_account,
				satoshis: a.satoshis,
				vault_pubkey: a.vault_pubkey,
				vault_claim_pubkey: a.vault_claim_pubkey,
				vault_xpub_sources: a.vault_xpub_sources,
				owner_pubkey: a.owner_pubkey,
				vault_claim_height: a.vault_claim_height,
				open_claim_height: a.open_claim_height,
				created_at_height: a.created_at_height,
				utxo_script_pubkey: a.utxo_script_pubkey,
				is_verified: a.is_verified,
				is_rejected_needs_release: false,
			})
		});

		OwedUtxoAggrieved::<T>::translate_values::<
			(T::AccountId, VaultId, T::Balance, old_storage::LockedBitcoin<T>),
			_,
		>(|(account_id, vault_id, b, lock)| {
			count += 1;
			Some((
				account_id,
				vault_id,
				b,
				LockedBitcoin {
					obligation_id: lock.obligation_id,
					vault_id: lock.vault_id,
					lock_price: lock.lock_price,
					owner_account: lock.owner_account,
					satoshis: lock.satoshis,
					vault_pubkey: lock.vault_pubkey,
					vault_claim_pubkey: lock.vault_claim_pubkey,
					vault_xpub_sources: lock.vault_xpub_sources,
					owner_pubkey: lock.owner_pubkey,
					vault_claim_height: lock.vault_claim_height,
					open_claim_height: lock.open_claim_height,
					created_at_height: lock.created_at_height,
					utxo_script_pubkey: lock.utxo_script_pubkey,
					is_verified: lock.is_verified,
					is_rejected_needs_release: false,
				},
			))
		});

		T::DbWeight::get().reads_writes(count as u64, count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;
		use sp_core::Decode;

		let old = <old_storage::Model<T>>::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		let new = LocksByUtxoId::<T>::iter().collect::<Vec<_>>();
		ensure!(old.locked_utxos.len() == new.len(), "locked_utxos length mismatch",);
		let new = OwedUtxoAggrieved::<T>::iter().collect::<Vec<_>>();
		ensure!(old.owed_utxos.len() == new.len(), "owed_utxos length mismatch");

		Ok(())
	}
}

pub type RejectedBitcoinMigration<T> = frame_support::migrations::VersionedMigration<
	0,
	1,
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use argon_primitives::bitcoin::{BitcoinCosignScriptPubkey, CompressedBitcoinPubkey};
	use frame_support::assert_ok;
	use sp_core::H256;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			let utxo_1 = old_storage::LockedBitcoin {
				obligation_id: 1,
				vault_id: 1,
				lock_price: 1,
				owner_account: 1,
				satoshis: 1,
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
				is_verified: true,
			};
			let utxo_2 = old_storage::LockedBitcoin {
				obligation_id: 2,
				vault_id: 2,
				lock_price: 2,
				owner_account: 2,
				satoshis: 2,
				vault_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_claim_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_xpub_sources: Default::default(),
				owner_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_claim_height: 2,
				open_claim_height: 2,
				created_at_height: 2,
				utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
					wscript_hash: H256::from([0u8; 32]),
				},
				is_verified: true,
			};
			let utxo_3 = old_storage::LockedBitcoin {
				obligation_id: 3,
				vault_id: 3,
				lock_price: 3,
				owner_account: 3,
				satoshis: 3,
				vault_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_claim_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_xpub_sources: Default::default(),
				owner_pubkey: CompressedBitcoinPubkey([1u8; 33]),
				vault_claim_height: 3,
				open_claim_height: 3,
				created_at_height: 3,
				utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
					wscript_hash: H256::from([0u8; 32]),
				},
				is_verified: true,
			};
			old_storage::LocksByUtxoId::<Test>::insert(1, utxo_1);
			old_storage::LocksByUtxoId::<Test>::insert(2, utxo_2);
			old_storage::OwedUtxoAggrieved::<Test>::insert(1, (1, 1, 1, utxo_3));

			// Get the pre_upgrade bytes
			let bytes = match InnerMigrate::<Test>::pre_upgrade() {
				Ok(bytes) => bytes,
				Err(e) => panic!("pre_upgrade failed: {:?}", e),
			};
			// Execute the migration
			let weight = InnerMigrate::<Test>::on_runtime_upgrade();

			// Verify post_upgrade succeeds
			assert_ok!(InnerMigrate::<Test>::post_upgrade(bytes));

			// The weight used should be 1 read for the old value, and 1 write for the new
			// value.
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 3));

			// check locks
			assert_eq!(LocksByUtxoId::<Test>::get(1).unwrap().obligation_id, 1);
			assert_eq!(LocksByUtxoId::<Test>::get(2).unwrap().obligation_id, 2)
		});
	}
}
