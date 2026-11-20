use crate::{bitcoin::LiquidId, tick::Tick, AccountId, Balance, Moment, Signature};
use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{CloneNoBound, DebugNoBound, EqNoBound, PartialEqNoBound};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256, U256};
use sp_runtime::{
	traits::{ConstU32, Get, Verify},
	BoundedVec,
};

/// Uniswap's square root of the raw token1/token0 ratio, scaled by 2^96.
/// Ethereum represents it as a uint160.
pub type UniswapSqrtPriceX96 = U256;

/// ERC-721 token ID within the Uniswap PositionManager contract that minted it.
pub type UniswapNftId = U256;

/// Signed Uniswap price-grid index. Ethereum represents it as an int24.
pub type UniswapTick = i32;

/// The Ethereum venue of a Uniswap v4 pool.
#[derive(
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct UniswapPoolIdentity {
	/// Ethereum execution chain ID.
	pub chain_id: u64,
	/// PoolManager contract on that chain.
	pub pool_manager: H160,
	/// Native pool ID within the manager, committing to the tokens, fee and hook.
	pub pool_id: H256,
}

/// The approved source of a stabilizer pool price observation.
#[derive(
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct StabilizerPoolPriceSource {
	/// Revision matched against the oracle submission when it was accepted.
	pub revision: u32,
	/// Ethereum venue bound to this revision.
	pub pool: UniswapPoolIdentity,
	/// Whether ARGON is token0 of this pool. Fixed by the approved revision.
	pub argon_is_token0: bool,
}

/// Authenticated Uniswap v4 purchase facts. Principal is the microgons actually delivered on Argon;
/// USDC spending includes fees and excludes refunded input. The gateway owns proof and replay
/// checks.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct UniswapPurchase {
	/// Authenticated position in the Ethereum gateway activity stream.
	pub gateway_activity_nonce: u64,
	/// Nonzero Liquid selected at purchase time; None means an ordinary buy.
	pub repay_liquid_id: Option<LiquidId>,
	/// Ethereum execution chain ID; scoped together with the pool manager and pool ID.
	pub chain_id: u64,
	/// Uniswap v4 PoolManager contract that executed the authenticated swap.
	pub pool_manager: H160,
	/// Pool identity within the source chain's manager, committing to tokens, fee and hook.
	pub pool_id: H256,
	/// Source-chain USDC contract whose actual spending is reported below.
	pub usdc_token: H160,
	/// Actual spending in six-decimal USDC units, including fees and excluding refunds.
	pub usdc_spent: U256,
	/// Whole microgons actually delivered to the Argon recipient, after Ethereum unit conversion.
	pub principal: Balance,
	/// sqrt(raw token1 / raw token0) immediately before execution, scaled by 2^96; uint160.
	pub sqrt_price_before_x96: UniswapSqrtPriceX96,
	/// The same pool's square-root price immediately after execution, with the same scaling.
	pub sqrt_price_after_x96: UniswapSqrtPriceX96,
	/// Authenticated Ethereum purchase time mapped to an Argon tick, independent of relay arrival.
	pub tick: Tick,
}

/// An Argon account authorizes repayment buys from one Ethereum wallet. The gateway
/// authenticates that wallet as the sender; authorization can be reused for its later buys.
pub struct LiquidRepayAuthorization {
	/// Ethereum wallet allowed to select the signing Argon account's Liquids for repayment.
	pub ethereum_sender: H160,
}

impl LiquidRepayAuthorization {
	/// Domain-separated bytes signed by the Argon account, containing the raw 20-byte sender.
	pub fn signing_payload(&self) -> Vec<u8> {
		[b"argon-liquid-repay:".as_slice(), self.ethereum_sender.as_bytes()].concat()
	}

	/// Verify the canonical signing payload against the Argon account being authorized.
	pub fn verify(&self, signature: &Signature, account: &AccountId) -> bool {
		signature.verify(self.signing_payload().as_slice(), account)
	}
}

/// Ethereum logs support up to four indexed topics.
pub const MAX_ETHEREUM_LOG_TOPICS: u32 = 4;
/// Largest gateway event: TransferOutOfArgonFinalized's 288-byte ABI head plus 25
/// minting-authority collateral rows of 96 bytes each.
pub const MAX_ETHEREUM_LOG_DATA_BYTES: u32 = 288 + 25 * 96;
/// Combined receipt proofs de-duplicate trie nodes shared across several receipts in one block.
pub const MAX_ETHEREUM_COMBINED_RECEIPT_PROOF_NODES: u32 = 128;
/// Room for the largest gateway event plus the receipt bloom, topics, token-transfer logs,
/// and trie-leaf RLP overhead. This is a receipt envelope, not an account/storage-node bound.
/// Arbitrarily batched receipts can exceed it; storage proofs avoid carrying the whole receipt.
pub const MAX_ETHEREUM_RECEIPT_PROOF_NODE_BYTES: u32 = 4_096;
/// Account/storage tries use fixed-length hashed keys. Their largest node is a branch
/// containing sixteen 33-byte child hashes, an empty value, and a three-byte list prefix.
pub const MAX_ETHEREUM_TRIE_PROOF_NODE_BYTES: u32 = 16 * 33 + 1 + 3;
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

