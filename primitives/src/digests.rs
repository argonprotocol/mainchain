use crate::{
	tick::Tick, BlockSealAuthoritySignature, BlockVotingPower, NotebookAuditResult, VotingKey,
};
use alloc::{vec, vec::Vec};
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support_procedural::DefaultNoBound;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ed25519::Signature, RuntimeDebug, U256};
use sp_runtime::{ConsensusEngineId, Digest, DigestItem};

/// The block creator account_id - matches POW so that we can use the built-in front end decoding
pub const AUTHOR_DIGEST_ID: ConsensusEngineId = [b'p', b'o', b'w', b'_'];

/// Seal Digest ID for the high level block seal details - used to quickly check the seal
/// details in the node.
pub const BLOCK_SEAL_DIGEST_ID: ConsensusEngineId = [b's', b'e', b'a', b'l'];

/// The tick of the given block
pub const TICK_DIGEST_ID: ConsensusEngineId = [b't', b'i', b'c', b'k'];

/// Key for the block vote digest in a block header
pub const BLOCK_VOTES_DIGEST_ID: ConsensusEngineId = [b'v', b'o', b't', b'e'];
/// Key for the block vote digest in a block header
pub const NOTEBOOKS_DIGEST_ID: ConsensusEngineId = [b'b', b'o', b'o', b'k'];
/// Parent Voting Key Digest
pub const PARENT_VOTING_KEY_DIGEST: ConsensusEngineId = [b'p', b'k', b'e', b'y'];

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BlockSealDigest {
	Vote { seal_strength: U256, signature: BlockSealAuthoritySignature },
	Compute { nonce: U256 },
}

impl BlockSealDigest {
	pub fn pre_final_vote(seal_strength: U256) -> Self {
		BlockSealDigest::Vote { seal_strength, signature: Signature::from_raw([0u8; 64]).into() }
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct TickDigest {
	#[codec(compact)]
	pub tick: Tick,
}

impl TryFrom<DigestItem> for TickDigest {
	type Error = codec::Error;

	fn try_from(digest_item: DigestItem) -> Result<Self, Self::Error> {
		if let DigestItem::PreRuntime(TICK_DIGEST_ID, value) = digest_item {
			return TickDigest::decode(&mut &value[..])
		}
		Err(codec::Error::from("Digest not found"))
	}
}

impl BlockSealDigest {
	pub fn is_vote(&self) -> bool {
		matches!(self, BlockSealDigest::Vote { .. })
	}

	pub fn to_digest(&self) -> DigestItem {
		DigestItem::Seal(BLOCK_SEAL_DIGEST_ID, self.encode())
	}

	pub fn is_seal(digest_item: &DigestItem) -> bool {
		if let DigestItem::Seal(id, _) = digest_item {
			if id == &BLOCK_SEAL_DIGEST_ID {
				return true
			}
		}
		false
	}

	pub fn try_from(digest_item: &DigestItem) -> Option<Self> {
		digest_item.seal_try_to::<Self>(&BLOCK_SEAL_DIGEST_ID)
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
	pub notebooks: Vec<NotebookAuditResult<VerifyError>>,
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
pub struct NotebookHeaderData<VerifyError: Codec> {
	pub signed_headers: Vec<Vec<u8>>,
	pub notebook_digest: NotebookDigest<VerifyError>,
	pub vote_digest: BlockVoteDigest,
}

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct Digestset<NotebookVerifyError: Codec + Clone, AccountId: Codec + Clone> {
	pub author: AccountId,
	pub block_vote: BlockVoteDigest,
	// this is optional because it is generated in the runtime, so will not be available in a newly
	// created block
	pub voting_key: Option<ParentVotingKeyDigest>,
	pub tick: TickDigest,
	pub notebooks: NotebookDigest<NotebookVerifyError>,
}

impl<N: Codec + Clone, AC: Codec + Clone> Digestset<N, AC> {
	pub fn create_pre_runtime_digest(&self) -> Digest {
		Digest {
			logs: vec![
				DigestItem::PreRuntime(AUTHOR_DIGEST_ID, self.author.encode()),
				DigestItem::PreRuntime(TICK_DIGEST_ID, self.tick.encode()),
				DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, self.block_vote.encode()),
				DigestItem::PreRuntime(NOTEBOOKS_DIGEST_ID, self.notebooks.encode()),
			],
		}
	}
}
