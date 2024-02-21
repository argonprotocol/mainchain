use codec::Decode;
use sp_core::{ed25519, H256};
use sqlx::{PgConnection, PgPool};
use subxt::{
	blocks::{Block, BlockRef},
	config::substrate::DigestItem,
};
use tracing::info;

pub use ulixee_client;
use ulixee_client::{
	api, api::runtime_types::bounded_collections::bounded_vec::BoundedVec, try_until_connected,
	UlxClient, UlxConfig,
};
use ulx_primitives::{tick::Ticker, AccountId, NotaryId, NotebookDigest};

use crate::{
	stores::{
		blocks::BlocksStore,
		chain_transfer::ChainTransferStore,
		notebook_header::NotebookHeaderStore,
		notebook_status::{NotebookFinalizationStep, NotebookStatusStore},
		registered_key::RegisteredKeyStore,
	},
	Error,
};

pub async fn spawn_block_sync(
	rpc_url: String,
	notary_id: NotaryId,
	pool: &PgPool,
	ticker: Ticker,
) -> anyhow::Result<(), Error> {
	sync_finalized_blocks(rpc_url.clone(), notary_id, pool, ticker.clone())
		.await
		.map_err(|e| Error::BlockSyncError(e.to_string()))?;
	track_blocks(rpc_url.clone(), notary_id, pool, ticker.clone());

	Ok(())
}

pub(crate) fn track_blocks(
	rpc_url: String,
	notary_id: NotaryId,
	pool: &PgPool,
	ticker: Ticker,
) -> tokio::task::JoinHandle<()> {
	let pool = pool.clone();
	tokio::task::spawn(async move {
		let ticker = ticker.clone();
		loop {
			match subscribe_to_blocks(rpc_url.clone(), notary_id.clone(), &pool.clone(), &ticker)
				.await
			{
				Ok(_) => break,
				Err(e) => tracing::error!("Error polling mainchain blocks: {:?}", e),
			}
		}
	})
}

async fn sync_finalized_blocks(
	url: String,
	notary_id: NotaryId,
	pool: &PgPool,
	ticker: Ticker,
) -> anyhow::Result<()> {
	let client = try_until_connected(url, 2500, 120_000).await?;
	let notaries_query = api::storage().notaries().active_notaries();

	let active_notaries = client
		.storage()
		.at_latest()
		.await?
		.fetch(&notaries_query)
		.await?
		.unwrap_or(BoundedVec(vec![]));

	let Some(notary) = active_notaries.0.iter().find(|notary| notary.notary_id == notary_id) else {
		info!("NOTE: Notary {} is not active", notary_id);
		return Ok(());
	};
	let oldest_block_to_sync = notary.activated_block;

	let mut tx = pool.begin().await?;
	BlocksStore::lock(&mut tx).await?;

	let last_synched_block = BlocksStore::get_latest_finalized_block_number(&mut *tx).await?;

	let latest_finalized_hash = client.backend().latest_finalized_block_ref().await?;

	let mut block_hash = latest_finalized_hash;
	let mut missing_blocks: Vec<Block<UlxConfig, _>> = vec![];
	loop {
		let block = client.blocks().at(block_hash.clone()).await?;
		if block.number() <= last_synched_block || block.number() <= oldest_block_to_sync {
			break
		}
		block_hash = BlockRef::from(block.header().parent_hash);
		missing_blocks.insert(0, block);
	}

	info!(
		"Current synched finalized block: {}. Missing {}",
		last_synched_block,
		missing_blocks.len()
	);

	for block in missing_blocks.into_iter() {
		process_block(&mut *tx, &client, &block, notary_id).await?;
		process_finalized_block(&mut *tx, block, notary_id, &ticker).await?;
	}
	tx.commit().await?;
	Ok(())
}

