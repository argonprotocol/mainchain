// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
pub use crate::mock::*;
use crate::{
	fixture_conversions::execution_proof_from_fixture,
	functions::{compute_epoch, compute_period as raw_compute_period},
	mock::{
		load_checkpoint_update_fixture, load_execution_proof_fixture,
		load_finalized_header_update_fixture, load_next_finalized_header_update_fixture,
		load_next_sync_committee_update_fixture, load_sync_committee_update_fixture,
	},
	storage_proof_fixture::load_account_storage_proof_fixture as load_storage_proof_fixture,
	sync_committee_sum,
	types::{CheckpointUpdate, ExecutionHeaderProof, Update},
	verify_merkle_branch, BasicOperatingMode, BeaconHeader, Error, ExecutionHeaderAnchor,
	ExecutionHeaderAnchors, ExecutionHeaderAnchorsByBlockNumber, ExecutionProof,
	FinalizedBeaconHeaderState, FinalizedBeaconState, Fork, ForkVersionSchedule, ForkVersions,
	LatestExecutionHeaderAnchorBlockHash, LatestFinalizedBlockRoot,
	LatestSyncCommitteeUpdatePeriod, NextSyncCommittee, SyncCommitteePrepared,
};
use alloy_consensus::{Header as AlloyHeader, Receipt, ReceiptEnvelope};
use alloy_primitives::{Address, Bytes, Log, B256};
use alloy_rlp::Encodable;
use alloy_trie::{proof::ProofRetainer, HashBuilder, Nibbles};
use argon_primitives::{
	ethereum::{
		EthereumReceiptLogProofBatch, EthereumReceiptLogProofBlock,
		MAX_ETHEREUM_COMBINED_RECEIPT_PROOF_NODES, MAX_ETHEREUM_EXECUTION_HEADER_RLP_BYTES,
		MAX_ETHEREUM_HEADER_CHAIN_LEN, MAX_ETHEREUM_LOG_DATA_BYTES, MAX_ETHEREUM_LOG_TOPICS,
		MAX_ETHEREUM_RECEIPTS_PER_PROOF, MAX_ETHEREUM_RECEIPT_PROOF_NODE_BYTES,
		MAX_ETHEREUM_RECEIPT_PROOF_NODE_REFS,
	},
	CallTxPoolKeyProvider, EthereumAccountStorageProof, EthereumBeaconPreset,
	EthereumCombinedReceiptProof, EthereumExecutionBlockProof, EthereumExecutionHeader,
	EthereumLog, EthereumReceiptLog, EthereumReceiptProofReceipt, EthereumVerifyError,
	EthereumVerifyProvider,
};
use codec::{Decode, Encode};
use hex_literal::hex;
use polkadot_sdk::{
	frame_support::{
		assert_err, assert_noop, assert_ok, dispatch::DispatchResult, pallet_prelude::Pays,
	},
	sp_core::{hashing::blake2_256, H160, H256},
	sp_runtime::{traits::ConstU32, DispatchError},
};
use snowbridge_beacon_primitives::merkle_proof::{generalized_index_length, subtree_index};
use std::{fs::File, path::PathBuf};

const MAINNET_SLOTS_PER_EPOCH: u64 = EthereumBeaconPreset::Mainnet.slots_per_epoch() as u64;
const MAINNET_EPOCHS_PER_SYNC_COMMITTEE_PERIOD: u64 =
	EthereumBeaconPreset::Mainnet.epochs_per_sync_committee_period() as u64;
const MAINNET_SLOTS_PER_HISTORICAL_ROOT: u64 =
	EthereumBeaconPreset::Mainnet.slots_per_historical_root() as u64;
const MAINNET_SYNC_COMMITTEE_SIZE: usize = EthereumBeaconPreset::Mainnet.sync_committee_size();
const MAINNET_SYNC_COMMITTEE_BITS_SIZE: usize =
	EthereumBeaconPreset::Mainnet.sync_committee_bits_size();
const MINIMAL_SYNC_COMMITTEE_SIZE: usize = EthereumBeaconPreset::Minimal.sync_committee_size();

/// Arbitrary hash used for tests and invalid hashes.
const TEST_HASH: [u8; 32] =
	hex!["5f6f02af29218292d21a69b64a794a7c0873b3e0f54611972863706e8cbdf371"];

fn anchor_execution_proof() -> ExecutionProof {
	execution_proof_from_fixture(load_execution_proof_fixture())
		.expect("execution proof fixture stays within bounded branch size")
}

fn attested_header(update: &Update) -> &BeaconHeader {
	&update.attested_header
}

fn attested_header_mut(update: &mut Update) -> &mut BeaconHeader {
	&mut update.attested_header
}

fn finalized_header(update: &Update) -> &BeaconHeader {
	&update.finalized_header
}

fn finalized_header_mut(update: &mut Update) -> &mut BeaconHeader {
	&mut update.finalized_header
}

fn signature_slot(update: &Update) -> u64 {
	update.signature_slot
}

fn signature_slot_mut(update: &mut Update) -> &mut u64 {
	&mut update.signature_slot
}

fn next_sync_committee_update(update: &Update) -> &crate::types::NextSyncCommitteeUpdate {
	update
		.next_sync_committee_update
		.as_ref()
		.expect("fixture should carry the next sync committee witness")
}

