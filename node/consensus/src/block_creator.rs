use std::{convert::Into, sync::Arc, time::Duration};

use argon_bitcoin_utxo_tracker::{get_bitcoin_inherent, UtxoTracker};
use codec::Codec;
use futures::{channel::mpsc::*, prelude::*};
use log::*;
use sc_client_api::{AuxStore, BlockOf, BlockchainEvents};
use sc_consensus::{
	BlockImport, BlockImportParams, BoxBlockImport, ImportResult, StateAction, StorageChanges,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Environment, Proposal, Proposer, SelectChain};
use sp_core::H256;
use sp_inherents::InherentDataProvider;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use sp_timestamp::Timestamp;

use argon_node_runtime::{NotaryRecordT, NotebookVerifyError};
use argon_primitives::{
	inherents::{
		BitcoinInherentDataProvider, BlockSealInherentDataProvider, BlockSealInherentNodeSide,
		NotebooksInherentDataProvider,
	},
	tick::Tick,
	Balance, BestBlockVoteSeal, BitcoinApis, BlockSealApis, BlockSealAuthorityId,
	BlockSealAuthoritySignature, BlockSealDigest, NotaryApis, NotebookApis, TickApis,
	VotingSchedule,
};

use crate::{
	aux_client::ArgonAux,
	digests::{create_pre_runtime_digests, create_seal_digest},
	error::Error,
	notary_client::{get_notebook_header_data, notary_sync_task, NotaryClient},
	notebook_sealer::NotebookSealer,
};

const LOG_TARGET: &str = "node::consensus::block_creator";

pub struct CreateTaxVoteBlock<Block: BlockT, AccountId: Clone + Codec> {
	pub current_tick: Tick,
	pub timestamp_millis: u64,
	pub parent_hash: Block::Hash,
	pub vote: BestBlockVoteSeal<AccountId, BlockSealAuthorityId>,
	pub signature: BlockSealAuthoritySignature,
}

pub fn block_creation_task<B, C, SC, AC>(
	client: Arc<C>,
	select_chain: SC,
	aux_client: ArgonAux<B, C>,
	keystore: KeystorePtr,
) -> (
	impl Future<Output = ()>,
	impl Future<Output = ()>,
	impl Future<Output = ()>,
	Receiver<CreateTaxVoteBlock<B, AC>>,
)
where
	B: BlockT<Hash = H256>,
	C: ProvideRuntimeApi<B>
		+ BlockchainEvents<B>
		+ HeaderBackend<B>
		+ AuxStore
		+ BlockOf
		+ Send
		+ Sync
		+ 'static,
	C::Api: NotebookApis<B, NotebookVerifyError>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ NotaryApis<B, NotaryRecordT>
		+ TickApis<B>,
	SC: SelectChain<B> + 'static,
	AC: Codec + Clone + Send + Sync + 'static,
{
	let (tax_vote_sender, tax_vote_rx) = channel(1000);
	let (notebook_tick_tx, mut notebook_tick_rx) =
		sc_utils::mpsc::tracing_unbounded("node::consensus::notebook_tick_stream", 100);
	let notary_client =
		Arc::new(NotaryClient::new(client.clone(), aux_client.clone(), notebook_tick_tx.clone()));

	let best_hash = client.info().best_hash;
	let ticker = client.runtime_api().ticker(best_hash).expect("Ticker not available");
	let tick_millis = ticker.tick_duration_millis;

	let notary_sync_task = notary_sync_task(client.clone(), notary_client.clone());

	let notary_queue_task = async move {
		loop {
			let has_more_work = notary_client
				.process_queues()
				.await
				.map_err(|err| {
					warn!(target: LOG_TARGET, "Error while processing notary queues: {:?}", err);
				})
				.unwrap_or(false);

			let mut delay = 20;
			if !has_more_work {
				if tick_millis <= 10_000 {
					delay = 100;
				} else {
					delay = 1000;
				}
			}
			tokio::time::sleep(Duration::from_millis(delay)).await;
		}
	};

	let seal_watch_task = async move {
		let notebook_sealer = NotebookSealer::new(
			client.clone(),
			ticker,
			select_chain,
			keystore,
			aux_client,
			tax_vote_sender.clone(),
		);

		while let Some((notebook_tick, voting_power, notebooks)) = notebook_tick_rx.next().await {
			if let Err(err) = notebook_sealer
				.check_for_new_blocks(notebook_tick, voting_power, notebooks)
				.await
			{
				warn!(target: LOG_TARGET, "Error while checking for new blocks: {:?}", err);
			}
		}
	};
	(seal_watch_task, notary_sync_task, notary_queue_task, tax_vote_rx)
}

