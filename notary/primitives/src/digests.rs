use codec::{Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{RuntimeDebug, H256};
use sp_runtime::{scale_info::TypeInfo, ConsensusEngineId};

use crate::BlockVotingPower;

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
	Default,
)]
pub struct BlockVoteDigest {
	/// The aggregate key of the notebooks parent keys and the parent notebooks' block vote roots
	pub parent_voting_key: Option<H256>,
	pub voting_power: BlockVotingPower,
	pub votes_count: u32,
	pub tick_notebooks: u32,
}
