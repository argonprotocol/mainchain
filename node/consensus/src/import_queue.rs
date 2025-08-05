use crate::{NotaryClient, aux_client::ArgonAux, compute_worker::BlockComputeNonce, error::Error};
use argon_bitcoin_utxo_tracker::{UtxoTracker, get_bitcoin_inherent};
use argon_primitives::{
	AccountId, Balance, BitcoinApis, BlockCreatorApis, BlockSealApis, BlockSealAuthorityId,
	BlockSealDigest, NotaryApis, NotebookApis, TickApis,
	digests::ArgonDigests,
	fork_power::ForkPower,
	inherents::{BitcoinInherentDataProvider, BlockSealInherentDataProvider},
};
use argon_runtime::{NotaryRecordT, NotebookVerifyError};
use codec::{Codec, Encode};
use polkadot_sdk::{
	sp_core::{H256, blake2_256},
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
use std::{marker::PhantomData, sync::Arc};
use tracing::error;

/// A block importer for argon.
pub struct ArgonBlockImport<B: BlockT, I, C: AuxStore, AC> {
	inner: I,
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	import_lock: Arc<tokio::sync::Mutex<()>>,
	_phantom: PhantomData<AC>,
}

impl<B: BlockT, I: Clone, C: AuxStore, AC: Codec> Clone for ArgonBlockImport<B, I, C, AC> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			client: self.client.clone(),
			aux_client: self.aux_client.clone(),
			import_lock: self.import_lock.clone(),
			_phantom: PhantomData,
		}
	}
}

