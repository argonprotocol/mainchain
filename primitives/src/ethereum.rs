use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{CloneNoBound, DebugNoBound, EqNoBound, PartialEqNoBound};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256};
use sp_runtime::{
	traits::{ConstU32, Get},
	BoundedVec,
};

/// Ethereum logs support up to four indexed topics.
pub const MAX_ETHEREUM_LOG_TOPICS: u32 = 4;
/// Burn-proof verification only needs modest log payload headroom above the 96-byte burn event.
pub const MAX_ETHEREUM_LOG_DATA_BYTES: u32 = 1_024;
/// Combined receipt proofs de-duplicate trie nodes shared across several receipts in one block.
pub const MAX_ETHEREUM_COMBINED_RECEIPT_PROOF_NODES: u32 = 128;
/// Individual receipt trie proof nodes can be large RLP blobs, but not unbounded.
pub const MAX_ETHEREUM_RECEIPT_PROOF_NODE_BYTES: u32 = 2_048;
/// A combined receipt proof can reference several receipts from one execution block.
pub const MAX_ETHEREUM_RECEIPTS_PER_PROOF: u32 = 32;
/// Any one receipt path should stay comfortably below this number of shared proof nodes.
pub const MAX_ETHEREUM_RECEIPT_PROOF_NODE_REFS: u32 = 32;
/// RLP-encoded execution headers should fit comfortably within a small bounded envelope.
pub const MAX_ETHEREUM_EXECUTION_HEADER_RLP_BYTES: u32 = 2_048;
/// Execution header chains should stay close to the relayed retained anchor cadence.
pub const MAX_ETHEREUM_HEADER_CHAIN_LEN: u32 = 64;

/// Canonical Ethereum execution-block height used anywhere Argon stores or compares expiry and
/// proof targets.
pub type EthereumBlockNumber = u64;
pub type EthereumLogTopics = BoundedVec<H256, ConstU32<MAX_ETHEREUM_LOG_TOPICS>>;
pub type EthereumLogData = BoundedVec<u8, ConstU32<MAX_ETHEREUM_LOG_DATA_BYTES>>;
pub type EthereumReceiptProofNode = BoundedVec<u8, ConstU32<MAX_ETHEREUM_RECEIPT_PROOF_NODE_BYTES>>;
pub type EthereumCombinedReceiptProofNodes =
	BoundedVec<EthereumReceiptProofNode, ConstU32<MAX_ETHEREUM_COMBINED_RECEIPT_PROOF_NODES>>;
pub type EthereumReceiptProofNodeIndexes =
	BoundedVec<u16, ConstU32<MAX_ETHEREUM_RECEIPT_PROOF_NODE_REFS>>;
pub type EthereumCombinedReceiptProofReceipts =
	BoundedVec<EthereumReceiptProofReceipt, ConstU32<MAX_ETHEREUM_RECEIPTS_PER_PROOF>>;
pub type EthereumExecutionHeaderRlp =
	BoundedVec<u8, ConstU32<MAX_ETHEREUM_EXECUTION_HEADER_RLP_BYTES>>;
pub type EthereumExecutionHeaderChain =
	BoundedVec<EthereumExecutionHeader, ConstU32<MAX_ETHEREUM_HEADER_CHAIN_LEN>>;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	Serialize,
	Deserialize,
)]
pub enum EthereumBeaconPreset {
	#[default]
	Mainnet,
	Minimal,
}

impl EthereumBeaconPreset {
	pub const fn slots_per_epoch(self) -> usize {
		match self {
			Self::Mainnet => 32,
			Self::Minimal => 8,
		}
	}

	pub const fn epochs_per_sync_committee_period(self) -> usize {
		match self {
			Self::Mainnet => 256,
			Self::Minimal => 8,
		}
	}

	pub const fn sync_committee_size(self) -> usize {
		match self {
			Self::Mainnet => 512,
			Self::Minimal => 32,
		}
	}

	pub const fn sync_committee_bits_size(self) -> usize {
		self.sync_committee_size() / 8
	}

	pub const fn slots_per_historical_root(self) -> usize {
		match self {
			Self::Mainnet => 8192,
			Self::Minimal => 64,
		}
	}
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct EthereumLog {
	pub address: H160,
	pub topics: EthereumLogTopics,
	pub data: EthereumLogData,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct EthereumReceiptProofReceipt {
	/// Transaction index used as the receipt-trie key.
	#[codec(compact)]
	pub transaction_index: u64,
	/// Ordered indexes into the shared combined proof-node set for this receipt.
	pub node_indexes: EthereumReceiptProofNodeIndexes,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct EthereumCombinedReceiptProof {
	/// Shared Merkle Patricia Trie proof nodes for all receipts included in this proof block.
	pub nodes: EthereumCombinedReceiptProofNodes,
	/// Receipt-specific views into the shared node set.
	pub receipts: EthereumCombinedReceiptProofReceipts,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct EthereumExecutionHeader {
	/// RLP-encoded Ethereum execution block header.
	pub rlp: EthereumExecutionHeaderRlp,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct EthereumExecutionBlockProof {
	pub anchor_block_hash: H256,
	/// Headers ordered from the burn block toward the retained anchor.
	///
	/// Empty means the burn block is the retained anchor. Otherwise, the first header is the burn
	/// block and the final supplied header must be the retained anchor's parent.
	pub target_to_anchor_header_chain: EthereumExecutionHeaderChain,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct EthereumReceiptLog {
	/// Transaction index used as the receipt-trie key.
	#[codec(compact)]
	pub transaction_index: u64,
	/// Raw Ethereum log proven from that receipt.
	pub event_log: EthereumLog,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
	DebugNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxReceiptLogs))]
pub struct EthereumReceiptLogProofBlock<MaxReceiptLogs: Get<u32>> {
	/// Target execution block number for this block's receipt proof.
	#[codec(compact)]
	pub target_block_number: EthereumBlockNumber,
	pub receipt_proof: EthereumCombinedReceiptProof,
	pub receipt_logs: BoundedVec<EthereumReceiptLog, MaxReceiptLogs>,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
	DebugNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxProofBlocks, MaxReceiptLogs))]
pub struct EthereumReceiptLogProofBatch<MaxProofBlocks: Get<u32>, MaxReceiptLogs: Get<u32>> {
	pub execution_block_proof: EthereumExecutionBlockProof,
	pub blocks: BoundedVec<EthereumReceiptLogProofBlock<MaxReceiptLogs>, MaxProofBlocks>,
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
