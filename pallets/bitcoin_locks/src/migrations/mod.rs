use crate::{
	Config, LastProcessedSecuritizationHoldBitcoinHeight, LockCosignDueByFrame,
	LockReleaseCosignHeight, LockReleaseCosignHeightById, LockedBitcoin, LocksById,
	MicrogonsAtTargetPerBtcHistory, NextBitcoinLockId, Pallet,
	SecuritizationHoldExpirationsByBitcoinHeight,
};
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinLockId, BitcoinScriptPubkey,
		CompressedBitcoinPubkey, Satoshis, UtxoRef, XPubChildNumber, XPubFingerprint,
		SATOSHIS_PER_BITCOIN,
	},
	prelude::FrameId,
	vault::{BitcoinSecuritizationBasis, BitcoinVaultProvider},
	VaultId,
};
use codec::{Decode, Encode};
use frame_support::{
	storage::{migration::move_prefix, storage_prefix},
	storage_alias,
	traits::UncheckedOnRuntimeUpgrade,
};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

#[derive(Decode, Encode)]
struct LockedBitcoinV10<T: Config> {
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
	is_flexible: bool,
	fund_hold_extensions: BoundedBTreeMap<BitcoinHeight, T::Balance, ConstU32<366>>,
	#[codec(compact)]
	created_at_argon_block: BlockNumberFor<T>,
}

#[derive(Decode, Encode)]
struct LockReleaseRequestV10<Balance: Codec> {
	#[codec(compact)]
	lock_id: BitcoinLockId,
	#[codec(compact)]
	vault_id: VaultId,
	#[codec(compact)]
	bitcoin_network_fee: Satoshis,
	#[codec(compact)]
	cosign_due_frame: FrameId,
	to_script_pubkey: BitcoinScriptPubkey,
	#[codec(compact)]
	securitization_at_risk: Balance,
}

mod v10 {
	use super::*;

	#[storage_alias]
	pub(super) type NextUtxoId<T: Config> = StorageValue<Pallet<T>, BitcoinLockId, OptionQuery>;

	#[storage_alias]
	pub(super) type LocksByUtxoId<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, BitcoinLockId, LockedBitcoinV10<T>, OptionQuery>;

	#[storage_alias]
	pub(super) type UtxoIdToFundingUtxoRef<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, BitcoinLockId, UtxoRef, OptionQuery>;

	#[storage_alias]
	pub(super) type LockReleaseRequestsByUtxoId<T: Config> = StorageMap<
		Pallet<T>,
		Twox64Concat,
		BitcoinLockId,
		LockReleaseRequestV10<<T as Config>::Balance>,
		OptionQuery,
	>;

	#[storage_alias]
	pub(super) type LockReleaseCosignHeightById<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, BitcoinLockId, BlockNumberFor<T>, OptionQuery>;
}

/// Convert version 10 Locks and their existing funded positions, move funding references from the
/// observer pallet, schedule expiration of their unused securitization, and cancel pending release
/// requests whose transaction format is no longer valid.
pub struct MigrateLockModel<T>(core::marker::PhantomData<T>);

