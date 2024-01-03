use codec::{Codec, Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{RuntimeDebug, H256, U256};
use sp_core_hashing::blake2_256;
use sp_runtime::scale_info::TypeInfo;
use sp_std::vec::Vec;

use crate::{AccountId, BlockVotingPower, DataDomain, MerkleProof, NotaryId, NotebookNumber};

pub type VoteMinimum = u128;

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
)]
#[serde(rename_all = "camelCase")]
pub struct BlockVoteT<Hash: Codec = H256> {
	/// The creator of the seal
	pub account_id: AccountId,
	/// The block hash being voted on. Must be in last 2 ticks.
	pub block_hash: Hash,
	/// A unique index per account for this notebook
	#[codec(compact)]
	pub index: u32,
	/// The voting power of this vote, determined from the amount of tax
	#[codec(compact)]
	pub power: BlockVotingPower,
	/// The data domain used to create this vote
	pub data_domain: DataDomain,
	/// The data domain payment address used to create this vote
	pub data_domain_account: AccountId,
}

pub type BlockVote = BlockVoteT<H256>;
pub type VotingKey = H256;

impl<Hash: Codec + Clone> BlockVoteT<Hash> {
	pub fn hash(&self) -> H256 {
		self.using_encoded(blake2_256).into()
	}

	pub fn get_seal_strength(&self, notary_id: NotaryId, voting_key: H256) -> U256 {
		Self::calculate_seal_strength(self.power, self.encode(), notary_id, voting_key)
	}

	pub fn calculate_seal_strength(
		power: BlockVotingPower,
		vote_bytes: Vec<u8>,
		notary_id: NotaryId,
		voting_key: H256,
	) -> U256 {
		let hash = BlockVoteProofHashMessage { notary_id, vote_bytes, voting_key }
			.using_encoded(blake2_256);
		U256::from_big_endian(&hash[..])
			.checked_div(U256::from(power))
			.unwrap_or(U256::zero())
	}

	pub fn seal_signature_message<H: Codec>(block_hash: &H, seal_strength: U256) -> [u8; 32] {
		let message = &[&block_hash.encode()[..], &seal_strength.encode()[..]].concat();
		message.using_encoded(blake2_256)
	}
}

#[derive(Encode)]
struct BlockVoteProofHashMessage {
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub vote_bytes: Vec<u8>,
	pub voting_key: H256,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestBlockVoteSeal<AccountId: Codec, Authority: Codec> {
	/// The seal strength (a smallest u256)
	pub seal_strength: U256,
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub block_vote_bytes: Vec<u8>,
	#[codec(compact)]
	pub source_notebook_number: NotebookNumber,
	/// Proof the vote was included in the block vote root of a notary header
	pub source_notebook_proof: MerkleProof,
	pub closest_miner: (AccountId, Authority),
}
