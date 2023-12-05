use codec::{Codec, Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{RuntimeDebug, H256, U256};
use sp_core_hashing::blake2_256;
use sp_runtime::scale_info::TypeInfo;

use crate::{AccountId, ChannelPass, MerkleProof, NotaryId};

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
	pub power: u128,
	/// Proof of the tax channel
	pub channel_pass: ChannelPass,
}

pub type BlockVote = BlockVoteT<H256>;

impl<Hash: Codec + Clone> BlockVoteT<Hash> {
	pub fn hash(&self) -> H256 {
		self.using_encoded(blake2_256).into()
	}

	pub fn calculate_block_nonce(&self, notary_id: NotaryId, voting_key: H256) -> U256 {
		let hash = BlockVoteNonceHashMessage {
			notary_id,
			account_id: self.account_id.clone(),
			index: self.index,
			voting_key,
		}
		.using_encoded(blake2_256);
		U256::from_big_endian(&hash[..])
			.checked_div(U256::from(self.power))
			.unwrap_or(U256::zero())
	}
}

#[derive(Encode)]
struct BlockVoteNonceHashMessage {
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub account_id: AccountId,
	#[codec(compact)]
	pub index: u32,
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
pub struct BestBlockNonceT<Hash: Codec = H256> {
	pub nonce: U256,
	pub block_vote: BlockVoteT<Hash>,
	pub proof: MerkleProof,
}

pub type BestBlockNonce = BestBlockNonceT<H256>;
