use codec::{Codec, Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{ConstU32, RuntimeDebug, H256, U256};
use sp_runtime::BoundedVec;
use sp_runtime::DispatchError;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
	block_seal::MiningAuthority,
	notary::{NotaryId, NotaryNotebookVoteDetails, NotaryNotebookVoteDigestDetails},
	tick::{Tick, Ticker},
	BestBlockVoteSeal, BlockVoteDigest, BlockVotingPower, NotebookNumber, VoteMinimum,
};

sp_api::decl_runtime_apis! {
	pub trait BlockSealApis<AccountId:Codec, BlockSealAuthorityId:Codec> {
		fn vote_minimum() -> VoteMinimum;
		fn compute_difficulty() -> u128;
		fn create_vote_digest(tick: Tick, included_notebooks: Vec<NotaryNotebookVoteDigestDetails>) -> BlockVoteDigest;
		fn find_vote_block_seals(
			votes: Vec<NotaryNotebookVotes>,
			with_better_strength: U256,
		) -> Result<BoundedVec<BestBlockVoteSeal<AccountId, BlockSealAuthorityId>, ConstU32<2>>, DispatchError>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait TickApis {
		fn current_tick() -> Tick;
		fn ticker() -> Ticker;
		fn blocks_at_tick(tick: Tick) -> Vec<Block::Hash>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait NotaryApis<NotaryRecord> where
		NotaryRecord: Codec + MaxEncodedLen{
		fn notary_by_id(notary_id: NotaryId) -> Option<NotaryRecord>;
		fn notaries() -> Vec<NotaryRecord>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait MiningApis<AccountId:Codec, BlockSealAuthorityId:Codec>{
		fn get_authority_id(account_id: &AccountId) -> Option<MiningAuthority<BlockSealAuthorityId,AccountId>>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait NotebookApis<VerifyError: Codec> {
		fn audit_notebook_and_get_votes(
			version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			header_hash: H256,
			vote_minimums: &BTreeMap<Block::Hash, VoteMinimum>,
			bytes: &Vec<u8>,
		) -> Result<NotaryNotebookVotes, VerifyError>;


		fn decode_signed_raw_notebook_header(raw_header: Vec<u8>) -> Result<NotaryNotebookVoteDetails<Block::Hash>, DispatchError>;

		fn latest_notebook_by_notary() -> BTreeMap<NotaryId, (NotebookNumber, Tick)>;
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebug)]
pub struct NotaryNotebookVotes {
	#[codec(compact)]
	pub notary_id: NotaryId,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	pub raw_votes: Vec<(Vec<u8>, BlockVotingPower)>,
}
