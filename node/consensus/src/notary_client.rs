use crate::{
	aux_client::ArgonAux, error::Error, metrics::ConsensusMetrics,
	state_anchor::DEFAULT_STATE_LOOKBACK_DEPTH,
};
use argon_notary_apis::{
	ArchiveHost, Client, DownloadKind, DownloadPolicy, DownloadTrustMode, SystemRpcClient,
	get_download_path_suffix, get_header_url, get_notebook_url,
	notebook::{NotebookRpcClient, RawHeadersSubscription},
};
use argon_primitives::{
	BlockSealApis, BlockSealAuthorityId, BlockVotingPower, NotaryApis, NotaryId, NotebookApis,
	NotebookAuditResult, NotebookHeaderData, TickApis, VoteMinimum, VotingSchedule, ensure,
	notary::{
		NotaryNotebookAuditSummary, NotaryNotebookDetails, NotaryNotebookRawVotes, NotaryState,
		NotebookBytes, SignedHeaderBytes,
	},
	notebook::NotebookNumber,
	prelude::sc_client_api::BlockBackend,
	tick::{Tick, Ticker},
};
use argon_runtime::{NotaryRecordT, NotebookVerifyError};
use codec::Codec;
use futures::{Stream, StreamExt, future::join_all, task::noop_waker_ref};
use log::{info, trace, warn};
use polkadot_sdk::*;
use sc_client_api::{AuxStore, BlockchainEvents};
use sc_service::TaskManager;
use sc_utils::mpsc::{TracingUnboundedReceiver, TracingUnboundedSender, tracing_unbounded};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::H256;
use sp_runtime::{
	DispatchError,
	traits::{Block as BlockT, Header},
};
use std::{
	collections::{BTreeMap, BTreeSet},
	default::Default,
	marker::PhantomData,
	ops::Range,
	pin::Pin,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	task::{Context, Poll},
	time::{Duration, Instant},
};
use substrate_prometheus_endpoint::Registry;
use tokio::{
	sync::{Mutex, RwLock, Semaphore},
	time,
};
use tracing::error;

const MAX_PARALLEL_NOTARY_DOWNLOADS: usize = 8;
const MAX_PARALLEL_NOTARY_AUDITS: usize = 4;
const HEADER_PREFETCH_WINDOW: usize = MAX_PARALLEL_NOTARY_DOWNLOADS;
const MAX_NOTEBOOK_SUBSCRIPTION_TICK_LAG: Tick = 2;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum NotebookAuditMode {
	Sync,
	Import { max_wait: Duration },
}

pub trait NotaryApisExt<B: BlockT, AC> {
	fn has_block_state(&self, block_hash: B::Hash) -> bool;
	fn notaries(&self, block_hash: B::Hash) -> Result<Vec<NotaryRecordT>, Error>;
	fn latest_notebook_by_notary(
		&self,
		block_hash: B::Hash,
	) -> Result<BTreeMap<NotaryId, (NotebookNumber, Tick)>, Error>;
	fn current_tick(&self, block_hash: B::Hash) -> Result<Tick, Error>;
	#[allow(clippy::too_many_arguments)]
	fn audit_notebook_and_get_votes(
		&self,
		block_hash: B::Hash,
		version: u32,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		notebook_tick: Tick,
		header_hash: H256,
		notebook: &[u8],
		notebook_dependencies: Vec<NotaryNotebookAuditSummary>,
		block_hashes: &[B::Hash],
	) -> Result<Result<NotaryNotebookRawVotes, NotebookVerifyError>, Error>;
	fn vote_minimum(&self, block_hash: B::Hash) -> Result<VoteMinimum, Error>;
	fn decode_signed_raw_notebook_header(
		&self,
		block_hash: &B::Hash,
		raw_header: Vec<u8>,
	) -> Result<Result<NotaryNotebookDetails<B::Hash>, DispatchError>, Error>;
	fn best_hash(&self) -> B::Hash;
	fn finalized_hash(&self) -> B::Hash;
	fn parent_hash(&self, hash: &B::Hash) -> Result<B::Hash, Error>;
}

impl<B, C, AC> NotaryApisExt<B, AC> for C
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + BlockBackend<B>,
	C::Api: NotaryApis<B, NotaryRecordT>
		+ NotebookApis<B, NotebookVerifyError>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ TickApis<B>,
	AC: Clone + Codec,
{
	fn has_block_state(&self, block_hash: B::Hash) -> bool {
		self.block_status(block_hash).unwrap_or(sp_consensus::BlockStatus::Unknown) ==
			sp_consensus::BlockStatus::InChainWithState
	}

	fn notaries(&self, block_hash: B::Hash) -> Result<Vec<NotaryRecordT>, Error> {
		self.runtime_api().notaries(block_hash).map_err(Into::into)
	}
	fn latest_notebook_by_notary(
		&self,
		block_hash: B::Hash,
	) -> Result<BTreeMap<NotaryId, (NotebookNumber, Tick)>, Error> {
		self.runtime_api().latest_notebook_by_notary(block_hash).map_err(Into::into)
	}
	fn current_tick(&self, block_hash: B::Hash) -> Result<Tick, Error> {
		self.runtime_api().current_tick(block_hash).map_err(Into::into)
	}
	fn audit_notebook_and_get_votes(
		&self,
		block_hash: B::Hash,
		version: u32,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		notebook_tick: Tick,
		header_hash: H256,
		notebook: &[u8],
		notebook_dependencies: Vec<NotaryNotebookAuditSummary>,
		_block_hashes: &[B::Hash],
	) -> Result<Result<NotaryNotebookRawVotes, NotebookVerifyError>, Error> {
		self.runtime_api()
			.audit_notebook_and_get_votes_v2(
				block_hash,
				version,
				notary_id,
				notebook_number,
				notebook_tick,
				header_hash,
				&notebook.to_vec(),
				notebook_dependencies,
			)
			.map_err(Into::into)
	}
	fn vote_minimum(&self, block_hash: B::Hash) -> Result<VoteMinimum, Error> {
		self.runtime_api().vote_minimum(block_hash).map_err(Into::into)
	}
	fn decode_signed_raw_notebook_header(
		&self,
		block_hash: &B::Hash,
		raw_header: Vec<u8>,
	) -> Result<Result<NotaryNotebookDetails<B::Hash>, DispatchError>, Error> {
		self.runtime_api()
			.decode_signed_raw_notebook_header(*block_hash, raw_header)
			.map_err(Into::into)
	}
	fn best_hash(&self) -> B::Hash {
		self.info().best_hash
	}
	fn finalized_hash(&self) -> B::Hash {
		self.info().finalized_hash
	}
	fn parent_hash(&self, hash: &B::Hash) -> Result<B::Hash, Error> {
		let header = self.header(*hash)?.ok_or_else(|| {
			Error::BlockNotFound(format!("Unable to find parent block: {hash:?}"))
		})?;
		Ok(*header.parent_hash())
	}
}

#[allow(clippy::too_many_arguments)]
pub fn run_notary_sync<B, C, AC>(
	task_manager: &TaskManager,
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	no_work_delay_millis: u64,
	notebook_downloader: NotebookDownloader,
	registry: Option<&Registry>,
	ticker: Ticker,
	is_solving_blocks: bool,
) -> Arc<NotaryClient<B, C, AC>>
where
	B: BlockT,
	C: ProvideRuntimeApi<B>
		+ BlockchainEvents<B>
		+ BlockBackend<B>
		+ HeaderBackend<B>
		+ AuxStore
		+ Send
		+ Sync
		+ 'static,
	C::Api: NotebookApis<B, NotebookVerifyError>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ NotaryApis<B, NotaryRecordT>
		+ TickApis<B>,
	AC: Codec + Clone + Send + Sync + 'static,
{
	let metrics = registry.and_then(|r| ConsensusMetrics::new(r, client.clone()).ok());
	let metrics = Arc::new(metrics);

	let notary_client = Arc::new(NotaryClient::new(
		client.clone(),
		aux_client.clone(),
		notebook_downloader,
		metrics,
		ticker,
		Some(task_manager.spawn_handle()),
		Duration::from_millis(no_work_delay_millis),
		is_solving_blocks,
	));

	let notary_client_poll = Arc::clone(&notary_client);
	let best_block = client.best_hash();
	let notary_sync_task = async move {
		let idle_delay = if ticker.tick_duration_millis <= 10_000 { 100 } else { 1000 };
		let idle_delay = Duration::from_millis(idle_delay);
		let initial_notary_hash =
			match resolve_client_stateful_hash::<B, _, AC>(client.as_ref(), best_block) {
				Ok(hash) => hash,
				Err(err) => {
					warn!("Could not resolve a stateful hash for initial notary update - {err:?}");
					None
				},
			};
		if let Some(initial_notary_hash) = initial_notary_hash {
			notary_client_poll
				.update_notaries(&initial_notary_hash)
				.await
				.unwrap_or_else(|e| {
					warn!("Could not update notaries at startup hash {initial_notary_hash} - {e:?}")
				});
		} else {
			trace!("Skipping initial notary update because no stateful hash is available yet");
		}

		let mut best_block = Box::pin(client.every_import_notification_stream());
		let mut health_tick = time::interval(
			Duration::from_millis(ticker.tick_duration_millis / 3).max(Duration::from_secs(5)),
		);
		health_tick.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

		loop {
			tokio::select! {
				Some(ref block) = best_block.next() => {
					if block.is_new_best {
						let best_hash = block.hash;
						let Some(best_hash) =
							resolve_client_stateful_hash::<B, _, AC>(client.as_ref(), best_hash)
								.unwrap_or_else(|err| {
									warn!(
										"Could not resolve a stateful hash for best-block notary update - {err:?}"
									);
									None
								})
						else {
							continue;
						};
						if let Err(e) = notary_client_poll.update_notaries(&best_hash).await {
							warn!(

								"Could not update notaries at best hash {best_hash} - {e:?}"
							);
						}
					}
				},
				_ = health_tick.tick() => {
					let best_hash = client.best_hash();
					if let Some(best_hash) =
						resolve_client_stateful_hash::<B, _, AC>(client.as_ref(), best_hash)
							.unwrap_or_else(|err| {
								warn!(
									"Could not resolve a stateful hash for periodic notary update - {err:?}"
								);
								None
							})
					{
						let _ = notary_client_poll.update_notaries(&best_hash).await;
					}
				},
				// Prevent thrashing the polling when nothing is returned
				_yield = time::sleep(idle_delay) => {},
			}
		}
	};
	let handle = task_manager.spawn_essential_handle();
	handle.spawn("notary_sync_task", "notary_sync", notary_sync_task);

	notary_client
}

type WorkerHandle = Arc<NotaryWorker>;
type WorkersById = Arc<RwLock<BTreeMap<NotaryId, WorkerHandle>>>;
type ProcessingHashes<Hash> = (Hash, Hash);

type NotebookCount = u32;
pub type VotingPowerInfo = (Tick, BlockVotingPower, NotebookCount);

struct MissingAuditCatchup {
	by_notary: BTreeMap<NotaryId, Vec<NotebookNumber>>,
	needs_notary_updates: bool,
}

struct NotebookAuditTarget {
	notebook_number: NotebookNumber,
	header_bytes: Option<SignedHeaderBytes>,
	known_since: Instant,
}

struct NotebookAuditSelection {
	target: Option<NotebookAuditTarget>,
	finalized_notebooks_trimmed: u32,
	tracked_range: Range<NotebookNumber>,
}

#[derive(Default)]
struct NotebookAuditState {
	next_notebook_number: Option<NotebookNumber>,
	highest_notebook_number: Option<NotebookNumber>,
	known_since_by_notebook: BTreeMap<NotebookNumber, Instant>,
	cached_headers: BTreeMap<NotebookNumber, SignedHeaderBytes>,
}

impl NotebookAuditState {
	fn track_notebook(
		&mut self,
		notebook_number: NotebookNumber,
		header_bytes: Option<SignedHeaderBytes>,
		known_since: Option<Instant>,
	) {
		let known_since = known_since.unwrap_or_else(Instant::now);
		match self.bounds() {
			Some((next_notebook_number, highest_notebook_number)) => {
				if notebook_number < next_notebook_number {
					self.record_known_range(notebook_number, next_notebook_number - 1, known_since);
					self.next_notebook_number = Some(notebook_number);
				}

				if notebook_number > highest_notebook_number {
					self.record_known_range(
						highest_notebook_number + 1,
						notebook_number,
						known_since,
					);
					self.highest_notebook_number = Some(notebook_number);
				}
			},
			None => self.initialize(notebook_number, notebook_number, known_since),
		}

		if let Some(header_bytes) = header_bytes {
			self.cached_headers.entry(notebook_number).or_insert(header_bytes);
		}
	}

