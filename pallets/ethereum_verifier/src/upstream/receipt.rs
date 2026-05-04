// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
//! Receipt proof verification imported from Snowbridge verification primitives.
//!
//! Upstream source:
//! - crate: `snowbridge-verification-primitives`
//! - version: `0.8.1`
//! - repository: `https://github.com/paritytech/polkadot-sdk`
//! - path: `bridges/snowbridge/primitives/verification/src/receipt.rs`
//!
//! This file exists because the compatible Snowbridge dependency train used by this pallet does
//! not yet expose the indexed receipt helper. Keep local changes minimal so this can be replaced
//! by the upstream crate once the dependency train can advance.

use alloy_consensus::ReceiptEnvelope;
use alloy_primitives::{B256, Bytes};
use alloy_rlp::Decodable;
use alloy_trie::{
	Nibbles,
	proof::{ProofVerificationError, verify_proof},
};
use sp_core::H256;
use sp_std::prelude::*;

pub(crate) fn verify_receipt_proof(
	receipts_root: H256,
	tx_index: u64,
	proof: &[Vec<u8>],
) -> Option<ReceiptEnvelope> {
	let key = receipt_trie_key(tx_index);
	let root = B256::from_slice(receipts_root.as_bytes());
	let proof_nodes: Vec<Bytes> = proof.iter().map(|node| Bytes::copy_from_slice(node)).collect();

	// Call verify_proof with None to extract the value from an inclusion proof. For inclusion
	// proofs, alloy_trie returns ValueMismatch with the extracted value in `got`. The proof is
	// already cryptographically verified during this traversal.
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
	let encoded_index = alloy_rlp::encode(&tx_index);
	Nibbles::unpack(encoded_index.as_slice())
}
