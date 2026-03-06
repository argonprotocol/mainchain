use crate::{
	NotaryClient,
	aux_client::ArgonAux,
	compute_worker::BlockComputeNonce,
	error::Error,
	pending_import_replay::{
		DeferFullImportResult, PendingImportReplayQueue, spawn_pending_import_replay_task,
	},
};
use argon_bitcoin_utxo_tracker::{UtxoTracker, get_bitcoin_inherent};
use argon_primitives::{
	AccountId, Balance, BitcoinApis, BlockCreatorApis, BlockImportApis, BlockSealApis,
	BlockSealAuthorityId, BlockSealDigest, NotaryApis, NotebookApis, NotebookAuditResult, TickApis,
	digests::ArgonDigests,
	fork_power::ForkPower,
	inherents::{BitcoinInherentDataProvider, BlockSealInherentDataProvider},
	prelude::substrate_prometheus_endpoint::{CounterVec, Opts, Registry, U64, register},
};
use argon_runtime::{NotaryRecordT, NotebookVerifyError};
use codec::{Codec, Encode};
use polkadot_sdk::{
	sp_core::{H256, blake2_256},
	substrate_prometheus_endpoint::PrometheusError,
	*,
};
use sc_client_api::{self, BlockBackend, backend::AuxStore};
use sc_consensus::{
	BasicQueue, BlockCheckParams, BlockImport, BlockImportParams, BoxJustificationImport,
	ForkChoiceStrategy, ImportResult, StateAction, StorageChanges, Verifier as VerifierT,
};
use sc_telemetry::TelemetryHandle;
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::Zero;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, BlockStatus, Error as ConsensusError};
use sp_inherents::InherentDataProvider;
use sp_runtime::{
	Justification,
	traits::{Block as BlockT, Header as HeaderT, NumberFor},
};
use std::{fmt, marker::PhantomData, sync::Arc, time::Duration};
use tracing::{error, warn};

const IMPORT_NOTEBOOK_AUDIT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct ImportMetrics {
	/// Import a block with a vote seal
	imported_vote_sealed_blocks_total: CounterVec<U64>,
	/// Imported block with bitcoin block inherent
	imported_block_with_new_bitcoin_tip: CounterVec<U64>,
	/// Imported block with new price index inherent
	imported_block_with_new_price_index: CounterVec<U64>,
}

impl ImportMetrics {
	pub fn new(metrics_registry: &Registry) -> Result<Self, PrometheusError> {
		Ok(Self {
			imported_block_with_new_bitcoin_tip: register(
				CounterVec::new(
					Opts::new(
						"argon_imported_block_with_new_bitcoin_tip",
						"Total imported blocks with bitcoin block tip advanced",
					),
					&[],
				)?,
				metrics_registry,
			)?,
			imported_vote_sealed_blocks_total: register(
				CounterVec::new(
					Opts::new(
						"argon_imported_vote_sealed_blocks_total",
						"Total imported blocks with vote seals",
					),
					&[],
				)?,
				metrics_registry,
			)?,
			imported_block_with_new_price_index: register(
				CounterVec::new(
					Opts::new(
						"argon_imported_block_with_new_price_index",
						"Total imported blocks with new price index",
					),
					&[],
				)?,
				metrics_registry,
			)?,
		})
	}
}

pub trait ImportApisExt<B: BlockT, AC>: HeaderBackend<B> + BlockBackend<B> {
	fn has_new_bitcoin_tip(&self, hash: B::Hash) -> bool;
	fn has_new_price_index(&self, hash: B::Hash) -> bool;
	fn runtime_digest_notebooks(
		&self,
		parent_hash: B::Hash,
		digest: &sp_runtime::Digest,
	) -> Result<Vec<NotebookAuditResult<NotebookVerifyError>>, Error>;
}

impl<B: BlockT, C, AC> ImportApisExt<B, AC> for C
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + BlockBackend<B>,
	C::Api: BlockImportApis<B> + BlockCreatorApis<B, AC, NotebookVerifyError>,
	AC: Codec + Clone,
{
	fn has_new_bitcoin_tip(&self, hash: B::Hash) -> bool {
		self.runtime_api().has_new_bitcoin_tip(hash).unwrap_or(false)
	}

	fn has_new_price_index(&self, hash: B::Hash) -> bool {
		self.runtime_api().has_new_price_index(hash).unwrap_or(false)
	}

	fn runtime_digest_notebooks(
		&self,
		parent_hash: B::Hash,
		digest: &sp_runtime::Digest,
	) -> Result<Vec<NotebookAuditResult<NotebookVerifyError>>, Error> {
		self.runtime_api()
			.digest_notebooks(parent_hash, digest)
			.map_err(|e| {
				Error::MissingRuntimeData(format!("Error calling digest notebooks api: {e:?}"))
			})?
			.map_err(|e| {
				Error::MissingRuntimeData(format!("Failed to get digest notebooks: {e:?}"))
			})
	}
}

