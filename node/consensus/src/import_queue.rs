use codec::Codec;
use std::{marker::PhantomData, sync::Arc};

use argon_bitcoin_utxo_tracker::{get_bitcoin_inherent, UtxoTracker};
use log::info;
use sc_client_api::{self, backend::AuxStore, BlockOf, BlockchainEvents};
use sc_consensus::{
	BasicQueue, BlockCheckParams, BlockImport, BlockImportParams, ForkChoiceStrategy, ImportResult,
	Verifier,
};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Error as ConsensusError, SelectChain};
use sp_inherents::InherentDataProvider;
use sp_runtime::{
	generic::DigestItem,
	traits::{Block as BlockT, Header as HeaderT},
};

use argon_primitives::{
	inherents::{BitcoinInherentDataProvider, BlockSealInherentDataProvider},
	Balance, BitcoinApis, BlockSealApis, BlockSealAuthorityId, BlockSealDigest,
	BLOCK_SEAL_DIGEST_ID,
};

use crate::{
	aux_client::ArgonAux,
	compute_solver::BlockComputeNonce,
	compute_worker::randomx_key_block,
	digests::{load_digests, read_seal_digest},
	error::Error,
	notary_client::verify_notebook_audits,
};

/// A block importer for argon.
pub struct ArgonBlockImport<B: BlockT, I, C: AuxStore, S, AC> {
	inner: I,
	select_chain: S,
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	utxo_tracker: Arc<UtxoTracker>,
	_block: PhantomData<AC>,
}

impl<B: BlockT, I: Clone, C: AuxStore, S: Clone, AC: Codec> Clone
	for ArgonBlockImport<B, I, C, S, AC>
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			select_chain: self.select_chain.clone(),
			client: self.client.clone(),
			aux_client: self.aux_client.clone(),
			utxo_tracker: self.utxo_tracker.clone(),
			_block: PhantomData,
		}
	}
}

impl<B, I, C, S, AC> ArgonBlockImport<B, I, C, S, AC>
where
	B: BlockT,
	I: BlockImport<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B>
		+ Send
		+ Sync
		+ HeaderBackend<B>
		+ BlockchainEvents<B>
		+ AuxStore
		+ BlockOf,
	C::Api: BlockBuilderApi<B>,
	AC: Codec,
{
	/// Create a new block import suitable to be used in argon
	pub fn new(
		inner: I,
		client: Arc<C>,
		aux_client: ArgonAux<B, C>,
		select_chain: S,
		utxo_tracker: Arc<UtxoTracker>,
	) -> Self {
		Self { inner, client, select_chain, aux_client, utxo_tracker, _block: PhantomData }
	}
}

#[async_trait::async_trait]
impl<B, I, C, S, AC> BlockImport<B> for ArgonBlockImport<B, I, C, S, AC>
where
	B: BlockT,
	I: BlockImport<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	S: SelectChain<B>,
	C: ProvideRuntimeApi<B>
		+ Send
		+ Sync
		+ HeaderBackend<B>
		+ BlockchainEvents<B>
		+ AuxStore
		+ BlockOf
		+ 'static,
	C::Api:
		BlockBuilderApi<B> + BlockSealApis<B, AC, BlockSealAuthorityId> + BitcoinApis<B, Balance>,
	AC: Codec + Clone + Send + Sync,
{
	type Error = ConsensusError;

	async fn check_block(&self, block: BlockCheckParams<B>) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block).await.map_err(Into::into)
	}

	async fn import_block(
		&mut self,
		mut block: BlockImportParams<B>,
	) -> Result<ImportResult, Self::Error> {
		info!("Importing block with hash {:?} ({})", block.post_hash(), block.header.number());
		let digests = load_digests::<B>(&block.header)?;

		// if we're importing a non-finalized block from someone else, verify the notebook audits
		let latest_verified_finalized = self.client.info().finalized_number;
		if block.origin != BlockOrigin::Own && block.header.number() > &latest_verified_finalized {
			verify_notebook_audits(&self.aux_client, &digests.notebooks).await?;
		}

		let Some(Some(seal_digest)) = block.post_digests.last().map(read_seal_digest) else {
			return Err(Error::MissingBlockSealDigest.into());
		};
		let parent_hash = *block.header.parent_hash();

		if let Some(inner_body) = block.body.take() {
			let check_block = B::new(block.header.clone(), inner_body);

			if !block.state_action.skip_execution_checks() {
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
							None =>
								return Err(Error::CheckInherentsUnknownError(identifier).into()),
						}
					}
				}
			}

			block.body = Some(check_block.deconstruct().1);
		}

		let pre_hash = block.header.hash();
		let mut compute_difficulty = None;

		// NOTE: we verify compute nonce in import queue because we use the pre-hash, which we'd
		// have to inject into the runtime
		if let BlockSealDigest::Compute { nonce } = &seal_digest {
			// verify compute effort
			let difficulty =
				self.client.runtime_api().compute_difficulty(parent_hash).map_err(|e| {
					Error::MissingRuntimeData(
						format!("Failed to get difficulty from runtime: {}", e).to_string(),
					)
				})?;
			let key_block_hash = randomx_key_block(&self.client, &parent_hash)?;
			if !BlockComputeNonce::is_valid(
				nonce,
				pre_hash.as_ref().to_vec(),
				&key_block_hash,
				difficulty,
			) {
				return Err(Error::InvalidComputeNonce.into());
			}
			compute_difficulty = Some(difficulty);
		}

		let best_header = self
			.select_chain
			.best_chain()
			.await
			.map_err(|e| format!("Fetch best chain failed via select chain: {}", e))
			.map_err(ConsensusError::ChainLookup)?;

		let tick = digests.tick.tick;
		let (fork_power, best_fork_power) = self.aux_client.record_block(
			best_header,
			&mut block,
			digests.author,
			digests.voting_key.parent_voting_key,
			digests.notebooks.notebooks.iter().fold(0u32, |acc, x| {
				if tick == x.tick {
					acc + 1
				} else {
					acc
				}
			}),
			tick,
			digests.block_vote.voting_power,
			seal_digest,
			compute_difficulty,
		)?;

		if block.fork_choice.is_none() {
			block.fork_choice = Some(ForkChoiceStrategy::Custom(fork_power > best_fork_power));
		}

		self.inner.import_block(block).await.map_err(Into::into)
	}
}

pub struct ArgonVerifier<B: BlockT> {
	_marker: PhantomData<B>,
}

impl<B: BlockT> Default for ArgonVerifier<B> {
	fn default() -> Self {
		Self::new()
	}
}

impl<B: BlockT> ArgonVerifier<B> {
	pub fn new() -> Self {
		Self { _marker: PhantomData }
	}
}

#[async_trait::async_trait]
impl<B: BlockT> Verifier<B> for ArgonVerifier<B> {
	async fn verify(
		&self,
		mut block: BlockImportParams<B>,
	) -> Result<BlockImportParams<B>, String> {
		let mut header = block.header;
		let hash = header.hash();

		let seal_digest = match header.digest_mut().pop() {
			Some(DigestItem::Seal(id, signature_digest)) =>
				if id == BLOCK_SEAL_DIGEST_ID {
					Ok(DigestItem::Seal(id, signature_digest.clone()))
				} else {
					Err(Error::WrongEngine(id))
				},
			_ => Err(Error::MissingBlockSealDigest),
		}?;

		block.header = header;
		block.post_digests.push(seal_digest);
		block.post_hash = Some(hash);

		Ok(block)
	}
}

/// The argon import queue type.
pub type ArgonImportQueue<B> = BasicQueue<B>;
