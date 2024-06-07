///
///
/// NOTE - BAB: This file is copied only to modify a few functions because it is all crate
/// private. Please mark changes as they are made!!
use std::{
	pin::Pin,
	sync::Arc,
	time::{Duration, Instant},
};

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
use futures::{
	future::{select, Either},
	prelude::*,
	task::{Context, Poll},
};
use futures_timer::Delay;
use log::{debug, trace, warn};
use prometheus_endpoint::Registry;
use sc_client_api::BlockchainEvents;
use sc_consensus::import_queue::{
	buffered_link::{self, BufferedLinkReceiver, BufferedLinkSender},
	BlockImportError, BlockImportStatus, BoxBlockImport, BoxJustificationImport, ImportQueue,
	ImportQueueService, IncomingBlock, Link, RuntimeOrigin, Verifier,
};
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use sp_consensus::BlockOrigin;
use sp_runtime::{
	traits::{Block as BlockT, Header as HeaderT, NumberFor},
	Justification, Justifications,
};

use crate::{
	basic_queue::worker_messages::ImportBlocks,
	basic_queue_import::{import_single_block_metered, ImportOrFinalizeError},
	metrics::Metrics,
};

// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

const LOG_TARGET: &str = "sync::import-queue";

/// Interface to a basic block import queue that is importing blocks sequentially in a separate
/// task, with plugable verification.
pub struct BasicQueue<B: BlockT> {
	/// Handle for sending justification and block import messages to the background task.
	handle: BasicQueueHandle<B>,
	/// Results coming from the worker task.
	result_port: BufferedLinkReceiver<B>,
}

impl<B: BlockT> Drop for BasicQueue<B> {
	fn drop(&mut self) {
		// Flush the queue and close the receiver to terminate the future.
		self.handle.close();
		self.result_port.close();
	}
}

impl<B: BlockT> BasicQueue<B> {
	/// Instantiate a new basic queue, with given verifier.
	///
	/// This creates a background task, and calls `on_start` on the justification importer.
	pub fn new<V: 'static + Verifier<B>, S: Send + 'static + Sync + BlockchainEvents<B>>(
		verifier: V,
		block_import: BoxBlockImport<B>,
		client: Arc<S>,
		justification_import: Option<BoxJustificationImport<B>>,
		spawner: &impl sp_core::traits::SpawnEssentialNamed,
		prometheus_registry: Option<&Registry>,
	) -> Self {
		let (result_sender, result_port) = buffered_link::buffered_link(100_000);

		let metrics = prometheus_registry.and_then(|r| {
			Metrics::register(r)
				.map_err(|err| {
					log::warn!("Failed to register Prometheus metrics: {}", err);
				})
				.ok()
		});

		let (future, justification_sender, block_import_sender) = BlockImportWorker::new(
			result_sender,
			verifier,
			block_import,
			client,
			justification_import,
			metrics,
		);

		spawner.spawn_essential_blocking(
			"basic-block-import-worker",
			Some("block-import"),
			future.boxed(),
		);

		Self {
			handle: BasicQueueHandle::new(justification_sender, block_import_sender),
			result_port,
		}
	}
}

#[derive(Clone)]
struct BasicQueueHandle<B: BlockT> {
	/// Escrow to send justification import messages to the background task.
	justification_sender: TracingUnboundedSender<worker_messages::ImportJustification<B>>,
	/// Channel to send block import messages to the background task.
	block_import_sender: TracingUnboundedSender<worker_messages::ImportBlocks<B>>,
}

impl<B: BlockT> BasicQueueHandle<B> {
	pub fn new(
		justification_sender: TracingUnboundedSender<worker_messages::ImportJustification<B>>,
		block_import_sender: TracingUnboundedSender<worker_messages::ImportBlocks<B>>,
	) -> Self {
		Self { justification_sender, block_import_sender }
	}

	pub fn close(&mut self) {
		self.justification_sender.close();
		self.block_import_sender.close();
	}
}

