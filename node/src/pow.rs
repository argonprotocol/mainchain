use std::{marker::PhantomData, sync::Arc};

use sc_client_api::{backend::AuxStore, blockchain::HeaderBackend};
use sc_consensus_pow::{Error as PowError, Error, PowAlgorithm, PowAux};
use sp_api::ProvideRuntimeApi;
use sp_consensus_pow::Seal;
use sp_core::{blake2_256, U256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, Header as _},
	RuntimeString, SaturatedConversion,
};

use ulx_node_runtime::{digest_timestamp, Hash as UlxBlockHash, Hash};

use crate::harmonic_mean::HarmonicMean;

pub type Difficulty = U256;
type Threshold = U256;

const INITIAL_DIFFICULTY: u64 = 1_000_000;
const ADJUST_DIFFICULTY_DAMPING: u32 = 3;
const ADJUST_DIFFICULTY_CLAMPING: u32 = 2;
const ADJUST_DIFFICULTY_WINDOW_SIZE: u64 = 12;
const TARGET_BLOCK_TIME_MS: u64 = 60_000;
const TARGET_WINDOW_TIME_MS: u64 = ADJUST_DIFFICULTY_WINDOW_SIZE * TARGET_BLOCK_TIME_MS;

pub struct UlixeePowAlgorithm<B: BlockT<Hash = UlxBlockHash>, C> {
	client: Arc<C>,
	_block: PhantomData<B>,
}

impl<B: BlockT<Hash = UlxBlockHash>, C> UlixeePowAlgorithm<B, C> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _block: PhantomData }
	}
}

impl<B: BlockT<Hash = UlxBlockHash>, C> Clone for UlixeePowAlgorithm<B, C> {
	fn clone(&self) -> Self {
		Self::new(self.client.clone())
	}
}

// Implementing PowAlgorithm trait is a must
impl<B: BlockT<Hash = UlxBlockHash>, C> PowAlgorithm<B> for UlixeePowAlgorithm<B, C>
where
	C: HeaderBackend<B> + AuxStore + ProvideRuntimeApi<B>,
{
	type Difficulty = U256;

	// Get the next block's difficulty
	fn difficulty(&self, parent: B::Hash) -> Result<Self::Difficulty, PowError<B>> {
		let mut prev_header = self.header_and(parent)?;
		if (*prev_header.number()).saturated_into::<u64>() <= ADJUST_DIFFICULTY_WINDOW_SIZE {
			return Ok(Difficulty::from(INITIAL_DIFFICULTY))
		}
		let mut difficulty_mean = HarmonicMean::new();
		for _ in 0..ADJUST_DIFFICULTY_WINDOW_SIZE {
			let difficulty = self.block_difficulty(prev_header.hash())?;
			difficulty_mean.push(difficulty);
			prev_header = self.header_and(*prev_header.parent_hash())?;
		}
		let avg_difficulty = difficulty_mean.calculate();
		let time_observed = self.window_mining_time_ms(prev_header.hash(), parent)?;
		Ok(next_difficulty(avg_difficulty, time_observed))
	}

	// Verify that the difficulty is valid against given seal
	fn verify(
		&self,
		_parent: &BlockId<B>,
		pre_hash: &B::Hash,
		_pre_digest: Option<&[u8]>,
		seal: &Seal,
		difficulty: Self::Difficulty,
	) -> Result<bool, PowError<B>> {
		let mut verifier = NonceVerifier::new(pre_hash, difficulty);
		Ok(verifier.is_nonce_valid(&seal))
	}
}

