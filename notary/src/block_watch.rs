use codec::Decode;
use sp_core::{H256, ed25519::Public as Ed25519Public};
use sqlx::{PgConnection, PgPool};
use std::time::Duration;
use subxt::{
	blocks::{Block, BlockRef},
	config::substrate::DigestItem,
};
use tracing::{error, info, trace, warn};

pub use argon_client;
use argon_client::{ArgonConfig, ArgonOnlineClient, FetchAt, MainchainClient, api};
use argon_primitives::{
	NotebookDigest,
	prelude::*,
	tick::{TickDigest, Ticker},
};

use crate::stores::{
	blocks::BlocksStore,
	chain_transfer::ChainTransferStore,
	mainchain_identity::MainchainIdentityStore,
	notebook_audit_failure::NotebookAuditFailureStore,
	notebook_header::NotebookHeaderStore,
	notebook_status::{NotebookFinalizationStep, NotebookStatusStore},
	registered_key::RegisteredKeyStore,
};

pub async fn spawn_block_sync(
	rpc_url: String,
	notary_id: NotaryId,
	pool: PgPool,
	ticker: Ticker,
	reconnect_delay: Duration,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
	let notary_activated_block =
		get_notary_activation(rpc_url.clone(), notary_id, pool.clone(), &ticker).await?;

	let handle = tokio::task::spawn(async move {
		let ticker = ticker;
		loop {
			// this loop is to restart with a new client if the previous one fails
			match subscribe_to_blocks(&rpc_url, notary_id, notary_activated_block, &pool, &ticker)
				.await
			{
				// if it returns, it means the client disconnected
				Ok(_) => info!("Waiting 5 seconds to restart block watch thread"),
				Err(e) => error!("Error polling mainchain blocks: {:?}", e),
			}
			warn!("Block watch thread restarting after {:?}", reconnect_delay);
			tokio::time::sleep(reconnect_delay).await;
		}
	});
	Ok(handle)
}

async fn get_notary_activation(
	rpc_url: String,
	notary_id: NotaryId,
	pool: PgPool,
	ticker: &Ticker,
) -> anyhow::Result<BlockNumber> {
	let client = MainchainClient::try_until_connected(rpc_url.as_str(), 2500, 120_000).await?;
	let chain_identity = client.get_chain_identity().await?;

	{
		let mut db = pool.acquire().await?;
		MainchainIdentityStore::confirm_chain(&mut db, chain_identity).await?;
	}
	let notaries_query = api::storage().notaries().active_notaries();

	let active_notaries = client
		.fetch_storage(&notaries_query, FetchAt::Finalized)
		.await?
		.unwrap_or(vec![].into());

	let Some(notary) = active_notaries.0.iter().find(|notary| notary.notary_id == notary_id) else {
		info!("NOTE: Notary {} is not active", notary_id);
		return Ok(0);
	};
	let oldest_block_to_sync = notary.activated_block;
	{
		let mut db = pool.acquire().await?;

		let public = Ed25519Public::from_raw(notary.meta.public);
		let _ = activate_notebook_processing(
			&mut db,
			notary_id,
			public,
			notary.meta_updated_tick,
			ticker,
		)
		.await
		.ok();
	}
	Ok(oldest_block_to_sync)
}