impl<B: BlockT> ImportQueueService<B> for BasicQueueHandle<B> {
	fn import_blocks(&mut self, origin: BlockOrigin, blocks: Vec<IncomingBlock<B>>) {
		if blocks.is_empty() {
			return;
		}

		trace!(target: LOG_TARGET, "Scheduling {} blocks for import", blocks.len());
		let res = self
			.block_import_sender
			.unbounded_send(worker_messages::ImportBlocks(origin, blocks));

		if res.is_err() {
			log::error!(
				target: LOG_TARGET,
				"import_blocks: Background import task is no longer alive"
			);
		}
	}

	fn import_justifications(
		&mut self,
		who: RuntimeOrigin,
		hash: B::Hash,
		number: NumberFor<B>,
		justifications: Justifications,
	) {
		for justification in justifications {
			let res = self.justification_sender.unbounded_send(
				worker_messages::ImportJustification(who, hash, number, justification),
			);

			if res.is_err() {
				log::error!(
					target: LOG_TARGET,
					"import_justification: Background import task is no longer alive"
				);
			}
		}
	}
}

#[async_trait::async_trait]
impl<B: BlockT> ImportQueue<B> for BasicQueue<B> {
	/// Get handle to [`ImportQueueService`].
	fn service(&self) -> Box<dyn ImportQueueService<B>> {
		Box::new(self.handle.clone())
	}

	/// Get a reference to the handle to [`ImportQueueService`].
	fn service_ref(&mut self) -> &mut dyn ImportQueueService<B> {
		&mut self.handle
	}

	/// Poll actions from network.
	fn poll_actions(&mut self, cx: &mut Context, link: &mut dyn Link<B>) {
		if self.result_port.poll_actions(cx, link).is_err() {
			log::error!(
				target: LOG_TARGET,
				"poll_actions: Background import task is no longer alive"
			);
		}
	}

	/// Start asynchronous runner for import queue.
	///
	/// Takes an object implementing [`Link`] which allows the import queue to
	/// influece the synchronization process.
	async fn run(mut self, mut link: Box<dyn Link<B>>) {
		loop {
			if let Err(_) = self.result_port.next_action(&mut *link).await {
				log::error!(target: "sync", "poll_actions: Background import task is no longer alive");
				return;
			}
		}
	}
}

/// Messages destinated to the background worker.
mod worker_messages {
	use super::*;

	pub struct ImportBlocks<B: BlockT>(pub BlockOrigin, pub Vec<IncomingBlock<B>>);
	pub struct ImportJustification<B: BlockT>(
		pub RuntimeOrigin,
		pub B::Hash,
		pub NumberFor<B>,
		pub Justification,
	);
}

