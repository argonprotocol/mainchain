// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
//! Indexed receipt proof verification imported from Snowbridge verification primitives.
//!
//! Upstream source:
//! - crate: `snowbridge-verification-primitives`
//! - version: `0.9.0`
//! - repository: `https://github.com/paritytech/polkadot-sdk`
//! - path: `bridges/snowbridge/primitives/verification/src/receipt.rs`

use alloc::vec::Vec;
use alloy_consensus::ReceiptEnvelope;
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use alloy_trie::{
	proof::{verify_proof, ProofVerificationError},
	Nibbles,
};
use argon_primitives::ethereum::{EthereumCombinedReceiptProof, EthereumReceiptProofReceipt};
use polkadot_sdk::sp_core::H256;

pub(crate) fn verify_receipt_proof(
	receipts_root: H256,
	combined_proof: &EthereumCombinedReceiptProof,
	tx_index: u64,
) -> Option<ReceiptEnvelope> {
	let key = receipt_trie_key(tx_index);
	let root = B256::from_slice(receipts_root.as_bytes());
	let receipt = find_receipt_proof(&combined_proof.receipts, tx_index)?;
	let proof_nodes = receipt_proof_nodes(combined_proof, receipt)?;

	let value = match verify_proof(root, key, None, proof_nodes.iter()) {
		Ok(()) => return None,
		Err(ProofVerificationError::ValueMismatch { path, got: Some(value), expected: None })
			if path == key =>
			value.to_vec(),
		Err(_) => return None,
	};

	ReceiptEnvelope::decode(&mut value.as_slice()).ok()
}

fn find_receipt_proof(
	receipts: &[EthereumReceiptProofReceipt],
	tx_index: u64,
) -> Option<&EthereumReceiptProofReceipt> {
	let mut matches = receipts.iter().filter(|receipt| receipt.transaction_index == tx_index);
	let receipt = matches.next()?;
	if matches.next().is_some() {
		return None;
	}

	Some(receipt)
}

fn receipt_proof_nodes(
	combined_proof: &EthereumCombinedReceiptProof,
	receipt: &EthereumReceiptProofReceipt,
) -> Option<Vec<Bytes>> {
	let mut proof_nodes = Vec::with_capacity(receipt.node_indexes.len());
	let mut seen_indexes = Vec::with_capacity(receipt.node_indexes.len());

	for node_index in receipt.node_indexes.iter() {
		if seen_indexes.contains(node_index) {
			return None;
		}
		seen_indexes.push(*node_index);

		let node = combined_proof.nodes.get(*node_index as usize)?;
		proof_nodes.push(Bytes::copy_from_slice(node.as_slice()));
	}

	Some(proof_nodes)
}

fn receipt_trie_key(tx_index: u64) -> Nibbles {
	let encoded_index = alloy_rlp::encode(tx_index);
	Nibbles::unpack(encoded_index.as_slice())
}