/// A block importer for argon.
pub struct ArgonBlockImport<B: BlockT, I, C: AuxStore, AC> {
	inner: I,
	pub(crate) client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	pub(crate) notary_client: Arc<NotaryClient<B, C, AC>>,
	import_lock: Arc<tokio::sync::Mutex<()>>,
	pub(crate) pending_full_import_queue: PendingImportReplayQueue<B, C>,
	metrics: Arc<Option<ImportMetrics>>,
	_phantom: PhantomData<AC>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ImportContext<B: BlockT> {
	hash: B::Hash,
	number: NumberFor<B>,
	parent_hash: B::Hash,
	info: sp_blockchain::Info<B>,
	is_block_gap: bool,
	parent_block_state: BlockStatus,
	block_header_status: sp_blockchain::BlockStatus,
	state_action: ImportStateAction,
	skip_execution_checks: bool,
	has_body: bool,
	origin: BlockOrigin,
	finalized: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ImportStateAction {
	StateApply,
	StateImport,
	Execute,
	ExecuteIfPossible,
	Skip,
}

impl fmt::Debug for ImportStateAction {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let action = match self {
			Self::StateApply => "state_apply",
			Self::StateImport => "state_import",
			Self::Execute => "execute",
			Self::ExecuteIfPossible => "execute_if_possible",
			Self::Skip => "skip",
		};
		f.write_str(action)
	}
}

impl<B: BlockT> ImportContext<B> {
	fn from_block<C>(client: &C, block: &mut BlockImportParams<B>) -> Self
	where
		C: HeaderBackend<B> + BlockBackend<B>,
	{
		let hash = block.post_hash();
		let number = *block.header.number();
		let parent_hash = *block.header.parent_hash();
		// Prematurely set fork choice = false to avoid any chance of setting best block.
		block.fork_choice = Some(ForkChoiceStrategy::Custom(false));
		let info = client.info();
		let is_block_gap = info.block_gap.is_some_and(|a| a.start <= number && number <= a.end);
		let parent_block_state = client.block_status(parent_hash).unwrap_or(BlockStatus::Unknown);
		let block_header_status =
			client.status(hash).unwrap_or(sp_blockchain::BlockStatus::Unknown);
		let state_action = match block.state_action {
			StateAction::ApplyChanges(StorageChanges::Changes(_)) => ImportStateAction::StateApply,
			StateAction::ApplyChanges(StorageChanges::Import(_)) => ImportStateAction::StateImport,
			StateAction::Execute => ImportStateAction::Execute,
			StateAction::ExecuteIfPossible => ImportStateAction::ExecuteIfPossible,
			StateAction::Skip => ImportStateAction::Skip,
		};
		Self {
			hash,
			number,
			parent_hash,
			info,
			is_block_gap,
			parent_block_state,
			block_header_status,
			state_action,
			skip_execution_checks: block.state_action.skip_execution_checks(),
			has_body: block.body.is_some(),
			origin: block.origin,
			finalized: block.finalized,
		}
	}

	fn refresh<C>(&mut self, client: &C, block: &mut BlockImportParams<B>)
	where
		C: HeaderBackend<B> + BlockBackend<B>,
	{
		*self = Self::from_block(client, block);
	}

	fn is_parent_state_available(&self) -> bool {
		self.parent_block_state == BlockStatus::InChainWithState
	}

	fn is_parent_unknown(&self) -> bool {
		self.parent_block_state == BlockStatus::Unknown
	}

	fn is_header_already_imported(&self) -> bool {
		self.block_header_status == sp_blockchain::BlockStatus::InChain
	}

	fn is_my_block(&self) -> bool {
		self.origin == BlockOrigin::Own
	}

	fn is_initial_sync(&self) -> bool {
		self.origin == BlockOrigin::NetworkInitialSync
	}

	fn can_defer_full_import(&self) -> bool {
		self.has_body && !self.finalized && !self.is_my_block()
	}

	fn supports_deferred_full_import(&self) -> bool {
		matches!(
			self.state_action,
			ImportStateAction::Execute | ImportStateAction::ExecuteIfPossible
		)
	}

	fn can_defer_notebook_verification(&self) -> bool {
		self.supports_deferred_full_import() && self.can_defer_full_import()
	}

	fn are_import_details_already_queued(
		&self,
		is_full_import_already_queued: bool,
		has_justifications: bool,
	) -> bool {
		self.has_body &&
			!self.finalized &&
			!has_justifications &&
			self.supports_deferred_full_import() &&
			self.is_header_already_imported() &&
			is_full_import_already_queued
	}

	fn can_execute_block(&self) -> bool {
		self.state_action != ImportStateAction::ExecuteIfPossible ||
			self.is_parent_state_available()
	}

	fn should_verify_notebooks(&self) -> bool {
		!self.skip_execution_checks &&
			!self.is_initial_sync() &&
			!self.is_my_block() &&
			self.number > self.info.finalized_number &&
			!self.finalized
	}

