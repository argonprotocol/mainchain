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
fn calculates_both_principals_inside_and_outside_the_range_for_either_token_order() {
	// TickMath(-100), TickMath(0) and TickMath(100). Expected principal uses floor division
	// of L * Q96 * (upper - price) / (price * upper) and L * (price - lower) / Q96.
	let sqrt_lower = 78_833_030_112_140_176_575_862_854_579u128;
	let sqrt_upper = 79_625_275_426_524_748_796_330_556_128u128;
	let cases = [
		(sqrt_lower - 1, 9_999_541_693_800_299u128, 0u128),
		(sqrt_lower, 9_999_541_693_800_299, 0),
		(1u128 << 96, 4_987_272_070_749_096, 4_987_272_070_749_096),
		(79_327_254_682_584_544_153_556_411_592, 3_738_115_413_094_571, 6_237_991_072_371_352),
		(sqrt_upper, 0, 9_999_541_693_800_299),
		(sqrt_upper + 1, 0, 9_999_541_693_800_299),
	];

	for (sqrt_price, amount0, amount1) in cases {
		let argon_first = position_checkpoint(1_000_000_000_000_000_000, -100, 100)
			.amounts_at_price(U256::from(sqrt_price), true)
			.unwrap();
		assert_eq!(argon_first.argon, amount0 / 1_000_000_000_000);
		assert_eq!(argon_first.usdc, amount1);

		let usdc_first = position_checkpoint(1_000_000_000_000_000_000, -100, 100)
			.amounts_at_price(U256::from(sqrt_price), false)
			.unwrap();
		assert_eq!(usdc_first.usdc, amount0);
		assert_eq!(usdc_first.argon, amount1 / 1_000_000_000_000);
	}
}

#[test]
fn empty_and_fractional_positions_do_not_invent_principal() {
	for liquidity in [0, 1] {
		let amounts = position_checkpoint(liquidity, -100, 100)
			.amounts_at_price(U256::one() << 96, true)
			.unwrap();
		assert_eq!(amounts, PositionAmounts { argon: 0, usdc: 0 });
	}
}

#[test]
fn converts_argon_to_microgons_and_drops_sub_microgon_dust() {
	let cases = [
		(200_510_416_479_002_803_287u128, 999_999u128, 999_999_999_999_999_999u128),
		(200_510_416_479_002_803_288, MICROGONS_PER_ARGON, 1_000_000_000_000_000_000),
	];
	for (liquidity, microgons, usdc) in cases {
		let amounts = position_checkpoint(liquidity, -100, 100)
			.amounts_at_price(U256::one() << 96, true)
			.unwrap();
		assert_eq!(amounts, PositionAmounts { argon: microgons, usdc });
	}
}

#[test]
fn scales_argon_before_checking_whether_it_fits_balance() {
	// The raw 18-decimal amount exceeds u128, but its native microgon balance fits.
	let amounts = position_checkpoint(1_000_000_000_000_000_000_000_000, -887_272, 887_272)
		.amounts_at_price(U256::from(4_295_128_739u64), true)
		.unwrap();
	assert_eq!(amounts.argon, 18_446_050_707_367_246_063_248_664_439_159);
	assert_eq!(amounts.usdc, 0);
}

#[test]
fn rejects_principal_that_exceeds_its_balance_representation() {
	for argon_is_token0 in [true, false] {
		assert_eq!(
			position_checkpoint(u128::MAX, -887_272, 887_272)
				.amounts_at_price(U256::from(4_295_128_739u64), argon_is_token0),
			Err(PositionAmountError::AmountOverflow)
		);
	}
}

#[test]
fn rejects_invalid_tick_ranges_and_prices_without_truncating_them() {
	for (tick_lower, tick_upper) in [
		(100, 100),
		(100, -100),
		(-887_273, 100),
		(-100, 887_273),
		(i32::MIN, 100),
		(-100, i32::MAX),
	] {
		assert!(position_checkpoint(1_000, tick_lower, tick_upper)
			.amounts_at_price(U256::one() << 96, true)
			.is_err());
	}

	let max_sqrt_price =
		U256::from_dec_str("1461446703485210103287273052203988822378723970342").unwrap();
	for sqrt_price in [
		U256::zero(),
		U256::from(4_295_128_738u64),
		max_sqrt_price,
		(U256::one() << 160) + (U256::one() << 96),
		U256::MAX,
	] {
		assert!(position_checkpoint(1_000, -100, 100)
			.amounts_at_price(sqrt_price, true)
			.is_err());
	}
}

