use std::{convert::Into, sync::Arc, time::Duration};

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
use ulx_bitcoin_utxo_tracker::{get_bitcoin_inherent, UtxoTracker};

use ulx_node_runtime::{NotaryRecordT, NotebookVerifyError};
use ulx_primitives::{
	digests::FinalizedBlockNeededDigest,
	inherents::{
		BitcoinInherentDataProvider, BlockSealInherentDataProvider, BlockSealInherentNodeSide,
		NotebooksInherentDataProvider,
	},
	tick::Tick,
	Balance, BestBlockVoteSeal, BitcoinApis, BlockSealApis, BlockSealAuthorityId,
	BlockSealAuthoritySignature, BlockSealDigest, NotaryApis, NotaryId, NotebookApis, TickApis,
};

use crate::{
	aux_client::UlxAux,
	digests::{create_pre_runtime_digests, create_seal_digest},
	error::Error,
	notary_client::{get_notebook_header_data, NotaryClient},
	notebook_watch::NotebookWatch,
};

const LOG_TARGET: &str = "node::consensus::block_creator";

pub struct CreateTaxVoteBlock<Block: BlockT, AccountId: Clone + Codec> {
	pub tick: Tick,
	pub timestamp_millis: u64,
	pub parent_hash: Block::Hash,
	pub vote: BestBlockVoteSeal<AccountId, BlockSealAuthorityId>,
	pub signature: BlockSealAuthoritySignature,
}

pub fn notary_client_task<B, C, SC, AC>(
	client: Arc<C>,
	select_chain: SC,
	aux_client: UlxAux<B, C>,
	keystore: KeystorePtr,
) -> (impl Future<Output = ()>, Receiver<CreateTaxVoteBlock<B, AC>>)
where
	B: BlockT<Hash = H256>,
	C: ProvideRuntimeApi<B> + BlockchainEvents<B> + HeaderBackend<B> + AuxStore + BlockOf + 'static,
	C::Api: NotebookApis<B, NotebookVerifyError>
		+ BlockSealApis<B, AC, BlockSealAuthorityId>
		+ NotaryApis<B, NotaryRecordT>
		+ TickApis<B>,
	SC: SelectChain<B> + 'static,
	AC: Codec + Clone,
{
	let (sender, receiver) = channel(1000);

	let task = async move {
		let (header_tx, mut header_rx) =
			sc_utils::mpsc::tracing_unbounded("node::consensus::notebook_header_stream", 100);
		let notary_client =
			Arc::new(NotaryClient::new(client.clone(), aux_client.clone(), header_tx.clone()));
		let notebook_watch =
			NotebookWatch::new(client.clone(), select_chain, keystore, aux_client, sender.clone());

		let mut best_block = Box::pin(client.import_notification_stream());

		let mut update_notaries_with_hash = None;
		let mut to_remove: Vec<(NotaryId, Option<String>)> = Vec::new();

		loop {
			let _ = {
				let mut subscriptions_by_id = notary_client.subscriptions_by_id.lock().await;

				tokio::select! {biased;
					notebook =  futures::future::poll_fn(|cx| {
						let item = subscriptions_by_id.iter_mut().find_map(|(notary_id, sub)| {
							let sub = Pin::new(sub);
							match sub.poll_next(cx) {
								Pending => None,
								Ready(e) => Some((notary_id.clone(), e)),
							}
						});
						match item {
							Some((id, e)) => Ready((id, e)),
							None => Pending,
						}
					}) => {
						match notebook.into() {
							(notary_id, Some(Ok((notebook_number, header)))) => {
								match header_tx.unbounded_send((notary_id, notebook_number, header)) {
									Ok(_) => (),
									Err(e) => {
										warn!(
											"Could not send header to stream for notary {} - {:?}",
											notary_id, e
										)
									},
								}
							},
							(notary_id, None) => {
								to_remove.push((notary_id, None));
							},
							(notary_id, Some(Err(e))) => {
								let reason = e.to_string();
								to_remove.push((notary_id, Some(reason)));
							},
						}
					},
					header = header_rx.next() => {
						if let Some((notary_id, notebook_number, raw_data)) = header.into() {
							info!(target: LOG_TARGET, "Processing notebook for notary {}, #{}", notary_id, notebook_number);
							let _ = notebook_watch.on_notebook(
								notary_id,
								notebook_number,
								notary_client.clone(),
								raw_data,
							).map_err(move |e| {
								warn!(target: LOG_TARGET, "Error processing notebook from notary {}. Error: {}", notary_id, e.to_string());
							}).await;
						}
					},
					block = best_block.next () => {
						if let Some(block) = block.as_ref() {
							if block.is_new_best {
								update_notaries_with_hash = Some(block.hash);
							}
						}
					},
				}
			};

			if let Some(best_hash) = update_notaries_with_hash {
				if let Err(e) = notary_client.update_notaries(&best_hash).await {
					warn!(
						target: LOG_TARGET,
						"Could not update notaries at best hash {} - {:?}",
						best_hash,
						e
					);
				}
			}

			for (notary_id, reason) in &to_remove {
				notary_client.disconnect(&notary_id, reason.clone()).await;
			}
			to_remove.clear();
			update_notaries_with_hash = None;
			let _ = notary_client.retrieve_missing_notebooks().await;
		}
	};
	(task, receiver)
}

