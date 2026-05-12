// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
pub use crate::mock::*;
use crate::{
	config::{EPOCHS_PER_SYNC_COMMITTEE_PERIOD, SLOTS_PER_EPOCH, SLOTS_PER_HISTORICAL_ROOT},
	fixture_conversions::execution_proof_from_fixture,
	functions::{compute_epoch, compute_period},
	mock::{
		load_checkpoint_update_fixture, load_execution_proof_fixture,
		load_finalized_header_update_fixture, load_next_finalized_header_update_fixture,
		load_next_sync_committee_update_fixture, load_sync_committee_update_fixture,
	},
	sync_committee_sum,
	types::{CheckpointUpdate, NextSyncCommitteeUpdate},
	verify_merkle_branch, BasicOperatingMode, BeaconHeader, Error, ExecutionHeaderAnchor,
	ExecutionHeaderAnchors, ExecutionProof, FinalizedBeaconHeaderState, FinalizedBeaconState, Fork,
	ForkVersionSchedule, ForkVersions, LatestFinalizedBlockRoot, LatestSyncCommitteeUpdatePeriod,
	NextSyncCommittee, SyncCommitteePrepared,
};
use alloy_consensus::Header as AlloyHeader;
use alloy_primitives::B256;
use alloy_rlp::Encodable;
use argon_primitives::{
	ethereum::{
		MAX_ETHEREUM_EXECUTION_HEADER_RLP_BYTES, MAX_ETHEREUM_HEADER_CHAIN_LEN,
		MAX_ETHEREUM_LOG_DATA_BYTES, MAX_ETHEREUM_LOG_TOPICS, MAX_ETHEREUM_RECEIPT_PROOF_NODES,
		MAX_ETHEREUM_RECEIPT_PROOF_NODE_BYTES,
	},
	CallTxPoolKeyProvider, EthereumExecutionBlockProof, EthereumExecutionHeader, EthereumLog,
	EthereumProof, EthereumReceiptProof, EthereumVerifyError, EthereumVerifyProvider,
};
use codec::{Decode, Encode};
use hex_literal::hex;
use polkadot_sdk::{
	frame_support::{
		assert_err, assert_noop, assert_ok, dispatch::DispatchResult, pallet_prelude::Pays,
	},
	sp_core::{hashing::blake2_256, H160, H256},
	sp_runtime::DispatchError,
};
use snowbridge_beacon_primitives::merkle_proof::{generalized_index_length, subtree_index};

/// Arbitrary hash used for tests and invalid hashes.
const TEST_HASH: [u8; 32] =
	hex!["5f6f02af29218292d21a69b64a794a7c0873b3e0f54611972863706e8cbdf371"];

fn anchor_execution_proof() -> ExecutionProof {
	execution_proof_from_fixture(load_execution_proof_fixture())
		.expect("execution proof fixture stays within bounded branch size")
}

fn make_execution_header(
	block_number: u64,
	parent_hash: H256,
	receipts_root: H256,
) -> (EthereumExecutionHeader, H256) {
	let header = AlloyHeader {
		number: block_number,
		parent_hash: b256_from_h256(parent_hash),
		receipts_root: b256_from_h256(receipts_root),
		..Default::default()
	};

	let block_hash = H256::from_slice(header.hash_slow().as_slice());
	let mut rlp = Vec::new();
	header.encode(&mut rlp);

	(
		EthereumExecutionHeader {
			rlp: rlp.try_into().expect("test execution header stays within bounded RLP size"),
		},
		block_hash,
	)
}

fn b256_from_h256(value: H256) -> B256 {
	B256::from_slice(value.as_bytes())
}

fn process_checkpoint_update(update: &CheckpointUpdate) -> DispatchResult {
	EthereumBeaconClient::process_checkpoint_update(update, &ChainForkVersions::get())
}

fn retained_anchor_verification_payload() -> (EthereumLog, EthereumProof, ExecutionHeaderAnchor) {
	let inbound_fixture = snowbridge_pallet_ethereum_client_fixtures::make_inbound_fixture();
	let anchor_block_hash = H256::repeat_byte(9);
	let anchor = ExecutionHeaderAnchor {
		block_number: 100,
		block_hash: anchor_block_hash,
		parent_hash: H256::repeat_byte(8),
		receipts_root: inbound_fixture.event.proof.execution_proof.execution_header.receipts_root(),
	};
	let event_log = EthereumLog {
		address: inbound_fixture.event.event_log.address,
		topics: inbound_fixture
			.event
			.event_log
			.topics
			.try_into()
			.expect("fixture topics stay within bounded Ethereum log topics"),
		data: inbound_fixture
			.event
			.event_log
			.data
			.try_into()
			.expect("fixture event data stays within bounded Ethereum log payload"),
	};
	let proof = EthereumProof {
		execution_block_proof: EthereumExecutionBlockProof {
			anchor_block_hash,
			target_to_anchor_header_chain: Vec::new()
				.try_into()
				.expect("empty header chain stays within bounds"),
		},
		receipt_proof: EthereumReceiptProof {
			transaction_index: INBOUND_FIXTURE_RECEIPT_INDEX,
			nodes: inbound_fixture
				.event
				.proof
				.receipt_proof
				.into_iter()
				.map(|node| {
					node.try_into().expect("fixture receipt proof node stays within bounded size")
				})
				.collect::<Vec<_>>()
				.try_into()
				.expect("fixture receipt proof stays within bounded node count"),
		},
	};

	(event_log, proof, anchor)
}

/* UNIT TESTS */

#[test]
pub fn sum_sync_committee_participation() {
	new_tester().execute_with(|| {
		assert_eq!(sync_committee_sum(&[0, 1, 0, 1, 1, 0, 1, 0, 1]), 5);
	});
}

#[test]
pub fn compute_domain() {
	new_tester().execute_with(|| {
		let domain = EthereumBeaconClient::compute_domain(
			hex!("07000000").into(),
			hex!("00000001"),
			hex!("5dec7ae03261fde20d5b024dfabce8bac3276c9a4908e23d50ba8c9b50b0adff").into(),
		);

		assert_ok!(&domain);
		assert_eq!(
			domain.unwrap(),
			hex!("0700000046324489ceb6ada6d118eacdbe94f49b1fcb49d5481a685979670c7c").into()
		);
	});
}

