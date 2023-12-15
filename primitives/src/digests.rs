use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{RuntimeDebug, H256, U256};
use sp_runtime::{
	traits::{Block as BlockT, NumberFor},
	ConsensusEngineId,
};

use crate::BlockVotingPower;

/// The block creator account_id - matches POW so that we can use the built-in front end decoding
pub const AUTHOR_DIGEST_ID: ConsensusEngineId = [b'p', b'o', b'w', b'_'];

/// Seal Digest ID for the high level block seal details - used to quickly check the seal
/// details in the node.
pub const BLOCK_SEAL_DIGEST_ID: ConsensusEngineId = [b's', b'e', b'a', b'l'];

/// The finalized block needed to sync (FinalizedBlockNeededDigest)
pub const FINALIZED_BLOCK_DIGEST_ID: ConsensusEngineId = [b'f', b'i', b'n', b'_'];

/// The tick of the given block
pub const TICK_DIGEST_ID: ConsensusEngineId = [b't', b'i', b'c', b'k'];

/// Key for the block vote digest in a block header
pub const BLOCK_VOTES_DIGEST_ID: ConsensusEngineId = [b'v', b'o', b't', b'e'];

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct FinalizedBlockNeededDigest<B: BlockT> {
	#[codec(compact)]
	pub number: NumberFor<B>,
	pub hash: B::Hash,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BlockSealDigest {
	Vote { vote_proof: U256 },
	Compute { nonce: U256 },
}

impl BlockSealDigest {
	pub fn is_tax(&self) -> bool {
		match self {
			BlockSealDigest::Vote { .. } => true,
			_ => false,
		}
	}
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
	Default,
)]
pub struct BlockVoteDigest {
	/// The aggregate key of the notebooks parent keys and the parent notebooks' block vote roots
	pub parent_voting_key: Option<H256>,
	pub voting_power: BlockVotingPower,
	pub votes_count: u32,
	pub tick_notebooks: u32,
}
