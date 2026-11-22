use crate::{Config, Pallet};
use argon_primitives::{block_seal::FrameId, PriceIndex};
use frame_support::{storage_alias, traits::OnRuntimeUpgrade};
use pallet_prelude::*;

pub(crate) mod v1 {
	use super::*;

	#[storage_alias]
	pub type FrameStartPriceIndices<T: Config> =
		StorageValue<Pallet<T>, BoundedBTreeMap<FrameId, PriceIndex, ConstU32<33>>, ValueQuery>;
}

/// Retire opening prices; existing checkpoints and cursors remain available for collection.
pub struct CollectMigration<T>(PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for CollectMigration<T> {
	fn on_runtime_upgrade() -> Weight {
		let version = StorageVersion::get::<Pallet<T>>();
		if version != StorageVersion::new(1) && version != StorageVersion::new(2) {
			return T::DbWeight::get().reads(1)
		}

		let mut writes = 1;
		if version == StorageVersion::new(1) {
			// An opening observation cannot price a closed earning frame.
			v1::FrameStartPriceIndices::<T>::kill();
			writes += 1;
		}
		StorageVersion::new(3).put::<Pallet<T>>();
		T::DbWeight::get().reads_writes(1, writes)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock::*, *};
	use frame_support::{assert_ok, traits::fungible::Inspect};
	#[test]
	fn migration_preserves_checkpoints_for_finalized_collection_without_a_position_walk() {
		for version in [1, 2] {
			new_test_ext().execute_with(|| {
				let (position, mut checkpoint) = enroll_reward_position();
				for frame in 2..=3 {
					start_mining_frame(frame, frame * 10);
					sample_reward_ticks(frame * 10..(frame + 1) * 10);
				}
				checkpoint.proven_through_block = ethereum_proof_block(201);
				PositionCheckpoints::<Test>::insert(
					position.position_manager,
					position.uniswap_nft_id,
					&checkpoint,
				);
				let enrollment = StabilizerPoolRegistrations::<Test>::get(position);
				StorageVersion::new(version).put::<StabilizerPools>();

				crate::migrations::CollectMigration::<Test>::on_runtime_upgrade();
				assert_eq!(StorageVersion::get::<StabilizerPools>(), StorageVersion::new(3));
				assert_eq!(StabilizerPoolRegistrations::<Test>::get(position), enrollment);
				assert!(PaidClaims::get().is_empty());
				crate::migrations::CollectMigration::<Test>::on_runtime_upgrade();

				start_mining_frame(4, 40);
				assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(
					checkpoint.beneficiary.clone()
				)));
				let collected = Balances::balance(&checkpoint.beneficiary);
				assert!(collected > 0);
				assert_ok!(TreasuryPositions::collect(RuntimeOrigin::signed(
					checkpoint.beneficiary.clone()
				)));
				assert_eq!(Balances::balance(&checkpoint.beneficiary), collected);
			});
		}
	}

	#[test]
	fn upgrade_retires_opening_price_indices_and_preserves_checkpoints() {
		use crate::migrations::{v1::FrameStartPriceIndices, CollectMigration};

		new_test_ext().execute_with(|| {
			let opening_index = stabilizer_pool_price_index(20, U256::one() << 96);
			StorageVersion::new(1).put::<StabilizerPools>();
			FrameStartPriceIndices::<Test>::mutate(|history| {
				history.try_insert(1, opening_index).unwrap()
			});
			let token_id = U256::one();
			let checkpoint = position_checkpoint(1_000, -100, 100);
			PositionCheckpoints::<Test>::insert(config().position_manager, token_id, &checkpoint);

			CollectMigration::<Test>::on_runtime_upgrade();

			assert!(!FrameStartPriceIndices::<Test>::exists());
			assert!(!PendingFrameRewardValue::<Test>::exists());
			assert!(StabilizerPoolPricesByFrame::<Test>::get().is_empty());
			assert_eq!(
				PositionCheckpoints::<Test>::get(config().position_manager, token_id),
				Some(checkpoint.clone())
			);
			assert_eq!(StorageVersion::get::<StabilizerPools>(), StorageVersion::new(3));

			// An existing checkpoint becomes eligible only after a fresh proof. Its earlier
			// continuity can still be recorded without treating that time as registered
			// participation.
			assert_ok!(StabilizerPools::checkpoint_position(
				RuntimeOrigin::signed(checkpoint.beneficiary.clone()),
				pool_manager(),
				pool_id(),
				U256::one(),
				Box::new(evidence(
					200,
					1,
					H256::from_slice(checkpoint.beneficiary.as_ref()),
					1_000,
					true
				)),
			));
			assert_eq!(
				StabilizerPoolRegistrations::<Test>::get(StabilizerPoolPositionId {
					position_manager: config().position_manager,
					uniswap_nft_id: token_id,
				}),
				Some(StabilizerPoolRegistration::Enrolled(StabilizerPoolEnrollment {
					first_eligible_frame_id: 2,
					next_claim_frame_id: 2
				}))
			);
			let range = StabilizerPoolLiquidityRange::from(&checkpoint);
			assert_eq!(
				RegisteredLiquidityByRange::<Test>::get(range).liquidity_for_frame(1),
				Some(U256::zero())
			);
			assert_eq!(
				RegisteredLiquidityByRange::<Test>::get(range).liquidity_for_frame(2),
				Some(U256::from(1_000))
			);
		});
	}
}