#[test]
pub fn compute_signing_root_bls() {
	new_tester().execute_with(|| {
		let signing_root = EthereumBeaconClient::compute_signing_root(
			&BeaconHeader {
				slot: 3529537,
				proposer_index: 192549,
				parent_root: hex!(
					"1f8dc05ea427f78e84e2e2666e13c3befb7106fd1d40ef8a3f67cf615f3f2a4c"
				)
				.into(),
				state_root: hex!(
					"0dfb492a83da711996d2d76b64604f9bca9dc08b6c13cf63b3be91742afe724b"
				)
				.into(),
				body_root: hex!("66fba38f7c8c2526f7ddfe09c1a54dd12ff93bdd4d0df6a0950e88e802228bfa")
					.into(),
			},
			hex!("07000000afcaaba0efab1ca832a15152469bb09bb84641c405171dfa2d3fb45f").into(),
		);

		assert_ok!(&signing_root);
		assert_eq!(
			signing_root.unwrap(),
			hex!("3ff6e9807da70b2f65cdd58ea1b25ed441a1d589025d2c4091182026d7af08fb").into()
		);
	});
}

#[test]
pub fn compute_signing_root() {
	new_tester().execute_with(|| {
		let signing_root = EthereumBeaconClient::compute_signing_root(
			&BeaconHeader {
				slot: 222472,
				proposer_index: 10726,
				parent_root: hex!(
					"5d481a9721f0ecce9610eab51d400d223683d599b7fcebca7e4c4d10cdef6ebb"
				)
				.into(),
				state_root: hex!(
					"14eb4575895f996a84528b789ff2e4d5148242e2983f03068353b2c37015507a"
				)
				.into(),
				body_root: hex!("7bb669c75b12e0781d6fa85d7fc2f32d64eafba89f39678815b084c156e46cac")
					.into(),
			},
			hex!("07000000e7acb21061790987fa1c1e745cccfb358370b33e8af2b2c18938e6c2").into(),
		);

		assert_ok!(&signing_root);
		assert_eq!(
			signing_root.unwrap(),
			hex!("da12b6a6d3516bc891e8a49f82fc1925cec40b9327e06457f695035303f55cd8").into()
		);
	});
}

#[test]
pub fn compute_domain_bls() {
	new_tester().execute_with(|| {
		let domain = EthereumBeaconClient::compute_domain(
			hex!("07000000").into(),
			hex!("01000000"),
			hex!("4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95").into(),
		);

		assert_ok!(&domain);
		assert_eq!(
			domain.unwrap(),
			hex!("07000000afcaaba0efab1ca832a15152469bb09bb84641c405171dfa2d3fb45f").into()
		);
	});
}

#[test]
pub fn may_refund_call_fee() {
	let finalized_update = Box::new(load_next_finalized_header_update_fixture());
	let sync_committee_update = Box::new(load_sync_committee_update_fixture());
	new_tester().execute_with(|| {
		let free_headers_interval: u64 = crate::mock::FREE_SLOTS_INTERVAL as u64;
		// Not free, smaller than the allowed free header interval
		assert_eq!(
			EthereumBeaconClient::check_refundable(
				&finalized_update.clone(),
				finalized_update.finalized_header.slot + free_headers_interval
			),
			Pays::Yes
		);
		// Is free, larger than the minimum interval
		assert_eq!(
			EthereumBeaconClient::check_refundable(
				&finalized_update,
				finalized_update.finalized_header.slot - (free_headers_interval + 2)
			),
			Pays::No
		);
		// Is free, valid sync committee update
		assert_eq!(
			EthereumBeaconClient::check_refundable(
				&sync_committee_update,
				finalized_update.finalized_header.slot
			),
			Pays::No
		);
	});
}

#[test]
pub fn verify_merkle_branch_for_finalized_root() {
	new_tester().execute_with(|| {
		assert!(verify_merkle_branch(
			hex!("0000000000000000000000000000000000000000000000000000000000000000").into(),
			&[
				hex!("0000000000000000000000000000000000000000000000000000000000000000").into(),
				hex!("5f6f02af29218292d21a69b64a794a7c0873b3e0f54611972863706e8cbdf371").into(),
				hex!("e7125ff9ab5a840c44bedb4731f440a405b44e15f2d1a89e27341b432fabe13d").into(),
				hex!("002c1fe5bc0bd62db6f299a582f2a80a6d5748ccc82e7ed843eaf0ae0739f74a").into(),
				hex!("d2dc4ba9fd4edff6716984136831e70a6b2e74fca27b8097a820cbbaa5a6e3c3").into(),
				hex!("91f77a19d8afa4a08e81164bb2e570ecd10477b3b65c305566a6d2be88510584").into(),
			],
			subtree_index(crate::config::altair::FINALIZED_ROOT_INDEX),
			generalized_index_length(crate::config::altair::FINALIZED_ROOT_INDEX),
			hex!("e46559327592741956f6beaa0f52e49625eb85dce037a0bd2eff333c743b287f").into()
		));
	});
}

#[test]
pub fn verify_merkle_branch_fails_if_depth_and_branch_dont_match() {
	new_tester().execute_with(|| {
		assert!(!verify_merkle_branch(
			hex!("0000000000000000000000000000000000000000000000000000000000000000").into(),
			&[
				hex!("0000000000000000000000000000000000000000000000000000000000000000").into(),
				hex!("5f6f02af29218292d21a69b64a794a7c0873b3e0f54611972863706e8cbdf371").into(),
				hex!("e7125ff9ab5a840c44bedb4731f440a405b44e15f2d1a89e27341b432fabe13d").into(),
			],
			subtree_index(crate::config::altair::FINALIZED_ROOT_INDEX),
			generalized_index_length(crate::config::altair::FINALIZED_ROOT_INDEX),
			hex!("e46559327592741956f6beaa0f52e49625eb85dce037a0bd2eff333c743b287f").into()
		));
	});
}

#[test]
pub fn sync_committee_participation_is_supermajority() {
	let bits = hex!(
		"bffffffff7f1ffdfcfeffeffbfdffffbfffffdffffefefffdffff7f7ffff77fffdf7bff77ffdf7fffafffffff77fefffeff7effffffff5f7fedfffdfb6ddff7b"
	);
	let participation =
		snowbridge_beacon_primitives::decompress_sync_committee_bits::<512, 64>(bits);
	assert_ok!(EthereumBeaconClient::sync_committee_participation_is_supermajority(&participation));
}