fn next_sync_committee_update_mut(
	update: &mut Update,
) -> &mut crate::types::NextSyncCommitteeUpdate {
	update
		.next_sync_committee_update
		.as_mut()
		.expect("fixture should carry the next sync committee witness")
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

#[derive(serde::Deserialize)]
struct MinimalBootstrapFixture {
	header: BeaconHeader,
	current_sync_committee:
		snowbridge_beacon_primitives::SyncCommittee<{ MINIMAL_SYNC_COMMITTEE_SIZE }>,
	current_sync_committee_branch: Vec<H256>,
}

fn load_minimal_bootstrap_fixture(basename: &str) -> MinimalBootstrapFixture {
	let filepath: PathBuf =
		[env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", basename].iter().collect();
	serde_json::from_reader(File::open(filepath).unwrap()).unwrap()
}

fn load_account_storage_proof_fixture(
) -> (H160, ExecutionHeaderAnchor, EthereumAccountStorageProof<ConstU32<2>>) {
	let fixture = load_storage_proof_fixture();
	let proof = EthereumAccountStorageProof::<ConstU32<2>> {
		anchor_block_hash: fixture.anchor.block_hash,
		account_proof: fixture.account_proof,
		storage_proof: fixture.storage_proof,
		slots: vec![fixture.range_slot, fixture.root_slot]
			.try_into()
			.expect("fixture storage slots stay within bounded slot count"),
	};

	(fixture.gateway_address, fixture.anchor, proof)
}

fn compute_period(slot: u64) -> u64 {
	raw_compute_period(slot, MAINNET_SLOTS_PER_EPOCH, MAINNET_EPOCHS_PER_SYNC_COMMITTEE_PERIOD)
}

fn single_receipt_log_proof_batch(
	receipt_log: EthereumReceiptLog,
	execution_block_proof: EthereumExecutionBlockProof,
	receipt_proof: EthereumCombinedReceiptProof,
) -> EthereumReceiptLogProofBatch<ConstU32<1>, ConstU32<1>> {
	EthereumReceiptLogProofBatch {
		execution_block_proof,
		blocks: vec![EthereumReceiptLogProofBlock {
			target_block_number: 100,
			receipt_proof,
			receipt_logs: vec![receipt_log]
				.try_into()
				.expect("single receipt log stays within bounded log count"),
		}]
		.try_into()
		.expect("single proof block stays within bounded block count"),
	}
}

fn retained_anchor_verification_payload() -> (
	EthereumReceiptLog,
	EthereumReceiptLogProofBatch<ConstU32<1>, ConstU32<1>>,
	ExecutionHeaderAnchor,
) {
	let inbound_fixture = snowbridge_pallet_ethereum_client_fixtures::make_inbound_fixture();
	let anchor_block_hash = H256::repeat_byte(9);
	let anchor = ExecutionHeaderAnchor {
		block_number: 100,
		timestamp_millis: 0,
		block_hash: anchor_block_hash,
		parent_hash: H256::repeat_byte(8),
		state_root: H256::zero(),
		receipts_root: inbound_fixture.event.proof.execution_proof.execution_header.receipts_root(),
	};
	let event_log = EthereumReceiptLog {
		transaction_index: INBOUND_FIXTURE_RECEIPT_INDEX,
		event_log: EthereumLog {
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
		},
	};
	let receipt_proof_nodes = inbound_fixture
		.event
		.proof
		.receipt_proof
		.into_iter()
		.map(|node| node.try_into().expect("fixture receipt proof node stays within bounded size"))
		.collect::<Vec<_>>();
	let receipt_proof_node_indexes =
		(0..receipt_proof_nodes.len()).map(|index| index as u16).collect::<Vec<_>>();
	let proof_batch = single_receipt_log_proof_batch(
		event_log.clone(),
		EthereumExecutionBlockProof {
			anchor_block_hash,
			target_to_anchor_header_chain: Vec::new()
				.try_into()
				.expect("empty header chain stays within bounds"),
		},
		EthereumCombinedReceiptProof {
			nodes: receipt_proof_nodes
				.try_into()
				.expect("fixture receipt proof stays within bounded node count"),
			receipts: vec![EthereumReceiptProofReceipt {
				transaction_index: INBOUND_FIXTURE_RECEIPT_INDEX,
				node_indexes: receipt_proof_node_indexes
					.try_into()
					.expect("fixture node indexes stay within bounded receipt proof refs"),
			}]
			.try_into()
			.expect("fixture receipt proof stays within bounded receipt count"),
		},
	);

	(event_log, proof_batch, anchor)
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
	let finalized_update = Box::new(load_finalized_header_update_fixture());
	let sync_committee_update = Box::new(load_sync_committee_update_fixture());
	new_tester().execute_with(|| {
		let free_headers_interval: u64 = crate::mock::FREE_SLOTS_INTERVAL as u64;
		// Not free, smaller than the allowed free header interval
		assert_eq!(
			EthereumBeaconClient::check_refundable(
				&finalized_update.clone(),
				finalized_header(&finalized_update).slot + free_headers_interval
			),
			Pays::Yes
		);
		// Is free, larger than the minimum interval
		assert_eq!(
			EthereumBeaconClient::check_refundable(
				&finalized_update,
				finalized_header(&finalized_update).slot - (free_headers_interval + 2)
			),
			Pays::No
		);
		// Is free, valid sync committee update
		assert_eq!(
			EthereumBeaconClient::check_refundable(
				&sync_committee_update,
				finalized_header(&finalized_update).slot
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
	let participation = snowbridge_beacon_primitives::decompress_sync_committee_bits::<
		MAINNET_SYNC_COMMITTEE_SIZE,
		MAINNET_SYNC_COMMITTEE_BITS_SIZE,
	>(bits);
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
	let sync_committee_prepared: SyncCommitteePrepared = next_sync_committee_update(&update)
		.next_sync_committee
		.prepare(EthereumBeaconPreset::Mainnet)
		.unwrap();

	new_tester().execute_with(|| {
		let pubkeys = EthereumBeaconClient::find_pubkeys(
			&participation,
			sync_committee_prepared.pubkeys.as_ref(),
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
	let sync_committee_prepared: SyncCommitteePrepared = next_sync_committee_update(&update)
		.next_sync_committee
		.prepare(EthereumBeaconPreset::Mainnet)
		.unwrap();

	new_tester().execute_with(|| {
		let pubkeys = EthereumBeaconClient::find_pubkeys(
			&participation,
			sync_committee_prepared.pubkeys.as_ref(),
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
fn process_initial_checkpoint_with_invalid_execution_header_proof() {
	let mut checkpoint = Box::new(load_checkpoint_update_fixture());
	checkpoint.execution_header_proof.execution_branch[0] = TEST_HASH.into();

	new_tester().execute_with(|| {
		assert_err!(
			EthereumBeaconClient::force_checkpoint(
				RuntimeOrigin::root(),
				checkpoint,
				ChainForkVersions::get()
			),
			Error::<Test>::InvalidExecutionHeaderProof
		);
	});
}

#[test]
fn import_trusted_execution_header_backfill_uses_finalized_header_retention() {
	let evicted_root = H256::repeat_byte(0x44);
	let evicted_anchor = ExecutionHeaderAnchor {
		block_number: 100,
		timestamp_millis: 100_000,
		block_hash: H256::repeat_byte(0x45),
		parent_hash: H256::repeat_byte(0x46),
		state_root: H256::repeat_byte(0x47),
		receipts_root: H256::repeat_byte(0x48),
	};
	let historical_checkpoint = Box::new(load_checkpoint_update_fixture());
	let latest_checkpoint = Box::new(load_later_checkpoint_update_fixture());
	let historical_root = historical_checkpoint.header.hash_tree_root().unwrap();
	let latest_root = latest_checkpoint.header.hash_tree_root().unwrap();
	let historical_anchor = ExecutionHeaderAnchor::from_payload_header(
		&historical_checkpoint.execution_header_proof.execution_header,
	);

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&latest_checkpoint));
		let current_committee_before = <crate::CurrentSyncCommittee<Test>>::get();
		let latest_anchor_hash_before = <LatestExecutionHeaderAnchorBlockHash<Test>>::get();

		crate::FinalizedBeaconStateIndex::<Test>::set(0);
		crate::FinalizedBeaconStateMapping::<Test>::insert(1, evicted_root);
		crate::FinalizedBeaconState::<Test>::insert(
			evicted_root,
			FinalizedBeaconHeaderState {
				slot: historical_checkpoint.header.slot.saturating_sub(32),
			},
		);
		crate::FinalizedExecutionHeaderAnchor::<Test>::insert(
			evicted_root,
			evicted_anchor.block_hash,
		);
		ExecutionHeaderAnchors::<Test>::insert(evicted_anchor.block_hash, evicted_anchor);
		ExecutionHeaderAnchorsByBlockNumber::<Test>::insert(
			evicted_anchor.block_number.to_be_bytes(),
			evicted_anchor,
		);

		assert_ok!(EthereumBeaconClient::import_trusted_execution_header_backfill(
			RuntimeOrigin::root(),
			historical_root,
			historical_checkpoint.header,
			historical_checkpoint.execution_header_proof.clone(),
		));

		assert_eq!(crate::FinalizedBeaconStateIndex::<Test>::get(), 1);
		assert_eq!(crate::FinalizedBeaconStateMapping::<Test>::get(1), historical_root);
		assert_eq!(crate::LatestFinalizedBlockRoot::<Test>::get(), latest_root);
		assert!(<crate::CurrentSyncCommittee<Test>>::get() == current_committee_before);
		assert_eq!(<LatestExecutionHeaderAnchorBlockHash<Test>>::get(), latest_anchor_hash_before);
		assert_eq!(
			ExecutionHeaderAnchors::<Test>::get(historical_anchor.block_hash),
			Some(historical_anchor),
		);
		assert_eq!(
			ExecutionHeaderAnchorsByBlockNumber::<Test>::get(
				historical_anchor.block_number.to_be_bytes()
			),
			Some(historical_anchor),
		);
		assert_eq!(
			crate::FinalizedExecutionHeaderAnchor::<Test>::get(historical_root),
			Some(historical_anchor.block_hash),
		);
		assert_eq!(
			crate::FinalizedBeaconState::<Test>::get(historical_root),
			Some(FinalizedBeaconHeaderState { slot: historical_checkpoint.header.slot }),
		);
		assert_eq!(crate::FinalizedBeaconState::<Test>::get(evicted_root), None);
		assert_eq!(ExecutionHeaderAnchors::<Test>::get(evicted_anchor.block_hash), None,);
		assert_eq!(
			ExecutionHeaderAnchorsByBlockNumber::<Test>::get(
				evicted_anchor.block_number.to_be_bytes()
			),
			None,
		);
	});
}

#[test]
fn import_trusted_execution_header_backfill_rejects_mismatched_beacon_root() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&Box::new(load_later_checkpoint_update_fixture())));

		assert_err!(
			EthereumBeaconClient::import_trusted_execution_header_backfill(
				RuntimeOrigin::root(),
				H256::repeat_byte(0xaa),
				checkpoint.header,
				checkpoint.execution_header_proof.clone(),
			),
			Error::<Test>::InvalidBackfillHeaderRoot,
		);
	});
}

#[test]
fn import_trusted_execution_header_backfill_rejects_duplicate_anchor() {
	let historical_checkpoint = Box::new(load_checkpoint_update_fixture());
	let latest_checkpoint = Box::new(load_later_checkpoint_update_fixture());
	let historical_root = historical_checkpoint.header.hash_tree_root().unwrap();

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&latest_checkpoint));
		assert_ok!(EthereumBeaconClient::import_trusted_execution_header_backfill(
			RuntimeOrigin::root(),
			historical_root,
			historical_checkpoint.header,
			historical_checkpoint.execution_header_proof.clone(),
		));

		assert_err!(
			EthereumBeaconClient::import_trusted_execution_header_backfill(
				RuntimeOrigin::root(),
				historical_root,
				historical_checkpoint.header,
				historical_checkpoint.execution_header_proof.clone(),
			),
			Error::<Test>::ExecutionHeaderAnchorAlreadyImported,
		);
	});
}