	fn track_range(
		&mut self,
		first_notebook_number: NotebookNumber,
		last_notebook_number: NotebookNumber,
	) {
		let known_since = Instant::now();
		if first_notebook_number > last_notebook_number {
			return;
		}

		match self.bounds() {
			Some((next_notebook_number, highest_notebook_number)) => {
				if first_notebook_number < next_notebook_number {
					self.record_known_range(
						first_notebook_number,
						next_notebook_number - 1,
						known_since,
					);
					self.next_notebook_number = Some(first_notebook_number);
				}
				if last_notebook_number > highest_notebook_number {
					self.record_known_range(
						highest_notebook_number + 1,
						last_notebook_number,
						known_since,
					);
					self.highest_notebook_number = Some(last_notebook_number);
				}
			},
			None => self.initialize(first_notebook_number, last_notebook_number, known_since),
		}
	}

	fn clear(&mut self) {
		self.next_notebook_number = None;
		self.highest_notebook_number = None;
		self.known_since_by_notebook.clear();
		self.cached_headers.clear();
	}

	fn len(&self) -> usize {
		match self.bounds() {
			Some((next, highest)) if highest >= next => (highest - next + 1) as usize,
			_ => 0,
		}
	}

	fn range(&self) -> Range<NotebookNumber> {
		match self.bounds() {
			Some((start, end)) => Range { start, end },
			_ => 0..0,
		}
	}

	#[cfg(test)]
	fn snapshot(&self) -> Vec<(NotebookNumber, bool)> {
		let Some((next, highest)) = self.bounds() else {
			return Vec::new();
		};

		(next..=highest)
			.map(|number| {
				let has_header = self.cached_headers.contains_key(&number);
				(number, has_header)
			})
			.collect()
	}

	fn rewind_to(&mut self, notebook_number: NotebookNumber) {
		let known_since = Instant::now();
		match self.bounds() {
			Some((next_notebook_number, highest_notebook_number)) => {
				if notebook_number < next_notebook_number {
					self.record_known_range(notebook_number, next_notebook_number - 1, known_since);
					self.next_notebook_number = Some(notebook_number);
				} else if notebook_number > highest_notebook_number {
					self.record_known_range(
						highest_notebook_number + 1,
						notebook_number,
						known_since,
					);
					self.highest_notebook_number = Some(notebook_number);
				}
			},
			None => self.initialize(notebook_number, notebook_number, known_since),
		}
		self.cached_headers.remove(&notebook_number);
	}

	fn prefetch_targets(&self, window: usize) -> Vec<NotebookNumber> {
		let Some((next, highest)) = self.bounds() else {
			return Vec::new();
		};
		if highest <= next {
			return Vec::new();
		}

		let last = highest.min(next.saturating_add(window as NotebookNumber));
		((next + 1)..=last)
			.filter(|number| !self.cached_headers.contains_key(number))
			.collect()
	}

	fn cache_header(&mut self, notebook_number: NotebookNumber, header_bytes: SignedHeaderBytes) {
		let Some((next, highest)) = self.bounds() else {
			return;
		};
		if notebook_number < next || notebook_number > highest {
			return;
		}

		self.cached_headers.entry(notebook_number).or_insert(header_bytes);
	}

	fn select_next_audit(
		&mut self,
		finalized_notebook_number: NotebookNumber,
	) -> NotebookAuditSelection {
		let tracked_range = self.range();
		let Some((mut next, highest)) = self.bounds() else {
			return NotebookAuditSelection {
				target: None,
				finalized_notebooks_trimmed: 0,
				tracked_range,
			};
		};

		let mut finalized_notebooks_trimmed = 0;
		if next <= finalized_notebook_number {
			let new_next = finalized_notebook_number.saturating_add(1);
			if new_next > highest {
				finalized_notebooks_trimmed = highest - next + 1;
				self.clear();
				return NotebookAuditSelection {
					target: None,
					finalized_notebooks_trimmed,
					tracked_range,
				};
			}

			finalized_notebooks_trimmed = new_next - next;
			next = new_next;
			self.advance_to(new_next);
		}

		NotebookAuditSelection {
			target: Some(NotebookAuditTarget {
				notebook_number: next,
				header_bytes: self.cached_headers.get(&next).cloned(),
				known_since: self.known_since(next),
			}),
			finalized_notebooks_trimmed,
			tracked_range,
		}
	}

	fn bounds(&self) -> Option<(NotebookNumber, NotebookNumber)> {
		Some((self.next_notebook_number?, self.highest_notebook_number?))
	}

	fn initialize(
		&mut self,
		first_notebook_number: NotebookNumber,
		last_notebook_number: NotebookNumber,
		known_since: Instant,
	) {
		self.next_notebook_number = Some(first_notebook_number);
		self.highest_notebook_number = Some(last_notebook_number);
		self.known_since_by_notebook.clear();
		self.known_since_by_notebook.insert(first_notebook_number, known_since);
	}

	fn mark_processed(&mut self, notebook_number: NotebookNumber) {
		if self.next_notebook_number != Some(notebook_number) {
			return;
		}

		let Some(highest_notebook_number) = self.highest_notebook_number else {
			return;
		};
		self.cached_headers.remove(&notebook_number);
		if notebook_number >= highest_notebook_number {
			self.clear();
			return;
		}
		self.advance_to(notebook_number + 1);
	}

	fn advance_to(&mut self, next_notebook_number: NotebookNumber) {
		let Some(highest_notebook_number) = self.highest_notebook_number else {
			return;
		};
		if next_notebook_number > highest_notebook_number {
			self.clear();
			return;
		}

		let known_since = self.known_since(next_notebook_number);
		self.next_notebook_number = Some(next_notebook_number);
		self.cached_headers.retain(|number, _| *number >= next_notebook_number);
		self.known_since_by_notebook.retain(|number, _| *number >= next_notebook_number);
		self.known_since_by_notebook.insert(next_notebook_number, known_since);
	}

	fn record_known_range(
		&mut self,
		first_notebook_number: NotebookNumber,
		last_notebook_number: NotebookNumber,
		known_since: Instant,
	) {
		if first_notebook_number > last_notebook_number {
			return;
		}
		self.known_since_by_notebook.entry(first_notebook_number).or_insert(known_since);
	}

	fn known_since(&self, notebook_number: NotebookNumber) -> Instant {
		self.known_since_by_notebook
			.range(..=notebook_number)
			.next_back()
			.map(|(_, known_since)| *known_since)
			.unwrap_or_else(Instant::now)
	}
}

struct WorkerContext<B: BlockT, C: AuxStore, AC> {
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	notebook_downloader: NotebookDownloader,
	metrics: Arc<Option<ConsensusMetrics<C>>>,
	pause_notebook_audits: Arc<RwLock<bool>>,
	ticker: Ticker,
	tick_voting_power_sender: Arc<Mutex<TracingUnboundedSender<VotingPowerInfo>>>,
	download_slots: Arc<Semaphore>,
	audit_slots: Arc<Semaphore>,
	is_solving_blocks: bool,
	_phantom: PhantomData<AC>,
}

/// Owns the per-notary connection state and tracked notebook audits so stalls stay local.
struct NotaryWorker {
	notary_id: NotaryId,
	record: RwLock<Option<NotaryRecordT>>,
	client: RwLock<Option<Arc<Client>>>,
	archive_host: RwLock<Option<String>>,
	subscription: Mutex<Option<Pin<Box<RawHeadersSubscription>>>>,
	pending: Mutex<NotebookAuditState>,
	last_notebook_tick: RwLock<Option<Tick>>,
	last_connection_attempt_tick: RwLock<Option<Tick>>,
	connection_lock: Arc<Mutex<()>>,
	processing_lock: Arc<Mutex<()>>,
	background_task_started: AtomicBool,
}

impl NotaryWorker {
	fn new(notary_id: NotaryId) -> Self {
		Self {
			notary_id,
			record: Default::default(),
			client: Default::default(),
			archive_host: Default::default(),
			subscription: Default::default(),
			pending: Default::default(),
			last_notebook_tick: Default::default(),
			last_connection_attempt_tick: Default::default(),
			connection_lock: Arc::new(Mutex::new(())),
			processing_lock: Arc::new(Mutex::new(())),
			background_task_started: AtomicBool::new(false),
		}
	}

	async fn update_record(&self, record: NotaryRecordT) -> bool {
		let mut current = self.record.write().await;
		let previous_host = current.as_ref().and_then(|record| record.meta.hosts.first()).cloned();
		let next_host = record.meta.hosts.first().cloned();
		let host_changed = previous_host.is_some() && previous_host != next_host;
		*current = Some(record);
		host_changed
	}

	async fn clear_record(&self) {
		self.record.write().await.take();
	}

	async fn host(&self) -> Result<String, Error> {
		let record = self.record.read().await;
		let record = record
			.as_ref()
			.ok_or_else(|| Error::NotaryError("No rpc endpoints found for notary".to_string()))?;
		let host =
			record.meta.hosts.first().ok_or_else(|| {
				Error::NotaryError("No rpc endpoint found for notary".to_string())
			})?;
		host.clone().try_into().map_err(|e| {
			Error::NotaryError(format!(
				"Could not convert host to string for notary {} - {e:?}",
				self.notary_id
			))
		})
	}

