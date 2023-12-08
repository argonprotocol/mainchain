use std::{convert::Into, sync::Arc, time::Duration};

use codec::Encode;
use futures::{channel::mpsc::*, prelude::*};
use lazy_static::lazy_static;
use log::*;
use sc_client_api::{AuxStore, BlockOf, BlockchainEvents};
use sc_consensus::{
	BlockImport, BlockImportParams, BoxBlockImport, ImportResult, StateAction, StorageChanges,
};
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Environment, Proposal, Proposer, SelectChain};
use sp_core::{crypto::AccountId32, H256, U256};
use sp_inherents::InherentDataProvider;
use sp_keystore::KeystorePtr;
use sp_runtime::{
	traits::{Block as BlockT, Header as HeaderT},
	transaction_validity::TransactionTag,
};
use sp_timestamp::Timestamp;

use ulx_node_runtime::{BlockNumber, NotaryRecordT};
use ulx_primitives::{
	digests::{BlockVoteDigest, FinalizedBlockNeededDigest},
	inherents::{BlockSealInherent, BlockSealInherentDataProvider},
	tick::Tick,
	BlockSealDigest, BlockSealSpecApis, MiningAuthorityApis, NotaryApis, NotebookApis,
};

use crate::{
	digests::{create_digests, create_seal_digest},
	error::Error,
	notebook_auditor::NotebookAuditor,
	notebook_watch::NotebookState,
};

const LOG_TARGET: &str = "node::consensus::block_creator";

lazy_static! {
	static ref TX_NOTEBOOK_PROVIDE_PREFIX: Vec<u8> = ("Notebook").encode();
}

pub struct CreateTaxVoteBlock<Block: BlockT> {
	pub tick: Tick,
	pub timestamp_millis: u64,
	pub account_id: AccountId32,
	pub parent_hash: Block::Hash,
	pub block_vote_digest: BlockVoteDigest,
	pub seal_inherent: BlockSealInherent,
	pub vote_proof: U256,
	pub latest_finalized_block_needed: BlockNumber,
}

pub fn create_block_watch<B, TP, C, SC>(
	pool: Arc<TP>,
	client: Arc<C>,
	select_chain: SC,
	keystore: KeystorePtr,
) -> (impl Future<Output = ()>, Receiver<CreateTaxVoteBlock<B>>)
where
	B: BlockT<Hash = H256>,
	C: ProvideRuntimeApi<B> + BlockchainEvents<B> + HeaderBackend<B> + AuxStore + BlockOf + 'static,
	C::Api: NotebookApis<B>
		+ BlockSealSpecApis<B>
		+ NotaryApis<B, NotaryRecordT>
		+ MiningAuthorityApis<B>,
	TP: TransactionPool<Block = B>,
	SC: SelectChain<B>,
{
	let (sender, receiver) = channel(1000);
	let auditor = NotebookAuditor::new(client.clone());
	let state = NotebookState::new(pool, client.clone(), select_chain, keystore, sender.clone());
	let task = async move {
		let pool = state.pool.clone();
		let mut state = state;
		let mut auditor = auditor;
		let mut tx_stream = Box::pin(pool.import_notification_stream());
		while let Some(tx_hash) = tx_stream.next().await {
			let Some(tx) = pool.ready_transaction(&tx_hash) else { continue };

			let tag: TransactionTag = TX_NOTEBOOK_PROVIDE_PREFIX.to_vec();
			if tx.provides().len() > 0 && tx.provides()[0].starts_with(&tag) {
				info!("Got inbound Notebook. {:?}", tx_hash);
				let _ = state.try_process_notebook(tx.data(), &mut auditor).await.map_err(|e| {
					warn!(
						target: LOG_TARGET,
						"Unable to process notebook. Error: {}",
						e.to_string()
					);
				});
			}
		}
	};
	(task, receiver)
}
pub async fn tax_block_creator<Block, C, E, L, CS>(
	mut block_import: BoxBlockImport<Block>,
	client: Arc<C>,
	mut env: E,
	justification_sync_link: L,
	max_time_to_build_block: Duration,
	mut tax_block_create_stream: CS,
) where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
	C::Api: MiningAuthorityApis<Block>,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
	L: sc_consensus::JustificationSyncLink<Block> + 'static,
	CS: Stream<Item = CreateTaxVoteBlock<Block>> + Unpin + 'static,
{
	while let Some(command) = tax_block_create_stream.next().await {
		let vote_proof = match &command.seal_inherent {
			BlockSealInherent::Vote { vote_proof, .. } => vote_proof.clone(),
			_ => {
				warn!(target: LOG_TARGET, "Unable to propose new block - wrong seal inherent");
				continue
			},
		};

		let proposal = match propose(
			client.clone(),
			&mut env,
			command.account_id,
			command.tick,
			command.timestamp_millis,
			command.block_vote_digest,
			command.parent_hash,
			command.seal_inherent,
			command.latest_finalized_block_needed,
			max_time_to_build_block,
		)
		.await
		{
			Ok(x) => x,
			Err(err) => {
				warn!(target: LOG_TARGET, "Unable to propose new block: {:?}", err);
				continue
			},
		};
		submit_block::<Block, L, _>(
			&mut block_import,
			proposal,
			&justification_sync_link,
			BlockSealDigest::Vote { vote_proof },
		)
		.await;
	}
}

