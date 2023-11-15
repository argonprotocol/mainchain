use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::{CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use sp_api::BlockT;
use sp_core::{ed25519, Get, RuntimeDebug, H256};
use sp_runtime::{traits::NumberFor, BoundedVec};
use sp_std::{collections::btree_map::BTreeMap, fmt::Debug, vec::Vec};

pub use ulx_notary_primitives::NotaryId;
use ulx_notary_primitives::{
	BestBlockNonceT, BlockVotingPower, NotebookNumber, NotebookSecretHash,
};

use crate::block_seal::Host;

pub type NotaryPublic = ed25519::Public;
pub type NotarySignature = ed25519::Signature;

pub trait NotaryProvider<B: BlockT> {
	fn verify_signature(
		notary_id: NotaryId,
		at_block_height: NumberFor<B>,
		message: &H256,
		signature: &NotarySignature,
	) -> bool;
	fn active_notaries() -> Vec<NotaryId>;
}

#[derive(
	CloneNoBound, PartialEqNoBound, Eq, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxHosts))]
pub struct NotaryMeta<MaxHosts: Get<u32>> {
	pub public: NotaryPublic,
	pub hosts: BoundedVec<Host, MaxHosts>,
}

#[derive(
	CloneNoBound, PartialEqNoBound, Eq, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxHosts))]
pub struct NotaryRecord<
	AccountId: Codec + MaxEncodedLen + Clone + PartialEq + Eq + Debug,
	BlockNumber: Codec + MaxEncodedLen + Clone + PartialEq + Eq + Debug,
	MaxHosts: Get<u32>,
> {
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub operator_account_id: AccountId,
	#[codec(compact)]
	pub activated_block: BlockNumber,

	#[codec(compact)]
	pub meta_updated_block: BlockNumber,
	pub meta: NotaryMeta<MaxHosts>,
}

pub type VotingKey = H256;
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
pub struct NotaryNotebookSubmissionState<Hash: Codec> {
	pub notaries: u16,
	pub notebook_key_details_by_notary: BTreeMap<NotaryId, NotaryNotebookKeyDetails>,
	pub block_votes: u32,
	pub block_voting_power: BlockVotingPower,
	pub latest_finalized_block_needed: u32,
	pub next_parent_voting_key: VotingKey,
	pub best_nonces: Vec<(VotingKey, NotaryId, BestBlockNonceT<Hash>)>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NotaryNotebookVoteDetails<Hash: Codec> {
	pub notary_id: NotaryId,
	pub version: u32,
	pub notebook_number: NotebookNumber,
	pub finalized_block_number: u32,
	pub header_hash: H256,
	pub key_details: NotaryNotebookKeyDetails,
	pub block_votes: u32,
	pub block_voting_power: BlockVotingPower,
	pub blocks_with_votes: Vec<Hash>,
	pub best_nonces: Vec<(VotingKey, NotaryId, BestBlockNonceT<Hash>)>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NotaryNotebookKeyDetails {
	pub notebook_number: NotebookNumber,
	pub block_votes_root: H256,
	pub block_number: u32,
	pub secret_hash: NotebookSecretHash,
}
