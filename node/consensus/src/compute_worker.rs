use std::{
	pin::Pin,
	string::ToString,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	time::{Duration, Instant},
};

use argon_bitcoin_utxo_tracker::UtxoTracker;
use codec::Codec;
use futures::{
	executor::block_on,
	future::BoxFuture,
	prelude::*,
	task::{Context, Poll},
};
use futures_timer::Delay;
use log::*;
use parking_lot::Mutex;
use sc_client_api::{AuxStore, BlockchainEvents, ImportNotifications};
use sc_consensus::BoxBlockImport;
use sc_service::TaskManager;
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::UniqueSaturatedInto;
use sp_blockchain::HeaderBackend;
use sp_consensus::{Environment, Proposal, Proposer, SyncOracle};
use sp_core::{traits::SpawnEssentialNamed, RuntimeDebug, H256, U256};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_timestamp::Timestamp;

use argon_node_runtime::{NotaryRecordT, NotebookVerifyError};
use argon_primitives::{inherents::BlockSealInherentNodeSide, tick::Tick, *};
use argon_randomx::RandomXError;

use crate::{
	aux_client::ArgonAux, block_creator, block_creator::propose, compute_solver::ComputeSolver,
	digests::get_tick_digest, error::Error,
};

/// Version of the mining worker.
#[derive(Eq, PartialEq, Clone, Copy, RuntimeDebug)]
pub struct Version(pub usize);

/// Mining metadata. This is the information needed to start an actual mining loop.
#[derive(Clone, Eq, PartialEq)]
pub struct MiningMetadata<H> {
	/// Currently known best hash which the pre-hash is built on.
	pub best_hash: H,
	pub start_time: Instant,
	pub has_tax_votes: bool,
	pub parent_tick: Tick,
	/// At which tick do we kick in mining no matter what?
	pub emergency_tick: Tick,
	pub key_block_hash: H256,
}

struct MiningBuild<B: BlockT, Proof> {
	pub proposal: Proposal<B, Proof>,
	/// Mining pre-hash.
	pub pre_hash: B::Hash,
	/// Pre-runtime digest item.
	pub difficulty: ComputeDifficulty,
}

/// Mining worker that exposes structs to query the current mining build and submit mined blocks.
pub struct MiningHandle<Block: BlockT, L: sc_consensus::JustificationSyncLink<Block>, Proof> {
	version: Arc<AtomicUsize>,
	justification_sync_link: Arc<L>,
	metadata: Arc<Mutex<Option<MiningMetadata<Block::Hash>>>>,
	build: Arc<Mutex<Option<MiningBuild<Block, Proof>>>>,
	block_import: Arc<tokio::sync::Mutex<BoxBlockImport<Block>>>,
}

