use codec::{Decode, Encode};
use frame_support::{Deserialize, Serialize};
use scale_info::TypeInfo;
use sp_api::BlockT;
use sp_core::{RuntimeDebug, U256};
use sp_runtime::{traits::NumberFor, ConsensusEngineId};

pub use ulx_notary_primitives::digests::{
	BlockVoteDigest, NotaryNotebookDigest, BLOCK_VOTES_DIGEST_ID,
};
use ulx_notary_primitives::VoteMinimum;

use crate::{
	block_seal::BlockSealAuthoritySignature,
	runtime_decl_for_block_seal_minimum_apis::MaxEncodedLen, ComputeDifficulty,
};

/// The block creator account_id - matches POW so that we can use the built-in front end decoding
pub const AUTHOR_DIGEST_ID: ConsensusEngineId = [b'p', b'o', b'w', b'_'];

/// The block creator authority id
pub const AUTHORITY_DIGEST_ID: ConsensusEngineId = [b'a', b'u', b't', b'h'];

/// Seal Digest ID for the high level block seal details - used to quickly check the seal
/// details in the node.
pub const BLOCK_SEAL_DIGEST_ID: ConsensusEngineId = [b's', b'e', b'a', b'l'];

/// The finalized block needed to sync (FinalizedBlockNeededDigest)
pub const FINALIZED_BLOCK_DIGEST_ID: ConsensusEngineId = [b'f', b'i', b'n', b'_'];
pub const NEXT_SEAL_MINIMUMS_DIGEST_ID: ConsensusEngineId = [b'n', b'e', b'x', b't'];

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct FinalizedBlockNeededDigest<B: BlockT> {
	#[codec(compact)]
	pub number: NumberFor<B>,
	pub hash: B::Hash,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BlockSealDigest {
	pub nonce: U256,
	pub seal_source: SealSource,
	pub signature: BlockSealAuthoritySignature,
}

impl BlockSealDigest {
	pub fn is_tax(&self) -> bool {
		self.seal_source == SealSource::Vote
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum SealSource {
	Vote,
	Compute,
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
)]
pub struct BlockSealMinimumsDigest {
	pub vote_minimum: VoteMinimum,
	pub compute_difficulty: ComputeDifficulty,
}
