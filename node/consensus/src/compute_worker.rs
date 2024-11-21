use crate::block_creator::BlockProposal;
use argon_primitives::{tick::Tick, BlockSealDigest, ComputeDifficulty};
use argon_randomx::{calculate_hash, calculate_mining_hash, RandomXError};
use codec::Encode;
use frame_support::CloneNoBound;
use futures::prelude::*;
use log::*;
use parking_lot::Mutex;
use rand::Rng;
use sc_service::TaskManager;
use sc_utils::mpsc::TracingUnboundedSender;
use sp_core::{traits::SpawnEssentialNamed, H256, U256};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use std::{
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	time::Duration,
};

pub(crate) type Version = usize;

/// Mining metadata. This is the information needed to start an actual mining loop.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct MiningMetadata<H> {
	/// Currently known best hash which the pre-hash is built on.
	pub best_hash: H,
	/// The time at which mining is activated (in milliseconds).
	pub activate_mining_time: u64,
	pub has_tax_votes: bool,
	/// At which tick do we kick in mining no matter what?
	pub emergency_tick: Tick,
	/// The randomx key block hash.
	pub key_block_hash: H256,
	pub difficulty: ComputeDifficulty,
}

pub(crate) struct SolvingBlock<B: BlockT, Proof> {
	pub proposal: BlockProposal<B, Proof>,
	/// Mining pre-hash.
	pub pre_hash: B::Hash,
	/// Pre-runtime digest item.
	pub difficulty: ComputeDifficulty,
}

/// Mining worker that exposes structs to query the current mining build and submit mined blocks.
#[derive(CloneNoBound)]
pub(crate) struct ComputeHandle<B: BlockT, Proof> {
	version: Arc<AtomicUsize>,
	metadata: Arc<Mutex<Option<MiningMetadata<B::Hash>>>>,
	solving_block: Arc<Mutex<Option<SolvingBlock<B, Proof>>>>,
	block_found_tx: TracingUnboundedSender<(SolvingBlock<B, Proof>, BlockSealDigest)>,
}

impl<B, Proof> ComputeHandle<B, Proof>
where
	B: BlockT,
{
	pub(crate) fn new(
		block_found_tx: TracingUnboundedSender<(SolvingBlock<B, Proof>, BlockSealDigest)>,
	) -> Self {
		Self {
			version: Arc::new(AtomicUsize::new(0)),
			solving_block: Arc::new(Mutex::new(None)),
			block_found_tx,
			metadata: Arc::new(Mutex::new(None)),
		}
	}

	pub(crate) fn stop_solving_current(&self) {
		*self.solving_block.lock() = None;
		*self.metadata.lock() = None;
		self.increment_version();
	}

	pub(crate) fn new_best_block(&self, metadata: MiningMetadata<B::Hash>) {
		self.stop_solving_current();
		*self.metadata.lock() = Some(metadata);
	}

	pub(crate) fn start_solving(
		&self,
		best_hash: B::Hash,
		pre_hash: B::Hash,
		proposal: BlockProposal<B, Proof>,
	) {
		if self.best_hash() != Some(best_hash) {
			self.stop_solving_current();
			return;
		}
		let difficulty = self.metadata.lock().as_ref().map(|x| x.difficulty).unwrap_or_default();

		*self.solving_block.lock() = Some(SolvingBlock { pre_hash, difficulty, proposal });
	}

	fn increment_version(&self) {
		self.version.fetch_add(1, Ordering::SeqCst);
	}

	/// Get the version of the mining worker.
	///
	/// This returns type `Version` which can only compare equality. If `Version` is unchanged, then
	/// it can be certain that `best_hash` and `metadata` were not changed.
	pub fn version(&self) -> Version {
		self.version.load(Ordering::SeqCst)
	}

	/// Get the current best hash. `None` if the worker has just started or the client is doing
	/// major syncing.
	pub fn best_hash(&self) -> Option<B::Hash> {
		self.metadata.lock().as_ref().map(|b| b.best_hash)
	}

	pub fn proposal_parent_hash(&self) -> Option<B::Hash> {
		self.solving_block
			.lock()
			.as_ref()
			.map(|b| b.proposal.proposal.block.header().parent_hash())
			.cloned()
	}

	pub fn create_solver(&self) -> Option<ComputeSolver> {
		match self.solving_block.lock().as_ref() {
			Some(x) => {
				let key_block_hash = self.metadata.lock().as_ref()?.key_block_hash;

				Some(ComputeSolver::new(
					self.version(),
					x.pre_hash.as_ref().to_vec(),
					key_block_hash,
					x.difficulty,
				))
			},
			_ => None,
		}
	}

	pub fn is_valid_solver(&self, solver: &Option<Box<ComputeSolver>>) -> bool {
		solver.as_ref().map(|a| a.version) == Some(self.version())
	}

	pub fn ready_to_solve(&self, current_tick: Tick, now_millis: u64) -> bool {
		match self.metadata.lock().as_ref() {
			Some(x) => {
				// if we've passed the emergency tick, we should mine no matter what
				if current_tick > x.emergency_tick {
					return true;
				}
				// must be past the parent tick and not have tax votes
				if !x.has_tax_votes && x.activate_mining_time < now_millis {
					return true;
				}
				false
			},

			_ => false,
		}
	}

	pub fn submit(&self, nonce: U256) {
		let Some(build) = self.solving_block.lock().take() else {
			trace!("Unable to submit mined block in compute worker: internal build does not exist",);
			return;
		};

		self.increment_version();

		let _ = self
			.block_found_tx
			.unbounded_send((build, BlockSealDigest::Compute { nonce }))
			.inspect_err(|e| error!("Error sending block found message: {:?}", e));
	}
}