pub async fn propose<B, C, E>(
	client: Arc<C>,
	env: &mut E,
	author: AccountId32,
	tick: Tick,
	timestamp_millis: u64,
	block_vote_digest: BlockVoteDigest,
	parent_hash: B::Hash,
	seal_inherent: BlockSealInherent,
	latest_finalized_block_needed: BlockNumber,
	max_time_to_build_block: Duration,
) -> Result<Proposal<B, <E::Proposer as Proposer<B>>::Proof>, Error<B>>
where
	B: BlockT + 'static,
	C: ProvideRuntimeApi<B> + HeaderBackend<B> + 'static,
	E: Environment<B> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<B>,
{
	let parent_header = match client.header(parent_hash) {
		Ok(Some(x)) => x,
		Ok(None) => return Err(Error::BlockNotFound(parent_hash.to_string())),
		Err(err) => return Err(err.into()),
	};

	let timestamp = sp_timestamp::InherentDataProvider::new(Timestamp::new(timestamp_millis));
	let seal = BlockSealInherentDataProvider { seal: Some(seal_inherent.clone()), digest: None };
	let inherent_data = match (timestamp, seal).create_inherent_data().await {
		Ok(r) => r,
		Err(err) => {
			warn!(
				target: LOG_TARGET,
				"Unable to propose new block for authoring. \
				 Creating inherent data failed: {:?}",
				err,
			);
			return Err(err.into())
		},
	};

	let proposer: E::Proposer = match env.init(&parent_header).await {
		Ok(x) => x,
		Err(err) => {
			let msg = format!(
				"Unable to propose new block for authoring. \
						Initializing proposer failed: {:?}",
				err
			);
			return Err(Error::StringError(msg))
		},
	};

	let finalized_hash_needed = match client.hash(latest_finalized_block_needed.into()) {
		Ok(Some(x)) => x,
		Ok(None) => return Err(Error::InvalidFinalizedBlockNeeded),
		Err(err) => return Err(err.into()),
	};

	let inherent_digest = create_digests::<B>(
		author,
		tick,
		block_vote_digest,
		FinalizedBlockNeededDigest {
			number: latest_finalized_block_needed.into(),
			hash: finalized_hash_needed,
		},
	);

	let proposal = match proposer
		.propose(inherent_data, inherent_digest, max_time_to_build_block, None)
		.await
	{
		Ok(x) => x,
		Err(err) => {
			let msg = format!("Unable to propose. Creating proposer failed: {:?}", err);
			return Err(Error::StringError(msg))
		},
	};
	info!(target: LOG_TARGET, "Building next block (tick={}, parent={:?} ({})). Pre-hash {:?}", tick, parent_hash, parent_header.number(), proposal.block.header().hash());
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
