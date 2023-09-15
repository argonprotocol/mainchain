use std::{cmp::Ordering, marker::PhantomData, sync::Arc};

use codec::Decode;
use log::{info, trace};
use prometheus_endpoint::Registry;
use sc_client_api::{self, backend::AuxStore, BlockOf};
use sc_consensus::{
	BasicQueue, BlockCheckParams, BlockImport, BlockImportParams, BoxBlockImport,
	BoxJustificationImport, ForkChoiceStrategy, ImportResult, Verifier,
};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{Error as ConsensusError, SelectChain};
use sp_core::{crypto::AccountId32, U256};
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::{
	generic::DigestItem,
	traits::{Block as BlockT, Header as HeaderT},
};

use ulx_primitives::{
	inherents::UlxBlockSealInherent, AuthorityApis, ProofOfWorkType, UlxPreDigest, ULX_ENGINE_ID,
};

pub use crate::compute_worker::{MiningBuild, MiningHandle, MiningMetadata};
use crate::{authority::AuthoritySealer, aux::UlxAux, error::Error, NonceAlgorithm};

const LOG_TARGET: &str = "node::consensus::import_queue";
/// A block importer for Ulx.
pub struct UlxBlockImport<B: BlockT, I, C, S, Algorithm, CIDP> {
	pub algorithm: Algorithm,
	inner: I,
	select_chain: S,
	client: Arc<C>,
	create_inherent_data_providers: Arc<CIDP>,
	_block: PhantomData<B>,
}

impl<B: BlockT, I: Clone, C, S: Clone, Algorithm: Clone, CIDP> Clone
	for UlxBlockImport<B, I, C, S, Algorithm, CIDP>
{
	fn clone(&self) -> Self {
		Self {
			algorithm: self.algorithm.clone(),
			inner: self.inner.clone(),
			select_chain: self.select_chain.clone(),
			client: self.client.clone(),
			create_inherent_data_providers: self.create_inherent_data_providers.clone(),
			_block: PhantomData,
		}
	}
}

impl<B, I, C, S, Algorithm, CIDP> UlxBlockImport<B, I, C, S, Algorithm, CIDP>
where
	B: BlockT,
	I: BlockImport<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf,
	C::Api: BlockBuilderApi<B>,
	C::Api: AuthorityApis<B>,
	Algorithm: NonceAlgorithm<B>,
	CIDP: CreateInherentDataProviders<B, UlxBlockSealInherent> + Clone,
{
	/// Create a new block import suitable to be used in Ulx
	pub fn new(
		inner: I,
		client: Arc<C>,
		algorithm: Algorithm,
		select_chain: S,
		create_inherent_data_providers: CIDP,
	) -> Self {
		Self {
			inner,
			client,
			algorithm,
			select_chain,
			create_inherent_data_providers: Arc::new(create_inherent_data_providers),
			_block: PhantomData,
		}
	}

	async fn check_inherents(
		&self,
		block: B,
		at_hash: B::Hash,
		inherent_data_providers: CIDP::InherentDataProviders,
	) -> Result<(), Error<B>> {
		let inherent_data = inherent_data_providers
			.create_inherent_data()
			.await
			.map_err(|e| Error::CreateInherents(e))?;

		// inherent data passed in is what we would have generated...
		let inherent_res = self
			.client
			.runtime_api()
			.check_inherents(at_hash, block, inherent_data)
			.map_err(|e| Error::Client(e.into()))?;

		if !inherent_res.ok() {
			for (identifier, error) in inherent_res.into_errors() {
				match inherent_data_providers.try_handle_error(&identifier, &error).await {
					Some(res) => res.map_err(Error::CheckInherents)?,
					None => return Err(Error::CheckInherentsUnknownError(identifier)),
				}
			}
		}

		Ok(())
	}
}

/// Find Ulx pre-runtime.
pub fn find_pre_digest<B: BlockT>(header: &B::Header) -> Result<Option<UlxPreDigest>, Error<B>> {
	let mut pre_digest: Option<UlxPreDigest> = None;
	for log in header.digest().logs() {
		trace!(target: LOG_TARGET, "Checking log {:?}, looking for pre runtime digest", log);
		match (log, pre_digest.is_some()) {
			(DigestItem::PreRuntime(ULX_ENGINE_ID, _), true) =>
				return Err(Error::MultiplePreRuntimeDigests),
			(DigestItem::PreRuntime(ULX_ENGINE_ID, v), false) => {
				let ulx_pre_digest =
					UlxPreDigest::decode(&mut &v[..]).map_err(|e| Error::<B>::Codec(e.clone()))?;
				pre_digest = Some(ulx_pre_digest);
			},
			(_, _) => trace!(target: LOG_TARGET, "Ignoring digest not meant for us"),
		}
	}

	Ok(pre_digest)
}

