use crate::{
	aux_client::ArgonAux, compute_worker::BlockComputeNonce, error::Error,
	notary_client::verify_notebook_audits,
};
use argon_bitcoin_utxo_tracker::{get_bitcoin_inherent, UtxoTracker};
use argon_node_runtime::NotebookVerifyError;
use argon_primitives::{
	inherents::{BitcoinInherentDataProvider, BlockSealInherentDataProvider},
	Balance, BitcoinApis, BlockCreatorApis, BlockSealApis, BlockSealAuthorityId, BlockSealDigest,
	NotebookApis,
};
use codec::Codec;
use sc_client_api::{self, backend::AuxStore};
use sc_consensus::{
	BasicQueue, BlockCheckParams, BlockImport, BlockImportParams, ForkChoiceStrategy, ImportResult,
	Verifier as VerifierT,
};
use sc_telemetry::TelemetryHandle;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Error as ConsensusError};
use sp_inherents::InherentDataProvider;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use std::{marker::PhantomData, sync::Arc};

/// A block importer for argon.

pub struct ArgonBlockImport<B: BlockT, I, C: AuxStore, AC> {
	inner: I,
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	_phantom: PhantomData<AC>,
}

impl<B: BlockT, I: Clone, C: AuxStore, AC: Codec> Clone for ArgonBlockImport<B, I, C, AC> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			client: self.client.clone(),
			aux_client: self.aux_client.clone(),
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
	C: ProvideRuntimeApi<B> + Send + Sync + AuxStore + 'static,
	C::Api: BlockCreatorApis<B, AC, NotebookVerifyError>,
	AC: Codec + Send + Sync + 'static,
{
	type Error = ConsensusError;

	async fn check_block(&self, block: BlockCheckParams<B>) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block).await.map_err(Into::into)
	}

	async fn import_block(
		&self,
		mut block: BlockImportParams<B>,
	) -> Result<ImportResult, Self::Error> {
		let parent_hash = *block.header.parent_hash();

		let seal_digest = block
			.post_digests
			.iter()
			.find_map(BlockSealDigest::try_from)
			.ok_or(Error::MissingBlockSealDigest)?;

		let fork_power = self
			.client
			.runtime_api()
			.calculate_fork_power(parent_hash, seal_digest, &block.header)
			.map_err(Error::Api)?
			.map_err(|e| Error::MissingRuntimeData(format!("Failed to get fork power: {:?}", e)))?;

		let (block_author, tick, voting_key) = self
			.client
			.runtime_api()
			.decode_voting_author(parent_hash, block.header.digest())
			.map_err(Error::Api)?
			.map_err(|e| {
				Error::MissingRuntimeData(format!("Failed to get block_author power: {:?}", e))
			})?;
		let max_fork_power =
			self.aux_client.record_block(&mut block, block_author, voting_key, tick)?;
		let block_hash = block.post_hash();
		block.fork_choice = Some(ForkChoiceStrategy::Custom(fork_power > max_fork_power));

		// TODO: do we need to hold a lock here?
		match self.inner.import_block(block).await {
			Ok(result) => {
				self.aux_client.block_accepted(max_fork_power).inspect_err(|e| {
					log::error!("Failed to record block accepted for {:?}: {:?}", block_hash, e)
				})?;
				Ok(result)
			},
			Err(e) => Err(e.into()),
		}
	}
}

#[allow(dead_code)]
struct Verifier<B: BlockT, C: AuxStore, AC> {
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	utxo_tracker: Arc<UtxoTracker>,
	telemetry: Option<TelemetryHandle>,
	_phantom: PhantomData<AC>,
}

#[async_trait::async_trait]
impl<B: BlockT, C, AC> VerifierT<B> for Verifier<B, C, AC>
where
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + Send + Sync + AuxStore + 'static,
	C::Api: BlockBuilderApi<B>
		+ BitcoinApis<B, Balance>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ BlockCreatorApis<B, AC, NotebookVerifyError>
		+ NotebookApis<B, NotebookVerifyError>,
	AC: Codec + Clone + Send + Sync + 'static,
{
	async fn verify(
		&self,
		mut block_params: BlockImportParams<B>,
	) -> Result<BlockImportParams<B>, String> {
		// Skip checks that include execution, if being told so, or when importing only state.
		//
		// This is done for example when gap syncing and it is expected that the block after the gap
		// was checked/chosen properly, e.g. by warp syncing to this block using a finality proof.
		if block_params.state_action.skip_execution_checks() || block_params.with_state() {
			return Ok(block_params)
		}

		let post_hash = block_params.header.hash();
		let parent_hash = *block_params.header.parent_hash();

		let mut header = block_params.header;
		let raw_seal_digest = header.digest_mut().pop().ok_or(Error::MissingBlockSealDigest)?;
		let seal_digest =
			BlockSealDigest::try_from(&raw_seal_digest).ok_or(Error::MissingBlockSealDigest)?;

		block_params.header = header;
		block_params.post_digests.push(raw_seal_digest);
		block_params.post_hash = Some(post_hash);

		let digest = block_params.header.digest();

		if seal_digest.is_vote() {
			let pre_hash = block_params.header.hash();

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
			block_params.header.number() > &latest_verified_finalized
		{
			let digest_notebooks = self
				.client
				.runtime_api()
				.digest_notebooks(parent_hash, digest)
				.map_err(|e| format!("Error calling digest notebooks api {e:?}"))?
				.map_err(|e| format!("Failed to get digest notebooks: {:?}", e))?;
			verify_notebook_audits(&self.aux_client, digest_notebooks).await?;
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
			if !BlockComputeNonce::is_valid(
				nonce,
				post_hash.as_ref().to_vec(),
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

/// Start an import queue for a Cumulus node which checks blocks' seals and inherent data.
pub fn create_import_queue<C, B, I, AC>(
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
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
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + Send + Sync + AuxStore + 'static,
	C::Api: BlockBuilderApi<B>
		+ BlockCreatorApis<B, AC, NotebookVerifyError>
		+ BitcoinApis<B, Balance>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ NotebookApis<B, NotebookVerifyError>,
	AC: Codec + Clone + Send + Sync + 'static,
	I: BlockImport<B, Error = ConsensusError> + Send + Sync + 'static,
{
	let importer = ArgonBlockImport {
		inner: block_import,
		client: client.clone(),
		aux_client: ArgonAux::new(client.clone()),
		_phantom: PhantomData,
	};
	let verifier = Verifier::<B, C, AC> {
		client: client.clone(),
		utxo_tracker,
		aux_client,
		telemetry,
		_phantom: PhantomData,
	};

	(BasicQueue::new(verifier, Box::new(importer.clone()), None, spawner, registry), importer)
}