	fn has_state_or_block(&self) -> bool {
		!self.skip_execution_checks
	}

	fn can_finalize_import(&self, is_finalized_descendent: bool) -> bool {
		is_finalized_descendent || self.is_initial_sync() || self.finalized
	}
}

#[allow(clippy::large_enum_variant)]
enum ImportGateOutcome<B: BlockT> {
	Continue { block: BlockImportParams<B>, import_context: ImportContext<B> },
	Return(ImportResult),
}

impl<B: BlockT, I, C: AuxStore, AC> ArgonBlockImport<B, I, C, AC> {
	pub(crate) fn new_with_components(
		inner: I,
		client: Arc<C>,
		aux_client: ArgonAux<B, C>,
		notary_client: Arc<NotaryClient<B, C, AC>>,
		metrics: Arc<Option<ImportMetrics>>,
	) -> Self {
		let pending_full_import_queue = PendingImportReplayQueue::<B, C>::new(client.clone());
		Self {
			inner,
			client,
			aux_client,
			notary_client,
			import_lock: Default::default(),
			pending_full_import_queue,
			metrics,
			_phantom: PhantomData,
		}
	}

	#[cfg(test)]
	pub(crate) async fn pending_full_imports_len(&self) -> usize {
		self.pending_full_import_queue.len().await
	}
}

impl<B: BlockT, I: Clone, C: AuxStore, AC: Codec> Clone for ArgonBlockImport<B, I, C, AC> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			client: self.client.clone(),
			aux_client: self.aux_client.clone(),
			notary_client: self.notary_client.clone(),
			import_lock: self.import_lock.clone(),
			pending_full_import_queue: self.pending_full_import_queue.clone(),
			metrics: self.metrics.clone(),
			_phantom: PhantomData,
		}
	}
}

