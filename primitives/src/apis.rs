#![allow(clippy::ptr_arg)]
#![allow(clippy::too_many_arguments)]

use codec::{Codec, Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{ConstU32, RuntimeDebug, H256, U256};
use sp_runtime::{BoundedVec, DispatchError};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
	bitcoin::{BitcoinSyncStatus, Satoshis, UtxoRef, UtxoValue},
	block_seal::MiningAuthority,
	notary::{NotaryId, NotaryNotebookVoteDetails, NotaryNotebookVoteDigestDetails},
	tick::{Tick, Ticker},
	AccountOrigin, BestBlockVoteSeal, BlockVoteDigest, BlockVotingPower, NotebookNumber,
	NotebookSecretHash, TransferToLocalchainId, VoteMinimum,
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
			header_hash: H256,
			vote_minimums: &BTreeMap<Block::Hash, VoteMinimum>,
			bytes: &Vec<u8>,
			audit_dependency_summaries: Vec<NotebookAuditSummary>,
		) -> Result<NotebookAuditResult, VerifyError>;


		fn decode_signed_raw_notebook_header(raw_header: Vec<u8>) -> Result<NotaryNotebookVoteDetails<Block::Hash>, DispatchError>;

		fn latest_notebook_by_notary() -> BTreeMap<NotaryId, (NotebookNumber, Tick)>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait BitcoinApis<Balance: Codec> {
		fn get_sync_status() -> Option<BitcoinSyncStatus>;
		fn active_utxos() -> Vec<(Option<UtxoRef>, UtxoValue)>;
		fn redemption_rate(satoshis: Satoshis) -> Option<Balance>;
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebug)]
pub struct NotebookAuditSummary {
	#[codec(compact)]
	pub notary_id: NotaryId,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	#[codec(compact)]
	pub tick: Tick,
	pub changed_accounts_root: H256,
	pub account_changelist: Vec<AccountOrigin>,
	pub used_transfers_to_localchain: Vec<TransferToLocalchainId>,
	pub secret_hash: NotebookSecretHash,
	pub block_votes_root: H256,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebug)]
pub struct NotebookAuditResult {
	#[codec(compact)]
	pub notary_id: NotaryId,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	#[codec(compact)]
	pub tick: Tick,
	pub raw_votes: Vec<(Vec<u8>, BlockVotingPower)>,
	pub changed_accounts_root: H256,
	pub account_changelist: Vec<AccountOrigin>,
	pub used_transfers_to_localchain: Vec<TransferToLocalchainId>,
	pub secret_hash: NotebookSecretHash,
	pub block_votes_root: H256,
}

impl From<NotebookAuditResult> for (NotebookAuditSummary, NotaryNotebookVotes) {
	fn from(val: NotebookAuditResult) -> Self {
		(
			NotebookAuditSummary {
				notary_id: val.notary_id,
				notebook_number: val.notebook_number,
				tick: val.tick,
				changed_accounts_root: val.changed_accounts_root,
				account_changelist: val.account_changelist,
				used_transfers_to_localchain: val.used_transfers_to_localchain,
				secret_hash: val.secret_hash,
				block_votes_root: val.block_votes_root,
			},
			NotaryNotebookVotes {
				notary_id: val.notary_id,
				notebook_number: val.notebook_number,
				raw_votes: val.raw_votes,
			},
		)
	}
}
