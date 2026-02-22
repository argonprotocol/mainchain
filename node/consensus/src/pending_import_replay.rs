use codec::{Decode, Encode};
use futures::{Future, FutureExt};
use polkadot_sdk::*;
use sc_client_api::{BlockBackend, backend::AuxStore};
use sc_consensus::{BlockImportParams, ForkChoiceStrategy, StateAction};
use sp_consensus::{BlockOrigin, BlockStatus};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use std::{cmp::Ordering, sync::Arc, time::Duration};
use tracing::{debug, info, warn};

use crate::{
	error::Error,
	import_queue::ImportApisExt,
	notary_client::{NotaryApisExt, NotaryClient},
};

pub(crate) const MAX_PENDING_IMPORTS: usize = 1024;
const PENDING_IMPORTS_AUX_KEY: &[u8] = b"argon/consensus/pending-imports/v1";
const REPLAY_NOTEBOOK_AUDIT_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_REPLAY_SCAN_PER_PASS: usize = 64;

pub(crate) struct PendingBlockImport<B: BlockT> {
	pub(crate) hash: B::Hash,
	pub(crate) parent_hash: B::Hash,
	pub(crate) block: BlockImportParams<B>,
}

pub(crate) struct PendingImportReplayQueue<B: BlockT, C: AuxStore> {
	client: Arc<C>,
	pending_imports: Arc<tokio::sync::Mutex<Vec<PendingBlockImport<B>>>>,
}

pub(crate) enum EnqueueResult {
	Enqueued,
	AlreadyQueued,
	QueueFull,
}

pub(crate) enum DeferFullImportResult<B: BlockT> {
	Deferred(BlockImportParams<B>),
	SaturatedHeaderOnly(BlockImportParams<B>),
}

impl<B, C> Clone for PendingImportReplayQueue<B, C>
where
	B: BlockT,
	C: AuxStore,
{
	fn clone(&self) -> Self {
		Self { client: self.client.clone(), pending_imports: self.pending_imports.clone() }
	}
}