impl<B, I, C, AC> ArgonBlockImport<B, I, C, AC>
where
	B: BlockT,
	I: BlockImport<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ImportApisExt<B, AC>
		+ crate::notary_client::NotaryApisExt<B, AC>
		+ AuxStore
		+ Send
		+ Sync
		+ 'static,
	AC: Clone + Codec + Send + Sync + 'static,
{
	fn finalize_gate_outcome(
		block: BlockImportParams<B>,
		import_context: ImportContext<B>,
	) -> Result<ImportGateOutcome<B>, Error> {
		let has_justifications = block
			.justifications
			.as_ref()
			.is_some_and(|justifications| justifications.iter().next().is_some());
		// If the header is already in the DB we usually short-circuit unless the
		// new import carries something we still need (state/body or finality/justification).
		if import_context.is_header_already_imported() &&
			!import_context.has_state_or_block() &&
			!block.finalized &&
			!has_justifications
		{
			return Ok(ImportGateOutcome::Return(ImportResult::AlreadyInChain));
		}
		Ok(ImportGateOutcome::Continue { block, import_context })
	}

	async fn queue_full_import_for_replay(
		&self,
		block: BlockImportParams<B>,
		import_context: &mut ImportContext<B>,
	) -> Result<DeferFullImportResult<B>, Error> {
		match self.pending_full_import_queue.defer_full_import(block).await? {
			DeferFullImportResult::Deferred(mut queued_block) => {
				import_context.refresh(&*self.client, &mut queued_block);
				Ok(DeferFullImportResult::Deferred(queued_block))
			},
			DeferFullImportResult::SaturatedHeaderOnly(mut header_only) => {
				import_context.refresh(&*self.client, &mut header_only);
				Ok(DeferFullImportResult::SaturatedHeaderOnly(header_only))
			},
		}
	}

	async fn defer_full_import_or_missing_state(
		&self,
		block: BlockImportParams<B>,
		import_context: &mut ImportContext<B>,
	) -> Result<Option<BlockImportParams<B>>, Error> {
		match self.queue_full_import_for_replay(block, import_context).await {
			Ok(DeferFullImportResult::Deferred(queued_block)) => Ok(Some(queued_block)),
			Ok(DeferFullImportResult::SaturatedHeaderOnly(header_only_block)) => {
				warn!(
					context = ?import_context,
					capacity = crate::pending_import_replay::MAX_PENDING_IMPORTS,
					"Pending replay queue full; importing header-only without deferring full-body replay"
				);
				Ok(Some(header_only_block))
			},
			Err(Error::PendingImportUnsupported(reason)) => {
				warn!(
					context = ?import_context,
					?reason,
					"Deferred replay unsupported for this block; returning MissingState"
				);
				Ok(None)
			},
			Err(err) => Err(err),
		}
	}

	async fn run_pre_import_gates(
		&self,
		mut block: BlockImportParams<B>,
		mut import_context: ImportContext<B>,
		notary_client: &Arc<NotaryClient<B, C, AC>>,
		block_hash: B::Hash,
		block_number: NumberFor<B>,
		parent_hash: B::Hash,
	) -> Result<ImportGateOutcome<B>, Error> {
		let queued_before_import = self.pending_full_import_queue.has_hash(block_hash).await;
		let has_justifications = block
			.justifications
			.as_ref()
			.is_some_and(|justifications| justifications.iter().next().is_some());
		if import_context
			.are_import_details_already_queued(queued_before_import, has_justifications)
		{
			return Ok(ImportGateOutcome::Return(ImportResult::AlreadyInChain));
		}

		if !import_context.can_execute_block() {
			// Full execution blocked by missing parent state: queue full block and import header
			// path.
			if import_context.is_parent_unknown() || !import_context.can_defer_full_import() {
				return Ok(ImportGateOutcome::Return(ImportResult::MissingState));
			}
			let Some(queued_block) =
				self.defer_full_import_or_missing_state(block, &mut import_context).await?
			else {
				return Ok(ImportGateOutcome::Return(ImportResult::MissingState));
			};
			block = queued_block;
			return Self::finalize_gate_outcome(block, import_context);
		}

		if !import_context.should_verify_notebooks() {
			return Self::finalize_gate_outcome(block, import_context);
		}

		// Notebook verification requires parent state. If it's unavailable, defer or block.
		if !import_context.is_parent_state_available() {
			if import_context.is_parent_unknown() ||
				!import_context.can_defer_notebook_verification()
			{
				return Ok(ImportGateOutcome::Return(ImportResult::MissingState));
			}
			let Some(queued_block) =
				self.defer_full_import_or_missing_state(block, &mut import_context).await?
			else {
				return Ok(ImportGateOutcome::Return(ImportResult::MissingState));
			};
			block = queued_block;
			return Self::finalize_gate_outcome(block, import_context);
		}

		let digest_notebooks =
			self.client.runtime_digest_notebooks(parent_hash, block.header.digest())?;
		if digest_notebooks.is_empty() {
			return Self::finalize_gate_outcome(block, import_context);
		}
		if let Err(err) = notary_client
			.verify_notebook_audits_for_import(
				&parent_hash,
				digest_notebooks,
				IMPORT_NOTEBOOK_AUDIT_TIMEOUT,
			)
			.await
		{
			match err {
				err if err.is_retryable_notebook_audit_error() =>
					if import_context.can_defer_notebook_verification() {
						let Some(queued_block) = self
							.defer_full_import_or_missing_state(block, &mut import_context)
							.await?
						else {
							return Ok(ImportGateOutcome::Return(ImportResult::MissingState));
						};
						block = queued_block;
					} else {
						return Err(err);
					},
				Error::InvalidNotebookDigest(_) => {
					warn!(
						number = ?block_number,
						block_hash = ?block_hash,
						parent_hash = ?parent_hash,
						origin = ?block.origin,
						"Rejecting block with invalid notebook digest: {err}"
					);
					return Ok(ImportGateOutcome::Return(ImportResult::KnownBad));
				},
				e => return Err(e),
			}
		}

		Self::finalize_gate_outcome(block, import_context)
	}

	pub(crate) async fn replay_pending_full_imports(&self) {
		let pending_count = self.pending_full_import_queue.len().await;
		if pending_count == 0 {
			return;
		}

		let mut replayed = 0usize;
		while let Some((pending_import, replay_context)) = self
			.pending_full_import_queue
			.dequeue_ready_for_replay(&self.notary_client)
			.await
		{
			replayed = replayed.saturating_add(1);
			let mut replay_retry_block =
				PendingImportReplayQueue::<B, C>::retry_block_from_pending(&pending_import);
			match <Self as BlockImport<B>>::import_block(self, pending_import.block).await {
				Ok(ImportResult::KnownBad) => {
					warn!(
						block_hash = ?replay_context.hash,
						number = ?replay_context.number,
						"Pending full block import replay resolved as known-bad"
					);
				},
				Ok(ImportResult::MissingState) => {
					warn!(
						block_hash = ?replay_context.hash,
						number = ?replay_context.number,
						"Pending full block replay still missing state; requeueing"
					);
					if let Some(block) = replay_retry_block.take() {
						self.pending_full_import_queue.requeue_retry_block(block).await;
					}
				},
				Ok(ImportResult::AlreadyInChain) => {
					let state_status = self
						.client
						.block_status(replay_context.hash)
						.unwrap_or(BlockStatus::Unknown);
					if state_status != BlockStatus::InChainWithState {
						warn!(
							block_hash = ?replay_context.hash,
							number = ?replay_context.number,
							?state_status,
							"Pending full block replay returned AlreadyInChain without state; requeueing"
						);
						if let Some(block) = replay_retry_block.take() {
							self.pending_full_import_queue.requeue_retry_block(block).await;
						}
					}
				},
				Ok(_) => {},
				Err(err) => {
					warn!(
						block_hash = ?replay_context.hash,
						number = ?replay_context.number,
						error = ?err,
						"Pending full block replay failed; requeueing"
					);
					if let Some(block) = replay_retry_block.take() {
						self.pending_full_import_queue.requeue_retry_block(block).await;
					}
				},
			}
		}
	}
}

