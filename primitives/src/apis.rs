#![allow(clippy::ptr_arg)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::multiple_bound_locations)]

use crate::{
	BestBlockVoteSeal, BlockSealDigest, BlockVoteDigest, NotebookAuditResult, VoteMinimum,
	VotingKey,
	bitcoin::{BitcoinNetwork, BitcoinSyncStatus, Satoshis, UtxoRef, UtxoValue},
	block_seal::{BlockPayout, ComputePuzzle, MiningAuthority},
	notary::{
		NotaryId, NotaryNotebookAuditSummary, NotaryNotebookDetails, NotaryNotebookRawVotes,
		NotaryNotebookVoteDigestDetails,
	},
	prelude::*,
	tick::Ticker,
};
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use codec::Codec;
use polkadot_sdk::*;
use sp_core::{H256, U256};
sp_api::decl_runtime_apis! {
	#[api_version(2)]
	pub trait BlockSealApis<AccountId:Codec, BlockSealAuthorityId:Codec> {
		fn vote_minimum() -> VoteMinimum;
		fn compute_puzzle() -> ComputePuzzle<Block>;
		fn create_vote_digest(notebook_tick: Tick, included_notebooks: Vec<NotaryNotebookVoteDigestDetails>) -> BlockVoteDigest;
		#[api_version(2)]
		fn find_better_vote_block_seal(
			notebook_votes: Vec<NotaryNotebookRawVotes>,
			best_strength: U256,
			closest_miner_nonce_score: U256,
			with_signing_key: BlockSealAuthorityId,
			expected_notebook_tick: Tick,
		) -> Result<Option<BestBlockVoteSeal<AccountId, BlockSealAuthorityId>>, DispatchError>;
		fn has_eligible_votes() -> bool;
		fn is_bootstrap_mining() -> bool;
		fn is_valid_signature(block_hash: Block::Hash, seal: &BlockSealDigest, digest: &Digest) -> bool;
	}
}

use sp_runtime::{Digest, DispatchError};

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
sp_api::decl_runtime_apis!(
	pub trait BlockImportApis {
		fn has_new_bitcoin_tip() -> bool;
		fn has_new_price_index() -> bool;
	}
);

sp_api::decl_runtime_apis! {
	pub trait TickApis {
		fn current_tick() -> Tick;
		fn ticker() -> Ticker;
		fn blocks_at_tick(tick: Tick) -> Vec<Block::Hash>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait NotaryApis<NotaryRecord: Codec> {
		fn notary_by_id(notary_id: NotaryId) -> Option<NotaryRecord>;
		fn notaries() -> Vec<NotaryRecord>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait MiningApis<AccountId, BlockSealAuthorityId> where BlockSealAuthorityId: Codec + PartialEq, AccountId: Codec {
		fn get_authority_id(account_id: &AccountId) -> Option<MiningAuthority<BlockSealAuthorityId,AccountId>>;
		fn get_block_payouts() -> Vec<BlockPayout<AccountId, Balance>>;
	}
}

sp_api::decl_runtime_apis! {

	#[api_version(2)]
	pub trait NotebookApis<VerifyError: Codec> {
		#[api_version(2)]
		fn audit_notebook_and_get_votes_v2(
			version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			notebook_tick: Tick,
			header_hash: H256,
			bytes: &Vec<u8>,
			raw_audit_dependency_summaries: Vec<NotaryNotebookAuditSummary>,
		) -> Result<NotaryNotebookRawVotes, VerifyError>;

		fn decode_signed_raw_notebook_header(raw_header: Vec<u8>) -> Result<NotaryNotebookDetails<Block::Hash>, DispatchError>;

		fn latest_notebook_by_notary() -> BTreeMap<NotaryId, (NotebookNumber, Tick)>;
	}
}

sp_api::decl_runtime_apis! {
	#[api_version(2)]
	pub trait BitcoinApis<Balance: Codec> {
		fn get_sync_status() -> Option<BitcoinSyncStatus>;
		fn active_utxos() -> Vec<(Option<UtxoRef>, UtxoValue)>;
		#[api_version(2)]
		fn get_minimum_satoshis() -> Satoshis;
		fn redemption_rate(satoshis: Satoshis) -> Option<Balance>;
		fn market_rate(satoshis: Satoshis) -> Option<Balance>;
		fn get_bitcoin_network() -> BitcoinNetwork;
	}
}
