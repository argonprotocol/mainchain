//! Ethereum account/storage proof verification.
//!
//! This verifier anchors one account plus a bounded set of storage slots under a retained
//! execution header's `state_root` using `eth_getProof`-style account and storage proofs.
//!
//! TODO: extend this verifier with a generic batched storage-proof shape that can amortize one
//! account proof plus one shared storage-trie witness across multiple logical slot groups proved
//! against the same latest state root.

use super::*;
use alloc::vec::Vec;
use alloy_primitives::{keccak256, Bytes, B256, U256};
use alloy_rlp::Decodable;
use alloy_trie::{
	proof::{verify_proof, ProofVerificationError},
	Nibbles, TrieAccount,
};
use argon_primitives::{
	EthereumAccountStorageProof, EthereumReceiptProofNodeIndexes, EthereumTrieProofNodes,
	EthereumVerifyError,
};
use polkadot_sdk::{frame_support::ensure, sp_core::H160};

impl<T: Config> Pallet<T> {
	pub(crate) fn verify_account_storage_proof_against_state_root<MaxStorageSlots: Get<u32>>(
		state_root: H256,
		account_address: H160,
		proof: &EthereumAccountStorageProof<MaxStorageSlots>,
	) -> Result<(), EthereumVerifyError> {
		let account =
			extract_account_from_proof(state_root, account_address, &proof.account_proof)?;
		for slot in &proof.slots {
			verify_storage_slot_value(
				account.storage_root,
				slot.slot,
				slot.value,
				&proof.storage_proof,
				&slot.node_indexes,
			)?;
		}

		Ok(())
	}
}

fn extract_account_from_proof(
	state_root: H256,
	gateway_address: H160,
	account_proof: &EthereumTrieProofNodes,
) -> Result<TrieAccount, EthereumVerifyError> {
	let account_key = Nibbles::unpack(keccak256(gateway_address.as_bytes()));
	let proof_nodes = trie_proof_nodes(account_proof);

	// `verify_proof` only yields the proven value through its `ValueMismatch` error when the
	// caller passes `expected = None`. We use that alloy-trie convention here to recover the
	// account RLP while still checking that the proof path matches the requested account key.
	let value = match verify_proof(
		B256::from_slice(state_root.as_bytes()),
		account_key,
		None,
		proof_nodes.iter(),
	) {
		Ok(()) => return Err(EthereumVerifyError::InvalidProof),
		Err(ProofVerificationError::ValueMismatch { path, got: Some(value), expected: None })
			if path == account_key =>
			value.to_vec(),
		Err(_) => return Err(EthereumVerifyError::InvalidProof),
	};

	TrieAccount::decode(&mut value.as_slice()).map_err(|_| EthereumVerifyError::InvalidProof)
}

fn verify_storage_slot_value(
	storage_root: B256,
	slot: H256,
	expected_value: H256,
	shared_proof: &EthereumTrieProofNodes,
	node_indexes: &EthereumReceiptProofNodeIndexes,
) -> Result<(), EthereumVerifyError> {
	let slot_key = Nibbles::unpack(keccak256(slot.as_bytes()));
	let proof_nodes = indexed_trie_proof_nodes(shared_proof, node_indexes)?;
	let expected_value = u256_from_h256(expected_value);

	if expected_value.is_zero() {
		ensure!(
			verify_proof(storage_root, slot_key, None, proof_nodes.iter()).is_ok(),
			EthereumVerifyError::InvalidProof
		);
		return Ok(());
	}

	let expected_value = alloy_rlp::encode(expected_value).to_vec();
	ensure!(
		verify_proof(storage_root, slot_key, Some(expected_value), proof_nodes.iter()).is_ok(),
		EthereumVerifyError::InvalidProof
	);
	Ok(())
}

fn trie_proof_nodes(proof: &EthereumTrieProofNodes) -> Vec<Bytes> {
	proof.iter().map(|node| Bytes::copy_from_slice(node.as_slice())).collect()
}

fn indexed_trie_proof_nodes(
	proof: &EthereumTrieProofNodes,
	node_indexes: &EthereumReceiptProofNodeIndexes,
) -> Result<Vec<Bytes>, EthereumVerifyError> {
	let mut proof_nodes = Vec::with_capacity(node_indexes.len());
	let mut seen_indexes = Vec::with_capacity(node_indexes.len());

	for node_index in node_indexes.iter() {
		if seen_indexes.contains(node_index) {
			return Err(EthereumVerifyError::InvalidProof);
		}
		seen_indexes.push(*node_index);

		let node = proof.get(*node_index as usize).ok_or(EthereumVerifyError::InvalidProof)?;
		proof_nodes.push(Bytes::copy_from_slice(node.as_slice()));
	}

	Ok(proof_nodes)
}

fn u256_from_h256(value: H256) -> U256 {
	U256::from_be_slice(value.as_bytes())
}
