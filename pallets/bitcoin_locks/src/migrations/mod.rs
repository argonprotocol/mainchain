use crate::{
	Config, LastPendingFundingExpirationHeight, LockReleaseRequestsByUtxoId, LockedBitcoin,
	LocksByUtxoId, LocksPendingFundingByBitcoinHeight, MicrogonsAtTargetPerBtcHistory,
	MigratedReleaseHoldByUtxoId, Pallet,
};
use argon_primitives::{
	bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinHeight, CompressedBitcoinPubkey, Satoshis, UtxoId,
		XPubChildNumber, XPubFingerprint, SATOSHIS_PER_BITCOIN,
	},
	VaultId,
};
use codec::{Decode, Encode};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

#[cfg(feature = "try-runtime")]
use crate::HoldReason;
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

mod v10 {
	use super::*;

	#[storage_alias]
	pub(super) type LocksByUtxoId<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, UtxoId, LockedBitcoinV10<T>, OptionQuery>;
}

/// Convert version 10 Locks and their existing funded positions, move funding references from the
/// observer pallet, preserve in-flight release holds, and schedule unfunded Locks.
pub struct MigrateLockModel<T>(core::marker::PhantomData<T>);

impl<T> UncheckedOnRuntimeUpgrade for MigrateLockModel<T>
where
	T: Config + pallet_bitcoin_fissions::Config<Balance = <T as Config>::Balance>,
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let locks = v10::LocksByUtxoId::<T>::iter()
			.map(|(utxo_id, lock)| {
				(
					utxo_id,
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
		let mut release_holds: Vec<(
			<T as frame_system::Config>::AccountId,
			<T as Config>::Balance,
			u32,
		)> = Vec::new();
		for (utxo_id, release) in LockReleaseRequestsByUtxoId::<T>::iter() {
			if release.securitization_at_risk.is_zero() {
				continue;
			}
			let lock = v10::LocksByUtxoId::<T>::get(utxo_id).ok_or(TryRuntimeError::Other(
				"legacy release request is missing its bitcoin lock",
			))?;
			if let Some((_, total, _)) = release_holds
				.iter_mut()
				.find(|(account_id, _, _)| *account_id == lock.owner_account)
			{
				*total = total
					.checked_add(&release.securitization_at_risk)
					.ok_or(TryRuntimeError::Other("legacy release hold total overflowed"))?;
			} else {
				release_holds.push((
					lock.owner_account.clone(),
					release.securitization_at_risk,
					frame_system::Pallet::<T>::account(&lock.owner_account).providers,
				));
			}
		}
		for (account_id, expected_hold, _) in &release_holds {
			ensure!(
				<T as Config>::Currency::balance_on_hold(
					&HoldReason::ReleaseBitcoinLock.into(),
					account_id,
				) == *expected_hold,
				TryRuntimeError::Other(
					"legacy bitcoin release hold does not match release requests"
				),
			);
		}

		Ok((locks, release_holds).encode())
	}

	fn on_runtime_upgrade() -> Weight {
		frame_support::storage::migration::move_storage_from_pallet(
			b"UtxoIdToFundingUtxoRef",
			b"BitcoinUtxos",
			b"BitcoinLocks",
		);
		StorageVersion::new(1).put::<pallet_bitcoin_fissions::Pallet<T>>();

		// Current releases do not hold owner funds. Mark only the in-flight version 10 requests
		// whose existing holds must be retired by a later terminal path.
		let mut release_request_count = 0u64;
		let mut release_hold_count = 0u64;
		for (utxo_id, release) in LockReleaseRequestsByUtxoId::<T>::iter() {
			release_request_count = release_request_count.saturating_add(1);
			if release.securitization_at_risk.is_zero() {
				continue;
			}

			MigratedReleaseHoldByUtxoId::<T>::insert(utxo_id, release.securitization_at_risk);
			release_hold_count = release_hold_count.saturating_add(1);
		}

		let mut migrated_locks = 0u64;
		let mut fission_migration_weight = Weight::zero();
		let current_bitcoin_height = T::BitcoinBlockHeightChange::get().1;
		LastPendingFundingExpirationHeight::<T>::put(current_bitcoin_height);
		let current_tick = T::CurrentTick::get();
		let microgons_at_target_per_btc_history = MicrogonsAtTargetPerBtcHistory::<T>::get();
		LocksByUtxoId::<T>::translate::<LockedBitcoinV10<T>, _>(|utxo_id, lock| {
			migrated_locks = migrated_locks.saturating_add(1);
			let funded_satoshis =
				if lock.is_funded { lock.utxo_satoshis.unwrap_or(lock.satoshis) } else { 0 };
			let fissioned_satoshis = if lock.is_funded { lock.satoshis } else { 0 };
			let microgons_at_target_per_btc =
				FixedU128::from_rational(SATOSHIS_PER_BITCOIN as u128, lock.satoshis as u128)
					.saturating_mul_int(lock.locked_target_price);
			let securitization_tick = microgons_at_target_per_btc_history
				.iter()
				.rev()
				.find_map(|(tick, value)| (*value == microgons_at_target_per_btc).then_some(*tick))
				.unwrap_or(current_tick);
			let funding_expiration_height = lock
				.created_at_height
				.saturating_add(T::MaxPendingConfirmationBlocks::get())
				.saturating_add(1);
			let funding_expiration_height = if lock.is_funded {
				funding_expiration_height
			} else {
				funding_expiration_height.max(current_bitcoin_height.saturating_add(1))
			};
			if lock.is_funded {
				let fission_id = utxo_id;
				let mut fission_ids = BoundedBTreeSet::new();
				fission_ids
					.try_insert(fission_id)
					.expect("MaxFissionsPerLock must permit one migrated Fission");
				pallet_bitcoin_fissions::FissionByOwnerAndId::<T>::insert(
					&lock.owner_account,
					fission_id,
					pallet_bitcoin_fissions::Fission {
						liquid_id: utxo_id,
						utxo_id,
						satoshis: lock.satoshis,
						microgons_at_target_per_btc,
						last_ratchet_tick: securitization_tick,
						liquidity_promised: lock.liquidity_promised,
						created_at_argon_block: lock.created_at_argon_block,
						ratchet_number: 0,
						last_updated_argon_block: lock.created_at_argon_block,
					},
				);
				pallet_bitcoin_fissions::FissionIdsByLockId::<T>::insert(utxo_id, fission_ids);
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
				securitized_satoshis: lock.satoshis,
				microgons_at_target_per_btc,
				securitization_coverage_microgons: lock.liquidity_promised,
				securitization_tick,
				funded_satoshis,
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
				funding_expiration_height,
				utxo_script_pubkey: lock.utxo_script_pubkey,
				is_flexible: lock.is_flexible,
				fund_hold_extensions: lock.fund_hold_extensions,
				created_at_argon_block: lock.created_at_argon_block,
			})
		});

		let mut reads = migrated_locks.saturating_add(release_request_count).saturating_add(2);
		let mut writes = migrated_locks.saturating_add(release_hold_count).saturating_add(1);
		for (utxo_id, lock) in LocksByUtxoId::<T>::iter() {
			reads = reads.saturating_add(1);
			if lock.is_funded() {
				continue;
			}
			let inserted = LocksPendingFundingByBitcoinHeight::<T>::try_mutate(
				lock.funding_expiration_height,
				|locks| locks.try_insert(utxo_id),
			);
			if inserted.is_ok() {
				writes = writes.saturating_add(1);
			} else {
				log::error!(
					"Unable to schedule pending funding expiration for bitcoin lock {utxo_id:?}"
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
			UtxoId,
			Satoshis,
			<T as Config>::Balance,
			bool,
			Option<Satoshis>,
			<T as Config>::Balance,
			<T as frame_system::Config>::AccountId,
			BlockNumberFor<T>,
		);
		let (locks, release_holds): (
			Vec<LockState<T>>,
			Vec<(<T as frame_system::Config>::AccountId, <T as Config>::Balance, u32)>,
		) = Decode::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode bitcoin lock migration state"))?;

		ensure!(
			LocksByUtxoId::<T>::iter_keys().count() == locks.len(),
			TryRuntimeError::Other("bitcoin lock count changed during migration"),
		);
		ensure!(
			StorageVersion::get::<pallet_bitcoin_fissions::Pallet<T>>() == StorageVersion::new(1),
			TryRuntimeError::Other("bitcoin Fission storage version was not initialized"),
		);
		for (
			utxo_id,
			securitized_satoshis,
			locked_target_price,
			was_funded,
			funding_satoshis,
			liquidity_promised,
			owner_account,
			created_at_argon_block,
		) in locks
		{
			let lock = LocksByUtxoId::<T>::get(utxo_id)
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
				lock.securitized_satoshis == securitized_satoshis &&
					lock.microgons_at_target_per_btc == expected_rate &&
					lock.funded_satoshis == expected_funded_satoshis &&
					lock.fissioned_satoshis == expected_fissioned_satoshis &&
					lock.owner_account == owner_account &&
					lock.created_at_argon_block == created_at_argon_block,
				TryRuntimeError::Other("bitcoin lock accounting changed during migration"),
			);
			if was_funded {
				let fission =
					pallet_bitcoin_fissions::FissionByOwnerAndId::<T>::get(&owner_account, utxo_id)
						.ok_or(TryRuntimeError::Other(
							"funded bitcoin lock Fission was not migrated",
						))?;
				ensure!(
					fission.liquid_id == utxo_id &&
						fission.utxo_id == utxo_id &&
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
					pallet_bitcoin_fissions::FissionIdsByLockId::<T>::get(utxo_id)
						.contains(&utxo_id) &&
						pallet_bitcoin_fissions::NextFissionIdByOwner::<T>::get(&owner_account) >
							utxo_id,
					TryRuntimeError::Other("funded bitcoin lock Fission indexes were not migrated"),
				);
			} else {
				ensure!(
					!pallet_bitcoin_fissions::FissionByOwnerAndId::<T>::contains_key(
						&owner_account,
						utxo_id,
					),
					TryRuntimeError::Other("unfunded bitcoin lock created a Fission"),
				);
				ensure!(
					LocksPendingFundingByBitcoinHeight::<T>::get(lock.funding_expiration_height)
						.contains(&utxo_id),
					TryRuntimeError::Other("unfunded bitcoin lock was not scheduled for funding"),
				);
			}
		}
		for (account_id, expected_hold, providers_before) in release_holds {
			ensure!(
				<T as Config>::Currency::balance_on_hold(
					&HoldReason::ReleaseBitcoinLock.into(),
					&account_id,
				) == expected_hold,
				TryRuntimeError::Other("legacy bitcoin release hold changed during migration"),
			);
			ensure!(
				frame_system::Pallet::<T>::account(&account_id).providers == providers_before,
				TryRuntimeError::Other(
					"legacy bitcoin release provider reference changed during migration"
				),
			);
		}

		let mut release_hold_count = 0usize;
		for (utxo_id, release) in LockReleaseRequestsByUtxoId::<T>::iter() {
			if release.securitization_at_risk.is_zero() {
				ensure!(
					!MigratedReleaseHoldByUtxoId::<T>::contains_key(utxo_id),
					TryRuntimeError::Other("zero release amount created a compatibility hold"),
				);
				continue;
			}

			release_hold_count = release_hold_count.saturating_add(1);
			ensure!(
				MigratedReleaseHoldByUtxoId::<T>::get(utxo_id) ==
					Some(release.securitization_at_risk),
				TryRuntimeError::Other("legacy bitcoin release hold was not identified"),
			);
		}
		ensure!(
			MigratedReleaseHoldByUtxoId::<T>::iter_keys().count() == release_hold_count,
			TryRuntimeError::Other("unexpected legacy bitcoin release hold was created"),
		);
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
	use crate::{
		mock::*, HoldReason, LockReleaseRequest, LockReleaseRequestsByUtxoId,
		MigratedReleaseHoldByUtxoId,
	};
	use argon_primitives::bitcoin::BitcoinScriptPubkey;
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
	fn v10_lock_is_decoded_and_converted_before_pending_funding_is_scheduled() {
		new_test_ext().execute_with(|| {
			let utxo_id = 7;
			v10::LocksByUtxoId::<Test>::insert(utxo_id, old_lock(false));

			MigrateLockModel::<Test>::on_runtime_upgrade();

			let lock = LocksByUtxoId::<Test>::get(utxo_id).expect("converted lock");
			assert_eq!(lock.securitized_satoshis, SATOSHIS_PER_BITCOIN / 2);
			assert_eq!(lock.microgons_at_target_per_btc, 60_000_000_000);
			assert_eq!(lock.securitization_tick, 1);
			assert_eq!(lock.funded_satoshis, 0);
			assert_eq!(lock.fissioned_satoshis, 0);
			assert_eq!(lock.owner_account, 2);
			assert_eq!(lock.security_fees, 17);
			assert_eq!(lock.coupon_paid_fees, 5);
			assert_eq!(lock.created_at_argon_block, 9);
			assert_eq!(lock.funding_expiration_height, 245);
			assert!(LocksPendingFundingByBitcoinHeight::<Test>::get(245).contains(&utxo_id));
			assert_eq!(LastPendingFundingExpirationHeight::<Test>::get(), Some(0));
			assert!(!pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::contains_key(
				2, utxo_id
			));
		});
	}

	#[test]
	fn funded_v10_lock_recreates_its_existing_fission() {
		new_test_ext().execute_with(|| {
			let utxo_id = 7;
			v10::LocksByUtxoId::<Test>::insert(utxo_id, old_lock(true));

			MigrateLockModel::<Test>::on_runtime_upgrade();

			let lock = LocksByUtxoId::<Test>::get(utxo_id).expect("migrated lock");
			assert_eq!(lock.get_securitization().collateral_required(), 37_500_000_000);

			let fission = pallet_bitcoin_fissions::FissionByOwnerAndId::<Test>::get(2, utxo_id)
				.expect("migrated fission");
			assert_eq!(fission.liquid_id, utxo_id);
			assert_eq!(fission.utxo_id, utxo_id);
			assert_eq!(fission.satoshis, SATOSHIS_PER_BITCOIN / 2);
			assert_eq!(fission.microgons_at_target_per_btc, 60_000_000_000);
			assert_eq!(fission.last_ratchet_tick, 1);
			assert_eq!(fission.liquidity_promised, 25_000_000_000);
			assert_eq!(fission.created_at_argon_block, 9);
			assert_eq!(fission.last_updated_argon_block, 9);
			assert_eq!(fission.ratchet_number, 0);
			assert_eq!(pallet_bitcoin_fissions::NextFissionIdByOwner::<Test>::get(2), 8);
			assert!(pallet_bitcoin_fissions::FissionIdsByLockId::<Test>::get(utxo_id)
				.contains(&utxo_id));
		});
	}

	#[test]
	fn v10_release_hold_is_preserved_for_the_in_flight_release() {
		new_test_ext().execute_with(|| {
			let utxo_id = 7;
			let redemption_amount = 200;
			set_argons(2, 1_000);
			v10::LocksByUtxoId::<Test>::insert(utxo_id, old_lock(true));
			LockReleaseRequestsByUtxoId::<Test>::insert(
				utxo_id,
				LockReleaseRequest {
					utxo_id,
					vault_id: 1,
					bitcoin_network_fee: 100,
					cosign_due_frame: 5,
					to_script_pubkey: BitcoinScriptPubkey(BoundedVec::new()),
					securitization_at_risk: redemption_amount,
				},
			);

			let hold_reason = HoldReason::ReleaseBitcoinLock.into();
			assert_ok!(Balances::hold(&hold_reason, &2, redemption_amount));
			let providers_before_release = System::account(2).providers;
			System::inc_providers(&2);

			MigrateLockModel::<Test>::on_runtime_upgrade();

			assert_eq!(Balances::balance_on_hold(&hold_reason, &2), redemption_amount);
			assert_eq!(Balances::free_balance(2), 1_000 - redemption_amount);
			assert_eq!(System::account(2).providers, providers_before_release + 1);
			assert_eq!(MigratedReleaseHoldByUtxoId::<Test>::get(utxo_id), Some(redemption_amount));
			assert_eq!(
				LockReleaseRequestsByUtxoId::<Test>::get(utxo_id)
					.expect("release request")
					.securitization_at_risk,
				redemption_amount
			);
		});
	}
}
