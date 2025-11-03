use crate::{Config, MinerNonceScoring, MinerNonceScoringByCohort, Pallet};
use frame_support::{storage_alias, traits::UncheckedOnRuntimeUpgrade};
use pallet_prelude::*;
pub mod v8;

pub struct InnerMigrate<T: crate::Config>(core::marker::PhantomData<T>);

mod old_storage {
	use super::*;

	#[storage_alias]
	pub type MinerNonceScoringByCohort<T: Config> = StorageValue<
		Pallet<T>,
		BoundedBTreeMap<
			FrameId,
			BoundedVec<MinerNonceScoring<T>, <T as Config>::MaxCohortSize>,
			<T as Config>::FramesPerMiningTerm,
		>,
		ValueQuery,
	>;

	#[derive(
		Encode, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebugNoBound, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct MinerNonceScoring<T: Config> {
		pub nonce: U256,
		pub last_win_block: Option<BlockNumberFor<T>>,
		pub blocks_won: u32,
	}
}

impl<T: Config + pallet_treasury::Config> UncheckedOnRuntimeUpgrade for InnerMigrate<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let nonces = old_storage::MinerNonceScoringByCohort::<T>::get();
		Ok(nonces.encode())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		log::info!("Migrating mining slots");
		let modify_count = 1;
		let old_value = old_storage::MinerNonceScoringByCohort::<T>::take();
		MinerNonceScoringByCohort::<T>::mutate(|a| {
			for (frame_id, old_scoring_vec) in &old_value {
				let mut scoring_vec =
					BoundedVec::<MinerNonceScoring<T>, T::MaxCohortSize>::default();
				for old_scoring in old_scoring_vec.into_iter() {
					let _ = scoring_vec.try_push(MinerNonceScoring {
						nonce: old_scoring.nonce,
						last_win_block: old_scoring.last_win_block,
						blocks_won_in_frame: 0,
						frame_start_blocks_won_surplus: 0,
					});
				}
				let _ = a.try_insert(*frame_id, scoring_vec);
			}
		});

		T::DbWeight::get().reads_writes((modify_count) as u64, modify_count as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let original_keys = BoundedBTreeMap::<
			FrameId,
			BoundedVec<old_storage::MinerNonceScoring<T>, T::MaxCohortSize>,
			T::FramesPerMiningTerm,
		>::decode(&mut &state[..])
		.expect("Failed to decode pre-upgrade state");

		let miner_nonce_scoring = MinerNonceScoringByCohort::<T>::get();
		for key in miner_nonce_scoring.keys() {
			let orig = original_keys.get(key).unwrap().to_vec();
			let new = miner_nonce_scoring.get(key).unwrap();
			for (i, n) in new.iter().enumerate() {
				assert_eq!(
					n.blocks_won_in_frame, 0,
					"blocks_won_in_frame should be initialized to 0"
				);
				assert_eq!(
					n.frame_start_blocks_won_surplus, 0,
					"frame_start_blocks_won_surplus should be initialized to 0"
				);
				let matching_orig = &orig[i];
				assert_eq!(n.nonce, matching_orig.nonce, "nonce should match pre-migration value");
				assert_eq!(
					n.last_win_block, matching_orig.last_win_block,
					"last_win_block should match pre-migration value"
				);
			}
		}

		Ok(())
	}
}

pub type CohortFrameExpectedScoringMigration<T> = frame_support::migrations::VersionedMigration<
	8,
	9,
	InnerMigrate<T>,
	crate::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