	async fn prefetch_headers<B, C, AC>(&self, worker_context: &WorkerContext<B, C, AC>) -> bool
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		let targets = self.pending.lock().await.prefetch_targets(HEADER_PREFETCH_WINDOW);
		let mut prefetched = false;
		for notebook_number in targets {
			match self.download_header(worker_context, notebook_number, None).await {
				Ok(header_bytes) => {
					self.pending.lock().await.cache_header(notebook_number, header_bytes);
					prefetched = true;
				},
				Err(error) => {
					trace!(
						"Unable to prefetch notebook header for notary {} notebook {} - {error:?}",
						self.notary_id, notebook_number,
					);
				},
			}
		}
		prefetched
	}

	async fn clear_subscription(&self) {
		self.subscription.lock().await.take();
	}

	async fn clear_connection(&self) {
		let _connection_guard = self.connection_lock.lock().await;
		self.client.write().await.take();
		self.clear_subscription().await;
		self.last_notebook_tick.write().await.take();
	}

	async fn has_client(&self) -> bool {
		self.client.read().await.is_some()
	}

	async fn has_subscription(&self) -> bool {
		self.subscription.lock().await.is_some()
	}

	async fn ensure_connected_subscription(&self, current_tick: Tick) -> Result<(), Error> {
		let _connection_guard = self.connection_lock.lock().await;
		let can_connect = {
			let record = self.record.read().await;
			let Some(record) = record.as_ref() else {
				return Ok(());
			};
			!matches!(record.state, NotaryState::Locked { .. })
		};
		if !can_connect {
			return Ok(());
		}

		let notary_id = self.notary_id;
		let needs_client = !self.has_client().await;
		let needs_subscription = !self.has_subscription().await;
		if !needs_client && !needs_subscription {
			return Ok(());
		}

		{
			let mut last_connection_attempt_tick = self.last_connection_attempt_tick.write().await;
			if *last_connection_attempt_tick == Some(current_tick) {
				trace!(
					"Skipping notary connection attempt. notary_id={notary_id} current_tick={current_tick}"
				);
				return Ok(());
			}
			*last_connection_attempt_tick = Some(current_tick);
		}

		if needs_client {
			info!("Connecting to notary id={notary_id}");
			self.connect().await?;
		}

		if needs_subscription {
			self.subscribe().await?;
		}

		Ok(())
	}

	async fn disconnect_if_subscription_stale(&self, current_tick: Tick) {
		if !self.has_subscription().await {
			return;
		}

		let Some(last_notebook_tick) = *self.last_notebook_tick.read().await else {
			return;
		};

		let tick_lag = current_tick.saturating_sub(last_notebook_tick);
		if tick_lag <= MAX_NOTEBOOK_SUBSCRIPTION_TICK_LAG {
			return;
		}

		warn!(
			"Disconnecting stale notary subscription. notary_id={} last_notebook_tick={last_notebook_tick} current_tick={current_tick} tick_lag={tick_lag}",
			self.notary_id,
		);
		self.clear_connection().await;
	}

	async fn poll_subscription<B, C, AC>(
		&self,
		worker_context: &WorkerContext<B, C, AC>,
	) -> Option<NotebookNumber>
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		let next = {
			let mut subscription = self.subscription.lock().await;
			let subscription = subscription.as_mut()?;
			subscription.as_mut().poll_next(&mut Context::from_waker(noop_waker_ref()))
		};

		match next {
			Poll::Ready(Some(Ok(download_info))) => {
				*self.last_notebook_tick.write().await = Some(download_info.tick);
				if let Some(metrics) = worker_context.metrics.as_ref() {
					metrics.notebook_notification_received(
						self.notary_id,
						download_info.tick,
						&worker_context.ticker,
					);
				}
				trace!(
					"Tracking notebook {} for notary {}",
					download_info.notebook_number, self.notary_id
				);
				self.pending
					.lock()
					.await
					.track_notebook(download_info.notebook_number, None, None);
				Some(download_info.notebook_number)
			},
			Poll::Ready(Some(Err(error))) => {
				self.clear_connection().await;
				info!(
					"Notary client disconnected from notary #{} (or could not connect). Reason? {:?}",
					self.notary_id,
					Some(error.to_string())
				);
				None
			},
			Poll::Ready(None) => {
				self.clear_connection().await;
				info!(
					"Notary client disconnected from notary #{} (or could not connect). Reason? {:?}",
					self.notary_id, None::<String>
				);
				None
			},
			Poll::Pending => None,
		}
	}

	async fn run_background_pass<B, C, AC>(
		self: &Arc<Self>,
		worker_context: &WorkerContext<B, C, AC>,
	) -> Result<bool, Error>
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		if *worker_context.pause_notebook_audits.read().await {
			self.clear_subscription().await;
			return Ok(false);
		}
		let current_tick = if let Some(block_hash) = resolve_client_stateful_hash::<B, _, AC>(
			worker_context.client.as_ref(),
			worker_context.client.best_hash(),
		)? {
			Some(worker_context.client.current_tick(block_hash)?)
		} else {
			None
		};
		if let Some(current_tick) = current_tick {
			self.disconnect_if_subscription_stale(current_tick).await;
			self.ensure_connected_subscription(current_tick).await?;
		} else if !self.has_client().await || !self.has_subscription().await {
			return Ok(false);
		}
		let saw_subscription = self.poll_subscription(worker_context).await.is_some();
		let has_more_work = self.process_background_work(worker_context).await;
		Ok(saw_subscription || has_more_work)
	}

	fn start_background_task<B, C, AC>(
		self: &Arc<Self>,
		worker_context: Arc<WorkerContext<B, C, AC>>,
		spawn_handle: Option<sc_service::SpawnTaskHandle>,
		background_idle_delay: Duration,
	) where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		if self.background_task_started.swap(true, Ordering::AcqRel) {
			return;
		}
		let Some(spawn_handle) = spawn_handle else {
			self.background_task_started.store(false, Ordering::Release);
			return;
		};
		let worker = Arc::clone(self);
		let idle_delay = background_idle_delay;

		spawn_handle.spawn("notary_worker_task", "notary_worker", async move {
			let mut delay = Duration::ZERO;
			loop {
				time::sleep(delay).await;
				delay = match worker.run_background_pass(worker_context.as_ref()).await {
					Ok(true) => Duration::from_millis(20),
					Ok(false) => idle_delay,
					Err(error) => {
						warn!(
							"Error driving notary {} in background task: {error:?}",
							worker.notary_id,
						);
						worker.clear_connection().await;
						idle_delay
					},
				};
			}
		});
	}

	async fn connect(&self) -> Result<(), Error> {
		let client = self.get_or_connect_client().await?;
		let notebook_meta = client.metadata().await.map_err(|error| {
			Error::NotaryError(format!("Could not get metadata from notary - {error:?}"))
		})?;
		let archive_host = client.get_archive_base_url().await.map_err(|error| {
			Error::NotaryError(format!("Could not get archive host from notary - {error:?}"))
		})?;
		*self.archive_host.write().await = Some(archive_host);
		if notebook_meta.last_closed_notebook_number > 0 {
			*self.last_notebook_tick.write().await = Some(notebook_meta.last_closed_notebook_tick);
			trace!(
				"Tracking latest notebook {} for notary {}",
				notebook_meta.last_closed_notebook_number, self.notary_id
			);
			self.pending.lock().await.track_notebook(
				notebook_meta.last_closed_notebook_number,
				None,
				None,
			);
		}
		Ok(())
	}

	async fn subscribe(&self) -> Result<(), Error> {
		let client = self.get_or_connect_client().await?;
		let stream: RawHeadersSubscription = client.subscribe_headers().await.map_err(|error| {
			Error::NotaryError(format!("Could not subscribe to notebooks from notary - {error:?}"))
		})?;
		self.subscription.lock().await.replace(Box::pin(stream));
		info!("Subscribed to notary id={}", self.notary_id);
		Ok(())
	}

	async fn next_for_processing<B, C, AC>(
		&self,
		worker_context: &WorkerContext<B, C, AC>,
		finalized_notebook_number: NotebookNumber,
	) -> Option<(NotebookNumber, SignedHeaderBytes, Instant)>
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		let NotebookAuditSelection { target, finalized_notebooks_trimmed, tracked_range } =
			self.pending.lock().await.select_next_audit(finalized_notebook_number);

		if tracing::enabled!(tracing::Level::TRACE) {
			tracing::trace!(
				?finalized_notebooks_trimmed,
				notary_id = self.notary_id,
				"Selecting notebook audit target for notary. Range: {:?}",
				tracked_range,
			);
		}

		let NotebookAuditTarget { notebook_number, mut header_bytes, known_since } = target?;
		if header_bytes.is_none() {
			header_bytes = self.download_header(worker_context, notebook_number, None).await.ok();
			if let Some(header_bytes) = header_bytes.as_ref() {
				self.pending.lock().await.cache_header(notebook_number, header_bytes.clone());
			}
		}
		if let Some(header_bytes) = header_bytes {
			return Some((notebook_number, header_bytes, known_since));
		}
		None
	}

	async fn process_for_hashes<B, C, AC>(
		self: &Arc<Self>,
		worker_context: &WorkerContext<B, C, AC>,
		best_hash: B::Hash,
		finalized_hash: B::Hash,
	) -> Result<bool, Error>
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		if self.pending.lock().await.len() == 0 {
			return Ok(false);
		}
		let Some(_processing_guard) = self.processing_lock.clone().try_lock_owned().ok() else {
			return Ok(true);
		};
		let finalized_notebook_number =
			Self::latest_notebook_in_runtime(worker_context, finalized_hash, self.notary_id);
		let Some((notebook_number, raw_header, known_since)) =
			self.next_for_processing(worker_context, finalized_notebook_number).await
		else {
			return Ok(false);
		};

		let has_more_work = match self
			.process_notebook(
				worker_context,
				notebook_number,
				finalized_notebook_number,
				&best_hash,
				raw_header.clone(),
				known_since,
			)
			.await
		{
			Ok(()) => {
				self.pending.lock().await.mark_processed(notebook_number);
				let has_more_work = self.pending.lock().await.len() > 0;
				trace!(
					"Processed notebook {notebook_number} for notary {}. More work? {has_more_work}",
					self.notary_id,
				);
				has_more_work
			},
			Err(error) => {
				if matches!(
					error,
					Error::MissingNotebooksError(_) |
						Error::NotebookAuditBeforeTick(_) |
						Error::NotaryAuditDeferred(_)
				) {
					trace!("Will retry notebook audit for notary {} - {error:?}", self.notary_id,);
					tokio::time::sleep(Duration::from_secs(1)).await;
					true
				} else {
					self.pending.lock().await.mark_processed(notebook_number);
					return Err(error);
				}
			},
		};
		let prefetched = self.prefetch_headers(worker_context).await;
		Ok(has_more_work || prefetched)
	}

	async fn process_background_work<B, C, AC>(
		self: &Arc<Self>,
		worker_context: &WorkerContext<B, C, AC>,
	) -> bool
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		if *worker_context.pause_notebook_audits.read().await {
			return false;
		}
		if self.pending.lock().await.len() == 0 {
			return false;
		}
		let Some(best_hash) = resolve_client_stateful_hash::<B, _, AC>(
			worker_context.client.as_ref(),
			worker_context.client.best_hash(),
		)
		.unwrap_or_else(|error| {
			warn!(
				"Could not resolve stateful best hash for notary {} background processing - {error:?}",
				self.notary_id,
			);
			None
		}) else {
			return false;
		};
		let Some(finalized_hash) = resolve_client_stateful_hash::<B, _, AC>(
			worker_context.client.as_ref(),
			worker_context.client.finalized_hash(),
		)
		.unwrap_or_else(|error| {
			warn!(
				"Could not resolve stateful finalized hash for notary {} background processing - {error:?}",
				self.notary_id,
			);
			None
		}) else {
			return false;
		};
		match self.process_for_hashes(worker_context, best_hash, finalized_hash).await {
			Ok(has_more_work) => has_more_work,
			Err(error) => {
				warn!(
					"Error processing notebooks for notary {} in background task: {error:?}",
					self.notary_id,
				);
				self.pending.lock().await.len() > 0
			},
		}
	}

	async fn process_notebook<B, C, AC>(
		&self,
		worker_context: &WorkerContext<B, C, AC>,
		notebook_number: NotebookNumber,
		finalized_notebook_number: NotebookNumber,
		best_hash: &B::Hash,
		raw_header: SignedHeaderBytes,
		known_since: Instant,
	) -> Result<(), Error>
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		let notary_id = self.notary_id;
		let mut best_hash = *best_hash;
		if notebook_number <= finalized_notebook_number {
			tracing::info!(
				notary_id,
				notebook_number,
				finalized_notebook_number,
				"Skipping audit of finalized notebook.",
			);
			return Ok(());
		}

		let mut latest_notebook_in_runtime =
			Self::latest_notebook_in_runtime(worker_context, best_hash, notary_id);
		if latest_notebook_in_runtime >= notebook_number {
			let mut counter = 0;
			while latest_notebook_in_runtime >= notebook_number {
				counter += 1;
				if counter >= 500 {
					return Err(Error::NotaryError(format!(
						"Could not find place to audit this notebook {notebook_number} in runtime"
					)));
				}

				tracing::trace!(
					notary_id,
					notebook_number,
					latest_notebook_in_runtime,
					trying_block_hash = ?best_hash,
					"Checking if we can audit at parent block",
				);
				let parent_hash = worker_context.client.parent_hash(&best_hash)?;
				if parent_hash == best_hash {
					return Err(Error::NotaryAuditDeferred(format!(
						"Missing state while locating audit parent. Notary={notary_id}, notebook={notebook_number}, block={best_hash:?}"
					)));
				}
				if !worker_context.client.has_block_state(parent_hash) {
					return Err(Error::NotaryAuditDeferred(format!(
						"Missing state while locating audit parent. Notary={notary_id}, notebook={notebook_number}, block={parent_hash:?}"
					)));
				}
				best_hash = parent_hash;
				latest_notebook_in_runtime =
					Self::latest_notebook_in_runtime(worker_context, best_hash, notary_id);
			}
			tracing::info!(
				notary_id,
				notebook_number,
				latest_notebook_in_runtime,
				at_block_hash = ?best_hash,
				"Will audit notebook at block.",
			);
		}

		let _audit_slot = worker_context.audit_slots.acquire().await.map_err(|error| {
			Error::NotaryError(format!(
				"Could not acquire audit processing slot for notary {notary_id} - {error:?}",
			))
		})?;

		let notebook_details = worker_context
			.client
			.decode_signed_raw_notebook_header(&best_hash, raw_header.0.clone())?
			.map_err(|error| {
				Error::NotaryError(format!(
					"Unable to decode notebook header in runtime. Notary={notary_id}, notebook={notebook_number} -> {error:?}",
				))
			})?;

		let tick = notebook_details.tick;
		ensure!(
			notary_id == notebook_details.notary_id,
			Error::NotaryError("Notary ID mismatch".to_string())
		);
		ensure!(
			notebook_number == notebook_details.notebook_number,
			Error::NotaryError("Notebook number mismatch".to_string())
		);

		let audit_result =
			self.audit_notebook(worker_context, &best_hash, &notebook_details).await?;
		let runtime_tick = worker_context.client.current_tick(best_hash)?;
		let voting_power = worker_context.aux_client.store_notebook_result(
			audit_result,
			raw_header,
			notebook_details,
			finalized_notebook_number,
			runtime_tick,
		)?;

		if let Some(metrics) = worker_context.metrics.as_ref() {
			metrics.notebook_processed(notary_id, tick, known_since, &worker_context.ticker);
		}

		if worker_context.is_solving_blocks {
			worker_context
				.tick_voting_power_sender
				.lock()
				.await
				.unbounded_send(voting_power)
				.map_err(|error| {
					Error::NotaryError(format!(
						"Could not send tick state to sender (notary {notary_id}, notebook {notebook_number}) - {error:?}",
					))
				})?;
		}
		Ok(())
	}

	async fn audit_notebook<B, C, AC>(
		&self,
		worker_context: &WorkerContext<B, C, AC>,
		best_hash: &B::Hash,
		notebook_details: &NotaryNotebookDetails<B::Hash>,
	) -> Result<NotebookAuditResult<NotebookVerifyError>, Error>
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		let notary_id = self.notary_id;
		let tick = notebook_details.tick;
		let notebook_number = notebook_details.notebook_number;
		let notebook_dependencies = self
			.get_notebook_dependencies(worker_context, notebook_number, best_hash)
			.await?;
		tracing::trace!(
			notary_id,
			notebook_number,
			best_hash = ?best_hash,
			tick,
			notebook_dependencies = notebook_dependencies.len(),
			"Attempting to audit notebook",
		);

		let full_notebook = self.download_notebook(worker_context, notebook_number).await?;
		tracing::trace!(
			notary_id,
			notebook_number,
			bytes = full_notebook.0.len(),
			"Notebook downloaded.",
		);

		let audit_failure_reason = match worker_context.client.audit_notebook_and_get_votes(
			*best_hash,
			notebook_details.version,
			notary_id,
			notebook_number,
			tick,
			notebook_details.header_hash,
			&full_notebook.0,
			notebook_dependencies.clone(),
			&notebook_details.blocks_with_votes,
		)? {
			Ok(votes) => {
				let vote_count = votes.raw_votes.len();
				worker_context.aux_client.store_votes(tick, votes)?;
				tracing::info!(
					notary_id,
					notebook_number,
					tick,
					"Notebook audit successful. {vote_count} block vote(s).",
				);
				None
			},
			Err(error) => {
				if error == NotebookVerifyError::CatchupNotebooksMissing {
					tracing::warn!(
						notary_id,
						notebook_number,
						?notebook_dependencies,
						?best_hash,
						"Notebook audit failed for notary. Incorrect catchup provided",
					);
					return Err(Error::MissingNotebooksError(format!(
						"Possibly missing notebooks? Invalid catchup notebooks provided to audit. Notary {notary_id}, #{notebook_number}, tick {tick}.",
					)));
				}

				if tick > worker_context.client.current_tick(*best_hash)? {
					return Err(Error::NotebookAuditBeforeTick(format!(
						"Notebook tick is > runtime. Notary={notary_id}, notebook={notebook_number}, tick={tick}",
					)));
				}

				tracing::warn!(notary_id, notebook_number, tick, "Notebook audit failed ({error})",);
				Some(error)
			},
		};

		Ok(NotebookAuditResult {
			notary_id,
			tick,
			notebook_number,
			audit_first_failure: audit_failure_reason,
		})
	}

	async fn get_or_connect_client(&self) -> Result<Arc<Client>, Error> {
		if let Some(client) = self.client.read().await.clone() {
			return Ok(client);
		}
		let notary_id = self.notary_id;
		let host = self.host().await?;
		let client = Arc::new(argon_notary_apis::create_client(&host).await.map_err(|error| {
			Error::NotaryError(format!(
				"Could not connect to notary {notary_id} ({host}) for audit - {error:?}",
			))
		})?);
		*self.client.write().await = Some(client.clone());
		Ok(client)
	}

	async fn download_header<B, C, AC>(
		&self,
		worker_context: &WorkerContext<B, C, AC>,
		notebook_number: NotebookNumber,
		mut download_url: Option<String>,
	) -> Result<SignedHeaderBytes, Error>
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		let notary_id = self.notary_id;
		let _download_slot = worker_context.download_slots.acquire().await.map_err(|error| {
			Error::NotaryError(format!(
				"Could not acquire header download slot for notary {notary_id} - {error:?}",
			))
		})?;
		let expected_archive_origin = self.archive_host.read().await.clone();
		if download_url.is_none() {
			if let Some(archive_host) = expected_archive_origin.as_ref() {
				download_url = Some(get_header_url(archive_host, notary_id, notebook_number));
			}
		}
		worker_context
			.notebook_downloader
			.get_header(notary_id, notebook_number, download_url, expected_archive_origin)
			.await
			.map_err(|error| {
				Error::NotaryError(format!(
					"Could not get notary {notary_id}, notebook {notebook_number} from notebook downloader - {error:?}",
				))
			})
	}

	async fn download_notebook<B, C, AC>(
		&self,
		worker_context: &WorkerContext<B, C, AC>,
		notebook_number: NotebookNumber,
	) -> Result<NotebookBytes, Error>
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		let notary_id = self.notary_id;
		let _download_slot = worker_context.download_slots.acquire().await.map_err(|error| {
			Error::NotaryError(format!(
				"Could not acquire notebook download slot for notary {notary_id} - {error:?}",
			))
		})?;
		let expected_archive_origin = self.archive_host.read().await.clone();
		let download_url = if let Some(archive_host) = expected_archive_origin.as_ref() {
			Some(get_notebook_url(archive_host, notary_id, notebook_number))
		} else if worker_context.notebook_downloader.is_strict() {
			None
		} else {
			let client = self.get_or_connect_client().await.ok();
			if let Some(client) = client {
				client.get_notebook_download_url(notebook_number).await.ok()
			} else {
				None
			}
		};
		worker_context
			.notebook_downloader
			.get_body(notary_id, notebook_number, download_url, expected_archive_origin)
			.await
			.map_err(|error| {
				Error::NotaryError(format!(
					"Could not download notebook {notebook_number} from notary {notary_id} - {error:?}",
				))
			})
	}

	async fn get_notebook_dependencies<B, C, AC>(
		&self,
		worker_context: &WorkerContext<B, C, AC>,
		notebook_number: NotebookNumber,
		best_hash: &B::Hash,
	) -> Result<Vec<NotaryNotebookAuditSummary>, Error>
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		let notary_id = self.notary_id;
		let mut notebook_dependencies = vec![];
		let mut missing_notebooks = vec![];
		let latest_block_notebook =
			Self::latest_notebook_in_runtime(worker_context, *best_hash, notary_id);

		if latest_block_notebook < notebook_number - 1 {
			let notary_notebooks = worker_context.aux_client.get_audit_summaries(notary_id)?.get();
			for notebook_number_needed in (latest_block_notebook + 1)..notebook_number {
				if let Some(summary) =
					notary_notebooks.iter().find(|s| s.notebook_number == notebook_number_needed)
				{
					notebook_dependencies.push(summary.clone());
				} else {
					missing_notebooks.push(notebook_number_needed);
				}
			}
		}

		if !missing_notebooks.is_empty() {
			let first_missing = missing_notebooks[0];
			let last_missing = missing_notebooks[missing_notebooks.len() - 1];
			let notebook_range = first_missing..=last_missing;
			trace!(
				"Missing notebooks for notary {notary_id}. Tracking dependency catchup range: {notebook_range:?}",
			);
			self.pending.lock().await.track_range(first_missing, last_missing);
			return Err(Error::MissingNotebooksError(format!(
				"Missing notebooks #{notebook_range:?} to audit {notebook_number} for notary {notary_id}"
			)));
		}

		Ok(notebook_dependencies)
	}

	fn latest_notebook_in_runtime<B, C, AC>(
		worker_context: &WorkerContext<B, C, AC>,
		block_hash: B::Hash,
		notary_id: NotaryId,
	) -> NotebookNumber
	where
		B: BlockT,
		C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
		AC: Clone + Codec + Send + Sync + 'static,
	{
		if let Ok(latest_notebooks_in_runtime) =
			worker_context.client.latest_notebook_by_notary(block_hash)
		{
			if let Some((latest_notebook, _)) = latest_notebooks_in_runtime.get(&notary_id) {
				return *latest_notebook;
			}
		}
		0
	}
}