impl<B, C> PendingImportReplayQueue<B, C>
where
	B: BlockT,
	C: AuxStore,
{
	pub(crate) fn new(client: Arc<C>) -> Self {
		let pending_imports = load_pending_imports_from_aux::<B, C>(&client);
		if !pending_imports.is_empty() {
			info!(
				count = pending_imports.len(),
				"Recovered pending full block imports from aux store"
			);
		}
		Self { client, pending_imports: Arc::new(tokio::sync::Mutex::new(pending_imports)) }
	}

	pub(crate) async fn len(&self) -> usize {
		self.pending_imports.lock().await.len()
	}

	pub(crate) async fn has_hash(&self, hash: B::Hash) -> bool {
		self.pending_imports.lock().await.iter().any(|entry| entry.hash == hash)
	}

	pub(crate) async fn enqueue(&self, block: BlockImportParams<B>) -> EnqueueResult {
		let hash = block.post_hash();
		let number = *block.header.number();
		let parent_hash = *block.header.parent_hash();
		let mut pending_imports = self.pending_imports.lock().await;
		if pending_imports.iter().any(|entry| entry.hash == hash) {
			return EnqueueResult::AlreadyQueued;
		}

		if pending_imports.len() >= MAX_PENDING_IMPORTS {
			warn!(
				block_hash = ?hash,
				number = ?number,
				queue_len = pending_imports.len(),
				"Pending import replay queue is full"
			);
			return EnqueueResult::QueueFull;
		}

		pending_imports.push(PendingBlockImport { hash, parent_hash, block });
		sort_pending_imports(&mut pending_imports);
		self.persist_snapshot(&pending_imports);
		EnqueueResult::Enqueued
	}

	pub(crate) async fn defer_full_import(
		&self,
		block: BlockImportParams<B>,
	) -> Result<DeferFullImportResult<B>, Error> {
		if !block.intermediates.is_empty() {
			warn!(
				block_hash = ?block.post_hash(),
				number = ?block.header.number(),
				intermediates = block.intermediates.len(),
				"Cannot defer full import with unresolved intermediates"
			);
			return Err(Error::PendingImportUnsupported(
				"cannot defer imports with unresolved intermediates".into(),
			));
		}
		let (header_only, pending_full_import) = split_for_deferred_import(block);
		match self.enqueue(pending_full_import).await {
			EnqueueResult::Enqueued | EnqueueResult::AlreadyQueued =>
				Ok(DeferFullImportResult::Deferred(header_only)),
			EnqueueResult::QueueFull => Ok(DeferFullImportResult::SaturatedHeaderOnly(header_only)),
		}
	}

	pub(crate) async fn dequeue_ready(&self) -> Option<PendingBlockImport<B>>
	where
		C: BlockBackend<B>,
	{
		let mut pending_imports = self.pending_imports.lock().await;
		if pending_imports.is_empty() {
			return None;
		}

		let before = pending_imports.len();
		pending_imports.retain(|entry| {
			self.client.block_status(entry.hash).unwrap_or(BlockStatus::Unknown) !=
				BlockStatus::InChainWithState
		});
		let pruned = before.saturating_sub(pending_imports.len());
		if pruned > 0 {
			debug!(pruned, "Pruned pending imports already in chain with state");
		}

		sort_pending_imports(&mut pending_imports);
		let next_ready_index = pending_imports.iter().position(|entry| {
			self.client.block_status(entry.parent_hash).unwrap_or(BlockStatus::Unknown) ==
				BlockStatus::InChainWithState
		});
		let next = next_ready_index.map(|index| pending_imports.remove(index));
		self.persist_snapshot(&pending_imports);
		next
	}

	pub(crate) async fn pending_import_ready_for_replay<AC>(
		&self,
		notary_client: &Arc<NotaryClient<B, C, AC>>,
		ctx: &PendingReplayContext<B>,
	) -> bool
	where
		C: ImportApisExt<B, AC> + NotaryApisExt<B, AC> + BlockBackend<B> + 'static,
		AC: Clone + codec::Codec + Send + Sync + 'static,
	{
		if ctx.skip_execution_checks ||
			matches!(ctx.origin, BlockOrigin::Own | BlockOrigin::NetworkInitialSync) ||
			ctx.finalized
		{
			return true;
		}

		let latest_verified_finalized = self.client.info().finalized_number;
		if ctx.number <= latest_verified_finalized {
			return true;
		}

		if !matches!(
			self.client.block_status(ctx.parent_hash).unwrap_or(BlockStatus::Unknown),
			BlockStatus::InChainWithState
		) {
			return false;
		}

		let digest_notebooks =
			match self.client.runtime_digest_notebooks(ctx.parent_hash, &ctx.digest) {
				Ok(digest_notebooks) => digest_notebooks,
				Err(err) => {
					warn!(
						block_hash = ?ctx.hash,
						block_number = ?ctx.number,
						parent_hash = ?ctx.parent_hash,
						error = ?err,
						"Skipping pending replay: unable to load digest notebooks"
					);
					return false;
				},
			};
		if digest_notebooks.is_empty() {
			return true;
		}

		match notary_client
			.verify_notebook_audits_for_import(
				&ctx.parent_hash,
				digest_notebooks,
				REPLAY_NOTEBOOK_AUDIT_TIMEOUT,
			)
			.await
		{
			Ok(_) => true,
			Err(Error::InvalidNotebookDigest(_)) => true,
			Err(err) if err.is_retryable_notebook_audit_error() => false,
			Err(err) => {
				warn!(
					block_hash = ?ctx.hash,
					block_number = ?ctx.number,
					parent_hash = ?ctx.parent_hash,
					error = ?err,
					"Skipping pending replay due to unexpected notebook audit error"
				);
				false
			},
		}
	}

	pub(crate) async fn dequeue_ready_for_replay<AC>(
		&self,
		notary_client: &Arc<NotaryClient<B, C, AC>>,
	) -> Option<(PendingBlockImport<B>, PendingReplayContext<B>)>
	where
		C: ImportApisExt<B, AC> + NotaryApisExt<B, AC> + BlockBackend<B> + 'static,
		AC: Clone + codec::Codec + Send + Sync + 'static,
	{
		let mut deferred_candidates = Vec::new();
		let actual_pending_count = self.len().await;
		if actual_pending_count > MAX_REPLAY_SCAN_PER_PASS {
			debug!(
				pending_count = actual_pending_count,
				max_scan_per_pass = MAX_REPLAY_SCAN_PER_PASS,
				"Limiting pending import replay scan due to bounded scan limit"
			);
		}
		let pending_count = actual_pending_count.min(MAX_REPLAY_SCAN_PER_PASS);
		for _ in 0..pending_count {
			let Some(pending_import) = self.dequeue_ready().await else {
				break;
			};
			let replay_context =
				PendingReplayContext::from_block(pending_import.hash, &pending_import.block);
			if self.pending_import_ready_for_replay(notary_client, &replay_context).await {
				for pending in deferred_candidates {
					self.requeue_pending_import(pending).await;
				}
				return Some((pending_import, replay_context));
			}
			debug!(
				block_hash = ?replay_context.hash,
				number = ?replay_context.number,
				"Pending block replay deferred: prerequisites not ready"
			);
			deferred_candidates.push(pending_import);
		}

		for pending in deferred_candidates {
			self.requeue_pending_import(pending).await;
		}
		None
	}

	pub(crate) fn retry_block_from_pending(
		pending_import: &PendingBlockImport<B>,
	) -> Option<BlockImportParams<B>> {
		PersistedPendingBlockImport::try_from_pending(pending_import)
			.map(PersistedPendingBlockImport::into_pending)
			.map(|pending| pending.block)
	}

	pub(crate) async fn requeue_retry_block(&self, block: BlockImportParams<B>) {
		let hash = block.post_hash();
		let parent_hash = *block.header.parent_hash();
		self.requeue_pending_import(PendingBlockImport { hash, parent_hash, block })
			.await;
	}

	async fn requeue_pending_import(&self, pending_import: PendingBlockImport<B>) {
		let mut pending_imports = self.pending_imports.lock().await;
		if pending_imports.iter().any(|entry| entry.hash == pending_import.hash) {
			return;
		}
		if pending_imports.len() >= MAX_PENDING_IMPORTS {
			warn!(
				block_hash = ?pending_import.hash,
				number = ?pending_import.block.header.number(),
				queue_len = pending_imports.len(),
				"Pending replay queue is full while requeueing deferred import; keeping deferred import"
			);
		}
		pending_imports.push(pending_import);
		sort_pending_imports(&mut pending_imports);
		self.persist_snapshot(&pending_imports);
	}

	fn persist_snapshot(&self, pending_imports: &[PendingBlockImport<B>]) {
		if let Err(err) = persist_pending_imports_to_aux::<B, C>(&self.client, pending_imports) {
			warn!(error = ?err, "Failed to persist pending import queue");
		}
	}
}

