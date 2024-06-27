use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support_procedural::DefaultNoBound;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{RuntimeDebug, U256};
use sp_runtime::{
	traits::{Block as BlockT, NumberFor},
	ConsensusEngineId,
};
use sp_std::vec::Vec;

use crate::{tick::Tick, BlockVotingPower, NotaryId, NotebookNumber, VotingKey};

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
/// Key for the block vote digest in a block header
pub const NOTEBOOKS_DIGEST_ID: ConsensusEngineId = [b'b', b'o', b'o', b'k'];
/// Parent Voting Key Digest
pub const PARENT_VOTING_KEY_DIGEST: ConsensusEngineId = [b'p', b'k', b'e', b'y'];

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct FinalizedBlockNeededDigest<B: BlockT> {
	#[codec(compact)]
	pub number: NumberFor<B>,
	pub hash: B::Hash,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BlockSealDigest {
	Vote { seal_strength: U256 },
	Compute { nonce: U256 },
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct TickDigest {
	#[codec(compact)]
	pub tick: Tick,
}

impl BlockSealDigest {
	pub fn is_tax(&self) -> bool {
		matches!(self, BlockSealDigest::Vote { .. })
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
	#[codec(compact)]
	pub voting_power: BlockVotingPower,
	#[codec(compact)]
	pub votes_count: u32,
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	Serialize,
	Deserialize,
	DefaultNoBound,
)]
pub struct ParentVotingKeyDigest {
	pub parent_voting_key: Option<VotingKey>,
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	Serialize,
	Deserialize,
	DefaultNoBound,
)]
pub struct NotebookDigest<VerifyError: Codec> {
	pub notebooks: Vec<NotebookDigestRecord<VerifyError>>,
}
#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	Serialize,
	Deserialize,
	DefaultNoBound,
)]
pub struct NotebookHeaderData<VerifyError: Codec, BlockNumber: Default + Codec> {
	pub signed_headers: Vec<Vec<u8>>,
	pub notebook_digest: NotebookDigest<VerifyError>,
	pub vote_digest: BlockVoteDigest,
	pub latest_finalized_block_needed: BlockNumber,
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
pub struct NotebookDigestRecord<VerifyError: Codec> {
	#[codec(compact)]
	pub notary_id: NotaryId,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	#[codec(compact)]
	pub tick: Tick,
	pub audit_first_failure: Option<VerifyError>,
}