pub struct NotaryClient<B: BlockT, C: AuxStore, AC> {
	worker_context: Arc<WorkerContext<B, C, AC>>,
	workers_by_id: WorkersById,
	pub(crate) metrics: Arc<Option<ConsensusMetrics<C>>>,
	pub tick_voting_power_receiver: Arc<Mutex<TracingUnboundedReceiver<VotingPowerInfo>>>,
	pub pause_notebook_audits: Arc<RwLock<bool>>,
	background_idle_delay: Duration,
	spawn_handle: Option<sc_service::SpawnTaskHandle>,
}

impl<B, C, AC> NotaryClient<B, C, AC>
where
	B: BlockT,
	C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
	AC: Clone + Codec + Send + Sync + 'static,
{
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		client: Arc<C>,
		aux_client: ArgonAux<B, C>,
		notebook_downloader: NotebookDownloader,
		metrics: Arc<Option<ConsensusMetrics<C>>>,
		ticker: Ticker,
		spawn_handle: Option<sc_service::SpawnTaskHandle>,
		background_idle_delay: Duration,
		is_solving_blocks: bool,
	) -> Self {
		let (tick_voting_power_sender, tick_voting_power_receiver) =
			tracing_unbounded("node::consensus::notebook_tick_stream", 100);
		let pause_notebook_audits = Arc::new(RwLock::new(false));
		let worker_context = Arc::new(WorkerContext {
			client,
			aux_client,
			notebook_downloader,
			metrics: metrics.clone(),
			pause_notebook_audits: pause_notebook_audits.clone(),
			ticker,
			tick_voting_power_sender: Arc::new(Mutex::new(tick_voting_power_sender)),
			download_slots: Arc::new(Semaphore::new(MAX_PARALLEL_NOTARY_DOWNLOADS)),
			audit_slots: Arc::new(Semaphore::new(MAX_PARALLEL_NOTARY_AUDITS)),
			is_solving_blocks,
			_phantom: PhantomData,
		});

		Self {
			worker_context,
			workers_by_id: Default::default(),
			metrics,
			tick_voting_power_receiver: Arc::new(Mutex::new(tick_voting_power_receiver)),
			pause_notebook_audits,
			background_idle_delay,
			spawn_handle,
		}
	}

	async fn worker(&self, notary_id: NotaryId) -> Option<WorkerHandle> {
		self.workers_by_id.read().await.get(&notary_id).cloned()
	}

	async fn ensure_worker(&self, notary_id: NotaryId) -> WorkerHandle {
		if let Some(worker) = self.worker(notary_id).await {
			return worker;
		}
		let mut workers = self.workers_by_id.write().await;
		workers
			.entry(notary_id)
			.or_insert_with(|| Arc::new(NotaryWorker::new(notary_id)))
			.clone()
	}

	pub async fn update_notaries(self: &Arc<Self>, block_hash: &B::Hash) -> Result<(), Error> {
		let Some(block_hash) = resolve_client_stateful_hash::<B, _, AC>(
			self.worker_context.client.as_ref(),
			*block_hash,
		)?
		else {
			return Ok(());
		};
		let notaries = self.worker_context.client.notaries(block_hash)?;
		let current_tick = self.worker_context.client.current_tick(block_hash)?;
		let active_ids = notaries.iter().map(|notary| notary.notary_id).collect::<BTreeSet<_>>();
		let existing_workers = self
			.workers_by_id
			.read()
			.await
			.iter()
			.map(|(notary_id, worker)| (*notary_id, worker.clone()))
			.collect::<Vec<_>>();

		for (notary_id, worker) in existing_workers {
			if active_ids.contains(&notary_id) {
				continue;
			}
			worker.clear_record().await;
			worker.clear_connection().await;
		}

		for notary in notaries {
			let notary_id = notary.notary_id;
			let worker = self.ensure_worker(notary_id).await;
			worker.start_background_task(
				Arc::clone(&self.worker_context),
				self.spawn_handle.clone(),
				self.background_idle_delay,
			);
			let host_changed = worker.update_record(notary.clone()).await;
			if host_changed {
				worker.clear_connection().await;
			}

			match notary.state {
				NotaryState::Locked { .. } => {
					worker.clear_connection().await;
					continue;
				},
				NotaryState::Reactivated { reprocess_notebook_number } => {
					worker.pending.lock().await.rewind_to(reprocess_notebook_number);
					self.worker_context
						.aux_client
						.reprocess_notebook(notary_id, reprocess_notebook_number)?;
				},
				_ => {},
			}

			if *self.pause_notebook_audits.read().await {
				worker.clear_subscription().await;
				continue;
			}

			worker.disconnect_if_subscription_stale(current_tick).await;
			let is_connected = worker.has_client().await && worker.has_subscription().await;

			if !is_connected || host_changed {
				if let Err(e) = worker.ensure_connected_subscription(current_tick).await {
					self.disconnect(
						&notary_id,
						Some(format!("Notary {notary_id} sync failed. {e:?}")),
					)
					.await;
				}
			}
		}
		Ok(())
	}

	pub async fn process_background_audits(self: &Arc<Self>) -> Result<bool, Error> {
		if *self.pause_notebook_audits.read().await {
			return Ok(false);
		}
		let Some((best_hash, finalized_hash)) =
			self.resolve_processing_hashes(self.worker_context.client.best_hash())?
		else {
			return Ok(false);
		};
		self.process_audits_at_hash(best_hash, finalized_hash).await
	}

	pub async fn process_audits_at(self: &Arc<Self>, block_hash: B::Hash) -> Result<bool, Error> {
		let Some((best_hash, finalized_hash)) = self.resolve_processing_hashes(block_hash)? else {
			return Ok(true);
		};
		self.process_audits_at_hash(best_hash, finalized_hash).await
	}

	async fn process_selected_audits_at(
		self: &Arc<Self>,
		block_hash: B::Hash,
		notary_ids: &BTreeSet<NotaryId>,
	) -> Result<bool, Error> {
		let Some((best_hash, finalized_hash)) = self.resolve_processing_hashes(block_hash)? else {
			return Ok(true);
		};
		self.process_selected_audits_at_hash(best_hash, finalized_hash, notary_ids)
			.await
	}

	fn resolve_processing_hashes(
		&self,
		block_hash: B::Hash,
	) -> Result<Option<ProcessingHashes<B::Hash>>, Error> {
		let Some(best_hash) = resolve_client_stateful_hash::<B, _, AC>(
			self.worker_context.client.as_ref(),
			block_hash,
		)?
		else {
			return Ok(None);
		};
		let Some(finalized_hash) = resolve_client_stateful_hash::<B, _, AC>(
			self.worker_context.client.as_ref(),
			self.worker_context.client.finalized_hash(),
		)?
		else {
			return Ok(None);
		};
		Ok(Some((best_hash, finalized_hash)))
	}

	async fn process_audits_at_hash(
		self: &Arc<Self>,
		best_hash: B::Hash,
		finalized_hash: B::Hash,
	) -> Result<bool, Error> {
		let notary_ids = self.workers_by_id.read().await.keys().copied().collect::<BTreeSet<_>>();
		let has_more_work = self
			.process_selected_audits_at_hash(best_hash, finalized_hash, &notary_ids)
			.await?;
		if let Some(metrics) = self.worker_context.metrics.as_ref() {
			let workers = self.workers_by_id.read().await;
			for (notary_id, worker) in workers.iter() {
				metrics.record_queue_depth(*notary_id, worker.pending.lock().await.len() as u64);
			}
		}
		Ok(has_more_work)
	}

	async fn process_selected_audits_at_hash(
		self: &Arc<Self>,
		best_hash: B::Hash,
		finalized_hash: B::Hash,
		notary_ids: &BTreeSet<NotaryId>,
	) -> Result<bool, Error> {
		let workers = self
			.workers_by_id
			.read()
			.await
			.iter()
			.filter(|(notary_id, _)| notary_ids.contains(notary_id))
			.map(|(_, worker)| worker.clone())
			.collect::<Vec<_>>();
		let mut has_more_work = false;
		let mut processing = Vec::new();
		for worker in workers {
			if worker.pending.lock().await.len() == 0 {
				continue;
			}
			let worker_context = Arc::clone(&self.worker_context);
			processing.push(tokio::spawn(async move {
				worker
					.process_for_hashes(worker_context.as_ref(), best_hash, finalized_hash)
					.await
			}));
		}
		for result in join_all(processing).await {
			match result {
				Ok(Ok(x)) => has_more_work = has_more_work || x,
				Ok(Err(err)) => {
					has_more_work = true;
					warn!("Error processing notebooks for a notary {err:?}");
				},
				Err(join_error) => {
					has_more_work = true;
					warn!("Error while processing tracked notary audits - {join_error:?}");
				},
			}
		}
		Ok(has_more_work)
	}

	pub async fn disconnect(&self, notary_id: &NotaryId, reason: Option<String>) {
		info!(
			"Notary client disconnected from notary #{notary_id} (or could not connect). Reason? {reason:?}"
		);
		let Some(worker) = self.worker(*notary_id).await else {
			return;
		};
		worker.clear_connection().await;
	}

	pub(crate) async fn verify_notebook_audits(
		self: &Arc<Self>,
		parent_hash: &B::Hash,
		notebook_audit_results: Vec<NotebookAuditResult<NotebookVerifyError>>,
		mode: NotebookAuditMode,
	) -> Result<(), Error> {
		let max_wait = match mode {
			NotebookAuditMode::Sync => None,
			NotebookAuditMode::Import { max_wait } => Some(max_wait),
		};
		let mut missing_audits = self.collect_missing_audits(&notebook_audit_results).await?;
		if missing_audits.by_notary.is_empty() {
			return Ok(());
		}
		info!(
			"Notebook digest has missing audits. Will attempt to catchup now. {:#?}",
			missing_audits.by_notary
		);
		let wait_time = Self::missing_audit_wait_time(max_wait, missing_audits.by_notary.len());
		let start = Instant::now();
		let timeout_error = || {
			Error::UnableToSyncNotary(format!(
				"Could not process all missing audits in {} seconds",
				wait_time.as_secs()
			))
		};
		if missing_audits.needs_notary_updates {
			let Some(remaining) = wait_time.checked_sub(start.elapsed()) else {
				warn!("Timed out waiting for missing audits. {:#?}", missing_audits.by_notary);
				return Err(timeout_error());
			};
			tokio::time::timeout(remaining, self.update_notaries(parent_hash))
				.await
				.map_err(|_| timeout_error())??;
		}
		self.wait_for_missing_audits(parent_hash, &mut missing_audits.by_notary, wait_time)
			.await
	}

	async fn collect_missing_audits(
		&self,
		notebook_audit_results: &[NotebookAuditResult<NotebookVerifyError>],
	) -> Result<MissingAuditCatchup, Error> {
		let mut missing_audits =
			MissingAuditCatchup { by_notary: BTreeMap::new(), needs_notary_updates: false };

		for digest_record in notebook_audit_results {
			let notary_audits = self
				.worker_context
				.aux_client
				.get_notary_audit_history(digest_record.notary_id)?
				.get();
			let audit = notary_audits.get(&digest_record.notebook_number);

			if let Some(audit) = audit {
				if digest_record.audit_first_failure != audit.audit_first_failure {
					return Err(Error::InvalidNotebookDigest(format!(
						"Notary {}, notebook #{} has an audit mismatch \"{:?}\" with local result. \"{:?}\"",
						digest_record.notary_id,
						digest_record.notebook_number,
						digest_record.audit_first_failure,
						audit.audit_first_failure
					)));
				}
				continue;
			}

			let has_runtime_record =
				if let Some(worker) = self.worker(digest_record.notary_id).await {
					worker.record.read().await.is_some()
				} else {
					false
				};
			let has_client = if let Some(worker) = self.worker(digest_record.notary_id).await {
				worker.has_client().await
			} else {
				false
			};
			if !has_runtime_record || !has_client {
				missing_audits.needs_notary_updates = true;
			}
			let worker = self.ensure_worker(digest_record.notary_id).await;
			worker
				.pending
				.lock()
				.await
				.track_notebook(digest_record.notebook_number, None, None);
			missing_audits
				.by_notary
				.entry(digest_record.notary_id)
				.or_default()
				.push(digest_record.notebook_number);
		}

		Ok(missing_audits)
	}

	fn missing_audit_wait_time(
		max_wait: Option<Duration>,
		missing_notary_count: usize,
	) -> Duration {
		Duration::from_secs(
			max_wait
				.map(|duration| duration.as_secs().max(1))
				.unwrap_or((missing_notary_count * 5).clamp(6, 120) as u64),
		)
	}

	async fn wait_for_missing_audits(
		self: &Arc<Self>,
		parent_hash: &B::Hash,
		missing_audits_by_notary: &mut BTreeMap<NotaryId, Vec<NotebookNumber>>,
		wait_time: Duration,
	) -> Result<(), Error> {
		let start = Instant::now();
		let timeout_error = || {
			Error::UnableToSyncNotary(format!(
				"Could not process all missing audits in {} seconds",
				wait_time.as_secs()
			))
		};
		loop {
			if start.elapsed() > wait_time {
				warn!("Timed out waiting for missing audits. {missing_audits_by_notary:#?}");
				return Err(timeout_error());
			}

			let missing_notary_ids = missing_audits_by_notary
				.iter()
				.filter_map(|(notary_id, audits)| (!audits.is_empty()).then_some(*notary_id))
				.collect::<BTreeSet<_>>();
			if missing_notary_ids.is_empty() {
				return Ok(());
			}

			let Some(remaining) = wait_time.checked_sub(start.elapsed()) else {
				warn!("Timed out waiting for missing audits. {missing_audits_by_notary:#?}");
				return Err(timeout_error());
			};
			let has_more_work = tokio::time::timeout(
				remaining,
				self.process_selected_audits_at(*parent_hash, &missing_notary_ids),
			)
			.await
			.map_err(|_| timeout_error())??;
			let mut has_missing_audits = false;
			for (notary_id, audits) in missing_audits_by_notary.iter_mut() {
				let notary_audits =
					self.worker_context.aux_client.get_notary_audit_history(*notary_id)?.get();
				audits.retain(|notebook_number| !notary_audits.contains_key(notebook_number));
				if !audits.is_empty() {
					has_missing_audits = true;
				}
			}
			if !has_missing_audits {
				return Ok(());
			}

			let sleep_ms = if has_more_work { 30 } else { 250 };
			tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
		}
	}
}