pub(crate) fn spawn_pending_import_replay_task<F, Fut>(
	spawner: &impl sp_core::traits::SpawnEssentialNamed,
	mut replay_pending_imports: F,
) where
	F: FnMut() -> Fut + Send + 'static,
	Fut: Future<Output = ()> + Send + 'static,
{
	spawner.spawn_essential(
		"pending_block_replay",
		Some("notary_sync"),
		async move {
			let mut replay_tick = tokio::time::interval(std::time::Duration::from_secs(2));
			replay_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

			loop {
				replay_tick.tick().await;
				replay_pending_imports().await;
			}
		}
		.boxed(),
	);
}

#[derive(Clone, Copy, Decode, Encode)]
enum PersistedStateAction {
	Execute,
	ExecuteIfPossible,
}

impl PersistedStateAction {
	fn from_state_action<B: BlockT>(state_action: &StateAction<B>) -> Option<Self> {
		match state_action {
			StateAction::Execute => Some(Self::Execute),
			StateAction::ExecuteIfPossible => Some(Self::ExecuteIfPossible),
			_ => None,
		}
	}

	fn to_state_action<B: BlockT>(self) -> StateAction<B> {
		match self {
			Self::Execute => StateAction::Execute,
			Self::ExecuteIfPossible => StateAction::ExecuteIfPossible,
		}
	}
}

