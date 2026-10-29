use crate::{
	mock::{evidence as build_evidence, *},
	*,
};
use alloy_consensus::Header as EthereumHeader;
use alloy_primitives::B256;
use argon_primitives::{EthereumExecutionBlockProof, EthereumExecutionHeader};
use frame_support::{assert_noop, assert_ok};
use pallet_ethereum_verifier::{types::ExecutionHeaderAnchor, ExecutionHeaderAnchors};
use sp_runtime::AccountId32;

#[test]
fn registers_and_proves_maintained_intervals_without_sync_or_rewards() {
	new_test_ext().execute_with(|| {
		let proof = evidence(100, 1, beneficiary_account(), 1_000, true);
		let proof = PositionProof::decode(&mut proof.encode().as_slice()).unwrap();
		assert_ok!(submit(proof));
		assert_eq!(checkpoint().maintained_since.number, 100);

		let later = evidence(200, 1, beneficiary_account(), 1_000, true);
		assert_ok!(submit(later));
		let record = checkpoint();
		assert_eq!(record.proven_through.timestamp_millis, 2_400_000);
		assert_eq!(record.maintained_since.number, 100);
		System::assert_last_event(
			Event::PositionCheckpointed {
				position_manager: config().position_manager,
				token_id: U256::one(),
				checkpoint: Box::new(record),
				outcome: CheckpointOutcome::Maintained {
					from: ProofBlock {
						hash: H256::from_low_u64_be(100),
						number: 100,
						timestamp_millis: 1_200_000,
					},
				},
			}
			.into(),
		);
	});
}

#[test]
fn revision_change_discards_unverified_gap_even_when_liquidity_returns_to_same_value() {
	new_test_ext().execute_with(|| {
		let first = evidence(100, 1, beneficiary_account(), 1_000, true);
		assert_ok!(submit(first));

		let changed = evidence(200, 3, beneficiary_account(), 1_000, true);
		assert_ok!(submit(changed));
		assert_eq!(checkpoint().maintained_since.number, 200);
		assert_eq!(checkpoint().revision, U256::from(3));

		let later = evidence(300, 3, beneficiary_account(), 1_000, true);
		assert_ok!(submit(later));
		assert_eq!(checkpoint().maintained_since.number, 200);
	});
}

#[test]
fn unchanged_counter_cannot_hide_an_unsubscribe_and_failed_proofs_preserve_checkpoint() {
	new_test_ext().execute_with(|| {
		let first = evidence(100, 1, beneficiary_account(), 1_000, true);
		assert_ok!(submit(first));

		let before = checkpoint();
		let unsubscribed = evidence(200, 1, beneficiary_account(), 1_000, false);
		assert_noop!(submit(unsubscribed), Error::<Test>::InvalidSubscription);
		assert_eq!(checkpoint(), before);
	});
}

#[test]
fn new_beneficiary_must_authorize_reenrollment_and_keeps_global_replay_watermark() {
	new_test_ext().execute_with(|| {
		let first = evidence(100, 1, beneficiary_account(), 1_000, true);
		assert_ok!(submit(first));

		let next = AccountId32::new([0x24; 32]);

		let changed = evidence(200, 2, next.clone(), 1_000, true);
		assert_noop!(submit(changed.clone()), Error::<Test>::NotBeneficiary);
		assert_ok!(StabilizationPools::checkpoint_position(
			RuntimeOrigin::signed(next.clone()),
			pool_manager(),
			pool_id(),
			U256::one(),
			Box::new(changed)
		));
		assert_eq!(checkpoint().beneficiary, next);
		assert_eq!(checkpoint().maintained_since.number, 200);

		let old = evidence(100, 1, beneficiary_account(), 1_000, true);
		assert_noop!(submit(old), Error::<Test>::StaleCheckpoint);
	});
}

#[test]
fn rejects_replays_regressed_revisions_and_unchanged_revision_state_changes() {
	new_test_ext().execute_with(|| {
		let first = evidence(100, 3, beneficiary_account(), 1_000, true);
		assert_ok!(submit(first.clone()));
		assert_noop!(submit(first), Error::<Test>::StaleCheckpoint);

		let changed = evidence(200, 3, beneficiary_account(), 900, true);
		assert_noop!(submit(changed), Error::<Test>::PositionChangedWithoutRevision);

		let regressed = evidence(300, 2, beneficiary_account(), 1_000, true);
		assert_noop!(submit(regressed), Error::<Test>::RevisionRegressed);
	});
}