#[async_trait::async_trait]
impl<B, I, C, AC> BlockImport<B> for ArgonBlockImport<B, I, C, AC>
where
	B: BlockT,
	I: BlockImport<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ImportApisExt<B, AC>
		+ crate::notary_client::NotaryApisExt<B, AC>
		+ AuxStore
		+ Send
		+ Sync
		+ 'static,
	AC: Clone + Codec + Send + Sync + 'static,
{
	type Error = ConsensusError;

	async fn check_block(&self, block: BlockCheckParams<B>) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block.clone()).await.map_err(Into::into)
	}

	/// ARGON BLOCK IMPORT ‑ Quick‐reference (keep IDE‑friendly)
	///
	/// PIPELINE
	///   network → BasicQueue.check_block → Verifier.verify → *import_block* (this fn)
	///
	/// LOCKING
	///   - `import_lock` serialises every import so `client.info()` and aux writes remain atomic.
	///
	/// EARLY EXITS
	///   - Parent state missing + `ExecuteIfPossible` (detected *here*, not in `Verifier::verify`)
	///     → `ImportResult::MissingState` so `BasicQueue` can retry after parent state sync.
	///   - Header already in DB AND no new body/state → `ImportResult::AlreadyInChain`
	///
	/// FORK‑CHOICE
	///   - `fork_power > best_power`                                   ⇒ set_best = true
	///   - `fork_power == best_power` & `hash < best_hash`             ⇒ set_best = true
	///   - else                                                        ⇒ set_best = false
	///   - `set_best` additionally requires `has_state_or_block` & `can_finalize`
	///   - tie‑loser: `block.import_existing = true` + `aux.check_duplicate_block_at_tick(...)`
	///
	/// AUX WRITES
	///   - `clean_state_history()`      — winner of each `(height, power)`
	///   - `check_duplicate_block_at_tick()`  - block duplicated blocks
	///
	/// TYPICAL ENTRY VARIANTS
	///   - Gap header           : NetworkInitialSync + `Skip`                  → store header, not
	///     best
	///   - Warp header + state  : NetworkInitialSync + `Import(changes)`       → may become best
	///   - Gossip header + body : `ExecuteIfPossible`                          → exec or
	///     MissingState
	///   - Full block           : `ApplyChanges::Changes`                      → full import
	///   - Finalized block      : `finalized = true`                           → advance finalized
	///
	/// RE-IMPORTING THE SAME BLOCK
	/// Multiple legitimate paths can cause the same hash to arrive here again:
	///   • Gap header then later state: first call has state_action = Skip; the second
	///     call carries ApplyChanges::Import. This upgrades the header to
	///     InChainWithState because has_state_or_block is true.
	///   • Parent state race: a gossip block with ExecuteIfPossible beats its
	///     parent’s state. We return MissingState; BasicQueue retries the same
	///     block when the parent executes.
	///   • Multi-peer broadcast: two peers deliver an identical header; the second
	///     call hits AlreadyInChain and exits quickly.
	///   • Justification / finality upgrade: a header first imported without
	///     finality may be re-imported later with block.finalized = true or an
	///     attached justification. This second import can advance finality.
	///   • Tie‑loser replay: a block previously stored as non-best can be
	///     re-broadcast; import_existing = true makes the operation idempotent.
	///   • Node restart: after a restart the DB already has the header but peers
	///     replay it; AlreadyInChain handles the redundancy.
	///
	/// INVARIANTS
	///   - Deterministic fork‑choice (hash tie‑break).
	///   - ≤ 512/1024 children per height (fork‑tree limit).
	///   - Duplicate author spam at a tick rejected pre‑DB.
	///
	/// RETURN
	///   One of the standard `ImportResult` variants that BasicQueue converts
	///   into `BlockImportStatus`.
	async fn import_block(
		&self,
		mut block: BlockImportParams<B>,
	) -> Result<ImportResult, Self::Error> {
		// single thread the import queue to ensure we don't have multiple imports
		let _import_lock = self.import_lock.lock().await;
		let mut import_context = ImportContext::from_block(&*self.client, &mut block);
		let block_hash = import_context.hash;
		let block_number = import_context.number;
		let parent_hash = import_context.parent_hash;
		tracing::trace!(
			context = ?import_context,
			"Begin import."
		);

		match self
			.run_pre_import_gates(
				block,
				import_context,
				&self.notary_client,
				block_hash,
				block_number,
				parent_hash,
			)
			.await?
		{
			ImportGateOutcome::Continue { block: next_block, import_context: next_context } => {
				block = next_block;
				import_context = next_context;
			},
			ImportGateOutcome::Return(result) => return Ok(result),
		}
		let info = &import_context.info;

		let is_finalized_descendent = is_on_finalized_chain(
			&(*self.client),
			&block,
			&info.finalized_hash,
			info.finalized_number,
		)
		.unwrap_or_default();

		let can_finalize = import_context.can_finalize_import(is_finalized_descendent);

		let is_block_header_already_imported = import_context.is_header_already_imported();
		let has_state_or_block = import_context.has_state_or_block();
		// Otherwise (e.g. now finalized or now with state) we fall through and let
		// `inner.import_block` upgrade the existing header.
		let best_header = self
			.client
			.header(info.best_hash)
			.expect("Best header should always be available")
			.expect("Best header should always exist");

		let best_block_power = if best_header.number().is_zero() {
			ForkPower::default()
		} else {
			ForkPower::try_from(best_header.digest()).map_err(|e| {
				Error::MissingRuntimeData(format!("Failed to get best fork power: {e:?}"))
			})?
		};

		let mut set_to_best = false;
		let digest = block.header.digest();
		let block_author: AccountId = digest
			.convert_first(|a| a.as_author())
			.ok_or(Error::UnableToDecodeDigest("author".to_string()))?;
		let tick = digest
			.convert_first(|a| a.as_tick())
			.ok_or(Error::UnableToDecodeDigest("tick".to_string()))?
			.0;
		let fork_power = ForkPower::try_from(digest)
			.map_err(|e| Error::MissingRuntimeData(format!("Failed to get fork power: {e:?}")))?;

		if fork_power >= best_block_power {
			// NOTE: only import as best block if it beats the best stored block. There are cases
			// where importing a tie will yield too many blocks at a height and break substrate
			set_to_best = has_state_or_block && can_finalize;
			if set_to_best && fork_power.eq_weight(&best_block_power) {
				// if fork power is equal, choose a deterministic option to set the best block
				set_to_best = info.best_hash > block_hash;
				if !set_to_best {
					// this flag forces us to revalidate the block
					block.import_existing = true;
				}
			}
			if set_to_best {
				if *best_header.number() == block_number {
					tracing::info!(
						number=?block_number,
						beat_best_hash=?best_header.hash(),
						is_equal_weight=fork_power.eq_weight(&best_block_power),
						block_hash=?block_hash,
						is_finalized_descendent,
						finalized=block.finalized,
						"Replacing best block at number"
					);
				} else {
					tracing::info!(
						number=?block_number,
						prev_best_hash=?best_header.hash(),
						block_hash=?block_hash,
						is_finalized_descendent,
						finalized=block.finalized,
						"New best block imported"
					);
				}
			}
		}
		block.fork_choice = Some(ForkChoiceStrategy::Custom(set_to_best));

		if set_to_best {
			self.aux_client.clean_state_history(&mut block.auxiliary, tick)?;
		}

		let mut record_block_key_on_import = None;

		let is_vote_block = fork_power.is_latest_vote;
		if !import_context.is_block_gap &&
			!is_block_header_already_imported &&
			block.origin != BlockOrigin::NetworkInitialSync
		{
			// Block abuse prevention. Do not allow a block author to submit more than one vote
			// block per tick pertaining to the same voting key or more than one compute block with
			// the same voting power.
			let block_key = if is_vote_block {
				digest
					.convert_first(|a| a.as_parent_voting_key())
					.ok_or(Error::UnableToDecodeDigest("voting key".to_string()))?
					.parent_voting_key
					.unwrap_or(H256::zero())
			} else {
				H256::from(blake2_256(&fork_power.encode()))
			};
			record_block_key_on_import = Some(block_key);

			if self
				.aux_client
				.is_duplicated_block_key_for_author(&block_author, block_key, tick)
			{
				error!(
					?block_number,
					block_hash=?block_hash,
					?parent_hash,
					origin = ?block.origin,
					fork_power = ?fork_power,
					voting_key = ?block_key,
					"Author produced a duplicate block"
				);
				let block_type =
					if is_vote_block { "vote block" } else { "compute block" }.to_string();
				return Err(Error::DuplicateAuthoredBlock(
					block_author,
					block_type,
					block_key.to_string(),
				)
				.into());
			}
		}

		let res = self.inner.import_block(block).await.map_err(Into::into)?;
		if let Some(block_key) = record_block_key_on_import {
			// Record the block key on import, so that we can detect duplicate blocks
			// at a later time.
			self.aux_client.record_imported_block_key(
				block_author,
				block_key,
				tick,
				is_vote_block,
			)?;
			if let Some(metrics) = &*self.metrics {
				// Runtime-backed reads below require a successfully imported state.
				if has_state_or_block && !is_block_header_already_imported {
					if is_vote_block {
						metrics.imported_vote_sealed_blocks_total.with_label_values(&[]).inc();
					}
					if self.client.has_new_bitcoin_tip(block_hash) {
						metrics.imported_block_with_new_bitcoin_tip.with_label_values(&[]).inc();
					}
					if self.client.has_new_price_index(block_hash) {
						metrics.imported_block_with_new_price_index.with_label_values(&[]).inc();
					}
				}
			}
		}

		Ok(res)
	}
}