fn resolve_client_stateful_hash<B, C, AC>(
	client: &C,
	start_hash: B::Hash,
) -> Result<Option<B::Hash>, Error>
where
	B: BlockT,
	C: NotaryApisExt<B, AC> + ?Sized,
	AC: Clone + Codec,
{
	let mut cursor = start_hash;
	for _ in 0..DEFAULT_STATE_LOOKBACK_DEPTH {
		if client.has_block_state(cursor) {
			return Ok(Some(cursor));
		}

		let parent_hash = client.parent_hash(&cursor)?;
		if parent_hash == cursor {
			return Ok(None);
		}
		cursor = parent_hash;
	}

	Ok(None)
}

pub async fn get_notebook_header_data<B: BlockT, C, AccountId: Codec>(
	client: &Arc<C>,
	aux_client: &ArgonAux<B, C>,
	parent_hash: &B::Hash,
	voting_schedule: &VotingSchedule,
) -> Result<NotebookHeaderData<NotebookVerifyError>, Error>
where
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + AuxStore + 'static,
	C::Api: NotebookApis<B, NotebookVerifyError>
		+ NotaryApis<B, NotaryRecordT>
		+ BlockSealApis<B, AccountId, BlockSealAuthorityId>,
{
	let latest_notebooks_in_runtime =
		client.runtime_api().latest_notebook_by_notary(*parent_hash)?;
	let mut headers = NotebookHeaderData::default();
	let mut tick_notebooks = vec![];

	const MAX_NOTEBOOKS_PER_NOTARY: u32 = 20;
	let notaries = client.runtime_api().notaries(*parent_hash)?;
	for notary in notaries {
		if matches!(notary.state, NotaryState::Locked { .. }) {
			continue;
		}
		let (latest_runtime_notebook_number, _) =
			latest_notebooks_in_runtime.get(&notary.notary_id).unwrap_or(&(0, 0));
		let (mut notary_headers, tick_notebook) = match aux_client.get_notary_notebooks_for_header(
			notary.notary_id,
			*latest_runtime_notebook_number,
			voting_schedule,
			MAX_NOTEBOOKS_PER_NOTARY,
		) {
			Ok(x) => x,
			Err(e) => {
				error!(
					error = ?e,
					notary_id = notary.notary_id,
					notebook_tick = voting_schedule.notebook_tick(),
					"Error building notary notebooks");
				continue;
			},
		};

		let res = headers
			.notebook_digest
			.notebooks
			.try_append(&mut notary_headers.notebook_digest.notebooks.to_vec());
		if let Err(e) = res {
			error!(
				error = ?e,
				notary_id = notary.notary_id,
				notebook_tick = voting_schedule.notebook_tick(),
				"Error appending notary notebooks to digest"
			);
			break;
		}
		headers.signed_headers.append(&mut notary_headers.signed_headers);
		if let Some(tick_notebook) = tick_notebook {
			tick_notebooks.push(tick_notebook);
		}
	}

	headers.vote_digest = client.runtime_api().create_vote_digest(
		*parent_hash,
		voting_schedule.notebook_tick(),
		tick_notebooks,
	)?;
	Ok(headers)
}

