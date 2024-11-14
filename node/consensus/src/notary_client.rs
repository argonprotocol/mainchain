use codec::Codec;
use futures::{future::join_all, Stream, StreamExt};
use log::{info, trace, warn};
use sc_client_api::{AuxStore, BlockchainEvents};
use sc_service::TaskManager;
use sc_utils::mpsc::TracingUnboundedSender;
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
	time::Duration,
};
use tokio::sync::Mutex;

use crate::{aux_client::ArgonAux, error::Error};
use argon_node_runtime::{NotaryRecordT, NotebookVerifyError};
use argon_notary_apis::notebook::{NotebookRpcClient, RawHeadersSubscription};
use argon_primitives::{
	ensure,
	notary::{
		NotaryNotebookAuditSummary, NotaryNotebookDetails, NotaryNotebookRawVotes, NotaryState,
	},
	notebook::NotebookNumber,
	tick::Tick,
	Balance, BlockSealApis, BlockSealAuthorityId, BlockVotingPower, NotaryApis, NotaryId,
	NotebookApis, NotebookAuditResult, NotebookHeaderData, VoteMinimum, VotingSchedule,
};

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
	fn best_hash(&self) -> B::Hash {
		self.info().best_hash
	}
	fn finalized_hash(&self) -> B::Hash {
		self.info().finalized_hash
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
	fn audit_notebook_and_get_votes(
		&self,
		block_hash: B::Hash,
		version: u32,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
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
}

pub fn run_notary_sync<B, C, AC>(
	task_manager: &TaskManager,
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	notebook_tick_tx: TracingUnboundedSender<VotingPowerInfo>,
	no_work_delay_millis: u64,
) where
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
	let notary_client =
		Arc::new(NotaryClient::new(client.clone(), aux_client.clone(), notebook_tick_tx));

	let notary_client_clone = Arc::clone(&notary_client);
	let notary_sync_task = async move {
		let mut best_block = Box::pin(client.import_notification_stream());

		loop {
			tokio::select! {biased;
				notebook =  notary_client.poll_subscriptions() => {
					if let Some((notary_id, notebook_number)) = notebook {
						trace!( "Next notebook pushed (notary {}, notebook {})", notary_id, notebook_number);
					}
				},
				block = best_block.next () => {
					if let Some(block) = block.as_ref() {
						if block.is_new_best {
							let best_hash = block.hash;
							if let Err(e) = notary_client.update_notaries(&best_hash).await {
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
}

type PendingNotebook = (NotebookNumber, Option<Vec<u8>>);
pub type VotingPowerInfo = (Tick, BlockVotingPower, u32);
const MAX_QUEUE_DEPTH: usize = 100;

pub struct NotaryClient<B: BlockT, C: AuxStore, AC> {
	client: Arc<C>,
	pub notary_client_by_id: Arc<Mutex<BTreeMap<NotaryId, Arc<argon_notary_apis::Client>>>>,
	pub notaries_by_id: Arc<Mutex<BTreeMap<NotaryId, NotaryRecordT>>>,
	pub subscriptions_by_id: Arc<Mutex<BTreeMap<NotaryId, Pin<Box<RawHeadersSubscription>>>>>,
	tick_voting_power_sender: Arc<Mutex<TracingUnboundedSender<VotingPowerInfo>>>,
	notebook_queue_by_id: Arc<Mutex<BTreeMap<NotaryId, Vec<PendingNotebook>>>>,
	aux_client: ArgonAux<B, C>,
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
		tick_voting_power_sender: TracingUnboundedSender<VotingPowerInfo>,
	) -> Self {
		Self {
			client,
			subscriptions_by_id: Default::default(),
			notary_client_by_id: Default::default(),
			notaries_by_id: Default::default(),
			notebook_queue_by_id: Default::default(),
			tick_voting_power_sender: Arc::new(Mutex::new(tick_voting_power_sender)),
			aux_client,
			_block: PhantomData,
		}
	}

	pub async fn update_notaries(&self, block_hash: &B::Hash) -> Result<(), Error> {
		let notaries = self.client.notaries(*block_hash)?;
		let mut reconnect_ids = BTreeSet::new();

		{
			let mut notaries_by_id = self.notaries_by_id.lock().await;
			let next_notaries_by_id =
				notaries.iter().map(|n| (n.notary_id, n.clone())).collect::<BTreeMap<_, _>>();
			if next_notaries_by_id != *notaries_by_id {
				let mut subscriptions_by_id = self.subscriptions_by_id.lock().await;
				for notary in &notaries {
					if let Some(existing) = notaries_by_id.get(&notary.notary_id) {
						if existing.meta.hosts[0] != notary.meta.hosts[0] {
							reconnect_ids.insert(notary.notary_id);
						}
					}
				}
				*notaries_by_id = next_notaries_by_id.clone();

				self.notary_client_by_id.lock().await.retain(|id, _| {
					if let Some(entry) = notaries_by_id.get(id) {
						if Self::should_connect_to_notary(entry) {
							return true;
						}
					}

					subscriptions_by_id.remove(id);
					false
				});
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
			if let Some(queue) = self.notebook_queue_by_id.lock().await.get(&notary_id) {
				if queue.len() >= MAX_QUEUE_DEPTH {
					continue;
				}
			}

			let is_connected = self.notary_client_by_id.lock().await.contains_key(&notary_id) &&
				self.subscriptions_by_id.lock().await.contains_key(&notary_id);

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
		let mut futures = vec![];

		let mut subscriptions = self.subscriptions_by_id.lock().await;
		for (notary_id, sub) in subscriptions.iter_mut() {
			let notary_id = *notary_id;
			futures.push(Box::pin(async move {
				match futures::future::poll_fn(|cx| sub.as_mut().poll_next(cx)).await {
					Some(Ok((notebook_number, body))) => Ok((notary_id, notebook_number, body)),
					Some(Err(e)) => Err((notary_id, Some(e.to_string()))),
					None => Err((notary_id, None)), // Subscription ended
				}
			}));
		}

		if futures.is_empty() {
			return None;
		}

		let (result, _, _) = futures::future::select_all(futures).await;
		drop(subscriptions);

		match result {
			Ok((notary_id, notebook_number, header)) => {
				let _ = self
					.enqueue_notebooks(notary_id, vec![(notebook_number, Some(header))])
					.await
					.inspect_err(|e| {
						warn!("Could not enqueue notebook for notary {} - {:?}", notary_id, e);
					});
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
		let notaries = self.notaries_by_id.lock().await.keys().copied().collect::<Vec<_>>();

		let handles = notaries.iter().map(|notary_id| {
			let self_clone: Arc<Self> = Arc::clone(self);
			let notary_id = *notary_id;
			tokio::spawn(async move {
				self_clone.clean_queued_finalized_notebooks(&finalized_hash, notary_id).await;
				self_clone.retrieve_notary_missing_notebooks(notary_id).await?;

				let Some((notebook_number, raw_header)) = self_clone.dequeue_ready(notary_id).await
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
								.enqueue_notebooks(
									notary_id,
									vec![(notebook_number, Some(raw_header))],
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
				Ok(inner_result) => has_more_work = has_more_work || inner_result?,
				Err(join_error) => {
					warn!("Error while processing notary queue - {:?}", join_error);
				},
			}
		}
		Ok(has_more_work)
	}

	async fn clean_queued_finalized_notebooks(
		&self,
		finalized_hash: &B::Hash,
		notary_id: NotaryId,
	) {
		let finalized_notebook_number = self.latest_notebook_in_runtime(*finalized_hash, notary_id);

		let mut notary_queue = self.notebook_queue_by_id.lock().await;
		if let Some(queue) = notary_queue.get_mut(&notary_id) {
			queue.retain(|(notebook_number, _)| *notebook_number > finalized_notebook_number);
		}
	}

	async fn retrieve_notary_missing_notebooks(&self, notary_id: NotaryId) -> Result<(), Error> {
		let mut missing_notebooks = vec![];
		// use notebook lock in block
		{
			let notary_queue = self.notebook_queue_by_id.lock().await;
			if let Some(notary_queue) = notary_queue.get(&notary_id) {
				for (notebook_number, raw_header) in notary_queue {
					if raw_header.is_none() {
						missing_notebooks.push(*notebook_number);
					}
				}
			}
		}
		if missing_notebooks.is_empty() {
			return Ok(());
		}
		missing_notebooks.sort();
		let original_size = missing_notebooks.len();
		// only download 10 at a time
		let missing_notebooks = missing_notebooks.into_iter().take(10).collect();

		info!(
			"Retrieving missing notebooks from notary #{} - {:?} (of {original_size})",
			notary_id, missing_notebooks
		);

		let client = self.get_client(notary_id).await?;
		let headers = client.get_raw_headers(None, Some(missing_notebooks)).await.map_err(|e| {
			Error::NotaryError(format!("Could not get notebooks from notary - {:?}", e))
		})?;
		let headers = headers.into_iter().map(|(n, h)| (n, Some(h))).collect();

		self.enqueue_notebooks(notary_id, headers).await?;
		Ok(())
	}

	async fn queue_depth(&self, notary_id: NotaryId) -> usize {
		let notary_queue = self.notebook_queue_by_id.lock().await;
		notary_queue.get(&notary_id).map_or(0, |q| q.len())
	}

	async fn dequeue_ready(&self, notary_id: NotaryId) -> Option<(NotebookNumber, Vec<u8>)> {
		let mut notary_queue_by_id = self.notebook_queue_by_id.lock().await;

		let queue = notary_queue_by_id.get_mut(&notary_id)?;
		if queue.is_empty() {
			return None;
		}
		trace!(
			"Dequeuing notebook for notary {}. Queue: {:?}",
			notary_id,
			queue.iter().map(|(n, h)| (n, h.is_some())).collect::<Vec<_>>()
		);
		if let Some((_, Some(_))) = queue.first() {
			let (notebook_number, raw_header) = queue.remove(0);
			return Some((notebook_number, raw_header?))
		}
		None
	}

	async fn enqueue_notebooks(
		&self,
		notary_id: NotaryId,
		mut headers: Vec<PendingNotebook>,
	) -> Result<(), Error> {
		let mut notebook_queue_by_id = self.notebook_queue_by_id.lock().await;

		let queue = notebook_queue_by_id.entry(notary_id).or_insert_with(Vec::new);
		for (notebook_number, raw_header) in headers.drain(..) {
			let entry = queue.iter().position(|(n, _)| *n == notebook_number);
			if let Some(index) = entry {
				// only overwrite if missing
				if queue[index].1.is_none() {
					trace!(
						"Overwriting notebook {} header in queue for notary {} with header? {}",
						notebook_number,
						notary_id,
						raw_header.is_some()
					);
					queue[index].1 = raw_header;
				}
			} else {
				trace!(
					"Queuing notebook {} for notary {} with header? {}",
					notebook_number,
					notary_id,
					raw_header.is_some()
				);
				queue.insert(0, (notebook_number, raw_header));
			}
		}
		queue.sort_by(|a, b| a.0.cmp(&b.0));
		if queue.len() >= MAX_QUEUE_DEPTH {
			queue.drain(MAX_QUEUE_DEPTH..);
			self.disconnect(&notary_id, Some("Queue depth exceeded".to_string())).await;
		}
		Ok(())
	}

	async fn sync_notebooks(&self, id: NotaryId) -> Result<(), Error> {
		let client = self.get_client(id).await?;
		let notebook_meta = client.metadata().await.map_err(|e| {
			Error::NotaryError(format!("Could not get notebooks from notary - {:?}", e))
		})?;
		let notary_notebooks = self.aux_client.get_notary_audit_history(id)?.get();
		let latest_stored = notary_notebooks.last().map(|n| n.notebook_number).unwrap_or_default();

		if latest_stored < notebook_meta.finalized_notebook_number {
			let catchup = client.get_raw_headers(Some(latest_stored), None).await.map_err(|e| {
				Error::NotaryError(format!("Could not get notebooks from notary - {:?}", e))
			})?;
			let catchup = catchup.into_iter().map(|(n, h)| (n, Some(h))).collect();
			self.enqueue_notebooks(id, catchup).await?;
		}

		Ok(())
	}

	pub async fn disconnect(&self, notary_id: &NotaryId, reason: Option<String>) {
		let mut clients = self.notary_client_by_id.lock().await;
		info!(
			"Notary client disconnected from notary #{} (or could not connect). Reason? {:?}",
			notary_id, reason
		);
		if !clients.contains_key(notary_id) {
			return;
		}
		clients.remove(notary_id);
		let mut subs = self.subscriptions_by_id.lock().await;
		drop(subs.remove(notary_id));
	}

	async fn subscribe_to_notebooks(&self, id: NotaryId) -> Result<(), Error> {
		let client = self.get_client(id).await?;
		let stream: RawHeadersSubscription = client.subscribe_raw_headers().await.map_err(|e| {
			Error::NotaryError(format!("Could not subscribe to notebooks from notary - {:?}", e))
		})?;
		let mut subs = self.subscriptions_by_id.lock().await;
		subs.insert(id, Box::pin(stream));
		Ok(())
	}

	pub async fn process_notebook(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		finalized_hash: &B::Hash,
		best_hash: &B::Hash,
		raw_header: Vec<u8>,
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
			.decode_signed_raw_notebook_header(best_hash, raw_header.clone())?
			.map_err(|e| {
				Error::NotaryError(format!(
					"Unable to decode notebook header in runtime. Notary={}, notebook={} -> {:?}",
					notary_id, notebook_number, e
				))
			})?;

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

		self.tick_voting_power_sender.lock().await.unbounded_send(voting_power).map_err(|e| {
			Error::NotaryError(format!("Could not send tick state to sender (notary {notary_id}, notebook {notebook_number}) - {:?}", e))
		})?;
		Ok(())
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
			self.enqueue_notebooks(
				notary_id,
				missing_notebooks.iter().map(|n| (*n, None)).collect(),
			)
			.await?;
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
			full_notebook.len()
		);

		// audit on the best block since we're adding dependencies
		let audit_failure_reason = match self.client.audit_notebook_and_get_votes(
			*best_hash,
			notebook_details.version,
			notary_id,
			notebook_number,
			notebook_details.header_hash,
			&vote_minimums,
			&full_notebook,
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

	async fn get_client(
		&self,
		notary_id: NotaryId,
	) -> Result<Arc<argon_notary_apis::Client>, Error> {
		let mut clients = self.notary_client_by_id.lock().await;
		if let std::collections::btree_map::Entry::Vacant(e) = clients.entry(notary_id) {
			let notaries = self.notaries_by_id.lock().await;
			let record = notaries.get(&notary_id).ok_or_else(|| {
				Error::NotaryError("No rpc endpoints found for notary".to_string())
			})?;
			let host = record.meta.hosts.first().ok_or_else(|| {
				Error::NotaryError("No rpc endpoint found for notary".to_string())
			})?;
			let host_str: String = host.clone().try_into().map_err(|e| {
				Error::NotaryError(format!(
					"Could not convert host to string for notary {} - {:?}",
					notary_id, e
				))
			})?;
			let c = argon_notary_apis::create_client(&host_str).await.map_err(|e| {
				Error::NotaryError(format!(
					"Could not connect to notary {} ({}) for audit - {:?}",
					notary_id, host_str, e
				))
			})?;
			let c = Arc::new(c);
			e.insert(c.clone());
		}
		let client = clients.get(&notary_id).ok_or_else(|| {
			Error::NotaryError("Could not connect to notary for audit".to_string())
		})?;
		Ok(client.clone())
	}

	async fn download_notebook(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
	) -> Result<Vec<u8>, Error> {
		let client = self.get_client(notary_id).await?;

		match client.get_raw_body(notebook_number).await {
			Err(err) => {
				self.disconnect(&notary_id, Some(format!("Error downloading notebook: {}", err)))
					.await;
				Err(Error::NotaryError(format!("Error downloading notebook: {}", err)))
			},
			Ok(body) => Ok(body),
		}
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

pub async fn verify_notebook_audits<B: BlockT, C>(
	aux_client: &ArgonAux<B, C>,
	notebook_audit_results: Vec<NotebookAuditResult<NotebookVerifyError>>,
) -> Result<(), Error>
where
	C: AuxStore + 'static,
{
	for _ in 0..10 {
		let mut missing_audits = vec![];
		for digest_record in &notebook_audit_results {
			let notary_audits = aux_client.get_notary_audit_history(digest_record.notary_id)?.get();

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
				missing_audits.push(digest_record);
			}
		}
		if missing_audits.is_empty() {
			return Ok(());
		}

		info!(
			"Notebook digest has missing audits. Delaying to allow import. {:#?}",
			missing_audits
		);
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	}
	Err(Error::InvalidNotebookDigest(
		"Notebook digest record could not verify all records in local storage".to_string(),
	))
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

#[cfg(test)]
mod test {
	use super::*;
	use crate::{error::Error, mock_notary::MockNotary, notary_client::NotaryApisExt};
	use argon_node_runtime::Block;
	use argon_primitives::{
		notary::{
			NotaryMeta, NotaryNotebookAuditSummary, NotaryNotebookAuditSummaryDetails,
			NotaryNotebookRawVotes, NotaryRecordWithState,
		},
		AccountId, Balance, ChainTransfer, NotaryId, NotebookHeader, NotebookMeta, NotebookNumber,
	};
	use codec::{Decode, Encode};
	use sc_utils::mpsc::TracingUnboundedReceiver;
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

	async fn system() -> (
		MockNotary,
		Arc<TestNode>,
		Arc<NotaryClient<Block, TestNode, AccountId>>,
		TracingUnboundedReceiver<VotingPowerInfo>,
	) {
		let mut test_notary = MockNotary::new(1);
		test_notary.start().await.expect("could not start notary");
		test_notary.state.lock().await.metadata =
			Some(NotebookMeta { finalized_notebook_number: 1, finalized_tick: 1 });
		let client = Arc::new(TestNode::new());
		client.add_notary(&test_notary);
		let aux_client = ArgonAux::new(client.clone());
		let (notebook_tick_tx, notebook_tick_rx) =
			sc_utils::mpsc::tracing_unbounded("node::consensus::notebook_tick_stream", 100);
		let notary_client = NotaryClient::new(client.clone(), aux_client, notebook_tick_tx);
		let notary_client = Arc::new(notary_client);
		(test_notary, client, notary_client, notebook_tick_rx)
	}

	#[tokio::test]
	async fn adds_new_notaries() {
		let (test_notary, client, notary_client, _) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert_eq!(notary_client.notaries_by_id.lock().await.len(), 1);
		assert_eq!(notary_client.notary_client_by_id.lock().await.len(), 1);
		assert_eq!(notary_client.subscriptions_by_id.lock().await.len(), 1);

		test_notary.create_notebook_header(vec![]).await;
		let next = notary_client.poll_subscriptions().await;
		assert!(next.is_some());
		assert_eq!(next.unwrap().0, 1);

		// now mark the notary as locked
		(*client.notaries.lock()).get_mut(0).unwrap().state = NotaryState::Locked {
			failed_audit_reason: NotebookVerifyError::InvalidSecretProvided,
			at_tick: 1,
			notebook_number: 1,
		};
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert_eq!(notary_client.notaries_by_id.lock().await.len(), 1);
		assert_eq!(notary_client.notary_client_by_id.lock().await.len(), 0);
		assert_eq!(notary_client.subscriptions_by_id.lock().await.len(), 0);
		test_notary.create_notebook_header(vec![]).await;
		let next = notary_client.poll_subscriptions().await;
		assert!(next.is_none());
	}

	#[tokio::test]
	async fn wont_reconnect_if_queue_depth_exceeded() {
		let (test_notary, client, notary_client, _) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert_eq!(notary_client.notaries_by_id.lock().await.len(), 1);
		for _ in 0..MAX_QUEUE_DEPTH {
			test_notary.create_notebook_header(vec![]).await;
		}
		for i in 0..MAX_QUEUE_DEPTH - 1 {
			notary_client.poll_subscriptions().await;
			assert_eq!(
				notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap().len(),
				i + 1
			);
		}

		assert_eq!(notary_client.notebook_queue_by_id.lock().await.len(), 1);
		assert_eq!(
			notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap().len(),
			MAX_QUEUE_DEPTH - 1
		);
		assert_eq!(notary_client.notary_client_by_id.lock().await.len(), 1);
		assert_eq!(notary_client.subscriptions_by_id.lock().await.len(), 1);

		let last = test_notary.create_notebook_header(vec![]).await;
		let last_id = last.notebook_number;
		notary_client.poll_subscriptions().await;
		assert_eq!(
			notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap().len(),
			MAX_QUEUE_DEPTH
		);
		assert!(!notary_client
			.notebook_queue_by_id
			.lock()
			.await
			.get(&1)
			.unwrap()
			.iter()
			.any(|(n, _)| *n == last_id));
		// should have disconnected
		assert_eq!(notary_client.notary_client_by_id.lock().await.len(), 0);
		assert_eq!(notary_client.subscriptions_by_id.lock().await.len(), 0);
	}

	#[tokio::test]
	async fn handles_queueing_correctly() {
		let (_test_notary, _client, notary_client, _) = system().await;
		notary_client
			.enqueue_notebooks(1, vec![(3, None)])
			.await
			.expect("Could not enqueue");
		notary_client
			.enqueue_notebooks(1, vec![(1, None)])
			.await
			.expect("Could not enqueue");
		notary_client
			.enqueue_notebooks(1, vec![(2, None)])
			.await
			.expect("Could not enqueue");
		assert_eq!(
			notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap(),
			&vec![(1, None), (2, None), (3, None)]
		);
		let next = notary_client.dequeue_ready(1).await;
		assert!(next.is_none());

		notary_client
			.enqueue_notebooks(1, vec![(2, Some(vec![]))])
			.await
			.expect("Could not enqueue");
		let next = notary_client.dequeue_ready(1).await;
		assert!(next.is_none());
		notary_client
			.enqueue_notebooks(1, vec![(1, Some(vec![]))])
			.await
			.expect("Could not enqueue");
		assert_eq!(
			notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap(),
			&vec![(1, Some(vec![])), (2, Some(vec![])), (3, None)]
		);
		notary_client
			.enqueue_notebooks(1, vec![(1, None)])
			.await
			.expect("Could not enqueue");
		assert_eq!(
			notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap(),
			&vec![(1, Some(vec![])), (2, Some(vec![])), (3, None)],
			"should not change the queue"
		);
		let next = notary_client.dequeue_ready(1).await;
		assert!(next.is_some());
	}

	#[tokio::test]
	async fn downloads_missing_audit_notebooks() {
		let (test_notary, client, notary_client, _) = system().await;
		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		let notebooks = (1..=11u32).map(|n| (n, None)).collect::<Vec<_>>();
		notary_client
			.enqueue_notebooks(1, notebooks.clone())
			.await
			.expect("Could not enqueue");
		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap(), &notebooks);
		for _ in 0..12 {
			test_notary.create_notebook_header(vec![]).await;
		}
		notary_client
			.retrieve_notary_missing_notebooks(1)
			.await
			.expect("Could not retrieve missing notebooks");
		let pending_notebooks = {
			let queue = notary_client.notebook_queue_by_id.lock().await;
			queue.get(&1).expect("No queue").clone()
		};

		assert_eq!(
			pending_notebooks.iter().map(|(n, a)| (*n, a.is_some())).collect::<Vec<_>>(),
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

	#[tokio::test]
	async fn supplies_missing_notebooks_on_audit() {
		let (test_notary, client, notary_client, mut rx) = system().await;
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
			notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap(),
			&vec![
				(1, None),
				(2, None),
				(3, None),
				(4, None),
				(5, None),
				(6, None),
				(7, None),
				(8, None),
				(9, None)
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
			notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap(),
			&vec![(9, None)]
		);
		for _ in 0..10 {
			test_notary.create_notebook_header(vec![]).await;
			notary_client.process_queues().await.expect("Could not process queues");
		}
		let next_rx = rx.next().await.expect("Could not receive");
		assert_eq!(next_rx.0, 9);
		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap(), &vec![]);
		let result = notary_client
			.get_notebook_dependencies(test_notary.notary_id, 10, &client.best_hash())
			.await
			.expect("Could not retrieve missing notebooks");
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].notebook_number, 9);
	}

	#[tokio::test]
	async fn can_process_notebooks_in_parallel() {
		let (test_notary, client, notary_client, mut rx) = system().await;
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
		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap().len(), 1);
		assert!(notary_client.notebook_queue_by_id.lock().await.get(&2).is_none());
		notary_client.process_queues().await.expect("Could not process queues");
		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap().len(), 0);
		assert!(notary_client.notebook_queue_by_id.lock().await.get(&2).is_none());
		let next_rx = rx.next().await.expect("Could not receive");
		assert_eq!(next_rx, (1, 0, 1));

		notary_client
			.update_notaries(&client.best_hash())
			.await
			.expect("Could not update notaries");
		assert_eq!(notary_client.notary_client_by_id.lock().await.len(), 2);
		assert_eq!(notary_client.subscriptions_by_id.lock().await.len(), 2);

		test_notary.create_notebook_header(vec![]).await;
		test_notary2.create_notebook_header(vec![]).await;
		notary_client.poll_subscriptions().await;
		notary_client.poll_subscriptions().await;

		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap().len(), 1);
		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap()[0].0, 2);
		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&2).unwrap().len(), 2);
		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&2).unwrap()[0].0, 1);
		// should process one from each notary
		notary_client.process_queues().await.expect("Could not process queues");
		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&1).unwrap().len(), 0);
		assert_eq!(notary_client.notebook_queue_by_id.lock().await.get(&2).unwrap().len(), 1);
	}
}