#[test]
pub fn sync_committee_participation_is_supermajority_errors_when_not_supermajority() {
	new_tester().execute_with(|| {
		let participation = hex!("0000000000000000000000000000000000000001010100010100000000000000000000000101010101000100010101010101010101010101010101010100010101000000000001010101010100010101000000000000000000000000000101000101010101010001010101010100010101010101010101010101000101010101010100010101010100000000010101010100000000000000000001010101010101010101010101010101010100010101010101010001010101010101010101010101010101000101010101010101010101010100010101010101010101010101010101010101010101010101010101010101010001010100010101010101010101000101010101010101010001010101010101010101000101010100010101010101010101010100010000000000000000000100000000000001010100000001000100010101010100000000000000000000000000000000000000010101010101010100010101010101010101010100010101010001010101010101010101010101010100000000000000000101010101000000000001000000000000000000010000000000000000000101010101010100010001010101010101000101010101010101010101010101010101000101010101010101010101010101010001010101010101010001010001000000000000000000000000000001000000000000");

		assert_err!(
			EthereumBeaconClient::sync_committee_participation_is_supermajority(&participation),
			Error::<Test>::SyncCommitteeParticipantsNotSupermajority
		);
	});
}

#[test]
fn compute_fork_version() {
	let mock_fork_versions = ForkVersions {
		genesis: Fork { version: [0, 0, 0, 0], epoch: 0 },
		altair: Fork { version: [0, 0, 0, 1], epoch: 10 },
		bellatrix: Fork { version: [0, 0, 0, 2], epoch: 20 },
		capella: Fork { version: [0, 0, 0, 3], epoch: 30 },
		deneb: Fork { version: [0, 0, 0, 4], epoch: 40 },
		electra: Fork { version: [0, 0, 0, 5], epoch: 50 },
		fulu: Fork { version: [0, 0, 0, 6], epoch: 60 },
	};
	new_tester().execute_with(|| {
		assert_eq!(EthereumBeaconClient::select_fork_version(&mock_fork_versions, 0), [0, 0, 0, 0]);
		assert_eq!(EthereumBeaconClient::select_fork_version(&mock_fork_versions, 1), [0, 0, 0, 0]);
		assert_eq!(
			EthereumBeaconClient::select_fork_version(&mock_fork_versions, 10),
			[0, 0, 0, 1]
		);
		assert_eq!(
			EthereumBeaconClient::select_fork_version(&mock_fork_versions, 21),
			[0, 0, 0, 2]
		);
		assert_eq!(
			EthereumBeaconClient::select_fork_version(&mock_fork_versions, 20),
			[0, 0, 0, 2]
		);
		assert_eq!(
			EthereumBeaconClient::select_fork_version(&mock_fork_versions, 32),
			[0, 0, 0, 3]
		);
		assert_eq!(
			EthereumBeaconClient::select_fork_version(&mock_fork_versions, 40),
			[0, 0, 0, 4]
		);
		assert_eq!(
			EthereumBeaconClient::select_fork_version(&mock_fork_versions, 50),
			[0, 0, 0, 5]
		);
	});
}

#[test]
fn find_absent_keys() {
	let participation: [u8; 32] =
		hex!("0001010101010100010101010101010101010101010101010101010101010101");
	let update = load_sync_committee_update_fixture();
	let sync_committee_prepared: SyncCommitteePrepared =
		(&update.next_sync_committee_update.unwrap().next_sync_committee)
			.try_into()
			.unwrap();

	new_tester().execute_with(|| {
		let pubkeys = EthereumBeaconClient::find_pubkeys(
			&participation,
			(*sync_committee_prepared.pubkeys).as_ref(),
			false,
		);
		assert_eq!(pubkeys.len(), 2);
		assert_eq!(pubkeys[0], sync_committee_prepared.pubkeys[0]);
		assert_eq!(pubkeys[1], sync_committee_prepared.pubkeys[7]);
	});
}

#[test]
fn find_present_keys() {
	let participation: [u8; 32] =
		hex!("0001000000000000010000000000000000000000000000000000010000000100");
	let update = load_sync_committee_update_fixture();
	let sync_committee_prepared: SyncCommitteePrepared =
		(&update.next_sync_committee_update.unwrap().next_sync_committee)
			.try_into()
			.unwrap();

	new_tester().execute_with(|| {
		let pubkeys = EthereumBeaconClient::find_pubkeys(
			&participation,
			(*sync_committee_prepared.pubkeys).as_ref(),
			true,
		);
		assert_eq!(pubkeys.len(), 4);
		assert_eq!(pubkeys[0], sync_committee_prepared.pubkeys[1]);
		assert_eq!(pubkeys[1], sync_committee_prepared.pubkeys[8]);
		assert_eq!(pubkeys[2], sync_committee_prepared.pubkeys[26]);
		assert_eq!(pubkeys[3], sync_committee_prepared.pubkeys[30]);
	});
}

/* SYNC PROCESS TESTS */

#[test]
fn process_initial_checkpoint() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());

	new_tester().execute_with(|| {
		assert_ok!(EthereumBeaconClient::force_checkpoint(
			RuntimeOrigin::root(),
			checkpoint.clone(),
			ChainForkVersions::get()
		));
		let block_root: H256 = checkpoint.header.hash_tree_root().unwrap();
		assert!(<FinalizedBeaconState<Test>>::contains_key(block_root));
	});
}

#[test]
fn process_initial_checkpoint_with_invalid_sync_committee_proof() {
	let mut checkpoint = Box::new(load_checkpoint_update_fixture());
	checkpoint.current_sync_committee_branch[0] = TEST_HASH.into();

	new_tester().execute_with(|| {
		assert_err!(
			EthereumBeaconClient::force_checkpoint(
				RuntimeOrigin::root(),
				checkpoint,
				ChainForkVersions::get()
			),
			Error::<Test>::InvalidSyncCommitteeMerkleProof
		);
	});
}

#[test]
fn submit_update_in_current_period() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let update = Box::new(load_finalized_header_update_fixture());
	let initial_period = compute_period(checkpoint.header.slot);
	let update_period = compute_period(update.finalized_header.slot);
	assert_eq!(initial_period, update_period);

	new_tester().execute_with(|| {
		assert_ok!(EthereumBeaconClient::process_checkpoint_update(
			&checkpoint,
			&ChainForkVersions::get()
		));
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update.clone());
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);
		let block_root: H256 = update.finalized_header.hash_tree_root().unwrap();
		assert!(<FinalizedBeaconState<Test>>::contains_key(block_root));
	});
}