#[allow(clippy::too_many_arguments)]
pub async fn tax_block_creator<B, C, E, L, CS, A>(
	mut block_import: BoxBlockImport<B>,
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	mut env: E,
	justification_sync_link: L,
	max_time_to_build_block: Duration,
	mut tax_block_create_stream: CS,
	utxo_tracker: Arc<UtxoTracker>,
) where
	B: BlockT + 'static,
	B::Hash: Send + 'static,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + AuxStore + 'static,
	C::Api: NotebookApis<B, NotebookVerifyError>
		+ BlockSealApis<B, A, BlockSealAuthorityId>
		+ NotaryApis<B, NotaryRecordT>
		+ TickApis<B>
		+ BitcoinApis<B, Balance>,
	E: Environment<B> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<B>,
	L: sc_consensus::JustificationSyncLink<B> + 'static,
	CS: Stream<Item = CreateTaxVoteBlock<B, A>> + Unpin + 'static,
	A: Codec + Clone + Send + Sync + 'static,
{
	while let Some(command) = tax_block_create_stream.next().await {
		let vote = command.vote;
		let seal_strength = vote.seal_strength;

		let proposal = match propose(
			client.clone(),
			aux_client.clone(),
			&mut env,
			vote.closest_miner.0.clone(),
			command.current_tick,
			command.timestamp_millis,
			command.parent_hash,
			BlockSealInherentNodeSide::from_vote(vote, command.signature),
			utxo_tracker.clone(),
			max_time_to_build_block,
		)
		.await
		{
			Ok(x) => x,
			Err(err) => {
				warn!(target: LOG_TARGET, "Unable to propose new block: {:?}", err);
				continue;
			},
		};
		submit_block::<B, L, _>(
			&mut block_import,
			proposal,
			&justification_sync_link,
			BlockSealDigest::Vote { seal_strength },
		)
		.await;
	}
}