#[test]
fn opening_prices_are_fixed_and_history_expires_by_frame_age() {
	new_test_ext().execute_with(|| {
		let mut index = PriceIndex {
			btc_usd_price: FixedU128::from_u32(60_000),
			argonot_usd_price: FixedU128::from_u32(2),
			argon_usd_price: FixedU128::one(),
			argon_usd_target_price: FixedU128::one(),
			argon_time_weighted_average_liquidity: 1_000,
			tick: 20,
			usdc_usd_price: Some(FixedU128::one()),
			stabilizer_pool_sqrt_price_x96: Some(U256::one() << 96),
		};
		StabilizerPools::on_price_index(&index);
		assert_eq!(FrameStartPriceIndices::<Test>::get().get(&1), Some(&index));

		// Subsequent swaps and target changes must not revalue the opening reference.
		let opening = index;
		CurrentTick::set(21);
		index.tick = 21;
		index.stabilizer_pool_sqrt_price_x96 = Some(U256::from(2) << 96);
		index.argon_usd_target_price = FixedU128::from_rational(101, 100);
		StabilizerPools::on_price_index(&index);
		assert_eq!(FrameStartPriceIndices::<Test>::get().get(&1), Some(&opening));

		// The transition flag selects the new frame before the ordinary frame ID advances.
		NewlyStartedFrame::set(Some(2));
		index.stabilizer_pool_sqrt_price_x96 = None;
		StabilizerPools::on_price_index(&index);
		assert_eq!(FrameStartPriceIndices::<Test>::get().get(&2), Some(&index));

		CurrentFrame::set(2);
		NewlyStartedFrame::set(None);
		index.stabilizer_pool_sqrt_price_x96 = opening.stabilizer_pool_sqrt_price_x96;
		StabilizerPools::on_price_index(&index);
		assert_eq!(
			FrameStartPriceIndices::<Test>::get()
				.get(&2)
				.unwrap()
				.stabilizer_pool_sqrt_price_x96,
			None
		);

		// A missed frame remains absent. An older index cannot fill the next frame either.
		CurrentFrame::set(4);
		CurrentTick::set(40);
		StabilizerPools::on_price_index(&index);
		assert!(!FrameStartPriceIndices::<Test>::get().contains_key(&3));
		assert!(!FrameStartPriceIndices::<Test>::get().contains_key(&4));

		// Retention counts financial frames, not observations or the network's wall-clock speed.
		for frame_id in 4..=33 {
			CurrentFrame::set(frame_id);
			CurrentTick::set(frame_id * 10);
			index.tick = frame_id * 10;
			StabilizerPools::on_price_index(&index);
		}
		assert_eq!(FrameStartPriceIndices::<Test>::get().get(&1), Some(&opening));

		// Frame 34 expires frame 1 even though the missing frame 3 leaves unused capacity.
		CurrentFrame::set(34);
		CurrentTick::set(340);
		index.tick = 340;
		StabilizerPools::on_price_index(&index);
		let history = FrameStartPriceIndices::<Test>::get();
		assert!(!history.contains_key(&1));
		assert!(history.contains_key(&2));
		assert!(!history.contains_key(&3));
		assert_eq!(history.get(&34), Some(&index));
		assert!(history.len() <= 33);

		for frame_id in 35..=37 {
			CurrentFrame::set(frame_id);
			CurrentTick::set(frame_id * 10);
			index.tick = frame_id * 10;
			StabilizerPools::on_price_index(&index);
		}
		let history = FrameStartPriceIndices::<Test>::get();
		assert_eq!(history.keys().next(), Some(&5));
		assert_eq!(history.len(), 33);
		assert!(history.encoded_size() < 5_000);

		// Returning after an oracle gap removes all expired references in the same update.
		CurrentFrame::set(100);
		CurrentTick::set(1_000);
		index.tick = 1_000;
		StabilizerPools::on_price_index(&index);
		let history = FrameStartPriceIndices::<Test>::get();
		assert_eq!(history.keys().copied().collect::<Vec<_>>(), vec![100]);
		assert_eq!(history.get(&100), Some(&index));
	});
}

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
		assert_ok!(StabilizerPools::checkpoint_position(
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
			StabilizerPools::checkpoint_position(
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
			StabilizerPools::register_pool(
				RuntimeOrigin::signed(beneficiary_account()),
				pool_manager(),
				config()
			),
			DispatchError::BadOrigin
		);
		assert_noop!(
			StabilizerPools::register_pool(RuntimeOrigin::root(), pool_manager(), config()),
			Error::<Test>::PoolAlreadyRegistered
		);

		let mut changed = config();
		changed.subscriber = H160::repeat_byte(0x99);
		assert_noop!(
			StabilizerPools::register_pool(RuntimeOrigin::root(), pool_manager(), changed),
			Error::<Test>::PoolAlreadyRegistered
		);

		let mut invalid = config();
		invalid.key.tick_spacing = 0;
		assert_noop!(
			StabilizerPools::register_pool(RuntimeOrigin::root(), pool_manager(), invalid),
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
	StabilizerPools::checkpoint_position(
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
