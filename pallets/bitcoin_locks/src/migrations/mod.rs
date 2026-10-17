use crate::{Config, LocksById, Pallet};
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinLockId, CompressedBitcoinPubkey, Satoshis,
		UtxoRef, XPubChildNumber, XPubFingerprint,
	},
	tick::Tick,
	vault::BitcoinSecuritizationBasis,
	VaultId,
};
use codec::{Decode, Encode};
use frame_support::{
	migrations::VersionedMigration, storage_alias, traits::UncheckedOnRuntimeUpgrade,
};
use pallet_prelude::*;
use sp_runtime::BoundedBTreeMap;

// Mainnet's 146 Locks were all 1x at finalized block 957572 (runtime 159). At 1x the
// stored guarantee already equals required backing, so funding, maturities, Fissions,
// and upstream positions are unchanged. Preflight verifies this assumption again.
/// Rename the stored guarantee and remove the unused ratio, preserving funding and maturities.
pub struct RemoveSecuritizationRatio<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for RemoveSecuritizationRatio<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut count = 0u64;
		LocksById::<T>::translate::<v11::LockedBitcoin<T>, _>(|_, lock| {
			count += 1;
			Some(lock.into_current())
		});
		T::DbWeight::get().reads_writes(count + 1, count)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let mut locks = Vec::new();
		for (id, lock) in v11::LocksById::<T>::iter() {
			ensure!(
				lock.securitization_ratio == FixedU128::one(),
				"non-1x Bitcoin Lock requires conversion before runtime 161"
			);
			locks.push((id, lock.into_current().encode()));
		}
		Ok(locks.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let locks = Vec::<(BitcoinLockId, Vec<u8>)>::decode(&mut state.as_slice())
			.map_err(|_| "invalid Bitcoin Lock migration state")?;
		ensure!(LocksById::<T>::iter_keys().count() == locks.len(), "Bitcoin Lock count changed");
		for (id, encoded) in locks {
			ensure!(
				LocksById::<T>::get(id).map(|lock| lock.encode()) == Some(encoded),
				"Bitcoin Lock migration changed stored terms or funding"
			);
		}
		Ok(())
	}
}

pub type RemoveSecuritizationRatioMigration<T> = VersionedMigration<
	11,
	12,
	RemoveSecuritizationRatio<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

/// Runtime 160 storage; the funding-output limit is unchanged in runtime 161.
pub mod v11 {
	use super::*;

	#[derive(Encode, Decode, CloneNoBound)]
	pub struct LockedBitcoin<T: Config> {
		#[codec(compact)]
		pub vault_id: VaultId,
		/// Bitcoin amount and valuation used to price this Lock's securitization.
		pub securitization_basis: BitcoinSecuritizationBasis<T::Balance>,
		/// Microgon coverage purchased after applying the redemption curve.
		#[codec(compact)]
		pub securitization_coverage_microgons: T::Balance,
		/// Tick of the price-history entry used for the current securitization.
		#[codec(compact)]
		pub securitization_tick: Tick,
		/// Satoshis attached to this Lock by confirmed funding outputs. Zero means unfunded.
		#[codec(compact)]
		pub funded_satoshis: Satoshis,
		/// Confirmed Bitcoin outputs assigned to this Lock and their individual amounts.
		pub funding_utxos: BoundedBTreeMap<UtxoRef, Satoshis, T::MaxUtxosPerLock>,
		/// Funded satoshis currently split into active Fissions.
		#[codec(compact)]
		pub fissioned_satoshis: Satoshis,
		/// The owner account
		pub owner_account: T::AccountId,
		/// The guaranteed securitization ratio for this lock
		pub securitization_ratio: FixedU128,
		/// Sum of all lock fees (initial plus any resecuritization)
		#[codec(compact)]
		pub security_fees: T::Balance,
		/// Fees covered by the vault for this lock
		#[codec(compact)]
		pub coupon_paid_fees: T::Balance,
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
		/// The Bitcoin height through which unused securitization remains reserved.
		#[codec(compact)]
		pub securitization_hold_expiration_bitcoin_height: BitcoinHeight,
		/// The script pubkey where funds are sent to fund this bitcoin lock
		pub utxo_script_pubkey: BitcoinCosignScriptPubkey,
		/// Whether this operator-owned lock may be displaced by outside vault capital.
		pub is_flexible: bool,
		/// Funds used by this bitcoin that will need to be held for extended periods when released
		/// back to the vault
		pub fund_hold_extensions: BoundedBTreeMap<BitcoinHeight, T::Balance, ConstU32<366>>,
		/// The argon block when this lock was created
		#[codec(compact)]
		pub created_at_argon_block: BlockNumberFor<T>,
	}

