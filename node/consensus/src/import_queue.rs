use crate::{aux_client::ArgonAux, compute_worker::BlockComputeNonce, error::Error, NotaryClient};
use argon_bitcoin_utxo_tracker::{get_bitcoin_inherent, UtxoTracker};
use argon_primitives::{
	digests::ArgonDigests,
	fork_power::ForkPower,
	inherents::{BitcoinInherentDataProvider, BlockSealInherentDataProvider},
	Balance, BitcoinApis, BlockCreatorApis, BlockSealApis, BlockSealAuthorityId, BlockSealDigest,
	NotaryApis, NotebookApis, TickApis,
};
use argon_runtime::{NotaryRecordT, NotebookVerifyError};
use codec::Codec;
use sc_client_api::{self, backend::AuxStore, BlockBackend};
use sc_consensus::{
	BasicQueue, BlockCheckParams, BlockImport, BlockImportParams, BoxJustificationImport,
	ForkChoiceStrategy, ImportResult, StateAction, StorageChanges, Verifier as VerifierT,
};
use sc_telemetry::TelemetryHandle;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::{BlockStatus, HeaderBackend};
use sp_consensus::{BlockOrigin, Error as ConsensusError};
use sp_inherents::InherentDataProvider;
use sp_runtime::{
	traits::{Block as BlockT, Header as HeaderT, NumberFor},
	Justification,
};
use std::{marker::PhantomData, sync::Arc};
use tracing::warn;

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
	C: HeaderBackend<B> + AuxStore + Send + Sync + 'static,
	AC: Codec + Send + Sync + 'static,
{
	type Error = ConsensusError;

	async fn check_block(&self, block: BlockCheckParams<B>) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block.clone()).await.map_err(Into::into)
	}

	async fn import_block(
		&self,
		mut block: BlockImportParams<B>,
	) -> Result<ImportResult, Self::Error> {
		let hash = block.post_hash();
		let number = *block.header.number();

		let parent = *block.header.parent_hash();

		let info = self.client.info();
		let is_block_gap = info.block_gap.is_some_and(|(s, e)| s <= number && number <= e);
		// NOTE: do not access runtime here without knowing for CERTAIN state is successfully
		// imported. Various sync strategies will access this path without state set yet
		tracing::trace!(
			number=?number,
			hash=?hash,
			parent=?parent,
			is_block_gap,
			action=match block.state_action {
				StateAction::ApplyChanges(StorageChanges::Changes(_)) => "state_apply",
				StateAction::ApplyChanges(StorageChanges::Import(_)) => "state_import",
				StateAction::Execute => "execute",
				StateAction::ExecuteIfPossible => "execute_if_possible",
				StateAction::Skip => "skip",
			},
			is_ours=block.origin == BlockOrigin::Own,
			"Begin import."
		);
		let digest = block.header.digest();
		let block_author: AC = digest
			.convert_first(|a| a.as_author())
			.ok_or(Error::UnableToDecodeDigest("author".to_string()))?;

		let tick = digest
			.convert_first(|a| a.as_tick())
			.ok_or(Error::UnableToDecodeDigest("tick".to_string()))?
			.0;

		let voting_key = digest
			.convert_first(|a| a.as_parent_voting_key())
			.ok_or(Error::UnableToDecodeDigest("voting key".to_string()))?
			.parent_voting_key;

		let fork_power = ForkPower::try_from(digest)
			.map_err(|e| Error::MissingRuntimeData(format!("Failed to get fork power: {:?}", e)))?;

		// hold for rest of block import
		let _lock = self.import_lock.lock().await;

		block.fork_choice = Some(ForkChoiceStrategy::Custom(false));
		// We only want to set a best block if the state is imported. When
		// syncing, we will sometimes import state, but be grabbing a gap block, in which case we
		// don't want to set interim blocks as best block
		let has_state = match block.state_action {
			StateAction::ApplyChanges(_) | StateAction::Execute => true,
			StateAction::ExecuteIfPossible =>
				self.client.status(parent).unwrap_or(BlockStatus::Unknown) == BlockStatus::InChain,
			StateAction::Skip => false,
		};

		if has_state && !is_block_gap {
			let max_fork_power = self.aux_client.record_block(
				&mut block.auxiliary,
				block_author,
				voting_key,
				tick,
				fork_power.is_latest_vote,
			)?;
			// NOTE: only import as best block if it beats the best stored block. There are cases
			// where importing a tie will yield too many blocks at a height and break substrate
			if fork_power > max_fork_power {
				tracing::info!(
					block_hash = ?hash,
					?fork_power,
					"New best fork imported"
				);
				block.fork_choice = Some(ForkChoiceStrategy::Custom(true));
			}
		}
		match self.inner.import_block(block).await {
			Ok(result) => {
				self.aux_client.block_accepted(fork_power).inspect_err(|e| {
					log::error!("Failed to record block accepted for {:?}: {:?}", hash, e)
				})?;
				Ok(result)
			},
			Err(e) => Err(e.into()),
		}
	}
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
		let number = *block_params.header.number();
		let is_gap_sync =
			self.client.info().block_gap.is_some_and(|(s, e)| s <= number && number <= e);
		// Skip checks that include execution, if being told so, or when importing only state.
		//
		// This is done for example when gap syncing and it is expected that the block after the gap
		// was checked/chosen properly, e.g. by warp syncing to this block using a finality proof.
		if is_gap_sync ||
			block_params.state_action.skip_execution_checks() ||
			block_params.with_state()
		{
			return Ok(block_params)
		}

		let post_hash = block_params.header.hash();
		if matches!(block_params.state_action, StateAction::ExecuteIfPossible) {
			// ensure we have parent state
			if self
				.client
				.block_status(*block_params.header.parent_hash())
				.unwrap_or(sp_consensus::BlockStatus::Unknown) !=
				sp_consensus::BlockStatus::InChainWithState
			{
				warn!(
					block_hash = ?post_hash,
					origin = ?block_params.origin,
					fork_choice = ?block_params.fork_choice,
					import_existing = ?block_params.import_existing,
					finalized = ?block_params.finalized,
					"Unable to verify block due with ExecuteIfPossible to missing state for parent block."
				);
				if block_params.origin == BlockOrigin::NetworkInitialSync {
					return Ok(block_params)
				}

				return Err("Parent state not available".into())
			}
		}

		let parent_hash = *block_params.header.parent_hash();

		let mut header = block_params.header;
		let raw_seal_digest = header.digest_mut().pop().ok_or(Error::MissingBlockSealDigest)?;
		let seal_digest =
			BlockSealDigest::try_from(&raw_seal_digest).ok_or(Error::MissingBlockSealDigest)?;

		block_params.header = header;
		block_params.post_digests.push(raw_seal_digest);
		block_params.post_hash = Some(post_hash);

		let digest = block_params.header.digest();
		let pre_hash = block_params.header.hash();

		if seal_digest.is_vote() {
			let is_valid = self
				.client
				.runtime_api()
				.is_valid_signature(parent_hash, pre_hash, &seal_digest, digest)
				.map_err(|e| format!("Error verifying miner signature {:?}", e))?;
			if !is_valid {
				return Err(Error::InvalidVoteSealSignature.into());
			}
		}

		// if we're importing a non-finalized block from someone else, verify the notebook
		// audits
		let latest_verified_finalized = self.client.info().finalized_number;
		if block_params.origin != BlockOrigin::Own &&
			block_params.header.number() > &latest_verified_finalized &&
			!block_params.finalized
		{
			let digest_notebooks = self
				.client
				.runtime_api()
				.digest_notebooks(parent_hash, digest)
				.map_err(|e| format!("Error calling digest notebooks api {e:?}"))?
				.map_err(|e| format!("Failed to get digest notebooks: {:?}", e))?;
			self.notary_client
				.verify_notebook_audits(&parent_hash, digest_notebooks)
				.await?;
		}

		// NOTE: we verify compute nonce in import queue because we use the pre-hash, which
		// we'd have to inject into the runtime
		if let BlockSealDigest::Compute { nonce } = &seal_digest {
			let compute_puzzle = self
				.client
				.runtime_api()
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
				return Err(Error::InvalidComputeNonce.into());
			}
		}

		if let Some(inner_body) = block_params.body.clone() {
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
			let inherent_res = self
				.client
				.runtime_api()
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
	registry: Option<&prometheus_endpoint::Registry>,
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