impl<B: BlockT<Hash = UlxBlockHash>, C> UlixeePowAlgorithm<B, C>
where
	C: ProvideRuntimeApi<B>,
	C: AuxStore,
	C: HeaderBackend<B>,
{
	fn header_and(&self, block_hash: UlxBlockHash) -> Result<B::Header, Error<B>> {
		self.client
			.header(block_hash)
			.and_then(|num_opt| {
				num_opt.ok_or_else(|| {
					sp_blockchain::Error::UnknownBlock(format!(
						"Can't find a block for the hash {}",
						block_hash
					))
				})
			})
			.map_err(Error::Client)
	}

	fn block_difficulty(&self, block_hash: UlxBlockHash) -> Result<Difficulty, Error<B>> {
		PowAux::read(&*self.client, &block_hash).map(|difficulty| difficulty.difficulty)
	}

	/// Calculates time it took to mine the blocks in the window.
	///
	/// It accepts the last block in the window and the **parent** of the first block.
	/// This is necessary, because in order to obtain a block mining time one must calculate
	/// the difference between timestamps of the block and it's parent.
	fn window_mining_time_ms(
		&self,
		first_block_parent_hash: Hash,
		last_block_hash: Hash,
	) -> Result<u64, Error<B>> {
		let start = self.block_timestamp_ms(first_block_parent_hash)?;
		let end = self.block_timestamp_ms(last_block_hash)?;
		Ok(end - start)
	}

	fn block_timestamp_ms(&self, block_hash: Hash) -> Result<u64, Error<B>> {
		let header = self.header_and(block_hash)?;
		digest_timestamp::load(&header.digest())
			.ok_or_else(|| Error::Runtime(RuntimeString::from("Timestamp not set in digest")))?
			.map_err(Error::Codec)
	}
}

pub struct NonceVerifier {
	payload: Vec<u8>,
	threshold: Threshold,
}

impl NonceVerifier {
	pub fn new(pre_hash: &Hash, difficulty: Difficulty) -> Self {
		NonceVerifier {
			payload: pre_hash.as_bytes().to_vec(),
			threshold: Threshold::max_value() / difficulty,
		}
	}

	pub fn is_nonce_valid(&mut self, nonce: &[u8]) -> bool {
		let original_payload_len = self.payload.len();
		self.payload.extend_from_slice(nonce);
		let hash = blake2_256(&self.payload);
		self.payload.truncate(original_payload_len);

		let hash_value = Threshold::from_big_endian(hash.as_slice());
		hash_value <= self.threshold
	}
}

/// Calculates the difficulty for the next block based on the window of the previous blocks
///
/// `avg` - the average difficulty of the blocks in the window
/// `time_observed` - the total time it took to create the blocks in the window
fn next_difficulty(avg: Difficulty, time_observed: u64) -> Difficulty {
	// This won't overflow, because difficulty is capped at using only its low 192 bits
	let new_raw = avg * TARGET_WINDOW_TIME_MS / time_observed.max(1);
	if new_raw > avg {
		let delta = new_raw - avg;
		let damped_delta = delta / ADJUST_DIFFICULTY_DAMPING;
		let new_damped = avg + damped_delta;
		let new_max = avg * ADJUST_DIFFICULTY_CLAMPING;
		new_damped.min(new_max).min(max_difficulty())
	} else {
		let delta = avg - new_raw;
		let damped_delta = delta / ADJUST_DIFFICULTY_DAMPING;
		let new_damped = avg - damped_delta;
		// Clamping matters only when ADJUST_DIFFICULTY_CLAMPING > ADJUST_DIFFICULTY_DAMPING
		let new_min = avg / ADJUST_DIFFICULTY_CLAMPING;
		new_damped.max(new_min)
	}
}

// This should be a constant when it becomes possible
fn max_difficulty() -> U256 {
	U256::MAX >> 64
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn next_difficulty_tests() {
		assert_next_difficulty(200, 0);
		assert_next_difficulty(200, 1);
		assert_next_difficulty(200, 25);
		assert_next_difficulty(194, 26);
		assert_next_difficulty(133, 50);
		assert_next_difficulty(100, 100);
		assert_next_difficulty(84, 200);
		assert_next_difficulty(68, 5000);
		assert_next_difficulty(67, 5001);
		assert_next_difficulty(67, 10000);
	}

	// assume that the average window difficulty is 100 and the target window time is 100
	fn assert_next_difficulty(expected: u64, time_observed: u64) {
		let adjusted_time_observed = TARGET_WINDOW_TIME_MS * time_observed / 100;
		let actual = next_difficulty(U256::from(100), adjusted_time_observed);
		assert_eq!(U256::from(expected), actual, "Failed for time_observed {}", time_observed);
	}
}