#[derive(Clone, Copy, Decode, Encode)]
enum PersistedBlockOrigin {
	Genesis,
	NetworkInitialSync,
	NetworkBroadcast,
	ConsensusBroadcast,
	Own,
	File,
}

impl PersistedBlockOrigin {
	fn from_block_origin(origin: BlockOrigin) -> Self {
		match origin {
			BlockOrigin::Genesis => Self::Genesis,
			BlockOrigin::NetworkInitialSync => Self::NetworkInitialSync,
			BlockOrigin::NetworkBroadcast => Self::NetworkBroadcast,
			BlockOrigin::ConsensusBroadcast => Self::ConsensusBroadcast,
			BlockOrigin::Own => Self::Own,
			BlockOrigin::File => Self::File,
		}
	}

	fn to_block_origin(self) -> BlockOrigin {
		match self {
			Self::Genesis => BlockOrigin::Genesis,
			Self::NetworkInitialSync => BlockOrigin::NetworkInitialSync,
			Self::NetworkBroadcast => BlockOrigin::NetworkBroadcast,
			Self::ConsensusBroadcast => BlockOrigin::ConsensusBroadcast,
			Self::Own => BlockOrigin::Own,
			Self::File => BlockOrigin::File,
		}
	}
}

#[derive(Decode, Encode)]
struct PersistedPendingBlockImport<B: BlockT> {
	origin: PersistedBlockOrigin,
	header: B::Header,
	justifications: Option<sp_runtime::Justifications>,
	post_digests: Vec<sp_runtime::DigestItem>,
	body: Option<Vec<B::Extrinsic>>,
	indexed_body: Option<Vec<Vec<u8>>>,
	state_action: PersistedStateAction,
	finalized: bool,
	auxiliary: Vec<(Vec<u8>, Option<Vec<u8>>)>,
	import_existing: bool,
	create_gap: bool,
	post_hash: Option<B::Hash>,
}

impl<B: BlockT> PersistedPendingBlockImport<B> {
	fn try_from_pending(pending: &PendingBlockImport<B>) -> Option<Self> {
		let state_action = PersistedStateAction::from_state_action(&pending.block.state_action)?;
		Some(Self {
			origin: PersistedBlockOrigin::from_block_origin(pending.block.origin),
			header: pending.block.header.clone(),
			justifications: pending.block.justifications.clone(),
			post_digests: pending.block.post_digests.clone(),
			body: pending.block.body.clone(),
			indexed_body: pending.block.indexed_body.clone(),
			state_action,
			finalized: pending.block.finalized,
			auxiliary: pending.block.auxiliary.clone(),
			import_existing: pending.block.import_existing,
			create_gap: pending.block.create_gap,
			post_hash: pending.block.post_hash,
		})
	}

	fn into_pending(self) -> PendingBlockImport<B> {
		let mut block = BlockImportParams::<B>::new(self.origin.to_block_origin(), self.header);
		block.justifications = self.justifications;
		block.post_digests = self.post_digests;
		block.body = self.body;
		block.indexed_body = self.indexed_body;
		block.state_action = self.state_action.to_state_action();
		block.finalized = self.finalized;
		block.auxiliary = self.auxiliary;
		block.import_existing = self.import_existing;
		block.create_gap = self.create_gap;
		block.post_hash = self.post_hash;

		let hash = block.post_hash();
		let parent_hash: B::Hash = *block.header.parent_hash();
		PendingBlockImport { hash, parent_hash, block }
	}
}