impl<B, L, Proof> MiningHandle<B, L, Proof>
where
	B: BlockT,
	L: sc_consensus::JustificationSyncLink<B>,
	Proof: Send,
{
	fn increment_version(&self) {
		self.version.fetch_add(1, Ordering::SeqCst);
	}

	pub fn new(block_import: BoxBlockImport<B>, justification_sync_link: L) -> Self {
		Self {
			version: Arc::new(AtomicUsize::new(0)),
			justification_sync_link: Arc::new(justification_sync_link),
			build: Arc::new(Mutex::new(None)),
			block_import: Arc::new(tokio::sync::Mutex::new(block_import)),
			metadata: Arc::new(Mutex::new(None)),
		}
	}

	pub(crate) fn stop_solving_current(&self) {
		let mut build = self.build.lock();
		*build = None;
		let mut metadata = self.metadata.lock();
		*metadata = None;
		self.increment_version();
	}

	pub(crate) fn on_block(
		&self,
		best_hash: B::Hash,
		has_tax_votes: bool,
		parent_tick: Tick,
		emergency_tick: Tick,
		key_block: H256,
	) {
		self.stop_solving_current();
		let mut metadata = self.metadata.lock();
		*metadata = Some(MiningMetadata {
			best_hash,
			has_tax_votes,
			start_time: Instant::now(),
			parent_tick,
			key_block_hash: key_block,
			emergency_tick,
		});
	}

	pub(crate) fn start_solving(
		&self,
		best_hash: B::Hash,
		pre_hash: B::Hash,
		difficulty: ComputeDifficulty,
		proposal: Proposal<B, Proof>,
	) {
		if self.best_hash() != Some(best_hash) {
			self.stop_solving_current();
			return;
		}

		let mut build = self.build.lock();
		*build = Some(MiningBuild { pre_hash, difficulty, proposal });
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
	pub fn best_hash(&self) -> Option<B::Hash> {
		self.metadata.lock().as_ref().map(|b| b.best_hash)
	}

	/// Get the current key block hash. `None` if the worker has just started or the client is doing
	/// major syncing.
	pub fn key_block_hash(&self) -> Option<H256> {
		self.metadata.lock().as_ref().map(|b| b.key_block_hash)
	}

	pub fn build_hash(&self) -> Option<B::Hash> {
		self.build
			.lock()
			.as_ref()
			.map(|b| b.proposal.block.header().parent_hash())
			.cloned()
	}

	/// Get a copy of the current mining metadata, if available.
	pub fn metadata(&self) -> Option<MiningMetadata<B::Hash>> {
		self.metadata.lock().as_ref().cloned()
	}

	pub fn create_solver(&self) -> Option<ComputeSolver> {
		let key_block_hash = match self.metadata.lock().as_ref() {
			Some(x) => x.key_block_hash,
			_ => return None,
		};
		match self.build.lock().as_ref() {
			Some(x) => {
				let pre_hash = x.pre_hash;

				Some(ComputeSolver::new(
					self.version(),
					pre_hash.as_ref().to_vec(),
					H256::from_slice(key_block_hash.as_ref()),
					x.difficulty,
				))
			},
			_ => None,
		}
	}

	pub fn is_valid_solver(&self, solver: &Option<Box<ComputeSolver>>) -> bool {
		solver.as_ref().map(|a| a.version) == Some(self.version())
	}

	pub fn ready_to_solve(&self, current_tick: Tick) -> bool {
		match self.metadata.lock().as_ref() {
			Some(x) => {
				// if we've passed the emergency tick, we should mine no matter what
				if current_tick > x.emergency_tick {
					return true;
				}
				// must be past the parent tick and not have tax votes
				if x.parent_tick < current_tick && !x.has_tax_votes {
					return true;
				}
				false
			},

			_ => false,
		}
	}

	pub async fn submit(&mut self, nonce: U256) -> Result<(), Error> {
		let build = {
			let mut build = self.build.lock();
			// try to take out of option. if not exists, we've moved on
			build.take()
		};

		let Some(build) = build else {
			trace!("Unable to submit mined block in compute worker: internal build does not exist",);
			return Ok(());
		};

		self.increment_version();

		let mut block_import = self.block_import.lock().await;

		block_creator::submit_block::<B, L, Proof>(
			&mut block_import,
			build.proposal,
			&self.justification_sync_link,
			BlockSealDigest::Compute { nonce },
		)
		.await;
		Ok(())
	}
}

impl<B, L, Proof> Clone for MiningHandle<B, L, Proof>
where
	B: BlockT,
	L: sc_consensus::JustificationSyncLink<B>,
{
	fn clone(&self) -> Self {
		Self {
			version: self.version.clone(),
			justification_sync_link: self.justification_sync_link.clone(),
			build: self.build.clone(),
			metadata: self.metadata.clone(),
			block_import: self.block_import.clone(),
		}
	}
}

pub(crate) fn create_compute_solver_task<B, L, Proof>(
	mut worker: MiningHandle<B, L, Proof>,
) -> BoxFuture<'static, ()>
where
	B: BlockT,
	L: sc_consensus::JustificationSyncLink<B> + 'static,
	Proof: Send + 'static,
{
	async move {
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
				Ok(Some(nonce)) => {
					let _ = block_on(worker.submit(nonce.nonce));
				},
				Ok(None) => (),
				Err(RandomXError::CreationError(err)) => {
					warn!("RandomX creation failed for mining: {:?}", err);
					tokio::time::sleep(Duration::from_secs(10)).await;
				},
				Err(err) => {
					warn!("Mining failed: {:?}", err);
				},
			}
		}
	}
	.boxed()
}