async fn process_block(
	db: &mut PgConnection,
	block: &Block<ArgonConfig, ArgonOnlineClient>,
	notary_id: NotaryId,
) -> anyhow::Result<()> {
	info!("Processing block {} ({})", block.hash(), block.number());
	if BlocksStore::has_block(&mut *db, block.hash()).await? {
		info!("Duplicated block {} ({})", block.hash(), block.number());
		return Ok(());
	}

	let notebooks_header = block.header().digest.logs.iter().find_map(|log| match log {
		DigestItem::PreRuntime(argon_primitives::NOTEBOOKS_DIGEST_ID, data) =>
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

	let events = block.events().await?;
	for event in events.iter().flatten() {
		if let Some(Ok(notebook)) =
			event.as_event::<api::notebook::events::NotebookAuditFailure>().transpose()
		{
			if notebook.notary_id == notary_id {
				error!("Notebook audit failure: {:?}", notebook);
				NotebookAuditFailureStore::record(
					db,
					notebook.notebook_number,
					notebook.notebook_hash.into(),
					format!("{:?}", notebook.first_failure_reason),
					block.number(),
				)
				.await?;
				break;
			}
		}
	}

	let parent_hash = block.header().parent_hash;
	BlocksStore::record(db, block.number(), block.hash(), parent_hash, notebooks).await?;

	Ok(())
}

async fn find_missing_blocks(
	db: &mut PgConnection,
	client: &ArgonOnlineClient,
	block_hash: H256,
) -> anyhow::Result<Vec<Block<ArgonConfig, ArgonOnlineClient>>> {
	let mut blocks = vec![];
	let mut block_hash = block_hash;
	while !BlocksStore::has_block(db, block_hash).await? {
		let block = client.blocks().at(block_hash).await?;
		let is_genesis = block.header().number == 0;

		block_hash = block.header().parent_hash;
		blocks.insert(0, block);
		// can't get a parent of genesis block
		if is_genesis {
			break;
		}
	}
	Ok(blocks)
}

async fn process_fork(
	db: &mut PgConnection,
	client: &ArgonOnlineClient,
	block: &Block<ArgonConfig, ArgonOnlineClient>,
	notary_id: NotaryId,
) -> anyhow::Result<()> {
	trace!("Processing fork {} ({})", block.hash(), block.number());

	let missing_blocks = find_missing_blocks(db, client, block.hash()).await?;
	for missing_block in missing_blocks {
		process_block(db, &missing_block, notary_id).await?;
	}
	Ok(())
}

async fn activate_notebook_processing(
	db: &mut PgConnection,
	notary_id: NotaryId,
	public: Ed25519Public,
	active_at_tick: Tick,
	ticker: &Ticker,
) -> anyhow::Result<()> {
	// it might already be stored
	let _ = RegisteredKeyStore::store_public(&mut *db, public, active_at_tick).await.ok();
	let mut tick = ticker.current();
	if tick <= active_at_tick {
		tick = active_at_tick + 1;
	}
	NotebookHeaderStore::create(&mut *db, notary_id, 1, tick, ticker.time_for_tick(tick + 1))
		.await?;
	Ok(())
}

async fn process_finalized_block(
	db: &mut PgConnection,
	block: Block<ArgonConfig, ArgonOnlineClient>,
	notary_id: NotaryId,
	ticker: &Ticker,
) -> anyhow::Result<()> {
	if BlocksStore::record_finalized(db, block.hash()).await.is_err() {
		return Ok(());
	}
	trace!("Processing finalized {} ({})", block.hash(), block.number());

	let block_height = block.number();
	let tick = block
		.header()
		.digest
		.logs
		.iter()
		.find_map(|log| match log {
			DigestItem::PreRuntime(argon_primitives::TICK_DIGEST_ID, data) =>
				TickDigest::decode(&mut &data[..]).ok(),
			_ => None,
		})
		.map(|digest| digest.0)
		.unwrap_or(ticker.current());

	let events = block.events().await?;
	for event in events.iter().flatten() {
		if let Some(Ok(meta_change)) =
			event.as_event::<api::notaries::events::NotaryMetaUpdated>().transpose()
		{
			if meta_change.notary_id == notary_id {
				RegisteredKeyStore::store_public(
					&mut *db,
					Ed25519Public::from_raw(meta_change.meta.public),
					tick,
				)
				.await?;
			}
			continue;
		}
		if let Some(Ok(activated_event)) =
			event.as_event::<api::notaries::events::NotaryActivated>().transpose()
		{
			info!("Notary activated: {:?}", activated_event);
			if activated_event.notary.notary_id == notary_id {
				let public = Ed25519Public::from_raw(activated_event.notary.meta.public);
				activate_notebook_processing(&mut *db, notary_id, public, tick, ticker).await?;
			}
			continue;
		}

		if let Some(Ok(notebook)) =
			event.as_event::<api::notebook::events::NotebookSubmitted>().transpose()
		{
			if notebook.notary_id == notary_id {
				info!("Notebook finalized: {:?}", notebook);
				if let Err(e) = NotebookStatusStore::next_step(
					&mut *db,
					notebook.notebook_number,
					NotebookFinalizationStep::Closed,
				)
				.await
				{
					let status =
						NotebookStatusStore::get(&mut *db, notebook.notebook_number).await?;
					if status.step == NotebookFinalizationStep::Finalized {
						trace!("Notebook already finalized: {:?}", notebook);
					} else {
						error!("Error finalizing notebook: {:?} {:?}", notebook, e);
					}
				}
			}
			continue;
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
					to_localchain.expiration_tick,
					&AccountId::from(to_localchain.account_id.0),
					to_localchain.transfer_id,
					to_localchain.amount,
				)
				.await?;
			}
			continue;
		}
	}

	Ok(())
}

async fn get_finalized_block_path(
	client: &MainchainClient,
	oldest_finalized_block_number: BlockNumber,
) -> anyhow::Result<Vec<Block<ArgonConfig, ArgonOnlineClient>>> {
	let latest_finalized_hash = client.latest_finalized_block_hash().await?;

	let mut block_path = vec![];
	let mut block_hash = latest_finalized_hash;
	loop {
		let block = client.live.blocks().at(block_hash.clone()).await?;
		if block.number() <= oldest_finalized_block_number {
			break;
		}
		block_hash = BlockRef::from(block.header().parent_hash);
		block_path.insert(0, block);
	}

	Ok(block_path)
}

