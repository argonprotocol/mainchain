use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use sp_core::{H160, H256};

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct EthereumLog {
	pub address: H160,
	pub topics: Vec<H256>,
	pub data: Vec<u8>,
}

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct EthereumReceiptProof {
	/// Transaction index used as the receipt-trie key.
	#[codec(compact)]
	pub transaction_index: u64,
	/// Merkle Patricia Trie proof nodes for the Ethereum transaction receipt.
	pub nodes: Vec<Vec<u8>>,
}

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct EthereumExecutionHeader {
	/// RLP-encoded Ethereum execution block header.
	pub rlp: Vec<u8>,
}

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct EthereumExecutionBlockProof {
	pub anchor_block_hash: H256,
	/// Headers ordered from the burn block toward the retained anchor.
	///
	/// Empty means the burn block is the retained anchor. Otherwise, the first header is the burn
	/// block and the final supplied header must be the retained anchor's parent.
	pub target_to_anchor_header_chain: Vec<EthereumExecutionHeader>,
}

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct EthereumProof {
	pub execution_block_proof: EthereumExecutionBlockProof,
	pub receipt_proof: EthereumReceiptProof,
}

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum EthereumVerifyError {
	VerifierUnavailable,
	AnchorNotFound,
	InvalidHeader,
	InvalidHeaderChain,
	LogNotFound,
	InvalidProof,
}