#[derive(Clone, Eq, PartialEq, Encode)]
pub struct BlockComputeNonce {
	pub pre_hash: Vec<u8>,
	pub nonce: U256,
}

impl BlockComputeNonce {
	pub fn increment(&mut self) {
		self.nonce = self.nonce.checked_add(U256::one()).unwrap_or_default();
	}

	pub fn meets_threshold(hash: &[u8; 32], threshold: U256) -> bool {
		U256::from_big_endian(hash) <= threshold
	}

	pub fn threshold(difficulty: ComputeDifficulty) -> U256 {
		U256::MAX / U256::from(difficulty).max(U256::one())
	}

	pub fn is_valid(
		nonce: &U256,
		pre_hash: Vec<u8>,
		key_block_hash: &H256,
		compute_difficulty: ComputeDifficulty,
	) -> bool {
		let hash =
			Self { nonce: *nonce, pre_hash }.using_encoded(|x| calculate_hash(key_block_hash, x));
		let Ok(hash) = hash else {
			return false;
		};
		let threshold = Self::threshold(compute_difficulty);
		Self::meets_threshold(hash.as_fixed_bytes(), threshold)
	}
}

/// This is a lightweight struct that lives in-thread for each mining thread.
#[derive(Clone)]
pub(crate) struct ComputeSolver {
	pub version: Version,
	pub wip_nonce: BlockComputeNonce,
	pub wip_nonce_hash: Vec<u8>,
	pub threshold: U256,
	pub key_block_hash: H256,
}

impl ComputeSolver {
	pub fn new(
		version: Version,
		pre_hash: Vec<u8>,
		key_block_hash: H256,
		compute_difficulty: ComputeDifficulty,
	) -> Self {
		let mut rng = rand::thread_rng();
		let mut bytes = [0u8; 32];
		rng.fill(&mut bytes);
		let mut solver = ComputeSolver {
			version,
			threshold: BlockComputeNonce::threshold(compute_difficulty),
			wip_nonce_hash: vec![],
			wip_nonce: BlockComputeNonce { nonce: U256::from_big_endian(&bytes[..]), pre_hash },
			key_block_hash,
		};
		solver.wip_nonce_hash = solver.wip_nonce.encode().to_vec();
		solver
	}

	/// Synchronous step to look at the next nonce
	pub fn check_next(&mut self) -> Result<Option<BlockComputeNonce>, RandomXError> {
		self.wip_nonce.increment();

		let nonce_bytes = self.wip_nonce.nonce.encode();
		let payload = &mut self.wip_nonce_hash;
		payload.splice(payload.len() - nonce_bytes.len().., nonce_bytes);

		let hash = calculate_mining_hash(&self.key_block_hash, payload)?;
		if BlockComputeNonce::meets_threshold(hash.as_fixed_bytes(), self.threshold) {
			return Ok(Some(self.wip_nonce.clone()));
		}
		Ok(None)
	}
}

pub fn run_compute_solver_threads<B, Proof>(
	task_handle: &TaskManager,
	worker: ComputeHandle<B, Proof>,
	threads: u32,
) where
	B: BlockT,
	Proof: Send + 'static,
{
	let handle = task_handle.spawn_essential_handle();
	for _ in 0..threads {
		let worker = worker.clone();
		let task = async move {
			let mut solver_ref = None;
			loop {
				if !worker.is_valid_solver(&solver_ref) {
					solver_ref = worker.create_solver().map(Box::new);
				}

				let Some(solver) = solver_ref.as_mut() else {
					tokio::time::sleep(Duration::from_millis(500)).await;
					continue;
				};

				match solver.check_next() {
					Ok(Some(nonce)) => worker.submit(nonce.nonce),
					Err(err) => {
						warn!("Mining failed: {:?}", err);
						if matches!(err, RandomXError::CreationError(_)) {
							tokio::time::sleep(Duration::from_secs(10)).await;
						}
					},
					_ => (),
				}
			}
		}
		.boxed();
		handle.spawn_essential_blocking("mining-voter", Some("block-authoring"), task);
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		compute_worker::{BlockComputeNonce, ComputeSolver},
		mock_notary::setup_logs,
	};
	use codec::Encode;
	use sp_core::{H256, U256};

	#[test]
	fn nonce_verify_compute() {
		let mut bytes = [0u8; 32];
		bytes[31] = 1;

		let key_block_hash = H256::from_slice(&[1u8; 32]);

		assert!(BlockComputeNonce::is_valid(&U256::from(1), bytes.to_vec(), &key_block_hash, 1));

		assert!(!BlockComputeNonce::is_valid(
			&U256::from(1),
			bytes.to_vec(),
			&key_block_hash,
			10_000
		));
	}
	#[test]
	fn it_can_reuse_a_nonce_algorithm_multiple_times() {
		setup_logs();

		let mut bytes = [0u8; 32];
		bytes[31] = 2;
		let key_block_hash = H256::from_slice(&[1u8; 32]);
		let pre_hash = bytes.to_vec();
		let mut solver = ComputeSolver::new(0, pre_hash.clone(), key_block_hash, 1);

		for _ in 0..100 {
			let did_solve = solver.check_next().is_ok_and(|x| x.is_some());

			assert_eq!(solver.wip_nonce_hash, solver.wip_nonce.encode());
			assert_eq!(
				did_solve,
				BlockComputeNonce::is_valid(
					&solver.wip_nonce.nonce,
					pre_hash.clone(),
					&key_block_hash,
					1
				)
			);
		}
	}
}
