#[cfg(feature = "std")]
use crate::serialize_unsafe_u128_as_string;

use codec::{Codec, Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{H256, U256};
use sp_crypto_hashing::blake2_256;
use sp_debug_derive::RuntimeDebug;
use sp_runtime::{scale_info::TypeInfo, MultiSignature};
use sp_std::vec::Vec;

use crate::{AccountId, BlockVotingPower, DataDomainHash, MerkleProof, NotaryId, NotebookNumber};

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
	/// The creator of the block vote
	pub account_id: AccountId,
	/// The block hash being voted on. Must be in last 2 ticks.
	pub block_hash: Hash,
	/// A unique index per account for this notebook
	#[codec(compact)]
	pub index: u32,
	/// The voting power of this vote, determined from the amount of tax
	#[codec(compact)]
	#[cfg_attr(feature = "std", serde(with = "serialize_unsafe_u128_as_string"))]
	pub power: BlockVotingPower,
	/// The data domain used to create this vote
	pub data_domain_hash: DataDomainHash,
	/// The data domain payment address used to create this vote
	pub data_domain_account: AccountId,
	/// A signature of the vote by the account_id
	pub signature: MultiSignature,
	/// The claimer of rewards
	pub block_rewards_account_id: AccountId,
}

#[derive(Encode)]
struct BlockVoteHashMessage<Hash: Codec> {
	account_id: AccountId,
	block_hash: Hash,
	index: u32,
	power: BlockVotingPower,
	data_domain_hash: DataDomainHash,
	data_domain_account: AccountId,
	block_rewards_account_id: AccountId,
}

pub type BlockVote = BlockVoteT<H256>;
pub type VotingKey = H256;

impl<Hash: Codec + Clone> BlockVoteT<Hash> {
	pub fn hash(&self) -> H256 {
		BlockVoteHashMessage {
			account_id: self.account_id.clone(),
			block_hash: self.block_hash.clone(),
			index: self.index,
			power: self.power,
			data_domain_hash: self.data_domain_hash,
			data_domain_account: self.data_domain_account.clone(),
			block_rewards_account_id: self.block_rewards_account_id.clone(),
		}
		.using_encoded(blake2_256)
		.into()
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

	#[cfg(feature = "std")]
	pub fn sign<S: sp_core::Pair>(&mut self, pair: S) -> &Self
	where
		S::Signature: Into<MultiSignature>,
	{
		self.signature = pair.sign(&self.hash()[..]).into();
		self
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
