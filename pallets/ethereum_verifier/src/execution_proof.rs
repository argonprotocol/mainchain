//! Argon execution proof provider.
//!
//! This module owns the Ethereum execution-proof verification boundary:
//! - stored direct-finalized execution anchors imported by relayers
//! - execution header continuity from the proven block to that anchor
//! - receipt and account/storage trie proof verification

use super::{
	receipt::verify_receipt_proof,
	types::{ExecutionHeaderAnchor, ReceiptsRoot},
	*,
};
use alloc::{collections::BTreeMap, vec::Vec};
use alloy_consensus::Header as AlloyHeader;
use alloy_rlp::Decodable;
use argon_primitives::{
	EthereumAccountStorageProof, EthereumBlockNumber, EthereumCombinedReceiptProof,
	EthereumExecutionBlockProof, EthereumExecutionHeader, EthereumLog, EthereumReceiptLog,
	EthereumReceiptLogProofBatch, EthereumVerifyError, EthereumVerifyProvider, Moment,
};
use polkadot_sdk::{frame_support::ensure, sp_core::H160, sp_runtime::traits::Get};

impl<T: Config> EthereumVerifyProvider for Pallet<T> {
	type Weights = weights::ProviderWeightAdapter<T>;

	fn verify_receipt_logs<MaxProofBlocks, MaxReceiptLogs>(
		proof_batch: &EthereumReceiptLogProofBatch<MaxProofBlocks, MaxReceiptLogs>,
	) -> Result<(), EthereumVerifyError>
	where
		MaxProofBlocks: Get<u32>,
		MaxReceiptLogs: Get<u32>,
	{
		ensure!(!OperatingMode::<T>::get().is_halted(), EthereumVerifyError::VerifierUnavailable);

		let headers = Self::verify_execution_headers(&proof_batch.execution_block_proof)?;

		for proof_block in &proof_batch.blocks {
			let receipts_root = headers
				.iter()
				.find_map(|header| {
					(header.block_number == proof_block.target_block_number)
						.then_some(header.receipts_root)
				})
				.ok_or(EthereumVerifyError::InvalidProof)?;
			Self::verify_receipt_logs_against_root(
				receipts_root,
				&proof_block.receipt_logs,
				&proof_block.receipt_proof,
			)?;
		}

		Ok(())
	}

	fn verify_account_storage_proof<MaxStorageSlots>(
		account_address: H160,
		proof: &EthereumAccountStorageProof<MaxStorageSlots>,
	) -> Result<(), EthereumVerifyError>
	where
		MaxStorageSlots: Get<u32>,
	{
		ensure!(!OperatingMode::<T>::get().is_halted(), EthereumVerifyError::VerifierUnavailable);

		let headers = Self::verify_execution_headers(&proof.execution_block_proof)?;
		let target = headers.first().ok_or(EthereumVerifyError::InvalidHeaderChain)?;
		Self::verify_account_storage_proof_against_state_root(
			target.state_root,
			account_address,
			proof,
		)
	}

	fn latest_execution_block_number() -> Option<EthereumBlockNumber> {
		let latest_block_hash = LatestExecutionHeaderAnchorBlockHash::<T>::get()?;
		ExecutionHeaderAnchors::<T>::get(latest_block_hash).map(|anchor| anchor.block_number)
	}

	fn latest_execution_block_timestamp() -> Option<Moment> {
		let latest_block_hash = LatestExecutionHeaderAnchorBlockHash::<T>::get()?;
		ExecutionHeaderAnchors::<T>::get(latest_block_hash).map(|anchor| anchor.timestamp_millis)
	}
}

fn receipt_contains_log(
	expected_log: &EthereumLog,
	receipt: &alloy_consensus::ReceiptEnvelope,
) -> bool {
	let expected_topics = expected_log.topics.as_slice();

	receipt.logs().iter().any(|receipt_log| {
		receipt_log.address.0 == expected_log.address.0 &&
			receipt_log.data.data.0.as_ref() == expected_log.data.as_slice() &&
			receipt_log
				.topics()
				.iter()
				.map(|topic| topic.0)
				.eq(expected_topics.iter().map(|topic| topic.0))
	})
}