fn sort_pending_imports<B: BlockT>(pending_imports: &mut [PendingBlockImport<B>]) {
	pending_imports.sort_by(|a, b| {
		let by_number = a
			.block
			.header
			.number()
			.partial_cmp(b.block.header.number())
			.unwrap_or(Ordering::Equal);
		if by_number != Ordering::Equal {
			return by_number;
		}
		a.hash.encode().cmp(&b.hash.encode())
	});
}

fn split_for_deferred_import<B: BlockT>(
	block: BlockImportParams<B>,
) -> (BlockImportParams<B>, BlockImportParams<B>) {
	// `BlockImportParams` is intentionally not `Clone` (notably `intermediates` with `Box<dyn
	// Any>`), so keep the full block for replay and build a minimal header-only import for now.
	let mut header_only = BlockImportParams::new(block.origin, block.header.clone());
	header_only.justifications = block.justifications.clone();
	header_only.post_digests = block.post_digests.clone();
	header_only.post_hash = Some(block.post_hash());
	header_only.import_existing = block.import_existing;
	header_only.create_gap = block.create_gap;
	header_only.finalized = block.finalized;
	header_only.fork_choice = Some(ForkChoiceStrategy::Custom(false));
	header_only.state_action = StateAction::Skip;
	(header_only, block)
}

fn load_pending_imports_from_aux<B: BlockT, C: AuxStore>(
	client: &Arc<C>,
) -> Vec<PendingBlockImport<B>> {
	let Ok(Some(bytes)) = client.get_aux(PENDING_IMPORTS_AUX_KEY) else {
		return Vec::new();
	};

	let Ok(persisted) = Vec::<PersistedPendingBlockImport<B>>::decode(&mut &bytes[..]) else {
		warn!("Failed to decode persisted pending import queue. Starting with an empty queue.");
		return Vec::new();
	};

	let mut pending_imports = persisted
		.into_iter()
		.map(PersistedPendingBlockImport::into_pending)
		.collect::<Vec<_>>();
	sort_pending_imports(&mut pending_imports);
	pending_imports
}

fn persist_pending_imports_to_aux<B: BlockT, C: AuxStore>(
	client: &Arc<C>,
	pending_imports: &[PendingBlockImport<B>],
) -> Result<(), sp_blockchain::Error> {
	let mut persisted = Vec::with_capacity(pending_imports.len());
	for pending in pending_imports {
		let Some(entry) = PersistedPendingBlockImport::try_from_pending(pending) else {
			warn!(
				block_hash = ?pending.hash,
				number = ?pending.block.header.number(),
				"Skipping persistence for pending import with unsupported state action"
			);
			continue;
		};
		persisted.push(entry);
	}

	if persisted.is_empty() {
		return client.insert_aux(&[], &[PENDING_IMPORTS_AUX_KEY]);
	}

	let encoded = persisted.encode();
	client.insert_aux(&[(PENDING_IMPORTS_AUX_KEY, encoded.as_slice())], &[])
}

#[derive(Debug)]
pub(crate) struct PendingReplayContext<B: BlockT> {
	pub(crate) hash: B::Hash,
	pub(crate) number: sp_runtime::traits::NumberFor<B>,
	pub(crate) parent_hash: B::Hash,
	pub(crate) origin: BlockOrigin,
	pub(crate) finalized: bool,
	pub(crate) skip_execution_checks: bool,
	pub(crate) digest: sp_runtime::Digest,
}

impl<B: BlockT> PendingReplayContext<B> {
	pub(crate) fn from_block(hash: B::Hash, block: &BlockImportParams<B>) -> Self {
		Self {
			hash,
			number: *block.header.number(),
			parent_hash: *block.header.parent_hash(),
			origin: block.origin,
			finalized: block.finalized,
			skip_execution_checks: block.state_action.skip_execution_checks(),
			digest: block.header.digest().clone(),
		}
	}
}
