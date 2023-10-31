use sp_core::ed25519;
use sqlx::{PgConnection, PgPool};
use subxt::{
	blocks::{Block, BlockRef},
	config::Header,
};
use tracing::info;

use crate::Error;
pub use ulixee_client;
use ulixee_client::{
	api, api::runtime_types::bounded_collections::bounded_vec::BoundedVec, try_until_connected,
	UlxClient, UlxConfig,
};
use ulx_notary_primitives::{AccountId, NotaryId};

use crate::stores::{
	block_meta::BlockMetaStore,
	blocks::BlocksStore,
	chain_transfer::ChainTransferStore,
	notebook_header::NotebookHeaderStore,
	notebook_status::{NotebookFinalizationStep, NotebookStatusStore},
	registered_key::RegisteredKeyStore,
};

pub async fn spawn_block_sync(
	rpc_url: String,
	notary_id: NotaryId,
	pool: &PgPool,
) -> anyhow::Result<(), Error> {
	sync_blocks(rpc_url.clone(), notary_id, pool)
		.await
		.map_err(|e| Error::BlockSyncError(e.to_string()))?;
	track_blocks(rpc_url.clone(), notary_id, pool);

	Ok(())
}

pub(crate) fn track_blocks(rpc_url: String, notary_id: NotaryId, pool: &PgPool) {
	let pool = pool.clone();
	tokio::task::spawn(async move {
		loop {
			match subscribe_to_blocks(rpc_url.clone(), notary_id.clone(), &pool.clone()).await {
				Ok(_) => break,
				Err(e) => tracing::error!("Error polling mainchain blocks: {:?}", e),
			}
		}
	});
}

async fn sync_blocks(url: String, notary_id: NotaryId, pool: &PgPool) -> anyhow::Result<()> {
	let client = try_until_connected(url, 2500).await?;
	let notaries_query = api::storage().notaries().active_notaries();

	let active_notaries = client
		.storage()
		.at_latest()
		.await?
		.fetch(&notaries_query)
		.await?
		.unwrap_or(BoundedVec(vec![]));

	let notary = active_notaries
		.0
		.iter()
		.find(|notary| notary.notary_id == notary_id)
		.ok_or(anyhow::anyhow!("Notary not found"))?;
	let oldest_block_to_sync = notary.activated_block;

	let mut tx = pool.begin().await?;
	BlockMetaStore::lock(&mut tx).await?;

	let last_synched_block = BlocksStore::get_latest_finalized(&mut *tx).await?;
	let last_synched_block = match last_synched_block {
		Some(block) => block.block_number as u32,
		None => 0u32,
	};
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

	for block in missing_blocks.into_iter() {
		process_block(&mut *tx, block, notary_id).await?;
	}
	tx.commit().await?;
	Ok(())
}

async fn process_block(
	db: &mut PgConnection,
	block: Block<UlxConfig, UlxClient>,
	notary_id: NotaryId,
) -> anyhow::Result<()> {
	BlocksStore::record_finalized(db, block.number(), block.hash(), block.header().parent_hash)
		.await?;

	let block_height = block.header().number;
	let block_hash = block.header().hash();

	BlockMetaStore::store_finalized_block(&mut *db, block_height, block_hash.into()).await?;

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
					NotebookHeaderStore::create(
						&mut *db,
						notary_id,
						1,
						activated_event.notary.activated_block,
					)
					.await?;
				}
				continue
			}

			if let Some(Ok(notebook)) =
				event.as_event::<api::localchain_relay::events::NotebookSubmitted>().transpose()
			{
				if notebook.notary_id == notary_id {
					info!("Notebook finalized: {:?}", notebook);
					NotebookStatusStore::next_step(
						&mut *db,
						notebook.notebook_number,
						NotebookFinalizationStep::Submitted,
					)
					.await?;
				}
				continue
			}

			if let Some(Ok(to_localchain)) = event
				.as_event::<api::localchain_relay::events::TransferToLocalchain>()
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
) -> anyhow::Result<bool> {
	let client = try_until_connected(url, 2500).await?;
	let mut blocks_sub = client.blocks().subscribe_best().await?;
	let mut finalized_sub = client.blocks().subscribe_finalized().await?;

	let pool = pool.clone();
	loop {
		tokio::select! {biased;
			block_next = blocks_sub.next() => {
				match block_next {
					Some(Ok(block)) => {
						let mut db = pool.acquire().await?;
						BlockMetaStore::store_best_block(&mut db, block.number(), *block.hash().as_fixed_bytes()).await?;
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
						process_block(&mut *tx, block, notary_id).await?;
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