fn is_on_finalized_chain<C, B>(
	client: &C,
	block: &BlockImportParams<B>,
	finalized: &B::Hash,
	finalized_number: NumberFor<B>,
) -> Result<bool, Error>
where
	C: HeaderBackend<B>,
	B: BlockT,
{
	let mut number = *block.header.number();
	let mut parent_hash = *block.header.parent_hash();
	let mut block_hash = block.post_hash();
	while number >= finalized_number {
		if number == finalized_number {
			return Ok(block_hash == *finalized);
		}
		let header = client
			.header(parent_hash)?
			.ok_or(Error::BlockNotFound(format!("{parent_hash:?}")))?;
		number = *header.number();
		parent_hash = *header.parent_hash();
		block_hash = header.hash();
	}
	Ok(false)
}

#[async_trait::async_trait]
impl<B, I, C, AC> sc_consensus::JustificationImport<B> for ArgonBlockImport<B, I, C, AC>
where
	B: BlockT,
	I: sc_consensus::JustificationImport<B> + Send + Sync,
	C: AuxStore + Send + Sync,
	AC: Codec + Send + Sync,
{
	type Error = I::Error;

	async fn on_start(&mut self) -> Vec<(B::Hash, NumberFor<B>)> {
		self.inner.on_start().await
	}

	async fn import_justification(
		&mut self,
		hash: B::Hash,
		number: NumberFor<B>,
		justification: Justification,
	) -> Result<(), Self::Error> {
		self.inner.import_justification(hash, number, justification).await
	}
}