/// The process of importing blocks.
///
/// This polls the `block_import_receiver` for new blocks to import and than awaits on
/// importing these blocks. After each block is imported, this async function yields once
/// to give other futures the possibility to be run.
///
/// Returns when `block_import` ended.
///
/// ULIXEE MODIFICATION
/// We are going to add a stream to listen for finality notifications. This will allow us to
/// hold onto blocks that need processing dependent on finalization receipt, which can come from
/// notaries submitting notebook.
async fn block_import_process<B: BlockT, S: 'static + Send + BlockchainEvents<B>>(
	mut block_import: BoxBlockImport<B>,
	mut verifier: impl Verifier<B>,
	mut result_sender: BufferedLinkSender<B>,
	client: Arc<S>,
	mut block_import_receiver: TracingUnboundedReceiver<worker_messages::ImportBlocks<B>>,
	metrics: Option<Metrics>,
	delay_between_blocks: Duration,
	max_pending_finalization: Duration,
) {
	let mut pending_finalization: Vec<(JustificationNeeded<B>, ImportBlocks<B>, Instant)> = vec![];
	let mut latest_finalized: NumberFor<B> = 0u32.into();
	let mut has_updated_finality = false;
	// ULIXEE BAB - MODIFICATION
	// Some blocks are going to go into a holding pattern here. But we don't want to hold up
	// everything else
	// Create the stream (or you might have it from somewhere else)
	let mut finality_stream = client.finality_notification_stream();

	loop {
		// // loop internally while there are new finality notifications
		// // NOTE: we don't want to block the whole outerloop
		while let Poll::Ready(Some(finality)) = futures::poll!(finality_stream.next()) {
			if finality.header.number() > &latest_finalized {
				latest_finalized = *finality.header.number();
				has_updated_finality = true;
			}
		}

		let pending_finalization_count = pending_finalization.len();

		if has_updated_finality && pending_finalization_count > 0 {
			log::trace!(target: LOG_TARGET, "Finality notification received for block height {:?}. Total pending-finalization block-import groups {:?}", latest_finalized, pending_finalization_count);
			let mut retained_finalizations = vec![];
			for (justification, import_blocks, first_seen) in pending_finalization.drain(..) {
				if latest_finalized < justification.1 {
					if first_seen - Instant::now() > max_pending_finalization {
						warn!(
							target: LOG_TARGET,
							"Block set {:?} from {:?} has been waiting for finalization of {:?} for too long, dropping it",
							import_blocks.1.iter().map(|a| a.hash).collect::<Vec<_>>(),
							import_blocks.0,
							justification.1,
						)
					} else {
						retained_finalizations.push((justification, import_blocks, first_seen));
					}
					continue;
				}

				if let Some((just, remaining)) = process_ready_blocks(
					&mut block_import,
					&mut verifier,
					&mut result_sender,
					&metrics,
					delay_between_blocks,
					import_blocks,
				)
				.await
				{
					let pending_headers = remaining
						.1
						.iter()
						.map(|a| a.header.as_ref().map(|x| x.number()))
						.collect::<Vec<_>>();
					log::info!(target: LOG_TARGET, "{} blocks ({:?}) depend on finalizing block {} ({}) before import after retry", remaining.1.len(), pending_headers, just.1, just.0);
					retained_finalizations.push((just, remaining, first_seen));
				}
			}
			pending_finalization = retained_finalizations;
			has_updated_finality = false;
		}

		// We want to resume loop on either a block or finality (since this ticks on poll)
		let import = match select(finality_stream.next(), block_import_receiver.next()).await {
			Either::Right((Some(import), _)) => import,
			Either::Left((Some(finality), _)) => {
				if finality.header.number() > &latest_finalized {
					latest_finalized = *finality.header.number();
					has_updated_finality = true;
				}
				continue;
			},
			Either::Right((None, _)) => {
				log::info!(target: LOG_TARGET, "Block import channel closed, terminating");
				return;
			},
			Either::Left((None, _)) => {
				log::info!(target: LOG_TARGET, "Finality notification channel closed, terminating");
				return;
			},
		};

		let pending = process_ready_blocks(
			&mut block_import,
			&mut verifier,
			&mut result_sender,
			&metrics,
			delay_between_blocks,
			import,
		)
		.await;
		if let Some((just, remaining)) = pending {
			let pending_headers = remaining
				.1
				.iter()
				.map(|a| a.header.as_ref().map(|x| x.number()))
				.collect::<Vec<_>>();

			log::info!(target: LOG_TARGET, "{} blocks ({:?}) depend on finalizing block {} ({}) before import", remaining.1.len(),pending_headers, just.1, just.0);
			pending_finalization.push((just, remaining, Instant::now()));
		}
	}
}

async fn process_ready_blocks<B: BlockT, V: Verifier<B>>(
	import_handle: &mut BoxBlockImport<B>,
	verifier: &mut V,
	result_sender: &mut BufferedLinkSender<B>,
	metrics: &Option<Metrics>,
	delay_between_blocks: Duration,
	import: ImportBlocks<B>,
) -> Option<(JustificationNeeded<B>, ImportBlocks<B>)> {
	let ImportBlocks(origin, blocks) = import;
	let mut res = import_many_blocks(
		import_handle,
		origin,
		blocks.clone(),
		verifier,
		delay_between_blocks,
		metrics.clone(),
	)
	.await;

	if let Some(((hash, number), index)) = res.needs_justification {
		result_sender.request_justification(&hash, number);
		res.block_count = index;
		let _ = res.results.split_off(index);
		let remaining = blocks[index..].to_vec();
		result_sender.blocks_processed(res.imported, res.block_count, res.results);

		return Some(((hash, number), ImportBlocks(origin, remaining)));
	}

	result_sender.blocks_processed(res.imported, res.block_count, res.results);
	None
}

struct BlockImportWorker<B: BlockT> {
	result_sender: BufferedLinkSender<B>,
	justification_import: Option<BoxJustificationImport<B>>,
	metrics: Option<Metrics>,
}