#[test]
fn import_trusted_execution_header_backfill_rejects_non_historical_anchor() {
	let latest_checkpoint = Box::new(load_later_checkpoint_update_fixture());
	let newer_update = Box::new(load_next_finalized_header_update_fixture());
	let newer_root = newer_update.finalized_header.hash_tree_root().unwrap();
	let newer_anchor = ExecutionHeaderAnchor::from_payload_header(
		&newer_update.execution_header_proof.execution_header,
	);
	let latest_anchor = ExecutionHeaderAnchor::from_payload_header(
		&latest_checkpoint.execution_header_proof.execution_header,
	);

	assert!(newer_anchor.block_number >= latest_anchor.block_number);

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&latest_checkpoint));

		assert_err!(
			EthereumBeaconClient::import_trusted_execution_header_backfill(
				RuntimeOrigin::root(),
				newer_root,
				newer_update.finalized_header,
				newer_update.execution_header_proof.clone(),
			),
			Error::<Test>::ExecutionHeaderAnchorNotHistorical,
		);
	});
}

#[test]
fn import_trusted_execution_header_backfill_rejects_invalid_execution_proof() {
	let historical_checkpoint = Box::new(load_checkpoint_update_fixture());
	let latest_checkpoint = Box::new(load_later_checkpoint_update_fixture());
	let historical_root = historical_checkpoint.header.hash_tree_root().unwrap();
	let mut invalid_proof = historical_checkpoint.execution_header_proof.clone();
	invalid_proof.execution_branch[0] = TEST_HASH.into();

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&latest_checkpoint));

		assert_err!(
			EthereumBeaconClient::import_trusted_execution_header_backfill(
				RuntimeOrigin::root(),
				historical_root,
				historical_checkpoint.header,
				invalid_proof,
			),
			Error::<Test>::InvalidExecutionHeaderProof,
		);
	});
}

#[test]
fn process_minimal_bootstrap_checkpoint() {
	let bootstrap = load_minimal_bootstrap_fixture("minimal-bootstrap.json");
	let raw_sync_committee = bootstrap.current_sync_committee;
	let current_sync_committee = crate::types::SyncCommittee {
		pubkeys: raw_sync_committee
			.pubkeys
			.to_vec()
			.try_into()
			.expect("minimal bootstrap fixture stays within bounded sync committee size"),
		aggregate_pubkey: raw_sync_committee.aggregate_pubkey,
	};
	let current_sync_committee_root = current_sync_committee
		.hash_tree_root(EthereumBeaconPreset::Minimal)
		.expect("minimal committee root should serialize");

	assert_eq!(
		current_sync_committee_root,
		raw_sync_committee
			.hash_tree_root()
			.expect("raw minimal committee root should serialize")
	);
	assert!(verify_merkle_branch(
		current_sync_committee_root,
		&bootstrap.current_sync_committee_branch,
		subtree_index(crate::config::electra::CURRENT_SYNC_COMMITTEE_INDEX),
		generalized_index_length(crate::config::electra::CURRENT_SYNC_COMMITTEE_INDEX),
		bootstrap.header.state_root
	));

	let execution_proof = anchor_execution_proof();
	let mut header = bootstrap.header;
	header.body_root = execution_proof.header.body_root;
	let checkpoint = Box::new(CheckpointUpdate {
		header,
		current_sync_committee,
		current_sync_committee_branch: bootstrap
			.current_sync_committee_branch
			.try_into()
			.expect("minimal bootstrap branch stays within bounded proof size"),
		validators_root: H256::zero(),
		execution_header_proof: ExecutionHeaderProof::from(execution_proof),
	});

	new_tester().execute_with(|| {
		crate::pallet::BeaconPreset::<Test>::put(EthereumBeaconPreset::Minimal);

		assert_ok!(EthereumBeaconClient::force_checkpoint(
			RuntimeOrigin::root(),
			checkpoint,
			ForkVersions {
				genesis: Fork { version: hex!("00000000"), epoch: 0 },
				altair: Fork { version: hex!("01000000"), epoch: 0 },
				bellatrix: Fork { version: hex!("02000000"), epoch: 0 },
				capella: Fork { version: hex!("03000000"), epoch: 0 },
				deneb: Fork { version: hex!("04000000"), epoch: 0 },
				electra: Fork { version: hex!("05000000"), epoch: 0 },
				fulu: Fork { version: hex!("06000000"), epoch: 0 },
			}
		));
	});
}