/// Block authenticated by an Ethereum storage proof.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct EthereumProofBlock {
	/// Authenticated Ethereum execution-block hash.
	pub hash: H256,
	/// Height of the authenticated execution block.
	pub number: EthereumBlockNumber,
	/// Ethereum block time in Unix milliseconds, authenticated by the execution header.
	pub timestamp_millis: Moment,
}

pub type EthereumLogTopics = BoundedVec<H256, ConstU32<MAX_ETHEREUM_LOG_TOPICS>>;
pub type EthereumLogData = BoundedVec<u8, ConstU32<MAX_ETHEREUM_LOG_DATA_BYTES>>;
pub type EthereumReceiptProofNode = BoundedVec<u8, ConstU32<MAX_ETHEREUM_RECEIPT_PROOF_NODE_BYTES>>;
pub type EthereumCombinedReceiptProofNodes =
	BoundedVec<EthereumReceiptProofNode, ConstU32<MAX_ETHEREUM_COMBINED_RECEIPT_PROOF_NODES>>;
pub type EthereumReceiptProofNodeIndexes =
	BoundedVec<u16, ConstU32<MAX_ETHEREUM_RECEIPT_PROOF_NODE_REFS>>;
pub type EthereumCombinedReceiptProofReceipts =
	BoundedVec<EthereumReceiptProofReceipt, ConstU32<MAX_ETHEREUM_RECEIPTS_PER_PROOF>>;
pub type EthereumTrieProofNodes = BoundedVec<
	BoundedVec<u8, ConstU32<MAX_ETHEREUM_TRIE_PROOF_NODE_BYTES>>,
	ConstU32<MAX_ETHEREUM_COMBINED_RECEIPT_PROOF_NODES>,
>;
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
	/// Headers ordered from the proven execution block toward the retained anchor.
	///
	/// Empty means the proven block is the retained anchor. Otherwise, the first header is the
	/// proven block and the final supplied header must be the retained anchor's parent.
	pub target_to_anchor_header_chain: EthereumExecutionHeaderChain,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
)]
/// Proof of one storage slot in an Ethereum account's storage trie.
pub struct EthereumStorageSlotProof {
	/// Solidity storage-slot key before the storage-trie path hash is applied.
	pub slot: H256,
	/// Expected 32-byte value stored at that slot. Zero denotes an absent/zero slot.
	pub value: H256,
	/// Ordered indexes into the shared storage proof-node set for this slot.
	pub node_indexes: EthereumReceiptProofNodeIndexes,
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
#[scale_info(skip_type_params(MaxStorageSlots))]
/// Proof of one Ethereum account plus a bounded set of storage slots at an execution block
/// authenticated by a retained header anchor.
///
/// `account_proof` proves the account in the Ethereum state trie rooted at the proven execution
/// header's `state_root`. That account yields the account's `storage_root`.
///
/// `storage_proof` is then one shared node set from that account's storage trie. Each entry in
/// `slots` selects the nodes within that shared proof needed to verify one storage slot.
pub struct EthereumAccountStorageProof<MaxStorageSlots: Get<u32>> {
	/// Authenticates the execution header whose `state_root` this proof is checked against.
	pub execution_block_proof: EthereumExecutionBlockProof,
	/// State-trie proof for the target account.
	pub account_proof: EthereumTrieProofNodes,
	/// Shared storage-trie proof nodes for the proven storage slots.
	pub storage_proof: EthereumTrieProofNodes,
	pub slots: BoundedVec<EthereumStorageSlotProof, MaxStorageSlots>,
}

/// Account and storage trie paths checked against a separately authenticated execution block.
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
#[scale_info(skip_type_params(MaxStorageSlots))]
pub struct EthereumAccountStorageWitness<MaxStorageSlots: Get<u32>> {
	/// State-trie path to the contract account and its storage root.
	pub account_proof: EthereumTrieProofNodes,
	/// Shared storage-trie nodes referenced by the proven slots.
	pub storage_proof: EthereumTrieProofNodes,
	/// Solidity slot keys, expected values and paths into the shared nodes.
	pub slots: BoundedVec<EthereumStorageSlotProof, MaxStorageSlots>,
}

/// Borrowed trie evidence lets accounts with different slot bounds share one block verification.
#[derive(Clone, Copy)]
pub struct EthereumAccountStorageWitnessRef<'a> {
	/// State-trie path authenticating the contract account.
	pub account_proof: &'a EthereumTrieProofNodes,
	/// Shared storage-trie nodes for this account.
	pub storage_proof: &'a EthereumTrieProofNodes,
	/// Slot keys, expected values and paths into the shared nodes.
	pub slots: &'a [EthereumStorageSlotProof],
}

impl<'a, S: Get<u32>> From<&'a EthereumAccountStorageProof<S>>
	for EthereumAccountStorageWitnessRef<'a>
{
	fn from(proof: &'a EthereumAccountStorageProof<S>) -> Self {
		Self {
			account_proof: &proof.account_proof,
			storage_proof: &proof.storage_proof,
			slots: &proof.slots,
		}
	}
}

impl<'a, S: Get<u32>> From<&'a EthereumAccountStorageWitness<S>>
	for EthereumAccountStorageWitnessRef<'a>
{
	fn from(proof: &'a EthereumAccountStorageWitness<S>) -> Self {
		Self {
			account_proof: &proof.account_proof,
			storage_proof: &proof.storage_proof,
			slots: &proof.slots,
		}
	}
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