impl<B: BlockT> BlockImportWorker<B> {
	fn new<V: 'static + Verifier<B>, S: 'static + Send + Sync + BlockchainEvents<B>>(
		result_sender: BufferedLinkSender<B>,
		verifier: V,
		block_import: BoxBlockImport<B>,
		//  ULIXEE MODIFICATION
		client: Arc<S>,
		//  END ULIXEE MODIFICATION
		justification_import: Option<BoxJustificationImport<B>>,
		metrics: Option<Metrics>,
	) -> (
		impl Future<Output = ()> + Send,
		TracingUnboundedSender<worker_messages::ImportJustification<B>>,
		TracingUnboundedSender<worker_messages::ImportBlocks<B>>,
	) {
		use worker_messages::*;

		let (justification_sender, mut justification_port) =
			tracing_unbounded("mpsc_import_queue_worker_justification", 100_000);

		let (block_import_sender, block_import_port) =
			tracing_unbounded("mpsc_import_queue_worker_blocks", 100_000);

		let mut worker = BlockImportWorker { result_sender, justification_import, metrics };

		let delay_between_blocks = Duration::default();
		let max_pending_finalization = Duration::from_secs(60 * 5);

		let future = async move {
			// Let's initialize `justification_import`
			if let Some(justification_import) = worker.justification_import.as_mut() {
				for (hash, number) in justification_import.on_start().await {
					worker.result_sender.request_justification(&hash, number);
				}
			}

			let block_import_process = block_import_process(
				block_import,
				verifier,
				worker.result_sender.clone(),
				client.clone(),
				block_import_port,
				worker.metrics.clone(),
				delay_between_blocks,
				max_pending_finalization,
			);
			futures::pin_mut!(block_import_process);

			loop {
				// If the results sender is closed, that means that the import queue is shutting
				// down and we should end this future.
				if worker.result_sender.is_closed() {
					log::debug!(
						target: LOG_TARGET,
						"Stopping block import because result channel was closed!",
					);
					return;
				}

				// Make sure to first process all justifications
				while let Poll::Ready(justification) = futures::poll!(justification_port.next()) {
					match justification {
						Some(ImportJustification(who, hash, number, justification)) =>
							worker.import_justification(who, hash, number, justification).await,
						None => {
							log::debug!(
								target: LOG_TARGET,
								"Stopping block import because justification channel was closed!",
							);
							return;
						},
					}
				}

				if let Poll::Ready(()) = futures::poll!(&mut block_import_process) {
					return;
				}

				// All futures that we polled are now pending.
				futures::pending!()
			}
		};

		(future, justification_sender, block_import_sender)
	}

	async fn import_justification(
		&mut self,
		who: RuntimeOrigin,
		hash: B::Hash,
		number: NumberFor<B>,
		justification: Justification,
	) {
		let started = std::time::Instant::now();

		let success = match self.justification_import.as_mut() {
            Some(justification_import) => justification_import
                .import_justification(hash, number, justification)
                .await
                .map_err(|e| {
                    debug!(
						target: LOG_TARGET,
						"Justification import failed for hash = {:?} with number = {:?} coming from node = {:?} with error: {}",
						hash,
						number,
						who,
						e,
					);
                    e
                })
                .is_ok(),
            None => false,
        };

		if let Some(metrics) = self.metrics.as_ref() {
			metrics.justification_import_time.observe(started.elapsed().as_secs_f64());
		}

		self.result_sender.justification_imported(who, &hash, number, success);
	}
}

pub type JustificationNeeded<B> = (<B as BlockT>::Hash, NumberFor<B>);

/// Result of [`import_many_blocks`].
struct ImportManyBlocksResult<B: BlockT> {
	/// The number of blocks imported successfully.
	imported: usize,
	/// The total number of blocks processed.
	block_count: usize,
	/// The import results for each block.
	results: Vec<(Result<BlockImportStatus<NumberFor<B>>, BlockImportError>, B::Hash)>,
	/// Marker when we need to wait for justification, along with the index
	needs_justification: Option<(JustificationNeeded<B>, usize)>,
}