pub struct NotebookDownloader {
	pub archive_hosts: Vec<ArchiveHost>,
	pub trust_mode: DownloadTrustMode,
	pub header_max_bytes: Option<u64>,
	pub notebook_max_bytes: Option<u64>,
}

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

impl NotebookDownloader {
	pub fn new<AR, S>(
		archive_hosts: AR,
		trust_mode: DownloadTrustMode,
		header_max_bytes: Option<u64>,
		notebook_max_bytes: Option<u64>,
	) -> Result<Self, Error>
	where
		AR: IntoIterator<Item = S>,
		S: AsRef<str>,
	{
		let archive_hosts = archive_hosts
			.into_iter()
			.map(|host| ArchiveHost::new(host.as_ref().to_string()))
			.collect::<Result<Vec<_>, _>>()
			.map_err(|e| Error::NotaryArchiveError(e.to_string()))?;
		Ok(Self { archive_hosts, trust_mode, header_max_bytes, notebook_max_bytes })
	}

	pub fn is_strict(&self) -> bool {
		matches!(self.trust_mode, DownloadTrustMode::Strict)
	}

	pub async fn get_header(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		download_url: Option<String>,
		expected_archive_origin: Option<String>,
	) -> Result<SignedHeaderBytes, Error> {
		let path_suffix =
			get_download_path_suffix(DownloadKind::Header, notary_id, notebook_number);
		if let Some(url) = download_url {
			let policy = DownloadPolicy {
				trust_mode: self.trust_mode,
				expected_origin: expected_archive_origin.clone(),
				expected_path_suffix: Some(path_suffix.clone()),
				max_bytes: self.header_max_bytes,
			};
			if let Ok(header) =
				ArchiveHost::download_header_bytes_with_policy(url, DOWNLOAD_TIMEOUT, &policy).await
			{
				return Ok(header);
			}
		}
		for archive_host in &self.archive_hosts {
			let url = archive_host.get_header_url(notary_id, notebook_number);
			let policy = DownloadPolicy {
				trust_mode: self.trust_mode,
				expected_origin: Some(archive_host.url.as_str().to_string()),
				expected_path_suffix: Some(path_suffix.clone()),
				max_bytes: self.header_max_bytes,
			};
			if let Ok(header) =
				ArchiveHost::download_header_bytes_with_policy(url, DOWNLOAD_TIMEOUT, &policy).await
			{
				return Ok(header);
			}
		}
		Err(Error::NotaryError("Could not get header from notary or archive".to_string()))
	}

	/// Get notebook body from notary or archive
	pub async fn get_body(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		download_url: Option<String>,
		expected_archive_origin: Option<String>,
	) -> Result<NotebookBytes, Error> {
		let path_suffix =
			get_download_path_suffix(DownloadKind::Notebook, notary_id, notebook_number);
		if let Some(url) = download_url {
			let policy = DownloadPolicy {
				trust_mode: self.trust_mode,
				expected_origin: expected_archive_origin.clone(),
				expected_path_suffix: Some(path_suffix.clone()),
				max_bytes: self.notebook_max_bytes,
			};
			if let Ok(body) =
				ArchiveHost::download_notebook_bytes_with_policy(url, DOWNLOAD_TIMEOUT, &policy)
					.await
			{
				return Ok(body);
			}
		}
		for archive_host in &self.archive_hosts {
			let url = archive_host.get_notebook_url(notary_id, notebook_number);
			let policy = DownloadPolicy {
				trust_mode: self.trust_mode,
				expected_origin: Some(archive_host.url.as_str().to_string()),
				expected_path_suffix: Some(path_suffix.clone()),
				max_bytes: self.notebook_max_bytes,
			};
			if let Ok(body) =
				ArchiveHost::download_notebook_bytes_with_policy(url, DOWNLOAD_TIMEOUT, &policy)
					.await
			{
				return Ok(body);
			}
		}
		Err(Error::NotaryError("Could not get body from notary or archive".to_string()))
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{error::Error, mock_notary::MockNotary, notary_client::NotaryApisExt};
	use argon_primitives::{
		AccountId, ChainTransfer, NotaryId, NotebookHeader, NotebookMeta, NotebookNumber,
		notary::{
			NotaryMeta, NotaryNotebookAuditSummary, NotaryNotebookAuditSummaryDetails,
			NotaryNotebookRawVotes, NotaryRecordWithState,
		},
	};
	use argon_runtime::Block;
	use codec::{Decode, Encode};

	use crate::mock_notary::setup_logs;
	use sp_core::{H256, bounded_vec};
	use sp_keyring::Ed25519Keyring;
	use std::collections::{BTreeMap, BTreeSet};

	#[derive(Clone, Default)]
	struct TestNode {
		pub aux: Arc<parking_lot::Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
		pub notaries: Arc<parking_lot::Mutex<Vec<NotaryRecordT>>>,
		pub latest_notebook_by_notary:
			Arc<parking_lot::Mutex<BTreeMap<NotaryId, (NotebookNumber, Tick)>>>,
		pub audit_dependencies: Arc<parking_lot::Mutex<Vec<NotaryNotebookAuditSummary>>>,
		#[allow(clippy::type_complexity)]
		pub notebook_audit_votes: Arc<parking_lot::Mutex<Option<Vec<(Vec<u8>, BlockVotingPower)>>>>,
		pub current_tick: Arc<parking_lot::Mutex<Tick>>,
		pub audit_failure: Arc<parking_lot::Mutex<Option<NotebookVerifyError>>>,
		pub block_chain: Arc<parking_lot::Mutex<Vec<H256>>>,
		pub block_latest_notebook: Arc<parking_lot::Mutex<BTreeMap<H256, (NotebookNumber, Tick)>>>,
		pub decode_intercept:
			Arc<parking_lot::Mutex<Option<NotaryNotebookDetails<<Block as BlockT>::Hash>>>>,
		pub decode_intercepted_at_block: Arc<parking_lot::Mutex<Option<<Block as BlockT>::Hash>>>,
		pub block_state_by_hash: Arc<parking_lot::Mutex<BTreeMap<H256, bool>>>,
		pub best_hash: Arc<parking_lot::Mutex<H256>>,
		pub finalized_hash: Arc<parking_lot::Mutex<H256>>,
	}

	impl TestNode {
		fn new() -> Self {
			Self {
				best_hash: Arc::new(parking_lot::Mutex::new(H256::from_slice(&[1; 32]))),
				finalized_hash: Arc::new(parking_lot::Mutex::new(H256::from_slice(&[0; 32]))),
				..Default::default()
			}
		}

		pub fn add_notary(&self, notary: &MockNotary) -> usize {
			let index = self.notaries.lock().len();
			(*self.notaries.lock()).push(NotaryRecordWithState {
				notary_id: notary.notary_id,
				meta: NotaryMeta {
					name: "".into(),
					public: Ed25519Keyring::Bob.public(),
					hosts: bounded_vec![notary.addr.clone().into()],
				},
				meta_updated_block: 1,
				activated_block: 1,
				meta_updated_tick: 1,
				operator_account_id: Ed25519Keyring::Bob.to_account_id(),
				state: NotaryState::Active,
			});
			index
		}

		fn set_best_hash(&self, hash: H256) {
			*self.best_hash.lock() = hash;
		}

		fn set_finalized_hash(&self, hash: H256) {
			*self.finalized_hash.lock() = hash;
		}

		fn set_block_state(&self, hash: H256, has_state: bool) {
			self.block_state_by_hash.lock().insert(hash, has_state);
		}
	}

	impl AuxStore for TestNode {
		fn insert_aux<
			'a,
			'b: 'a,
			'c: 'a,
			I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
			D: IntoIterator<Item = &'a &'b [u8]>,
		>(
			&self,
			insert: I,
			delete: D,
		) -> sc_client_api::blockchain::Result<()> {
			let mut aux = self.aux.lock();
			for (k, v) in insert {
				aux.insert(k.to_vec(), v.to_vec());
			}
			for k in delete {
				aux.remove(*k);
			}
			Ok(())
		}

		fn get_aux(&self, key: &[u8]) -> sc_client_api::blockchain::Result<Option<Vec<u8>>> {
			let aux = self.aux.lock();
			Ok(aux.get(key).cloned())
		}
	}

	impl NotaryApisExt<Block, AccountId> for TestNode {
		fn has_block_state(&self, block_hash: <Block as BlockT>::Hash) -> bool {
			*self.block_state_by_hash.lock().get(&block_hash).unwrap_or(&true)
		}
		fn notaries(&self, _block_hash: H256) -> Result<Vec<NotaryRecordT>, Error> {
			Ok(self.notaries.lock().clone())
		}
		fn latest_notebook_by_notary(
			&self,
			block_hash: <Block as BlockT>::Hash,
		) -> Result<BTreeMap<NotaryId, (NotebookNumber, Tick)>, Error> {
			if let Some((notebook_number, tick)) =
				self.block_latest_notebook.lock().get(&block_hash)
			{
				return Ok(BTreeMap::from_iter(vec![(1, (*notebook_number, *tick))]));
			}
			Ok(self.latest_notebook_by_notary.lock().clone())
		}
		fn current_tick(&self, _block_hash: <Block as BlockT>::Hash) -> Result<Tick, Error> {
			Ok(*self.current_tick.lock())
		}
		fn audit_notebook_and_get_votes(
			&self,
			_block_hash: <Block as BlockT>::Hash,
			_version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			_tick: Tick,
			_header_hash: H256,
			_notebook: &[u8],
			notebook_dependencies: Vec<NotaryNotebookAuditSummary>,
			_blocks_with_votes: &[<Block as BlockT>::Hash],
		) -> Result<Result<NotaryNotebookRawVotes, NotebookVerifyError>, Error> {
			if let Some(err) = self.audit_failure.lock().take() {
				return Ok(Err(err));
			}
			*self.audit_dependencies.lock() = notebook_dependencies;
			let votes = self.notebook_audit_votes.lock().take();
			Ok(Ok(NotaryNotebookRawVotes {
				raw_votes: votes.unwrap_or_default(),
				notary_id,
				notebook_number,
			}))
		}
		fn vote_minimum(&self, _block_hash: <Block as BlockT>::Hash) -> Result<VoteMinimum, Error> {
			Ok(500)
		}
		fn decode_signed_raw_notebook_header(
			&self,
			block_hash: &<Block as BlockT>::Hash,
			raw_header: Vec<u8>,
		) -> Result<Result<NotaryNotebookDetails<<Block as BlockT>::Hash>, DispatchError>, Error>
		{
			if let Some(intercept) = self.decode_intercept.lock().take() {
				self.decode_intercepted_at_block.lock().replace(*block_hash);
				return Ok(Ok(intercept));
			}
			let header = NotebookHeader::decode(&mut raw_header.as_ref())
				.map_err(|_| Error::NotaryError("Unable to decode".to_string()))?;

			let summary = NotaryNotebookAuditSummaryDetails {
				changed_accounts_root: header.changed_accounts_root,
				account_changelist: header.changed_account_origins.clone().to_vec(),
				used_transfers_to_localchain: header
					.chain_transfers
					.iter()
					.filter_map(|t| match t {
						ChainTransfer::ToLocalchain { transfer_id } => Some(*transfer_id),
						_ => None,
					})
					.collect(),
				secret_hash: header.secret_hash,
				block_votes_root: header.block_votes_root,
			};

			Ok(Ok(NotaryNotebookDetails {
				notary_id: header.notary_id,
				notebook_number: header.notebook_number,
				version: header.version as u32,
				tick: header.tick,
				header_hash: header.hash(),
				block_votes_count: header.block_votes_count,
				block_voting_power: header.block_voting_power,
				blocks_with_votes: header.blocks_with_votes.to_vec().clone(),
				raw_audit_summary: summary.encode(),
			}))
		}
		fn best_hash(&self) -> <Block as BlockT>::Hash {
			*self.best_hash.lock()
		}
		fn finalized_hash(&self) -> <Block as BlockT>::Hash {
			*self.finalized_hash.lock()
		}
		fn parent_hash(
			&self,
			hash: &<Block as BlockT>::Hash,
		) -> Result<<Block as BlockT>::Hash, Error> {
			let block_chain = self.block_chain.lock();
			if let Some(pos) = block_chain.iter().position(|h| h == hash) {
				if pos > 0 {
					return Ok(block_chain[pos - 1]);
				}
			}
			Ok(H256::from_slice(&[3; 32]))
		}
	}

	async fn system() -> (MockNotary, Arc<TestNode>, Arc<NotaryClient<Block, TestNode, AccountId>>)
	{
		let mut test_notary = MockNotary::new(1);
		test_notary.start().await.expect("could not start notary");
		test_notary.state.lock().await.metadata =
			Some(NotebookMeta { last_closed_notebook_number: 0, last_closed_notebook_tick: 0 });
		let archive_host = test_notary.archive_host.clone();

		let client = Arc::new(TestNode::new());
		client.add_notary(&test_notary);
		let aux_client = ArgonAux::new(client.clone());

		let ticker = Ticker::new(2000, 2);

		let notebook_downloader =
			NotebookDownloader::new(vec![archive_host], DownloadTrustMode::Dev, None, None)
				.unwrap();
		let notary_client = NotaryClient::new(
			client.clone(),
			aux_client,
			notebook_downloader,
			Arc::new(None),
			ticker,
			None,
			Duration::from_millis(250),
			true,
		);
		let notary_client = Arc::new(notary_client);
		(test_notary, client, notary_client)
	}

	async fn worker(
		notary_client: &Arc<NotaryClient<Block, TestNode, AccountId>>,
		notary_id: NotaryId,
	) -> Option<Arc<NotaryWorker>> {
		notary_client.worker(notary_id).await
	}

	async fn worker_tracking_snapshot(
		notary_client: &Arc<NotaryClient<Block, TestNode, AccountId>>,
		notary_id: NotaryId,
	) -> Vec<(NotebookNumber, bool)> {
		let Some(worker) = worker(notary_client, notary_id).await else {
			return Vec::new();
		};
		worker.pending.lock().await.snapshot()
	}

	async fn worker_tracked_notebook_count(
		notary_client: &Arc<NotaryClient<Block, TestNode, AccountId>>,
		notary_id: NotaryId,
	) -> usize {
		let Some(worker) = worker(notary_client, notary_id).await else {
			return 0;
		};
		worker.pending.lock().await.len()
	}

	async fn track_worker_notebook(
		notary_client: &Arc<NotaryClient<Block, TestNode, AccountId>>,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		header_bytes: Option<SignedHeaderBytes>,
		known_since: Option<Instant>,
	) {
		let worker = notary_client.ensure_worker(notary_id).await;
		worker
			.pending
			.lock()
			.await
			.track_notebook(notebook_number, header_bytes, known_since);
	}

	async fn has_worker_client(
		notary_client: &Arc<NotaryClient<Block, TestNode, AccountId>>,
		notary_id: NotaryId,
	) -> bool {
		let Some(worker) = worker(notary_client, notary_id).await else {
			return false;
		};
		worker.has_client().await
	}

	async fn has_worker_subscription(
		notary_client: &Arc<NotaryClient<Block, TestNode, AccountId>>,
		notary_id: NotaryId,
	) -> bool {
		let Some(worker) = worker(notary_client, notary_id).await else {
			return false;
		};
		worker.has_subscription().await
	}

	async fn wait_for_subscription(
		notary_client: &Arc<NotaryClient<Block, TestNode, AccountId>>,
		wait_duration: Duration,
	) -> Option<(NotaryId, NotebookNumber)> {
		let start = Instant::now();
		loop {
			let worker_ids =
				notary_client.workers_by_id.read().await.keys().copied().collect::<Vec<_>>();
			for notary_id in worker_ids {
				let Some(worker) = worker(notary_client, notary_id).await else {
					continue;
				};
				if let Some(notebook_number) =
					worker.poll_subscription(notary_client.worker_context.as_ref()).await
				{
					return Some((notary_id, notebook_number));
				}
			}
			if start.elapsed() > wait_duration {
				return None;
			}
			tokio::task::yield_now().await;
		}
	}

	#[tokio::test]
	async fn adds_new_notaries() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert_eq!(notary_client.workers_by_id.read().await.len(), 1);
		assert!(has_worker_client(&notary_client, 1).await);
		assert!(has_worker_subscription(&notary_client, 1).await);

		test_notary.create_notebook_header(vec![]).await;
		let next = wait_for_subscription(&notary_client, Duration::from_millis(500)).await.unwrap();
		assert_eq!(next.0, 1);

		// now mark the notary as audit failed
		(*client.notaries.lock()).get_mut(0).unwrap().state = NotaryState::Locked {
			failed_audit_reason: NotebookVerifyError::InvalidSecretProvided,
			at_tick: 1,
			notebook_number: 1,
		};
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert_eq!(notary_client.workers_by_id.read().await.len(), 1);
		assert!(!has_worker_client(&notary_client, 1).await);
		assert!(!has_worker_subscription(&notary_client, 1).await);
		test_notary.create_notebook_header(vec![]).await;
		let next = wait_for_subscription(&notary_client, Duration::from_millis(500)).await;
		assert!(next.is_none());
	}