#[async_trait::async_trait]
impl<B, I, C, S, Algorithm, CIDP> BlockImport<B> for UlxBlockImport<B, I, C, S, Algorithm, CIDP>
where
	B: BlockT,
	I: BlockImport<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	S: SelectChain<B>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf,
	C::Api: BlockBuilderApi<B>,
	C::Api: AuthorityApis<B>,
	Algorithm: NonceAlgorithm<B> + Send + Sync,
	Algorithm::Difficulty: 'static + Send + From<u128>,
	CIDP: CreateInherentDataProviders<B, UlxBlockSealInherent> + Clone + Send + Sync,
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
		info!("Importing block with hash {:?} {}", block.post_hash(), block.header.number());
		let pre_digest = match find_pre_digest::<B>(&block.header) {
			Ok(Some(x)) => x,
			Ok(None) => return Err(Error::<B>::MissingPreRuntimeDigest.into()),
			Err(x) => return Err(x.into()),
		};

		// ensure our digest matches what is proposed in the pre-digest
		let calculated_digest = self.algorithm.next_digest(&block.header.parent_hash())?;
		if calculated_digest.work_type != pre_digest.work_type {
			return Err(Error::<B>::InvalidPredigestWorkType.into())
		}

		if calculated_digest.difficulty != pre_digest.difficulty {
			return Err(Error::<B>::InvalidPredigestDifficulty.into())
		}

		let parent_hash = *block.header.parent_hash();
		let seal = AuthoritySealer::<B, C, AccountId32>::fetch_ulx_seal(
			block.post_digests.last(),
			block.header.hash(),
		)?;
		if let Some(inner_body) = block.body.take() {
			let check_block = B::new(block.header.clone(), inner_body);

			if !block.state_action.skip_execution_checks() {
				let ulx_block_seal = UlxBlockSealInherent {
					work_type: pre_digest.work_type,
					tax_nonce: Some(U256::from(seal.nonce)),
					// We don't need tax block proof during this phase because we're simply checking
					// that the block seal matches the inherent nonce
					tax_block_proof: None,
				};

				// UlxBlockSealInherent will verify in the BlockSeal pallet that the nonce matches
				// the seal if this is proof of tax
				self.check_inherents(
					check_block.clone(),
					parent_hash,
					self.create_inherent_data_providers
						.create_inherent_data_providers(parent_hash, ulx_block_seal)
						.await?,
				)
				.await?;
			}

			block.body = Some(check_block.deconstruct().1);
		}

		let pre_hash = block.header.hash();

		if pre_digest.work_type == ProofOfWorkType::Tax {
			AuthoritySealer::<B, C, AccountId32>::verify_seal_signature(
				self.client.clone(),
				&seal,
				&parent_hash,
				&pre_hash,
			)?;
		}

		if !self.algorithm.verify(&parent_hash, &pre_hash, &pre_digest, &seal)? {
			return Err(Error::<B>::InvalidNonceDifficulty.into())
		}

		let best_header = self
			.select_chain
			.best_chain()
			.await
			.map_err(|e| format!("Fetch best chain failed via select chain: {}", e))
			.map_err(ConsensusError::ChainLookup)?;

		let (aux, best_aux) =
			UlxAux::record(&self.client, best_header, &mut block, pre_digest.difficulty)?;

		if block.fork_choice.is_none() {
			block.fork_choice = Some(ForkChoiceStrategy::Custom(
				match aux.total_difficulty.cmp(&best_aux.total_difficulty) {
					Ordering::Less => false,
					Ordering::Greater => true,
					Ordering::Equal => false,
				},
			));
		}
		info!(
			"Imported block {:?} {} {}",
			block.post_hash, aux.total_difficulty, &best_aux.total_difficulty
		);

		self.inner.import_block(block).await.map_err(Into::into)
	}
}

/// A verifier for Ulx blocks.
pub struct UlxVerifier<B: BlockT, Algorithm> {
	_algorithm: Algorithm,
	_marker: PhantomData<B>,
}

impl<B: BlockT, Algorithm> UlxVerifier<B, Algorithm> {
	pub fn new(algorithm: Algorithm) -> Self {
		Self { _algorithm: algorithm, _marker: PhantomData }
	}
}

#[async_trait::async_trait]
impl<B: BlockT, Algorithm> Verifier<B> for UlxVerifier<B, Algorithm>
where
	Algorithm: NonceAlgorithm<B> + Send + Sync,
	Algorithm::Difficulty: 'static + Send,
{
	async fn verify(
		&mut self,
		mut block: BlockImportParams<B>,
	) -> Result<BlockImportParams<B>, String> {
		let mut header = block.header;
		let hash = header.hash();

		let seal = match header.digest_mut().pop() {
			Some(DigestItem::Seal(id, seal)) =>
				if id == ULX_ENGINE_ID {
					Ok(DigestItem::Seal(id, seal.clone()))
				} else {
					Err(Error::<B>::WrongEngine(id))
				},
			_ => Err(Error::<B>::HeaderUnsealed(hash)),
		}?;

		block.header = header;
		block.post_digests.push(seal);
		block.post_hash = Some(hash);

		Ok(block)
	}
}

/// The Ulx import queue type.
pub type UlxImportQueue<B> = BasicQueue<B>;

/// Import queue for Ulx engine.
pub fn new<B, Algorithm>(
	block_import: BoxBlockImport<B>,
	justification_import: Option<BoxJustificationImport<B>>,
	algorithm: Algorithm,
	spawner: &impl sp_core::traits::SpawnEssentialNamed,
	registry: Option<&Registry>,
) -> Result<UlxImportQueue<B>, sp_consensus::Error>
where
	B: BlockT,
	Algorithm: NonceAlgorithm<B> + Clone + Send + Sync + 'static,
	Algorithm::Difficulty: Send,
{
	let verifier = UlxVerifier::new(algorithm);

	Ok(BasicQueue::new(verifier, block_import, justification_import, spawner, registry))
}
