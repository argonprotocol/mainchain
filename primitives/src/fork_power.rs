use crate::{BlockSealDigest, BlockVotingPower, ComputeDifficulty};
use codec::{Compact, Decode, Encode, MaxEncodedLen};
use core::cmp::Ordering;
use polkadot_sdk::sp_core::U256;
use scale_info::TypeInfo;

#[derive(Clone, Debug, Eq, PartialEq, Encode, MaxEncodedLen, TypeInfo)]
pub struct ForkPower {
	/// True if the fork is a vote block, false if it is a compute block.
	pub is_latest_vote: bool,
	#[codec(compact)]
	pub notebooks: u64,
	pub voting_power: U256,
	pub seal_strength: U256,
	pub total_compute_difficulty: U256,
	#[codec(compact)]
	pub vote_created_blocks: u128,
	/// The XOR distance of the miner to the vote
	pub miner_nonce_score: Option<U256>,
}

/// Custom Decode for backwards compatibility: older ForkPower encodings omitted the
/// `miner_nonce_score` Option field entirely when it was `None`.
impl Decode for ForkPower {
	fn decode<I: codec::Input + Sized>(input: &mut I) -> Result<Self, codec::Error> {
		let is_latest_vote = bool::decode(input)?;
		let notebooks = Compact::<u64>::decode(input)?.0;
		let voting_power = U256::decode(input)?;
		let seal_strength = U256::decode(input)?;
		let total_compute_difficulty = U256::decode(input)?;
		let vote_created_blocks = Compact::<u128>::decode(input)?.0;
		// Allow for backwards compatibility with older versions of the fork power struct that did
		// not include the miner_nonce_score field.
		let miner_nonce_score = match input.remaining_len()? {
			Some(0) | None => None,
			Some(_) => Option::<U256>::decode(input)?,
		};

		Ok(Self {
			is_latest_vote,
			notebooks,
			voting_power,
			seal_strength,
			total_compute_difficulty,
			vote_created_blocks,
			miner_nonce_score,
		})
	}
}

impl ForkPower {
	pub fn add(
		&mut self,
		block_voting_power: BlockVotingPower,
		notebooks: u32,
		seal_digest: BlockSealDigest,
		compute_difficulty: ComputeDifficulty,
	) {
		match seal_digest {
			BlockSealDigest::Vote { seal_strength, .. } => {
				self.add_vote(block_voting_power, notebooks, seal_strength, None);
			},
			BlockSealDigest::Compute { .. } => {
				self.add_compute(block_voting_power, notebooks, compute_difficulty);
			},
		}
	}

	pub fn add_vote(
		&mut self,
		block_voting_power: BlockVotingPower,
		notebooks: u32,
		seal_strength: U256,
		miner_nonce_score: Option<U256>,
	) {
		self.voting_power = self.voting_power.saturating_add(U256::from(block_voting_power));
		self.notebooks = self.notebooks.saturating_add(notebooks as u64);
		self.seal_strength = seal_strength;
		self.vote_created_blocks = self.vote_created_blocks.saturating_add(1);
		self.is_latest_vote = true;
		self.miner_nonce_score = miner_nonce_score;
	}

	pub fn add_compute(
		&mut self,
		block_voting_power: BlockVotingPower,
		notebooks: u32,
		compute_difficulty: ComputeDifficulty,
	) {
		self.voting_power = self.voting_power.saturating_add(U256::from(block_voting_power));
		self.notebooks = self.notebooks.saturating_add(notebooks as u64);
		self.total_compute_difficulty =
			self.total_compute_difficulty.saturating_add(compute_difficulty.into());
		self.is_latest_vote = false;
		self.seal_strength = U256::MAX; // Compute blocks have no seal strength
		self.miner_nonce_score = None;
	}

	pub fn eq_weight(&self, other: &Self) -> bool {
		self.partial_cmp(other) == Some(Ordering::Equal)
	}
}

impl Default for ForkPower {
	fn default() -> Self {
		Self {
			is_latest_vote: false,
			voting_power: U256::zero(),
			notebooks: 0,
			seal_strength: U256::MAX,
			total_compute_difficulty: U256::zero(),
			vote_created_blocks: 0,
			miner_nonce_score: None,
		}
	}
}