pub fn run_compute_solver_threads<B, L, Proof>(
	task_handle: &TaskManager,
	worker: MiningHandle<B, L, Proof>,
	threads: usize,
) where
	B: BlockT,
	L: sc_consensus::JustificationSyncLink<B> + 'static,
	Proof: Send + 'static,
{
	for _ in 0..threads {
		let worker = worker.clone();
		task_handle.spawn_essential_handle().spawn_essential_blocking(
			"mining-voter",
			Some("block-authoring"),
			create_compute_solver_task(worker),
		);
	}
}

#[allow(clippy::too_many_arguments)]
pub fn create_compute_miner<B, C, E, SO, L, AccountId>(
	block_import: BoxBlockImport<B>,
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	mut env: E,
	sync_oracle: SO,
	account_id: AccountId,
	justification_sync_link: L,
	utxo_tracker: Arc<UtxoTracker>,
	max_time_to_build_block: Duration,
) -> (MiningHandle<B, L, <E::Proposer as Proposer<B>>::Proof>, impl Future<Output = ()> + 'static)
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + BlockchainEvents<B> + HeaderBackend<B> + AuxStore + 'static,
	C::Api: BlockSealApis<B, AccountId, BlockSealAuthorityId>
		+ TickApis<B>
		+ NotebookApis<B, NotebookVerifyError>
		+ NotaryApis<B, NotaryRecordT>
		+ BitcoinApis<B, Balance>,
	E: Environment<B> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<B>,
	SO: SyncOracle + Clone + Send + Sync + 'static,
	L: sc_consensus::JustificationSyncLink<B> + 'static,
	AccountId: Codec + Clone + 'static,
{
	// create a timer that fires whenever there are new blocks, or 500 ms go by
	let mut timer = UntilImportedOrTimeout::new(
		client.import_notification_stream(),
		Duration::from_millis(1000),
	);
	let mining_handle = MiningHandle::new(block_import, justification_sync_link);

	let handle_to_return = mining_handle.clone();

	let ticker = match client.runtime_api().ticker(client.info().finalized_hash) {
		Ok(x) => x,
		Err(err) => {
			panic!("Unable to pull ticker from runtime api: {}", err)
		},
	};

	let task = async move {
		loop {
			if timer.next().await.is_none() {
				// this should occur if the block import notifications completely stop... indicating
				// we should exit
				break;
			}
			if sync_oracle.is_major_syncing() {
				debug!("Skipping proposal due to sync.");
				mining_handle.stop_solving_current();
				continue;
			}

			let best_hash = client.info().best_hash;

			if mining_handle.best_hash() != Some(best_hash) {
				let Ok(Some(best_header)) = client.header(best_hash) else {
					continue;
				};
				let block_tick = get_tick_digest(best_header.digest()).unwrap_or_default();
				let emergency_tick = block_tick + 3;
				let has_eligible_votes =
					client.runtime_api().has_eligible_votes(best_hash).unwrap_or_default();
				let key_block = randomx_key_block(&client, &best_hash).unwrap_or_default();

				mining_handle.on_block(
					best_hash,
					has_eligible_votes,
					block_tick,
					emergency_tick,
					key_block,
				);
			}

			let time = Timestamp::current();
			let tick = ticker.tick_for_time(time.as_millis());
			if !mining_handle.ready_to_solve(tick) {
				let time_to_next_tick = ticker.duration_to_next_tick();
				tokio::time::sleep(time_to_next_tick).await;
				continue;
			}

			if mining_handle.build_hash() != Some(best_hash) {
				let difficulty = match client.runtime_api().compute_difficulty(best_hash) {
					Ok(x) => x,
					Err(err) => {
						warn!(
							"Unable to pull new block for compute miner. No difficulty found!! {}",
							err
						);
						continue;
					},
				};

				let proposal = match propose(
					client.clone(),
					aux_client.clone(),
					&mut env,
					account_id.clone(),
					tick,
					time.as_millis(),
					best_hash,
					BlockSealInherentNodeSide::Compute,
					utxo_tracker.clone(),
					max_time_to_build_block,
				)
				.await
				{
					Ok(x) => x,
					Err(err) => {
						warn!("Unable to propose a new block {}", err);
						continue;
					},
				};

				let pre_hash = proposal.block.header().hash();

				mining_handle.start_solving(best_hash, pre_hash, difficulty, proposal);
			}
		}
	};

	(handle_to_return, task)
}

/// The key K is selected to be the hash of a block in the blockchain - this block is called the
/// 'key block'. For optimal mining and verification performance, the key should change every 2048
/// blocks (~2.8 days) and there should be a delay of 64 blocks (~2 hours) between the key block and
/// the change of the key K. This can be achieved by changing the key when blockHeight % 2048 == 64
/// and selecting key block such that keyBlockHeight % 2048 == 0.
pub fn randomx_key_block<B, C>(client: &Arc<C>, parent_hash: &B::Hash) -> Result<H256, Error>
where
	B: BlockT,
	C: HeaderBackend<B>,
{
	const PERIOD: u32 = (1440.0 * 2.8) as u32; // 2.8 days
	const OFFSET: u32 = 120; // 2 hours

	let parent_number = client
		.number(*parent_hash)
		.map_err(|e| Error::Environment(format!("Client execution error: {:?}", e)))?
		.ok_or(Error::Environment("Parent header not found".to_string()))?;
	let parent_number = UniqueSaturatedInto::<u32>::unique_saturated_into(parent_number);

	let mut key_block = parent_number.saturating_sub(parent_number % PERIOD);

	// if we're before offset, stick with previous key block
	if parent_number % PERIOD < OFFSET {
		key_block = key_block.saturating_sub(PERIOD)
	};

	trace!("Using RandomX key block height: {}", key_block);

	let hash = client
		.hash(key_block.unique_saturated_into())
		.map_err(|e| Error::Environment(format!("Key hash lookup error: {:?}", e)))?
		.ok_or(Error::Environment("Key hash not found".to_string()))?;
	Ok(H256::from_slice(hash.as_ref()))
}

/// A stream that waits for a block import or timeout.
pub struct UntilImportedOrTimeout<Block: BlockT> {
	import_notifications: ImportNotifications<Block>,
	timeout: Duration,
	inner_delay: Option<Delay>,
}

impl<Block: BlockT> UntilImportedOrTimeout<Block> {
	/// Create a new stream using the given import notification and timeout duration.
	pub fn new(import_notifications: ImportNotifications<Block>, timeout: Duration) -> Self {
		Self { import_notifications, timeout, inner_delay: None }
	}
}

impl<Block: BlockT> Stream for UntilImportedOrTimeout<Block> {
	type Item = ();

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<()>> {
		let mut fire = false;

		loop {
			match Stream::poll_next(Pin::new(&mut self.import_notifications), cx) {
				Poll::Pending => break,
				Poll::Ready(Some(_)) => {
					fire = true;
				},
				Poll::Ready(None) => return Poll::Ready(None),
			}
		}

		let timeout = self.timeout;
		let inner_delay = self.inner_delay.get_or_insert_with(|| Delay::new(timeout));

		match Future::poll(Pin::new(inner_delay), cx) {
			Poll::Pending => (),
			Poll::Ready(()) => {
				fire = true;
			},
		}

		if fire {
			self.inner_delay = None;
			Poll::Ready(Some(()))
		} else {
			Poll::Pending
		}
	}
}
