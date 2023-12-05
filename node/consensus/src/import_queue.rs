use std::{marker::PhantomData, sync::Arc};

use log::info;
use sc_client_api::{self, backend::AuxStore, BlockOf, BlockchainEvents};
use sc_consensus::{
	BlockCheckParams, BlockImport, BlockImportParams, ForkChoiceStrategy, ImportResult, Verifier,
};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{Error as ConsensusError, SelectChain};
use sp_inherents::InherentDataProvider;
use sp_runtime::{
	generic::DigestItem,
	traits::{Block as BlockT, Header as HeaderT},
};

use ulx_notary::ensure;
use ulx_primitives::{
	digests::SealSource, BlockSealMinimumApis, MiningAuthorityApis, BLOCK_SEAL_DIGEST_ID,
};

use crate::{
	authority::verify_seal_signature,
	aux::UlxAux,
	basic_queue::BasicQueue,
	compute_worker::BlockComputeNonce,
	digests::{load_digests, read_seal_digest},
	error::Error,
};

/// A block importer for Ulx.
pub struct UlxBlockImport<B: BlockT, I, C, S> {
	inner: I,
	select_chain: S,
	client: Arc<C>,
	_block: PhantomData<B>,
}

impl<B: BlockT, I: Clone, C, S: Clone> Clone for UlxBlockImport<B, I, C, S> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			select_chain: self.select_chain.clone(),
			client: self.client.clone(),
			_block: PhantomData,
		}
	}
}

impl<B, I, C, S> UlxBlockImport<B, I, C, S>
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
	C::Api: MiningAuthorityApis<B>,
{
	/// Create a new block import suitable to be used in Ulx
	pub fn new(inner: I, client: Arc<C>, select_chain: S) -> Self {
		Self { inner, client, select_chain, _block: PhantomData }
	}
}

#[async_trait::async_trait]
impl<B, I, C, S> BlockImport<B> for UlxBlockImport<B, I, C, S>
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
	C::Api: BlockBuilderApi<B> + MiningAuthorityApis<B> + BlockSealMinimumApis<B>,
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
		let Some(Some(seal)) = block.post_digests.last().map(read_seal_digest) else {
			return Err(Error::<B>::MissingBlockSealDigest.into())
		};
		let parent_hash = *block.header.parent_hash();

		if let Some(inner_body) = block.body.take() {
			let check_block = B::new(block.header.clone(), inner_body);

			if !block.state_action.skip_execution_checks() {
				let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
				let inherent_data_providers = timestamp;

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

		verify_seal_signature::<B>(&seal, &pre_hash, &digests.authority)?;
		if seal.seal_source == SealSource::Compute {
			// verify compute effort
			let difficulty =
				self.client.runtime_api().compute_difficulty(parent_hash).map_err(|e| {
					Error::<B>::MissingRuntimeData(
						format!("Failed to get difficulty from runtime: {}", e).to_string(),
					)
				})?;
			ensure!(
				BlockComputeNonce { nonce: seal.nonce, pre_hash: pre_hash.as_ref().to_vec() }
					.is_valid(difficulty),
				Error::<B>::InvalidComputeNonce
			);
		}

		let best_header = self
			.select_chain
			.best_chain()
			.await
			.map_err(|e| format!("Fetch best chain failed via select chain: {}", e))
			.map_err(ConsensusError::ChainLookup)?;

		let (fork_power, best_fork_power) = UlxAux::record_block(
			&self.client,
			best_header,
			&mut block,
			digests.author,
			digests.block_vote.parent_voting_key,
			digests.block_vote.notebook_numbers.len() as u32,
			digests.block_vote.voting_power,
			seal.nonce,
			seal.is_tax(),
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
