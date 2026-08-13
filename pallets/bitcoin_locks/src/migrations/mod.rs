use crate::{Config, LockedBitcoin, LocksByUtxoId, Pallet};
use alloc::{collections::BTreeMap, vec::Vec};
use argon_primitives::{bitcoin::UtxoId, vault::BitcoinVaultProvider, VaultId};
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use pallet_prelude::*;
use sp_arithmetic::FixedU128;

#[cfg(feature = "try-runtime")]
use codec::{Decode, Encode};
#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

pub struct CorrectLegacyRatchetedLiquidity<T>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for CorrectLegacyRatchetedLiquidity<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let locks = LocksByUtxoId::<T>::iter().collect::<Vec<_>>();
		let lock_count_by_vault = Self::lock_count_by_vault(&locks);
		if let Some((vault_id, lock_count)) =
			Self::underbacked_multi_lock_vaults(&locks, &lock_count_by_vault).first()
		{
			log::error!(
				"Cannot safely correct legacy ratcheted liquidity for underbacked vault {vault_id}: found {lock_count} active locks"
			);
			return Err(TryRuntimeError::Other(
				"legacy ratcheted liquidity correction found an underbacked vault with multiple active locks",
			));
		}
		let corrections = locks
			.into_iter()
			.filter_map(|(utxo_id, lock)| {
				let corrected_liquidity = Self::corrected_liquidity(&lock, &lock_count_by_vault)?;
				Some((utxo_id, corrected_liquidity))
			})
			.collect::<Vec<_>>();

		Ok(corrections.encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let locks = LocksByUtxoId::<T>::iter().collect::<Vec<_>>();
		let lock_count_by_vault = Self::lock_count_by_vault(&locks);
		for (vault_id, lock_count) in
			Self::underbacked_multi_lock_vaults(&locks, &lock_count_by_vault)
		{
			log::warn!(
				"Skipping legacy ratcheted liquidity correction for underbacked vault {vault_id}: found {lock_count} active locks"
			);
		}
		let reads = (locks.len() as u64).saturating_mul(2);
		let mut writes = 0u64;
		let corrections = locks
			.into_iter()
			.filter_map(|(utxo_id, lock)| {
				let corrected_liquidity = Self::corrected_liquidity(&lock, &lock_count_by_vault)?;
				Some((utxo_id, lock, corrected_liquidity))
			})
			.collect::<Vec<_>>();

		for (utxo_id, mut lock, new_liquidity_promised) in corrections {
			lock.liquidity_promised = new_liquidity_promised;
			LocksByUtxoId::<T>::insert(utxo_id, &lock);
			writes.saturating_accrue(1);
		}

		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let corrections = <Vec<(UtxoId, T::Balance)>>::decode(&mut state.as_slice())
			.map_err(|_| TryRuntimeError::Other("could not decode ratchet correction state"))?;
		for (utxo_id, expected_liquidity) in corrections {
			let lock = LocksByUtxoId::<T>::get(utxo_id)
				.ok_or(TryRuntimeError::Other("corrected bitcoin lock was removed"))?;
			ensure!(
				lock.liquidity_promised == expected_liquidity,
				TryRuntimeError::Other("bitcoin lock liquidity was not corrected"),
			);
		}

		Ok(())
	}
}

impl<T: Config> CorrectLegacyRatchetedLiquidity<T> {
	fn lock_count_by_vault(locks: &[(UtxoId, LockedBitcoin<T>)]) -> BTreeMap<VaultId, u32> {
		let mut lock_count_by_vault = BTreeMap::<VaultId, u32>::new();
		for (_, lock) in locks {
			lock_count_by_vault
				.entry(lock.vault_id)
				.and_modify(|count| count.saturating_accrue(1))
				.or_insert(1);
		}

		lock_count_by_vault
	}

	fn corrected_liquidity(
		lock: &LockedBitcoin<T>,
		lock_count_by_vault: &BTreeMap<VaultId, u32>,
	) -> Option<T::Balance> {
		if !lock.is_funded || lock.securitization_ratio != FixedU128::one() {
			return None;
		}

		if lock_count_by_vault.get(&lock.vault_id) != Some(&1) {
			return None;
		}

		let corrected_liquidity = T::VaultProvider::get_locked_securitization(lock.vault_id)?;
		(lock.liquidity_promised > corrected_liquidity).then_some(corrected_liquidity)
	}

	fn underbacked_multi_lock_vaults(
		locks: &[(UtxoId, LockedBitcoin<T>)],
		lock_count_by_vault: &BTreeMap<VaultId, u32>,
	) -> Vec<(VaultId, u32)> {
		lock_count_by_vault
			.iter()
			.filter(|(_, count)| **count > 1)
			.filter_map(|(vault_id, lock_count)| {
				let mut liquidity_promised = T::Balance::zero();
				for (_, lock) in locks.iter().filter(|(_, lock)| lock.vault_id == *vault_id) {
					if !lock.is_funded || lock.securitization_ratio != FixedU128::one() {
						return None;
					}
					liquidity_promised.saturating_accrue(lock.liquidity_promised);
				}

				let securitization_locked = T::VaultProvider::get_locked_securitization(*vault_id)?;
				(liquidity_promised > securitization_locked).then_some((*vault_id, *lock_count))
			})
			.collect()
	}
}