pub async fn tax_block_creator<B, C, E, L, CS, A>(
	mut block_import: BoxBlockImport<B>,
	client: Arc<C>,
	aux_client: UlxAux<B, C>,
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
			command.tick,
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

pub async fn propose<B, C, E, A>(
	client: Arc<C>,
	aux_client: UlxAux<B, C>,
	env: &mut E,
	author: A,
	tick: Tick,
	timestamp_millis: u64,
	parent_hash: B::Hash,
	seal_inherent: BlockSealInherentNodeSide,
	utxo_tracker: Arc<UtxoTracker>,
	max_time_to_build_block: Duration,
) -> Result<Proposal<B, <E::Proposer as Proposer<B>>::Proof>, Error<B>>
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

	let notebook_header_data =
		match get_notebook_header_data(&client, &aux_client, &parent_hash, tick).await {
			Ok(x) => x,
			Err(err) => {
				warn!(
					target: LOG_TARGET,
					"Unable to pull new block for compute miner. No notebook header data found!! {}", err
				);
				return Err(err.into());
			},
		};

	info!(target: LOG_TARGET, "Proposing block at tick {} with {} notebooks", tick, notebook_header_data.notebook_digest.notebooks.len());

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

	let latest_finalized_block_needed = notebook_header_data.latest_finalized_block_needed;
	let finalized_hash_needed = match client.hash(latest_finalized_block_needed.into()) {
		Ok(Some(x)) => x,
		Ok(None) => return Err(Error::InvalidFinalizedBlockNeeded),
		Err(err) => return Err(err.into()),
	};

	let inherent_digest = create_pre_runtime_digests(
		author,
		tick,
		notebook_header_data.vote_digest,
		FinalizedBlockNeededDigest::<B> {
			number: latest_finalized_block_needed.into(),
			hash: finalized_hash_needed,
		},
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
	let parent_hash = header.parent_hash().clone();
	let block_number = header.number().clone();

	let mut block_import_params = BlockImportParams::new(BlockOrigin::Own, header);

	let seal = create_seal_digest(&block_seal_digest);

	block_import_params.post_digests.push(seal);
	block_import_params.body = Some(body);
	block_import_params.state_action =
		StateAction::ApplyChanges(StorageChanges::Changes(proposal.storage_changes));

	let post_hash = block_import_params.post_hash();
	info!(target: LOG_TARGET, "Importing self-generated block: {:?}. {:?}", &post_hash, &block_seal_digest);
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