#[test]
fn process_later_minimal_bootstrap_checkpoint() {
	let bootstrap = load_minimal_bootstrap_fixture("minimal-bootstrap-later.json");
	let raw_sync_committee = bootstrap.current_sync_committee;
	let current_sync_committee = crate::types::SyncCommittee {
		pubkeys: raw_sync_committee
			.pubkeys
			.to_vec()
			.try_into()
			.expect("later minimal bootstrap fixture stays within bounded sync committee size"),
		aggregate_pubkey: raw_sync_committee.aggregate_pubkey,
	};
	let execution_proof = anchor_execution_proof();
	let mut header = bootstrap.header;
	header.body_root = execution_proof.header.body_root;
	let checkpoint = Box::new(CheckpointUpdate {
		header,
		current_sync_committee,
		current_sync_committee_branch: bootstrap
			.current_sync_committee_branch
			.try_into()
			.expect("later minimal bootstrap branch stays within bounded proof size"),
		validators_root: H256::zero(),
		execution_header_proof: ExecutionHeaderProof::from(execution_proof),
	});

	new_tester().execute_with(|| {
		crate::pallet::BeaconPreset::<Test>::put(EthereumBeaconPreset::Minimal);

		assert_ok!(EthereumBeaconClient::force_checkpoint(
			RuntimeOrigin::root(),
			checkpoint,
			ForkVersions {
				genesis: Fork { version: hex!("00000000"), epoch: 0 },
				altair: Fork { version: hex!("01000000"), epoch: 0 },
				bellatrix: Fork { version: hex!("02000000"), epoch: 0 },
				capella: Fork { version: hex!("03000000"), epoch: 0 },
				deneb: Fork { version: hex!("04000000"), epoch: 0 },
				electra: Fork { version: hex!("05000000"), epoch: 0 },
				fulu: Fork { version: hex!("06000000"), epoch: 0 },
			}
		));
	});
}

#[test]
fn submit_update_in_current_period() {
	let checkpoint = Box::new(load_later_checkpoint_update_fixture());
	let update = Box::new(load_finalized_header_update_fixture());
	let block_hash = update.execution_header_proof.execution_header.block_hash();
	let checkpoint_period = compute_period(checkpoint.header.slot);
	let update_period = compute_period(finalized_header(&update).slot);
	assert_eq!(checkpoint_period, update_period);

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		assert!(!<NextSyncCommittee<Test>>::exists());

		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update.clone());
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);
		assert!(!<NextSyncCommittee<Test>>::exists());
		assert!(ExecutionHeaderAnchors::<Test>::contains_key(block_hash));

		let latest_root = <LatestFinalizedBlockRoot<Test>>::get();
		assert_eq!(latest_root, finalized_header(&update).hash_tree_root().unwrap());
		assert_eq!(
			<FinalizedBeaconState<Test>>::get(latest_root),
			Some(FinalizedBeaconHeaderState { slot: finalized_header(&update).slot }),
		);
	});
}

#[test]
fn submit_update_with_sync_committee_in_current_period() {
	let checkpoint = Box::new(load_later_checkpoint_update_fixture());
	let update = Box::new(load_next_sync_committee_update_fixture());
	let block_hash = update.execution_header_proof.execution_header.block_hash();
	let init_period = compute_period(checkpoint.header.slot);
	let update_period = compute_period(finalized_header(&update).slot);
	assert_eq!(init_period, update_period);

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		assert!(!<NextSyncCommittee<Test>>::exists());
		assert!(ExecutionHeaderAnchors::<Test>::contains_key(block_hash));
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);
		assert!(<NextSyncCommittee<Test>>::exists());
		assert!(ExecutionHeaderAnchors::<Test>::contains_key(block_hash));
	});
}

#[test]
fn reject_submit_update_in_next_period() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let sync_committee_update = Box::new(load_sync_committee_update_fixture());
	let mut update = load_next_sync_committee_update_fixture();
	let sync_committee_period = compute_period(finalized_header(&sync_committee_update).slot);
	let next_sync_committee_period = compute_period(finalized_header(&update).slot);
	assert_eq!(sync_committee_period + 1, next_sync_committee_period);
	update.next_sync_committee_update = None;
	let update = Box::new(update);
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
	let update_period = compute_period(finalized_header(&update).slot);
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
fn submit_update_with_invalid_execution_header_proof() {
	let checkpoint = Box::new(load_later_checkpoint_update_fixture());
	let mut update = Box::new(load_finalized_header_update_fixture());
	update.execution_header_proof.execution_branch[0] = TEST_HASH.into();

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_err!(result, Error::<Test>::InvalidExecutionHeaderProof);
		assert_eq!(result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_update_with_invalid_next_sync_committee_proof() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let mut update = Box::new(load_sync_committee_update_fixture());
	let init_period = compute_period(checkpoint.header.slot);
	let update_period = compute_period(finalized_header(&update).slot);
	assert_eq!(init_period, update_period);
	next_sync_committee_update_mut(&mut update).next_sync_committee_branch[0] = TEST_HASH.into();

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
	*signature_slot_mut(&mut update) +=
		MAINNET_EPOCHS_PER_SYNC_COMMITTEE_PERIOD * MAINNET_SLOTS_PER_EPOCH;
	attested_header_mut(&mut update).slot = signature_slot(&update) - 1;

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
	let expected_anchor = ExecutionHeaderAnchor::from_payload_header(
		&next_update.execution_header_proof.execution_header,
	);
	let update_period = compute_period(finalized_header(&update).slot);
	let next_update_period = compute_period(finalized_header(&next_update).slot);
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
		assert_eq!(
			<ExecutionHeaderAnchors<Test>>::get(expected_anchor.block_hash),
			Some(expected_anchor),
		);
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
		*signature_slot_mut(&mut update) = attested_header(&update).slot;

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
	let next_sync_committee_period = compute_period(finalized_header(&finalized_update).slot);
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
		finalized_header_mut(&mut update).slot = checkpoint.header.slot;
		attested_header_mut(&mut update).slot = checkpoint.header.slot;
		*signature_slot_mut(&mut update) = checkpoint.header.slot + 1;

		let result = EthereumBeaconClient::submit(RuntimeOrigin::signed(1), update);
		assert_err!(result, Error::<Test>::IrrelevantUpdate);
		assert_eq!(result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

#[test]
fn submit_update_with_missing_bootstrap() {
	let update = Box::new(load_sync_committee_update_fixture());

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
			*x = Some(FinalizedBeaconHeaderState { slot: attested_header(&next_update).slot });
		});
		attested_header_mut(&mut next_update).slot += 1;
		*signature_slot_mut(&mut next_update) = attested_header(&next_update).slot + 1;
		next_sync_committee_update_mut(&mut next_update).next_sync_committee.pubkeys[0] =
			Default::default();

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
	let slot_with_large_gap = checkpoint.header.slot + MAINNET_SLOTS_PER_HISTORICAL_ROOT + 1;

	finalized_header_mut(&mut next_update).slot = slot_with_large_gap;
	// Adding some slots to the attested header and signature slot since they need to be ahead
	// of the finalized header.
	attested_header_mut(&mut next_update).slot = slot_with_large_gap + 33;
	*signature_slot_mut(&mut next_update) = slot_with_large_gap + 43;

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

	finalized_header_mut(&mut next_update).slot =
		checkpoint.header.slot + MAINNET_SLOTS_PER_HISTORICAL_ROOT;
	// Adding some slots to the attested header and signature slot since they need to be ahead
	// of the finalized header.
	attested_header_mut(&mut next_update).slot =
		checkpoint.header.slot + MAINNET_SLOTS_PER_HISTORICAL_ROOT + 33;
	*signature_slot_mut(&mut next_update) =
		checkpoint.header.slot + MAINNET_SLOTS_PER_HISTORICAL_ROOT + 43;

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
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), sync_committee_update.clone());
		assert_ok!(second_result);
		assert_eq!(second_result.unwrap().pays_fee, Pays::Yes);
	});
}

