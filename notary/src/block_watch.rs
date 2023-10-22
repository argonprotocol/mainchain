use sp_core::{crypto::AccountId32, ed25519};
use sqlx::{PgConnection, PgPool};
use subxt::{
	blocks::{Block, BlockRef},
	config::Header,
};

pub use ulixee_client;
use ulixee_client::{
	api, api::runtime_types::bounded_collections::bounded_vec::BoundedVec, try_until_connected,
	UlxClient, UlxConfig,
};
use ulx_notary_primitives::NotaryId;

use crate::stores::{
	block_meta::BlockMetaStore,
	blocks::BlocksStore,
	chain_transfer::ChainTransferStore,
	notebook_header::NotebookHeaderStore,
	notebook_status::{NotebookFinalizationStep, NotebookStatusStore},
	registered_key::RegisteredKeyStore,
};

pub fn track_blocks(rpc_url: String, notary_id: NotaryId, pool: &PgPool) {
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

pub async fn sync_blocks(url: String, notary_id: NotaryId, pool: &PgPool) -> anyhow::Result<()> {
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
	for meta_change in events.find::<api::notaries::events::NotaryMetaUpdated>() {
		if let Ok(meta_change) = meta_change {
			if meta_change.notary_id == notary_id {
				RegisteredKeyStore::store_public(
					&mut *db,
					ed25519::Public(meta_change.meta.public.0),
					block_height,
				)
				.await?;
			}
		}
	}

	for notebook in events.find::<api::notaries::events::NotaryActivated>() {
		if let Ok(notebook) = notebook {
			if notebook.notary.notary_id == notary_id {
				if NotebookHeaderStore::create(
					&mut *db,
					notary_id,
					1,
					notebook.notary.activated_block,
				)
				.await
				.is_err()
				{
					continue
				}
			}
		}
	}

	for notebook in events.find::<api::localchain_relay::events::NotebookSubmitted>() {
		if let Ok(notebook) = notebook {
			if notebook.notary_id == notary_id {
				NotebookStatusStore::next_step(
					&mut *db,
					notebook.notebook_number,
					NotebookFinalizationStep::Submitted,
				)
				.await?;
			}
		}
	}

	for to_localchain in events.find::<api::localchain_relay::events::TransferToLocalchain>() {
		if let Ok(to_localchain) = to_localchain {
			if to_localchain.notary_id == notary_id {
				ChainTransferStore::record_transfer_to_local_from_block(
					&mut *db,
					block_height,
					&AccountId32::from(to_localchain.account_id.0),
					to_localchain.nonce,
					to_localchain.amount,
				)
				.await?;
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
						let mut db = pool.acquire().await?;
						process_block(&mut *db, block, notary_id).await?;
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
