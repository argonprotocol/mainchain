use codec::Decode;
use sp_core::{ed25519::Public as Ed25519Public, H256};
use sqlx::{PgConnection, PgPool};
use subxt::{
	blocks::{Block, BlockRef},
	config::substrate::DigestItem,
};
use tracing::info;

pub use argon_client;
use argon_client::{api, ArgonConfig, ArgonOnlineClient, MainchainClient};
use argon_primitives::{
	tick::{Tick, Ticker},
	AccountId, NotaryId, NotebookDigest, TickDigest,
};

use crate::{
	stores::{
		blocks::BlocksStore,
		chain_transfer::ChainTransferStore,
		mainchain_identity::MainchainIdentityStore,
		notebook_header::NotebookHeaderStore,
		notebook_status::{NotebookFinalizationStep, NotebookStatusStore},
		registered_key::RegisteredKeyStore,
	},
	Error,
};

pub async fn spawn_block_sync(
	rpc_url: String,
	notary_id: NotaryId,
	pool: PgPool,
	ticker: Ticker,
) -> anyhow::Result<(), Error> {
	sync_finalized_blocks(rpc_url.clone(), notary_id, pool.clone(), ticker)
		.await
		.map_err(|e| Error::BlockSyncError(e.to_string()))?;
	track_blocks(rpc_url.clone(), notary_id, pool.clone(), ticker);

	Ok(())
}

pub(crate) fn track_blocks(
	rpc_url: String,
	notary_id: NotaryId,
	pool: PgPool,
	ticker: Ticker,
) -> tokio::task::JoinHandle<()> {
	tokio::task::spawn(async move {
		let ticker = ticker;
		loop {
			match subscribe_to_blocks(rpc_url.clone(), notary_id, &pool.clone(), &ticker).await {
				Ok(_) => break,
				Err(e) => tracing::error!("Error polling mainchain blocks: {:?}", e),
			}
		}
	})
}

async fn sync_finalized_blocks(
	url: String,
	notary_id: NotaryId,
	pool: PgPool,
	ticker: Ticker,
) -> anyhow::Result<()> {
	let client = MainchainClient::try_until_connected(url.as_str(), 2500, 120_000).await?;
	let chain_identity = client.get_chain_identity().await?;

	{
		let mut db = pool.acquire().await?;
		MainchainIdentityStore::confirm_chain(&mut db, chain_identity).await?;
	}
	let notaries_query = api::storage().notaries().active_notaries();

	let active_notaries =
		client.fetch_storage(&notaries_query, None).await?.unwrap_or(vec![].into());

	let Some(notary) = active_notaries.0.iter().find(|notary| notary.notary_id == notary_id) else {
		info!("NOTE: Notary {} is not active", notary_id);
		return Ok(());
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
			&ticker,
		)
		.await
		.ok();
	}

	let mut tx = pool.begin().await?;
	BlocksStore::lock(&mut tx).await?;

	let last_synched_block = BlocksStore::get_latest_finalized_block_number(&mut tx).await?;

	let latest_finalized_hash = client.latest_finalized_block_hash().await?;

	let mut block_hash = latest_finalized_hash;
	let mut missing_blocks: Vec<Block<ArgonConfig, _>> = vec![];
	loop {
		let block = client.live.blocks().at(block_hash.clone()).await?;
		if block.number() <= last_synched_block || block.number() <= oldest_block_to_sync {
			break;
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
		process_block(&mut tx, &client.live, &block, notary_id).await?;
		process_finalized_block(&mut tx, block, notary_id, &ticker).await?;
	}
	tx.commit().await?;
	Ok(())
}

async fn process_block(
	db: &mut PgConnection,
	client: &ArgonOnlineClient,
	block: &Block<ArgonConfig, ArgonOnlineClient>,
	notary_id: NotaryId,
) -> anyhow::Result<()> {
	let next_vote_minimum = client
		.storage()
		.at(block.hash())
		.fetch(&api::storage().block_seal_spec().current_vote_minimum())
		.await?
		.unwrap_or_default();

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

	let parent_hash = block.header().parent_hash;
	BlocksStore::record(
		db,
		block.number(),
		block.hash(),
		parent_hash,
		next_vote_minimum,
		notebooks,
	)
	.await?;

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
	info!("Processing fork {} ({})", block.hash(), block.number());

	let missing_blocks = find_missing_blocks(db, client, block.hash()).await?;
	for missing_block in missing_blocks {
		process_block(db, client, &missing_block, notary_id).await?;
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
	info!("Processing finalized {} ({})", block.hash(), block.number());
	BlocksStore::record_finalized(db, block.hash()).await?;

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
		.map(|digest| digest.tick)
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
				activate_notebook_processing(&mut *db, notary_id, public, block_height, ticker)
					.await?;
			}
			continue;
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
			continue;
		}

		if let Some(Ok(notebook)) =
			event.as_event::<api::notebook::events::NotebookAuditFailure>().transpose()
		{
			if notebook.notary_id == notary_id {
				panic!("Notebook audit failed! Need to shut down {:?}", notebook);
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

/// Loop through new block events until a client disconnects
async fn subscribe_to_blocks(
	url: String,
	notary_id: NotaryId,
	pool: &PgPool,
	ticker: &Ticker,
) -> anyhow::Result<bool> {
	let mainchain_client =
		MainchainClient::try_until_connected(url.as_str(), 2500, 120_000).await?;
	let client = mainchain_client.live.clone();
	let mut blocks_sub = client.blocks().subscribe_all().await?;
	let mut finalized_sub = client.blocks().subscribe_finalized().await?;

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
						if !BlocksStore::has_block(&mut tx, block.hash()).await? {
							process_block(&mut tx, &client, &block, notary_id).await?;
						}
						process_finalized_block(&mut tx, block, notary_id, ticker).await?;
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