#[test]
fn rejects_forged_values_and_wrong_slots_or_token_ids() {
	new_test_ext().execute_with(|| {
		let proof = evidence(100, 1, beneficiary_account(), 1_000, true);

		let mut tampered = proof.clone();
		tampered.pool_manager.slots[0].value = H256::from_low_u64_be(10_000);
		assert_noop!(submit(tampered), Error::<Test>::InvalidProof);

		let mut tampered = proof.clone();
		tampered.subscriber.slots[0].slot = H256::zero();
		assert_noop!(submit(tampered), Error::<Test>::InvalidStorageSlots);
		assert_noop!(
			StabilizationPools::checkpoint_position(
				RuntimeOrigin::signed(beneficiary_account()),
				pool_manager(),
				pool_id(),
				U256::from(2),
				Box::new(proof)
			),
			Error::<Test>::InvalidStorageSlots
		);
		assert!(PositionCheckpoints::<Test>::get(config().position_manager, U256::one()).is_none());
	});
}

#[test]
fn rejects_different_source_blocks_and_uninitialized_subscriptions() {
	new_test_ext().execute_with(|| {
		let mut proof = evidence(100, 1, beneficiary_account(), 1_000, true);
		proof.pool_manager.execution_block_proof.anchor_block_hash = H256::repeat_byte(9);
		assert_noop!(submit(proof), Error::<Test>::InconsistentProofBlocks);
		let proof = evidence(200, 0, beneficiary_account(), 1_000, true);
		assert_noop!(submit(proof), Error::<Test>::InvalidSubscription);
	});
}

#[test]
fn uses_authenticated_target_timestamp_when_proof_is_an_ancestor_of_anchor() {
	new_test_ext().execute_with(|| {
		let mut proof = evidence(100, 1, beneficiary_account(), 1_000, true);
		let state = ExecutionHeaderAnchors::<Test>::get(H256::from_low_u64_be(100)).unwrap();
		let target = EthereumHeader {
			number: 100,
			timestamp: 123,
			state_root: B256::from(state.state_root.0),
			..Default::default()
		};
		let anchor = ExecutionHeaderAnchor {
			block_hash: H256::repeat_byte(0xaa),
			parent_hash: H256::from(target.hash_slow().0),
			block_number: 101,
			timestamp_millis: 135_000,
			..state
		};
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		let execution = EthereumExecutionBlockProof {
			anchor_block_hash: anchor.block_hash,
			target_to_anchor_header_chain: vec![EthereumExecutionHeader {
				rlp: alloy_rlp::encode(&target).try_into().unwrap(),
			}]
			.try_into()
			.unwrap(),
		};
		proof.subscriber.execution_block_proof = execution.clone();
		proof.position_manager.execution_block_proof = execution.clone();
		proof.pool_manager.execution_block_proof = execution;
		assert_ok!(submit(proof));
		assert_eq!(checkpoint().proven_through.hash, H256::from(target.hash_slow().0));
		assert_eq!(checkpoint().proven_through.timestamp_millis, 123_000);
	});
}

#[test]
fn approvals_are_root_only_and_cannot_reconfigure_existing_pool_history() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			StabilizationPools::register_pool(
				RuntimeOrigin::signed(beneficiary_account()),
				pool_manager(),
				config()
			),
			DispatchError::BadOrigin
		);
		assert_noop!(
			StabilizationPools::register_pool(RuntimeOrigin::root(), pool_manager(), config()),
			Error::<Test>::PoolAlreadyRegistered
		);

		let mut changed = config();
		changed.subscriber = H160::repeat_byte(0x99);
		assert_noop!(
			StabilizationPools::register_pool(RuntimeOrigin::root(), pool_manager(), changed),
			Error::<Test>::PoolAlreadyRegistered
		);

		let mut invalid = config();
		invalid.key.tick_spacing = 0;
		assert_noop!(
			StabilizationPools::register_pool(RuntimeOrigin::root(), pool_manager(), invalid),
			Error::<Test>::InvalidPool
		);
	});
}

fn beneficiary_account() -> AccountId32 {
	AccountId32::new([0x42; 32])
}

fn evidence(
	block: u64,
	revision: u64,
	beneficiary: AccountId32,
	liquidity: u128,
	subscribed: bool,
) -> PositionProof {
	build_evidence::<Test>(
		block,
		revision,
		H256::from_slice(beneficiary.as_ref()),
		liquidity,
		subscribed,
	)
}

fn submit(proof: PositionProof) -> DispatchResult {
	StabilizationPools::checkpoint_position(
		RuntimeOrigin::signed(beneficiary_account()),
		pool_manager(),
		pool_id(),
		U256::one(),
		Box::new(proof),
	)
}

fn checkpoint() -> PositionCheckpoint<AccountId32> {
	PositionCheckpoints::<Test>::get(config().position_manager, U256::one()).unwrap()
}
