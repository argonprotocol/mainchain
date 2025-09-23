use crate::{
	aux_client::ArgonAux, error::Error, metrics::ConsensusMetrics,
	notary_client::get_notebook_header_data,
};
use argon_bitcoin_utxo_tracker::{UtxoTracker, get_bitcoin_inherent};
use argon_primitives::{
	Balance, BestBlockVoteSeal, BitcoinApis, BlockSealApis, BlockSealAuthorityId, BlockSealDigest,
	Digestset, NotaryApis, NotebookApis, TickApis, VotingSchedule,
	inherents::{
		BitcoinInherentDataProvider, BlockSealInherentDataProvider, BlockSealInherentNodeSide,
		NotebooksInherentDataProvider,
	},
	tick::{Tick, TickDigest, Ticker},
};
use argon_runtime::{NotaryRecordT, NotebookVerifyError};
use codec::{Codec, MaxEncodedLen};
use frame_support::CloneNoBound;
use log::*;
use polkadot_sdk::*;
use sc_client_api::AuxStore;
use sc_consensus::{BlockImport, BlockImportParams, ImportResult, StateAction, StorageChanges};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Environment, Proposal, Proposer};
use sp_inherents::{InherentData, InherentDataProvider};
use sp_runtime::{
	Digest,
	traits::{Block as BlockT, Header as HeaderT},
};
use sp_timestamp::Timestamp;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

pub struct CreateTaxVoteBlock<Block: BlockT, AccountId: Clone + Codec> {
	pub current_tick: Tick,
	pub timestamp_millis: u64,
	pub parent_hash: Block::Hash,
	pub vote: BestBlockVoteSeal<AccountId, BlockSealAuthorityId>,
}

#[derive(CloneNoBound)]
pub struct BlockCreator<Block: BlockT, BI: Clone, Client: AuxStore, PF, JS: Clone, A: Clone, B> {
	/// Used to actually import blocks.
	pub block_import: BI,
	/// The underlying para client.
	pub client: Arc<Client>,
	/// The underlying block proposer this should call into.
	pub proposer: Arc<Mutex<PF>>,
	/// The amount of time to spend authoring each block.
	pub authoring_duration: Duration,
	pub justification_sync_link: JS,
	pub aux_client: ArgonAux<Block, Client>,
	pub backend: Arc<B>,
	pub utxo_tracker: Arc<UtxoTracker>,
	pub(crate) _phantom: std::marker::PhantomData<A>,
	pub(crate) metrics: Arc<Option<ConsensusMetrics<Client>>>,
}

pub struct ProposalMeta {
	pub notebooks: u32,
	pub tick: Tick,
	pub is_compute: bool,
}

