use std::{convert::Into, default::Default, sync::Arc, time::Duration};

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

use ulx_node_runtime::{BlockNumber, NotaryRecordT};
use ulx_primitives::{
	digests::{BlockVoteDigest, SealSource},
	inherents::{BlockSealInherent, BlockSealInherentDataProvider},
	BlockSealAuthorityId, BlockSealMinimumApis, MiningAuthorityApis, NotaryApis, NotebookApis,
};

use crate::{
	authority::{sign_seal, AuthorityClient},
	digests::{create_digests, create_seal_digest},
	error::Error,
	notebook_watch::NotebookState,
};

const LOG_TARGET: &str = "node::consensus::block_creator";

lazy_static! {
	static ref TX_NOTEBOOK_PROVIDE_PREFIX: Vec<u8> = ("Notebook").encode();
}

pub struct CreateBlockEvent<Block: BlockT> {
	pub parent_block_number: BlockNumber,
	pub parent_hash: Block::Hash,
	pub block_vote_digest: BlockVoteDigest,
	pub seal_inherent: BlockSealInherent,
	pub nonce: U256,
	pub block_seal_authority: BlockSealAuthorityId,
	pub latest_finalized_block_needed: BlockNumber,
}

pub fn create_block_watch<B, TP, C, SC>(
	pool: Arc<TP>,
	client: Arc<C>,
	select_chain: SC,
	keystore: KeystorePtr,
) -> (impl Future<Output = ()>, Receiver<CreateBlockEvent<B>>)
where
	B: BlockT<Hash = H256>,
	C: ProvideRuntimeApi<B> + BlockchainEvents<B> + HeaderBackend<B> + AuxStore + BlockOf + 'static,
	C::Api: NotebookApis<B>
		+ BlockSealMinimumApis<B>
		+ NotaryApis<B, NotaryRecordT>
		+ MiningAuthorityApis<B>,
	TP: TransactionPool<Block = B>,
	SC: SelectChain<B>,
{
	let (sender, receiver) = channel(1000);
	let authority_sealer = AuthorityClient::<B, C>::new(client.clone(), keystore);
	let state = NotebookState::new(pool, client, select_chain, authority_sealer, sender.clone());
	let task = async move {
		let pool = state.pool.clone();
		let mut state = state;
		let mut tx_stream = Box::pin(pool.import_notification_stream());
		loop {
			tokio::select! {
				tx_hash = tx_stream.next() => {
					let Some(tx_hash) = tx_hash.as_ref() else {
						continue
					};

					let Some(tx) = pool.ready_transaction(&tx_hash) else {
						continue
					};

					let tag: TransactionTag = TX_NOTEBOOK_PROVIDE_PREFIX.to_vec();
					if tx.provides().len() > 0 && tx.provides()[0].starts_with(&tag) {
						info!("Got inbound Notebook. {:?}", tx_hash);
						let _ = state.try_process_notebook(tx.data()).await.map_err(|e| {
							warn!(
								target: LOG_TARGET,
								"Unable to process notebook. Error: {}",
								e.to_string()
							);
						});
					}
				}
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
	author: AccountId32,
	max_time_to_build_block: Duration,
	mut tax_block_create_stream: CS,
	keystore: KeystorePtr,
) where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
	C::Api: MiningAuthorityApis<Block>,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
	L: sc_consensus::JustificationSyncLink<Block> + 'static,
	CS: Stream<Item = CreateBlockEvent<Block>> + Unpin + 'static,
{
	while let Some(command) = tax_block_create_stream.next().await {
		let seal_source = command.seal_inherent.to_seal_source();
		let block_seal_authority = &command.block_seal_authority;
		let proposal = match propose(
			client.clone(),
			&mut env,
			&author,
			command.block_vote_digest,
			command.parent_block_number,
			command.parent_hash,
			command.seal_inherent,
			block_seal_authority.clone(),
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
			&keystore,
			&command.nonce,
			seal_source,
			block_seal_authority,
		)
		.await;
	}
}

pub async fn propose<Block, C, E>(
	client: Arc<C>,
	env: &mut E,
	author: &AccountId32,
	block_vote_digest: BlockVoteDigest,
	parent_block_number: BlockNumber,
	parent_hash: Block::Hash,
	seal_inherent: BlockSealInherent,
	block_seal_authority: BlockSealAuthorityId,
	latest_finalized_block_needed: BlockNumber,
	max_time_to_build_block: Duration,
) -> Result<Proposal<Block, <E::Proposer as Proposer<Block>>::Proof>, Error<Block>>
where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
	C::Api: MiningAuthorityApis<Block>,
	E: Environment<Block> + Send + Sync + 'static,
	E::Error: std::fmt::Debug,
	E::Proposer: Proposer<Block>,
{
	info!(target: LOG_TARGET, "Building next block (at={}, parent={:?})", parent_block_number + 1, parent_hash);

	let best_header = match client.header(parent_hash) {
		Ok(Some(x)) => x,
		Ok(None) => return Err(Error::BlockNotFound(parent_hash.to_string())),
		Err(err) => return Err(err.into()),
	};

	let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
	let seal = BlockSealInherentDataProvider::new(seal_inherent.clone());
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

	let proposer: E::Proposer = match env.init(&best_header).await {
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

	let inherent_digest = create_digests::<Block>(
		author,
		block_vote_digest,
		&block_seal_authority,
		latest_finalized_block_needed,
		finalized_hash_needed,
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
	Ok(proposal)
}

pub(crate) async fn submit_block<Block, L, Proof>(
	block_import: &mut BoxBlockImport<Block>,
	proposal: Proposal<Block, Proof>,
	justification_sync_link: &L,
	keystore: &KeystorePtr,
	nonce: &U256,
	seal_source: SealSource,
	block_seal_authority: &BlockSealAuthorityId,
) where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	L: sc_consensus::JustificationSyncLink<Block>,
{
	let (header, body) = proposal.block.deconstruct();
	let pre_hash = &header.hash();
	let parent_hash = header.parent_hash().clone();
	let block_number = header.number().clone();

	let mut block_import_params = BlockImportParams::new(BlockOrigin::Own, header);

	let signature = match sign_seal(&keystore, &block_seal_authority, pre_hash.as_ref(), nonce) {
		Ok(x) => x,
		Err(err) => {
			warn!(target: LOG_TARGET, "Unable to sign seal digest: {:?}", err);
			return
		},
	};

	let seal = create_seal_digest(nonce, seal_source, signature);

	block_import_params.post_digests.push(seal);
	block_import_params.body = Some(body);
	block_import_params.state_action =
		StateAction::ApplyChanges(StorageChanges::Changes(proposal.storage_changes));

	let post_hash = block_import_params.post_hash();
	info!(target: LOG_TARGET, "Importing generated block: {:?}", &post_hash);
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
				warn!(target: LOG_TARGET, "Import result not success: {:?}", other);
			},
		},
		Err(err) => {
			warn!(target: LOG_TARGET, "Unable to import block: {:?}", err);
		},
	}
}
