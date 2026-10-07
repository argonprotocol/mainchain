use crate::{
	block_creator::BlockProposal, error::Error, metrics::ConsensusMetrics,
	notary_client::VotingPowerInfo,
};
use argon_primitives::{
	block_seal::ComputePuzzle,
	prelude::*,
	tick::{Ticker, MAX_BLOCKS_PER_TICK},
	ArgonDigests, BlockSealApis, BlockSealAuthorityId, BlockSealDigest, ComputeDifficulty,
	NotebookApis, TickApis,
};
use argon_randomx::{calculate_hash, calculate_mining_hash, RandomXError};
use argon_runtime::NotebookVerifyError;
use codec::{Codec, Encode};
use frame_support::CloneNoBound;
use futures::prelude::*;
use log::*;
use parking_lot::Mutex;
use rand::RngExt;
use sc_client_api::AuxStore;
use sc_service::TaskManager;
use sc_utils::mpsc::TracingUnboundedSender;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::{traits::SpawnEssentialNamed, H256, U256};
use sp_runtime::traits::{Block as BlockT, Header};
use std::{
	marker::PhantomData,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	time::Duration,
};
use tokio::task::yield_now;

pub(crate) type Version = usize;
const VOTE_BLOCKS_BEFORE_MEMORY_EVICTION: usize = 10;

/// Mining metadata. This is the information needed to start an actual mining loop.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct MiningMetadata<H> {
	/// Currently known best hash which the pre-hash is built on.
	pub best_hash: H,
	/// The time at which mining is activated (in milliseconds).
	pub activate_mining_time: u64,
	/// The time at which we should emergency submit notebooks (in milliseconds).
	pub submit_notebooks_time: u64,
	/// Are there valid votes for the block?
	pub has_eligible_votes: bool,
	/// Is the node bootstrap mining
	pub is_bootstrap_mining: bool,
	/// At which tick do we kick in mining no matter what?
	pub emergency_tick: Tick,
	/// The randomx key block hash.
	pub key_block_hash: H256,
	/// The puzzle difficulty
	pub difficulty: ComputeDifficulty,
	/// Solving with notebooks at tick
	pub solving_with_notebooks_at_tick: (Tick, u32),
}

pub(crate) struct SolvingBlock<B: BlockT> {
	pub proposal: BlockProposal<B>,
	/// Mining pre-hash.
	pub pre_hash: B::Hash,
	/// Pre-runtime digest item.
	pub difficulty: ComputeDifficulty,
	/// Key block hash.
	pub key_block_hash: H256,
}

/// Mining worker that exposes structs to query the current mining build and submit mined blocks.
#[derive(CloneNoBound)]
pub(crate) struct ComputeHandle<B: BlockT> {
	version: Arc<AtomicUsize>,
	metadata: Arc<Mutex<Option<MiningMetadata<B::Hash>>>>,
	solving_block: Arc<Mutex<Option<SolvingBlock<B>>>>,
	block_found_tx: TracingUnboundedSender<(SolvingBlock<B>, BlockSealDigest)>,
	last_memory_check: Arc<Mutex<Option<B::Hash>>>,
}

