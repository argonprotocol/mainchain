use std::{
	pin::Pin,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
	time::{Duration, Instant},
};

use codec::Encode;
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
use sp_blockchain::HeaderBackend;
use sp_consensus::{Environment, Proposal, Proposer, SelectChain, SyncOracle};
use sp_core::{blake2_256, crypto::AccountId32, RuntimeDebug, U256};
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

use ulx_primitives::{digests::SealSource, inherents::BlockSealInherent, *};

use crate::{
	authority::AuthorityClient,
	block_creator,
	block_creator::propose,
	convert_u32,
	error::Error,
	notebook_watch::{get_notary_state, has_applicable_tax_votes, notary_state_to_vote_digest},
};

#[derive(Clone, Eq, PartialEq, Encode)]
pub struct BlockComputeNonce {
	pub pre_hash: Vec<u8>,
	pub nonce: U256,
}

impl BlockComputeNonce {
	pub fn increment(&mut self) {
		self.nonce = self.nonce.checked_add(U256::one()).unwrap_or_default();
	}

	pub fn check_output_hash(&self, payload: &mut Vec<u8>) -> U256 {
		let nonce = self.nonce.encode();
		payload.splice(payload.len() - nonce.len().., nonce);
		let hash = blake2_256(&payload);
		U256::from_big_endian(&hash)
	}

	pub fn is_valid(&self, difficulty: ComputeDifficulty) -> bool {
		let hash = self.using_encoded(blake2_256);
		let threshold = U256::MAX / U256::from(difficulty).max(U256::one());
		U256::from_big_endian(&hash) <= threshold
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
			threshold: U256::MAX / U256::from(difficulty).max(U256::one()),
			wip_nonce_hash: vec![],
			wip_nonce: BlockComputeNonce { nonce: U256::from(rand::random::<u64>()), pre_hash },
		};
		solver.wip_nonce_hash = solver.wip_nonce.encode().to_vec();
		solver
	}

	/// Synchronous step to look at the next nonce
	pub fn check_next(&mut self) -> Option<BlockComputeNonce> {
		self.wip_nonce.increment();

		let hash = self.wip_nonce.check_output_hash(&mut self.wip_nonce_hash);
		if hash <= self.threshold {
			return Some(self.wip_nonce.clone())
		}
		None
	}
}

/// Version of the mining worker.
#[derive(Eq, PartialEq, Clone, Copy, RuntimeDebug)]
pub struct Version(pub usize);

/// Mining metadata. This is the information needed to start an actual mining loop.
#[derive(Clone, Eq, PartialEq)]
pub struct MiningMetadata<H> {
	/// Currently known best hash which the pre-hash is built on.
	pub best_hash: H,
	pub import_time: Instant,
	pub has_tax_votes: bool,
}

struct MiningBuild<Block: BlockT, Proof> {
	proposal: Proposal<Block, Proof>,
	/// The block seal authority id
	block_seal_authority_id: BlockSealAuthorityId,
	/// Mining pre-hash.
	pub pre_hash: Block::Hash,
	/// Pre-runtime digest item.
	pub difficulty: ComputeDifficulty,
}

/// Mining worker that exposes structs to query the current mining build and submit mined blocks.
pub struct MiningHandle<Block: BlockT, L: sc_consensus::JustificationSyncLink<Block>, Proof> {
	version: Arc<AtomicUsize>,
	justification_sync_link: Arc<L>,
	keystore: KeystorePtr,
	metadata: Arc<Mutex<Option<MiningMetadata<Block::Hash>>>>,
	build: Arc<Mutex<Option<MiningBuild<Block, Proof>>>>,
	block_import: Arc<Mutex<BoxBlockImport<Block>>>,
}

