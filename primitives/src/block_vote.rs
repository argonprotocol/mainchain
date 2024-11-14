#[cfg(feature = "std")]
use crate::serialize_unsafe_u128_as_string;

use alloc::vec::Vec;
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{H256, U256};
use sp_crypto_hashing::blake2_256;
use sp_debug_derive::RuntimeDebug;
use sp_runtime::{scale_info::TypeInfo, MultiSignature};

use crate::{tick::Tick, AccountId, BlockVotingPower, MerkleProof, NotaryId, NotebookNumber};

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
	/// A user chosen index per account for this notebook
	#[codec(compact)]
	pub index: u32,
	/// The voting power of this vote, determined from the amount of tax
	#[codec(compact)]
	#[cfg_attr(feature = "std", serde(with = "serialize_unsafe_u128_as_string"))]
	pub power: BlockVotingPower,
	/// A signature of the vote by the account_id
	pub signature: MultiSignature,
	/// The claimer of rewards
	pub block_rewards_account_id: AccountId,
	/// The tick of the block vote
	#[codec(compact)]
	pub tick: Tick,
}

#[derive(Encode)]
struct BlockVoteHashMessage<Hash: Codec> {
	prefix: &'static str,
	account_id: AccountId,
	block_hash: Hash,
	index: u32,
	power: BlockVotingPower,
	block_rewards_account_id: AccountId,
	tick: Tick,
}

/// This struct exists mostly because the mental model of the voting is so difficult to
/// conceptualize.
///
/// When a vote is created, the voter will lookup a grandparent block to vote on. This is a
/// grandparent in terms of occurring 2 ticks prior to the active one.
///
/// Current Tick = 10
/// GrandParent to Vote on = 0x233..044 from tick 8
///
/// This vote is created and submitted to a notebook in tick 10.
/// Notebook 3 - Tick 10
/// Votes in notebook 3 will have votes for blocks at tick 8.
///
/// Notebook 3 goes into a block at tick 11.
///
/// At tick 11, the block creation process looks at the votes that could be used to create a new
/// block. These are the votes from notebooks in tick 10, which will themselves have a tick of 9.
///
/// Therefor, eligible votes -> tick 9
/// Votes for blocks -> tick 7
pub struct VotingSchedule {
	pub context: VotingContext,
}

pub enum VotingContext {
	/// When creating votes, you operate on the current tick and votes are for blocks 2 back
	Voting {
		current_tick: Tick,
	},
	/// When creating a block, you operate on the current tick and include notebooks from the
	/// previous tick
	CreatingBlock {
		current_tick: Tick,
	},
	/// When in runtime, the notebooks are all 1 tick behind
	Runtime {
		current_tick: Tick,
	},
	/// When evaluating a runtime seal in the `block_seal` pallet, you are looking at notebooks
	/// from 2 ticks back
	RuntimeSeal {
		current_tick: Tick,
	},
	EvaluateRuntimeVotes {
		current_tick: Tick,
	},
}

impl VotingSchedule {
	pub fn on_notebook_tick_state(notebook_tick: Tick) -> Self {
		Self {
			context: VotingContext::CreatingBlock { current_tick: notebook_tick.saturating_add(1) },
		}
	}

	pub fn from_runtime_current_tick(current_tick: Tick) -> Self {
		Self { context: VotingContext::Runtime { current_tick } }
	}

	pub fn when_creating_votes(current_tick: Tick) -> Self {
		Self { context: VotingContext::Voting { current_tick } }
	}

	pub fn when_evaluating_runtime_seals(current_tick: Tick) -> Self {
		Self { context: VotingContext::RuntimeSeal { current_tick } }
	}

	pub fn when_evaluating_runtime_votes(current_tick: Tick) -> Self {
		Self { context: VotingContext::EvaluateRuntimeVotes { current_tick } }
	}

	pub fn new(context: VotingContext) -> Self {
		Self { context }
	}

	pub fn when_creating_block(current_tick: Tick) -> Self {
		Self { context: VotingContext::CreatingBlock { current_tick } }
	}

	/// Which tick will the notebook be included in
	pub fn block_tick(&self) -> Tick {
		match self.context {
			VotingContext::CreatingBlock { current_tick } => current_tick,
			VotingContext::Runtime { current_tick } => current_tick,
			VotingContext::RuntimeSeal { current_tick } => current_tick,
			VotingContext::Voting { current_tick } => current_tick.saturating_add(1),
			VotingContext::EvaluateRuntimeVotes { current_tick } => current_tick,
		}
	}

	/// The parent block of the block with the notebook included in it
	///
	/// -> Block at Tick 3 -> Parent Notebook (notebook tick 2) <-- Parent Tick
	/// -> Block at Tick 4 -> Notebook (notebook tick 3)
	pub fn parent_block_tick(&self) -> Tick {
		self.block_tick().saturating_sub(1)
	}

	pub fn notebook_tick(&self) -> Tick {
		match self.context {
			VotingContext::CreatingBlock { current_tick } => current_tick.saturating_sub(1),
			VotingContext::Runtime { current_tick } => current_tick.saturating_sub(1),
			VotingContext::RuntimeSeal { current_tick } => current_tick.saturating_sub(2),
			VotingContext::Voting { current_tick } => current_tick,
			VotingContext::EvaluateRuntimeVotes { current_tick } => current_tick.saturating_sub(1),
		}
	}

	/// there won't be a grandparent block to vote for until block 2, and those votes
	/// don't count until tick 3
	pub fn is_voting_started(&self) -> bool {
		self.notebook_tick() >= 3
	}

	/// When do we evaluate the votes relative to the current notebook
	pub fn eligible_votes_tick(&self) -> Tick {
		match self.context {
			VotingContext::CreatingBlock { .. } => self.notebook_tick().saturating_sub(1),
			VotingContext::Runtime { .. } => self.notebook_tick().saturating_sub(1),
			VotingContext::RuntimeSeal { .. } => self.notebook_tick(),
			VotingContext::Voting { .. } => self.notebook_tick().saturating_sub(1),
			VotingContext::EvaluateRuntimeVotes { .. } => self.notebook_tick(),
		}
	}

	/// Which blocks were voted on relative to the current notebook
	pub fn grandparent_votes_tick(&self) -> Tick {
		self.notebook_tick().saturating_sub(2)
	}
}

pub type BlockVote = BlockVoteT<H256>;
pub type VotingKey = H256;

impl<Hash: Codec + Clone> BlockVoteT<Hash> {
	pub fn hash(&self) -> H256 {
		const PREFIX: &str = "BlockVote";
		BlockVoteHashMessage {
			prefix: PREFIX,
			account_id: self.account_id.clone(),
			block_hash: self.block_hash.clone(),
			index: self.index,
			power: self.power,
			block_rewards_account_id: self.block_rewards_account_id.clone(),
			tick: self.tick,
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

	pub fn seal_signature_message<H: AsRef<[u8]>>(block_hash: H) -> [u8; 32] {
		const PREFIX: &[u8] = b"BlockVoteSeal";
		let message = &[PREFIX, block_hash.as_ref()].concat();
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