impl<T> UncheckedOnRuntimeUpgrade for MigrateLockModel<T>
where
	T: Config + pallet_bitcoin_fissions::Config<Balance = <T as Config>::Balance>,
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let locks = v10::LocksByUtxoId::<T>::iter()
			.map(|(lock_id, lock)| {
				(
					lock_id,
					lock.satoshis,
					lock.locked_target_price,
					lock.is_funded,
					lock.utxo_satoshis,
					lock.liquidity_promised,
					lock.owner_account,
					lock.created_at_argon_block,
				)
			})
			.collect::<Vec<_>>();
		let release_request_count =
			v10::LockReleaseRequestsByUtxoId::<T>::iter_keys().count() as u64;
		let release_cosign_count =
			v10::LockReleaseCosignHeightById::<T>::iter_keys().count() as u64;
		Ok((locks, release_request_count, release_cosign_count).encode())
	}

	fn on_runtime_upgrade() -> Weight {
		frame_support::storage::migration::move_storage_from_pallet(
			b"UtxoIdToFundingUtxoRef",
			b"BitcoinUtxos",
			b"BitcoinLocks",
		);
		StorageVersion::new(1).put::<pallet_bitcoin_fissions::Pallet<T>>();

		let mut reads = 3u64;
		let mut writes = 2u64;
		let mut fission_migration_weight = Weight::zero();
		let current_bitcoin_height = T::BitcoinBlockHeightChange::get().1;
		LastProcessedSecuritizationHoldBitcoinHeight::<T>::put(current_bitcoin_height);
		let current_tick = T::CurrentTick::get();
		let microgons_at_target_per_btc_history = MicrogonsAtTargetPerBtcHistory::<T>::get();

		reads = reads.saturating_add(1);
		if let Some(next_lock_id) = v10::NextUtxoId::<T>::take() {
			NextBitcoinLockId::<T>::put(next_lock_id);
			writes = writes.saturating_add(2);
		}

		let pallet_prefix = b"BitcoinLocks";
		move_prefix(
			&storage_prefix(pallet_prefix, b"LocksByUtxoId"),
			&storage_prefix(pallet_prefix, b"LocksById"),
		);
		move_prefix(
			&storage_prefix(pallet_prefix, b"UtxoIdsByVaultId"),
			&storage_prefix(pallet_prefix, b"LockIdsByVaultId"),
		);
		move_prefix(
			&storage_prefix(pallet_prefix, b"UtxoIdsByOwnerAccount"),
			&storage_prefix(pallet_prefix, b"LockIdsByOwnerAccount"),
		);

		let mut cancelled_release_requests = 0u64;
		for (lock_id, request) in v10::LockReleaseRequestsByUtxoId::<T>::drain() {
			cancelled_release_requests = cancelled_release_requests.saturating_add(1);
			LockCosignDueByFrame::<T>::mutate(request.cosign_due_frame, |locks| {
				locks.remove(&lock_id);
			});
			if let Err(error) =
				T::VaultProvider::update_pending_cosign_list(request.vault_id, lock_id, true)
			{
				log::error!(
					"Unable to clear legacy Bitcoin release {lock_id:?} from vault {:?}: {error:?}",
					request.vault_id,
				);
			}
		}
		reads = reads.saturating_add(cancelled_release_requests.saturating_mul(3));
		writes = writes.saturating_add(cancelled_release_requests.saturating_mul(3));

		let mut migrated_release_cosigns = 0u64;
		LockReleaseCosignHeightById::<T>::translate::<BlockNumberFor<T>, _>(
			|_lock_id, cosign_height| {
				migrated_release_cosigns = migrated_release_cosigns.saturating_add(1);
				Some(LockReleaseCosignHeight {
					cosign_height,
					previous_cosign_height: None,
					release_number: 1,
				})
			},
		);
		reads = reads.saturating_add(migrated_release_cosigns);
		writes = writes.saturating_add(migrated_release_cosigns);

		let mut migrated_locks = 0u64;
		LocksById::<T>::translate::<LockedBitcoinV10<T>, _>(|lock_id, lock| {
			migrated_locks = migrated_locks.saturating_add(1);
			reads = reads.saturating_add(3);
			writes = writes.saturating_add(4);
			let funded_satoshis =
				if lock.is_funded { lock.utxo_satoshis.unwrap_or(lock.satoshis) } else { 0 };
			let mut funding_utxos = BoundedBTreeMap::new();
			if let Some(utxo_ref) = v10::UtxoIdToFundingUtxoRef::<T>::take(lock_id) {
				funding_utxos
					.try_insert(utxo_ref, funded_satoshis)
					.expect("MaxUtxosPerLock must permit one migrated funding UTXO");
			}
			let fissioned_satoshis = if lock.is_funded { lock.satoshis } else { 0 };
			let microgons_at_target_per_btc =
				FixedU128::from_rational(SATOSHIS_PER_BITCOIN as u128, lock.satoshis as u128)
					.saturating_mul_int(lock.locked_target_price);
			let securitization_tick = microgons_at_target_per_btc_history
				.iter()
				.rev()
				.find_map(|(tick, value)| (*value == microgons_at_target_per_btc).then_some(*tick))
				.unwrap_or(current_tick);
			let securitization_hold_expiration_bitcoin_height = lock
				.created_at_height
				.saturating_add(T::SecuritizationHoldBlocks::get())
				.saturating_add(1);
			let securitization_hold_expiration_bitcoin_height = if funded_satoshis >= lock.satoshis
			{
				securitization_hold_expiration_bitcoin_height
			} else {
				securitization_hold_expiration_bitcoin_height
					.max(current_bitcoin_height.saturating_add(1))
			};
			let securitization_basis =
				BitcoinSecuritizationBasis { satoshis: lock.satoshis, microgons_at_target_per_btc };
			if lock.is_funded {
				let fission_id = lock_id;
				let mut fission_ids = BoundedBTreeSet::new();
				fission_ids
					.try_insert(fission_id)
					.expect("MaxFissionsPerLock must permit one migrated Fission");
				pallet_bitcoin_fissions::FissionByOwnerAndId::<T>::insert(
					&lock.owner_account,
					fission_id,
					pallet_bitcoin_fissions::Fission {
						liquid_id: lock_id,
						lock_id,
						satoshis: lock.satoshis,
						microgons_at_target_per_btc,
						last_ratchet_tick: securitization_tick,
						liquidity_promised: lock.liquidity_promised,
						created_at_argon_block: lock.created_at_argon_block,
						ratchet_number: 0,
						last_updated_argon_block: lock.created_at_argon_block,
					},
				);
				pallet_bitcoin_fissions::FissionIdsByLockId::<T>::insert(lock_id, fission_ids);
				pallet_bitcoin_fissions::NextFissionIdByOwner::<T>::mutate(
					&lock.owner_account,
					|next_fission_id| {
						*next_fission_id = (*next_fission_id).max(fission_id.saturating_add(1));
					},
				);
				fission_migration_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 3));
			}

			Some(LockedBitcoin {
				vault_id: lock.vault_id,
				securitization_basis,
				securitization_coverage_microgons: lock.liquidity_promised,
				securitization_tick,
				funded_satoshis,
				funding_utxos,
				fissioned_satoshis,
				owner_account: lock.owner_account,
				securitization_ratio: lock.securitization_ratio,
				security_fees: lock.security_fees,
				coupon_paid_fees: lock.coupon_paid_fees,
				vault_pubkey: lock.vault_pubkey,
				vault_claim_pubkey: lock.vault_claim_pubkey,
				vault_xpub_sources: lock.vault_xpub_sources,
				owner_pubkey: lock.owner_pubkey,
				vault_claim_height: lock.vault_claim_height,
				open_claim_height: lock.open_claim_height,
				created_at_height: lock.created_at_height,
				securitization_hold_expiration_bitcoin_height,
				utxo_script_pubkey: lock.utxo_script_pubkey,
				is_flexible: lock.is_flexible,
				fund_hold_extensions: lock.fund_hold_extensions,
				created_at_argon_block: lock.created_at_argon_block,
			})
		});
		// Each Lock owns one vault index and one owner index, and at most one release request.
		reads = reads.saturating_add(migrated_locks.saturating_mul(3));
		writes = writes.saturating_add(migrated_locks.saturating_mul(6));

		for (lock_id, lock) in LocksById::<T>::iter() {
			reads = reads.saturating_add(1);
			if lock.funded_satoshis >= lock.securitization_basis.satoshis {
				continue;
			}
			let inserted = SecuritizationHoldExpirationsByBitcoinHeight::<T>::try_mutate(
				lock.securitization_hold_expiration_bitcoin_height,
				|locks| locks.try_insert(lock_id),
			);
			if inserted.is_ok() {
				writes = writes.saturating_add(1);
			} else {
				log::error!(
					"Unable to schedule unused securitization expiration for bitcoin lock {lock_id:?}"
				);
			}
		}

		T::DbWeight::get()
			.reads_writes(reads, writes)
			.saturating_add(fission_migration_weight)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		type LockState<T> = (
			BitcoinLockId,
			Satoshis,
			<T as Config>::Balance,
			bool,
			Option<Satoshis>,
			<T as Config>::Balance,
			<T as frame_system::Config>::AccountId,
			BlockNumberFor<T>,
		);
		let (locks, release_request_count, release_cosign_count): (Vec<LockState<T>>, u64, u64) =
			Decode::decode(&mut state.as_slice()).map_err(|_| {
				TryRuntimeError::Other("could not decode bitcoin lock migration state")
			})?;

		ensure!(
			LocksById::<T>::iter_keys().count() == locks.len(),
			TryRuntimeError::Other("bitcoin lock count changed during migration"),
		);
		ensure!(
			StorageVersion::get::<pallet_bitcoin_fissions::Pallet<T>>() == StorageVersion::new(1),
			TryRuntimeError::Other("bitcoin Fission storage version was not initialized"),
		);
		ensure!(
			v10::LockReleaseRequestsByUtxoId::<T>::iter_keys().next().is_none(),
			TryRuntimeError::Other("legacy Bitcoin release requests remain after migration"),
		);
		ensure!(
			LockReleaseCosignHeightById::<T>::iter_keys().count() == release_cosign_count as usize &&
				LockReleaseCosignHeightById::<T>::iter_values()
					.all(|cosign| cosign.release_number == 1),
			TryRuntimeError::Other("bitcoin release cosign tombstones were not migrated"),
		);
		log::info!("Cancelled {release_request_count} unsigned legacy Bitcoin release requests");
		for (
			lock_id,
			securitized_satoshis,
			locked_target_price,
			was_funded,
			funding_satoshis,
			liquidity_promised,
			owner_account,
			created_at_argon_block,
		) in locks
		{
			let lock = LocksById::<T>::get(lock_id)
				.ok_or(TryRuntimeError::Other("bitcoin lock was not migrated"))?;
			let expected_rate = FixedU128::from_rational(
				SATOSHIS_PER_BITCOIN as u128,
				securitized_satoshis as u128,
			)
			.saturating_mul_int(locked_target_price);
			let expected_funded_satoshis =
				if was_funded { funding_satoshis.unwrap_or(securitized_satoshis) } else { 0 };
			let expected_fissioned_satoshis = if was_funded { securitized_satoshis } else { 0 };
			ensure!(
				lock.securitization_basis.satoshis == securitized_satoshis &&
					lock.securitization_basis.microgons_at_target_per_btc == expected_rate &&
					lock.funded_satoshis == expected_funded_satoshis &&
					lock.fissioned_satoshis == expected_fissioned_satoshis &&
					lock.owner_account == owner_account &&
					lock.created_at_argon_block == created_at_argon_block,
				TryRuntimeError::Other("bitcoin lock accounting changed during migration"),
			);
			if was_funded {
				let fission =
					pallet_bitcoin_fissions::FissionByOwnerAndId::<T>::get(&owner_account, lock_id)
						.ok_or(TryRuntimeError::Other(
							"funded bitcoin lock Fission was not migrated",
						))?;
				ensure!(
					fission.liquid_id == lock_id &&
						fission.lock_id == lock_id &&
						fission.satoshis == securitized_satoshis &&
						fission.microgons_at_target_per_btc == expected_rate &&
						fission.last_ratchet_tick == lock.securitization_tick &&
						fission.liquidity_promised == liquidity_promised &&
						fission.created_at_argon_block == created_at_argon_block &&
						fission.ratchet_number == 0 &&
						fission.last_updated_argon_block == created_at_argon_block,
					TryRuntimeError::Other("funded bitcoin lock Fission accounting changed"),
				);
				ensure!(
					pallet_bitcoin_fissions::FissionIdsByLockId::<T>::get(lock_id)
						.contains(&lock_id) &&
						pallet_bitcoin_fissions::NextFissionIdByOwner::<T>::get(&owner_account) >
							lock_id,
					TryRuntimeError::Other("funded bitcoin lock Fission indexes were not migrated"),
				);
			} else {
				ensure!(
					!pallet_bitcoin_fissions::FissionByOwnerAndId::<T>::contains_key(
						&owner_account,
						lock_id,
					),
					TryRuntimeError::Other("unfunded bitcoin lock created a Fission"),
				);
				ensure!(
					SecuritizationHoldExpirationsByBitcoinHeight::<T>::get(
						lock.securitization_hold_expiration_bitcoin_height,
					)
					.contains(&lock_id),
					TryRuntimeError::Other(
						"unused bitcoin lock securitization was not scheduled for expiration"
					),
				);
			}
		}
		Ok(())
	}
}