impl<T: Config> Pallet<T> {
	fn verify_receipt_logs_against_root(
		receipts_root: ReceiptsRoot,
		receipt_logs: &[EthereumReceiptLog],
		receipt_proof: &EthereumCombinedReceiptProof,
	) -> Result<(), EthereumVerifyError> {
		let mut verified_receipts = BTreeMap::new();

		for receipt_log in receipt_logs {
			let receipt = match verified_receipts.get(&receipt_log.transaction_index) {
				Some(receipt) => receipt,
				None => {
					let receipt = verify_receipt_proof(
						receipts_root,
						receipt_proof,
						receipt_log.transaction_index,
					)
					.ok_or(EthereumVerifyError::InvalidProof)?;
					verified_receipts.insert(receipt_log.transaction_index, receipt);
					verified_receipts
						.get(&receipt_log.transaction_index)
						.expect("verified receipt was just inserted")
				},
			};
			ensure!(
				receipt_contains_log(&receipt_log.event_log, receipt),
				EthereumVerifyError::LogNotFound
			);
		}

		Ok(())
	}

	/// Verify ancestry and return the exact execution header used by a storage witness.
	pub fn verified_execution_header(
		proof: &EthereumExecutionBlockProof,
	) -> Result<ExecutionHeaderAnchor, EthereumVerifyError> {
		ensure!(!OperatingMode::<T>::get().is_halted(), EthereumVerifyError::VerifierUnavailable);
		Self::verify_execution_headers(proof)?
			.into_iter()
			.next()
			.ok_or(EthereumVerifyError::InvalidHeaderChain)
	}

	#[cfg(test)]
	pub(crate) fn verify_execution_block_proof(
		proof: &EthereumExecutionBlockProof,
	) -> Result<ReceiptsRoot, EthereumVerifyError> {
		let headers = Self::verify_execution_headers(proof)?;
		headers
			.first()
			.map(|header| header.receipts_root)
			.ok_or(EthereumVerifyError::InvalidProof)
	}

	fn verify_execution_headers(
		proof: &EthereumExecutionBlockProof,
	) -> Result<Vec<ExecutionHeaderAnchor>, EthereumVerifyError> {
		let anchor = ExecutionHeaderAnchors::<T>::get(proof.anchor_block_hash)
			.ok_or(EthereumVerifyError::AnchorNotFound)?;
		let mut verified_headers =
			Vec::with_capacity(proof.target_to_anchor_header_chain.len().saturating_add(1));

		if proof.target_to_anchor_header_chain.is_empty() {
			verified_headers.push(anchor);
			return Ok(verified_headers)
		}

		let mut headers = proof.target_to_anchor_header_chain.iter();
		let target = Self::decode_execution_header(
			headers.next().ok_or(EthereumVerifyError::InvalidHeaderChain)?,
		)?;
		verified_headers.push(target);
		let mut previous = target;

		for header in headers {
			let current = Self::decode_execution_header(header)?;
			ensure!(
				current.parent_hash == previous.block_hash &&
					current.block_number.checked_sub(1) == Some(previous.block_number),
				EthereumVerifyError::InvalidHeaderChain
			);
			verified_headers.push(current);
			previous = current;
		}

		ensure!(
			anchor.parent_hash == previous.block_hash &&
				anchor.block_number.checked_sub(1) == Some(previous.block_number),
			EthereumVerifyError::InvalidHeaderChain
		);
		verified_headers.push(anchor);

		Ok(verified_headers)
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
			timestamp_millis: decoded.timestamp.saturating_mul(1_000),
			// Recompute the sealed hash from the verified RLP bytes so the
			// header chain is anchored to the actual encoded header contents.
			block_hash: H256::from_slice(decoded.hash_slow().as_slice()),
			parent_hash: H256::from_slice(decoded.parent_hash.as_slice()),
			state_root: H256::from_slice(decoded.state_root.as_slice()),
			receipts_root: H256::from_slice(decoded.receipts_root.as_slice()),
		})
	}
}
