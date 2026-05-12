//! Argon execution proof provider.
//!
//! This module owns the Argon event-log verification boundary:
//! - stored direct-finalized execution anchors imported by relayers
//! - execution header continuity from the burn block to that anchor
//! - receipt trie proof verification

use super::*;
use crate::{
	receipt::verify_receipt_proof,
	types::{ExecutionHeaderAnchor, ReceiptsRoot},
};
use alloy_consensus::Header as AlloyHeader;
use alloy_rlp::Decodable;
use argon_primitives::{
	EthereumExecutionBlockProof, EthereumExecutionHeader, EthereumLog, EthereumProof,
	EthereumVerifyError, EthereumVerifyProvider,
};
use polkadot_sdk::frame_support::ensure;

impl<T: Config> EthereumVerifyProvider for Pallet<T> {
	type Weights = weights::ProviderWeightAdapter<T>;

	fn verify_event_log(
		event_log: &EthereumLog,
		proof: &EthereumProof,
	) -> Result<(), EthereumVerifyError> {
		ensure!(!OperatingMode::<T>::get().is_halted(), EthereumVerifyError::VerifierUnavailable);

		let receipts_root = Self::verify_execution_block_proof(&proof.execution_block_proof)?;
		let receipt = verify_receipt_proof(
			receipts_root,
			proof.receipt_proof.transaction_index,
			&proof.receipt_proof.nodes,
		)
		.ok_or(EthereumVerifyError::InvalidProof)?;

		let has_log = receipt.logs().iter().any(|receipt_log| {
			receipt_log.data.data.0.as_ref() == event_log.data.as_slice() &&
				receipt_log.address.0 == event_log.address.0 &&
				receipt_log.topics().len() == event_log.topics.len() &&
				receipt_log
					.topics()
					.iter()
					.zip(event_log.topics.iter())
					.all(|(topic1, topic2)| topic1.0 == topic2.0)
		});
		ensure!(has_log, EthereumVerifyError::LogNotFound);

		Ok(())
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn verify_execution_block_proof(
		proof: &EthereumExecutionBlockProof,
	) -> Result<ReceiptsRoot, EthereumVerifyError> {
		let anchor = ExecutionHeaderAnchors::<T>::get(proof.anchor_block_hash)
			.ok_or(EthereumVerifyError::AnchorNotFound)?;

		if proof.target_to_anchor_header_chain.is_empty() {
			return Ok(anchor.receipts_root)
		}

		let mut headers = proof.target_to_anchor_header_chain.iter();
		let target = Self::decode_execution_header(
			headers.next().ok_or(EthereumVerifyError::InvalidHeaderChain)?,
		)?;
		let receipts_root = target.receipts_root;
		let mut previous = target;

		for header in headers {
			let current = Self::decode_execution_header(header)?;
			ensure!(
				current.parent_hash == previous.block_hash &&
					current.block_number.checked_sub(1) == Some(previous.block_number),
				EthereumVerifyError::InvalidHeaderChain
			);
			previous = current;
		}

		ensure!(
			anchor.parent_hash == previous.block_hash &&
				anchor.block_number.checked_sub(1) == Some(previous.block_number),
			EthereumVerifyError::InvalidHeaderChain
		);

		Ok(receipts_root)
	}

	fn decode_execution_header(
		header: &EthereumExecutionHeader,
	) -> Result<ExecutionHeaderAnchor, EthereumVerifyError> {
		let mut bytes = header.rlp.as_slice();
		let decoded =
			AlloyHeader::decode(&mut bytes).map_err(|_| EthereumVerifyError::InvalidHeader)?;
		ensure!(bytes.is_empty(), EthereumVerifyError::InvalidHeader);

		Ok(ExecutionHeaderAnchor {
			block_number: decoded.number,
			// Recompute the sealed hash from the verified RLP bytes so the
			// header chain is anchored to the actual encoded header contents.
			block_hash: H256::from_slice(decoded.hash_slow().as_slice()),
			parent_hash: H256::from_slice(decoded.parent_hash.as_slice()),
			receipts_root: H256::from_slice(decoded.receipts_root.as_slice()),
		})
	}
}
