use crate::{
	ensure, fork_power::ForkPower, tick::TickDigest, BlockSealAuthoritySignature, BlockVotingPower,
	NotebookAuditResult, VotingKey,
};
use alloc::{vec, vec::Vec};
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support_procedural::DefaultNoBound;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ed25519::Signature, RuntimeDebug, U256};
use sp_runtime::{ConsensusEngineId, Digest, DigestItem};

/// The block creator account_id - matches POW so that we can use the built-in front end decoding
pub const AUTHOR_DIGEST_ID: ConsensusEngineId = *b"pow_";
/// The "tick" for the block - matches aura to provide compatibility for aura slot information.
pub const TICK_DIGEST_ID: ConsensusEngineId = *b"aura";

/// Seal Digest ID for the high level block seal details - used to quickly check the seal
/// details in the node.
pub const BLOCK_SEAL_DIGEST_ID: ConsensusEngineId = *b"seal";

/// Key for the block vote digest in a block header
pub const BLOCK_VOTES_DIGEST_ID: ConsensusEngineId = *b"vote";
/// Key for the block vote digest in a block header
pub const NOTEBOOKS_DIGEST_ID: ConsensusEngineId = *b"book";
/// Parent Voting Key Digest
pub const PARENT_VOTING_KEY_DIGEST: ConsensusEngineId = *b"pkey";

/// Fork Power
pub const FORK_POWER_DIGEST: ConsensusEngineId = *b"powr";

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

impl TryFrom<DigestItem> for TickDigest {
	type Error = codec::Error;

	fn try_from(digest_item: DigestItem) -> Result<Self, Self::Error> {
		digest_item.as_tick().ok_or(codec::Error::from("Digest not found"))
	}
}

impl TryFrom<&Digest> for ForkPower {
	type Error = codec::Error;

	fn try_from(digest: &Digest) -> Result<Self, Self::Error> {
		for digest_item in digest.logs.iter() {
			if let Some(fork) = digest_item.as_fork_power() {
				return Ok(fork)
			}
		}
		Err(codec::Error::from("Digest not found"))
	}
}

#[derive(thiserror::Error, Debug, Copy, Clone)]
pub enum DecodeDigestError {
	#[error("Duplicate block vote digest")]
	DuplicateBlockVoteDigest,
	#[error("Duplicate block author")]
	DuplicateAuthorDigest,
	#[error("Duplicate tick digest")]
	DuplicateTickDigest,
	#[error("Duplicate parent voting key digest")]
	DuplicateParentVotingKeyDigest,
	#[error("Duplicate notebooks digest")]
	DuplicateNotebookDigest,
	#[error("Duplicate fork power digest")]
	DuplicateForkPowerDigest,
	#[error("Missing block author")]
	MissingBlockVoteDigest,
	#[error("Missing block vote digest")]
	MissingAuthorDigest,
	#[error("Missing tick digest")]
	MissingTickDigest,
	#[error("Missing fork power digest")]
	MissingParentVotingKeyDigest,
	#[error("Missing notebooks digest")]
	MissingNotebookDigest,
	#[error("Could not decode digest")]
	CouldNotDecodeDigest,
}