#[test]
fn sync_committee_update_for_sync_committee_already_imported_are_not_free() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let sync_committee_update = Box::new(load_sync_committee_update_fixture());
	let next_sync_committee_update = Box::new(load_next_sync_committee_update_fixture());
	let later_finalized_update = Box::new(load_finalized_header_update_fixture());
	let sync_committee_period = compute_period(finalized_header(&sync_committee_update).slot);
	let next_sync_committee_period =
		compute_period(finalized_header(&next_sync_committee_update).slot);
	assert_eq!(sync_committee_period + 1, next_sync_committee_period);

	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&checkpoint));
		assert_eq!(<LatestSyncCommitteeUpdatePeriod<Test>>::get(), 0);

		let result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), sync_committee_update.clone());
		assert_ok!(result);
		assert_eq!(result.unwrap().pays_fee, Pays::No);
		assert_eq!(<LatestSyncCommitteeUpdatePeriod<Test>>::get(), sync_committee_period);

		let second_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), sync_committee_update.clone());
		assert_ok!(second_result);
		assert_eq!(second_result.unwrap().pays_fee, Pays::Yes);
		assert_eq!(<LatestSyncCommitteeUpdatePeriod<Test>>::get(), sync_committee_period);

		let third_result = EthereumBeaconClient::submit(
			RuntimeOrigin::signed(1),
			next_sync_committee_update.clone(),
		);
		assert_ok!(third_result);
		assert_eq!(third_result.unwrap().pays_fee, Pays::No);
		assert_eq!(<LatestSyncCommitteeUpdatePeriod<Test>>::get(), next_sync_committee_period);

		let fourth_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), next_sync_committee_update);
		assert_ok!(fourth_result);
		assert_eq!(fourth_result.unwrap().pays_fee, Pays::Yes);

		let fifth_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), later_finalized_update);
		assert_ok!(fifth_result);
		assert_eq!(fifth_result.unwrap().pays_fee, Pays::No);

		let sixth_result =
			EthereumBeaconClient::submit(RuntimeOrigin::signed(1), sync_committee_update);
		assert_err!(sixth_result, Error::<Test>::SkippedSyncCommitteePeriod);
		assert_eq!(sixth_result.unwrap_err().post_info.pays_fee, Pays::Yes);
	});
}

/* IMPLS */

#[test]
fn verify_message() {
	let (_event_log, proof, anchor) = retained_anchor_verification_payload();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_ok!(<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof,));
	});
}

#[test]
fn verify_message_accepts_multiple_claims_from_the_same_receipt() {
	let (_event_log, proof, anchor) = retained_anchor_verification_payload();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_ok!(<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof,));
	});
}

#[test]
fn verify_message_accepts_multiple_receipts_from_one_combined_proof() {
	let first_address = Address::repeat_byte(0x11);
	let second_address = Address::repeat_byte(0x22);
	let first_topic = B256::repeat_byte(0xa1);
	let second_topic = B256::repeat_byte(0xb2);
	let first_data = Bytes::from_static(b"first-transfer");
	let second_data = Bytes::from_static(b"second-transfer");

	let first_receipt = ReceiptEnvelope::Legacy(
		Receipt {
			status: true.into(),
			cumulative_gas_used: 10,
			logs: vec![Log::new_unchecked(first_address, vec![first_topic], first_data.clone())],
		}
		.with_bloom(),
	);
	let second_receipt = ReceiptEnvelope::Legacy(
		Receipt {
			status: true.into(),
			cumulative_gas_used: 20,
			logs: vec![Log::new_unchecked(second_address, vec![second_topic], second_data.clone())],
		}
		.with_bloom(),
	);

	let mut first_receipt_bytes = Vec::new();
	first_receipt.encode(&mut first_receipt_bytes);
	let mut second_receipt_bytes = Vec::new();
	second_receipt.encode(&mut second_receipt_bytes);

	let first_path = Nibbles::unpack(alloy_rlp::encode(0u64));
	let second_path = Nibbles::unpack(alloy_rlp::encode(1u64));
	let mut hash_builder = HashBuilder::default()
		.with_proof_retainer(ProofRetainer::new(vec![first_path, second_path]));
	let mut trie_receipts =
		vec![(first_path, first_receipt_bytes), (second_path, second_receipt_bytes)];
	trie_receipts.sort_unstable_by_key(|(path, _)| *path);

	for (path, receipt_bytes) in &trie_receipts {
		hash_builder.add_leaf(*path, receipt_bytes);
	}

	let receipts_root = H256::from_slice(hash_builder.root().as_slice());
	let proof_nodes = hash_builder.take_proof_nodes();
	let sorted_nodes = proof_nodes.nodes_sorted();
	let node_paths = sorted_nodes.iter().map(|(path, _)| *path).collect::<Vec<_>>();
	let anchor = ExecutionHeaderAnchor {
		block_number: 200,
		timestamp_millis: 0,
		block_hash: H256::repeat_byte(0x44),
		parent_hash: H256::repeat_byte(0x33),
		state_root: H256::zero(),
		receipts_root,
	};
	let first_event_log = EthereumReceiptLog {
		transaction_index: 0,
		event_log: EthereumLog {
			address: H160::from_slice(first_address.as_slice()),
			topics: vec![H256::from_slice(first_topic.as_slice())]
				.try_into()
				.expect("single topic stays within bounded topic count"),
			data: first_data
				.to_vec()
				.try_into()
				.expect("first log data stays within bounded payload size"),
		},
	};
	let second_event_log = EthereumReceiptLog {
		transaction_index: 1,
		event_log: EthereumLog {
			address: H160::from_slice(second_address.as_slice()),
			topics: vec![H256::from_slice(second_topic.as_slice())]
				.try_into()
				.expect("single topic stays within bounded topic count"),
			data: second_data
				.to_vec()
				.try_into()
				.expect("second log data stays within bounded payload size"),
		},
	};
	let proof = EthereumReceiptLogProofBatch::<ConstU32<1>, ConstU32<2>> {
		execution_block_proof: EthereumExecutionBlockProof {
			anchor_block_hash: anchor.block_hash,
			target_to_anchor_header_chain: Vec::new()
				.try_into()
				.expect("empty header chain stays within bounds"),
		},
		blocks: vec![EthereumReceiptLogProofBlock::<ConstU32<2>> {
			target_block_number: anchor.block_number,
			receipt_proof: EthereumCombinedReceiptProof {
				nodes: sorted_nodes
					.iter()
					.map(|(_, node)| {
						node.to_vec()
							.try_into()
							.expect("retained proof node stays within bounded node size")
					})
					.collect::<Vec<_>>()
					.try_into()
					.expect("combined proof nodes stay within bounded node count"),
				receipts: vec![
					EthereumReceiptProofReceipt {
						transaction_index: 0,
						node_indexes: proof_nodes
							.matching_nodes_sorted(&first_path)
							.into_iter()
							.map(|(path, _)| {
								node_paths
									.iter()
									.position(|candidate| *candidate == path)
									.expect("first receipt proof nodes should be retained")
									as u16
							})
							.collect::<Vec<_>>()
							.try_into()
							.expect("first receipt node refs stay within bounds"),
					},
					EthereumReceiptProofReceipt {
						transaction_index: 1,
						node_indexes: proof_nodes
							.matching_nodes_sorted(&second_path)
							.into_iter()
							.map(|(path, _)| {
								node_paths
									.iter()
									.position(|candidate| *candidate == path)
									.expect("second receipt proof nodes should be retained")
									as u16
							})
							.collect::<Vec<_>>()
							.try_into()
							.expect("second receipt node refs stay within bounds"),
					},
				]
				.try_into()
				.expect("combined proof receipts stay within bounded receipt count"),
			},
			receipt_logs: vec![first_event_log, second_event_log]
				.try_into()
				.expect("combined receipt logs stay within bounded log count"),
		}]
		.try_into()
		.expect("single proof block stays within bounded block count"),
	};

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_ok!(<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof,));
	});
}