	#[storage_alias]
	pub type LocksById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, BitcoinLockId, LockedBitcoin<T>, OptionQuery>;

	impl<T: Config> LockedBitcoin<T> {
		pub fn into_current(self) -> crate::LockedBitcoin<T> {
			crate::LockedBitcoin {
				vault_id: self.vault_id,
				securitization_basis: self.securitization_basis,
				securitization_microgons: self.securitization_coverage_microgons,
				securitization_tick: self.securitization_tick,
				funded_satoshis: self.funded_satoshis,
				funding_utxos: self.funding_utxos,
				fissioned_satoshis: self.fissioned_satoshis,
				owner_account: self.owner_account,
				security_fees: self.security_fees,
				coupon_paid_fees: self.coupon_paid_fees,
				vault_pubkey: self.vault_pubkey,
				vault_claim_pubkey: self.vault_claim_pubkey,
				vault_xpub_sources: self.vault_xpub_sources,
				owner_pubkey: self.owner_pubkey,
				vault_claim_height: self.vault_claim_height,
				open_claim_height: self.open_claim_height,
				created_at_height: self.created_at_height,
				securitization_hold_expiration_bitcoin_height: self
					.securitization_hold_expiration_bitcoin_height,
				utxo_script_pubkey: self.utxo_script_pubkey,
				is_flexible: self.is_flexible,
				fund_hold_extensions: self.fund_hold_extensions,
				created_at_argon_block: self.created_at_argon_block,
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use argon_primitives::bitcoin::H256Le;
	#[cfg(feature = "try-runtime")]
	use frame_support::assert_ok;
	use frame_support::traits::OnRuntimeUpgrade;
	use sp_runtime::bounded_btree_map;

	fn stored_lock() -> v11::LockedBitcoin<Test> {
		v11::LockedBitcoin {
			vault_id: 1,
			securitization_basis: BitcoinSecuritizationBasis {
				satoshis: 10,
				microgons_at_target_per_btc: 123_000_000,
			},
			securitization_coverage_microgons: 11,
			securitization_tick: 7,
			funded_satoshis: 8,
			funding_utxos: bounded_btree_map! { UtxoRef { txid: H256Le([4; 32]), output_index: 2 } => 8 },
			fissioned_satoshis: 6,
			owner_account: 2,
			securitization_ratio: FixedU128::one(),
			security_fees: 17,
			coupon_paid_fees: 5,
			vault_pubkey: CompressedBitcoinPubkey([2; 33]),
			vault_claim_pubkey: CompressedBitcoinPubkey([3; 33]),
			vault_xpub_sources: ([6; 4], 2, 3),
			owner_pubkey: CompressedBitcoinPubkey([4; 33]),
			vault_claim_height: 100,
			open_claim_height: 1_000,
			created_at_height: 1,
			securitization_hold_expiration_bitcoin_height: 70,
			utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
				wscript_hash: H256::repeat_byte(8),
			},
			is_flexible: true,
			fund_hold_extensions: bounded_btree_map! { 432 => 7, 576 => 4 },
			created_at_argon_block: 9,
		}
	}

	#[test]
	fn migration_preserves_partial_and_overfunded_locks_and_runs_once() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(11).put::<Pallet<Test>>();
			let partial = stored_lock();
			let mut overfunded = partial.clone();
			overfunded.funded_satoshis = 15;
			overfunded
				.funding_utxos
				.try_insert(UtxoRef { txid: H256Le([5; 32]), output_index: 0 }, 7)
				.unwrap();
			v11::LocksById::<Test>::insert(1, &partial);
			v11::LocksById::<Test>::insert(2, &overfunded);
			crate::LockIdsByVaultId::<Test>::insert(1, 1, ());
			crate::LockIdsByVaultId::<Test>::insert(1, 2, ());
			let index = crate::LockIdsByVaultId::<Test>::iter().collect::<Vec<_>>();
			let position = pallet_treasury_positions::PositionsByAccount::<Test>::get(2).encode();
			#[cfg(feature = "try-runtime")]
			let state = RemoveSecuritizationRatio::<Test>::pre_upgrade().unwrap();

			RemoveSecuritizationRatioMigration::<Test>::on_runtime_upgrade();

			#[cfg(feature = "try-runtime")]
			assert_ok!(RemoveSecuritizationRatio::<Test>::post_upgrade(state));
			for (id, original) in [(1, partial), (2, overfunded)] {
				let lock = LocksById::<Test>::get(id).unwrap();
				assert_eq!(lock.securitization_microgons, 11);
				assert_eq!(lock.securitization_basis, original.securitization_basis);
				assert_eq!(lock.securitization_tick, 7);
				assert_eq!(lock.funded_satoshis, original.funded_satoshis);
				assert_eq!(lock.funding_utxos, original.funding_utxos);
				assert_eq!(lock.fissioned_satoshis, 6);
				assert_eq!((lock.security_fees, lock.coupon_paid_fees), (17, 5));
				assert_eq!(
					(lock.vault_claim_height, lock.open_claim_height, lock.created_at_height),
					(100, 1_000, 1)
				);
				assert_eq!(lock.securitization_hold_expiration_bitcoin_height, 70);
				assert_eq!(lock.fund_hold_extensions, original.fund_hold_extensions);
				assert_eq!(lock.utxo_script_pubkey, original.utxo_script_pubkey);
				assert_eq!(lock.vault_pubkey, original.vault_pubkey);
				assert_eq!(lock.vault_claim_pubkey, original.vault_claim_pubkey);
				assert_eq!(lock.owner_pubkey, original.owner_pubkey);
			}
			assert_eq!(crate::LockIdsByVaultId::<Test>::iter().collect::<Vec<_>>(), index);
			assert_eq!(
				pallet_treasury_positions::PositionsByAccount::<Test>::get(2).encode(),
				position
			);
			assert_eq!(StorageVersion::get::<Pallet<Test>>(), StorageVersion::new(12));
			let locks = LocksById::<Test>::iter()
				.map(|(id, lock)| (id, lock.encode()))
				.collect::<Vec<_>>();
			RemoveSecuritizationRatioMigration::<Test>::on_runtime_upgrade();
			assert_eq!(
				LocksById::<Test>::iter()
					.map(|(id, lock)| (id, lock.encode()))
					.collect::<Vec<_>>(),
				locks
			);
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn pre_upgrade_rejects_non_one_ratios_without_writing_storage() {
		new_test_ext().execute_with(|| {
			let mut lock = stored_lock();
			lock.securitization_ratio = FixedU128::from_rational(3, 2);
			v11::LocksById::<Test>::insert(1, &lock);
			assert!(RemoveSecuritizationRatio::<Test>::pre_upgrade().is_err());
			assert_eq!(v11::LocksById::<Test>::get(1).unwrap().encode(), lock.encode());
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn post_upgrade_detects_changed_terms_and_missing_locks() {
		new_test_ext().execute_with(|| {
			v11::LocksById::<Test>::insert(1, stored_lock());
			let state = RemoveSecuritizationRatio::<Test>::pre_upgrade().unwrap();
			RemoveSecuritizationRatio::<Test>::on_runtime_upgrade();
			LocksById::<Test>::mutate(1, |lock| lock.as_mut().unwrap().security_fees += 1);
			assert!(RemoveSecuritizationRatio::<Test>::post_upgrade(state.clone()).is_err());
			LocksById::<Test>::remove(1);
			assert!(RemoveSecuritizationRatio::<Test>::post_upgrade(state).is_err());
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn migration_handles_empty_storage() {
		new_test_ext().execute_with(|| {
			let state = RemoveSecuritizationRatio::<Test>::pre_upgrade().unwrap();
			RemoveSecuritizationRatio::<Test>::on_runtime_upgrade();
			assert_ok!(RemoveSecuritizationRatio::<Test>::post_upgrade(state));
		});
	}
}