impl<B> ComputeHandle<B>
where
	B: BlockT,
{
	pub(crate) fn new(
		block_found_tx: TracingUnboundedSender<(SolvingBlock<B>, BlockSealDigest)>,
	) -> Self {
		Self {
			version: Arc::new(AtomicUsize::new(0)),
			solving_block: Arc::new(Mutex::new(None)),
			block_found_tx,
			metadata: Arc::new(Mutex::new(None)),
			last_memory_check: Arc::new(Mutex::new(None)),
		}
	}

	pub fn on_best_block<C: HeaderBackend<B>>(&self, client: &C, best_hash: B::Hash) {
		if !argon_randomx::full_vm::has_cached_data() {
			return;
		}

		let mut last_memory_check = self.last_memory_check.lock();
		if *last_memory_check == Some(best_hash) {
			return;
		}

		// Walk the selected branch so repeated imports and competing forks cannot inflate the
		// streak.
		let mut hash = best_hash;
		for _ in 0..VOTE_BLOCKS_BEFORE_MEMORY_EVICTION {
			let header = match client.header(hash) {
				Ok(Some(header)) => header,
				Ok(None) => return,
				Err(err) => {
					warn!("Unable to inspect vote block streak for compute memory eviction: {err}");
					return;
				},
			};
			let is_vote = header
				.digest()
				.convert_first(|item| item.as_block_seal())
				.is_some_and(|seal| seal.is_vote());
			if !is_vote {
				*last_memory_check = Some(best_hash);
				return;
			}
			hash = *header.parent_hash();
		}

		self.stop_solving_current();
		if argon_randomx::full_vm::evict_mining_memory() {
			*last_memory_check = Some(best_hash);
			info!("Evicted compute mining memory after {VOTE_BLOCKS_BEFORE_MEMORY_EVICTION} consecutive vote blocks");
		}
	}

	pub(crate) fn stop_solving_current(&self) {
		*self.solving_block.lock() = None;
		*self.metadata.lock() = None;
		self.increment_version();
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

	fn solving_with_notebooks_at_tick(&self) -> Option<(Tick, u32)> {
		self.metadata.lock().as_ref().map(|x| x.solving_with_notebooks_at_tick)
	}

	pub fn is_solving(&self) -> bool {
		self.solving_block.lock().is_some()
	}

	pub(crate) fn new_best_block(&self, metadata: MiningMetadata<B::Hash>) {
		self.stop_solving_current();
		*self.metadata.lock() = Some(metadata);
	}

	pub(crate) fn start_solving(&self, proposal: BlockProposal<B>) {
		let best_hash = proposal.proposal.block.header().parent_hash();
		if self.best_hash() != Some(*best_hash) {
			self.stop_solving_current();
			return;
		}
		let difficulty = self.metadata.lock().as_ref().map(|x| x.difficulty).unwrap_or_default();
		let pre_hash = proposal.proposal.block.header().hash();
		let key_block_hash =
			self.metadata.lock().as_ref().map(|x| x.key_block_hash).unwrap_or_default();

		*self.solving_block.lock() =
			Some(SolvingBlock { pre_hash, difficulty, proposal, key_block_hash });
	}

	pub fn is_valid_solver(&self, solver: &Option<Box<ComputeSolver>>) -> bool {
		solver.as_ref().map(|a| a.version) == Some(self.version())
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

	pub fn ready_to_solve(&self, current_tick: Tick, now_millis: u64) -> bool {
		match self.metadata.lock().as_ref() {
			Some(x) => {
				// if we've passed the emergency tick, we should mine no matter what
				if current_tick > x.emergency_tick {
					return true;
				}
				if x.is_bootstrap_mining {
					return true;
				}
				// must be past the parent tick and not have tax votes
				if !x.has_eligible_votes && x.activate_mining_time < now_millis {
					return true;
				}
				if x.solving_with_notebooks_at_tick.1 > 0 && x.submit_notebooks_time < now_millis {
					return true;
				}
				false
			},

			_ => false,
		}
	}

	pub fn submit(&self, nonce: BlockComputeNonce, key_block_hash: &H256) {
		// Hold the lock for the entire decision so we don't race a new build / best-hash update.
		let mut guard = self.solving_block.lock();
		let Some(build) = guard.as_ref() else {
			trace!("Unable to submit mined block in compute worker: internal build does not exist");
			return;
		};

		if build.pre_hash.as_ref() != nonce.pre_hash.as_slice() {
			trace!(
				"Unable to submit mined block in compute worker: pre-hash mismatch. Expected {:?}, got {:?}",
				build.pre_hash, nonce.pre_hash
			);
			return;
		}
		if build.key_block_hash != *key_block_hash {
			trace!(
				"Unable to submit mined block in compute worker: key block hash mismatch. Expected {:?}, got {:?}",
				build.key_block_hash, key_block_hash
			);
			return;
		}

		// Consume the build (first valid submit wins).
		let build = guard.take().expect("checked Some above");
		drop(guard);

		let _ = self
			.block_found_tx
			.unbounded_send((build, BlockSealDigest::Compute { nonce: nonce.nonce }))
			.inspect_err(|e| error!("Error sending block found message: {e:?}"));
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
		let hash = match hash {
			Ok(hash) => hash,
			Err(e) => {
				// Local verifier failure must be treated as node-fatal to avoid classifying a
				// peer's block as invalid due to this node's RandomX/runtime issue.
				panic!(
					"Local RandomX error while verifying compute nonce; refusing to mark peer block invalid: {e:?}"
				);
			},
		};
		let threshold = Self::threshold(compute_difficulty);
		Self::meets_threshold(hash.as_fixed_bytes(), threshold)
	}
}

/// This is a lightweight struct that lives in-thread for each mining thread.
#[derive(Clone)]
pub(crate) struct ComputeSolver {
	pub version: Version,
	memory_generation: usize,
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
		let mut rng = rand::rng();
		let mut bytes = [0u8; 32];
		rng.fill(&mut bytes);
		let mut solver = ComputeSolver {
			version,
			memory_generation: argon_randomx::full_vm::cache_generation(),
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

		let Some(hash) =
			calculate_mining_hash(&self.key_block_hash, payload, self.memory_generation)?
		else {
			return Ok(None);
		};
		if BlockComputeNonce::meets_threshold(hash.as_fixed_bytes(), self.threshold) {
			return Ok(Some(self.wip_nonce.clone()));
		}
		Ok(None)
	}
}

pub fn run_compute_solver_threads<B, C>(
	task_handle: &TaskManager,
	worker: ComputeHandle<B>,
	threads: u32,
	consensus_metrics: Arc<Option<ConsensusMetrics<C>>>,
) where
	B: BlockT,
	C: AuxStore + Send + Sync + 'static,
{
	let handle = task_handle.spawn_essential_handle();
	for _ in 0..threads {
		let worker = worker.clone();
		let metrics_copy = consensus_metrics.clone();
		let task = async move {
			let mut counter = 0;
			let mut solver_ref = None;
			loop {
				argon_randomx::full_vm::release_vm_if_evicted();
				if !worker.is_valid_solver(&solver_ref) {
					solver_ref = worker.create_solver().map(Box::new);
					if counter > 0 &&
						let Some(metrics_copy) = metrics_copy.as_ref()
					{
						metrics_copy.record_compute_hashes(counter);
					}
					counter = 0;
				}

				let Some(solver) = solver_ref.as_mut() else {
					tokio::time::sleep(Duration::from_millis(500)).await;
					continue;
				};

				counter += 1;
				let key_block_hash = solver.key_block_hash;
				match solver.check_next() {
					Ok(Some(nonce)) => worker.submit(nonce, &key_block_hash),
					Err(err) => {
						warn!("Mining failed: {err:?}");
						if matches!(err, RandomXError::CreationError(_)) {
							tokio::time::sleep(Duration::from_secs(10)).await;
						}
					},
					_ => (),
				}
				yield_now().await;
				#[cfg(feature = "ci")]
				{
					// add some yielding to help CI tests with less cpus
					if counter % 10 == 0 {
						tokio::time::sleep(Duration::from_millis(10)).await;
					}
				}
			}
		}
		.boxed();
		handle.spawn_essential_blocking("mining-voter", Some("block-authoring"), task);
	}
}

pub trait ComputeApisExt<B: BlockT, AC> {
	fn current_tick(&self, block_hash: B::Hash) -> Result<Tick, Error>;
	fn genesis_hash(&self) -> B::Hash;
	fn has_eligible_votes(&self, block_hash: B::Hash) -> Result<bool, Error>;
	fn is_bootstrap_mining(&self, block_hash: B::Hash) -> Result<bool, Error>;
	fn compute_puzzle(&self, block_hash: B::Hash) -> Result<ComputePuzzle<B>, Error>;
	fn blocks_at_tick(&self, block_hash: B::Hash, tick: Tick) -> Result<u32, Error>;
}

impl<B, C, AC> ComputeApisExt<B, AC> for C
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + AuxStore,
	C::Api: NotebookApis<B, NotebookVerifyError>
		+ TickApis<B>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>,
	AC: Clone + Codec,
{
	fn current_tick(&self, block_hash: B::Hash) -> Result<Tick, Error> {
		self.runtime_api().current_tick(block_hash).map_err(Into::into)
	}

	fn genesis_hash(&self) -> B::Hash {
		self.info().genesis_hash
	}

	fn has_eligible_votes(&self, block_hash: B::Hash) -> Result<bool, Error> {
		self.runtime_api().has_eligible_votes(block_hash).map_err(Into::into)
	}

	fn is_bootstrap_mining(&self, block_hash: B::Hash) -> Result<bool, Error> {
		self.runtime_api().is_bootstrap_mining(block_hash).map_err(Into::into)
	}

	fn compute_puzzle(&self, block_hash: B::Hash) -> Result<ComputePuzzle<B>, Error> {
		self.runtime_api().compute_puzzle(block_hash).map_err(Into::into)
	}

	fn blocks_at_tick(&self, block_hash: B::Hash, tick: Tick) -> Result<u32, Error> {
		self.runtime_api()
			.blocks_at_tick(block_hash, tick)
			.map(|a| a.len() as u32)
			.map_err(Into::into)
	}
}

pub struct ComputeState<B: BlockT, C, A> {
	compute_handle: ComputeHandle<B>,
	client: Arc<C>,
	genesis_hash: B::Hash,
	ticker: Ticker,
	compute_delay: u64,
	_phantom: PhantomData<A>,
}

impl<B: BlockT, C, A> ComputeState<B, C, A>
where
	C: ComputeApisExt<B, A> + AuxStore + Send + Sync + 'static,
	A: Codec + Clone,
{
	pub fn new(compute_handle: ComputeHandle<B>, client: Arc<C>, ticker: Ticker) -> Self {
		// wait a little before we start mining
		let compute_delay = ticker.tick_duration_millis / 5;
		let genesis_hash = client.genesis_hash();
		ComputeState {
			compute_handle,
			client,
			genesis_hash,
			ticker,
			compute_delay,
			_phantom: PhantomData,
		}
	}

	/// This is called when a new notebook is found at the same tick as the best block.
	///
	/// NOTE: ensure this block hash has state before calling this.
	pub fn on_new_notebook_tick(
		&self,
		best_hash: B::Hash,
		updated_notebooks_at_tick: Option<VotingPowerInfo>,
		consensus_metrics: &Arc<Option<ConsensusMetrics<C>>>,
		submitting_tick: Tick,
	) -> Option<B::Hash> {
		// see if we have more notebooks at the same tick. if so, we should restart compute to
		// include them
		let latest_block_tick = self.client.current_tick(best_hash).unwrap_or_default();
		let mut solve_notebook_tick = latest_block_tick;
		let mut notebooks = 0;

		if let Some((notebook_tick, _, notebooks_at_latest_tick)) = updated_notebooks_at_tick {
			let (solving_for_tick, solving_with_notebooks) = self
				.compute_handle
				.solving_with_notebooks_at_tick()
				.unwrap_or((notebook_tick, 0));
			// If we're solving with no notebooks, we should stop and start solving with the new
			// ones. However, if you try too hard to optimize the notebook inclusion, you won't
			// ever finish a block. So we only do this if we're not already solving with notebooks.
			if notebook_tick >= solving_for_tick &&
				solving_with_notebooks == 0 &&
				notebooks_at_latest_tick > 0
			{
				tracing::info!(
					?notebooks_at_latest_tick,
					notebook_tick,
					"Found new notebooks at tick. Will try to solve tick with compute"
				);
				solve_notebook_tick = notebook_tick;
				notebooks = notebooks_at_latest_tick;
				self.compute_handle.stop_solving_current();
				if let Some(metrics) = consensus_metrics.as_ref() {
					metrics.did_reset_compute_for_notebooks();
				}
			}
		}

		// don't add 5th notebook at tick unless we have a notebook
		let blocks_at_tick =
			self.client.blocks_at_tick(best_hash, submitting_tick).unwrap_or_default();
		if (blocks_at_tick == MAX_BLOCKS_PER_TICK - 1 && notebooks == 0) ||
			blocks_at_tick == MAX_BLOCKS_PER_TICK
		{
			tracing::info!(
				?notebooks,
				blocks_at_tick,
				"Max blocks at tick (or without notebooks). Stop solving."
			);
			self.compute_handle.stop_solving_current();
			return None;
		}

		if self.compute_handle.best_hash() != Some(best_hash) {
			tracing::trace!(
				new_best_hash = ?best_hash,
				current_best_hash = ?self.compute_handle.best_hash(),
				"Best hash has changed. Setting new compute handle"
			);
			let has_eligible_votes = self.client.has_eligible_votes(best_hash).unwrap_or_default();
			let is_bootstrap_mining =
				self.client.is_bootstrap_mining(best_hash).unwrap_or_default();
			let compute_puzzle = self
				.client
				.compute_puzzle(best_hash)
				.inspect_err(|err| {
					warn!("Unable to pull new block for compute miner. No difficulty found!! {err}")
				})
				.ok()?;
			let activate_mining_time =
				self.ticker.time_for_tick(solve_notebook_tick + 1) + self.compute_delay;
			self.compute_handle.new_best_block(MiningMetadata {
				best_hash,
				has_eligible_votes,
				activate_mining_time,
				is_bootstrap_mining,
				submit_notebooks_time: activate_mining_time + (2 * self.compute_delay),
				key_block_hash: compute_puzzle.get_key_block(self.genesis_hash),
				emergency_tick: latest_block_tick + 2,
				difficulty: compute_puzzle.difficulty,
				solving_with_notebooks_at_tick: (solve_notebook_tick, notebooks),
			});
		}

		Some(best_hash)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock_notary::setup_logs;
	use argon_primitives::{tick::Ticker, HashOutput};
	use argon_runtime::Block;
	use codec::Encode;
	use sc_utils::mpsc::tracing_unbounded;
	use sp_core::{H256, U256};
	static MINING_MEMORY_TEST_LOCK: Mutex<()> = Mutex::new(());

	struct ApiState {
		ticker: Ticker,
		best_hash: HashOutput,
		genesis_hash: HashOutput,
		is_bootstrap_mining: bool,
		has_eligible_votes: bool,
		compute_puzzle: ComputePuzzle<Block>,
	}

	#[derive(Clone)]
	pub struct Api {
		state: Arc<Mutex<ApiState>>,
	}
	impl AuxStore for Api {
		fn insert_aux<
			'a,
			'b: 'a,
			'c: 'a,
			I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
			D: IntoIterator<Item = &'a &'b [u8]>,
		>(
			&self,
			_insert: I,
			_delete: D,
		) -> sp_blockchain::Result<()> {
			Ok(())
		}
		fn get_aux(&self, _key: &[u8]) -> sp_blockchain::Result<Option<Vec<u8>>> {
			Ok(None)
		}
	}
	impl ComputeApisExt<Block, AccountId> for Api {
		fn current_tick(&self, _block_hash: HashOutput) -> Result<Tick, Error> {
			Ok(self.state.lock().ticker.current())
		}

		fn genesis_hash(&self) -> HashOutput {
			self.state.lock().genesis_hash
		}

		fn has_eligible_votes(&self, _block_hash: HashOutput) -> Result<bool, Error> {
			Ok(self.state.lock().has_eligible_votes)
		}

		fn is_bootstrap_mining(&self, _block_hash: HashOutput) -> Result<bool, Error> {
			Ok(self.state.lock().is_bootstrap_mining)
		}

		fn compute_puzzle(&self, _block_hash: HashOutput) -> Result<ComputePuzzle<Block>, Error> {
			Ok(self.state.lock().compute_puzzle.clone())
		}

		fn blocks_at_tick(&self, _block_hash: HashOutput, _tick: Tick) -> Result<u32, Error> {
			Ok(0)
		}
	}

	#[test]
	fn cancelled_solver_cannot_repopulate_evicted_memory() {
		let _mining_memory_guard = MINING_MEMORY_TEST_LOCK.lock();
		let (tx, _) = tracing_unbounded("node::consensus::compute_block_stream", 10);
		let worker = ComputeHandle::<Block>::new(tx);
		let key = H256::repeat_byte(73);
		let mut solver = Some(Box::new(ComputeSolver::new(worker.version(), vec![1; 32], key, 1)));
		assert!(solver.as_mut().unwrap().check_next().unwrap().is_some());
		assert!(worker.is_valid_solver(&solver));

		// Eviction can happen after admission to the loop, before the next hash starts.
		worker.stop_solving_current();
		assert!(argon_randomx::full_vm::evict_mining_memory());
		assert!(!argon_randomx::full_vm::has_cached_data());
		assert!(solver.as_mut().unwrap().check_next().unwrap().is_none());
		assert!(!argon_randomx::full_vm::has_cached_data(), "cancelled work reallocated a dataset");
		argon_randomx::full_vm::release_vm_if_evicted();
	}

	#[test]
	fn vote_block_recovery_evicts_memory_without_disabling_emergency_fallback() {
		use crate::mock_importer::{Block as ImportedBlock, MemChain};
		use argon_primitives::BlockSealAuthoritySignature;
		use sp_core::ByteArray;
		use sp_runtime::{generic::Header as GenericHeader, traits::BlakeTwo256, Digest};
		let _mining_memory_guard = MINING_MEMORY_TEST_LOCK.lock();

		let genesis = GenericHeader::<u32, BlakeTwo256>::new(
			0,
			H256::zero(),
			H256::zero(),
			H256::zero(),
			Digest::default(),
		);
		let client = MemChain::new(genesis);
		let (tx, _) = tracing_unbounded("node::consensus::compute_block_stream", 10);
		let worker = ComputeHandle::<ImportedBlock>::new(tx);
		let key = H256::repeat_byte(72);
		assert!(calculate_mining_hash(
			&key,
			b"fallback block",
			argon_randomx::full_vm::cache_generation(),
		)
		.unwrap()
		.is_some());

		let mut best_hash = client.info().best_hash;
		for number in 1..=13 {
			let mut seal = BlockSealDigest::Vote {
				seal_strength: U256::one(),
				signature: BlockSealAuthoritySignature::from_slice(&[0; 64]).unwrap(),
				miner_nonce_score: None,
			};
			if number == 3 {
				seal = BlockSealDigest::Compute { nonce: U256::one() };
			}
			let header = GenericHeader::<u32, BlakeTwo256>::new(
				number,
				H256::zero(),
				H256::zero(),
				best_hash,
				Digest { logs: vec![seal.to_digest()] },
			);
			best_hash = header.hash();
			client.insert(header);
			client.force_best(number, best_hash);
			// Polling the same selected tip cannot count as additional vote blocks.
			for _ in 0..10 {
				worker.on_best_block(&client, best_hash);
			}
			assert_eq!(argon_randomx::full_vm::has_cached_data(), number < 13);
		}
		assert_eq!(worker.version(), 1);

		// Eligible votes on a stalled vote tip must not prevent compute from restarting.
		worker.new_best_block(MiningMetadata {
			best_hash,
			activate_mining_time: u64::MAX,
			submit_notebooks_time: u64::MAX,
			has_eligible_votes: true,
			is_bootstrap_mining: false,
			emergency_tick: 20,
			key_block_hash: key,
			difficulty: 1,
			solving_with_notebooks_at_tick: (18, 0),
		});
		assert!(!worker.ready_to_solve(20, 0));
		assert!(worker.ready_to_solve(21, 0));
		assert!(calculate_mining_hash(
			&key,
			b"next fallback block",
			argon_randomx::full_vm::cache_generation(),
		)
		.unwrap()
		.is_some());
		worker.on_best_block(&client, best_hash);
		assert!(argon_randomx::full_vm::has_cached_data());

		assert!(argon_randomx::full_vm::evict_mining_memory());
		argon_randomx::full_vm::release_vm_if_evicted();
	}

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
		let _mining_memory_guard = MINING_MEMORY_TEST_LOCK.lock();

		let mut bytes = [0u8; 32];
		bytes[31] = 2;
		let key_block_hash = H256::from_slice(&[1u8; 32]);
		let pre_hash = bytes.to_vec();
		let mut solver = ComputeSolver::new(0, pre_hash.clone(), key_block_hash, 1);

		for _ in 0..2 {
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

	#[test]
	fn it_prefers_to_solve_with_a_notebook() {
		setup_logs();

		let ticker = Ticker::new(1000, 2);
		let (tx, _) = tracing_unbounded("node::consensus::compute_block_stream", 10);
		let compute_handle = ComputeHandle::<Block>::new(tx);

		let state_best_hash = H256::from_slice(&[1u8; 32]);
		let state = Arc::new(Mutex::new(ApiState {
			ticker,
			best_hash: state_best_hash,
			genesis_hash: H256::from_slice(&[0u8; 32]),
			is_bootstrap_mining: false,
			has_eligible_votes: false,
			compute_puzzle: ComputePuzzle {
				difficulty: 1,
				randomx_key_block: H256::from_slice(&[1u8; 32]).into(),
			},
		}));
		let api = Api { state: state.clone() };

		let compute_state =
			ComputeState::new(compute_handle.clone(), Arc::new(api.clone()), ticker);

		let latest_tick = api.state.lock().ticker.current();
		let best_hash =
			compute_state.on_new_notebook_tick(state_best_hash, None, &Arc::new(None), latest_tick);

		assert_eq!(best_hash, Some(api.state.lock().best_hash));
		assert_eq!(compute_handle.best_hash().clone(), Some(api.state.lock().best_hash));

		// if we already have notebooks at the tick, we should try to solve with them
		let best_hash = compute_state.on_new_notebook_tick(
			state_best_hash,
			Some((latest_tick, 1, 2)),
			&Arc::new(None),
			latest_tick,
		);
		assert_eq!(best_hash, Some(api.state.lock().best_hash));
		assert_eq!(
			compute_handle
				.metadata
				.lock()
				.as_ref()
				.map(|a| a.solving_with_notebooks_at_tick),
			Some((latest_tick, 2))
		);

		// we should not replace for an older tick
		let _best_hash = compute_state.on_new_notebook_tick(
			state_best_hash,
			Some((latest_tick - 1, 1, 1)),
			&Arc::new(None),
			latest_tick,
		);
		let notebooks_at_tick = compute_state.compute_handle.solving_with_notebooks_at_tick();
		assert_eq!(notebooks_at_tick, Some((latest_tick, 2)));
	}
}
