use crate::{Config, LocksByUtxoId};
use frame_support::{traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use pallet_prelude::*;

mod old_storage {
	use crate::Config;
	use argon_primitives::{
		VaultId,
		bitcoin::{
			BitcoinCosignScriptPubkey, BitcoinHeight, CompressedBitcoinPubkey, Satoshis, UtxoId,
			XPubChildNumber, XPubFingerprint,
		},
	};
	use frame_support::storage_alias;
	use pallet_prelude::*;

	#[derive(Decode, Encode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct LockedBitcoin<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		/// The mintable liquidity of this lock, in microgons
		pub liquidity_promised: T::Balance,
		/// The market rate of the satoshis locked, adjusted for any inflation offset of the argon
		pub locked_market_rate: T::Balance,
		/// The owner account
		pub owner_account: T::AccountId,
		/// Sum of all lock fees (initial plus any ratcheting)
		pub security_fees: T::Balance,
		/// Fees paid using coupons for this lock
		pub coupon_paid_fees: T::Balance,
		/// The number of satoshis reserved for this lock
		#[codec(compact)]
		pub satoshis: Satoshis,
		/// The number of satoshis in the funding utxo (allowed some variance from the `satoshis`
		/// field)
		pub utxo_satoshis: Option<Satoshis>,
		/// The vault pubkey used in the cosign script to lock (and unlock) the bitcoin
		pub vault_pubkey: CompressedBitcoinPubkey,
		/// The vault pubkey used to claim the bitcoin after the lock expiration
		pub vault_claim_pubkey: CompressedBitcoinPubkey,
		/// The vault xpub sources. First is the cosign number, second is the claim number
		pub vault_xpub_sources: (XPubFingerprint, XPubChildNumber, XPubChildNumber),
		/// The bitcoin pubkey provided by the owner of the bitcoin lock that will be needed to
		/// spend the bitcoin (owner side of cosign)
		pub owner_pubkey: CompressedBitcoinPubkey,
		/// The height where the vault has exclusive rights to claim the bitcoin
		#[codec(compact)]
		pub vault_claim_height: BitcoinHeight,
		/// The height where either owner or vault can claim the bitcoin
		#[codec(compact)]
		pub open_claim_height: BitcoinHeight,
		/// The bitcoin height when this lock was created
		#[codec(compact)]
		pub created_at_height: BitcoinHeight,
		/// The script pubkey where funds are sent to fund this bitcoin lock
		pub utxo_script_pubkey: BitcoinCosignScriptPubkey,
		/// Whether this lock has been verified on-bitcoin
		pub is_verified: bool,
		/// Funds used by this bitcoin that will need to be held for extended periods when released
		/// back to the vault
		pub fund_hold_extensions: BoundedBTreeMap<BitcoinHeight, T::Balance, ConstU32<366>>,
		/// The argon block when this lock was created
		pub created_at_argon_block: BlockNumberFor<T>,
	}

	#[derive(codec::Encode, codec::Decode)]
	pub struct Model<T: Config> {
		pub locked_utxos: Vec<(UtxoId, LockedBitcoin<T>)>,
	}

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

		Ok(<old_storage::Model<T>>::encode(&old_storage::Model { locked_utxos }))
	}

	fn on_runtime_upgrade() -> Weight {
		let mut count = 0u64;
		log::info!("Migrating Bitcoin locks");

		LocksByUtxoId::<T>::translate::<old_storage::LockedBitcoin<T>, _>(|_utxo_id, old| {
			count += 1;
			Some(crate::LockedBitcoin {
				vault_id: old.vault_id,
				locked_market_rate: old.locked_market_rate,
				liquidity_promised: old.liquidity_promised,
				coupon_paid_fees: old.coupon_paid_fees,
				securitization_ratio: FixedU128::from_u32(1),
				utxo_satoshis: old.utxo_satoshis,
				created_at_argon_block: old.created_at_argon_block,
				owner_account: old.owner_account,
				satoshis: old.satoshis,
				vault_pubkey: old.vault_pubkey,
				vault_claim_pubkey: old.vault_claim_pubkey,
				vault_xpub_sources: old.vault_xpub_sources,
				owner_pubkey: old.owner_pubkey,
				vault_claim_height: old.vault_claim_height,
				open_claim_height: old.open_claim_height,
				created_at_height: old.created_at_height,
				utxo_script_pubkey: old.utxo_script_pubkey,
				is_funded: old.is_verified,
				fund_hold_extensions: old.fund_hold_extensions,
				security_fees: old.security_fees,
			})
		});

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

		Ok(())
	}
}

pub type SecuritizationMigration<T> = frame_support::migrations::VersionedMigration<
	5,
	6,
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
				vault_id: 1,
				liquidity_promised: 1,
				locked_market_rate: 1,
				owner_account: 1,
				satoshis: 1,
				utxo_satoshis: Some(1),
				created_at_argon_block: 1,
				coupon_paid_fees: 0u128,
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
				fund_hold_extensions: Default::default(),
				security_fees: 10u128,
			};
			old_storage::LocksByUtxoId::<Test>::insert(1, utxo_1.clone());

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
			assert_eq!(weight, <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1));

			// check locks
			let mut locks = LocksByUtxoId::<Test>::iter().collect::<Vec<_>>();
			locks.sort_by(|a, b| a.0.cmp(&b.0));
			assert_eq!(locks.len(), 1);
			assert_eq!(locks[0].1.locked_market_rate, 1);
			assert_eq!(locks[0].1.liquidity_promised, 1);
			assert_eq!(locks[0].1.securitization_ratio, FixedU128::one());
		});
	}
}