#[allow(clippy::too_many_arguments)]
pub async fn propose<B, C, E, A>(
	client: Arc<C>,
	aux_client: ArgonAux<B, C>,
	env: &mut E,
	author: A,
	submitting_tick: Tick,
	timestamp_millis: u64,
	parent_hash: B::Hash,
	seal_inherent: BlockSealInherentNodeSide,
	utxo_tracker: Arc<UtxoTracker>,
	max_time_to_build_block: Duration,
) -> Result<Proposal<B, <E::Proposer as Proposer<B>>::Proof>, Error>
where
	B: BlockT + 'static,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + AuxStore + 'static,
	C::Api: NotebookApis<B, NotebookVerifyError>
		+ BlockSealApis<B, A, BlockSealAuthorityId>
		+ NotaryApis<B, NotaryRecordT>
		+ TickApis<B>
		+ BitcoinApis<B, Balance>,
	E: Environment<B> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<B>,
	A: Codec + Clone,
{
	let parent_header = match client.header(parent_hash) {
		Ok(Some(x)) => x,
		Ok(None) => return Err(Error::BlockNotFound(parent_hash.to_string())),
		Err(err) => return Err(err.into()),
	};

	let bitcoin_utxo_sync = get_bitcoin_inherent(&utxo_tracker, &client, &parent_hash)
		.unwrap_or_else(|err| {
			warn!(target: LOG_TARGET, "Unable to get bitcoin inherent: {:?}", err);
			None
		});

	let voting_schedule = VotingSchedule::when_creating_block(submitting_tick);
	let notebook_header_data = match get_notebook_header_data(
		&client,
		&aux_client,
		&parent_hash,
		&voting_schedule,
	)
	.await
	{
		Ok(x) => x,
		Err(err) => {
			warn!(
				target: LOG_TARGET,
				"Unable to pull new block for compute miner. No notebook header data found!! {}", err
			);
			return Err(err);
		},
	};

	info!(target: LOG_TARGET, "Proposing block at tick {} with {} notebooks", submitting_tick, notebook_header_data.notebook_digest.notebooks.len());

	let timestamp = sp_timestamp::InherentDataProvider::new(Timestamp::new(timestamp_millis));
	let seal = BlockSealInherentDataProvider { seal: Some(seal_inherent.clone()), digest: None };
	let notebooks =
		NotebooksInherentDataProvider { raw_notebooks: notebook_header_data.signed_headers };

	let mut inherent_data = match (timestamp, seal, notebooks).create_inherent_data().await {
		Ok(r) => r,
		Err(err) => {
			warn!(
				target: LOG_TARGET,
				"Unable to propose new block for authoring. \
				 Creating inherent data failed: {:?}",
				err,
			);
			return Err(err.into());
		},
	};

	if let Some(bitcoin_utxo_sync) = bitcoin_utxo_sync {
		BitcoinInherentDataProvider { bitcoin_utxo_sync }
			.provide_inherent_data(&mut inherent_data)
			.await?;
	}

	let proposer: E::Proposer = match env.init(&parent_header).await {
		Ok(x) => x,
		Err(err) => {
			let msg = format!(
				"Unable to propose new block for authoring. \
						Initializing proposer failed: {:?}",
				err
			);
			return Err(Error::StringError(msg));
		},
	};

	let inherent_digest = create_pre_runtime_digests(
		author,
		submitting_tick,
		notebook_header_data.vote_digest,
		notebook_header_data.notebook_digest,
	);

	let proposal = match proposer
		.propose(inherent_data, inherent_digest, max_time_to_build_block, None)
		.await
	{
		Ok(x) => x,
		Err(err) => {
			let msg = format!("Unable to propose. Creating proposer failed: {:?}", err);
			return Err(Error::StringError(msg));
		},
	};
	Ok(proposal)
}

pub(crate) async fn submit_block<Block, L, Proof>(
	block_import: &mut BoxBlockImport<Block>,
	proposal: Proposal<Block, Proof>,
	justification_sync_link: &L,
	block_seal_digest: BlockSealDigest,
) where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	L: sc_consensus::JustificationSyncLink<Block>,
{
	let (header, body) = proposal.block.deconstruct();
	let parent_hash = *header.parent_hash();
	let block_number = *header.number();

	let mut block_import_params = BlockImportParams::new(BlockOrigin::Own, header);

	let seal = create_seal_digest(&block_seal_digest);

	block_import_params.post_digests.push(seal);
	block_import_params.body = Some(body);
	block_import_params.state_action =
		StateAction::ApplyChanges(StorageChanges::Changes(proposal.storage_changes));

	let post_hash = block_import_params.post_hash();
	trace!(target: LOG_TARGET, "Importing self-generated block: {:?}. {:?}", &post_hash, &block_seal_digest);
	match block_import.import_block(block_import_params).await {
		Ok(res) => match res {
			ImportResult::Imported(_) => {
				res.handle_justification(&post_hash, block_number, justification_sync_link);

				info!(
					target: LOG_TARGET,
					"✅ Successfully mined block on top of: {} -> {}", parent_hash, post_hash
				);
			},
			other => {
				warn!(target: LOG_TARGET, "Import of own block - result not success: {:?}", other);
			},
		},
		Err(err) => {
			warn!(target: LOG_TARGET, "Unable to import own block: {:?}", err);
		},
	}
}
