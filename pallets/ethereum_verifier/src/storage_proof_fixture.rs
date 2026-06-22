use crate::types::ExecutionHeaderAnchor;
use alloc::{string::String, vec::Vec};
use alloy_primitives::hex::FromHex;
use argon_primitives::{EthereumStorageSlotProof, EthereumTrieProofNodes};
use polkadot_sdk::sp_core::{H160, H256};
use serde::Deserialize;

/// One real `eth_getProof` witness captured from the local Hardhat gateway deployment.
pub(crate) struct LoadedAccountStorageProofFixture {
	pub gateway_address: H160,
	pub anchor: ExecutionHeaderAnchor,
	pub account_proof: EthereumTrieProofNodes,
	pub storage_proof: EthereumTrieProofNodes,
	pub range_slot: EthereumStorageSlotProof,
	pub root_slot: EthereumStorageSlotProof,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountStorageProofFixture {
	anchor: AccountStorageProofFixtureAnchor,
	account_address: String,
	proof: AccountStorageProofFixtureProof,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountStorageProofFixtureAnchor {
	block_hash: String,
	block_number: u64,
	timestamp_millis: u64,
	parent_hash: String,
	state_root: String,
	receipts_root: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountStorageProofFixtureProof {
	account_proof: Vec<String>,
	storage_proof: Vec<String>,
	range_slot: String,
	range_slot_value: String,
	range_node_indexes: Vec<u16>,
	root_slot: String,
	activity_root: String,
	root_node_indexes: Vec<u16>,
}

fn fixture_h160(value: &str) -> H160 {
	let bytes = Vec::<u8>::from_hex(value.trim_start_matches("0x"))
		.expect("fixture H160 hex should stay valid");
	H160::from_slice(bytes.as_slice())
}

fn fixture_h256(value: &str) -> H256 {
	let bytes = Vec::<u8>::from_hex(value.trim_start_matches("0x"))
		.expect("fixture H256 hex should stay valid");
	H256::from_slice(bytes.as_slice())
}

fn fixture_proof_nodes(nodes: Vec<String>) -> EthereumTrieProofNodes {
	nodes
		.into_iter()
		.map(|node| {
			Vec::<u8>::from_hex(node.trim_start_matches("0x"))
				.expect("fixture proof node hex should stay valid")
				.try_into()
				.expect("fixture proof node stays within bounded node size")
		})
		.collect::<Vec<_>>()
		.try_into()
		.expect("fixture proof stays within bounded node count")
}

pub(crate) fn load_account_storage_proof_fixture() -> LoadedAccountStorageProofFixture {
	let fixture: AccountStorageProofFixture = serde_json::from_str(include_str!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/tests/fixtures/account-storage-proof.json"
	)))
	.expect("account-storage-proof fixture should stay valid");

	LoadedAccountStorageProofFixture {
		gateway_address: fixture_h160(&fixture.account_address),
		anchor: ExecutionHeaderAnchor {
			block_number: fixture.anchor.block_number,
			timestamp_millis: fixture.anchor.timestamp_millis,
			block_hash: fixture_h256(&fixture.anchor.block_hash),
			parent_hash: fixture_h256(&fixture.anchor.parent_hash),
			state_root: fixture_h256(&fixture.anchor.state_root),
			receipts_root: fixture_h256(&fixture.anchor.receipts_root),
		},
		account_proof: fixture_proof_nodes(fixture.proof.account_proof),
		storage_proof: fixture_proof_nodes(fixture.proof.storage_proof),
		range_slot: EthereumStorageSlotProof {
			slot: fixture_h256(&fixture.proof.range_slot),
			value: fixture_h256(&fixture.proof.range_slot_value),
			node_indexes: fixture
				.proof
				.range_node_indexes
				.try_into()
				.expect("fixture locator range node refs stay within bounds"),
		},
		root_slot: EthereumStorageSlotProof {
			slot: fixture_h256(&fixture.proof.root_slot),
			value: fixture_h256(&fixture.proof.activity_root),
			node_indexes: fixture
				.proof
				.root_node_indexes
				.try_into()
				.expect("fixture locator root node refs stay within bounds"),
		},
	}
}
