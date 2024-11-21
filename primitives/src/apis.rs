#![allow(clippy::ptr_arg)]
#![allow(clippy::too_many_arguments)]

use crate::{
	bitcoin::{BitcoinNetwork, BitcoinSyncStatus, Satoshis, UtxoRef, UtxoValue},
	block_seal::{ComputePuzzle, MiningAuthority},
	notary::{
		NotaryId, NotaryNotebookAuditSummary, NotaryNotebookDetails, NotaryNotebookRawVotes,
		NotaryNotebookVoteDigestDetails,
	},
	tick::{Tick, Ticker},
	BestBlockVoteSeal, BlockSealDigest, BlockVoteDigest, NotebookAuditResult, NotebookNumber,
	VoteMinimum, VotingKey,
};
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use codec::Codec;
use sp_core::{ConstU32, H256, U256};
use sp_runtime::{BoundedVec, Digest, DispatchError};

sp_api::decl_runtime_apis! {
	pub trait BlockSealApis<AccountId:Codec, BlockSealAuthorityId:Codec> {
		fn vote_minimum() -> VoteMinimum;
		fn compute_puzzle() -> ComputePuzzle<Block>;
		fn create_vote_digest(notebook_tick: Tick, included_notebooks: Vec<NotaryNotebookVoteDigestDetails>) -> BlockVoteDigest;
		fn find_vote_block_seals(
			votes: Vec<NotaryNotebookRawVotes>,
			with_better_strength: U256,
			expected_notebook_tick: Tick,
		) -> Result<BoundedVec<BestBlockVoteSeal<AccountId, BlockSealAuthorityId>, ConstU32<2>>, DispatchError>;
		fn has_eligible_votes() -> bool;
		fn is_valid_signature(block_hash: Block::Hash, seal: &BlockSealDigest, digest: &Digest) -> bool;
	}
}

sp_api::decl_runtime_apis!(
	pub trait BlockCreatorApis<AccountId: Codec, VerifyError: Codec> {
		fn decode_voting_author(
			digests: &Digest,
		) -> Result<(AccountId, Tick, Option<VotingKey>), DispatchError>;
		fn digest_notebooks(
			digests: &Digest,
		) -> Result<Vec<NotebookAuditResult<VerifyError>>, DispatchError>;
	}
);

sp_api::decl_runtime_apis! {
	pub trait TickApis {
		fn current_tick() -> Tick;
		fn ticker() -> Ticker;
		fn block_at_tick(tick: Tick) -> Option<Block::Hash>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait NotaryApis<NotaryRecord: Codec> {
		fn notary_by_id(notary_id: NotaryId) -> Option<NotaryRecord>;
		fn notaries() -> Vec<NotaryRecord>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait MiningApis<AccountId, BlockSealAuthorityId> where BlockSealAuthorityId: Codec, AccountId: Codec {
		fn get_authority_id(account_id: &AccountId) -> Option<MiningAuthority<BlockSealAuthorityId,AccountId>>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait NotebookApis<VerifyError: Codec> {
		fn audit_notebook_and_get_votes(
			version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			notebook_tick: Tick,
			header_hash: H256,
			vote_minimums: &BTreeMap<Block::Hash, VoteMinimum>,
			bytes: &Vec<u8>,
			raw_audit_dependency_summaries: Vec<NotaryNotebookAuditSummary>,
		) -> Result<NotaryNotebookRawVotes, VerifyError>;


		fn decode_signed_raw_notebook_header(raw_header: Vec<u8>) -> Result<NotaryNotebookDetails<Block::Hash>, DispatchError>;

		fn latest_notebook_by_notary() -> BTreeMap<NotaryId, (NotebookNumber, Tick)>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait BitcoinApis<Balance: Codec> {
		fn get_sync_status() -> Option<BitcoinSyncStatus>;
		fn active_utxos() -> Vec<(Option<UtxoRef>, UtxoValue)>;
		fn redemption_rate(satoshis: Satoshis) -> Option<Balance>;
		fn market_rate(satoshis: Satoshis) -> Option<Balance>;
		fn get_bitcoin_network() -> BitcoinNetwork;
	}
}