#[test]
fn submit_update_with_sync_committee_in_current_period() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let update = Box::new(load_sync_committee_update_fixture());
	let execution_proof = anchor_execution_proof();
	let block_hash = execution_proof.execution_header.block_hash();
	let init_period = compute_period(checkpoint.header.slot);
	let update_period = compute_period(update.finalized_header.slot);
	assert_eq!(init_period, update_period);

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		assert!(!<NextSyncCommittee<Test>>::exists());
		assert!(!ExecutionHeaderAnchors::<Test>::contains_key(block_hash));
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);
		assert!(<NextSyncCommittee<Test>>::exists());
		assert!(!ExecutionHeaderAnchors::<Test>::contains_key(block_hash));
	});
}

#[test]
fn reject_submit_update_in_next_period() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let sync_committee_update = Box::new(load_sync_committee_update_fixture());
	let update = Box::new(load_next_finalized_header_update_fixture());
	let sync_committee_period = compute_period(sync_committee_update.finalized_header.slot);
	let next_sync_committee_period = compute_period(update.finalized_header.slot);
	assert_eq!(sync_committee_period + 1, next_sync_committee_period);
	let next_sync_committee_update = Box::new(load_next_sync_committee_update_fixture());

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), sync_committee_update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);

		// check an update in the next period is rejected
		let second_result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update.clone());
		assert_err!(second_result, Error::<Test>::SyncCommitteeUpdateRequired);
		assert_eq!(second_result.unwrap_err().post_info.pays_fee, Pays::Yes);

		// submit update with next sync committee
		let third_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), next_sync_committee_update);
		assert_ok!(third_result);
		assert_eq!(third_result.unwrap().pays_fee, Pays::No);
		// check same header in the next period can now be submitted successfully
		assert_ok!(EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update.clone()));
		let block_root: H256 = update.finalized_header.clone().hash_tree_root().unwrap();
		assert!(<FinalizedBeaconState<Test>>::contains_key(block_root));
	});
}

#[test]
fn submit_update_with_invalid_header_proof() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let mut update = Box::new(load_sync_committee_update_fixture());
	let init_period = compute_period(checkpoint.header.slot);
	let update_period = compute_period(update.finalized_header.slot);
	assert_eq!(init_period, update_period);
	update.finality_branch[0] = TEST_HASH.into();

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		assert!(!<NextSyncCommittee<Test>>::exists());
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_err!(result, Error::<Test>::InvalidHeaderMerkleProof);
		assert_eq!(result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_update_with_invalid_next_sync_committee_proof() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let mut update = Box::new(load_sync_committee_update_fixture());
	let init_period = compute_period(checkpoint.header.slot);
	let update_period = compute_period(update.finalized_header.slot);
	assert_eq!(init_period, update_period);
	if let Some(ref mut next_sync_committee_update) = update.next_sync_committee_update {
		next_sync_committee_update.next_sync_committee_branch[0] = TEST_HASH.into();
	}

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		assert!(!<NextSyncCommittee<Test>>::exists());
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_err!(result, Error::<Test>::InvalidSyncCommitteeMerkleProof);
		assert_eq!(result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_update_with_skipped_period() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let sync_committee_update = Box::new(load_sync_committee_update_fixture());
	let mut update = Box::new(load_next_finalized_header_update_fixture());
	update.signature_slot += (EPOCHS_PER_SYNC_COMMITTEE_PERIOD * SLOTS_PER_EPOCH) as u64;
	update.attested_header.slot = update.signature_slot - 1;

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), sync_committee_update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);

		let second_result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_err!(second_result, Error::<Test>::SkippedSyncCommitteePeriod);
		assert_eq!(second_result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_update_with_sync_committee_in_next_period() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let update = Box::new(load_sync_committee_update_fixture());
	let next_update = Box::new(load_next_sync_committee_update_fixture());
	let update_period = compute_period(update.finalized_header.slot);
	let next_update_period = compute_period(next_update.finalized_header.slot);
	assert_eq!(update_period + 1, next_update_period);

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		assert!(!<NextSyncCommittee<Test>>::exists());

		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);
		assert!(<NextSyncCommittee<Test>>::exists());

		let second_result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), next_update);
		assert_ok!(second_result);
		assert_eq!(second_result.unwrap().pays_fee, Pays::No);
		let last_finalized_state =
			FinalizedBeaconState::<Test>::get(LatestFinalizedBlockRoot::<Test>::get()).unwrap();
		let last_synced_period = compute_period(last_finalized_state.slot);
		assert_eq!(last_synced_period, next_update_period);
	});
}

