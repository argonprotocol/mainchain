use crate::{BlockVotingPower, NotaryId, NotebookNumber, MAX_NOTARIES};
use codec::{Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{bounded::BoundedVec, ConstU32, RuntimeDebug, H256};
use sp_runtime::{scale_info::TypeInfo, ConsensusEngineId};

pub const NEXT_VOTE_ELIGIBILITY_DIGEST_ID: ConsensusEngineId = [b'n', b'e', b'x', b't'];

/// Key for the block vote digest in a block header
pub const BLOCK_VOTES_DIGEST_ID: ConsensusEngineId = [b'v', b'o', b't', b'e'];

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
pub struct BlockVoteDigest {
	/// The aggregate key of the notebooks parent keys and the parent notebooks' block vote roots
	pub parent_voting_key: Option<H256>,
	pub notebook_numbers: BoundedVec<NotaryNotebookDigest, ConstU32<MAX_NOTARIES>>,
	pub voting_power: BlockVotingPower,
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
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
pub struct NotaryNotebookDigest {
	pub notary_id: NotaryId,
	pub notebook_number: NotebookNumber,
}