/// Loop through new block events until a client disconnects
async fn subscribe_to_blocks(
	rpc_url: &str,
	notary_id: NotaryId,
	notary_activated_block: BlockNumber,
	pool: &PgPool,
	ticker: &Ticker,
) -> anyhow::Result<bool> {
	let mainchain_client = MainchainClient::try_until_connected(rpc_url, 2500, 120_000).await?;
	let client = mainchain_client.live.clone();
	let mut blocks_sub = client.blocks().subscribe_all().await?;
	let mut finalized_sub = client.blocks().subscribe_finalized().await?;

	{
		let mut tx = pool.begin().await?;
		BlocksStore::lock(&mut tx).await?;

		let last_synched_block = BlocksStore::get_latest_finalized_block_number(&mut tx).await?;
		let oldest_block_needed = notary_activated_block.max(last_synched_block);
		let missing_blocks =
			get_finalized_block_path(&mainchain_client, oldest_block_needed).await?;

		info!(
			"Current synched finalized block: {}. Missing {}",
			last_synched_block,
			missing_blocks.len()
		);

		for block in missing_blocks.into_iter() {
			process_block(&mut tx, &block, notary_id).await?;
			process_finalized_block(&mut tx, block, notary_id, ticker).await?;
		}
		tx.commit().await?;
	}

	let pool = pool.clone();
	loop {
		tokio::select! {biased;
			block_next = blocks_sub.next() => {
				match block_next {
					Some(Ok(block)) => {
						let mut tx = pool.begin().await?;
						process_fork(&mut tx, &client, &block, notary_id).await?;
						tx.commit().await?;
					},
					Some(Err(e)) => {
						error!("Error polling best blocks: {:?}", e);
						return Err(e.into());
					},
					None => break
				}
			},
			block_next = finalized_sub.next() => {
				match block_next {
					Some(Ok(block)) => {
						let mut tx = pool.begin().await?;
						process_block(&mut tx, &block, notary_id).await?;
						process_finalized_block(&mut tx, block, notary_id, ticker).await?;
						tx.commit().await?;
					},
					Some(Err(e)) => {
						error!("Error polling finalized blocks: {:?}", e);
						return Err(e.into());
					},
					None => break
				}
			},
		}
	}

	Ok(true)
}

#[cfg(test)]
mod tests {
	use super::*;
	use argon_testing::start_argon_test_node;
	use serial_test::serial;
	use sqlx::PgPool;
	use subxt::config::Header;

	#[sqlx::test]
	#[serial]
	async fn test_handles_duplicate_blocks(pool: PgPool) -> anyhow::Result<()> {
		let node = start_argon_test_node().await;
		let _ = tracing_subscriber::fmt::try_init();

		let ticker = node.client.lookup_ticker().await?;
		spawn_block_sync(
			node.client.url.clone(),
			1,
			pool.clone(),
			ticker,
			Duration::from_millis(100),
		)
		.await
		.expect("Failed to setup block sync");

		let mut finalized_sub = node.client.live.blocks().subscribe_finalized().await?;
		let mut finalized = node.client.latest_finalized_block_hash().await?;
		for i in 0..5 {
			if i == 2 {
				finalized = node.client.latest_finalized_block_hash().await?;
			}
			finalized_sub.next().await;
		}
		let finalized_hash = finalized.hash();
		let finalized_block = node.client.live.blocks().at(finalized).await?;
		let mut tx = pool.begin().await?;
		assert!(BlocksStore::has_block(&mut tx, finalized_hash).await?);
		assert!(BlocksStore::record_finalized(&mut tx, finalized_hash).await.is_err());
		tx.commit().await?;

		let mut db = pool.acquire().await?;
		assert!(BlocksStore::has_block(&mut db, finalized_hash).await?);
		process_block(&mut db, &finalized_block, 1)
			.await
			.expect("should not fail with a duplicate");
		assert!(process_finalized_block(&mut db, finalized_block, 1, &ticker).await.is_ok());

		Ok(())
	}

	#[sqlx::test]
	#[serial]
	async fn can_survive_a_reboot(pool: PgPool) -> anyhow::Result<()> {
		let mut node = start_argon_test_node().await;
		let _ = tracing_subscriber::fmt::try_init();

		let ticker = node.client.lookup_ticker().await?;
		let _handle = spawn_block_sync(
			node.client.url.clone(),
			1,
			pool.clone(),
			ticker,
			Duration::from_millis(100),
		)
		.await
		.expect("Failed to setup block sync");

		{
			let mut finalized_sub = node.client.live.blocks().subscribe_finalized().await?;
			for _ in 0..2 {
				finalized_sub.next().await;
			}
		}

		node.restart(Duration::from_secs(1)).await.expect("should restart");

		let mut last_finalized_hash = node.client.latest_finalized_block_hash().await?.hash();
		let mut finalized_sub = node.client.live.blocks().subscribe_finalized().await?;
		for _ in 0..2 {
			let next = finalized_sub.next().await.expect("should get")?;
			last_finalized_hash = next.hash();
		}

		tokio::time::sleep(Duration::from_millis(200)).await;
		let mut tx = pool.begin().await?;
		let mut parent = last_finalized_hash;
		loop {
			assert!(BlocksStore::has_block(&mut tx, parent).await?);
			let header = node
				.client
				.live
				.backend()
				.block_header(parent)
				.await?
				.expect("should have a header");
			if header.number() == 0 {
				break;
			}
			parent = header.parent_hash
		}

		Ok(())
	}
}