impl<NV, AC> TryFrom<Digest> for Digestset<NV, AC>
where
	NV: Codec + Clone,
	AC: Codec + Clone,
{
	type Error = DecodeDigestError;

	fn try_from(value: Digest) -> Result<Self, Self::Error> {
		let mut author = None;
		let mut block_vote = None;
		let mut voting_key = None;
		let mut fork_power = None;
		let mut tick = None;
		let mut notebooks = None;

		for digest_item in value.logs.iter() {
			if let Some(a) = digest_item.as_author() {
				ensure!(author.is_none(), DecodeDigestError::DuplicateAuthorDigest);
				author = Some(a);
			} else if let Some(bv) = digest_item.as_block_vote() {
				ensure!(block_vote.is_none(), DecodeDigestError::DuplicateBlockVoteDigest);
				block_vote = Some(bv);
			} else if let Some(t) = digest_item.as_tick() {
				ensure!(tick.is_none(), DecodeDigestError::DuplicateTickDigest);
				tick = Some(t);
			} else if let Some(n) = digest_item.as_notebooks() {
				ensure!(notebooks.is_none(), DecodeDigestError::DuplicateNotebookDigest);
				notebooks = Some(n);
			} else if let Some(vk) = digest_item.as_parent_voting_key() {
				ensure!(voting_key.is_none(), DecodeDigestError::DuplicateParentVotingKeyDigest);
				voting_key = Some(vk);
			} else if let Some(fp) = digest_item.as_fork_power() {
				ensure!(fork_power.is_none(), DecodeDigestError::DuplicateForkPowerDigest);
				fork_power = Some(fp);
			}
		}

		Ok(Digestset {
			author: author.ok_or(DecodeDigestError::MissingAuthorDigest)?,
			block_vote: block_vote.ok_or(DecodeDigestError::MissingBlockVoteDigest)?,
			voting_key,
			fork_power,
			tick: tick.ok_or(DecodeDigestError::MissingTickDigest)?,
			notebooks: notebooks.ok_or(DecodeDigestError::MissingNotebookDigest)?,
		})
	}
}

pub trait ArgonDigests {
	fn as_tick(&self) -> Option<TickDigest>;
	fn as_author<AC: Codec>(&self) -> Option<AC>;
	fn as_block_vote(&self) -> Option<BlockVoteDigest>;
	fn as_notebooks<VerifyError: Codec>(&self) -> Option<NotebookDigest<VerifyError>>;
	fn as_parent_voting_key(&self) -> Option<ParentVotingKeyDigest>;
	fn as_fork_power(&self) -> Option<ForkPower>;
	fn as_block_seal(&self) -> Option<BlockSealDigest>;
}

impl ArgonDigests for DigestItem {
	fn as_tick(&self) -> Option<TickDigest> {
		if let DigestItem::PreRuntime(TICK_DIGEST_ID, value) = self {
			return TickDigest::decode(&mut &value[..]).ok()
		}
		None
	}

	fn as_author<AC: Codec>(&self) -> Option<AC> {
		if let DigestItem::PreRuntime(AUTHOR_DIGEST_ID, value) = self {
			return AC::decode(&mut &value[..]).ok()
		}
		None
	}

	fn as_block_vote(&self) -> Option<BlockVoteDigest> {
		if let DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, value) = self {
			return BlockVoteDigest::decode(&mut &value[..]).ok()
		}
		None
	}

	fn as_notebooks<VerifyError: Codec>(&self) -> Option<NotebookDigest<VerifyError>> {
		if let DigestItem::PreRuntime(NOTEBOOKS_DIGEST_ID, value) = self {
			return NotebookDigest::<VerifyError>::decode(&mut &value[..]).ok()
		}
		None
	}

	fn as_parent_voting_key(&self) -> Option<ParentVotingKeyDigest> {
		if let DigestItem::Consensus(PARENT_VOTING_KEY_DIGEST, value) = self {
			return ParentVotingKeyDigest::decode(&mut &value[..]).ok()
		}
		None
	}

	fn as_fork_power(&self) -> Option<ForkPower> {
		if let DigestItem::Consensus(FORK_POWER_DIGEST, value) = self {
			return ForkPower::decode(&mut &value[..]).ok()
		}
		None
	}

	fn as_block_seal(&self) -> Option<BlockSealDigest> {
		if let DigestItem::Seal(BLOCK_SEAL_DIGEST_ID, value) = self {
			return BlockSealDigest::decode(&mut &value[..]).ok()
		}
		None
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
	// this is optional because it is generated in the runtime, so will not be available in a newly
	// created block
	pub fork_power: Option<ForkPower>,
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