#[test]
fn submit_update_with_sync_committee_invalid_signature_slot() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let mut update = Box::new(load_sync_committee_update_fixture());

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));

		// makes an invalid update with signature_slot should be more than attested_slot
		update.signature_slot = update.attested_header.slot;

		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_err!(result, Error::<Test>::InvalidUpdateSlot);
		assert_eq!(result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_update_with_skipped_sync_committee_period() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let finalized_update = Box::new(load_next_finalized_header_update_fixture());
	let checkpoint_period = compute_period(checkpoint.header.slot);
	let next_sync_committee_period = compute_period(finalized_update.finalized_header.slot);
	assert_eq!(checkpoint_period + 1, next_sync_committee_period);

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), finalized_update);
		assert_err!(result, Error::<Test>::SkippedSyncCommitteePeriod);
		assert_eq!(result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_irrelevant_update() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let mut update = Box::new(load_next_finalized_header_update_fixture());

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));

		// makes an invalid update where the attested_header slot value should be greater than the
		// checkpoint slot value
		update.finalized_header.slot = checkpoint.header.slot;
		update.attested_header.slot = checkpoint.header.slot;
		update.signature_slot = checkpoint.header.slot + 1;

		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_err!(result, Error::<Test>::IrrelevantUpdate);
		assert_eq!(result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_update_with_missing_bootstrap() {
	let update = Box::new(load_next_finalized_header_update_fixture());

	new_tester().execute_with(|| {
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_err!(result, Error::<Test>::NotBootstrapped);
		assert_eq!(result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_update_with_invalid_sync_committee_update() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let update = Box::new(load_sync_committee_update_fixture());
	let mut next_update = Box::new(load_next_sync_committee_update_fixture());

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));

		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);

		// makes update with invalid next_sync_committee
		<FinalizedBeaconState<Test>>::mutate(<LatestFinalizedBlockRoot<Test>>::get(), |x| {
			*x = Some(FinalizedBeaconHeaderState { slot: next_update.attested_header.slot });
		});
		next_update.attested_header.slot += 1;
		next_update.signature_slot = next_update.attested_header.slot + 1;
		let next_sync_committee = NextSyncCommitteeUpdate {
			next_sync_committee: Default::default(),
			next_sync_committee_branch: Default::default(),
		};
		next_update.next_sync_committee_update = Some(next_sync_committee);

		let second_result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), next_update);
		assert_err!(second_result, Error::<Test>::InvalidSyncCommitteeUpdate);
		assert_eq!(second_result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

/// Check that a gap of more than 8192 slots between finalized headers is not allowed.
#[test]
fn submit_finalized_header_update_with_too_large_gap() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let update = Box::new(load_sync_committee_update_fixture());
	let mut next_update = Box::new(load_next_sync_committee_update_fixture());

	// Adds 8193 slots, so that the next update is still in the next sync committee, but the
	// gap between the finalized headers is more than 8192 slots.
	let slot_with_large_gap = checkpoint.header.slot + SLOTS_PER_HISTORICAL_ROOT as u64 + 1;

	next_update.finalized_header.slot = slot_with_large_gap;
	// Adding some slots to the attested header and signature slot since they need to be ahead
	// of the finalized header.
	next_update.attested_header.slot = slot_with_large_gap + 33;
	next_update.signature_slot = slot_with_large_gap + 43;

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);
		assert!(<NextSyncCommittee<Test>>::exists());

		let second_result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), next_update);
		assert_err!(second_result, Error::<Test>::InvalidFinalizedHeaderGap);
		assert_eq!(second_result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

/// Check that a gap of 8192 slots between finalized headers is allowed.
#[test]
fn submit_finalized_header_update_with_gap_at_limit() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let update = Box::new(load_sync_committee_update_fixture());
	let mut next_update = Box::new(load_next_sync_committee_update_fixture());

	next_update.finalized_header.slot = checkpoint.header.slot + SLOTS_PER_HISTORICAL_ROOT as u64;
	// Adding some slots to the attested header and signature slot since they need to be ahead
	// of the finalized header.
	next_update.attested_header.slot =
		checkpoint.header.slot + SLOTS_PER_HISTORICAL_ROOT as u64 + 33;
	next_update.signature_slot = checkpoint.header.slot + SLOTS_PER_HISTORICAL_ROOT as u64 + 43;

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));

		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);
		assert!(<NextSyncCommittee<Test>>::exists());

		let second_result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), next_update);
		assert_err!(
			second_result,
			// The test should pass the InvalidFinalizedHeaderGap check, and will fail at the
			// next check, the merkle proof, because we changed the next_update slots.
			Error::<Test>::InvalidHeaderMerkleProof
		);
		assert_eq!(second_result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn duplicate_sync_committee_updates_are_not_free() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let sync_committee_update = Box::new(load_sync_committee_update_fixture());

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		let result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), sync_committee_update.clone());
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);

		// Check that if the same update is submitted, the update is not free.
		let second_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), sync_committee_update);
		assert_ok!(second_result);
		assert_eq!(second_result.unwrap().pays_fee, Pays::Yes);
	});
}

#[test]
fn sync_committee_update_for_sync_committee_already_imported_are_not_free() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let sync_committee_update = Box::new(load_sync_committee_update_fixture()); // slot 129
	let second_sync_committee_update = load_sync_committee_update_period_0(); // slot 128
	let third_sync_committee_update = load_sync_committee_update_period_0_newer_fixture(); // slot 224
	let fourth_sync_committee_update = load_sync_committee_update_period_0_older_fixture(); // slot 96
	let fith_sync_committee_update = Box::new(load_next_sync_committee_update_fixture()); // slot 8259

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		assert_eq!(<LatestSyncCommitteeUpdatePeriod<Test>>::get(), 0);

		// Check that setting the next sync committee for period 0 is free (it is not set yet).
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), sync_committee_update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);
		assert_eq!(<LatestSyncCommitteeUpdatePeriod<Test>>::get(), 0);

		// Check that setting the next sync committee for period 0 again is not free.
		let second_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), second_sync_committee_update);
		assert_eq!(second_result.unwrap().pays_fee, Pays::Yes);
		assert_eq!(<LatestSyncCommitteeUpdatePeriod<Test>>::get(), 0);

		// Check that setting an update with a sync committee that has already been set, but with a
		// newer finalized header, is free.
		let third_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), third_sync_committee_update);
		assert_eq!(third_result.unwrap().pays_fee, Pays::No);
		assert_eq!(<LatestSyncCommitteeUpdatePeriod<Test>>::get(), 0);

		// Check that setting the next sync committee for period 0 again with an earlier slot is not
		// free.
		let fourth_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), fourth_sync_committee_update);
		assert_err!(fourth_result, Error::<Test>::IrrelevantUpdate);
		assert_eq!(fourth_result.unwrap_err().post_info.pays_fee, Pays::Yes);

		// Check that setting the next sync committee for period 1 is free.
		let fith_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), fith_sync_committee_update);
		assert_eq!(fith_result.unwrap().pays_fee, Pays::No);
		assert_eq!(<LatestSyncCommitteeUpdatePeriod<Test>>::get(), 1);
	});
}

/* IMPLS */

#[test]
fn verify_message() {
	let (event_log, proof, anchor) = retained_anchor_verification_payload();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_ok!(<EthereumBeaconClient as EthereumVerifyProvider>::verify_event_log(
			&event_log, &proof,
		));
	});
}

#[test]
fn verify_message_invalid_proof() {
	let (event_log, mut proof, anchor) = retained_anchor_verification_payload();
	proof.receipt_proof.nodes[0] =
		vec![1, 2, 3].try_into().expect("tiny malformed node stays within bounded size");

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_event_log(&event_log, &proof,),
			Err(EthereumVerifyError::InvalidProof)
		);
	});
}

#[test]
fn verify_message_invalid_receipts_root() {
	let (event_log, proof, mut anchor) = retained_anchor_verification_payload();
	anchor.receipts_root = TEST_HASH.into();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_event_log(&event_log, &proof,),
			Err(EthereumVerifyError::InvalidProof)
		);
	});
}