#[allow(dead_code)]
struct Verifier<B: BlockT, C: AuxStore, AC> {
	client: Arc<C>,
	utxo_tracker: Arc<UtxoTracker>,
	telemetry: Option<TelemetryHandle>,
	_phantom: PhantomData<(B, AC)>,
}

#[async_trait::async_trait]
impl<B: BlockT, C, AC> VerifierT<B> for Verifier<B, C, AC>
where
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + BlockBackend<B> + Send + Sync + AuxStore + 'static,
	C::Api: BlockBuilderApi<B>
		+ BitcoinApis<B, Balance>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ BlockCreatorApis<B, AC, NotebookVerifyError>
		+ NotebookApis<B, NotebookVerifyError>
		+ NotaryApis<B, NotaryRecordT>
		+ TickApis<B>,
	AC: Codec + Clone + Send + Sync + 'static,
{
	async fn verify(
		&self,
		mut block_params: BlockImportParams<B>,
	) -> Result<BlockImportParams<B>, String> {
		let block_number = *block_params.header.number();
		let is_gap_sync = self
			.client
			.info()
			.block_gap
			.is_some_and(|gap| gap.start <= block_number && block_number <= gap.end);

		let post_hash = block_params.header.hash();
		let parent_hash = *block_params.header.parent_hash();
		let mut header = block_params.header;
		let raw_seal_digest = header.digest_mut().pop().ok_or(Error::MissingBlockSealDigest)?;
		let seal_digest = BlockSealDigest::try_from(&raw_seal_digest)
			.ok_or(Error::UnableToDecodeDigest("Seal Digest".into()))?;

		block_params.header = header;
		block_params.post_digests.push(raw_seal_digest);
		block_params.post_hash = Some(post_hash);
		// Skip checks that include execution, if being told so, or when importing only state. Runs
		// after post hash to preserve that logic across imports.
		//
		// This is done, for example, when gap syncing, and it's expected that the block after the
		// gap was checked/chosen properly, e.g. by warp syncing to this block using a finality
		// proof.
		if is_gap_sync ||
			block_params.state_action.skip_execution_checks() ||
			(block_params.with_state() && block_params.body.is_none())
		{
			// In the verifier, we don't want to set a best block yet. Wait for state to import
			block_params.fork_choice = Some(ForkChoiceStrategy::Custom(false));
			return Ok(block_params);
		}

		if block_params.body.is_some() &&
			self.client.block_status(parent_hash).unwrap_or(BlockStatus::Unknown) !=
				BlockStatus::InChainWithState
		{
			// Parent state is *not* available yet (pruned or unknown).
			//
			// IMPORTANT: The `Verifier` trait cannot signal `MissingState` (it returns only
			// `Result<BlockImportParams<B>, String>`). We therefore:
			//   * skip heavy runtime verification (since we cannot execute),
			//   * leave `block_params.state_action` unchanged (e.g. `ExecuteIfPossible`),
			//   * return the params so that `ArgonBlockImport::import_block` can detect the missing
			//     parent state and return `ImportResult::MissingState`, which `BasicQueue`
			//     understands and will retry once the parent’s state becomes available.
			return Ok(block_params);
		}

		// The import queue can import headers and also blocks. Sometimes these blocks come in and
		// the parent state isn't yet available
		if let Some(inner_body) = block_params.body.clone() {
			let runtime_api = self.client.runtime_api();

			let digest = block_params.header.digest();
			let pre_hash = block_params.header.hash();

			// TODO: should we move all of this to the runtime? Main holdup is building randomx for
			// 	wasm
			if seal_digest.is_vote() {
				let is_valid = runtime_api
					.is_valid_signature(parent_hash, pre_hash, &seal_digest, digest)
					.map_err(|e| format!("Error verifying miner signature {e:?}"))?;
				if !is_valid {
					return Err(Error::InvalidVoteSealSignature.into());
				}
			}

			// NOTE: we verify compute nonce in import queue because we use the pre-hash, which
			// we'd have to inject into the runtime
			if let BlockSealDigest::Compute { nonce } = &seal_digest {
				let compute_puzzle = runtime_api
					.compute_puzzle(parent_hash)
					.map_err(|e| format!("Error calling compute difficulty api {e:?}"))?;

				let key_block_hash = compute_puzzle.get_key_block(self.client.info().genesis_hash);
				let compute_difficulty = compute_puzzle.difficulty;

				tracing::info!(?key_block_hash, ?compute_difficulty, ?nonce, block_hash=?post_hash, "Verifying compute nonce");
				if !BlockComputeNonce::is_valid(
					nonce,
					pre_hash.as_ref().to_vec(),
					&key_block_hash,
					compute_difficulty,
				) {
					tracing::warn!(?key_block_hash, ?compute_difficulty, ?nonce, block_hash=?post_hash, pre_hash=?pre_hash, "Invalid compute nonce!");
					return Err(Error::InvalidComputeNonce.into());
				}
			}

			let check_block = B::new(block_params.header.clone(), inner_body);

			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			let seal =
				BlockSealInherentDataProvider { seal: None, digest: Some(seal_digest.clone()) };
			let inherent_data_providers = (timestamp, seal);

			let mut inherent_data = inherent_data_providers
				.create_inherent_data()
				.await
				.map_err(Error::CreateInherents)?;

			if let Ok(Some(bitcoin_utxo_sync)) =
				get_bitcoin_inherent(&self.utxo_tracker, &self.client, &parent_hash)
			{
				BitcoinInherentDataProvider { bitcoin_utxo_sync }
					.provide_inherent_data(&mut inherent_data)
					.await
					.map_err(Error::CreateInherents)?;
			}

			// inherent data passed in is what we would have generated...
			let inherent_res = runtime_api
				.check_inherents(parent_hash, check_block.clone().into(), inherent_data)
				.map_err(|e| Error::Client(e.into()))?;

			if !inherent_res.ok() {
				for (identifier, error) in inherent_res.into_errors() {
					match inherent_data_providers.try_handle_error(&identifier, &error).await {
						Some(res) => res.map_err(Error::CheckInherents)?,
						None => return Err(Error::CheckInherentsUnknownError(identifier).into()),
					}
				}
			}
		}

		Ok(block_params)
	}
}