impl PartialOrd for ForkPower {
	/// This comparator is quite sensitive. It compares the entire fork power of each block, which
	/// means we need to account for scenarios where one block is a vote and the other is a compute
	/// block.
	///
	/// Things that must be considered:
	/// - A compute block will not have a vote, so seal strength and xor distance will not be
	///   compared.
	/// - Compute forks are measured by total compute difficulty
	/// - We prefer vote over compute, so always take a fork with more vote blocks
	/// - Requiring compute to have notebooks means it cannot be used as a fallback mechanism, so we
	///   will ignore that. It also skews compute towards the moments after a notebook arrives.
	/// - NOTE: notebook count is intentionally excluded from ordering to avoid private/withheld
	///   notebook release influencing fork choice.
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		// count forks with tax votes over compute
		let mut cmp = self.vote_created_blocks.cmp(&other.vote_created_blocks);
		if cmp == Ordering::Equal {
			// total spend on vote tax
			cmp = self.voting_power.cmp(&other.voting_power);
		}

		// we should only compare these when both are vote blocks since a compute block would lose
		// this comparison with seal strength = u256::MAX
		if self.is_latest_vote && other.is_latest_vote {
			if cmp == Ordering::Equal {
				// smaller vote proof is better
				cmp = other.seal_strength.cmp(&self.seal_strength)
			}
			if cmp == Ordering::Equal {
				let self_miner_nonce_score = self.miner_nonce_score.as_ref().unwrap_or(&U256::MAX);
				let other_miner_nonce_score =
					other.miner_nonce_score.as_ref().unwrap_or(&U256::MAX);
				// smaller xor distance is better
				cmp = other_miner_nonce_score.cmp(self_miner_nonce_score);
			}
		}

		if cmp == Ordering::Equal {
			cmp = self.total_compute_difficulty.cmp(&other.total_compute_difficulty)
		}
		Some(cmp)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use polkadot_sdk::frame_support::assert_ok;

	#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, MaxEncodedLen, TypeInfo)]
	pub struct ForkPowerV0 {
		/// True if the fork is a vote block, false if it is a compute block.
		pub is_latest_vote: bool,
		#[codec(compact)]
		pub notebooks: u64,
		pub voting_power: U256,
		pub seal_strength: U256,
		pub total_compute_difficulty: U256,
		#[codec(compact)]
		pub vote_created_blocks: u128,
	}
	#[test]
	fn it_should_compare_fork_power() {
		assert_eq!(ForkPower::default(), ForkPower::default());

		let mut fork_a = ForkPower::default();
		let mut fork_b = ForkPower::default();

		fork_a.voting_power = U256::one();
		fork_b.voting_power = U256::zero();

		assert!(fork_a > fork_b, "more voting power should be better");
		fork_a.voting_power = fork_b.voting_power;

		fork_a.add_vote(0, 2, U256::one(), None);
		fork_b.add_vote(0, 1, U256::one(), None);
		assert!(fork_a.eq_weight(&fork_b), "notebook count should not affect vote fork comparison");

		fork_a.add_compute(0, 0, 1000);
		fork_b.add_compute(0, 2, 1000);
		assert!(
			fork_a.eq_weight(&fork_b),
			"Compute blocks should be equal if they have the same compute difficulty - notebooks don't matter"
		);
		fork_a.vote_created_blocks = fork_b.vote_created_blocks;

		fork_a.add_vote(0, 0, U256::zero(), None);
		assert!(fork_a > fork_b, "more vote blocks should be better");

		fork_b.add_vote(0, 0, U256::one(), None);
		assert!(fork_a > fork_b, "smaller voting strength should be better");

		fork_a.add_vote(0, 0, U256::one(), Some(U256::from(10)));
		fork_b.add_vote(0, 0, U256::one(), Some(U256::from(20)));
		assert!(fork_a > fork_b, "smaller xor distance should be better");
	}

	#[test]
	fn it_can_decode_old_fork_power() {
		let hex = "01f25e0e00000000000000000000000000000000000000000000000000000000000000000028f6beac0bd41db02f6e1e3e74d0b81ee231ebc0fa8c03f2c101a14b6f55223956003c8c00000000000000000000000000000000000000000000000000000000b2c10300";
		let bytes = hex::decode(hex).unwrap();
		let mut fork_power = ForkPower::decode(&mut &bytes[..]).unwrap();
		assert!(fork_power.is_latest_vote);
		assert_eq!(fork_power.notebooks, 235452);
		assert_eq!(fork_power.miner_nonce_score, None);

		assert_ok!(ForkPowerV0::decode(&mut &bytes[..]));

		fork_power.miner_nonce_score = Some(U256::zero());
		let new_encoded = fork_power.encode();

		let fork_power_v0 = ForkPowerV0::decode(&mut &new_encoded[..]).unwrap();
		assert_eq!(fork_power_v0.is_latest_vote, fork_power.is_latest_vote);
	}
}