#[test]
fn verify_message_invalid_log() {
	let (mut event_log, proof, anchor) = retained_anchor_verification_payload();
	event_log.topics[0] = H256::zero();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_event_log(&event_log, &proof,),
			Err(EthereumVerifyError::LogNotFound)
		);
	});
}

#[test]
fn verify_message_receipt_does_not_contain_log() {
	let (mut event_log, proof, anchor) = retained_anchor_verification_payload();
	event_log.data = hex!("f9013c94ee9170abfbf9421ad6dd07f6bdec9d89f2b581e0f863a01b11dcf133cc240f682dab2d3a8e4cd35c5da8c9cf99adac4336f8512584c5ada000000000000000000000000000000000000000000000000000000000000003e8a00000000000000000000000000000000000000000000000000000000000000002b8c000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000068000f000000000000000101d184c103f7acc340847eee82a0b909e3358bc28d440edffa1352b13227e8ee646f3ea37456dec70100000101001cbd2d43530a44705ad088af313e18f80b53ef16b36177cd4b77b846f2a5f07c0000e8890423c78a0000000000000000000000000000000000000000000000000000000000000000")
		.to_vec()
		.try_into()
		.expect("mutated log payload stays within bounded log data size");

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_event_log(&event_log, &proof,),
			Err(EthereumVerifyError::LogNotFound)
		);
	});
}

#[test]
fn set_operating_mode() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let update = Box::new(load_finalized_header_update_fixture());

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));

		assert_ok!(EthereumBeaconClient::set_operating_mode(
			RuntimeOrigin::root(),
			BasicOperatingMode::Halted
		));

		assert_noop!(
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update),
			Error::<Test>::Halted
		);
	});
}

#[test]
fn set_operating_mode_root_only() {
	new_tester().execute_with(|| {
		assert_noop!(
			EthereumBeaconClient::set_operating_mode(
				RuntimeOrigin::signed(1),
				BasicOperatingMode::Halted
			),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn verify_execution_proof_invalid_execution_header_proof() {
	let mut execution_header_update = anchor_execution_proof();
	execution_header_update.execution_branch[0] = TEST_HASH.into();

	new_tester().execute_with(|| {
		assert_ok!(EthereumBeaconClient::store_finalized_header(execution_header_update.header));
		assert_err!(
			EthereumBeaconClient::import_execution_header_anchor(
				RuntimeOrigin::signed(1),
				execution_header_update,
			),
			Error::<Test>::InvalidExecutionHeaderProof
		);
	});
}

#[test]
fn import_execution_header_anchor_requires_matching_stored_finalized_header() {
	let execution_header_update = anchor_execution_proof();
	let mismatched_slot_execution_header_update = anchor_execution_proof();

	new_tester().execute_with(|| {
		assert_err!(
			EthereumBeaconClient::import_execution_header_anchor(
				RuntimeOrigin::signed(1),
				execution_header_update,
			),
			Error::<Test>::ExpectedFinalizedHeaderNotStored
		);

		let block_root: H256 =
			mismatched_slot_execution_header_update.header.hash_tree_root().unwrap();

		<FinalizedBeaconState<Test>>::insert(
			block_root,
			FinalizedBeaconHeaderState {
				slot: mismatched_slot_execution_header_update.header.slot + 1,
			},
		);

		assert_err!(
			EthereumBeaconClient::import_execution_header_anchor(
				RuntimeOrigin::signed(1),
				mismatched_slot_execution_header_update,
			),
			Error::<Test>::ExpectedFinalizedHeaderNotStored
		);
	});
}

#[test]
fn verify_message_invalid_topic() {
	let (event_log, proof, anchor) = retained_anchor_verification_payload();
	let mut event_log_muted = event_log.clone();
	event_log_muted.topics[0] = H256::default();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_event_log(
				&event_log_muted,
				&proof,
			),
			Err(EthereumVerifyError::LogNotFound)
		);
	});
}

#[test]
fn signing_root_uses_previous_slot_for_fork_version() {
	new_tester().execute_with(|| {
		ForkVersionSchedule::<Test>::put(ChainForkVersions::get());

		// Use a signature_slot at a fork boundary (first slot of the fulu epoch).
		// In mock.rs: electra.epoch = 0, fulu.epoch = 100000000
		let fulu_epoch = ChainForkVersions::get().fulu.epoch;
		let signature_slot: u64 = fulu_epoch * (SLOTS_PER_EPOCH as u64);

		// Verify this is the first slot of the epoch
		assert_eq!(signature_slot % (SLOTS_PER_EPOCH as u64), 0);

		let header = BeaconHeader {
			slot: signature_slot - 1,
			proposer_index: 0,
			parent_root: H256::repeat_byte(0x11),
			state_root: H256::repeat_byte(0x22),
			body_root: H256::repeat_byte(0x33),
		};

		let validators_root = H256::repeat_byte(0x44);

		// Get fork versions for comparison
		let fork_version_at_signature_slot = EthereumBeaconClient::compute_fork_version(
			compute_epoch(signature_slot, SLOTS_PER_EPOCH as u64),
		)
		.unwrap();
		let fork_version_at_previous_slot = EthereumBeaconClient::compute_fork_version(
			compute_epoch(signature_slot.saturating_sub(1), SLOTS_PER_EPOCH as u64),
		)
		.unwrap();

		// At the fork boundary, these should differ
		assert_ne!(
			fork_version_at_signature_slot, fork_version_at_previous_slot,
			"Test setup error: fork versions should differ at fork boundary"
		);

		// Compute signing roots using both fork versions
		let domain_type = crate::config::DOMAIN_SYNC_COMMITTEE.to_vec();

		let domain_with_previous_slot = EthereumBeaconClient::compute_domain(
			domain_type.clone(),
			fork_version_at_previous_slot,
			validators_root,
		)
		.unwrap();

		let signing_root_with_previous_slot =
			EthereumBeaconClient::compute_signing_root(&header, domain_with_previous_slot).unwrap();

		// The pallet's signing_root should use the previous slot's fork version (per spec)
		let pallet_signing_root =
			EthereumBeaconClient::signing_root(&header, validators_root, signature_slot).unwrap();

		assert_eq!(
			pallet_signing_root, signing_root_with_previous_slot,
			"signing_root should use fork version from signature_slot - 1"
		);
	});
}

#[test]
fn signing_root_handles_signature_slot_zero() {
	// Per spec: fork_version_slot = max(signature_slot, 1) - 1
	// When signature_slot = 0, saturating_sub(1) = 0, which matches max(0, 1) - 1 = 0
	new_tester().execute_with(|| {
		ForkVersionSchedule::<Test>::put(ChainForkVersions::get());

		let header = BeaconHeader {
			slot: 0,
			proposer_index: 0,
			parent_root: H256::repeat_byte(0x11),
			state_root: H256::repeat_byte(0x22),
			body_root: H256::repeat_byte(0x33),
		};

		let validators_root = H256::repeat_byte(0x44);

		// Should not panic and should use epoch 0 fork version
		let result = EthereumBeaconClient::signing_root(&header, validators_root, 0);
		assert!(result.is_ok(), "signing_root should handle signature_slot = 0");
	});
}

/* ARGON RETAINED ANCHOR TESTS */

#[test]
fn submit_duplicate_update_pays_fee() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let update = Box::new(load_finalized_header_update_fixture());
	let duplicate_update = Box::new(load_finalized_header_update_fixture());

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);

		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), duplicate_update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_pool_key_uses_exact_update_payload() {
	let update = Box::new(load_finalized_header_update_fixture());
	let mut different_proof_update = load_finalized_header_update_fixture();
	different_proof_update.signature_slot += 1;

	let first_call =
		RuntimeCall::EthereumBeaconClient(crate::Call::<Test>::submit { update: update.clone() });
	let retry_call =
		RuntimeCall::EthereumBeaconClient(crate::Call::<Test>::submit { update: update.clone() });
	let different_proof_call = RuntimeCall::EthereumBeaconClient(crate::Call::<Test>::submit {
		update: Box::new(different_proof_update),
	});

	let first_key = <EthereumBeaconClient as CallTxPoolKeyProvider<RuntimeCall, u64>>::key_for(
		&first_call,
		Some(&1),
	)
	.expect("submit should publish a pool key");
	let retry_key = <EthereumBeaconClient as CallTxPoolKeyProvider<RuntimeCall, u64>>::key_for(
		&retry_call,
		Some(&99),
	)
	.expect("submit retry should publish a pool key");
	let different_proof_key =
		<EthereumBeaconClient as CallTxPoolKeyProvider<RuntimeCall, u64>>::key_for(
			&different_proof_call,
			Some(&1),
		)
		.expect("submit with different proof should publish a pool key");

	assert_eq!(first_key, retry_key);
	assert_ne!(first_key, different_proof_key);
	assert_eq!(
		first_key,
		(b"ethereum_verifier:submit".as_slice(), update.using_encoded(blake2_256),)
			.using_encoded(blake2_256)
			.to_vec()
	);
}