#[async_trait::async_trait]
impl<B, I, C, AC> BlockImport<B> for ArgonBlockImport<B, I, C, AC>
where
	B: BlockT,
	I: BlockImport<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: HeaderBackend<B> + BlockBackend<B> + AuxStore + Send + Sync + 'static,
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
		let block_hash = block.post_hash();
		let block_number = *block.header.number();
		let parent_hash = *block.header.parent_hash();

		let info = self.client.info();
		let is_block_gap =
			info.block_gap.is_some_and(|a| a.start <= block_number && block_number <= a.end);
		let parent_block_state =
			self.client.block_status(parent_hash).unwrap_or(BlockStatus::Unknown);
		let block_status =
			self.client.status(block_hash).unwrap_or(sp_blockchain::BlockStatus::Unknown);
		// NOTE: do not access runtime here without knowing for CERTAIN state is successfully
		// imported. Various sync strategies will access this path without state set yet
		tracing::trace!(
			number=?block_number,
			hash=?block_hash,
			parent=?parent_hash,
			is_block_gap,
			action=match block.state_action {
				StateAction::ApplyChanges(StorageChanges::Changes(_)) => "state_apply",
				StateAction::ApplyChanges(StorageChanges::Import(_)) => "state_import",
				StateAction::Execute => "execute",
				StateAction::ExecuteIfPossible => "execute_if_possible",
				StateAction::Skip => "skip",
			},
			origin=?block.origin,
			parent_block_state=?parent_block_state,
			block_status=?block_status,
			"Begin import."
		);

		if matches!(block.state_action, StateAction::ExecuteIfPossible) &&
			parent_block_state != BlockStatus::InChainWithState
		{
			tracing::debug!(?block_hash, ?block_number, parent=?parent_hash, "Parent state missing; returning missing state action");
			return Ok(ImportResult::MissingState);
		}

		let is_finalized_descendent = is_on_finalized_chain(
			&(*self.client),
			&block,
			&info.finalized_hash,
			info.finalized_number,
		)
		.unwrap_or_default();

		let can_finalize = is_finalized_descendent ||
			block.origin == BlockOrigin::NetworkInitialSync ||
			block.finalized;

		let is_block_already_imported = block_status == sp_blockchain::BlockStatus::InChain;
		let has_state_or_block = !block.state_action.skip_execution_checks();
		// If the header is already in the DB we usually short‑circuit unless the
		// new import carries *something* we still need (state/body *or* finality).
		if is_block_already_imported && !has_state_or_block && !block.finalized {
			tracing::debug!(
				?block_hash,
				?block_number,
				"Skipping reimport of known block without state or finalization"
			);
			return Ok(ImportResult::AlreadyInChain);
		}
		// Otherwise (e.g. now finalized or now with state) we fall through and let
		// `inner.import_block` upgrade the existing header.
		let best_header = self
			.client
			.header(info.best_hash)
			.expect("Best header should always be available")
			.expect("Best header should always exist");

		let best_block_power = if info.best_number.is_zero() {
			ForkPower::default()
		} else {
			ForkPower::try_from(best_header.digest()).map_err(|e| {
				Error::MissingRuntimeData(format!("Failed to get best fork power: {:?}", e))
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
			.map_err(|e| Error::MissingRuntimeData(format!("Failed to get fork power: {:?}", e)))?;

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
			tracing::info!(
				number=?block_number,
				block_hash = ?block_hash,
				can_finalize = is_finalized_descendent,
				finalized = block.finalized,
				set_to_best,
				"New best fork imported"
			);
		}
		block.fork_choice = Some(ForkChoiceStrategy::Custom(set_to_best));

		if set_to_best {
			self.aux_client.clean_state_history(&mut block.auxiliary, tick)?;
		}

		let mut record_block_key_on_import = None;

		let is_vote_block = fork_power.is_latest_vote;
		let block_type = if is_vote_block { "vote block" } else { "compute block" }.to_string();
		if !is_block_gap && !is_block_already_imported {
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
			.ok_or(Error::BlockNotFound(format!("{:?}", parent_hash)))?;
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
	notary_client: Arc<NotaryClient<B, C, AC>>,
	utxo_tracker: Arc<UtxoTracker>,
	telemetry: Option<TelemetryHandle>,
	_phantom: PhantomData<AC>,
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
		// Skip checks that include execution, if being told so, or when importing only state.
		//
		// This is done for example when gap syncing and it is expected that the block after the gap
		// was checked/chosen properly, e.g. by warp syncing to this block using a finality proof.
		if is_gap_sync ||
			block_params.state_action.skip_execution_checks() ||
			block_params.with_state()
		{
			// When we are importing only the state of a block or its from network sync, it will be
			// the best block.
			block_params.fork_choice = Some(ForkChoiceStrategy::Custom(
				block_params.with_state() || block_params.origin == BlockOrigin::NetworkInitialSync,
			));
			return Ok(block_params);
		}

		let post_hash = block_params.header.hash();
		let parent_hash = *block_params.header.parent_hash();
		let mut header = block_params.header;
		let raw_seal_digest = header.digest_mut().pop().ok_or(Error::MissingBlockSealDigest)?;
		let seal_digest = BlockSealDigest::try_from(&raw_seal_digest)
			.ok_or(Error::UnableToDecodeDigest("Seal Digest".into()))?;

		block_params.header = header;
		block_params.post_digests.push(raw_seal_digest);
		block_params.post_hash = Some(post_hash);
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
					.map_err(|e| format!("Error verifying miner signature {:?}", e))?;
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

			// if we're importing a non-finalized block from someone else, verify the notebook
			// audits
			let latest_verified_finalized = self.client.info().finalized_number;
			if !matches!(block_params.origin, BlockOrigin::Own | BlockOrigin::NetworkInitialSync) &&
				block_number > latest_verified_finalized &&
				!block_params.finalized
			{
				let digest_notebooks = runtime_api
					.digest_notebooks(parent_hash, digest)
					.map_err(|e| format!("Error calling digest notebooks api {e:?}"))?
					.map_err(|e| format!("Failed to get digest notebooks: {:?}", e))?;
				self.notary_client
					.verify_notebook_audits(&parent_hash, digest_notebooks)
					.await
					.inspect_err(|e| {
						error!(
						?block_number,
						block_hash=?post_hash,
						?parent_hash,
						origin = ?block_params.origin,
						import_existing = block_params.import_existing,
						finalized = block_params.finalized,
						has_justification = block_params.justifications.is_some(),
						"Failed to verify notebook audits {}", e.to_string())
					})?;
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
				.check_inherents(parent_hash, check_block.clone(), inherent_data)
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
		+ BitcoinApis<B, Balance>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ NotebookApis<B, NotebookVerifyError>
		+ NotaryApis<B, NotaryRecordT>
		+ TickApis<B>,
	AC: Codec + Clone + Send + Sync + 'static,
	I: BlockImport<B, Error = ConsensusError> + Send + Sync + 'static,
{
	let importer = ArgonBlockImport {
		inner: block_import,
		client: client.clone(),
		aux_client,
		import_lock: Default::default(),
		_phantom: PhantomData,
	};
	let verifier = Verifier::<B, C, AC> {
		client: client.clone(),
		utxo_tracker,
		notary_client,
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

#[cfg(test)]
mod test {
	use super::*;
	use argon_primitives::{
		BlockSealAuthoritySignature, ComputeDifficulty, Digestset, FORK_POWER_DIGEST,
		HashOutput as BlockHash, NotebookDigest, PARENT_VOTING_KEY_DIGEST, ParentVotingKeyDigest,
		VotingKey,
		prelude::{
			sp_runtime::{generic::SignedBlock, traits::BlakeTwo256},
			*,
		},
	};
	use async_trait::async_trait;
	use polkadot_sdk::{
		frame_support::{BoundedVec, assert_ok},
		sc_client_api::KeyValueStates,
		sc_consensus::ImportedState,
		sp_core::{ByteArray, U256},
		sp_runtime::DigestItem,
	};
	use sc_consensus::{
		BlockCheckParams, BlockImportParams, ImportResult, ImportedAux, StateAction::*,
	};
	use sp_blockchain::{BlockStatus, Error as BlockchainError, HeaderBackend};
	use sp_runtime::{
		Digest, OpaqueExtrinsic as UncheckedExtrinsic, generic,
		traits::{Block as BlockT, Header as HeaderT, NumberFor},
	};
	use std::{
		collections::{BTreeMap, HashMap},
		sync::{Arc, Mutex},
	};
	// -------------------------------------------
	// Tiny in–memory client & mini importer
	// -------------------------------------------

	#[derive(Clone)]
	pub struct MemChain {
		headers: Arc<Mutex<HashMap<BlockHash, Header>>>,
		block_status: Arc<Mutex<HashMap<BlockHash, BlockStatus>>>,
		block_state: Arc<Mutex<HashMap<BlockHash, sp_consensus::BlockStatus>>>,
		best: Arc<Mutex<(BlockNumber, BlockHash)>>,
		finalized: Arc<Mutex<(BlockNumber, BlockHash)>>,
		aux: Arc<Mutex<BTreeMap<Vec<u8>, Vec<u8>>>>,
	}
	impl MemChain {
		pub fn new(genesis: Header) -> Self {
			let h = genesis.hash();
			Self {
				headers: Arc::new(Mutex::new([(h, genesis)].into())),
				block_status: Arc::new(Mutex::new([(h, BlockStatus::InChain)].into())),
				block_state: Arc::new(Mutex::new(
					[(h, sp_consensus::BlockStatus::InChainWithState)].into(),
				)),
				best: Arc::new(Mutex::new((0u32, h))),
				finalized: Arc::new(Mutex::new((0u32, h))),
				aux: Arc::new(Mutex::new(BTreeMap::new())),
			}
		}
		pub fn insert(&self, hdr: Header) {
			let h = hdr.hash();
			self.block_status.lock().unwrap().insert(h, BlockStatus::InChain);
			self.block_state
				.lock()
				.unwrap()
				.insert(h, sp_consensus::BlockStatus::InChainPruned); // header only, no state yet
			self.headers.lock().unwrap().insert(h, hdr);
		}

		pub fn set_state(&self, h: BlockHash, state: sp_consensus::BlockStatus) {
			self.block_status.lock().unwrap().insert(h, BlockStatus::InChain);
			self.block_state.lock().unwrap().insert(h, state);
		}
	}
	impl HeaderBackend<Block> for MemChain {
		fn header(&self, h: BlockHash) -> Result<Option<Header>, BlockchainError> {
			Ok(self.headers.lock().unwrap().get(&h).cloned())
		}
		fn info(&self) -> sp_blockchain::Info<Block> {
			let best = *self.best.lock().unwrap();
			let fin = *self.finalized.lock().unwrap();
			sp_blockchain::Info {
				finalized_hash: fin.1,
				finalized_number: fin.0,
				finalized_state: None,
				best_hash: best.1,
				best_number: best.0,
				block_gap: None,
				genesis_hash: best.1,
				number_leaves: 0,
			}
		}
		fn status(&self, h: BlockHash) -> Result<BlockStatus, BlockchainError> {
			Ok(if self.headers.lock().unwrap().contains_key(&h) {
				BlockStatus::InChain
			} else {
				BlockStatus::Unknown
			})
		}

		fn number(&self, hash: BlockHash) -> sp_blockchain::Result<Option<BlockNumber>> {
			Ok(self.headers.lock().unwrap().get(&hash).map(|h| *h.number()))
		}

		fn hash(&self, number: NumberFor<Block>) -> sp_blockchain::Result<Option<BlockHash>> {
			Ok(self
				.headers
				.lock()
				.unwrap()
				.values()
				.find(|h| *h.number() == number)
				.map(|h| h.hash()))
		}
	}

	impl BlockBackend<Block> for MemChain {
		fn block_body(
			&self,
			_hash: BlockHash,
		) -> sc_client_api::blockchain::Result<Option<Vec<UncheckedExtrinsic>>> {
			Ok(Some(Vec::new()))
		}
		fn block(&self, hash: BlockHash) -> sp_blockchain::Result<Option<SignedBlock<Block>>> {
			if let Some(header) = self.headers.lock().unwrap().get(&hash) {
				Ok(Some(SignedBlock {
					block: Block::new(header.clone(), Vec::new()),
					justifications: None,
				}))
			} else {
				Ok(None)
			}
		}
		fn block_status(
			&self,
			hash: BlockHash,
		) -> sc_client_api::blockchain::Result<sp_consensus::BlockStatus> {
			let map = self.block_state.lock().unwrap();
			Ok(map.get(&hash).cloned().unwrap_or(sp_consensus::BlockStatus::Unknown))
		}

		fn justifications(
			&self,
			_hash: BlockHash,
		) -> sc_client_api::blockchain::Result<Option<sp_runtime::Justifications>> {
			Ok(None)
		}

		fn block_indexed_body(
			&self,
			_hash: BlockHash,
		) -> sc_client_api::blockchain::Result<Option<Vec<Vec<u8>>>> {
			Ok(Some(Vec::new()))
		}

		fn requires_full_sync(&self) -> bool {
			false
		}

		fn block_hash(&self, number: NumberFor<Block>) -> sp_blockchain::Result<Option<BlockHash>> {
			Ok(self
				.headers
				.lock()
				.unwrap()
				.values()
				.find(|h| *h.number() == number)
				.map(|h| h.hash()))
		}

		fn indexed_transaction(&self, _hash: BlockHash) -> sp_blockchain::Result<Option<Vec<u8>>> {
			Ok(None)
		}
	}

	impl AuxStore for MemChain {
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
			let mut aux = self.aux.lock().expect("aux is poisoned");
			for (k, v) in insert {
				aux.insert(k.to_vec(), v.to_vec());
			}
			for k in delete {
				aux.remove(*k);
			}
			Ok(())
		}

		fn get_aux(&self, key: &[u8]) -> sc_client_api::blockchain::Result<Option<Vec<u8>>> {
			let aux = self.aux.lock().unwrap();
			Ok(aux.get(key).cloned())
		}
	}

	/// Opaque block header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	type Block = generic::Block<Header, UncheckedExtrinsic>;

	#[async_trait]
	impl sc_consensus::BlockImport<Block> for MemChain {
		type Error = ConsensusError;

		async fn check_block(
			&self,
			_: BlockCheckParams<Block>,
		) -> Result<ImportResult, Self::Error> {
			Ok(ImportResult::Imported(ImportedAux::default()))
		}

		async fn import_block(
			&self,
			params: BlockImportParams<Block>,
		) -> Result<ImportResult, Self::Error> {
			let num = *params.header.number();
			let hash = params.header.hash();
			// store/overwrite header so later calls see it in MemChain
			self.insert(params.header.clone());
			match params.state_action {
				ApplyChanges(_) | Execute =>
					self.set_state(hash, sp_consensus::BlockStatus::InChainWithState),
				ExecuteIfPossible => {
					// If the parent already has full state we can execute immediately.
					let p_hash = *params.header.parent_hash();
					let parent_has_state = matches!(
						self.block_state.lock().unwrap().get(&p_hash),
						Some(sp_consensus::BlockStatus::InChainWithState)
					);
					if parent_has_state {
						self.set_state(hash, sp_consensus::BlockStatus::InChainWithState);
					} else {
						// header‑only until the parent’s state arrives
						// (behaves like real client which retries later)
					}
				},
				Skip => {}, // header only
			}
			// minimal: just mark best/finalized if fork_choice says so
			if let Some(fork_choice) = params.fork_choice {
				match fork_choice {
					sc_consensus::ForkChoiceStrategy::Custom(set_best) if set_best => {
						let mut best = self.best.lock().unwrap();
						if num > best.0 || (num == best.0 && hash != best.1) {
							*best = (num, hash);
						}
					},
					_ => {},
				}
			}
			if params.finalized {
				let mut fin = self.finalized.lock().unwrap();
				if num > fin.0 || (num == fin.0 && hash != fin.1) {
					*fin = (num, hash);
				}
			}
			Ok(ImportResult::Imported(ImportedAux::default()))
		}
	}

	fn create_header(
		number: BlockNumber,
		parent: H256,
		compute_difficulty: ComputeDifficulty,
		voting_key: Option<VotingKey>,
		author: AccountId,
	) -> (Header, DigestItem) {
		let (digest, block_digest) = create_digest(compute_difficulty, voting_key, author);
		(Header::new(number, H256::zero(), H256::zero(), parent, digest), block_digest)
	}
	fn has_state(db: &MemChain, h: H256) -> bool {
		matches!(
			db.block_state.lock().unwrap().get(&h),
			Some(sp_consensus::BlockStatus::InChainWithState)
		)
	}

	fn create_digest(
		compute_difficulty: ComputeDifficulty,
		voting_key: Option<VotingKey>,
		author: AccountId,
	) -> (Digest, DigestItem) {
		let mut power = ForkPower::default();
		if compute_difficulty > 0 {
			power.add(0, 0, BlockSealDigest::Compute { nonce: U256::zero() }, compute_difficulty);
		} else {
			power.add(
				500,
				1,
				BlockSealDigest::Vote {
					seal_strength: U256::one(),
					xor_distance: Some(U256::one()),
					signature: BlockSealAuthoritySignature::from_slice(&[0u8; 64])
						.expect("serialization of block seal strength failed"),
				},
				0,
			);
		}
		let digests = Digestset {
			author,
			block_vote: Default::default(),
			voting_key: None, // runtime digest
			fork_power: None, // runtime digest
			tick: Default::default(),
			notebooks: NotebookDigest::<NotebookVerifyError> { notebooks: BoundedVec::new() },
		};
		let mut digest = digests.create_pre_runtime_digest();
		digest.push(DigestItem::Consensus(FORK_POWER_DIGEST, power.encode()));
		digest.push(DigestItem::Consensus(
			PARENT_VOTING_KEY_DIGEST,
			ParentVotingKeyDigest { parent_voting_key: voting_key }.encode(),
		));
		let block_digest = BlockSealDigest::Compute { nonce: U256::zero() };
		(digest, block_digest.to_digest())
	}

	fn new_importer() -> (ArgonBlockImport<Block, MemChain, MemChain, H256>, MemChain) {
		let genesis = Header::new(0, H256::zero(), H256::zero(), H256::zero(), Digest::default());
		let client = MemChain::new(genesis.clone());
		let db_arc = std::sync::Arc::new(client.clone());
		let importer = ArgonBlockImport::<Block, _, _, _> {
			inner: client.clone(),
			client: db_arc.clone(),
			aux_client: ArgonAux::new(db_arc.clone()),
			import_lock: Default::default(),
			_phantom: PhantomData,
		};
		(importer, client)
	}

	fn create_params(
		block_number: BlockNumber,
		parent_hash: H256,
		compute_difficulty: ComputeDifficulty,
		voting_key: Option<VotingKey>,
		origin: BlockOrigin,
		state_action: StateAction<Block>,
		author: Option<AccountId>,
	) -> BlockImportParams<Block> {
		let author = author.unwrap_or_else(|| AccountId::from([0u8; 32]));
		let (header, post_digest) =
			create_header(block_number, parent_hash, compute_difficulty, voting_key, author);
		let mut params = BlockImportParams::new(origin, header.clone());
		params.state_action = state_action;
		params.post_digests.push(post_digest);
		params.post_hash = Some(header.hash());
		params
	}

	#[tokio::test]
	async fn gap_header_not_best() {
		let (importer, client) = new_importer();
		let parent = client.info().best_hash;
		let params = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::NetworkInitialSync,
			StateAction::Skip,
			None,
		);

		let res = importer.import_block(params).await.unwrap();
		assert!(matches!(res, ImportResult::Imported(_)));
		assert_eq!(client.info().best_number, 0u32);
	}

	#[tokio::test]
	async fn higher_fork_power_sets_best() {
		let (importer, client) = new_importer();
		let parent = client.info().best_hash;

		// weaker block (power 1)
		let p1 = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::NetworkInitialSync,
			StateAction::Execute,
			None,
		);

		let _ = importer.import_block(p1).await.unwrap();

		// stronger block (power 2)
		let p2 = create_params(
			1,
			parent,
			2,
			None,
			BlockOrigin::NetworkInitialSync,
			StateAction::Execute,
			None,
		);
		let h2 = p2.header.hash();
		let _ = importer.import_block(p2).await.unwrap();

		assert_eq!(client.info().best_hash, h2);
	}

	#[tokio::test]
	async fn header_plus_state_can_be_best() {
		let (importer, _client) = new_importer();
		let parent = importer.client.info().best_hash;
		let params = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::NetworkInitialSync,
			StateAction::ApplyChanges(StorageChanges::Import(ImportedState {
				block: H256::zero(),
				state: KeyValueStates(Vec::new()),
			})),
			None,
		);

		let res = importer.import_block(params).await.unwrap();
		// We just care that full import ran; NoopImport returns Imported(...)
		assert!(matches!(res, ImportResult::Imported(_)));
	}

	#[tokio::test]
	async fn state_ugprade_test() {
		let (importer, client) = new_importer();
		let parent = importer.client.info().best_hash;
		// header-only import
		let gap = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::NetworkInitialSync,
			StateAction::Skip,
			None,
		);
		let hash = gap.header.hash();
		importer.import_block(gap).await.unwrap();
		assert!(!has_state(&client, hash));

		// now with state
		let state = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::NetworkInitialSync,
			StateAction::ApplyChanges(StorageChanges::Import(ImportedState {
				block: H256::zero(),
				state: KeyValueStates(Vec::new()),
			})),
			None,
		);
		importer.import_block(state).await.unwrap();
		assert!(has_state(&client, hash));
	}

	#[tokio::test]
	async fn finalized_upgrade_reimports() {
		let (importer, _client) = new_importer();
		let parent = importer.client.info().best_hash;
		let params1 = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::NetworkBroadcast,
			StateAction::Skip,
			None,
		);
		let _ = importer.import_block(params1).await.unwrap();

		// second import – now marked finalized
		let mut params2 = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::NetworkBroadcast,
			StateAction::Skip,
			None,
		);
		params2.finalized = true;

		let res2 = importer.import_block(params2).await.unwrap();
		assert!(matches!(res2, ImportResult::Imported(_)));
	}

	#[tokio::test]
	async fn duplicate_header_short_circuits() {
		let (importer, _client) = new_importer();
		let parent = importer.client.info().best_hash;
		let params = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::NetworkBroadcast,
			StateAction::Skip,
			None,
		);

		// first import
		let _ = importer.import_block(params).await.unwrap();

		// build identical params again (BlockImportParams isn't Clone)
		let params2 = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::NetworkBroadcast,
			StateAction::Skip,
			None,
		);

		let res2 = importer.import_block(params2).await.unwrap();
		assert!(matches!(res2, ImportResult::AlreadyInChain));
	}

	#[tokio::test]
	async fn tie_loser_test() {
		let (importer, client) = new_importer();
		let parent = client.info().best_hash;

		// loser (hash2 > hash1)
		let loser = create_params(1, parent, 1, None, BlockOrigin::Own, StateAction::Execute, None);
		assert_ok!(importer.import_block(loser).await); // Imported

		// winner (smaller hash)
		let winner = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::Own,
			StateAction::Execute,
			Some(AccountId::from([2u8; 32])),
		);
		let h_win = winner.header.hash();
		assert_ok!(importer.import_block(winner).await); // Imported(best)

		assert_eq!(client.info().best_hash, h_win);

		// replay loser
		let loser2 = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::Own,
			StateAction::Skip,
			Some(AccountId::from([0u8; 32])),
		);
		let res = importer.import_block(loser2).await.unwrap();
		assert!(matches!(res, ImportResult::AlreadyInChain));
		assert_eq!(client.info().best_hash, h_win);
	}

	#[tokio::test]
	async fn duplicate_vote_block_same_tick_fails() {
		let (importer, _client) = new_importer();
		let parent = importer.client.info().best_hash;

		let author = AccountId::from([9u8; 32]);
		let vote_key = H256::random();

		// First vote → ok
		let p1 = create_params(
			1,
			parent,
			0,
			Some(vote_key),
			BlockOrigin::NetworkBroadcast,
			StateAction::Execute,
			Some(author.clone()),
		);
		let p1_hash = p1.post_hash();
		assert!(matches!(importer.import_block(p1).await.unwrap(), ImportResult::Imported(_)));

		// Second vote by same author + same voting_key at same tick ⇒ Err
		let mut p2 = create_params(
			1,
			parent,
			0,
			Some(vote_key),
			BlockOrigin::NetworkBroadcast,
			StateAction::Execute,
			Some(author),
		);
		p2.header.extrinsics_root = H256::random();
		p2.header.state_root = H256::random();
		p2.post_hash = Some(p2.header.hash()); // refresh the cached value
		let p2_hash = p2.post_hash();
		let err = importer.import_block(p2).await;

		assert_ne!(p1_hash, p2_hash, "post hashes should differ");

		assert!(err.is_err(), "duplicate vote block should fail");
	}

	#[tokio::test]
	async fn duplicate_compute_loser_same_power_fails() {
		let (importer, client) = new_importer();
		let parent = client.info().best_hash;

		// Winner first (smaller hash) so later blocks at same power are losers
		let winner = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::Own,
			StateAction::Execute,
			Some(AccountId::from([1u8; 32])),
		);
		let h_winner = winner.post_hash();
		importer.import_block(winner).await.unwrap();

		// First loser from author X
		let author = AccountId::from([2u8; 32]);
		let mut loser1 = create_params(
			1,
			parent,
			1,
			None,
			BlockOrigin::Own,
			StateAction::Execute,
			Some(author.clone()),
		);
		loser1.header.extrinsics_root = H256::random();
		loser1.header.state_root = H256::random();
		loser1.post_hash = Some(loser1.header.hash()); // refresh the cached
		assert_ne!(loser1.post_hash(), h_winner, "loser1 should differ from winner");
		importer.import_block(loser1).await.unwrap(); // ok

		// Second loser, same author, same fork-power, same tick ⇒ Err
		let loser2 =
			create_params(1, parent, 1, None, BlockOrigin::Own, StateAction::Execute, Some(author));
		let err = importer.import_block(loser2).await;
		assert!(err.is_err(), "duplicate compute loser should fail");
	}

	#[tokio::test]
	async fn reorg_to_lower_power_then_recover() {
		let (importer, client) = new_importer();
		let genesis = client.info().best_hash;

		// Node-2 fork: height 1..100, power 200.
		let mut parent = genesis;
		let mut hash50 = H256::zero();
		for n in 1..=100 {
			let p =
				create_params(n, parent, 200 + n as u128, None, BlockOrigin::Own, Execute, None);
			if n == 50 {
				hash50 = p.header.hash();
			}
			parent = p.header.hash();
			importer.import_block(p).await.unwrap();
		}
		assert_eq!(client.info().best_number, 100);

		// Archive node finalises *lower-power* fork at height 51, power 151.
		let p = create_params(51, hash50, 151, None, BlockOrigin::Own, Execute, None);
		let back_best = p.header.hash();
		assert_ok!(importer.import_block(p).await);
		*client.best.lock().unwrap() = (51, back_best);

		// Fresh block from node-2 with power 152 must become best.
		let p =
			create_params(52, client.info().best_hash, 152, None, BlockOrigin::Own, Execute, None);
		let hash130 = p.header.hash();
		importer.import_block(p).await.unwrap();

		assert_eq!(client.info().best_hash, hash130);
	}
}