async fn process_block(
	db: &mut PgConnection,
	client: &UlxClient,
	block: &Block<UlxConfig, UlxClient>,
	notary_id: NotaryId,
) -> anyhow::Result<()> {
	let next_vote_minimum = client
		.storage()
		.at(block.hash())
		.fetch(&api::storage().block_seal_spec().current_vote_minimum())
		.await?
		.unwrap_or_default();

	let notebooks_header = block.header().digest.logs.iter().find_map(|log| match log {
		DigestItem::PreRuntime(ulx_primitives::NOTEBOOKS_DIGEST_ID, data) =>
			NotebookDigest::decode(&mut &data[..]).ok(),
		_ => None,
	});

	let notebooks = notebooks_header
		.map(|digest| {
			digest
				.notebooks
				.into_iter()
				.filter(|notebook| notebook.notary_id == notary_id)
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();

	let parent_hash = block.header().parent_hash;
	BlocksStore::record(
		db,
		block.number(),
		block.hash(),
		parent_hash.clone(),
		next_vote_minimum,
		notebooks,
	)
	.await?;

	Ok(())
}

async fn find_missing_blocks(
	db: &mut PgConnection,
	client: &UlxClient,
	block_hash: H256,
) -> anyhow::Result<Vec<Block<UlxConfig, UlxClient>>> {
	let mut blocks = vec![];
	let mut block_hash = block_hash;
	while !BlocksStore::has_block(db, block_hash).await? {
		let block = client.blocks().at(block_hash.clone()).await?;
		let is_genesis = block.header().number == 0;

		block_hash = block.header().parent_hash.clone();
		blocks.insert(0, block);
		// can't get a parent of genesis block
		if is_genesis {
			break
		}
	}
	Ok(blocks)
}

async fn process_fork(
	db: &mut PgConnection,
	client: &UlxClient,
	block: &Block<UlxConfig, UlxClient>,
	notary_id: NotaryId,
) -> anyhow::Result<()> {
	info!("Processing fork {} ({})", block.hash(), block.number());

	let missing_blocks = find_missing_blocks(db, client, block.hash()).await?;
	for missing_block in missing_blocks {
		process_block(db, &client, &missing_block, notary_id).await?;
	}
	Ok(())
}

async fn process_finalized_block(
	db: &mut PgConnection,
	block: Block<UlxConfig, UlxClient>,
	notary_id: NotaryId,
	ticker: &Ticker,
) -> anyhow::Result<()> {
	info!("Processing finalized {} ({})", block.hash(), block.number());
	BlocksStore::record_finalized(db, block.hash()).await?;

	let block_height = block.number();

	let events = block.events().await?;
	for event in events.iter() {
		if let Ok(event) = event {
			if let Some(Ok(meta_change)) =
				event.as_event::<api::notaries::events::NotaryMetaUpdated>().transpose()
			{
				if meta_change.notary_id == notary_id {
					RegisteredKeyStore::store_public(
						&mut *db,
						ed25519::Public(meta_change.meta.public.0),
						block_height,
					)
					.await?;
				}
				continue
			}
			if let Some(Ok(activated_event)) =
				event.as_event::<api::notaries::events::NotaryActivated>().transpose()
			{
				info!("Notary activated: {:?}", activated_event);
				if activated_event.notary.notary_id == notary_id {
					RegisteredKeyStore::store_public(
						&mut *db,
						ed25519::Public(activated_event.notary.meta.public.0),
						block_height,
					)
					.await?;
					let tick = ticker.current();
					NotebookHeaderStore::create(
						&mut *db,
						notary_id,
						1,
						tick,
						ticker.time_for_tick(tick + 1),
					)
					.await?;
				}
				continue
			}

			if let Some(Ok(notebook)) =
				event.as_event::<api::notebook::events::NotebookSubmitted>().transpose()
			{
				if notebook.notary_id == notary_id {
					info!("Notebook finalized: {:?}", notebook);
					NotebookStatusStore::next_step(
						&mut *db,
						notebook.notebook_number,
						NotebookFinalizationStep::Closed,
					)
					.await?;
				}
				continue
			}

			if let Some(Ok(notebook)) =
				event.as_event::<api::notebook::events::NotebookAuditFailure>().transpose()
			{
				if notebook.notary_id == notary_id {
					panic!("Notebook audit failed! Need to shut down {:?}", notebook);
				}
				continue
			}

			if let Some(Ok(to_localchain)) = event
				.as_event::<api::chain_transfer::events::TransferToLocalchain>()
				.transpose()
			{
				if to_localchain.notary_id == notary_id {
					info!("Transfer to localchain: {:?}", to_localchain);
					ChainTransferStore::record_transfer_to_local_from_block(
						&mut *db,
						block_height,
						&AccountId::from(to_localchain.account_id.0),
						to_localchain.account_nonce,
						to_localchain.amount,
					)
					.await?;
				}
				continue
			}
		}
	}

	Ok(())
}

/// Loop through new block events until a client disconnects
async fn subscribe_to_blocks(
	url: String,
	notary_id: NotaryId,
	pool: &PgPool,
	ticker: &Ticker,
) -> anyhow::Result<bool> {
	let client = try_until_connected(url, 2500, 120_000).await?;
	let mut blocks_sub = client.blocks().subscribe_all().await?;
	let mut finalized_sub = client.blocks().subscribe_finalized().await?;

	let pool = pool.clone();
	loop {
		tokio::select! {biased;
			block_next = blocks_sub.next() => {
				match block_next {
					Some(Ok(block)) => {
						let mut tx = pool.begin().await?;
						process_fork(&mut *tx, &client, &block, notary_id).await?;
						tx.commit().await?;
					},
					Some(Err(e)) => {
						tracing::error!("Error polling best blocks: {:?}", e);
						return Err(e.into());
					},
					None => break
				}
			},
			block_next = finalized_sub.next() => {
				match block_next {
					Some(Ok(block)) => {
						let mut tx = pool.begin().await?;
						if !BlocksStore::has_block(&mut *tx, block.hash()).await? {
							process_block(&mut *tx, &client, &block, notary_id).await?;
						}
						process_finalized_block(&mut *tx, block, notary_id, &ticker).await?;
						tx.commit().await?;
					},
					Some(Err(e)) => {
						tracing::error!("Error polling finalized blocks: {:?}", e);
						return Err(e.into());
					},
					None => break
				}
			},
		}
	}

	Ok(true)
}
