use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_api::BlockT;
use sp_core::{MaxEncodedLen, RuntimeDebug};
use sp_runtime::{traits::NumberFor, ConsensusEngineId};

use crate::block_seal::BlockSealAuthoritySignature;

pub const ULX_ENGINE_ID: ConsensusEngineId = [b'u', b'l', b'x', b'_'];
// matches POW so that we can use the built-in front end decoding
pub const AUTHOR_ID: ConsensusEngineId = [b'p', b'o', b'w', b'_'];

pub const FINALIZED_BLOCK_DIGEST_ID: ConsensusEngineId = [b'f', b'i', b'n', b'_'];

pub type Difficulty = u128;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProofOfWorkType {
	Tax,
	Compute,
}

impl Default for ProofOfWorkType {
	fn default() -> Self {
		ProofOfWorkType::Compute
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct FinalizedBlockNeededDigest<B: BlockT> {
	#[codec(compact)]
	pub number: NumberFor<B>,
	pub hash: B::Hash,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UlxSeal {
	/// How much did we ease difficulty due to tax provided
	#[codec(compact)]
	pub easing: Difficulty,
	pub nonce: [u8; 32],
	/// authority index and signature of miner sealing the block
	pub authority: Option<(u16, BlockSealAuthoritySignature)>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UlxPreDigest {
	pub work_type: ProofOfWorkType,
	#[codec(compact)]
	pub difficulty: Difficulty,
}
