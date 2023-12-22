use codec::Codec;
use std::{marker::PhantomData, sync::Arc};

use log::info;
use sc_client_api::{self, backend::AuxStore, BlockOf, BlockchainEvents};
use sc_consensus::{
	BlockCheckParams, BlockImport, BlockImportParams, ForkChoiceStrategy, ImportResult, Verifier,
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

use ulx_primitives::{
	inherents::BlockSealInherentDataProvider, BlockSealApis, BlockSealAuthorityId, BlockSealDigest,
	BLOCK_SEAL_DIGEST_ID,
};

use crate::{
	aux::UlxAux,
	basic_queue::BasicQueue,
	compute_solver::BlockComputeNonce,
	digests::{load_digests, read_seal_digest},
	error::Error,
	notary_client::verify_notebook_audits,
};

/// A block importer for Ulx.
pub struct UlxBlockImport<B: BlockT, I, C: AuxStore, S, AC> {
	inner: I,
	select_chain: S,
	client: Arc<C>,
	aux_client: UlxAux<B, C>,
	_block: PhantomData<AC>,
}

impl<B: BlockT, I: Clone, C: AuxStore, S: Clone, AC: Codec> Clone
	for UlxBlockImport<B, I, C, S, AC>
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			select_chain: self.select_chain.clone(),
			client: self.client.clone(),
			aux_client: self.aux_client.clone(),
			_block: PhantomData,
		}
	}
}

impl<B, I, C, S, AC> UlxBlockImport<B, I, C, S, AC>
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
	/// Create a new block import suitable to be used in Ulx
	pub fn new(inner: I, client: Arc<C>, aux_client: UlxAux<B, C>, select_chain: S) -> Self {
		Self { inner, client, select_chain, aux_client, _block: PhantomData }
	}
}

#[async_trait::async_trait]
impl<B, I, C, S, AC> BlockImport<B> for UlxBlockImport<B, I, C, S, AC>
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
		+ BlockOf,
	C::Api: BlockBuilderApi<B> + BlockSealApis<B, AC, BlockSealAuthorityId>,
	AC: Codec + Send + Sync,
{
	type Error = ConsensusError;

	async fn check_block(
		&mut self,
		block: BlockCheckParams<B>,
	) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block).await.map_err(Into::into)
	}

	async fn import_block(
		&mut self,
		mut block: BlockImportParams<B>,
	) -> Result<ImportResult, Self::Error> {
		info!("Importing block with hash {:?} ({})", block.post_hash(), block.header.number());
		let digests = load_digests::<B>(&block.header)?;

		if &digests.finalized_block.number > block.header.number() {
			return Err(Error::<B>::InvalidFinalizedBlockDigest.into())
		}

		let latest_verified_finalized = self.client.info().finalized_number;
		if digests.finalized_block.number > latest_verified_finalized {
			return Err(Error::<B>::PendingFinalizedBlockDigest(
				digests.finalized_block.hash,
				digests.finalized_block.number,
			)
			.into())
		}

		// if we're importing a non-finalized block from someone else, verify the notebook audits
		if block.origin != BlockOrigin::Own && block.header.number() > &latest_verified_finalized {
			verify_notebook_audits(&self.aux_client, &digests.notebooks).await?;
		}

		let Some(Some(seal_digest)) = block.post_digests.last().map(read_seal_digest) else {
			return Err(Error::<B>::MissingBlockSealDigest.into())
		};
		let parent_hash = *block.header.parent_hash();

		if let Some(inner_body) = block.body.take() {
			let check_block = B::new(block.header.clone(), inner_body);

			if !block.state_action.skip_execution_checks() {
				let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
				let seal =
					BlockSealInherentDataProvider { seal: None, digest: Some(seal_digest.clone()) };
				let inherent_data_providers = (timestamp, seal);

				let inherent_data = inherent_data_providers
					.create_inherent_data()
					.await
					.map_err(|e| Error::<B>::CreateInherents(e))?;

				// inherent data passed in is what we would have generated...
				let inherent_res = self
					.client
					.runtime_api()
					.check_inherents(parent_hash, check_block.clone(), inherent_data)
					.map_err(|e| Error::<B>::Client(e.into()))?;

				if !inherent_res.ok() {
					for (identifier, error) in inherent_res.into_errors() {
						match inherent_data_providers.try_handle_error(&identifier, &error).await {
							Some(res) => res.map_err(Error::<B>::CheckInherents)?,
							None =>
								return Err(Error::<B>::CheckInherentsUnknownError(identifier).into()),
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
		match &seal_digest {
			BlockSealDigest::Compute { nonce } => {
				// verify compute effort
				let difficulty =
					self.client.runtime_api().compute_difficulty(parent_hash).map_err(|e| {
						Error::<B>::MissingRuntimeData(
							format!("Failed to get difficulty from runtime: {}", e).to_string(),
						)
					})?;
				if !BlockComputeNonce::is_valid(nonce, pre_hash.as_ref().to_vec(), difficulty) {
					return Err(Error::<B>::InvalidComputeNonce.into())
				}
				compute_difficulty = Some(difficulty);
			},
			_ => {},
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

pub struct UlxVerifier<B: BlockT> {
	_marker: PhantomData<B>,
}

impl<B: BlockT> UlxVerifier<B> {
	pub fn new() -> Self {
		Self { _marker: PhantomData }
	}
}

#[async_trait::async_trait]
impl<B: BlockT> Verifier<B> for UlxVerifier<B> {
	async fn verify(
		&mut self,
		mut block: BlockImportParams<B>,
	) -> Result<BlockImportParams<B>, String> {
		let mut header = block.header;
		let hash = header.hash();

		let seal_digest = match header.digest_mut().pop() {
			Some(DigestItem::Seal(id, signature_digest)) =>
				if id == BLOCK_SEAL_DIGEST_ID {
					Ok(DigestItem::Seal(id, signature_digest.clone()))
				} else {
					Err(Error::<B>::WrongEngine(id))
				},
			_ => Err(Error::<B>::MissingBlockSealDigest),
		}?;

		block.header = header;
		block.post_digests.push(seal_digest);
		block.post_hash = Some(hash);

		Ok(block)
	}
}

/// The Ulx import queue type.
pub type UlxImportQueue<B> = BasicQueue<B>;
