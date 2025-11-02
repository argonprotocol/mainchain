use crate::{
	AccountIndexLookup, ActiveMinersCount, Config, MinerNonceScoring, MinerNonceScoringByCohort,
	MinersByCohort, Pallet,
};

use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

#[storage_alias]
pub type MinerXorKeysByCohort<T: Config> = StorageValue<
	Pallet<T>,
	BoundedBTreeMap<
		FrameId,
		BoundedVec<U256, <T as Config>::MaxCohortSize>,
		<T as Config>::FramesPerMiningTerm,
	>,
	ValueQuery,
>;

impl<T: Config + pallet_treasury::Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let xor_keys = MinerXorKeysByCohort::<T>::get();
		Ok(xor_keys.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		log::info!("Migrating mining slots");
		let mut modify_count = 0;
		let xor_keys = MinerXorKeysByCohort::<T>::take();
		let mut oldest_frame: Option<FrameId> = None;
		MinerNonceScoringByCohort::<T>::mutate(|a| {
			for (frame_id, xor_key) in &xor_keys {
				if oldest_frame.is_none() {
					oldest_frame = Some(*frame_id);
				} else {
					oldest_frame = Some((*frame_id).min(oldest_frame.unwrap()));
				}
				let mut scoring_vec =
					BoundedVec::<MinerNonceScoring<T>, T::MaxCohortSize>::default();
				for nonce in xor_key.into_iter() {
					let _ = scoring_vec.try_push(MinerNonceScoring {
						nonce: *nonce,
						last_win_block: None,
						blocks_won: 0,
					});
				}
				let _ = a.try_insert(*frame_id, scoring_vec);
			}
		});
		let oldest_frame = oldest_frame.expect("At least one cohort should exist");
		log::info!("Oldest frame to keep: {:?}. {:#?}", oldest_frame, xor_keys);
		modify_count += 1;
		let cohort_frames = MinersByCohort::<T>::iter_keys().collect::<Vec<_>>();
		let mut active_miners = 0;
		for frame_id in cohort_frames {
			if frame_id < oldest_frame {
				log::info!("Cleaning up cohort frame {:?}", frame_id);
				modify_count += 1;
				let rotating_out = MinersByCohort::<T>::take(frame_id);
				for miner in rotating_out {
					let account_id = miner.account_id.clone();
					AccountIndexLookup::<T>::remove(&account_id);
					modify_count += 1;
					// assume this window is already past
					crate::Pallet::<T>::release_mining_seat_argonots(&miner, false);
				}
			} else {
				let cohort_miners = MinersByCohort::<T>::get(frame_id).len();
				log::info!("Keeping cohort frame {:?}, {} miners", frame_id, cohort_miners);
				active_miners += cohort_miners;
			}
		}
		let pool_frames = pallet_treasury::VaultPoolsByFrame::<T>::iter_keys().collect::<Vec<_>>();
		for frame_id in pool_frames {
			if frame_id < oldest_frame {
				let vault_pool = pallet_treasury::VaultPoolsByFrame::<T>::take(frame_id);
				modify_count += 1;
				for (vault_id, pool) in vault_pool {
					log::info!(
						"Refunding treasury pool for vault {:?}, frame {:?}",
						vault_id,
						frame_id
					);
					for (account_id, holder) in pool.bond_holders {
						pallet_treasury::Pallet::<T>::refund_fund_capital(
							frame_id,
							vault_id,
							&account_id,
							holder.pool_managed_balance(),
						);
						modify_count += 1;
					}
				}
			}
		}
		ActiveMinersCount::<T>::put(active_miners as u16);
		modify_count += 1;

		T::DbWeight::get().reads_writes((modify_count) as u64, modify_count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let original_keys = BoundedBTreeMap::<
			FrameId,
			BoundedVec<U256, T::MaxCohortSize>,
			T::FramesPerMiningTerm,
		>::decode(&mut &state[..])
		.expect("Failed to decode pre-upgrade state");

		let miner_nonce_scoring = MinerNonceScoringByCohort::<T>::get();
		let mut count = 0u16;
		for key in miner_nonce_scoring.keys() {
			assert!(
				MinersByCohort::<T>::contains_key(key),
				"Cohort frame {:?} should exist after migration",
				key
			);
			let orig = original_keys.get(key).unwrap().to_vec();
			let new = miner_nonce_scoring
				.get(key)
				.unwrap()
				.iter()
				.map(|s| s.nonce)
				.collect::<Vec<_>>();
			assert_eq!(orig, new, "Cohort frame {:?} should exist in pre-upgrade state", key);
			count += MinersByCohort::<T>::get(key).len() as u16;
		}
		assert_eq!(count, ActiveMinersCount::<T>::get());
		log::info!("Post-migration active miners count: {}", count);

		let pool_frames = pallet_treasury::VaultPoolsByFrame::<T>::iter_keys().collect::<Vec<_>>();
		for frame_id in pool_frames {
			assert!(
				frame_id >= *original_keys.keys().min().unwrap(),
				"Vault pool for frame {:?} should have been removed",
				frame_id
			);
		}

		Ok(())
	}
}

pub type MissedCohortsMigration<T> = frame_support::migrations::VersionedMigration<
	7,
	8,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