/// Import several blocks at once, returning import result for each block.
///
/// This will yield after each imported block once, to ensure that other futures can
/// be called as well.
async fn import_many_blocks<B: BlockT, V: Verifier<B>>(
	import_handle: &mut BoxBlockImport<B>,
	blocks_origin: BlockOrigin,
	blocks: Vec<IncomingBlock<B>>,
	verifier: &mut V,
	delay_between_blocks: Duration,
	metrics: Option<Metrics>,
) -> ImportManyBlocksResult<B> {
	let count = blocks.len();

	let blocks_range = match (
		blocks.first().and_then(|b| b.header.as_ref().map(|h| h.number())),
		blocks.last().and_then(|b| b.header.as_ref().map(|h| h.number())),
	) {
		(Some(first), Some(last)) if first != last => format!(" ({}..{})", first, last),
		(Some(first), Some(_)) => format!(" ({})", first),
		_ => Default::default(),
	};

	trace!(target: LOG_TARGET, "Starting import of {} blocks {}", count, blocks_range);

	let mut imported = 0;
	let mut results = vec![];
	let mut has_error = false;
	let mut blocks = blocks.into_iter();
	let mut needs_justification: Option<(JustificationNeeded<B>, usize)> = None;

	// Blocks in the response/drain should be in ascending order.
	loop {
		// Is there any block left to import?
		let block = match blocks.next() {
			Some(b) => b,
			None => {
				// No block left to import, success!
				return ImportManyBlocksResult {
					block_count: count,
					imported,
					results,
					needs_justification,
				};
			},
		};

		let block_number = block.header.as_ref().map(|h| *h.number());
		let block_hash = block.hash;
		let import_result = if has_error {
			Err(BlockImportError::Cancelled)
		} else {
			// The actual import.
			import_single_block_metered(
				import_handle,
				blocks_origin,
				block,
				verifier,
				metrics.clone(),
			)
			.await
			// ULIXEE BAB - translate finalized errors returned. Cancel anything after
			.map_err(|e| match e {
				ImportOrFinalizeError::<B>::BlockImportError(err) => err,
				ImportOrFinalizeError::<B>::FinalizedBlockNeeded(hash, number) => {
					log::warn!(
						target: LOG_TARGET,
						"Block {} ({}) is not finalized yet, waiting for justification",
						number,
						hash,
					);
					needs_justification = Some(((hash, number), results.len()));
					BlockImportError::Cancelled
				},
			})
		};

		if let Some(metrics) = metrics.as_ref() {
			metrics.report_import::<B>(&import_result);
		}

		if import_result.is_ok() {
			trace!(
				target: LOG_TARGET,
				"Block imported successfully {:?} ({})",
				block_number,
				block_hash,
			);
			imported += 1;
		} else {
			has_error = true;
		}

		results.push((import_result, block_hash));

		if delay_between_blocks != Duration::default() && !has_error {
			Delay::new(delay_between_blocks).await;
		} else {
			Yield::new().await
		}
	}
}

/// A future that will always `yield` on the first call of `poll` but schedules the
/// current task for re-execution.
///
/// This is done by getting the waker and calling `wake_by_ref` followed by returning
/// `Pending`. The next time the `poll` is called, it will return `Ready`.
struct Yield(bool);

impl Yield {
	fn new() -> Self {
		Self(false)
	}
}

impl Future for Yield {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
		if !self.0 {
			self.0 = true;
			cx.waker().wake_by_ref();
			Poll::Pending
		} else {
			Poll::Ready(())
		}
	}
}

#[cfg(test)]
mod tests {
	use core::default::Default;
	use std::{collections::BTreeMap, ops::Range};

	use env_logger::{Builder, Env};
	use futures::{executor::block_on, task::Poll, Future};
	use parking_lot::Mutex;
	use sc_client_api::{
		FinalityNotification, FinalityNotifications, FinalizeSummary, ImportNotifications,
		StorageEventStream, StorageKey,
	};
	use sc_consensus::{
		block_import::{
			BlockCheckParams, BlockImport, BlockImportParams, ImportResult, JustificationImport,
		},
		import_queue::Verifier,
	};
	use sc_network_types::PeerId;
	use sp_consensus::BlockOrigin;
	use sp_test_primitives::{Block, BlockNumber, Hash, Header};