#[test]
fn import_execution_header_anchor_imports_after_beacon_update() {
	let execution_proof = anchor_execution_proof();
	let duplicate_execution_proof = anchor_execution_proof();
	let block_hash = execution_proof.execution_header.block_hash();
	let block_number = execution_proof.execution_header.block_number();

	new_tester().execute_with(|| {
		assert_ok!(EthereumBeaconClient::store_finalized_header(execution_proof.header));

		let result = EthereumBeaconClient::import_execution_header_anchor(
			RuntimeOrigin::signed(1),
			execution_proof,
		);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::Yes);

		let anchor = ExecutionHeaderAnchors::<Test>::get(block_hash).unwrap();
		assert_eq!(anchor.block_hash, block_hash);
		assert_eq!(anchor.block_number, block_number);

		let result = EthereumBeaconClient::import_execution_header_anchor(
			RuntimeOrigin::signed(1),
			duplicate_execution_proof,
		);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::Yes);
	});
}

#[test]
fn execution_header_anchor_pool_key_uses_exact_proof_payload() {
	let execution_proof = anchor_execution_proof();
	let mut different_proof = execution_proof.clone();
	different_proof.header.slot += 1;

	let first_call =
		RuntimeCall::EthereumBeaconClient(crate::Call::<Test>::import_execution_header_anchor {
			execution_proof: execution_proof.clone(),
		});
	let retry_call =
		RuntimeCall::EthereumBeaconClient(crate::Call::<Test>::import_execution_header_anchor {
			execution_proof: execution_proof.clone(),
		});
	let different_proof_call =
		RuntimeCall::EthereumBeaconClient(crate::Call::<Test>::import_execution_header_anchor {
			execution_proof: different_proof,
		});

	let first_key = <EthereumBeaconClient as CallTxPoolKeyProvider<RuntimeCall, u64>>::key_for(
		&first_call,
		Some(&1),
	)
	.expect("execution anchor should publish a pool key");
	let retry_key = <EthereumBeaconClient as CallTxPoolKeyProvider<RuntimeCall, u64>>::key_for(
		&retry_call,
		Some(&99),
	)
	.expect("execution anchor retry should publish a pool key");
	let different_proof_key =
		<EthereumBeaconClient as CallTxPoolKeyProvider<RuntimeCall, u64>>::key_for(
			&different_proof_call,
			Some(&1),
		)
		.expect("execution anchor with different proof should publish a pool key");

	assert_eq!(first_key, retry_key);
	assert_ne!(first_key, different_proof_key);
	assert_eq!(
		first_key,
		(
			b"ethereum_verifier:execution_header_anchor".as_slice(),
			execution_proof.execution_header.block_hash(),
			execution_proof.using_encoded(blake2_256),
		)
			.using_encoded(blake2_256)
			.to_vec()
	);
}

#[test]
fn verify_event_log_availability_gate() {
	let (event_log, proof, anchor) = retained_anchor_verification_payload();
	let unavailable_proof = EthereumProof {
		execution_block_proof: EthereumExecutionBlockProof {
			anchor_block_hash: H256::repeat_byte(1),
			target_to_anchor_header_chain: Vec::new()
				.try_into()
				.expect("empty header chain stays within bounds"),
		},
		receipt_proof: EthereumReceiptProof {
			transaction_index: 0,
			nodes: Vec::new()
				.try_into()
				.expect("empty receipt proof stays within bounded node count"),
		},
	};

	new_tester().execute_with(|| {
		EventLogVerifierEnabled::set(true);
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);

		assert_ok!(EthereumBeaconClient::verify_event_log(event_log, proof,));
		EventLogVerifierEnabled::set(false);

		assert_eq!(
			EthereumBeaconClient::verify_event_log(
				EthereumLog {
					address: Default::default(),
					topics: Vec::new().try_into().expect("empty topics stay within bounds"),
					data: Vec::new().try_into().expect("empty data stays within bounds"),
				},
				unavailable_proof,
			),
			Err(EthereumVerifyError::VerifierUnavailable)
		);
	});
}