pub type MigrateLockModelMigration<T> = frame_support::migrations::VersionedMigration<
	10,
	11,
	MigrateLockModel<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use polkadot_sdk::sp_core::H256;

	fn old_lock(is_funded: bool) -> LockedBitcoinV10<Test> {
		LockedBitcoinV10 {
			vault_id: 1,
			liquidity_promised: 25_000_000_000,
			locked_target_price: 30_000_000_000,
			owner_account: 2,
			securitization_ratio: FixedU128::from_rational(3, 2),
			security_fees: 17,
			coupon_paid_fees: 5,
			satoshis: SATOSHIS_PER_BITCOIN / 2,
			utxo_satoshis: is_funded.then_some(52_000_000),
			vault_pubkey: CompressedBitcoinPubkey([1; 33]),
			vault_claim_pubkey: CompressedBitcoinPubkey([2; 33]),
			vault_xpub_sources: ([3; 4], 4, 5),
			owner_pubkey: CompressedBitcoinPubkey([6; 33]),
			vault_claim_height: 1_000,
			open_claim_height: 1_030,
			created_at_height: 100,
			utxo_script_pubkey: BitcoinCosignScriptPubkey::P2WSH {
				wscript_hash: H256::repeat_byte(7),
			},
			is_funded,
			is_flexible: true,
			fund_hold_extensions: BoundedBTreeMap::new(),
			created_at_argon_block: 9,
		}
	}

	#[test]
	fn v10_lock_is_decoded_before_its_securitization_hold_is_scheduled() {
		new_test_ext().execute_with(|| {
			let lock_id = 7;
			v10::LocksByUtxoId::<Test>::insert(lock_id, old_lock(false));

			MigrateLockModel::<Test>::on_runtime_upgrade();

			let lock = LocksById::<Test>::get(lock_id).expect("converted lock");
			assert_eq!(lock.securitization_basis.satoshis, SATOSHIS_PER_BITCOIN / 2);
			assert_eq!(lock.securitization_basis.microgons_at_target_per_btc, 60_000_000_000);
			assert_eq!(lock.securitization_tick, 1);
			assert_eq!(lock.funded_satoshis, 0);
			assert_eq!(lock.fissioned_satoshis, 0);
			assert_eq!(lock.owner_account, 2);
			assert_eq!(lock.security_fees, 17);
			assert_eq!(lock.coupon_paid_fees, 5);
			assert_eq!(lock.created_at_argon_block, 9);
			assert_eq!(lock.securitization_hold_expiration_bitcoin_height, 245);
			assert!(
				SecuritizationHoldExpirationsByBitcoinHeight::<Test>::get(245).contains(&lock_id)
			);
			assert_eq!(LastProcessedSecuritizationHoldBitcoinHeight::<Test>::get(), Some(0));
			assert!(!pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::contains_key(
				2, lock_id
			));
		});
	}

	#[test]
	fn funded_v10_lock_recreates_its_existing_fission() {
		new_test_ext().execute_with(|| {
			let lock_id = 7;
			v10::LocksByUtxoId::<Test>::insert(lock_id, old_lock(true));
			MigrateLockModel::<Test>::on_runtime_upgrade();

			let lock = LocksById::<Test>::get(lock_id).expect("migrated lock");
			assert_eq!(lock.get_securitization().collateral_required(), 37_500_000_000);

			let fission = pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::get(2, lock_id)
				.expect("migrated fission");
			assert_eq!(fission.liquid_id, lock_id);
			assert_eq!(fission.lock_id, lock_id);
			assert_eq!(fission.satoshis, SATOSHIS_PER_BITCOIN / 2);
			assert_eq!(fission.microgons_at_target_per_btc, 60_000_000_000);
			assert_eq!(fission.last_ratchet_tick, 1);
			assert_eq!(fission.liquidity_promised, 25_000_000_000);
			assert_eq!(fission.created_at_argon_block, 9);
			assert_eq!(fission.last_updated_argon_block, 9);
			assert_eq!(fission.ratchet_number, 0);
			assert_eq!(pallet_bitcoin_fissions::NextFissionIdByOwner::<Test>::get(2), 8);
			assert!(pallet_bitcoin_fissions::FissionIdsByLockId::<Test>::get(lock_id)
				.contains(&lock_id));
		});
	}

	#[test]
	fn v10_release_cosign_height_becomes_a_legacy_recovery_tombstone() {
		new_test_ext().execute_with(|| {
			let lock_id = 7;
			v10::LockReleaseCosignHeightById::<Test>::insert(lock_id, 42);

			MigrateLockModel::<Test>::on_runtime_upgrade();

			assert_eq!(
				LockReleaseCosignHeightById::<Test>::get(lock_id),
				Some(LockReleaseCosignHeight {
					cosign_height: 42,
					previous_cosign_height: None,
					release_number: 1,
				})
			);
		});
	}

	#[test]
	fn v10_pending_releases_are_cancelled_while_migrating_the_lock() {
		new_test_ext().execute_with(|| {
			let lock_id = 7;
			let cosign_due_frame = 5;
			v10::LocksByUtxoId::<Test>::insert(lock_id, old_lock(false));
			v10::LockReleaseRequestsByUtxoId::<Test>::insert(
				lock_id,
				LockReleaseRequestV10 {
					lock_id,
					vault_id: 1,
					bitcoin_network_fee: 1_000,
					cosign_due_frame,
					to_script_pubkey: BitcoinScriptPubkey(BoundedVec::truncate_from(vec![1; 32])),
					securitization_at_risk: 1,
				},
			);
			LockCosignDueByFrame::<Test>::mutate(cosign_due_frame, |locks| {
				locks.try_insert(lock_id).expect("one lock fits");
			});
			StaticVaultProvider::update_pending_cosign_list(1, lock_id, false)
				.expect("pending list");

			MigrateLockModel::<Test>::on_runtime_upgrade();

			assert!(LocksById::<Test>::contains_key(lock_id));
			assert!(!v10::LockReleaseRequestsByUtxoId::<Test>::contains_key(lock_id));
			assert!(LockCosignDueByFrame::<Test>::get(cosign_due_frame).is_empty());
			assert!(VaultViewOfCosignPendingLocks::get().get(&1).unwrap().is_empty());
		});
	}
}
