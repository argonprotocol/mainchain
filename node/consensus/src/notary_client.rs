use crate::{aux_client::ArgonAux, error::Error, metrics::ConsensusMetrics};
use argon_notary_apis::{
	get_header_url, get_notebook_url,
	notebook::{NotebookRpcClient, RawHeadersSubscription},
	ArchiveHost, Client, SystemRpcClient,
};
use argon_primitives::{
	ensure,
	notary::{
		NotaryNotebookAuditSummary, NotaryNotebookDetails, NotaryNotebookRawVotes, NotaryState,
		NotebookBytes, SignedHeaderBytes,
	},
	notebook::NotebookNumber,
	tick::{Tick, Ticker},
	Balance, BlockSealApis, BlockSealAuthorityId, BlockVotingPower, NotaryApis, NotaryId,
	NotebookApis, NotebookAuditResult, NotebookHeaderData, VoteMinimum, VotingSchedule,
};
use argon_runtime::{NotaryRecordT, NotebookVerifyError};
use codec::Codec;
use futures::{future::join_all, Stream, StreamExt};
use log::{info, trace, warn};
use prometheus_endpoint::Registry;
use sc_client_api::{AuxStore, BlockchainEvents};
use sc_service::TaskManager;
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::H256;
use sp_runtime::{traits::Block as BlockT, DispatchError};
use std::{
	collections::{BTreeMap, BTreeSet},
	default::Default,
	marker::PhantomData,
	pin::Pin,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::{Mutex, RwLock};

const MAX_QUEUE_DEPTH: usize = 1440 * 2; // a notary can be down 2 days before we start dropping history

pub trait NotaryApisExt<B: BlockT, AC> {
	fn notaries(&self, block_hash: B::Hash) -> Result<Vec<NotaryRecordT>, Error>;
	fn latest_notebook_by_notary(
		&self,
		block_hash: B::Hash,
	) -> Result<BTreeMap<NotaryId, (NotebookNumber, Tick)>, Error>;
	#[allow(clippy::too_many_arguments)]
	fn audit_notebook_and_get_votes(
		&self,
		block_hash: B::Hash,
		version: u32,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		notebook_tick: Tick,
		header_hash: H256,
		vote_minimums: &BTreeMap<B::Hash, Balance>,
		notebook: &[u8],
		notebook_dependencies: Vec<NotaryNotebookAuditSummary>,
	) -> Result<Result<NotaryNotebookRawVotes, NotebookVerifyError>, Error>;
	fn vote_minimum(&self, block_hash: B::Hash) -> Result<VoteMinimum, Error>;
	fn decode_signed_raw_notebook_header(
		&self,
		block_hash: &B::Hash,
		raw_header: Vec<u8>,
	) -> Result<Result<NotaryNotebookDetails<B::Hash>, DispatchError>, Error>;
	fn best_hash(&self) -> B::Hash;
	fn finalized_hash(&self) -> B::Hash;
}

impl<B, C, AC> NotaryApisExt<B, AC> for C
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B>,
	C::Api: NotaryApis<B, NotaryRecordT>
		+ NotebookApis<B, NotebookVerifyError>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>,
	AC: Clone + Codec,
{
	fn notaries(&self, block_hash: B::Hash) -> Result<Vec<NotaryRecordT>, Error> {
		self.runtime_api().notaries(block_hash).map_err(Into::into)
	}
	fn latest_notebook_by_notary(
		&self,
		block_hash: B::Hash,
	) -> Result<BTreeMap<NotaryId, (NotebookNumber, Tick)>, Error> {
		self.runtime_api().latest_notebook_by_notary(block_hash).map_err(Into::into)
	}
	fn audit_notebook_and_get_votes(
		&self,
		block_hash: B::Hash,
		version: u32,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		notebook_tick: Tick,
		header_hash: H256,
		vote_minimums: &BTreeMap<B::Hash, Balance>,
		notebook: &[u8],
		notebook_dependencies: Vec<NotaryNotebookAuditSummary>,
	) -> Result<Result<NotaryNotebookRawVotes, NotebookVerifyError>, Error> {
		self.runtime_api()
			.audit_notebook_and_get_votes(
				block_hash,
				version,
				notary_id,
				notebook_number,
				notebook_tick,
				header_hash,
				vote_minimums,
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
}

pub fn run_notary_sync<B, C, AC>(
	task_manager: &TaskManager,
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	no_work_delay_millis: u64,
	notebook_downloader: NotebookDownloader,
	registry: Option<&Registry>,
	ticker: Ticker,
) -> Arc<NotaryClient<B, C, AC>>
where
	B: BlockT,
	C: ProvideRuntimeApi<B>
		+ BlockchainEvents<B>
		+ HeaderBackend<B>
		+ AuxStore
		+ Send
		+ Sync
		+ 'static,
	C::Api: NotebookApis<B, NotebookVerifyError>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ NotaryApis<B, NotaryRecordT>,
	AC: Codec + Clone + Send + Sync + 'static,
{
	let metrics = registry.and_then(|r| ConsensusMetrics::new(r).ok());
	let metrics = Arc::new(metrics);

	let notary_client = Arc::new(NotaryClient::new(
		client.clone(),
		aux_client.clone(),
		notebook_downloader,
		metrics,
		ticker,
	));

	let notary_client_clone = Arc::clone(&notary_client);
	let notary_client_poll = Arc::clone(&notary_client);
	let best_block = client.best_hash();
	let notary_sync_task = async move {
		notary_client_poll.update_notaries(&best_block).await.unwrap_or_else(|e| {
			warn!("Could not update notaries at best hash {} - {:?}", best_block, e)
		});

		let mut best_block = Box::pin(client.every_import_notification_stream());

		loop {
			tokio::select! {biased;
				notebook =  notary_client_poll.poll_subscriptions() => {
					if let Some((notary_id, notebook_number)) = notebook {
						trace!( "Next notebook pushed (notary {}, notebook {})", notary_id, notebook_number);
					}
				},
				block = best_block.next () => {
					if let Some(block) = block.as_ref() {
						if block.is_new_best {
							let best_hash = block.hash;
							if let Err(e) = notary_client_poll.update_notaries(&best_hash).await {
								warn!(

									"Could not update notaries at best hash {} - {:?}",
									best_hash,
									e
								);
							}
						}
					}
				},
			}
		}
	};

	let notary_queue_task = async move {
		let notary_client = notary_client_clone;
		loop {
			let has_more_work = notary_client
				.process_queues()
				.await
				.inspect_err(|err| {
					warn!("Error while processing notary queues: {:?}", err);
				})
				.unwrap_or(false);

			let mut delay = 20;
			if !has_more_work {
				delay = no_work_delay_millis
			}
			tokio::time::sleep(Duration::from_millis(delay)).await;
		}
	};
	let handle = task_manager.spawn_essential_handle();
	handle.spawn("notary_sync_task", "notary_sync", notary_sync_task);
	// Making this blocking due to the runtime calls and potentially heavy decodes
	handle.spawn_blocking("notary_queue_task", "notary_queue", notary_queue_task);

	notary_client
}

type PendingNotebook = (NotebookNumber, Option<SignedHeaderBytes>, Instant);

pub type VotingPowerInfo = (Tick, BlockVotingPower, u32);
pub struct NotaryClient<B: BlockT, C: AuxStore, AC> {
	client: Arc<C>,
	pub notary_client_by_id: Arc<RwLock<BTreeMap<NotaryId, Arc<Client>>>>,
	pub notary_archive_host_by_id: Arc<RwLock<BTreeMap<NotaryId, String>>>,
	pub notaries_by_id: Arc<RwLock<BTreeMap<NotaryId, NotaryRecordT>>>,
	pub subscriptions_by_id: Arc<RwLock<BTreeMap<NotaryId, Pin<Box<RawHeadersSubscription>>>>>,
	tick_voting_power_sender: Arc<Mutex<TracingUnboundedSender<VotingPowerInfo>>>,
	pub tick_voting_power_receiver: Arc<Mutex<TracingUnboundedReceiver<VotingPowerInfo>>>,
	notebook_queue_by_id: Arc<RwLock<BTreeMap<NotaryId, Vec<PendingNotebook>>>>,
	aux_client: ArgonAux<B, C>,
	notebook_downloader: NotebookDownloader,
	pub(crate) metrics: Arc<Option<ConsensusMetrics>>,
	ticker: Ticker,
	_block: PhantomData<AC>,
}

impl<B, C, AC> NotaryClient<B, C, AC>
where
	B: BlockT,
	C: NotaryApisExt<B, AC> + AuxStore + Send + Sync + 'static,
	AC: Clone + Codec + Send + Sync + 'static,
{
	pub fn new(
		client: Arc<C>,
		aux_client: ArgonAux<B, C>,
		notebook_downloader: NotebookDownloader,
		metrics: Arc<Option<ConsensusMetrics>>,
		ticker: Ticker,
	) -> Self {
		let (tick_voting_power_sender, tick_voting_power_receiver) =
			tracing_unbounded("node::consensus::notebook_tick_stream", 100);

		Self {
			client,
			subscriptions_by_id: Default::default(),
			notary_client_by_id: Default::default(),
			notary_archive_host_by_id: Default::default(),
			notaries_by_id: Default::default(),
			notebook_queue_by_id: Default::default(),
			tick_voting_power_sender: Arc::new(Mutex::new(tick_voting_power_sender)),
			tick_voting_power_receiver: Arc::new(Mutex::new(tick_voting_power_receiver)),
			aux_client,
			notebook_downloader,
			metrics,
			ticker,
			_block: PhantomData,
		}
	}

	pub async fn update_notaries(&self, block_hash: &B::Hash) -> Result<(), Error> {
		let notaries = self.client.notaries(*block_hash)?;
		let mut reconnect_ids = BTreeSet::new();

		{
			let next_notaries_by_id =
				notaries.iter().map(|n| (n.notary_id, n.clone())).collect::<BTreeMap<_, _>>();
			let mut notaries_by_id = self.notaries_by_id.write().await;
			if next_notaries_by_id != *notaries_by_id {
				for notary in &notaries {
					if let Some(existing) = notaries_by_id.get(&notary.notary_id) {
						if existing.meta.hosts[0] != notary.meta.hosts[0] {
							reconnect_ids.insert(notary.notary_id);
						}
					}
				}
				*notaries_by_id = next_notaries_by_id.clone();

				let existing_notary_ids =
					self.notary_client_by_id.read().await.keys().copied().collect::<Vec<_>>();
				for id in existing_notary_ids {
					if let Some(entry) = notaries_by_id.get(&id) {
						if Self::should_connect_to_notary(entry) {
							continue;
						}
					}
					self.disconnect(&id, None).await;
				}
			}
		}

		for notary in notaries {
			let notary_id = notary.notary_id;
			match notary.state {
				NotaryState::Locked { .. } => {
					// don't reconnect to a locked notary
					continue;
				},
				NotaryState::Reactivated { reprocess_notebook_number } => {
					self.aux_client.reprocess_notebook(notary_id, reprocess_notebook_number)?;
				},
				_ => {},
			}

			// don't connect if exceeded queue depth
			if self.queue_depth(notary_id).await > MAX_QUEUE_DEPTH {
				continue;
			}

			let is_connected =
				self.has_client(notary_id).await && self.has_subscription(notary_id).await;

			if !is_connected || reconnect_ids.contains(&notary_id) {
				info!("Connecting to notary id={}", notary_id);
				if let Err(e) = self.sync_notebooks(notary_id).await {
					self.disconnect(
						&notary_id,
						Some(format!("Notary {} sync failed. {:?}", notary_id, e)),
					)
					.await;
					continue;
				}

				if let Err(e) = self.subscribe_to_notebooks(notary_id).await {
					self.disconnect(
						&notary_id,
						Some(format!("Notary {} subscription failed. {:?}", notary_id, e)),
					)
					.await;
				}
			}
		}

		Ok(())
	}

	pub async fn poll_subscriptions(&self) -> Option<(NotaryId, NotebookNumber)> {
		let result = {
			let mut subscriptions = self.subscriptions_by_id.write().await;
			if subscriptions.is_empty() {
				return None;
			}

			let futures = subscriptions.iter_mut().map(|(notary_id, sub)| {
				let notary_id = *notary_id;
				Box::pin(async move {
					match futures::future::poll_fn(|cx| sub.as_mut().poll_next(cx)).await {
						Some(Ok(download_info)) => Ok((notary_id, download_info)),
						Some(Err(e)) => Err((notary_id, Some(e.to_string()))),
						None => Err((notary_id, None)), // Subscription ended
					}
				})
			});

			let (result, _, _) = futures::future::select_all(futures).await;
			result
		};

		match result {
			Ok((notary_id, download_info)) => {
				let notebook_number = download_info.notebook_number;
				if let Some(metrics) = self.metrics.as_ref() {
					metrics.notebook_notification_received(
						notary_id,
						download_info.tick,
						&self.ticker,
					);
				}
				if let Ok(did_overflow) = self
					.download_header(
						notary_id,
						notebook_number,
						Some(download_info.header_download_url),
					)
					.await
				{
					if did_overflow {
						info!("Overflowed queue for notary {}", notary_id);
						self.unsubscribe_if_overflowed(notary_id).await;
					}
				}
				Some((notary_id, notebook_number))
			},
			Err((notary_id, reason)) => {
				self.disconnect(&notary_id, reason).await;
				None
			},
		}
	}

	pub async fn process_queues(self: &Arc<Self>) -> Result<bool, Error> {
		let finalized_hash = self.client.finalized_hash();
		let best_hash = self.client.best_hash();
		let queued_notaries =
			self.notebook_queue_by_id.read().await.keys().cloned().collect::<Vec<_>>();

		let handles = queued_notaries.into_iter().map(|notary_id| {
			let self_clone: Arc<Self> = Arc::clone(self);
			tokio::spawn(async move {
				self_clone.clean_queued_finalized_notebooks(&finalized_hash, notary_id).await;
				self_clone.retrieve_next_notary_missing_notebooks(notary_id, 10).await?;

				let Some((notebook_number, raw_header, time)) =
					self_clone.dequeue_ready(notary_id).await
				else {
					return Ok::<_, Error>(false);
				};
				match self_clone
					.process_notebook(
						notary_id,
						notebook_number,
						&finalized_hash,
						&best_hash,
						raw_header.clone(),
						time,
					)
					.await
				{
					Ok(()) => {
						let has_more_work = self_clone.queue_depth(notary_id).await > 0;
						trace!(
							"Processed notebook {} for notary {}. More work? {}",
							notebook_number,
							notary_id,
							has_more_work
						);
						Ok::<_, Error>(has_more_work)
					},
					Err(e) => {
						if let Error::MissingNotebooksError(_) = e {
							self_clone
								.enqueue_notebook(
									notary_id,
									notebook_number,
									Some(raw_header),
									Some(time),
								)
								.await?;
							return Ok::<_, Error>(true);
						}
						Err(e)
					},
				}
			})
		});
		let results = join_all(handles).await;

		let mut has_more_work = false;
		for result in results {
			match result {
				Ok(inner_result) => match inner_result {
					Ok(x) => has_more_work = has_more_work || x,
					Err(err) => warn!("Error processing notebooks for a notary {:?}", err),
				},
				Err(join_error) => {
					warn!("Error while processing notary queue - {:?}", join_error);
				},
			}
		}
		self.log_queue_depth().await;
		Ok(has_more_work)
	}

	async fn clean_queued_finalized_notebooks(
		&self,
		finalized_hash: &B::Hash,
		notary_id: NotaryId,
	) {
		let finalized_notebook_number = self.latest_notebook_in_runtime(*finalized_hash, notary_id);

		let mut notary_queue = self.notebook_queue_by_id.write().await;
		if let Some(queue) = notary_queue.get_mut(&notary_id) {
			queue.retain(|(notebook_number, _, _)| *notebook_number > finalized_notebook_number);
		}
	}

	async fn retrieve_next_notary_missing_notebooks(
		&self,
		notary_id: NotaryId,
		next_batch: u16,
	) -> Result<(), Error> {
		let mut missing_notebooks = vec![];
		let mut count = 0;
		// use notebook lock in block
		{
			let notary_queue = self.notebook_queue_by_id.read().await;
			if let Some(notary_queue) = notary_queue.get(&notary_id) {
				for (notebook_number, raw_header, _) in notary_queue {
					if raw_header.is_none() && missing_notebooks.len() < next_batch as usize {
						missing_notebooks.push(*notebook_number);
					}
					count += 1;
				}
			}
		}
		if missing_notebooks.is_empty() {
			trace!("No missing notebooks for notary {}", notary_id);
			return Ok(());
		}

		info!(
			"Retrieving missing notebooks from notary #{} - {:?} (of {count})",
			notary_id, missing_notebooks
		);
		for notebook_number in missing_notebooks {
			self.download_header(notary_id, notebook_number, None).await?;
		}

		Ok(())
	}

	async fn log_queue_depth(&self) {
		if let Some(metrics) = self.metrics.as_ref() {
			let notary_queue = self.notebook_queue_by_id.read().await;
			for (notary_id, queue) in notary_queue.iter() {
				metrics.record_queue_depth(*notary_id, queue.len() as u64);
			}
		}
	}

	async fn queue_depth(&self, notary_id: NotaryId) -> usize {
		let notary_queue = self.notebook_queue_by_id.read().await;
		notary_queue.get(&notary_id).map_or(0usize, |q| q.len())
	}

	async fn dequeue_ready(
		&self,
		notary_id: NotaryId,
	) -> Option<(NotebookNumber, SignedHeaderBytes, Instant)> {
		let mut queues = self.notebook_queue_by_id.write().await;
		let queue = queues.get_mut(&notary_id)?;
		if queue.is_empty() {
			return None;
		}

		if tracing::enabled!(tracing::Level::TRACE) {
			trace!(
				"Dequeuing notebook for notary {}. Queue: {:?}",
				notary_id,
				queue.iter().map(|(n, h, _)| (n, h.is_some())).collect::<Vec<_>>()
			);
		}
		if let Some((_, Some(_), _)) = queue.first() {
			let (notebook_number, raw_header, time) = queue.remove(0);
			return Some((notebook_number, raw_header?, time))
		}
		None
	}

	/// Enqueue the notebook and return true if the queue overflowed
	async fn enqueue_notebook(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		header_bytes: Option<SignedHeaderBytes>,
		enqueue_time: Option<Instant>,
	) -> Result<bool, Error> {
		let mut notebook_queue_by_id = self.notebook_queue_by_id.write().await;

		let queue = notebook_queue_by_id.entry(notary_id).or_insert_with(Vec::new);
		let entry = queue.iter().position(|(n, _, _)| *n == notebook_number);
		if let Some(index) = entry {
			// only overwrite if missing
			if queue[index].1.is_none() {
				trace!(
					"Overwriting notebook {} header in queue for notary {} with header? {}",
					notebook_number,
					notary_id,
					header_bytes.is_some()
				);
				queue[index].1 = header_bytes;
			}
			Ok(false)
		} else {
			trace!(
				"Queuing notebook {} for notary {} with header? {}",
				notebook_number,
				notary_id,
				header_bytes.is_some()
			);
			// look from back of list since we're normally appending
			let pos = queue
				.iter()
				.rposition(|(n, _, _)| *n < notebook_number)
				.map(|p| p + 1)
				.unwrap_or(0);
			queue.insert(
				pos,
				(notebook_number, header_bytes, enqueue_time.unwrap_or(Instant::now())),
			);

			Ok(queue.len() > MAX_QUEUE_DEPTH)
		}
	}

	async fn unsubscribe_if_overflowed(&self, notary_id: NotaryId) {
		if self.queue_depth(notary_id).await <= MAX_QUEUE_DEPTH {
			return;
		}

		self.subscriptions_by_id.write().await.remove(&notary_id);
	}

	async fn download_header(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		mut download_url: Option<String>,
	) -> Result<bool, Error> {
		if download_url.is_none() {
			if let Some(archive_host) = self.notary_archive_host_by_id.read().await.get(&notary_id)
			{
				download_url = Some(get_header_url(archive_host, notary_id, notebook_number));
			}
		}
		let header = self
			.notebook_downloader
			.get_header(notary_id, notebook_number, download_url)
			.await
			.map_err(|e| {
				Error::NotaryError(format!(
					"Could not get notary {notary_id}, notebook {notebook_number} from notebook downloader - {:?}",
					e
				))
			})?;

		self.enqueue_notebook(notary_id, notebook_number, Some(header), None).await
	}

	async fn download_notebook(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
	) -> Result<NotebookBytes, Error> {
		let download_url = if let Some(archive_host) =
			self.notary_archive_host_by_id.read().await.get(&notary_id)
		{
			Some(get_notebook_url(archive_host, notary_id, notebook_number))
		} else {
			let client = self.get_or_connect_to_client(notary_id).await.ok();
			if let Some(client) = client {
				client.get_notebook_download_url(notebook_number).await.ok()
			} else {
				None
			}
		};
		let bytes = self
			.notebook_downloader
			.get_body(notary_id, notebook_number, download_url)
			.await
			.map_err(|e| {
				Error::NotaryError(format!(
					"Could not download notebook {} from notary {} - {:?}",
					notebook_number, notary_id, e
				))
			})?;
		Ok(bytes)
	}

	async fn sync_notebooks(&self, id: NotaryId) -> Result<(), Error> {
		let client = self.get_or_connect_to_client(id).await?;
		let notebook_meta = client.metadata().await.map_err(|e| {
			Error::NotaryError(format!("Could not get metadata from notary - {:?}", e))
		})?;
		let notary_notebooks = self.aux_client.get_notary_audit_history(id)?.get();
		let latest_stored = notary_notebooks.last().map(|n| n.notebook_number).unwrap_or_default();
		let archive_host = client.get_archive_base_url().await.map_err(|e| {
			Error::NotaryError(format!("Could not get archive host from notary - {:?}", e))
		})?;
		self.notary_archive_host_by_id.write().await.insert(id, archive_host.clone());

		if latest_stored < notebook_meta.finalized_notebook_number {
			let start = latest_stored + 1;
			for i in start..=notebook_meta.finalized_notebook_number {
				self.download_header(id, i, None).await?;
			}
		}

		Ok(())
	}

	pub async fn disconnect(&self, notary_id: &NotaryId, reason: Option<String>) {
		info!(
			"Notary client disconnected from notary #{} (or could not connect). Reason? {:?}",
			notary_id, reason
		);
		self.notary_client_by_id.write().await.remove(notary_id);
		self.subscriptions_by_id.write().await.remove(notary_id);
	}

	async fn subscribe_to_notebooks(&self, id: NotaryId) -> Result<(), Error> {
		let client = self.get_or_connect_to_client(id).await?;
		let stream: RawHeadersSubscription = client.subscribe_headers().await.map_err(|e| {
			Error::NotaryError(format!("Could not subscribe to notebooks from notary - {:?}", e))
		})?;
		self.subscriptions_by_id.write().await.insert(id, Box::pin(stream));
		Ok(())
	}

	pub async fn process_notebook(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		finalized_hash: &B::Hash,
		best_hash: &B::Hash,
		raw_header: SignedHeaderBytes,
		enqueue_time: Instant,
	) -> Result<(), Error> {
		// if we have a good notebook with this number, don't re-audit
		if self.aux_client.has_successful_audit(notary_id, notebook_number) {
			return Ok(())
		}
		let finalized_notebook_number = self.latest_notebook_in_runtime(*finalized_hash, notary_id);
		if notebook_number <= finalized_notebook_number {
			info!(

				"Skipping audit of finalized notebook. Notary {notary_id}, #{notebook_number}, finalized #{finalized_notebook_number}.",
			);
			return Ok(());
		}

		let notebook_details = self
			.client
			.decode_signed_raw_notebook_header(best_hash, raw_header.0.clone())?
			.map_err(|e| {
				Error::NotaryError(format!(
					"Unable to decode notebook header in runtime. Notary={}, notebook={} -> {:?}",
					notary_id, notebook_number, e
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

		let audit_result = self.audit_notebook(best_hash, &notebook_details).await?;

		let voting_power = self.aux_client.store_notebook_result(
			audit_result,
			raw_header,
			notebook_details,
			finalized_notebook_number,
		)?;

		if let Some(metrics) = self.metrics.as_ref() {
			metrics.notebook_processed(notary_id, tick, enqueue_time, &self.ticker);
		}

		self.tick_voting_power_sender.lock().await.unbounded_send(voting_power).map_err(|e| {
			Error::NotaryError(format!("Could not send tick state to sender (notary {notary_id}, notebook {notebook_number}) - {:?}", e))
		})?;
		Ok(())
	}

	pub async fn verify_notebook_audits(
		self: &Arc<Self>,
		parent_hash: &B::Hash,
		notebook_audit_results: Vec<NotebookAuditResult<NotebookVerifyError>>,
	) -> Result<(), Error> {
		for _ in 0..2 {
			let mut missing_audits_by_notary = BTreeSet::new();
			let notary_ids = self.get_notary_ids().await;
			let mut needs_notary_updates = false;
			for digest_record in &notebook_audit_results {
				let notary_audits =
					self.aux_client.get_notary_audit_history(digest_record.notary_id)?.get();

				let audit = notary_audits
					.iter()
					.find(|a| a.notebook_number == digest_record.notebook_number);

				if let Some(audit) = audit {
					if digest_record.audit_first_failure != audit.audit_first_failure {
						return Err(Error::InvalidNotebookDigest(format!(
							"Notary {}, notebook #{} has an audit mismatch \"{:?}\" with local result. \"{:?}\"",
							digest_record.notary_id, digest_record.notebook_number, digest_record.audit_first_failure, audit.audit_first_failure
						)));
					}
				} else {
					if !notary_ids.contains(&digest_record.notary_id) {
						needs_notary_updates = true;
					}
					self.enqueue_notebook(
						digest_record.notary_id,
						digest_record.notebook_number,
						None,
						None,
					)
					.await?;
					missing_audits_by_notary
						.insert((digest_record.notary_id, digest_record.notebook_number));
				}
			}
			if missing_audits_by_notary.is_empty() {
				return Ok(());
			}

			info!(
				"Notebook digest has missing audits. Will attempt to catchup now. {:#?}",
				missing_audits_by_notary
			);

			if needs_notary_updates {
				self.update_notaries(parent_hash).await?;
			}

			// drain queues
			// NOTE: only do this for 10 seconds
			let start = std::time::Instant::now();
			while self.process_queues().await? {
				tokio::time::sleep(Duration::from_millis(30)).await;
				if start.elapsed() > Duration::from_secs(10) {
					return Err(Error::UnableToSyncNotary(
						"Could not process all missing audits in 10 seconds".to_string(),
					));
				}
			}
		}
		Err(Error::InvalidNotebookDigest(
			"Notebook digest record could not verify all records in local storage".to_string(),
		))
	}

	async fn get_notebook_dependencies(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		best_hash: &B::Hash,
	) -> Result<Vec<NotaryNotebookAuditSummary>, Error> {
		let mut notebook_dependencies = vec![];
		let mut missing_notebooks = vec![];

		let latest_block_notebook = self.latest_notebook_in_runtime(*best_hash, notary_id);

		// get any missing notebooks
		if latest_block_notebook < notebook_number - 1 {
			let notary_notebooks = self.aux_client.get_audit_summaries(notary_id)?.get();
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
			for missing in &missing_notebooks {
				self.enqueue_notebook(notary_id, *missing, None, None).await?;
			}
			return Err(Error::MissingNotebooksError(format!(
				"Missing notebooks #{:?} to audit {} for notary {}",
				missing_notebooks, notebook_number, notary_id
			)));
		}
		Ok(notebook_dependencies)
	}

	async fn audit_notebook(
		&self,
		best_hash: &B::Hash,
		notebook_details: &NotaryNotebookDetails<B::Hash>,
	) -> Result<NotebookAuditResult<NotebookVerifyError>, Error> {
		let tick = notebook_details.tick;
		let notary_id = notebook_details.notary_id;
		let notebook_number = notebook_details.notebook_number;
		let notebook_dependencies =
			self.get_notebook_dependencies(notary_id, notebook_number, best_hash).await?;
		trace!(
			"Attempting to audit notebook. Notary {notary_id}, #{notebook_number}, tick {tick}.",
		);

		let mut vote_minimums = BTreeMap::new();
		for block_hash in &notebook_details.blocks_with_votes {
			vote_minimums.insert(
				*block_hash,
				self.client.vote_minimum(*block_hash).map_err(|e| {
					let message = format!(
						"Error getting vote minimums for block {}. Notary {}, notebook {}. {:?}",
						block_hash, notary_id, notebook_number, e
					);
					Error::StringError(message)
				})?,
			);
		}

		let full_notebook = self.download_notebook(notary_id, notebook_number).await?;
		trace!(
			"Notebook downloaded. Notary {notary_id}, #{notebook_number}, tick {tick}. {} bytes.",
			full_notebook.0.len()
		);

		// audit on the best block since we're adding dependencies
		let audit_failure_reason = match self.client.audit_notebook_and_get_votes(
			*best_hash,
			notebook_details.version,
			notary_id,
			notebook_number,
			tick,
			notebook_details.header_hash,
			&vote_minimums,
			&full_notebook.0,
			notebook_dependencies,
		)? {
			Ok(votes) => {
				let vote_count = votes.raw_votes.len();
				self.aux_client.store_votes(tick, votes)?;

				info!(

					"Notebook audit successful. Notary {notary_id}, #{notebook_number}, tick {tick}. {vote_count} block vote(s).",
				);
				None
			},
			Err(error) => {
				warn!(

					"Notebook audit failed ({}). Notary {notary_id}, #{notebook_number}, tick {tick}.",
					error
				);

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

	fn should_connect_to_notary(notary_record: &NotaryRecordT) -> bool {
		!matches!(notary_record.state, NotaryState::Locked { .. })
	}

	async fn get_notary_ids(&self) -> Vec<NotaryId> {
		self.notaries_by_id.read().await.keys().copied().collect()
	}

	async fn get_notary_host(&self, notary_id: NotaryId) -> Result<String, Error> {
		let notaries = self.notaries_by_id.read().await;
		let record = notaries
			.get(&notary_id)
			.ok_or_else(|| Error::NotaryError("No rpc endpoints found for notary".to_string()))?;
		let host =
			record.meta.hosts.first().ok_or_else(|| {
				Error::NotaryError("No rpc endpoint found for notary".to_string())
			})?;
		host.clone().try_into().map_err(|e| {
			Error::NotaryError(format!(
				"Could not convert host to string for notary {} - {:?}",
				notary_id, e
			))
		})
	}

	async fn get_or_connect_to_client(
		&self,
		notary_id: NotaryId,
	) -> Result<Arc<argon_notary_apis::Client>, Error> {
		if let std::collections::btree_map::Entry::Vacant(e) =
			self.notary_client_by_id.write().await.entry(notary_id)
		{
			let host_str = self.get_notary_host(notary_id).await?;
			let c = argon_notary_apis::create_client(&host_str).await.map_err(|e| {
				Error::NotaryError(format!(
					"Could not connect to notary {} ({}) for audit - {:?}",
					notary_id, host_str, e
				))
			})?;
			let c = Arc::new(c);
			e.insert(c.clone());
		}
		self.get_client(notary_id)
			.await
			.ok_or_else(|| Error::NotaryError("Could not connect to notary for audit".to_string()))
	}

	async fn has_client(&self, notary_id: NotaryId) -> bool {
		self.notary_client_by_id.read().await.contains_key(&notary_id)
	}

	async fn has_subscription(&self, notary_id: NotaryId) -> bool {
		self.subscriptions_by_id.read().await.contains_key(&notary_id)
	}

	async fn get_client(&self, notary_id: NotaryId) -> Option<Arc<argon_notary_apis::Client>> {
		self.notary_client_by_id.read().await.get(&notary_id).cloned()
	}

	fn latest_notebook_in_runtime(
		&self,
		block_hash: B::Hash,
		notary_id: NotaryId,
	) -> NotebookNumber {
		if let Ok(latest_notebooks_in_runtime) = self.client.latest_notebook_by_notary(block_hash) {
			if let Some((latest_notebook, _)) = latest_notebooks_in_runtime.get(&notary_id) {
				return *latest_notebook;
			}
		}
		0
	}
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

	let notaries = client.runtime_api().notaries(*parent_hash)?;
	for notary in notaries {
		if matches!(notary.state, NotaryState::Locked { .. }) {
			continue;
		}
		let (latest_runtime_notebook_number, _) =
			latest_notebooks_in_runtime.get(&notary.notary_id).unwrap_or(&(0, 0));
		let Ok((mut notary_headers, tick_notebook)) = aux_client.get_notary_notebooks_for_header(
			notary.notary_id,
			*latest_runtime_notebook_number,
			voting_schedule,
		) else {
			continue;
		};

		headers.signed_headers.append(&mut notary_headers.signed_headers);
		headers
			.notebook_digest
			.notebooks
			.append(&mut notary_headers.notebook_digest.notebooks);
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
}

impl NotebookDownloader {
	pub fn new<AR, S>(archive_hosts: AR) -> Result<Self, Error>
	where
		AR: IntoIterator<Item = S>,
		S: AsRef<str>,
	{
		let archive_hosts = archive_hosts
			.into_iter()
			.map(|host| ArchiveHost::new(host.as_ref().to_string()))
			.collect::<Result<Vec<_>, _>>()
			.map_err(|e| Error::NotaryArchiveError(e.to_string()))?;
		Ok(Self { archive_hosts })
	}

	pub async fn get_header(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		download_url: Option<String>,
	) -> Result<SignedHeaderBytes, Error> {
		if let Some(url) = download_url {
			if let Ok(header) = ArchiveHost::download_header_bytes(url).await {
				return Ok(header);
			}
		}
		for archive_host in &self.archive_hosts {
			if let Ok(header) = archive_host.get_header(notary_id, notebook_number).await {
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
	) -> Result<NotebookBytes, Error> {
		if let Some(url) = download_url {
			if let Ok(body) = ArchiveHost::download_notebook_bytes(url).await {
				return Ok(body);
			}
		}
		for archive_host in &self.archive_hosts {
			if let Ok(body) = archive_host.get_notebook(notary_id, notebook_number).await {
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
	use argon_notary_apis::notebook::NotebookRpcServer;
	use argon_primitives::{
		notary::{
			NotaryMeta, NotaryNotebookAuditSummary, NotaryNotebookAuditSummaryDetails,
			NotaryNotebookRawVotes, NotaryRecordWithState,
		},
		AccountId, Balance, ChainTransfer, NotaryId, NotebookHeader, NotebookMeta, NotebookNumber,
	};
	use argon_runtime::Block;
	use codec::{Decode, Encode};

	use crate::mock_notary::setup_logs;
	use sp_core::{bounded_vec, H256};
	use sp_keyring::Ed25519Keyring;
	use std::collections::BTreeMap;

	#[derive(Clone, Default)]
	struct TestNode {
		pub aux: Arc<parking_lot::Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
		pub notaries: Arc<parking_lot::Mutex<Vec<NotaryRecordT>>>,
		pub latest_notebook_by_notary:
			Arc<parking_lot::Mutex<BTreeMap<NotaryId, (NotebookNumber, Tick)>>>,
		pub audit_dependencies: Arc<parking_lot::Mutex<Vec<NotaryNotebookAuditSummary>>>,
		#[allow(clippy::type_complexity)]
		pub notebook_audit_votes: Arc<parking_lot::Mutex<Option<Vec<(Vec<u8>, BlockVotingPower)>>>>,
	}

	impl TestNode {
		fn new() -> Self {
			Self::default()
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
		fn notaries(&self, _block_hash: H256) -> Result<Vec<NotaryRecordT>, Error> {
			Ok(self.notaries.lock().clone())
		}
		fn latest_notebook_by_notary(
			&self,
			_block_hash: <Block as BlockT>::Hash,
		) -> Result<BTreeMap<NotaryId, (NotebookNumber, Tick)>, Error> {
			Ok(self.latest_notebook_by_notary.lock().clone())
		}
		fn audit_notebook_and_get_votes(
			&self,
			_block_hash: <Block as BlockT>::Hash,
			_version: u32,
			notary_id: NotaryId,
			notebook_number: NotebookNumber,
			_tick: Tick,
			_header_hash: H256,
			_vote_minimums: &BTreeMap<<Block as BlockT>::Hash, Balance>,
			_notebook: &[u8],
			notebook_dependencies: Vec<NotaryNotebookAuditSummary>,
		) -> Result<Result<NotaryNotebookRawVotes, NotebookVerifyError>, Error> {
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
			_block_hash: &<Block as BlockT>::Hash,
			raw_header: Vec<u8>,
		) -> Result<Result<NotaryNotebookDetails<<Block as BlockT>::Hash>, DispatchError>, Error>
		{
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
			H256::from_slice(&[1; 32])
		}
		fn finalized_hash(&self) -> <Block as BlockT>::Hash {
			H256::from_slice(&[0; 32])
		}
	}

	async fn system() -> (MockNotary, Arc<TestNode>, Arc<NotaryClient<Block, TestNode, AccountId>>)
	{
		let mut test_notary = MockNotary::new(1);
		test_notary.start().await.expect("could not start notary");
		test_notary.state.lock().await.metadata =
			Some(NotebookMeta { finalized_notebook_number: 0, finalized_tick: 0 });
		let archive_host = test_notary.archive_host.clone();

		let client = Arc::new(TestNode::new());
		client.add_notary(&test_notary);
		let aux_client = ArgonAux::new(client.clone());

		let ticker = Ticker::new(2000, 2);

		let notebook_downloader = NotebookDownloader::new(vec![archive_host]).unwrap();
		let notary_client = NotaryClient::new(
			client.clone(),
			aux_client,
			notebook_downloader,
			Arc::new(None),
			ticker,
		);
		let notary_client = Arc::new(notary_client);
		(test_notary, client, notary_client)
	}

	#[tokio::test]
	async fn adds_new_notaries() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert_eq!(notary_client.notaries_by_id.read().await.len(), 1);
		assert_eq!(notary_client.notary_client_by_id.read().await.len(), 1);
		assert_eq!(notary_client.subscriptions_by_id.read().await.len(), 1);

		test_notary.create_notebook_header(vec![]).await;
		let next = notary_client.poll_subscriptions().await;
		assert!(next.is_some());
		assert_eq!(next.unwrap().0, 1);

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
		assert_eq!(notary_client.notaries_by_id.read().await.len(), 1);
		assert_eq!(notary_client.notary_client_by_id.read().await.len(), 0);
		assert_eq!(notary_client.subscriptions_by_id.read().await.len(), 0);
		test_notary.create_notebook_header(vec![]).await;
		let next = notary_client.poll_subscriptions().await;
		assert!(next.is_none());
	}

	#[tokio::test]
	async fn wont_reconnect_if_queue_depth_exceeded() {
		setup_logs();
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert_eq!(notary_client.notaries_by_id.read().await.len(), 1);
		assert_eq!(notary_client.notary_client_by_id.read().await.len(), 1);
		for i in 0..MAX_QUEUE_DEPTH {
			test_notary.create_notebook_header(vec![]).await;
			notary_client.poll_subscriptions().await;
			assert_eq!(notary_client.queue_depth(1).await, i + 1);
		}

		assert_eq!(notary_client.notebook_queue_by_id.read().await.len(), 1);

		assert_eq!(notary_client.queue_depth(1).await, MAX_QUEUE_DEPTH);
		assert_eq!(notary_client.notary_client_by_id.read().await.len(), 1);
		assert_eq!(notary_client.subscriptions_by_id.read().await.len(), 1);

		test_notary.create_notebook_header(vec![]).await;
		notary_client.poll_subscriptions().await;
		assert_eq!(notary_client.queue_depth(1).await, MAX_QUEUE_DEPTH + 1);
		// should have disconnected subscriptions, but kept notary client
		assert_eq!(notary_client.notary_client_by_id.read().await.len(), 1);
		assert_eq!(notary_client.subscriptions_by_id.read().await.len(), 0);
	}

	#[tokio::test]
	async fn handles_queueing_correctly() {
		let (_test_notary, _client, notary_client) = system().await;
		notary_client
			.enqueue_notebook(1, 3, None, None)
			.await
			.expect("Could not enqueue");
		notary_client
			.enqueue_notebook(1, 1, None, None)
			.await
			.expect("Could not enqueue");
		notary_client
			.enqueue_notebook(1, 2, None, None)
			.await
			.expect("Could not enqueue");
		assert_eq!(
			notary_client
				.notebook_queue_by_id
				.read()
				.await
				.get(&1)
				.unwrap()
				.iter()
				.map(|(n, a, _)| (*n, a.is_some()))
				.collect::<Vec<_>>(),
			vec![(1, false), (2, false), (3, false)]
		);
		let next = notary_client.dequeue_ready(1).await;
		assert!(next.is_none());

		notary_client
			.enqueue_notebook(1, 2, Some(Default::default()), None)
			.await
			.expect("Could not enqueue");
		let next = notary_client.dequeue_ready(1).await;
		assert!(next.is_none());
		notary_client
			.enqueue_notebook(1, 1, Some(Default::default()), None)
			.await
			.expect("Could not enqueue");

		assert_eq!(
			notary_client
				.notebook_queue_by_id
				.read()
				.await
				.get(&1)
				.unwrap()
				.iter()
				.map(|(n, a, _)| (*n, a.is_some()))
				.collect::<Vec<_>>(),
			vec![(1, true), (2, true), (3, false)]
		);

		notary_client
			.enqueue_notebook(1, 1, None, None)
			.await
			.expect("Could not enqueue");
		assert_eq!(
			notary_client
				.notebook_queue_by_id
				.read()
				.await
				.get(&1)
				.unwrap()
				.iter()
				.map(|(n, a, _)| (*n, a.is_some()))
				.collect::<Vec<_>>(),
			vec![(1, true), (2, true), (3, false)]
		);
		let next = notary_client.dequeue_ready(1).await;
		assert!(next.is_some());
	}

	#[tokio::test]
	async fn downloads_missing_audit_notebooks() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		let notebooks = (1..=11u32).map(|n| (n, false)).collect::<Vec<_>>();
		for (n, _) in &notebooks {
			notary_client
				.enqueue_notebook(1, *n, None, None)
				.await
				.expect("Could not enqueue");
		}
		assert_eq!(
			notary_client
				.notebook_queue_by_id
				.read()
				.await
				.get(&1)
				.unwrap()
				.iter()
				.map(|(n, x, _)| { (*n, x.is_some()) })
				.collect::<Vec<_>>(),
			notebooks
		);
		for _ in 1..=11 {
			test_notary.create_notebook_header(vec![]).await;
		}
		assert_eq!(
			test_notary.metadata().await.expect(""),
			NotebookMeta { finalized_notebook_number: 11, finalized_tick: 11 }
		);
		notary_client
			.retrieve_next_notary_missing_notebooks(1, 10)
			.await
			.expect("Could not retrieve missing notebooks");
		let pending_notebooks = {
			let queue = notary_client.notebook_queue_by_id.read().await;
			queue.get(&1).expect("No queue").clone()
		};

		assert_eq!(
			pending_notebooks.iter().map(|(n, a, _)| (*n, a.is_some())).collect::<Vec<_>>(),
			vec![
				(1, true),
				(2, true),
				(3, true),
				(4, true),
				(5, true),
				(6, true),
				(7, true),
				(8, true),
				(9, true),
				(10, true),
				(11, false),
			],
			"should download the first 10"
		);
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
			)
			.await
			.expect("Could not retrieve missing notebooks");
	}

	#[tokio::test]
	async fn supplies_missing_notebooks_on_audit() {
		let (test_notary, client, notary_client) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");

		let result = notary_client
			.get_notebook_dependencies(test_notary.notary_id, 10, &client.best_hash())
			.await
			.expect_err("Should not have all dependencies");
		assert!(matches!(result, Error::MissingNotebooksError(_)),);
		assert_eq!(
			notary_client
				.notebook_queue_by_id
				.read()
				.await
				.get(&1)
				.unwrap()
				.iter()
				.map(|(n, a, _)| (*n, a.is_some()))
				.collect::<Vec<_>>(),
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
			.get_notebook_dependencies(test_notary.notary_id, 10, &client.best_hash())
			.await
			.expect_err("Should have all dependencies");

		// still missing number 9
		assert!(matches!(result, Error::MissingNotebooksError(_)),);
		assert!(result.to_string().contains("[9]"));

		for _ in 0..9 {
			notary_client.process_queues().await.expect("Could not process queues");
		}
		assert_eq!(
			notary_client
				.notebook_queue_by_id
				.read()
				.await
				.get(&1)
				.unwrap()
				.iter()
				.map(|(n, a, _)| { (*n, a.is_some()) })
				.collect::<Vec<_>>(),
			vec![(9, false)]
		);
		for _ in 0..10 {
			test_notary.create_notebook_header(vec![]).await;
			notary_client.process_queues().await.expect("Could not process queues");
		}
		let mut rx = notary_client.tick_voting_power_receiver.lock().await;
		let next_rx = rx.next().await.expect("Could not receive");
		assert_eq!(next_rx.0, 9);
		assert_eq!(notary_client.notebook_queue_by_id.read().await.get(&1).unwrap(), &vec![]);
		let result = notary_client
			.get_notebook_dependencies(test_notary.notary_id, 10, &client.best_hash())
			.await
			.expect("Could not retrieve missing notebooks");
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].notebook_number, 9);
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
		notary_client.poll_subscriptions().await;
		assert_eq!(notary_client.queue_depth(1).await, 1);
		assert!(notary_client.notebook_queue_by_id.read().await.get(&2).is_none());
		notary_client.process_queues().await.expect("Could not process queues");
		assert_eq!(notary_client.queue_depth(1).await, 0);
		assert!(notary_client.notebook_queue_by_id.read().await.get(&2).is_none());

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
		assert_eq!(notary_client.notary_client_by_id.read().await.len(), 2);
		assert_eq!(notary_client.subscriptions_by_id.read().await.len(), 2);

		test_notary.create_notebook_header(vec![]).await;
		test_notary2.create_notebook_header(vec![]).await;
		notary_client.poll_subscriptions().await;
		notary_client.poll_subscriptions().await;

		assert_eq!(notary_client.queue_depth(1).await, 1);
		assert_eq!(notary_client.notebook_queue_by_id.read().await.get(&1).unwrap()[0].0, 2);
		assert_eq!(notary_client.notebook_queue_by_id.read().await.get(&2).unwrap().len(), 2);
		assert_eq!(notary_client.notebook_queue_by_id.read().await.get(&2).unwrap()[0].0, 1);
		// should process one from each notary
		notary_client.process_queues().await.expect("Could not process queues");
		assert_eq!(notary_client.queue_depth(1).await, 0);
		assert_eq!(notary_client.notebook_queue_by_id.read().await.get(&2).unwrap().len(), 1);
	}
}
