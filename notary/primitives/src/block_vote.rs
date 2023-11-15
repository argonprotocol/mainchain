use codec::{Codec, Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{RuntimeDebug, H256, U256};
use sp_core_hashing::blake2_256;
use sp_runtime::scale_info::TypeInfo;

use crate::{AccountId, ChannelPass, MerkleProof, NotaryId};

#[derive(
	Clone,
	Copy,
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
pub enum BlockVoteSource {
	Tax,
	Compute,
}

impl Default for BlockVoteSource {
	fn default() -> Self {
		BlockVoteSource::Compute
	}
}

impl From<i32> for BlockVoteSource {
	fn from(i: i32) -> Self {
		match i {
			0 => BlockVoteSource::Tax,
			1 => BlockVoteSource::Compute,
			_ => panic!("Invalid BlockVoteSource"),
		}
	}
}

pub type VotingMinimum = u128;
#[derive(
	Clone,
	Copy,
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
pub struct BlockVoteEligibility {
	/// The minimum tax milligons or compute power required to seal the next block
	#[codec(compact)]
	pub minimum: VotingMinimum,

	/// The type of votes the next block is accepting
	pub allowed_sources: BlockVoteSource,
}

impl BlockVoteEligibility {
	pub fn new(minimum: u128, allowed_sources: BlockVoteSource) -> Self {
		Self { minimum, allowed_sources }
	}
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
pub struct BlockVoteT<Hash: Codec = H256> {
	/// The creator of the seal
	pub account_id: AccountId,
	/// The parent block hash you wish to close
	pub block_hash: Hash,
	/// A unique index per account for this notebook
	#[codec(compact)]
	pub index: u32,
	/// The voting power of this vote, determined from the amount of tax or hashing
	#[codec(compact)]
	pub power: u128,
	/// The source of voting power. Tax or compute
	pub vote_source: VoteSource,
}

impl<H: Codec> BlockVoteT<H> {
	pub fn is_tax(&self) -> bool {
		match self.vote_source {
			VoteSource::Tax { .. } => true,
			_ => false,
		}
	}
	pub fn is_compute(&self) -> bool {
		match self.vote_source {
			VoteSource::Compute { .. } => true,
			_ => false,
		}
	}
}

pub type BlockVote = BlockVoteT<H256>;

#[derive(Clone, Eq, PartialEq, Encode)]
pub struct ComputePuzzle<Hash: Codec> {
	pub account_id: AccountId,
	pub block_hash: Hash,
	pub index: u32,
	pub puzzle_proof: U256,
}

impl<Hash: Codec + Clone> Into<BlockVoteT<Hash>> for ComputePuzzle<Hash> {
	fn into(self) -> BlockVoteT<Hash> {
		let mut vote = BlockVoteT {
			account_id: self.account_id,
			block_hash: self.block_hash,
			index: self.index,
			power: 0,
			vote_source: VoteSource::Compute { puzzle_proof: self.puzzle_proof },
		};
		vote.power = BlockVote::calculate_compute_power(&self.puzzle_proof);
		vote
	}
}

impl<Hash: Codec + Clone> BlockVoteT<Hash> {
	pub fn hash(&self) -> H256 {
		self.using_encoded(blake2_256).into()
	}

	pub fn calculate_puzzle_nonce(&self) -> U256 {
		let VoteSource::Compute { puzzle_proof } = self.vote_source else { return U256::zero() };
		let hash = ComputePuzzle {
			account_id: self.account_id.clone(),
			block_hash: self.block_hash.clone(),
			index: self.index,
			puzzle_proof,
		}
		.using_encoded(blake2_256);
		U256::from_big_endian(&hash[..])
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

	pub fn calculate_compute_power(puzzle_nonce: &U256) -> u128 {
		2u128.saturating_pow(puzzle_nonce.leading_zeros())
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
pub enum VoteSource {
	/// The block vote was formed by tax. Authority is from a channel
	Tax { channel_pass: ChannelPass },
	/// Compute votes include a puzzle that must be solved to prove cpu was expended
	Compute { puzzle_proof: U256 },
}
