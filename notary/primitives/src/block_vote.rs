use codec::{Codec, Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{RuntimeDebug, H256, U256};
use sp_core_hashing::blake2_256;
use sp_runtime::scale_info::TypeInfo;
use sp_std::vec::Vec;

use crate::{AccountId, BlockVotingPower, ChannelPass, MerkleProof, NotaryId};

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
	/// The grandparent block hash you wish to vote for
	pub grandparent_block_hash: Hash,
	/// A unique index per account for this notebook
	#[codec(compact)]
	pub index: u32,
	/// The voting power of this vote, determined from the amount of tax
	#[codec(compact)]
	pub power: BlockVotingPower,
	/// Proof of the tax channel
	pub channel_pass: ChannelPass,
}

pub type BlockVote = BlockVoteT<H256>;

impl<Hash: Codec + Clone> BlockVoteT<Hash> {
	pub fn hash(&self) -> H256 {
		self.using_encoded(blake2_256).into()
	}

	pub fn vote_proof(&self, notary_id: NotaryId, voting_key: H256) -> U256 {
		Self::calculate_vote_proof(self.power, self.encode(), notary_id, voting_key)
	}

	pub fn calculate_vote_proof(
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

	pub fn vote_proof_signature_message(vote_proof: U256) -> [u8; 32] {
		vote_proof.using_encoded(blake2_256)
	}
}

#[derive(Encode)]
struct BlockVoteProofHashMessage {
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub vote_bytes: Vec<u8>,
	pub voting_key: H256,
}

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
pub struct BestBlockVoteProofT<Hash: Codec = H256> {
	/// The vote proof (a smallest u256)
	pub vote_proof: U256,
	pub notary_id: NotaryId,
	pub block_vote: BlockVoteT<Hash>,
	/// Proof the vote was included in the block vote root of a notary header
	pub source_notebook_proof: MerkleProof,
}

pub type BestBlockNonce = BestBlockVoteProofT<H256>;