/// Start an import queue which checks blocks' seals and inherent data.
#[allow(clippy::too_many_arguments)]
pub fn create_import_queue<C, B, I, AC>(
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	notary_client: Arc<NotaryClient<B, C, AC>>,
	justification_import: Option<BoxJustificationImport<B>>,
	block_import: I,
	spawner: &impl sp_core::traits::SpawnEssentialNamed,
	registry: Option<&substrate_prometheus_endpoint::Registry>,
	telemetry: Option<TelemetryHandle>,
	utxo_tracker: Arc<UtxoTracker>,
) -> (BasicQueue<B>, ArgonBlockImport<B, I, C, AC>)
where
	B: BlockT,
	I: BlockImport<B> + Clone + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + BlockBackend<B> + Send + Sync + AuxStore + 'static,
	C::Api: BlockBuilderApi<B>
		+ BlockCreatorApis<B, AC, NotebookVerifyError>
		+ BlockImportApis<B>
		+ BitcoinApis<B, Balance>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ NotebookApis<B, NotebookVerifyError>
		+ NotaryApis<B, NotaryRecordT>
		+ TickApis<B>,
	AC: Codec + Clone + Send + Sync + 'static,
	I: BlockImport<B, Error = ConsensusError> + Send + Sync + 'static,
{
	let metrics = registry.and_then(|r| ImportMetrics::new(r).ok());
	let metrics = Arc::new(metrics);

	let importer = ArgonBlockImport::new_with_components(
		block_import,
		client.clone(),
		aux_client,
		notary_client.clone(),
		metrics,
	);
	let replay_importer = importer.clone();
	spawn_pending_import_replay_task(spawner, move || {
		let replay_importer = replay_importer.clone();
		async move { replay_importer.replay_pending_full_imports().await }
	});

	let verifier = Verifier::<B, C, AC> {
		client: client.clone(),
		utxo_tracker,
		telemetry,
		_phantom: PhantomData,
	};

	(
		BasicQueue::new(
			verifier,
			Box::new(importer.clone()),
			justification_import,
			spawner,
			registry,
		),
		importer,
	)
}