impl<Block: BlockT, BI, C, PF, JS, A, Proof, B> BlockCreator<Block, BI, C, PF, JS, A, B>
where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	BI: BlockImport<Block> + Clone + Send + Sync + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + AuxStore + 'static,
	C::Api: NotebookApis<Block, NotebookVerifyError>
		+ BlockSealApis<Block, A, BlockSealAuthorityId>
		+ NotaryApis<Block, NotaryRecordT>
		+ TickApis<Block>
		+ BitcoinApis<Block, Balance>,
	PF: Environment<Block> + Send + Sync + 'static,
	PF::Proposer: Proposer<Block, Proof = Proof>,
	A: Codec + MaxEncodedLen + Clone + Send + Sync + 'static,
	JS: sc_consensus::JustificationSyncLink<Block> + Clone + Send + Sync + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
{
	pub async fn propose(
		&self,
		author: A,
		submitting_tick: Tick,
		timestamp_millis: u64,
		parent_hash: Block::Hash,
		seal_inherent: BlockSealInherentNodeSide,
	) -> Option<BlockProposal<Block, Proof>> {
		let parent_header = match self.client.header(parent_hash) {
			Ok(Some(x)) => x,
			Ok(None) => {
				tracing::warn!("Parent header not found {:?}", parent_hash);
				return None;
			},
			Err(err) => {
				tracing::error!(?err, ?parent_hash, "Error while fetching parent header");
				return None;
			},
		};

		let (inherent_data, inherent_digest, proposal_meta) = self
			.create_inherents(author, parent_hash, submitting_tick, timestamp_millis, seal_inherent)
			.await
			.ok()?;

		let mut proposer = self.proposer.lock().await;
		let proposer: PF::Proposer = match proposer.init(&parent_header).await {
			Ok(x) => x,
			Err(err) => {
				tracing::warn!(?err, "Unable to propose. Creating proposer failed");
				return None;
			},
		};
		let size_limit = None;
		let proposal = proposer
			.propose(inherent_data, inherent_digest, self.authoring_duration, size_limit)
			.await
			.inspect_err(|err| {
				tracing::warn!(?err, "Unable to propose. Creating proposer failed");
			})
			.ok()?;

		Some(BlockProposal { proposal, proposal_meta })
	}

	async fn create_inherents(
		&self,
		author: A,
		parent_hash: Block::Hash,
		submitting_tick: Tick,
		timestamp_millis: u64,
		seal_inherent: BlockSealInherentNodeSide,
	) -> Result<(InherentData, Digest, ProposalMeta), Error> {
		let voting_schedule = VotingSchedule::when_creating_block(submitting_tick);
		let notebook_header_data = get_notebook_header_data(
			&self.client,
			&self.aux_client,
			&parent_hash,
			&voting_schedule,
		)
		.await
		.inspect_err(|err| {
			tracing::warn!(?err, "Unable to get inherent data");
		})?;

		info!(
			"Proposing block at tick {} with {} notebooks",
			submitting_tick,
			notebook_header_data.notebook_digest.notebooks.len()
		);

		let meta = ProposalMeta {
			notebooks: notebook_header_data.notebook_digest.notebooks.len() as u32,
			tick: submitting_tick,
			is_compute: matches!(seal_inherent, BlockSealInherentNodeSide::Compute),
		};

		let timestamp = sp_timestamp::InherentDataProvider::new(Timestamp::new(timestamp_millis));
		let seal =
			BlockSealInherentDataProvider { seal: Some(seal_inherent.clone()), digest: None };
		let notebooks =
			NotebooksInherentDataProvider { raw_notebooks: notebook_header_data.signed_headers };

		let mut inherent_data =
			(timestamp, seal, notebooks).create_inherent_data().await.inspect_err(|err| {
				tracing::warn!(
					?err,
					"Unable to propose new block for authoring. Creating inherent data failed",
				);
			})?;

		let bitcoin_utxo_sync =
			get_bitcoin_inherent(&self.utxo_tracker, &self.client, &parent_hash).unwrap_or_else(
				|err| {
					tracing::warn!(?err, "Unable to get bitcoin inherent");
					None
				},
			);
		if let Some(bitcoin_utxo_sync) = bitcoin_utxo_sync {
			BitcoinInherentDataProvider { bitcoin_utxo_sync }
				.provide_inherent_data(&mut inherent_data)
				.await
				.inspect_err(|err| {
					tracing::warn!(?err, "Unable to provide bitcoin inherent data");
				})?;
		}

		let inherent_digest = Digestset {
			author,
			tick: TickDigest(submitting_tick),
			block_vote: notebook_header_data.vote_digest,
			notebooks: notebook_header_data.notebook_digest,
			// these are from the runtime
			voting_key: Default::default(),
			fork_power: Default::default(),
		}
		.create_pre_runtime_digest();

		Ok((inherent_data, inherent_digest, meta))
	}

	pub async fn submit_block(
		&self,
		block_proposal: BlockProposal<Block, Proof>,
		block_seal_digest: BlockSealDigest,
		ticker: &Ticker,
	) {
		let BlockProposal { proposal, proposal_meta } = block_proposal;

		let (pre_header, body) = proposal.block.deconstruct();
		let pre_hash = pre_header.hash();
		let parent_hash = *pre_header.parent_hash();
		let block_number = *pre_header.number();

		// seal the block.
		let seal = block_seal_digest.to_digest();
		let mut block_import_params = BlockImportParams::new(BlockOrigin::Own, pre_header);

		block_import_params.post_digests.push(seal);
		block_import_params.body = Some(body.clone());
		block_import_params.state_action =
			StateAction::ApplyChanges(StorageChanges::Changes(proposal.storage_changes));
		let post_hash = block_import_params.post_hash();

		if let Err(e) = self.backend.state_at(parent_hash) {
			tracing::warn!(
				err = ?e,
				"🚽 Parent block not found in state at {}. Likely dumped. Skipping block submission.",
				parent_hash,
			);
			return;
		}

		tracing::trace!(
			"🔖 Pre-sealed block for proposal at {}. Hash now {}, previously {}.",
			block_number,
			post_hash,
			pre_hash,
		);

		// ensure we don't dump the parent block, which will get us banned if we broadcast this and
		// don't have it on request
		let _ = self.backend.pin_block(parent_hash);
		match self.block_import.import_block(block_import_params).await {
			Ok(res) => match res {
				ImportResult::Imported(_) => {
					res.handle_justification(
						&post_hash,
						block_number,
						&self.justification_sync_link,
					);
					if let Some(metrics) = self.metrics.as_ref() {
						metrics.on_block_created(ticker, &proposal_meta);
					}
					tracing::info!(
						"✅ Successfully mined block on top of: {} -> {}",
						parent_hash,
						post_hash
					);
				},
				other => {
					self.backend.unpin_block(parent_hash);
					warn!("Import of own block - result not success: {:?}", other);
				},
			},
			Err(e) => {
				self.backend.unpin_block(parent_hash);
				tracing::error!(?e, "Failed to produce candidate");
			},
		}
	}
}

pub struct BlockProposal<Block: BlockT, Proof> {
	pub proposal: Proposal<Block, Proof>,
	pub proposal_meta: ProposalMeta,
}