#[test]
fn verify_message_invalid_proof() {
	let (_event_log, mut proof, anchor) = retained_anchor_verification_payload();
	proof.blocks[0].receipt_proof.nodes[0] =
		vec![1, 2, 3].try_into().expect("tiny malformed node stays within bounded size");

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof),
			Err(EthereumVerifyError::InvalidProof)
		);
	});
}

#[test]
fn verify_message_rejects_duplicate_receipt_entries_for_one_transaction_index() {
	let (_event_log, mut proof, anchor) = retained_anchor_verification_payload();
	let mut receipts = proof.blocks[0].receipt_proof.receipts.to_vec();
	receipts.push(receipts[0].clone());
	proof.blocks[0].receipt_proof.receipts = receipts
		.try_into()
		.expect("duplicated receipt entries stay within bounded receipt count");

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof),
			Err(EthereumVerifyError::InvalidProof)
		);
	});
}

#[test]
fn verify_message_rejects_empty_and_duplicated_receipt_proofs() {
	let (_event_log, proof, anchor) = retained_anchor_verification_payload();

	let mut empty_proof = proof.clone();
	empty_proof.blocks[0].receipt_proof.nodes = Vec::new()
		.try_into()
		.expect("empty receipt proof stays within bounded node count");

	let mut duplicated_proof = proof.clone();
	let mut duplicated_indexes =
		duplicated_proof.blocks[0].receipt_proof.receipts[0].node_indexes.to_vec();
	duplicated_indexes.push(
		*duplicated_indexes
			.last()
			.expect("fixture receipt proof includes at least one node index"),
	);
	duplicated_proof.blocks[0].receipt_proof.receipts[0].node_indexes = duplicated_indexes
		.try_into()
		.expect("duplicated receipt proof stays within bounded node refs");

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);

		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&empty_proof,),
			Err(EthereumVerifyError::InvalidProof)
		);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(
				&duplicated_proof,
			),
			Err(EthereumVerifyError::InvalidProof)
		);
	});
}

#[test]
fn verify_message_rejects_missing_receipt_entries_and_out_of_range_node_indexes() {
	let (_event_log, proof, anchor) = retained_anchor_verification_payload();

	let mut missing_receipt = proof.clone();
	missing_receipt.blocks[0].receipt_proof.receipts = Vec::new()
		.try_into()
		.expect("empty receipt list stays within bounded receipt count");

	let mut out_of_range_node_index = proof.clone();
	out_of_range_node_index.blocks[0].receipt_proof.receipts[0].node_indexes = vec![999u16]
		.try_into()
		.expect("out-of-range node index stays within bounded receipt proof refs");

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&missing_receipt,),
			Err(EthereumVerifyError::InvalidProof)
		);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(
				&out_of_range_node_index,
			),
			Err(EthereumVerifyError::InvalidProof)
		);
	});
}

#[test]
fn verify_message_invalid_receipts_root() {
	let (_event_log, proof, mut anchor) = retained_anchor_verification_payload();
	anchor.receipts_root = TEST_HASH.into();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof),
			Err(EthereumVerifyError::InvalidProof)
		);
	});
}

#[test]
fn verify_message_rejects_batches_with_any_invalid_log() {
	let (_event_log, mut proof, anchor) = retained_anchor_verification_payload();
	proof.blocks[0].receipt_logs[0].event_log.topics[0] = H256::zero();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof),
			Err(EthereumVerifyError::LogNotFound)
		);
	});
}

#[test]
fn verify_message_invalid_log() {
	let (_event_log, mut proof, anchor) = retained_anchor_verification_payload();
	proof.blocks[0].receipt_logs[0].event_log.topics[0] = H256::zero();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof),
			Err(EthereumVerifyError::LogNotFound)
		);
	});
}

#[test]
fn verify_message_receipt_does_not_contain_log() {
	let (_event_log, mut proof, anchor) = retained_anchor_verification_payload();
	proof.blocks[0].receipt_logs[0].event_log.data = hex!("f9013c94ee9170abfbf9421ad6dd07f6bdec9d89f2b581e0f863a01b11dcf133cc240f682dab2d3a8e4cd35c5da8c9cf99adac4336f8512584c5ada000000000000000000000000000000000000000000000000000000000000003e8a00000000000000000000000000000000000000000000000000000000000000002b8c000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000068000f000000000000000101d184c103f7acc340847eee82a0b909e3358bc28d440edffa1352b13227e8ee646f3ea37456dec70100000101001cbd2d43530a44705ad088af313e18f80b53ef16b36177cd4b77b846f2a5f07c0000e8890423c78a0000000000000000000000000000000000000000000000000000000000000000")
		.to_vec()
		.try_into()
		.expect("mutated log payload stays within bounded log data size");

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof),
			Err(EthereumVerifyError::LogNotFound)
		);
	});
}

#[test]
fn set_operating_mode() {
	let checkpoint = Box::new(load_checkpoint_update_fixture());
	let update = Box::new(load_sync_committee_update_fixture());

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
	let execution_proof = anchor_execution_proof();
	let mut execution_header_proof = ExecutionHeaderProof::from(&execution_proof);
	execution_header_proof.execution_branch[0] = TEST_HASH.into();

	new_tester().execute_with(|| {
		assert_err!(
			EthereumBeaconClient::verify_execution_proof_for_header(
				&execution_proof.header,
				&execution_header_proof,
			),
			Error::<Test>::InvalidExecutionHeaderProof
		);
	});
}

