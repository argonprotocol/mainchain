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
use polkadot_sdk::sp_core::H256;

pub(crate) fn verify_receipt_proof(
	receipts_root: H256,
	tx_index: u64,
	proof: &[Vec<u8>],
) -> Option<ReceiptEnvelope> {
	let key = receipt_trie_key(tx_index);
	let root = B256::from_slice(receipts_root.as_bytes());
	let proof_nodes: Vec<Bytes> = proof.iter().map(|node| Bytes::copy_from_slice(node)).collect();

	let value = match verify_proof(root, key, None, proof_nodes.iter()) {
		Ok(()) => return None,
		Err(ProofVerificationError::ValueMismatch { path, got: Some(value), expected: None })
			if path == key =>
			value.to_vec(),
		Err(_) => return None,
	};

	ReceiptEnvelope::decode(&mut value.as_slice()).ok()
}

fn receipt_trie_key(tx_index: u64) -> Nibbles {
	let encoded_index = alloy_rlp::encode(tx_index);
	Nibbles::unpack(encoded_index.as_slice())
}
