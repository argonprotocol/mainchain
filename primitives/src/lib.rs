#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode, FullCodec, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_api::BlockT;
use sp_core::{crypto::AccountId32, RuntimeDebug, H256};
use sp_runtime::DispatchResult;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

pub use block_seal::{BlockSealAuthorityId, BlockSealAuthoritySignature, BLOCK_SEAL_KEY_TYPE};
pub use digests::{BlockSealDigest, AUTHOR_DIGEST_ID, BLOCK_SEAL_DIGEST_ID};
pub use ulx_notary_primitives::{MerkleProof, NotaryId};
use ulx_notary_primitives::{NotebookHeader, NotebookNumber, VoteMinimum};

use crate::{
	block_seal::{AuthorityIndex, Host, MiningAuthority},
	notary::NotaryNotebookVoteDetails,
};

pub mod block_seal;
pub mod bond;
pub mod digests;
pub mod inherents;
pub mod notary;

pub mod notebook {
	pub use ulx_notary_primitives::{
		AccountOrigin, AccountOriginUid, BalanceTip, BestBlockNonce, BlockVotingKey, ChainTransfer,
		MaxNotebookNotarizations, NewAccountOrigin, Notarization, Notebook, NotebookHeader,
		NotebookNumber, NotebookSecretHash,
	};
}

pub type ComputeDifficulty = u128;

pub mod localchain {
	pub use ulx_notary_primitives::{
		AccountType, BalanceChange, BlockVote, BlockVoteT, ChannelPass, Note, NoteType, VoteMinimum,
	};
}

pub trait NotebookProvider {
	/// Returns a block voting root only if submitted in time for previous block
	fn get_eligible_block_votes_root(
		notary_id: NotaryId,
		block_number: u32,
	) -> Option<(H256, NotebookNumber)>;
}

pub trait ChainTransferLookup<Nonce, AccountId> {
	fn is_valid_transfer_to_localchain(
		notary_id: NotaryId,
		account_id: &AccountId,
		nonce: Nonce,
	) -> bool;
}

pub trait BlockVotingProvider<Block: BlockT> {
	fn grandparent_vote_minimum() -> Option<VoteMinimum>;
	fn parent_voting_key() -> Option<H256>;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
pub struct BlockSealerInfo<AccountId: FullCodec> {
	pub miner_rewards_account: AccountId,
	pub block_vote_rewards_account: AccountId,
	pub notaries_included: u32,
}

pub trait BlockSealerProvider<AccountId: FullCodec> {
	fn get_sealer_info() -> BlockSealerInfo<AccountId>;
}

pub trait AuthorityProvider<AuthorityId, Block, AccountId>
where
	Block: BlockT,
{
	fn miner_zero() -> Option<(AuthorityIndex, AuthorityId, Vec<Host>, AccountId)>;
	fn authorities() -> Vec<AuthorityId>;
	fn authority_id_by_index() -> BTreeMap<AuthorityIndex, AuthorityId>;
	fn authority_count() -> u16;
	fn is_active(authority_id: &AuthorityId) -> bool;
	fn get_authority(author: AccountId) -> Option<AuthorityId>;
	fn get_rewards_account(author: AccountId) -> Option<AccountId>;
	fn block_peer(
		block_hash: &Block::Hash,
		account_id: &AccountId32,
	) -> Option<MiningAuthority<AuthorityId>>;
}

/// An event handler to listen for submitted notebook
pub trait NotebookEventHandler {
	fn notebook_submitted(header: &NotebookHeader) -> DispatchResult;
}

#[impl_trait_for_tuples::impl_for_tuples(5)]
impl NotebookEventHandler for Tuple {
	fn notebook_submitted(header: &NotebookHeader) -> DispatchResult {
		for_tuples!( #( Tuple::notebook_submitted(&header); )* );
		Ok(())
	}
}

sp_api::decl_runtime_apis! {
	pub trait MiningAuthorityApis {
		fn authority_id_by_index() -> BTreeMap<AuthorityIndex, BlockSealAuthorityId>;
		fn active_authorities() -> AuthorityIndex;
		fn is_valid_authority(authority_id: BlockSealAuthorityId) -> bool;
		fn authority_id_for_account(account_id: AccountId32) -> Option<BlockSealAuthorityId>;
		fn block_peer(account_id: AccountId32) -> Option<MiningAuthority<BlockSealAuthorityId>>;
	}
}

sp_api::decl_runtime_apis! {
	pub trait BlockSealMinimumApis {
		fn vote_minimum() -> VoteMinimum;
		fn compute_difficulty() -> u128;
		fn parent_voting_key() -> Option<H256>;
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
		fn audit_notebook(
			version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			header_hash: H256,
			vote_minimums: &BTreeMap<Block::Hash, VoteMinimum>,
			bytes: &Vec<u8>,
		) -> Result<bool, ulx_notary_audit::VerifyError>;

		fn decode_notebook_vote_details(extrinsic: &<Block as BlockT>::Extrinsic) -> Option<NotaryNotebookVoteDetails<Block::Hash>>;
	}
}

#[derive(
	Encode,
	Decode,
	Copy,
	Clone,
	Eq,
	PartialEq,
	TypeInfo,
	MaxEncodedLen,
	Ord,
	PartialOrd,
	RuntimeDebug,
)]
pub enum BalanceFreezeId {
	MaturationPeriod,
}