	#[tokio::test]
	async fn handles_notebook_tracking_correctly() {
		let (_test_notary, _client, notary_client) = system().await;
		track_worker_notebook(&notary_client, 1, 3, None, None).await;
		track_worker_notebook(&notary_client, 1, 1, None, None).await;
		track_worker_notebook(&notary_client, 1, 2, None, None).await;
		assert_eq!(
			worker_tracking_snapshot(&notary_client, 1).await,
			vec![(1, false), (2, false), (3, false)]
		);
		let next = worker(&notary_client, 1)
			.await
			.expect("worker should exist")
			.next_for_processing(notary_client.worker_context.as_ref(), 0)
			.await;
		assert!(next.is_none());
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 3);

		track_worker_notebook(&notary_client, 1, 2, Some(Default::default()), None).await;
		track_worker_notebook(&notary_client, 1, 1, Some(Default::default()), None).await;

		assert_eq!(
			worker_tracking_snapshot(&notary_client, 1).await,
			vec![(1, true), (2, true), (3, false)]
		);

		track_worker_notebook(&notary_client, 1, 1, None, None).await;
		assert_eq!(
			worker_tracking_snapshot(&notary_client, 1).await,
			vec![(1, true), (2, true), (3, false)]
		);
		let next = worker(&notary_client, 1)
			.await
			.expect("worker should exist")
			.next_for_processing(notary_client.worker_context.as_ref(), 0)
			.await;
		assert!(next.is_some());
	}

	/// Test that if a notary disconnects and then a new block comes in, the client is able to
	/// reconnect in order to retrieve and audit the missing notebooks
	#[tokio::test]
	async fn handles_audit_reconnect() {
		setup_logs();
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let mut last = test_notary.create_notebook_header(vec![]).await;
		for _ in 0..12 {
			last = test_notary.create_notebook_header(vec![]).await;
		}

		// disconnect the notary
		notary_client.disconnect(&1, None).await;

		// now simulate a new block coming in
		notary_client
			.verify_notebook_audits(
				&client.best_hash(),
				vec![NotebookAuditResult {
					notary_id: 1,
					tick: last.tick,
					notebook_number: last.notebook_number,
					audit_first_failure: None,
				}],
				NotebookAuditMode::Sync,
			)
			.await
			.expect("Could not retrieve missing notebooks");
	}

	#[tokio::test]
	async fn handles_audit_reconnect_in_import_mode() {
		setup_logs();
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let mut last = test_notary.create_notebook_header(vec![]).await;
		for _ in 0..12 {
			last = test_notary.create_notebook_header(vec![]).await;
		}

		notary_client.disconnect(&1, None).await;

		notary_client
			.verify_notebook_audits(
				&client.best_hash(),
				vec![NotebookAuditResult {
					notary_id: 1,
					tick: last.tick,
					notebook_number: last.notebook_number,
					audit_first_failure: None,
				}],
				NotebookAuditMode::Import { max_wait: Duration::from_secs(20) },
			)
			.await
			.expect("Could not retrieve missing notebooks in import mode");
	}

	#[tokio::test]
	async fn reconnects_on_periodic_refresh_without_new_block() {
		setup_logs();
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert!(has_worker_client(&notary_client, 1).await);
		assert!(has_worker_subscription(&notary_client, 1).await);

		notary_client.disconnect(&1, None).await;
		assert!(!has_worker_client(&notary_client, 1).await);
		assert!(!has_worker_subscription(&notary_client, 1).await);

		// Periodic notary refreshes reuse the same best hash when no new block has arrived,
		// but connection attempts are limited to once per tick.
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not refresh notaries");
		assert!(!has_worker_client(&notary_client, 1).await);
		assert!(!has_worker_subscription(&notary_client, 1).await);

		*client.current_tick.lock() = 1;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not refresh notaries");
		assert!(has_worker_client(&notary_client, 1).await);
		assert!(has_worker_subscription(&notary_client, 1).await);

		test_notary.create_notebook_header(vec![]).await;
		let next = wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("worker should resubscribe without a new block");
		assert_eq!(next, (1, 1));
	}

	#[tokio::test]
	async fn disconnects_stale_worker_subscription_after_missing_ticks() {
		let (mut test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		test_notary.create_notebook_header(vec![]).await;
		let next = wait_for_subscription(&notary_client, Duration::from_millis(500)).await.unwrap();
		assert_eq!(next, (1, 1));

		let worker = worker(&notary_client, 1).await.expect("worker should exist");
		assert_eq!(*worker.last_notebook_tick.read().await, Some(1));

		*client.current_tick.lock() = 3;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert_eq!(*worker.last_notebook_tick.read().await, Some(1));
		assert!(worker.has_client().await);
		assert!(worker.has_subscription().await);

		test_notary.stop().await;
		*client.current_tick.lock() = 4;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		assert_eq!(*worker.last_notebook_tick.read().await, None);
		assert!(!worker.has_client().await);
		assert!(!worker.has_subscription().await);
		assert_eq!(*worker.last_connection_attempt_tick.read().await, Some(4));

		test_notary.start().await.expect("could not restart notary");
		client.notaries.lock()[0].meta.hosts = bounded_vec![test_notary.addr.clone().into()];
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		assert!(!worker.has_client().await);
		assert!(!worker.has_subscription().await);

		*client.current_tick.lock() = 5;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		assert_eq!(*worker.last_connection_attempt_tick.read().await, Some(5));
		assert!(worker.has_client().await);
		assert!(worker.has_subscription().await);
	}

	#[tokio::test]
	async fn supplies_missing_notebooks_on_audit() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let result = notary_client
			.worker(test_notary.notary_id)
			.await
			.expect("worker should exist")
			.get_notebook_dependencies(
				notary_client.worker_context.as_ref(),
				10,
				&client.best_hash(),
			)
			.await
			.expect_err("Should not have all dependencies");
		assert!(matches!(result, Error::MissingNotebooksError(_)),);
		assert_eq!(
			worker_tracking_snapshot(&notary_client, 1).await,
			vec![
				(1, false),
				(2, false),
				(3, false),
				(4, false),
				(5, false),
				(6, false),
				(7, false),
				(8, false),
				(9, false)
			]
		);
		client.latest_notebook_by_notary.lock().insert(1, (8, 1));
		let result = notary_client
			.worker(test_notary.notary_id)
			.await
			.expect("worker should exist")
			.get_notebook_dependencies(
				notary_client.worker_context.as_ref(),
				10,
				&client.best_hash(),
			)
			.await
			.expect_err("Should have all dependencies");

		// still missing number 9
		assert!(matches!(result, Error::MissingNotebooksError(_)),);
		assert!(result.to_string().contains("#9..=9"));

		for _ in 0..9 {
			notary_client
				.process_background_audits()
				.await
				.expect("Could not process queues");
		}
		assert_eq!(worker_tracking_snapshot(&notary_client, 1).await, vec![(9, false)]);
		for _ in 0..10 {
			test_notary.create_notebook_header(vec![]).await;
			notary_client
				.process_background_audits()
				.await
				.expect("Could not process queues");
		}
		let mut rx = notary_client.tick_voting_power_receiver.lock().await;
		let next_rx = rx.next().await.expect("Could not receive");
		assert_eq!(next_rx.0, 9);
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 0);
		let result = notary_client
			.worker(test_notary.notary_id)
			.await
			.expect("worker should exist")
			.get_notebook_dependencies(
				notary_client.worker_context.as_ref(),
				10,
				&client.best_hash(),
			)
			.await
			.expect("Could not retrieve missing notebooks");
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].notebook_number, 9);
	}

	#[tokio::test]
	async fn tracks_full_missing_notebook_dependency_range() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let result = notary_client
			.worker(test_notary.notary_id)
			.await
			.expect("worker should exist")
			.get_notebook_dependencies(
				notary_client.worker_context.as_ref(),
				42,
				&client.best_hash(),
			)
			.await
			.expect_err("Should not have all dependencies");
		assert!(matches!(result, Error::MissingNotebooksError(_)));
		assert_eq!(
			worker_tracking_snapshot(&notary_client, 1).await,
			(1..42).map(|number| (number, false)).collect::<Vec<_>>()
		);
	}

	#[tokio::test]
	async fn can_process_notebooks_in_parallel() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let mut test_notary2 = MockNotary::new(2);
		test_notary2.start().await.expect("could not start notary");
		client.add_notary(&test_notary2);
		test_notary.create_notebook_header(vec![]).await;
		test_notary2.create_notebook_header(vec![]).await;

		wait_for_subscription(&notary_client, Duration::from_millis(500)).await;
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 1);
		assert!(
			worker(&notary_client, 2).await.is_none() ||
				worker_tracked_notebook_count(&notary_client, 2).await == 0
		);
		notary_client
			.process_background_audits()
			.await
			.expect("Could not process queues");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 0);
		assert!(
			worker(&notary_client, 2).await.is_none() ||
				worker_tracked_notebook_count(&notary_client, 2).await == 0
		);

		let next_rx = notary_client
			.tick_voting_power_receiver
			.lock()
			.await
			.next()
			.await
			.expect("Could not receive");
		assert_eq!(next_rx, (1, 0, 1));

		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert!(has_worker_client(&notary_client, 1).await);
		assert!(has_worker_client(&notary_client, 2).await);
		assert!(has_worker_subscription(&notary_client, 1).await);
		assert!(has_worker_subscription(&notary_client, 2).await);

		test_notary.create_notebook_header(vec![]).await;
		test_notary2.create_notebook_header(vec![]).await;

		wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("Could not get next");
		wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("Could not get next");

		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 1);
		assert_eq!(worker_tracking_snapshot(&notary_client, 1).await[0].0, 2);
		assert_eq!(worker_tracked_notebook_count(&notary_client, 2).await, 2);
		assert_eq!(worker_tracking_snapshot(&notary_client, 2).await[0].0, 1);
		// should process one from each notary
		notary_client
			.process_background_audits()
			.await
			.expect("Could not process queues");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 0);
		assert_eq!(worker_tracked_notebook_count(&notary_client, 2).await, 1);
	}

	#[tokio::test]
	async fn targeted_import_catchup_only_processes_missing_notaries() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let mut test_notary2 = MockNotary::new(2);
		test_notary2.start().await.expect("could not start second notary");
		client.add_notary(&test_notary2);
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let notebook_1 = test_notary.create_notebook_header(vec![]).await;
		let notebook_2 = test_notary2.create_notebook_header(vec![]).await;
		track_worker_notebook(&notary_client, 1, notebook_1.notebook_number, None, None).await;
		track_worker_notebook(&notary_client, 2, notebook_2.notebook_number, None, None).await;

		let targeted = BTreeSet::from([1]);
		notary_client
			.process_selected_audits_at(client.best_hash(), &targeted)
			.await
			.expect("Could not process targeted queues");

		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 0);
		assert_eq!(worker_tracked_notebook_count(&notary_client, 2).await, 1);
	}

	#[tokio::test]
	async fn uses_stateful_ancestor_when_finalized_lacks_state() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		test_notary.create_notebook_header(vec![]).await;
		wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("Could not get next");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 1);

		let block_0 = H256::from_slice(&[0; 32]);
		let block_1 = H256::from_slice(&[1; 32]);
		let block_2 = H256::from_slice(&[2; 32]);
		client.block_chain.lock().append(&mut vec![block_0, block_1, block_2]);
		client.set_best_hash(block_2);
		client.set_finalized_hash(block_1);
		client.set_block_state(block_2, true);
		client.set_block_state(block_1, false);
		client.set_block_state(block_0, true);
		client.decode_intercept.lock().replace(NotaryNotebookDetails {
			notary_id: 1,
			notebook_number: 1,
			version: 1,
			tick: 1,
			header_hash: H256::from_slice(&[9; 32]),
			block_votes_count: 0,
			block_voting_power: 0,
			blocks_with_votes: vec![],
			raw_audit_summary: vec![],
		});

		let has_more_work = notary_client
			.process_background_audits()
			.await
			.expect("Could not process queues");
		assert!(!has_more_work);
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 0);
		assert_eq!(*client.decode_intercepted_at_block.lock(), Some(block_2));
	}

	#[tokio::test]
	async fn uses_stateful_ancestor_when_best_lacks_state() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		test_notary.create_notebook_header(vec![]).await;
		wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("Could not get next");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 1);

		let block_0 = H256::from_slice(&[0; 32]);
		let block_1 = H256::from_slice(&[1; 32]);
		let block_2 = H256::from_slice(&[2; 32]);
		client.block_chain.lock().append(&mut vec![block_0, block_1, block_2]);
		client.set_best_hash(block_2);
		client.set_finalized_hash(block_0);
		client.set_block_state(block_2, false);
		client.set_block_state(block_1, true);
		client.set_block_state(block_0, true);

		client.decode_intercept.lock().replace(NotaryNotebookDetails {
			notary_id: 1,
			notebook_number: 1,
			version: 1,
			tick: 1,
			header_hash: H256::from_slice(&[9; 32]),
			block_votes_count: 0,
			block_voting_power: 0,
			blocks_with_votes: vec![],
			raw_audit_summary: vec![],
		});

		let has_more_work = notary_client
			.process_background_audits()
			.await
			.expect("Could not process queues");
		assert!(!has_more_work);
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 0);
		assert_eq!(*client.decode_intercepted_at_block.lock(), Some(block_1));
	}

	#[tokio::test]
	async fn paused_notebook_audits_return_false_and_keep_tracking() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		test_notary.create_notebook_header(vec![]).await;
		wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("Could not get next");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 1);

		*notary_client.pause_notebook_audits.write().await = true;

		let has_more_work = notary_client
			.process_background_audits()
			.await
			.expect("Could not process queues");
		assert!(!has_more_work);
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 1);
	}

	#[tokio::test]
	async fn paused_notebook_audits_disable_subscriptions_until_resumed() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert!(has_worker_subscription(&notary_client, 1).await);

		*notary_client.pause_notebook_audits.write().await = true;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert!(!has_worker_subscription(&notary_client, 1).await);

		test_notary.create_notebook_header(vec![]).await;
		let next = wait_for_subscription(&notary_client, Duration::from_millis(250)).await;
		assert!(next.is_none());
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 0);

		*notary_client.pause_notebook_audits.write().await = false;
		*client.current_tick.lock() = 1;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert!(has_worker_subscription(&notary_client, 1).await);

		test_notary.create_notebook_header(vec![]).await;
		let next = wait_for_subscription(&notary_client, Duration::from_millis(500)).await;
		assert_eq!(next, Some((1, 2)));
	}

	#[tokio::test]
	async fn import_catchup_uses_stateful_ancestor_when_target_hash_lacks_state() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		test_notary.create_notebook_header(vec![]).await;
		wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("Could not get next");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 1);

		let block_0 = H256::from_slice(&[0; 32]);
		let block_1 = H256::from_slice(&[1; 32]);
		let block_2 = H256::from_slice(&[2; 32]);
		client.block_chain.lock().append(&mut vec![block_0, block_1, block_2]);
		client.set_best_hash(block_2);
		client.set_finalized_hash(block_0);
		client.set_block_state(block_0, true);
		client.set_block_state(block_1, true);
		client.set_block_state(block_2, false);
		client.decode_intercept.lock().replace(NotaryNotebookDetails {
			notary_id: 1,
			notebook_number: 1,
			version: 1,
			tick: 1,
			header_hash: H256::from_slice(&[9; 32]),
			block_votes_count: 0,
			block_voting_power: 0,
			blocks_with_votes: vec![],
			raw_audit_summary: vec![],
		});

		let has_more_work = notary_client
			.process_audits_at(block_2)
			.await
			.expect("Could not process queues");
		assert!(!has_more_work);
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 0);
		assert_eq!(*client.decode_intercepted_at_block.lock(), Some(block_1));
	}

	#[tokio::test]
	async fn requeues_notebooks_failing_audit_before_tick() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		test_notary.create_notebook_header(vec![]).await;
		test_notary.create_notebook_header(vec![]).await;
		test_notary.create_notebook_header(vec![]).await;

		wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("Could not get next");
		wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("Could not get next");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 2);
		wait_for_subscription(&notary_client, Duration::from_millis(500))
			.await
			.expect("Could not get next");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 3);

		notary_client
			.process_background_audits()
			.await
			.expect("Could not process queues");
		notary_client
			.process_background_audits()
			.await
			.expect("Could not process queues");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 1);

		*client.current_tick.lock() = 2;
		client
			.audit_failure
			.lock()
			.replace(NotebookVerifyError::InvalidChainTransfersList);
		notary_client
			.process_background_audits()
			.await
			.expect("Could not process queues");
		assert_eq!(worker_tracked_notebook_count(&notary_client, 1).await, 1);
	}

	#[tokio::test]
	async fn import_wait_timeout_does_not_block_on_slow_audit_work() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let notebook = test_notary.create_notebook_header(vec![]).await;
		track_worker_notebook(&notary_client, 1, notebook.notebook_number, None, None).await;

		let audit_slots = notary_client
			.worker_context
			.audit_slots
			.acquire_many(MAX_PARALLEL_NOTARY_AUDITS as u32)
			.await
			.expect("Could not exhaust audit slots");
		let mut missing_audits = BTreeMap::from([(1, vec![notebook.notebook_number])]);
		let start = Instant::now();
		let result = notary_client
			.wait_for_missing_audits(
				&client.best_hash(),
				&mut missing_audits,
				Duration::from_millis(50),
			)
			.await;
		assert!(matches!(result, Err(Error::UnableToSyncNotary(_))));
		assert!(
			start.elapsed() < Duration::from_millis(250),
			"import wait should return promptly once its deadline is exceeded",
		);

		drop(audit_slots);

		let mut audit_completed = false;
		for _ in 0..25 {
			if notary_client
				.worker_context
				.aux_client
				.get_notary_audit_history(1)
				.expect("could not read audit history")
				.get()
				.contains_key(&notebook.notebook_number)
			{
				audit_completed = true;
				break;
			}
			tokio::time::sleep(Duration::from_millis(20)).await;
		}

		assert!(
			audit_completed,
			"in-flight notary work should continue after the import wait times out",
		);
	}

	#[tokio::test]
	async fn finds_correct_parent_if_already_audited() {
		let (_test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let block_0 = H256::from_slice(&[0; 32]);
		let block_1 = H256::from_slice(&[1; 32]);
		let block_2 = H256::from_slice(&[2; 32]);
		let block_3 = H256::from_slice(&[3; 32]);
		let block_4 = H256::from_slice(&[4; 32]);

		client
			.block_chain
			.lock()
			.append(&mut vec![block_0, block_1, block_2, block_3, block_4]);
		client.block_latest_notebook.lock().insert(block_0, (1, 1));
		client.block_latest_notebook.lock().insert(block_1, (2, 1));
		client.block_latest_notebook.lock().insert(block_2, (2, 1));
		client.block_latest_notebook.lock().insert(block_3, (3, 1));
		client.block_latest_notebook.lock().insert(block_4, (3, 1));

		client.decode_intercept.lock().replace(NotaryNotebookDetails {
			notary_id: 1,
			notebook_number: 3,
			version: 1,
			tick: 1,
			header_hash: H256::from_slice(&[1; 32]),
			block_votes_count: 0,
			block_voting_power: 0,
			blocks_with_votes: vec![],
			raw_audit_summary: vec![],
		});
		let _ = worker(&notary_client, 1)
			.await
			.expect("worker should exist")
			.process_notebook(
				notary_client.worker_context.as_ref(),
				3,
				2,
				&H256::from_slice(&[4; 32]),
				SignedHeaderBytes(vec![]),
				Instant::now(),
			)
			.await;

		let attempted_decode_at = client.decode_intercepted_at_block.lock().take();
		assert_eq!(attempted_decode_at, Some(block_2));
	}
}