#[test]
fn verify_message_invalid_topic() {
	let (_event_log, mut proof, anchor) = retained_anchor_verification_payload();
	proof.blocks[0].receipt_logs[0].event_log.topics[0] = H256::default();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof),
			Err(EthereumVerifyError::LogNotFound)
		);
	});
}

#[test]
fn verify_message_is_unavailable_when_halted() {
	let (_event_log, proof, anchor) = retained_anchor_verification_payload();

	new_tester().execute_with(|| {
		assert_ok!(EthereumBeaconClient::set_operating_mode(
			RuntimeOrigin::root(),
			BasicOperatingMode::Halted
		));
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);

		assert_eq!(
			<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(&proof),
			Err(EthereumVerifyError::VerifierUnavailable)
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
		let signature_slot: u64 = fulu_epoch * MAINNET_SLOTS_PER_EPOCH;

		// Verify this is the first slot of the epoch
		assert_eq!(signature_slot % MAINNET_SLOTS_PER_EPOCH, 0);

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
			compute_epoch(signature_slot, MAINNET_SLOTS_PER_EPOCH),
		)
		.unwrap();
		let fork_version_at_previous_slot = EthereumBeaconClient::compute_fork_version(
			compute_epoch(signature_slot.saturating_sub(1), MAINNET_SLOTS_PER_EPOCH),
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
	let checkpoint = Box::new(load_later_checkpoint_update_fixture());
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
	*signature_slot_mut(&mut different_proof_update) += 1;

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
		(b"ethereum_verifier:submit".as_slice(), update.using_encoded(blake2_256))
			.using_encoded(blake2_256)
			.to_vec()
	);
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
	#[derive(Clone, Encode)]
	struct UnboundedEthereumReceiptProofReceipt {
		#[codec(compact)]
		transaction_index: u64,
		node_indexes: Vec<u16>,
	}

	#[derive(Encode)]
	struct UnboundedEthereumCombinedReceiptProof {
		nodes: Vec<Vec<u8>>,
		receipts: Vec<UnboundedEthereumReceiptProofReceipt>,
	}

	let oversized_node = UnboundedEthereumCombinedReceiptProof {
		nodes: vec![vec![0u8; (MAX_ETHEREUM_RECEIPT_PROOF_NODE_BYTES + 1) as usize]],
		receipts: vec![UnboundedEthereumReceiptProofReceipt {
			transaction_index: 0,
			node_indexes: vec![0],
		}],
	}
	.encode();
	assert!(EthereumCombinedReceiptProof::decode(&mut &oversized_node[..]).is_err());

	let oversized_node_count = UnboundedEthereumCombinedReceiptProof {
		nodes: vec![vec![0u8]; (MAX_ETHEREUM_COMBINED_RECEIPT_PROOF_NODES + 1) as usize],
		receipts: vec![UnboundedEthereumReceiptProofReceipt {
			transaction_index: 0,
			node_indexes: vec![0],
		}],
	}
	.encode();
	assert!(EthereumCombinedReceiptProof::decode(&mut &oversized_node_count[..]).is_err());

	let oversized_receipt_count = UnboundedEthereumCombinedReceiptProof {
		nodes: vec![vec![0u8]],
		receipts: vec![
			UnboundedEthereumReceiptProofReceipt {
				transaction_index: 0,
				node_indexes: vec![0],
			};
			(MAX_ETHEREUM_RECEIPTS_PER_PROOF + 1) as usize
		],
	}
	.encode();
	assert!(EthereumCombinedReceiptProof::decode(&mut &oversized_receipt_count[..]).is_err());

	let oversized_receipt_node_refs = UnboundedEthereumCombinedReceiptProof {
		nodes: vec![vec![0u8]],
		receipts: vec![UnboundedEthereumReceiptProofReceipt {
			transaction_index: 0,
			node_indexes: vec![0u16; (MAX_ETHEREUM_RECEIPT_PROOF_NODE_REFS + 1) as usize],
		}],
	}
	.encode();
	assert!(EthereumCombinedReceiptProof::decode(&mut &oversized_receipt_node_refs[..]).is_err());
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
		timestamp_millis: 0,
		block_hash: anchor_block_hash,
		parent_hash: target_block_hash,
		state_root: H256::zero(),
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
fn verify_execution_block_proof_accepts_multi_hop_header_chain_to_anchor() {
	let target_receipts_root = H256::repeat_byte(0x51);
	let (target_header, target_block_hash) =
		make_execution_header(100, H256::repeat_byte(0x10), target_receipts_root);
	let (intermediate_header, intermediate_block_hash) =
		make_execution_header(101, target_block_hash, H256::repeat_byte(0x52));
	let anchor_block_hash = H256::repeat_byte(0x53);
	let anchor = ExecutionHeaderAnchor {
		block_number: 102,
		timestamp_millis: 0,
		block_hash: anchor_block_hash,
		parent_hash: intermediate_block_hash,
		state_root: H256::zero(),
		receipts_root: H256::repeat_byte(0x54),
	};
	let proof = EthereumExecutionBlockProof {
		anchor_block_hash,
		target_to_anchor_header_chain: vec![target_header, intermediate_header]
			.try_into()
			.expect("two-hop header chain stays within bounded header chain length"),
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
fn verify_receipt_logs_accepts_shared_header_chain_suffixes_for_multiple_blocks() {
	let older_address = Address::repeat_byte(0x71);
	let older_topic = B256::repeat_byte(0x72);
	let older_data = Bytes::from_static(&[0x01, 0x02]);
	let newer_address = Address::repeat_byte(0x81);
	let newer_topic = B256::repeat_byte(0x82);
	let newer_data = Bytes::from_static(&[0x03, 0x04]);

	let older_receipt = ReceiptEnvelope::Legacy(
		Receipt {
			status: true.into(),
			cumulative_gas_used: 10,
			logs: vec![Log::new_unchecked(older_address, vec![older_topic], older_data.clone())],
		}
		.with_bloom(),
	);
	let newer_receipt = ReceiptEnvelope::Legacy(
		Receipt {
			status: true.into(),
			cumulative_gas_used: 20,
			logs: vec![Log::new_unchecked(newer_address, vec![newer_topic], newer_data.clone())],
		}
		.with_bloom(),
	);

	let build_receipt_proof =
		|receipt: &ReceiptEnvelope, address: Address, topic: B256, data: Bytes| {
			let mut receipt_bytes = Vec::new();
			receipt.encode(&mut receipt_bytes);

			let path = Nibbles::unpack(alloy_rlp::encode(0u64));
			let mut hash_builder =
				HashBuilder::default().with_proof_retainer(ProofRetainer::new(vec![path]));
			hash_builder.add_leaf(path, &receipt_bytes);

			let receipts_root = H256::from_slice(hash_builder.root().as_slice());
			let proof_nodes = hash_builder.take_proof_nodes();
			let sorted_nodes = proof_nodes.nodes_sorted();
			let node_paths = sorted_nodes.iter().map(|(path, _)| *path).collect::<Vec<_>>();

			(
				EthereumCombinedReceiptProof {
					nodes: sorted_nodes
						.iter()
						.map(|(_, node)| {
							node.to_vec()
								.try_into()
								.expect("retained proof node stays within bounded node size")
						})
						.collect::<Vec<_>>()
						.try_into()
						.expect("combined proof nodes stay within bounded node count"),
					receipts: vec![EthereumReceiptProofReceipt {
						transaction_index: 0,
						node_indexes: proof_nodes
							.matching_nodes_sorted(&path)
							.into_iter()
							.map(|(path, _)| {
								node_paths
									.iter()
									.position(|candidate| *candidate == path)
									.expect("receipt proof nodes should be retained") as u16
							})
							.collect::<Vec<_>>()
							.try_into()
							.expect("receipt node refs stay within bounds"),
					}]
					.try_into()
					.expect("single receipt proof stays within bounded receipt count"),
				},
				receipts_root,
				EthereumReceiptLog {
					transaction_index: 0,
					event_log: EthereumLog {
						address: H160::from_slice(address.as_slice()),
						topics: vec![H256::from_slice(topic.as_slice())]
							.try_into()
							.expect("single topic stays within bounded topic count"),
						data: data
							.to_vec()
							.try_into()
							.expect("log data stays within bounded payload size"),
					},
				},
			)
		};

	let (older_receipt_proof, older_receipts_root, older_receipt_log) =
		build_receipt_proof(&older_receipt, older_address, older_topic, older_data);
	let (newer_receipt_proof, newer_receipts_root, newer_receipt_log) =
		build_receipt_proof(&newer_receipt, newer_address, newer_topic, newer_data);

	let older_parent_hash = H256::repeat_byte(0x61);
	let (older_header, older_block_hash) =
		make_execution_header(10, older_parent_hash, older_receipts_root);
	let (newer_header, newer_block_hash) =
		make_execution_header(11, older_block_hash, newer_receipts_root);
	let anchor_block_hash = H256::repeat_byte(0x62);
	let anchor = ExecutionHeaderAnchor {
		block_number: 12,
		timestamp_millis: 0,
		block_hash: anchor_block_hash,
		parent_hash: newer_block_hash,
		state_root: H256::zero(),
		receipts_root: H256::repeat_byte(0x63),
	};
	let proof_batch = EthereumReceiptLogProofBatch::<ConstU32<2>, ConstU32<1>> {
		execution_block_proof: EthereumExecutionBlockProof {
			anchor_block_hash,
			target_to_anchor_header_chain: vec![older_header, newer_header]
				.try_into()
				.expect("two-hop shared chain stays within bounded header chain length"),
		},
		blocks: vec![
			EthereumReceiptLogProofBlock::<ConstU32<1>> {
				target_block_number: 10,
				receipt_proof: older_receipt_proof,
				receipt_logs: vec![older_receipt_log]
					.try_into()
					.expect("single receipt log stays within bounded log count"),
			},
			EthereumReceiptLogProofBlock::<ConstU32<1>> {
				target_block_number: 11,
				receipt_proof: newer_receipt_proof,
				receipt_logs: vec![newer_receipt_log]
					.try_into()
					.expect("single receipt log stays within bounded log count"),
			},
		]
		.try_into()
		.expect("two proof blocks stay within bounded block count"),
	};

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor_block_hash, anchor);
		assert_ok!(<EthereumBeaconClient as EthereumVerifyProvider>::verify_receipt_logs(
			&proof_batch,
		));
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
		timestamp_millis: 0,
		block_hash: anchor_block_hash,
		parent_hash: H256::repeat_byte(2),
		state_root: H256::zero(),
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

#[test]
fn verify_account_storage_proof_matches_hardhat_eth_getproof_fixture() {
	let (gateway_address, anchor, proof) = load_account_storage_proof_fixture();

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor.clone());
		assert_ok!(EthereumBeaconClient::verify_account_storage_proof_against_state_root(
			anchor.state_root,
			gateway_address,
			&proof,
		));
		assert_eq!(
			EthereumBeaconClient::verify_account_storage_proof(gateway_address, &proof),
			Ok(()),
		);
	});
}

#[test]
fn verify_account_storage_proof_fixture_rejects_wrong_account() {
	let (_gateway_address, anchor, proof) = load_account_storage_proof_fixture();
	let wrong_gateway_address = H160::repeat_byte(0x24);

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor.clone());
		assert_eq!(
			EthereumBeaconClient::verify_account_storage_proof_against_state_root(
				anchor.state_root,
				wrong_gateway_address,
				&proof,
			),
			Err(EthereumVerifyError::InvalidProof)
		);
		assert_eq!(
			EthereumBeaconClient::verify_account_storage_proof(wrong_gateway_address, &proof),
			Err(EthereumVerifyError::InvalidProof),
		);
	});
}