#[test]
fn ethereum_log_decode_rejects_oversized_topics_and_data() {
	#[derive(Encode)]
	struct UnboundedEthereumLog {
		address: H160,
		topics: Vec<H256>,
		data: Vec<u8>,
	}

	let oversized_topics = UnboundedEthereumLog {
		address: H160::zero(),
		topics: vec![H256::zero(); (MAX_ETHEREUM_LOG_TOPICS + 1) as usize],
		data: Vec::new(),
	}
	.encode();
	assert!(EthereumLog::decode(&mut &oversized_topics[..]).is_err());

	let oversized_data = UnboundedEthereumLog {
		address: H160::zero(),
		topics: Vec::new(),
		data: vec![0u8; (MAX_ETHEREUM_LOG_DATA_BYTES + 1) as usize],
	}
	.encode();
	assert!(EthereumLog::decode(&mut &oversized_data[..]).is_err());
}

#[test]
fn ethereum_receipt_proof_decode_rejects_oversized_nodes() {
	#[derive(Encode)]
	struct UnboundedEthereumReceiptProof {
		#[codec(compact)]
		transaction_index: u64,
		nodes: Vec<Vec<u8>>,
	}

	let oversized_node = UnboundedEthereumReceiptProof {
		transaction_index: 0,
		nodes: vec![vec![0u8; (MAX_ETHEREUM_RECEIPT_PROOF_NODE_BYTES + 1) as usize]],
	}
	.encode();
	assert!(EthereumReceiptProof::decode(&mut &oversized_node[..]).is_err());

	let oversized_node_count = UnboundedEthereumReceiptProof {
		transaction_index: 0,
		nodes: vec![vec![0u8]; (MAX_ETHEREUM_RECEIPT_PROOF_NODES + 1) as usize],
	}
	.encode();
	assert!(EthereumReceiptProof::decode(&mut &oversized_node_count[..]).is_err());
}

#[test]
fn execution_block_and_anchor_proof_decode_reject_oversized_vectors() {
	#[derive(Clone, Encode)]
	struct UnboundedEthereumExecutionHeader {
		rlp: Vec<u8>,
	}

	#[derive(Encode)]
	struct UnboundedEthereumExecutionBlockProof {
		anchor_block_hash: H256,
		target_to_anchor_header_chain: Vec<UnboundedEthereumExecutionHeader>,
	}

	#[derive(Encode)]
	struct UnboundedExecutionProof {
		header: BeaconHeader,
		execution_header: snowbridge_beacon_primitives::VersionedExecutionPayloadHeader,
		execution_branch: Vec<H256>,
	}

	let oversized_header_rlp = UnboundedEthereumExecutionHeader {
		rlp: vec![0u8; (MAX_ETHEREUM_EXECUTION_HEADER_RLP_BYTES + 1) as usize],
	};
	let oversized_rlp = UnboundedEthereumExecutionBlockProof {
		anchor_block_hash: H256::zero(),
		target_to_anchor_header_chain: vec![oversized_header_rlp],
	}
	.encode();
	assert!(EthereumExecutionBlockProof::decode(&mut &oversized_rlp[..]).is_err());

	let oversized_header_count = UnboundedEthereumExecutionBlockProof {
		anchor_block_hash: H256::zero(),
		target_to_anchor_header_chain: vec![
			UnboundedEthereumExecutionHeader { rlp: vec![0u8] };
			(MAX_ETHEREUM_HEADER_CHAIN_LEN + 1) as usize
		],
	}
	.encode();
	assert!(EthereumExecutionBlockProof::decode(&mut &oversized_header_count[..]).is_err());

	let proof = anchor_execution_proof();
	let oversized_branch = UnboundedExecutionProof {
		header: proof.header,
		execution_header: proof.execution_header,
		execution_branch: vec![H256::zero(); crate::config::MAX_BRANCH_PROOF_SIZE + 1],
	}
	.encode();
	assert!(ExecutionProof::decode(&mut &oversized_branch[..]).is_err());
}

#[test]
fn verify_execution_block_proof_accepts_header_chain_to_anchor() {
	let target_receipts_root = H256::repeat_byte(7);
	let (target_header, target_block_hash) =
		make_execution_header(100, H256::repeat_byte(1), target_receipts_root);
	let anchor_block_hash = H256::repeat_byte(9);
	let anchor = ExecutionHeaderAnchor {
		block_number: 101,
		block_hash: anchor_block_hash,
		parent_hash: target_block_hash,
		receipts_root: H256::repeat_byte(8),
	};
	let proof = EthereumExecutionBlockProof {
		anchor_block_hash,
		target_to_anchor_header_chain: vec![target_header]
			.try_into()
			.expect("single-header chain stays within bounded header chain length"),
	};

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor_block_hash, anchor);
		assert_eq!(
			EthereumBeaconClient::verify_execution_block_proof(&proof),
			Ok(target_receipts_root)
		);
	});
}

#[test]
fn verify_execution_block_proof_rejects_invalid_client_headers() {
	let (target_header, _target_block_hash) =
		make_execution_header(100, H256::repeat_byte(1), H256::repeat_byte(7));
	let mut malformed_header = target_header.clone();
	malformed_header
		.rlp
		.try_push(0)
		.expect("malformed header stays within bounded RLP size");
	let anchor_block_hash = H256::repeat_byte(9);
	let anchor = ExecutionHeaderAnchor {
		block_number: 101,
		block_hash: anchor_block_hash,
		parent_hash: H256::repeat_byte(2),
		receipts_root: H256::repeat_byte(8),
	};
	let proof = EthereumExecutionBlockProof {
		anchor_block_hash,
		target_to_anchor_header_chain: vec![target_header]
			.try_into()
			.expect("single-header chain stays within bounded header chain length"),
	};

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor_block_hash, anchor);
		assert_eq!(
			EthereumBeaconClient::verify_execution_block_proof(&EthereumExecutionBlockProof {
				anchor_block_hash,
				target_to_anchor_header_chain: vec![malformed_header]
					.try_into()
					.expect("single malformed header stays within bounded chain length"),
			}),
			Err(EthereumVerifyError::InvalidHeader)
		);
		assert_eq!(
			EthereumBeaconClient::verify_execution_block_proof(&proof),
			Err(EthereumVerifyError::InvalidHeaderChain)
		);
	});
}
