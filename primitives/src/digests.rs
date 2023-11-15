use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_api::BlockT;
use sp_core::{RuntimeDebug, U256};
use sp_runtime::{traits::NumberFor, ConsensusEngineId};

pub use ulx_notary_primitives::{
	digests::{
		BlockVoteDigest, NotaryNotebookDigest, BLOCK_VOTES_DIGEST_ID,
		NEXT_VOTE_ELIGIBILITY_DIGEST_ID,
	},
	BlockVoteSource,
};

use crate::block_seal::BlockSealAuthoritySignature;

/// The block creator account_id - matches POW so that we can use the built-in front end decoding
pub const AUTHOR_DIGEST_ID: ConsensusEngineId = [b'p', b'o', b'w', b'_'];

/// Pre-runtime Digest ID for the high level block seal details - used to quickly check the seal
/// details in the node.
pub const BLOCK_SEAL_DIGEST_ID: ConsensusEngineId = [b's', b'e', b'a', b'l'];

/// The finalized block needed to sync (FinalizedBlockNeededDigest)
pub const FINALIZED_BLOCK_DIGEST_ID: ConsensusEngineId = [b'f', b'i', b'n', b'_'];

/// A Seal digest item that contains the signature of the block by the author id
pub const SIGNATURE_DIGEST_ID: ConsensusEngineId = [b's', b'i', b'g', b'n'];
/// Stores a temporary authority during compute phase
pub const COMPUTE_AUTHORITY_DIGEST_ID: ConsensusEngineId = [b'a', b'u', b't', b'h'];

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct FinalizedBlockNeededDigest<B: BlockT> {
	#[codec(compact)]
	pub number: NumberFor<B>,
	pub hash: B::Hash,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BlockSealDigest {
	pub nonce: U256,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct BlockSealSignatureDigest {
	pub signature: BlockSealAuthoritySignature,
}