#[test]
fn verify_account_storage_proof_fixture_rejects_wrong_slot_value() {
	let (gateway_address, anchor, mut proof) = load_account_storage_proof_fixture();
	proof.slots[1].value = H256::repeat_byte(0x36);

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor.clone());
		assert_eq!(
			EthereumBeaconClient::verify_account_storage_proof_against_state_root(
				anchor.state_root,
				gateway_address,
				&proof,
			),
			Err(EthereumVerifyError::InvalidProof)
		);
		assert_eq!(
			EthereumBeaconClient::verify_account_storage_proof(gateway_address, &proof),
			Err(EthereumVerifyError::InvalidProof),
		);
	});
}

#[test]
fn verify_account_storage_proof_fixture_rejects_out_of_range_shared_node_index() {
	let (gateway_address, anchor, mut proof) = load_account_storage_proof_fixture();
	proof.slots[1].node_indexes = vec![999u16]
		.try_into()
		.expect("single out-of-range index stays within bounded node refs");

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor.clone());
		assert_eq!(
			EthereumBeaconClient::verify_account_storage_proof_against_state_root(
				anchor.state_root,
				gateway_address,
				&proof,
			),
			Err(EthereumVerifyError::InvalidProof)
		);
		assert_eq!(
			EthereumBeaconClient::verify_account_storage_proof(gateway_address, &proof),
			Err(EthereumVerifyError::InvalidProof),
		);
	});
}

#[test]
fn verify_account_storage_proof_fixture_rejects_duplicate_shared_node_index() {
	let (gateway_address, anchor, mut proof) = load_account_storage_proof_fixture();
	let duplicated_index = proof.slots[0].node_indexes[0];
	proof.slots[0].node_indexes = vec![duplicated_index, duplicated_index]
		.try_into()
		.expect("duplicate indexes stay within bounded node refs");

	new_tester().execute_with(|| {
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor.clone());
		assert_eq!(
			EthereumBeaconClient::verify_account_storage_proof_against_state_root(
				anchor.state_root,
				gateway_address,
				&proof,
			),
			Err(EthereumVerifyError::InvalidProof)
		);
		assert_eq!(
			EthereumBeaconClient::verify_account_storage_proof(gateway_address, &proof),
			Err(EthereumVerifyError::InvalidProof),
		);
	});
}
