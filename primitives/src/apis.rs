use codec::{Codec, Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_api::BlockT;
use sp_core::{ConstU32, RuntimeDebug, H256, U256};
use sp_runtime::{BoundedVec, DispatchError};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use ulx_notary_primitives::{
	AccountId, BestBlockVoteProofT, BlockVoteDigest, BlockVotingPower, NotebookNumber, VoteMinimum,
};
pub use ulx_notary_primitives::{MerkleProof, NotaryId};

use crate::{
	block_seal::MiningAuthority,
	notary::{NotaryNotebookVoteDetails, NotaryNotebookVoteDigestDetails},
	tick::{Tick, Ticker},
	BlockSealAuthorityId,
};

sp_api::decl_runtime_apis! {
	pub trait MiningAuthorityApis {
		fn xor_closest_authority(nonce: U256) -> Option<MiningAuthority<BlockSealAuthorityId, AccountId>>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait BlockSealSpecApis {
		fn vote_minimum() -> VoteMinimum;
		fn compute_difficulty() -> u128;
		fn parent_voting_key() -> Option<H256>;
		fn create_vote_digest(tick_notebooks: Vec<NotaryNotebookVoteDigestDetails>) -> BlockVoteDigest;
	}
}

sp_api::decl_runtime_apis! {
	pub trait TickApis {
		fn current_tick() -> Tick;
		fn ticker() -> Ticker;
	}
}

sp_api::decl_runtime_apis! {
	pub trait NotaryApis<NotaryRecord> where
		NotaryRecord: Codec + MaxEncodedLen{
		fn notary_by_id(notary_id: NotaryId) -> Option<NotaryRecord>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait NotebookApis {
		fn audit_notebook_and_get_votes(
			version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			header_hash: H256,
			vote_minimums: &BTreeMap<Block::Hash, VoteMinimum>,
			bytes: &Vec<u8>,
		) -> Result<NotebookVotes, ulx_notary_audit::VerifyError>;

		fn get_best_vote_proofs(
			votes: &BTreeMap<NotaryId, NotebookVotes>,
		) -> Result<BoundedVec<BestBlockVoteProofT<Block::Hash>, ConstU32<2>>, DispatchError>;

		fn decode_notebook_vote_details(extrinsic: &<Block as BlockT>::Extrinsic) -> Option<NotaryNotebookVoteDetails<Block::Hash>>;
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebug)]
pub struct NotebookVotes {
	pub raw_votes: Vec<(Vec<u8>, BlockVotingPower)>,
}
