use crate::{
	Config, LockCosignDueByFrame, LockReleaseRequest, LockReleaseRequestsByUtxoId,
	pallet::LocksByUtxoId,
};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;

mod old_storage {
	use crate::{Config, Pallet};
	use argon_primitives::{
		VaultId,
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinScriptPubkey, CompressedBitcoinPubkey,
			Satoshis, UtxoId, XPubChildNumber, XPubFingerprint,
		},
	};
	use frame_support::storage_alias;
	use pallet_prelude::*;

	#[derive(Decode, Encode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct LockedBitcoin<T: Config> {
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
		pub is_rejected_needs_release: bool,
		pub fund_hold_extensions: BoundedBTreeMap<BitcoinHeight, T::Balance, ConstU32<366>>,
	}

	#[derive(
		Decode,
		Encode,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct LockReleaseRequest<
		Balance: Clone + Eq + PartialEq + TypeInfo + Codec + MaxEncodedLen,
	> {
		#[codec(compact)]
		pub utxo_id: UtxoId,
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
				fund_hold_extensions: val.fund_hold_extensions,
				security_fees: Default::default(),
			}
		}
	}

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub locked_utxos: Vec<(UtxoId, LockedBitcoin<T>)>,
		pub locks_pending_release: LocksPendingRelease<T>,
	}

	pub type LocksPendingRelease<T> = BoundedBTreeMap<
		UtxoId,
		LockReleaseRequest<<T as Config>::Balance>,
		<T as Config>::MaxConcurrentlyReleasingLocks,
	>;

	#[storage_alias]
	pub type LocksPendingReleaseByUtxoId<T: Config> =
		StorageValue<Pallet<T>, LocksPendingRelease<T>>;
	#[storage_alias]
	pub(super) type LocksByUtxoId<T: Config> =
		StorageMap<crate::Pallet<T>, Twox64Concat, UtxoId, LockedBitcoin<T>, OptionQuery>;
}

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let locked_utxos = old_storage::LocksByUtxoId::<T>::iter().collect::<Vec<_>>();
		let locks_pending_release =
			old_storage::LocksPendingReleaseByUtxoId::<T>::get().unwrap_or_default();

		Ok(<old_storage::Model<T>>::encode(&old_storage::Model {
			locked_utxos,
			locks_pending_release,
		}))
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut count = 0;
		log::info!("Migrating Bitcoin locks");

		LocksByUtxoId::<T>::translate::<old_storage::LockedBitcoin<T>, _>(|_utxo_id, old| {
			count += 1;
			Some(old.into())
		});

		let current_frame = T::CurrentFrameId::get();
		let locks_pending_release =
			old_storage::LocksPendingReleaseByUtxoId::<T>::take().unwrap_or_default();
		for (utxo_id, lock) in locks_pending_release {
			// just default to 10 frames out
			let cosign_due_frame = current_frame + T::LockReleaseCosignDeadlineFrames::get();
			LockCosignDueByFrame::<T>::mutate(cosign_due_frame, |a| {
				a.try_insert(utxo_id)
					.expect("Failed to push UTXO ID to LockCosignDueByBitcoinHeight");
			});

			LockReleaseRequestsByUtxoId::<T>::insert(
				utxo_id,
				LockReleaseRequest {
					utxo_id,
					vault_id: lock.vault_id,
					bitcoin_network_fee: lock.bitcoin_network_fee,
					cosign_due_frame,
					to_script_pubkey: lock.to_script_pubkey,
					redemption_price: lock.redemption_price,
				},
			);
			count += 2;
		}

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

		for (utxo_id, old_lock) in old.locks_pending_release {
			let lock =
				LockReleaseRequestsByUtxoId::<T>::get(utxo_id).expect("Should be able to get");
			assert_eq!(lock.bitcoin_network_fee, old_lock.bitcoin_network_fee);
			let should_be_in_expirations =
				LockCosignDueByFrame::<T>::get(&lock.cosign_due_frame).contains(&utxo_id);
			assert!(
				should_be_in_expirations,
				"UTXO ID {:?} should be in LockCosignDueByBitcoinHeight",
				utxo_id
			);
		}

		Ok(())
	}
}

pub type ReleaseRequestMigration<T> = frame_support::migrations::VersionedMigration<
	2,
	3,
	InnerMigrate<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
#[cfg(all(feature = "try-runtime", test))]
mod test {
	use self::InnerMigrate;
	use super::*;
	use crate::mock::{CurrentFrameId, LockReleaseCosignDeadlineFrames, Test, new_test_ext};
	use argon_primitives::bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinScriptPubkey, CompressedBitcoinPubkey,
	};
	use frame_support::assert_ok;
	use sp_core::H256;

	#[test]
	fn handles_existing_value() {
		new_test_ext().execute_with(|| {
			let utxo_1 = old_storage::LockedBitcoin {
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
				fund_hold_extensions: Default::default(),
			};
			old_storage::LocksByUtxoId::<Test>::insert(1, utxo_1.clone());
			old_storage::LocksPendingReleaseByUtxoId::<Test>::mutate(|a| {
				let a = a.get_or_insert_with(Default::default);
				let _ = a.try_insert(
					1,
					old_storage::LockReleaseRequest {
						cosign_due_block: 1500,
						vault_id: 1,
						redemption_price: 10,
						to_script_pubkey: BitcoinScriptPubkey(BoundedVec::truncate_from(vec![
							0u8;
							33
						])),
						bitcoin_network_fee: 1,
						utxo_id: 1,
					},
				);
				let _ = a.try_insert(
					2,
					old_storage::LockReleaseRequest {
						cosign_due_block: 1000,
						vault_id: 1,
						redemption_price: 20,
						to_script_pubkey: BitcoinScriptPubkey(BoundedVec::truncate_from(vec![
							0u8;
							33
						])),
						bitcoin_network_fee: 2,
						utxo_id: 2,
					},
				);
			});

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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(5, 5));

			// check locks
			let mut locks = LocksByUtxoId::<Test>::iter().collect::<Vec<_>>();
			locks.sort_by(|a, b| a.0.cmp(&b.0));
			assert_eq!(locks.len(), 1);
			assert_eq!(locks[0].1.lock_price, 1);

			let current_frame = CurrentFrameId::get();
			assert_eq!(
				LockReleaseRequestsByUtxoId::<Test>::get(&1).unwrap().cosign_due_frame,
				current_frame + LockReleaseCosignDeadlineFrames::get()
			);
			assert_eq!(
				LockReleaseRequestsByUtxoId::<Test>::get(&2).unwrap().cosign_due_frame,
				current_frame + LockReleaseCosignDeadlineFrames::get()
			);
			assert_eq!(
				LockCosignDueByFrame::<Test>::get(
					LockReleaseCosignDeadlineFrames::get() + CurrentFrameId::get()
				)
				.into_iter()
				.collect::<Vec<_>>(),
				vec![1, 2]
			);
		});
	}
}
