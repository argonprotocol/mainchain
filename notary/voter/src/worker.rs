use std::{
	sync::{
		Arc,
		atomic::{AtomicU32, AtomicUsize, Ordering},
	},
	thread,
	thread::JoinHandle,
	time::Duration,
};
use std::time::Instant;

use codec::Encode;
use log::*;
use parking_lot::Mutex;
use sc_utils::notification::{NotificationSender, NotificationStream, TracingKeyStr};
use sp_core::{blake2_256, H256, RuntimeDebug, U256};

use ulx_notary_primitives::{AccountId, BlockVoteEligibility, BlockVoteSource, ComputePuzzle};

type BlockHash = H256;
pub enum NonceSolverResult {
	Found { puzzle: ComputePuzzle<BlockHash> },
	MovedToTax,
	NotFound,
	Waiting,
}

pub struct ComputeSolver {
	pub version: Version,
	puzzle: ComputePuzzle<BlockHash>,
	threshold: U256,
	pre_hash: Vec<u8>,
	allowed_source: BlockVoteSource,
}

impl ComputeSolver {
	pub fn new(
		version: Version,
		block_hash: BlockHash,
		account_id: AccountId,
		index: u32,
		block_vote_eligibility: BlockVoteEligibility
	) -> Self {
		let mut solver = ComputeSolver {
			version,
			threshold: U256::MAX / U256::from(block_vote_eligibility.minimum).max(U256::one()),
			pre_hash: vec![],
			puzzle: ComputePuzzle {
				account_id,
				block_hash,
				index,
				puzzle_proof: U256::from(rand::random::<u64>()),
			},
			allowed_source: block_vote_eligibility.allowed_sources,
		};
		solver.pre_hash = solver.puzzle.encode().to_vec();
		solver
	}

	pub fn update_index(&mut self, index: u32) {
		self.puzzle.index = index;
		self.pre_hash = self.puzzle.encode().to_vec();
	}

	pub fn check_next(&mut self) -> NonceSolverResult {
		if self.allowed_source == BlockVoteSource::Tax {
			return NonceSolverResult::MovedToTax;
		}

		self.puzzle.puzzle_proof =
			self.puzzle.puzzle_proof.checked_add(U256::one()).unwrap_or_default();

		self.re_encode();

		let nonce = self.calculate_puzzle_nonce();
		if nonce <= self.threshold {
			NonceSolverResult::Found { puzzle: self.puzzle.clone() }
		} else {
			NonceSolverResult::NotFound
		}
	}

	fn re_encode(&mut self) {
		let prehash = &mut self.pre_hash;
		let puzzle_proof = self.puzzle.puzzle_proof;
		prehash.truncate(prehash.len() - puzzle_proof.size_hint());
		puzzle_proof.encode_to(prehash);
	}

	fn calculate_puzzle_nonce(&self) -> U256 {
		let hash = blake2_256(&self.pre_hash);

		U256::from_big_endian(&hash[..])
	}
}

/// Version of the mining worker.
#[derive(Eq, PartialEq, Clone, Copy, RuntimeDebug)]
pub struct Version(pub usize);

/// Mining worker that exposes structs to query the current mining build and submit mined blocks.
#[derive(Clone)]
pub struct MiningHandle {
	version: Arc<AtomicUsize>,
	account_id: AccountId,
	index: Arc<AtomicU32>,
	metadata: Arc<Mutex<Option<MiningMetadata>>>,
}

/// Mining metadata. This is the information needed to start an actual mining loop.
#[derive(Clone, Eq, PartialEq)]
pub struct MiningMetadata {
	/// Currently known best hash which the pre-hash is built on.
	pub best_hash: BlockHash,
	pub vote_eligibility: BlockVoteEligibility,
	pub seen_time: Instant,
}

impl MiningHandle {
	fn increment_version(&self) {
		self.version.fetch_add(1, Ordering::SeqCst);
	}

	fn increment_account_index(&self) -> u32 {
		self.index.fetch_add(1, Ordering::SeqCst)
	}

	pub fn new(account_id: AccountId, index: Arc<AtomicU32>) -> Self {
		Self {
			version: Arc::new(AtomicUsize::new(0)),
			metadata: Arc::new(Mutex::new(None)),
			account_id,
			index,
		}
	}

	pub fn on_build(&self, best_hash: BlockHash, vote_eligibility: BlockVoteEligibility) {
		let mut metadata = self.metadata.lock();
		*metadata = Some(MiningMetadata { best_hash, vote_eligibility, seen_time: Instant::now() });
		self.increment_version();
	}
	/// Get the version of the mining worker.
	///
	/// This returns type `Version` which can only compare equality. If `Version` is unchanged, then
	/// it can be certain that `best_hash` and `metadata` were not changed.
	pub fn version(&self) -> Version {
		Version(self.version.load(Ordering::SeqCst))
	}

	/// Get the current best hash. `None` if the worker has just started or the client is doing
	/// major syncing.
	pub fn best_hash(&self) -> Option<BlockHash> {
		self.metadata.lock().as_ref().map(|b| b.best_hash)
	}

	/// Get a copy of the current mining metadata, if available.
	pub fn metadata(&self) -> Option<MiningMetadata> {
		self.metadata.lock().as_ref().map(|b| b.clone())
	}

	pub fn create_solver(&self) -> Option<ComputeSolver> {
		match self.metadata() {
			Some(x) => {
				Some(ComputeSolver::new(
					self.version(),
					x.best_hash.clone(),
					self.account_id.clone(),
					self.increment_account_index(),
					x.vote_eligibility,
				))
			},
			_ => None,
		}
	}

	pub fn is_valid_solver(&self, solver: &Option<Box<ComputeSolver>>) -> bool {
		solver.as_ref().map(|a| a.version) == Some(self.version())
	}
}

const LOG_TARGET: &str = "voter::compute::miner";
pub fn run_compute_miner_thread(worker: MiningHandle, vote_sender: NotificationSender<ComputePuzzle<BlockHash>>) -> JoinHandle<()> {
	let mut solver: Option<Box<ComputeSolver>> = None;
	thread::spawn(move || loop {
		if !worker.is_valid_solver(&solver) {
			if let Some(finder) = worker.create_solver() {
				solver = Some(Box::new(finder));
			}
		}

		if let Some(mut_solver) = solver.as_mut() {
			match mut_solver.check_next() {
				NonceSolverResult::Found { puzzle } => {
					let next_index = worker.increment_account_index();
					mut_solver.update_index(next_index);
					vote_sender.notify(|| Ok::<_, anyhow::Error>(puzzle.clone())).expect("should be able to send a vote");
					info!(target: LOG_TARGET, "Found a solution for block {}, index {} vote: {:?}",
						puzzle.block_hash, 
						puzzle.index, 
						puzzle.puzzle_proof);
				},
				NonceSolverResult::NotFound => continue,
				NonceSolverResult::MovedToTax => {
					info!(target: LOG_TARGET, "Proof of Tax is activated, leaving mining thread.");
					return
				},
				NonceSolverResult::Waiting => {
					thread::sleep(Duration::new(1, 0));
				},
			}
		} else {
			thread::sleep(Duration::from_millis(500));
		}
	})
}

pub type ComputeVoteStream = NotificationStream<ComputePuzzle<BlockHash>, NotebookHeaderTracingKey>;

#[derive(Clone)]
pub struct NotebookHeaderTracingKey;
impl TracingKeyStr for NotebookHeaderTracingKey {
	const TRACING_KEY: &'static str = "mpsc_block_vote_notification_stream";
}