impl<Block, L, Proof> MiningHandle<Block, L, Proof>
where
	Block: BlockT,
	L: sc_consensus::JustificationSyncLink<Block>,
	Proof: Send,
{
	fn increment_version(&self) {
		self.version.fetch_add(1, Ordering::SeqCst);
	}

	pub fn new(
		block_import: BoxBlockImport<Block>,
		justification_sync_link: L,
		keystore: KeystorePtr,
	) -> Self {
		Self {
			version: Arc::new(AtomicUsize::new(0)),
			justification_sync_link: Arc::new(justification_sync_link),
			build: Arc::new(Mutex::new(None)),
			block_import: Arc::new(Mutex::new(block_import)),
			metadata: Arc::new(Mutex::new(None)),
			keystore,
		}
	}

	pub(crate) fn stop_solving_current(&self) {
		let mut build = self.build.lock();
		*build = None;
		let mut metadata = self.metadata.lock();
		*metadata = None;
		self.increment_version();
	}

	pub(crate) fn on_block(&self, best_hash: Block::Hash, has_tax_votes: bool) {
		self.stop_solving_current();
		let mut metadata = self.metadata.lock();
		*metadata = Some(MiningMetadata { best_hash, has_tax_votes, import_time: Instant::now() });
	}
	pub(crate) fn start_solving(
		&self,
		best_hash: Block::Hash,
		pre_hash: Block::Hash,
		difficulty: ComputeDifficulty,
		proposal: Proposal<Block, Proof>,
		block_seal_authority_id: BlockSealAuthorityId,
	) {
		if self.best_hash() != Some(best_hash) {
			self.stop_solving_current();
			return
		}

		let mut build = self.build.lock();
		*build = Some(MiningBuild {
			pre_hash: pre_hash.clone(),
			difficulty,
			proposal,
			block_seal_authority_id,
		});
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
	pub fn best_hash(&self) -> Option<Block::Hash> {
		self.metadata.lock().as_ref().map(|b| b.best_hash)
	}

	/// Get a copy of the current mining metadata, if available.
	pub fn metadata(&self) -> Option<MiningMetadata<Block::Hash>> {
		self.metadata.lock().as_ref().cloned()
	}

	pub fn create_solver(&self) -> Option<ComputeSolver> {
		match self.build.lock().as_ref() {
			Some(x) => {
				let pre_hash = x.pre_hash;

				Some(ComputeSolver::new(self.version(), pre_hash.as_ref().to_vec(), x.difficulty))
			},
			_ => None,
		}
	}

	pub fn is_valid_solver(&self, solver: &Option<Box<ComputeSolver>>) -> bool {
		solver.as_ref().map(|a| a.version) == Some(self.version())
	}

	pub fn ready_to_solve(&self, time_to_wait_for_tax_block: Duration) -> bool {
		match self.metadata.lock().as_ref() {
			Some(x) => {
				if !x.has_tax_votes {
					return true
				}
				x.import_time.elapsed() >= time_to_wait_for_tax_block
			},
			_ => false,
		}
	}

	pub async fn submit(&mut self, nonce: BlockComputeNonce) -> Result<(), Error<Block>> {
		let build = match {
			let mut build = self.build.lock();
			// try to take out of option. if not exists, we've moved on
			build.take()
		} {
			Some(x) => x,
			_ => {
				trace!(target: LOG_TARGET, "Unable to submit mined block in compute worker: internal build does not exist",);
				return Ok(())
			},
		};

		let mut block_import = self.block_import.lock();

		block_creator::submit_block::<Block, L, Proof>(
			&mut block_import,
			build.proposal,
			&self.justification_sync_link,
			&self.keystore,
			&nonce.nonce,
			SealSource::Compute,
			&build.block_seal_authority_id,
		)
		.await;
		Ok(())
	}
}

impl<Block, L, Proof> Clone for MiningHandle<Block, L, Proof>
where
	Block: BlockT,
	L: sc_consensus::JustificationSyncLink<Block>,
{
	fn clone(&self) -> Self {
		Self {
			version: self.version.clone(),
			justification_sync_link: self.justification_sync_link.clone(),
			build: self.build.clone(),
			metadata: self.metadata.clone(),
			block_import: self.block_import.clone(),
			keystore: self.keystore.clone(),
		}
	}
}

const LOG_TARGET: &str = "voter::compute::miner";
pub(crate) fn create_compute_solver_task<B, L, Proof>(
	mut worker: MiningHandle<B, L, Proof>,
) -> BoxFuture<'static, ()>
where
	B: BlockT,
	L: sc_consensus::JustificationSyncLink<B> + 'static,
	Proof: Send + 'static,
{
	async move {
		let mut solver: Option<Box<ComputeSolver>> = None;
		loop {
			if !worker.is_valid_solver(&solver) {
				solver = worker.create_solver().map(Box::new);
			}

			if let Some(solver) = solver.as_mut() {
				if let Some(nonce) = solver.check_next() {
					let _ = block_on(worker.submit(nonce));
				}
			} else {
				tokio::time::sleep(Duration::from_millis(50)).await;
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
		task_handle.spawn_essential_handle().spawn_blocking(
			"mining-voter",
			Some("block-authoring"),
			create_compute_solver_task(worker),
		);
	}
}

pub fn create_compute_miner<Block, C, S, E, SO, L>(
	block_import: BoxBlockImport<Block>,
	client: Arc<C>,
	select_chain: S,
	mut env: E,
	sync_oracle: SO,
	account_id: AccountId32,
	keystore: KeystorePtr,
	justification_sync_link: L,
	time_to_wait_for_tax_block: Duration,
	max_time_to_build_block: Duration,
) -> (
	MiningHandle<Block, L, <E::Proposer as Proposer<Block>>::Proof>,
	impl Future<Output = ()> + 'static,
)
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block>
		+ BlockchainEvents<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ 'static,
	C::Api: MiningAuthorityApis<Block> + BlockSealMinimumApis<Block>,
	S: SelectChain<Block> + 'static,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
	SO: SyncOracle + Clone + Send + Sync + 'static,
	L: sc_consensus::JustificationSyncLink<Block> + 'static,
{
	// create a timer that fires whenever there are new blocks, or 500 ms go by
	let mut timer = UntilImportedOrTimeout::new(
		client.import_notification_stream(),
		Duration::from_millis(1000),
	);
	let authority_client = AuthorityClient::new(client.clone(), keystore.clone());
	let mining_handle = MiningHandle::new(block_import, justification_sync_link, keystore);

	let handle_to_return = mining_handle.clone();

	let task = async move {
		loop {
			if timer.next().await.is_none() {
				// this should occur if the block import notifications completely stop... indicating
				// we should exit
				break
			}
			if sync_oracle.is_major_syncing() {
				debug!(target: LOG_TARGET, "Skipping proposal due to sync.");
				mining_handle.stop_solving_current();
				continue
			}

			let best_header = match select_chain.best_chain().await {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to pull new block for compute miner. Select best chain error: {}",
						err
					);
					continue
				},
			};
			let best_hash = best_header.hash();

			if mining_handle.best_hash() != Some(best_hash) {
				let has_tax_votes = has_applicable_tax_votes(&client, &best_header);
				mining_handle.on_block(best_hash, has_tax_votes);
			}

			if !mining_handle.ready_to_solve(time_to_wait_for_tax_block) {
				continue
			}

			let difficulty = match client.runtime_api().compute_difficulty(best_hash) {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to pull new block for compute miner. No difficulty found!! {}", err
					);
					continue
				},
			};
			let block_seal_authority_id = match authority_client.get_preferred_authority(&best_hash)
			{
				Some(x) => x,
				None => {
					warn!(
						target: LOG_TARGET,
						"No authority installed for compute block creation!!"
					);
					continue
				},
			};

			let block_number = convert_u32::<Block>(best_header.number());
			let notary_state = match get_notary_state::<Block, C>(&client, block_number) {
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to pull new block for compute miner. No notary state found!! {}", err
					);
					continue
				},
			};

			let latest_finalized_block_needed = notary_state.latest_finalized_block_needed;

			let proposal = match propose(
				client.clone(),
				&mut env,
				&account_id,
				notary_state_to_vote_digest(&notary_state),
				block_number,
				best_hash,
				BlockSealInherent::Compute,
				block_seal_authority_id.clone(),
				latest_finalized_block_needed,
				max_time_to_build_block,
			)
			.await
			{
				Ok(x) => x,
				Err(err) => {
					warn!(
						target: LOG_TARGET,
						"Unable to propose a new block {}", err
					);
					continue
				},
			};

			let pre_hash = proposal.block.header().hash();

			mining_handle.start_solving(
				best_hash,
				pre_hash,
				difficulty,
				proposal,
				block_seal_authority_id,
			);
		}
	};

	(handle_to_return, task)
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