	use crate::error::Error;

	use super::*;

	#[derive(Clone, Default)]
	struct Test {
		pending_justification_error:
			Arc<Mutex<BTreeMap<BlockNumber, (<Header as HeaderT>::Hash, BlockNumber)>>>,
	}

	#[async_trait::async_trait]
	impl Verifier<Block> for Test {
		async fn verify(
			&mut self,
			block: BlockImportParams<Block>,
		) -> Result<BlockImportParams<Block>, String> {
			Ok(BlockImportParams::new(block.origin, block.header))
		}
	}

	#[async_trait::async_trait]
	impl BlockImport<Block> for Test {
		type Error = sp_consensus::Error;

		async fn check_block(
			&mut self,
			_block: BlockCheckParams<Block>,
		) -> Result<ImportResult, Self::Error> {
			Ok(ImportResult::imported(false))
		}

		async fn import_block(
			&mut self,
			block: BlockImportParams<Block>,
		) -> Result<ImportResult, Self::Error> {
			if let Some((hash, number)) =
				self.pending_justification_error.lock().remove(&block.header.number())
			{
				Err(Error::<Block>::PendingFinalizedBlockDigest(hash, number).into())
			} else {
				Ok(ImportResult::imported(true))
			}
		}
	}

	#[async_trait::async_trait]
	impl JustificationImport<Block> for Test {
		type Error = sp_consensus::Error;

		async fn on_start(&mut self) -> Vec<(Hash, BlockNumber)> {
			Vec::new()
		}

		async fn import_justification(
			&mut self,
			_hash: Hash,
			_number: BlockNumber,
			_justification: Justification,
		) -> Result<(), Self::Error> {
			Ok(())
		}
	}

	struct Client<Block: BlockT> {
		finality_notification_sinks:
			Mutex<Vec<TracingUnboundedSender<FinalityNotification<Block>>>>,
	}

	impl<Block: BlockT> BlockchainEvents<Block> for Client<Block> {
		fn import_notification_stream(&self) -> ImportNotifications<Block> {
			unimplemented!()
		}
		fn every_import_notification_stream(&self) -> ImportNotifications<Block> {
			unimplemented!()
		}
		fn finality_notification_stream(&self) -> FinalityNotifications<Block> {
			let (sink, stream) = tracing_unbounded("mpsc_finality_notification_stream", 100_000);
			self.finality_notification_sinks.lock().push(sink);
			stream
		}
		fn storage_changes_notification_stream(
			&self,
			_filter_keys: Option<&[StorageKey]>,
			_child_filter_keys: Option<&[(StorageKey, Option<Vec<StorageKey>>)]>,
		) -> sp_blockchain::Result<StorageEventStream<Block::Hash>> {
			unimplemented!()
		}
	}

	impl<Block: BlockT> Client<Block> {
		fn notify_finalized(
			&self,
			notification: Option<FinalityNotification<Block>>,
		) -> sp_blockchain::Result<()> {
			let mut sinks = self.finality_notification_sinks.lock();

			let notification = match notification {
				Some(notify_finalized) => notify_finalized,
				None => {
					// Cleanup any closed finality notification sinks
					// since we won't be running the loop below which
					// would also remove any closed sinks.
					sinks.retain(|sink| !sink.is_closed());
					return Ok(());
				},
			};

			sinks.retain(|sink| sink.unbounded_send(notification.clone()).is_ok());

			Ok(())
		}
	}

	#[derive(Debug, PartialEq)]
	enum Event {
		JustificationImported(Hash),
		BlockImported(Hash, BlockNumber),
	}

	#[derive(Default)]
	struct TestLink {
		events: Vec<Event>,
	}

	impl Link<Block> for TestLink {
		fn blocks_processed(
			&mut self,
			_imported: usize,
			_count: usize,
			results: Vec<(Result<BlockImportStatus<BlockNumber>, BlockImportError>, Hash)>,
		) {
			for (result, hash) in results.into_iter() {
				if let Ok(BlockImportStatus::ImportedUnknown(number, ..)) = result {
					self.events.push(Event::BlockImported(hash, number));
				}
			}
		}

