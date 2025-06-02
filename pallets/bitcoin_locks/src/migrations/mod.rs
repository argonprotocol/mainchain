use crate::{
	Config, LockReleaseRequest,
	pallet::{LockExpirationsByBitcoinHeight, LocksByUtxoId, LocksPendingReleaseByUtxoId},
};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use super::*;
	use crate::Config;
	use argon_primitives::{
		VaultId,
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinScriptPubkey, CompressedBitcoinPubkey,
			Satoshis, UtxoId, XPubChildNumber, XPubFingerprint,
		},
	};
	use frame_support::storage_alias;
	pub type ObligationId = u64;

	#[derive(Decode, Encode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct LockedBitcoin<T: Config> {
		#[codec(compact)]
		pub obligation_id: ObligationId,
		#[codec(compact)]
		pub vault_id: VaultId,
		pub lock_price: <T as Config>::Balance,
		pub owner_account: <T as frame_system::Config>::AccountId,
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
		pub is_rejected_needs_release: bool,
	}

	impl<T: Config> From<LockedBitcoin<T>> for crate::LockedBitcoin<T> {
		fn from(val: LockedBitcoin<T>) -> Self {
			crate::LockedBitcoin {
				vault_id: val.vault_id,
				lock_price: val.lock_price,
				owner_account: val.owner_account,
				satoshis: val.satoshis,
				vault_pubkey: val.vault_pubkey,
				vault_claim_pubkey: val.vault_claim_pubkey,
				vault_xpub_sources: val.vault_xpub_sources,
				owner_pubkey: val.owner_pubkey,
				vault_claim_height: val.vault_claim_height,
				open_claim_height: val.open_claim_height,
				created_at_height: val.created_at_height,
				utxo_script_pubkey: val.utxo_script_pubkey,
				is_verified: val.is_verified,
				is_rejected_needs_release: val.is_rejected_needs_release,
				fund_hold_extensions: Default::default(),
			}
		}
	}

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub locked_utxos: Vec<(UtxoId, LockedBitcoin<T>)>,
		pub locks_pending_release: Vec<(UtxoId, LockReleaseRequest<T::Balance>)>,
	}

	pub type LocksPendingRelease<T> = BoundedBTreeMap<
		UtxoId,
		LockReleaseRequest<<T as Config>::Balance>,
		<T as Config>::MaxConcurrentlyReleasingLocks,
	>;
	#[derive(Decode, Encode, CloneNoBound, PartialEqNoBound, EqNoBound, RuntimeDebug, TypeInfo)]
	pub struct LockReleaseRequest<Balance: Clone + Eq + PartialEq + TypeInfo + Codec> {
		#[codec(compact)]
		pub utxo_id: UtxoId,
		#[codec(compact)]
		pub obligation_id: ObligationId,
		#[codec(compact)]
		pub vault_id: VaultId,
		#[codec(compact)]
		pub bitcoin_network_fee: Satoshis,
		#[codec(compact)]
		pub cosign_due_block: BitcoinHeight,
		pub to_script_pubkey: BitcoinScriptPubkey,
		#[codec(compact)]
		pub redemption_price: Balance,
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
	pub(super) type LocksPendingReleaseByUtxoId<T: Config> =
		StorageValue<crate::Pallet<T>, LocksPendingRelease<T>, ValueQuery>;
	#[storage_alias]
	pub(super) type LocksByUtxoId<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, UtxoId, LockedBitcoin<T>, OptionQuery>;

	#[storage_alias]
	pub(super) type ObligationIdToUtxoId<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, ObligationId, UtxoId, OptionQuery>;
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let locked_utxos = old_storage::LocksByUtxoId::<T>::iter().collect::<Vec<_>>();
		let locks_pending_release = old_storage::LocksPendingReleaseByUtxoId::<T>::get()
			.into_iter()
			.collect::<Vec<_>>();

		Ok(<old_storage::Model<T>>::encode(&old_storage::Model {
			locked_utxos,
			locks_pending_release,
		}))
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating Bitcoin locks");

		let ob_ids = old_storage::ObligationIdToUtxoId::<T>::drain().collect::<Vec<_>>();
		count += ob_ids.len() as u64;

		let drained_owed = old_storage::OwedUtxoAggrieved::<T>::drain().collect::<Vec<_>>();
		count += drained_owed.len() as u64;

		LocksByUtxoId::<T>::translate::<old_storage::LockedBitcoin<T>, _>(|utxo_id, old| {
			count += 1;
			LockExpirationsByBitcoinHeight::<T>::mutate(old.vault_claim_height, |v| {
				let _ = v.try_insert(utxo_id);
			});
			Some(old.into())
		});

		LocksPendingReleaseByUtxoId::<T>::translate::<old_storage::LocksPendingRelease<T>, _>(
			|a| {
				if let Some(a) = a {
					let mut new_map = BoundedBTreeMap::new();
					for (utxo_id, old) in a {
						let _ = new_map.try_insert(
							utxo_id,
							LockReleaseRequest {
								utxo_id,
								vault_id: old.vault_id,
								bitcoin_network_fee: old.bitcoin_network_fee,
								cosign_due_block: old.cosign_due_block,
								to_script_pubkey: old.to_script_pubkey,
								redemption_price: old.redemption_price,
							},
						);
					}
					Some(new_map)
				} else {
					None
				}
			},
		)
		.expect("Failed to migrate LocksPendingReleaseByUtxoId");
		count += 1;

		T::DbWeight::get().reads_writes(count, count)
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

		let new = LocksPendingReleaseByUtxoId::<T>::get();
		ensure!(
			old.locks_pending_release.len() == new.len(),
			"locks_pending_release length mismatch"
		);

		Ok(())
	}
}

pub type RatchetMigration<T> = frame_support::migrations::VersionedMigration<
	1,
	2,
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{Test, new_test_ext};
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
				is_rejected_needs_release: false,
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
				is_rejected_needs_release: false,
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
				is_rejected_needs_release: false,
			};
			old_storage::LocksByUtxoId::<Test>::insert(1, utxo_1.clone());
			old_storage::LocksByUtxoId::<Test>::insert(2, utxo_2.clone());
			old_storage::LocksByUtxoId::<Test>::insert(3, utxo_2.clone());
			old_storage::OwedUtxoAggrieved::<Test>::insert(1, (1, 1, 1, utxo_3.clone()));
			old_storage::ObligationIdToUtxoId::<Test>::insert(1, 1);
			old_storage::ObligationIdToUtxoId::<Test>::insert(2, 2);
			old_storage::ObligationIdToUtxoId::<Test>::insert(3, 3);

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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(8, 8));

			// check locks
			let mut locks = LocksByUtxoId::<Test>::iter().collect::<Vec<_>>();
			locks.sort_by(|a, b| a.0.cmp(&b.0));
			assert_eq!(locks.len(), 3);
			assert_eq!(locks[0].1.lock_price, 1);
			assert_eq!(locks[1].1.lock_price, 2);
			assert_eq!(
				LockExpirationsByBitcoinHeight::<Test>::get(utxo_1.vault_claim_height)
					.into_iter()
					.collect::<Vec<_>>(),
				vec![1]
			);
			assert_eq!(
				LockExpirationsByBitcoinHeight::<Test>::get(utxo_2.vault_claim_height)
					.into_iter()
					.collect::<Vec<_>>(),
				vec![2, 3]
			);

			assert_eq!(
				old_storage::ObligationIdToUtxoId::<Test>::iter_keys().collect::<Vec<_>>().len(),
				0
			);
		});
	}
}