pub type CorrectLegacyRatchetedLiquidityMigration<T> =
	frame_support::migrations::VersionedMigration<
		9,
		10,
		CorrectLegacyRatchetedLiquidity<T>,
		Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, LockOptions, LocksByUtxoId, MicrogonPerBtcHistory, Pallet};
	use argon_primitives::{
		bitcoin::{CompressedBitcoinPubkey, SATOSHIS_PER_BITCOIN},
		BitcoinUtxoEvents, MICROGONS_PER_ARGON,
	};
	use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};

	#[test]
	fn derives_ratchet_correction_from_vault_backing() {
		new_test_ext().execute_with(|| {
			setup_overpromised_lock();

			#[cfg(feature = "try-runtime")]
			let state = CorrectLegacyRatchetedLiquidityMigration::<Test>::pre_upgrade()
				.expect("pre-upgrade checks");
			CorrectLegacyRatchetedLiquidityMigration::<Test>::on_runtime_upgrade();
			#[cfg(feature = "try-runtime")]
			CorrectLegacyRatchetedLiquidityMigration::<Test>::post_upgrade(state)
				.expect("post-upgrade checks");

			assert_eq!(LocksByUtxoId::<Test>::get(1).unwrap().liquidity_promised, 13_075_028_428);
		});
	}

	#[test]
	fn does_not_correct_an_unfunded_lock() {
		new_test_ext().execute_with(|| {
			let original_liquidity = setup_overpromised_lock();
			LocksByUtxoId::<Test>::mutate(1, |lock| {
				lock.as_mut().expect("initialized lock").is_funded = false;
			});

			CorrectLegacyRatchetedLiquidityMigration::<Test>::on_runtime_upgrade();

			assert_eq!(
				LocksByUtxoId::<Test>::get(1).unwrap().liquidity_promised,
				original_liquidity
			);
		});
	}

	#[test]
	fn does_not_correct_a_non_one_to_one_lock() {
		new_test_ext().execute_with(|| {
			let original_liquidity = setup_overpromised_lock();
			LocksByUtxoId::<Test>::mutate(1, |lock| {
				lock.as_mut().expect("initialized lock").securitization_ratio =
					FixedU128::from_rational(2, 1);
			});

			CorrectLegacyRatchetedLiquidityMigration::<Test>::on_runtime_upgrade();

			assert_eq!(
				LocksByUtxoId::<Test>::get(1).unwrap().liquidity_promised,
				original_liquidity
			);
		});
	}

	#[test]
	fn does_not_correct_a_vault_with_multiple_active_locks() {
		new_test_ext().execute_with(|| {
			let original_liquidity = setup_overpromised_lock();
			insert_second_lock();

			CorrectLegacyRatchetedLiquidityMigration::<Test>::on_runtime_upgrade();

			assert_eq!(
				LocksByUtxoId::<Test>::get(1).unwrap().liquidity_promised,
				original_liquidity
			);
			assert_eq!(
				LocksByUtxoId::<Test>::get(2).unwrap().liquidity_promised,
				original_liquidity
			);
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn try_runtime_rejects_an_underbacked_vault_with_multiple_active_locks() {
		new_test_ext().execute_with(|| {
			setup_overpromised_lock();
			insert_second_lock();

			assert!(CorrectLegacyRatchetedLiquidityMigration::<Test>::pre_upgrade().is_err());
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn try_runtime_allows_a_fully_backed_vault_with_multiple_active_locks() {
		new_test_ext().execute_with(|| {
			let liquidity_promised = setup_overpromised_lock();
			insert_second_lock();
			DefaultVault::mutate(|vault| {
				vault.securitization_locked = liquidity_promised.saturating_mul(2);
			});

			assert!(CorrectLegacyRatchetedLiquidityMigration::<Test>::pre_upgrade().is_ok());
		});
	}

	#[test]
	fn does_not_correct_liquidity_already_covered_by_vault_backing() {
		new_test_ext().execute_with(|| {
			setup_overpromised_lock();
			let covered_liquidity = 13_000_000_000;
			LocksByUtxoId::<Test>::mutate(1, |lock| {
				lock.as_mut().expect("initialized lock").liquidity_promised = covered_liquidity;
			});

			CorrectLegacyRatchetedLiquidityMigration::<Test>::on_runtime_upgrade();

			assert_eq!(
				LocksByUtxoId::<Test>::get(1).unwrap().liquidity_promised,
				covered_liquidity
			);
		});
	}

	fn setup_overpromised_lock() -> Balance {
		System::set_block_number(1);
		set_bitcoin_height(1);
		let who = 1;
		let target = 13_121_461_837;
		let liquidity_promised = 13_200_000_000;
		MicrogonPerBtcHistory::<Test>::mutate(|history| {
			_ = history.try_push((1, target));
		});
		DefaultVault::mutate(|vault| {
			vault.securitization = 50_000 * MICROGONS_PER_ARGON;
			vault.securitization_target = vault.securitization;
		});
		assert_ok!(BitcoinLocks::initialize(
			RuntimeOrigin::signed(who),
			1,
			SATOSHIS_PER_BITCOIN,
			CompressedBitcoinPubkey([1; 33]),
			Some(LockOptions::V1 { microgons_at_target_per_btc: Some(target) }),
		));
		assert_ok!(BitcoinLocks::funding_received(1, SATOSHIS_PER_BITCOIN));
		LocksByUtxoId::<Test>::mutate(1, |lock| {
			lock.as_mut().expect("initialized lock").liquidity_promised = liquidity_promised;
		});
		DefaultVault::mutate(|vault| {
			vault.securitization_locked = 13_075_028_428;
		});
		StorageVersion::new(9).put::<Pallet<Test>>();

		liquidity_promised
	}

	fn insert_second_lock() {
		let second_lock = LocksByUtxoId::<Test>::get(1).expect("initialized lock");
		LocksByUtxoId::<Test>::insert(2, second_lock);
	}
}