		fn justification_imported(
			&mut self,
			_who: RuntimeOrigin,
			hash: &Hash,
			_number: BlockNumber,
			_success: bool,
		) {
			self.events.push(Event::JustificationImported(*hash))
		}
	}

	pub fn setup_logs() {
		let env = Env::new().default_filter_or("debug"); //info,sync=debug,sc_=debug,sub-libp2p=debug,node=debug,runtime=debug");
		let _ = Builder::from_env(env).is_test(true).try_init();
		sp_tracing::try_init_simple();
	}

	#[test]
	fn prioritizes_finality_work_over_block_import() {
		setup_logs();
		let (result_sender, mut result_port) = buffered_link::buffered_link(100_000);

		let client = Arc::new(Client { finality_notification_sinks: Default::default() });
		let test = Test { pending_justification_error: Default::default() };

		let (worker, finality_sender, block_import_sender) = BlockImportWorker::new(
			result_sender,
			test.clone(),
			Box::new(test.clone()),
			client.clone(),
			Some(Box::new(test.clone())),
			None,
		);
		futures::pin_mut!(worker);

		let import_block = |n| {
			let header = Header {
				parent_hash: Hash::random(),
				number: n,
				extrinsics_root: Hash::random(),
				state_root: Default::default(),
				digest: Default::default(),
			};

			let hash = header.hash();

			block_import_sender
				.unbounded_send(worker_messages::ImportBlocks(
					BlockOrigin::Own,
					vec![IncomingBlock {
						hash,
						header: Some(header),
						body: None,
						indexed_body: None,
						justifications: None,
						origin: None,
						allow_missing_state: false,
						import_existing: false,
						state: None,
						skip_execution: false,
					}],
				))
				.unwrap();

			hash
		};

		let import_justification = || {
			let hash = Hash::random();
			finality_sender
				.unbounded_send(worker_messages::ImportJustification(
					PeerId::random(),
					hash,
					1,
					(*b"TEST", Vec::new()),
				))
				.unwrap();

			hash
		};

		let mut link = TestLink::default();

		// we send a bunch of tasks to the worker
		let block1 = import_block(1);
		let block2 = import_block(2);
		let block3 = import_block(3);
		let justification1 = import_justification();
		let justification2 = import_justification();
		let block4 = import_block(4);
		let block5 = import_block(5);
		let block6 = import_block(6);
		let justification3 = import_justification();

		// we poll the worker until we have processed 9 events
		block_on(futures::future::poll_fn(|cx| {
			while link.events.len() < 9 {
				match Future::poll(Pin::new(&mut worker), cx) {
					Poll::Pending => {},
					Poll::Ready(()) => panic!("import queue worker should not conclude."),
				}

				result_port.poll_actions(cx, &mut link).unwrap();
			}

			Poll::Ready(())
		}));

		// all justification tasks must be done before any block import work
		assert_eq!(
			link.events,
			vec![
				Event::JustificationImported(justification1),
				Event::JustificationImported(justification2),
				Event::JustificationImported(justification3),
				Event::BlockImported(block1, 1),
				Event::BlockImported(block2, 2),
				Event::BlockImported(block3, 3),
				Event::BlockImported(block4, 4),
				Event::BlockImported(block5, 5),
				Event::BlockImported(block6, 6),
			]
		);
	}
	#[test]
	fn handles_blocks_needing_ancestor_finalization_before_import() {
		setup_logs();
		let (result_sender, mut result_port) = buffered_link::buffered_link(100_000);

		let client = Arc::new(Client { finality_notification_sinks: Default::default() });
		let test = Test { pending_justification_error: Default::default() };
		let (worker, finality_sender, block_import_sender) = BlockImportWorker::new(
			result_sender,
			test.clone(),
			Box::new(test.clone()),
			client.clone(),
			Some(Box::new(test.clone())),
			None,
		);
		futures::pin_mut!(worker);

		let import_blocks = |rng: Range<BlockNumber>| {
			let mut parent_hash = Hash::random();
			let blocks = rng
				.into_iter()
				.map(|n| {
					let header = Header {
						parent_hash,
						number: n,
						extrinsics_root: Hash::random(),
						state_root: Default::default(),
						digest: Default::default(),
					};
					let hash = header.hash();
					parent_hash = hash;
					IncomingBlock {
						hash,
						header: Some(header),
						body: None,
						indexed_body: None,
						justifications: None,
						origin: None,
						allow_missing_state: false,
						import_existing: false,
						state: None,
						skip_execution: false,
					}
				})
				.collect::<Vec<_>>();

			block_import_sender
				.unbounded_send(ImportBlocks(BlockOrigin::Own, blocks.clone()))
				.unwrap();

			blocks
		};

		let import_justification = || {
			let hash = Hash::random();
			finality_sender
				.unbounded_send(worker_messages::ImportJustification(
					PeerId::random(),
					hash,
					1,
					(*b"TEST", Vec::new()),
				))
				.unwrap();

			hash
		};

		let mut link = TestLink::default();

		// we send a bunch of tasks to the worker
		let group1 = import_blocks(1..4);
		let block3 = group1[2].clone();

		let justification1 = import_justification();
		let justification2 = import_justification();
		test.pending_justification_error.lock().insert(6, (block3.hash, 3));
		test.pending_justification_error.lock().insert(9, (block3.hash, 5));
		let group2 = import_blocks(4..9);
		let justification3 = import_justification();
		let group3 = import_blocks(7..10);
		let group4 = import_blocks(11..12);

		// Build the notification.
		let (sink, _stream) = tracing_unbounded("test_sink", 100_000);

		let block3_finalized_notification = FinalityNotification::from_summary(
			FinalizeSummary {
				header: block3.header.unwrap(),
				finalized: vec![block3.hash],
				stale_heads: vec![],
			},
			sink.clone(),
		);
		let mut has_triggered_block_3_finalization = false;

		let block_4_finalized_notification = FinalityNotification::from_summary(
			FinalizeSummary {
				header: group2[0].header.clone().unwrap(),
				finalized: vec![group2[0].hash],
				stale_heads: vec![],
			},
			sink.clone(),
		);
		let mut has_triggered_block_4_finalization = false;

		let block_5_finalized_notification = FinalityNotification::from_summary(
			FinalizeSummary {
				header: group2[1].header.clone().unwrap(),
				finalized: vec![group2[1].hash],
				stale_heads: vec![],
			},
			sink,
		);
		let mut has_triggered_block_5_finalization = false;

		block_on(futures::future::poll_fn(|cx| {
			while link.events.len() < 15 {
				match Future::poll(Pin::new(&mut worker), cx) {
					Poll::Pending => {},
					Poll::Ready(()) => panic!("import queue worker should not conclude."),
				}

				if link.events.len() >= 8 && !has_triggered_block_3_finalization {
					has_triggered_block_3_finalization = true;
					client.notify_finalized(Some(block3_finalized_notification.clone())).unwrap();
				}
				if link.events.len() >= 10 && !has_triggered_block_4_finalization {
					has_triggered_block_4_finalization = true;
					client.notify_finalized(Some(block_4_finalized_notification.clone())).unwrap();
				}
				if link.events.len() >= 13 && !has_triggered_block_5_finalization {
					has_triggered_block_5_finalization = true;
					client.notify_finalized(Some(block_5_finalized_notification.clone())).unwrap();
				}

				result_port.poll_actions(cx, &mut link).unwrap();
			}

			Poll::Ready(())
		}));

		assert_eq!(
			link.events,
			vec![
				Event::JustificationImported(justification1),
				Event::JustificationImported(justification2),
				Event::JustificationImported(justification3),
				Event::BlockImported(group1[0].hash, 1),
				Event::BlockImported(group1[1].hash, 2),
				Event::BlockImported(group1[2].hash, 3),
				Event::BlockImported(group2[0].hash, 4),
				Event::BlockImported(group2[1].hash, 5),
				// now should be block 7
				Event::BlockImported(group3[0].hash, 7),
				Event::BlockImported(group3[1].hash, 8),
				// finalized! 6/7/8 should be good now
				Event::BlockImported(group2[2].hash, 6),
				Event::BlockImported(group2[3].hash, 7),
				Event::BlockImported(group2[4].hash, 8),
				// finalize block 4 at this point
				Event::BlockImported(group4[0].hash, 11),
				Event::BlockImported(group3[2].hash, 9),
			]
		);
	}
}
