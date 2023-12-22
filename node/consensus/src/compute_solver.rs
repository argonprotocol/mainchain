use codec::Encode;
use sp_core::{blake2_256, U256};

use ulx_primitives::*;

use crate::compute_worker::Version;

#[derive(Clone, Eq, PartialEq, Encode)]
pub struct BlockComputeNonce {
	pub pre_hash: Vec<u8>,
	pub nonce: U256,
}

impl BlockComputeNonce {
	pub fn increment(&mut self) {
		self.nonce = self.nonce.checked_add(U256::one()).unwrap_or_default();
	}

	pub fn meets_threshold(hash: [u8; 32], threshold: U256) -> bool {
		U256::from_big_endian(&hash) <= threshold
	}

	pub fn threshold(difficulty: ComputeDifficulty) -> U256 {
		U256::MAX / U256::from(difficulty).max(U256::one())
	}

	pub fn is_valid(nonce: &U256, pre_hash: Vec<u8>, difficulty: ComputeDifficulty) -> bool {
		let hash = Self { nonce: nonce.clone(), pre_hash }.using_encoded(blake2_256);
		let threshold = Self::threshold(difficulty);
		Self::meets_threshold(hash, threshold)
	}
}

#[derive(Clone)]
pub struct ComputeSolver {
	pub version: Version,
	pub wip_nonce: BlockComputeNonce,
	pub wip_nonce_hash: Vec<u8>,
	pub threshold: U256,
}

impl ComputeSolver {
	pub fn new(version: Version, pre_hash: Vec<u8>, difficulty: ComputeDifficulty) -> Self {
		let mut solver = ComputeSolver {
			version,
			threshold: BlockComputeNonce::threshold(difficulty),
			wip_nonce_hash: vec![],
			wip_nonce: BlockComputeNonce { nonce: U256::from(rand::random::<u64>()), pre_hash },
		};
		solver.wip_nonce_hash = solver.wip_nonce.encode().to_vec();
		solver
	}

	/// Synchronous step to look at the next nonce
	pub fn check_next(&mut self) -> Option<BlockComputeNonce> {
		self.wip_nonce.increment();

		let nonce_bytes = self.wip_nonce.nonce.encode();
		let payload = &mut self.wip_nonce_hash;
		payload.splice(payload.len() - nonce_bytes.len().., nonce_bytes);

		let hash = blake2_256(&payload);
		if BlockComputeNonce::meets_threshold(hash, self.threshold) {
			return Some(self.wip_nonce.clone())
		}
		None
	}
}

#[cfg(test)]
mod tests {
	use codec::Encode;
	use sp_core::U256;

	use crate::{
		compute_solver::{BlockComputeNonce, ComputeSolver},
		compute_worker::Version,
		tests::setup_logs,
	};

	#[test]
	fn nonce_verify_compute() {
		setup_logs();

		let mut bytes = [0u8; 32];
		bytes[31] = 1;

		assert_eq!(BlockComputeNonce::is_valid(&U256::from(1), bytes.to_vec(), 1), true);
		assert_eq!(BlockComputeNonce::is_valid(&U256::from(1), bytes.to_vec(), 10_000), false);
	}

	#[test]
	fn it_can_reuse_a_nonce_algorithm_multiple_times() {
		setup_logs();

		let mut bytes = [0u8; 32];
		bytes[31] = 2;
		let pre_hash = bytes.to_vec();
		let mut solver = ComputeSolver::new(Version(0), pre_hash.clone(), 1);

		for _ in 0..100 {
			let did_solve = solver.check_next().is_some();

			assert_eq!(solver.wip_nonce_hash, solver.wip_nonce.encode());
			assert_eq!(
				did_solve,
				BlockComputeNonce::is_valid(&solver.wip_nonce.nonce, pre_hash.clone(), 1)
			);
		}
	}
}
