use codec::Decode;
use sp_core::{ed25519, H256};
use sqlx::{PgConnection, PgPool};
use subxt::{
	blocks::{Block, BlockRef},
	config::substrate::DigestItem::{Consensus, PreRuntime},
};
use tracing::info;

pub use ulixee_client;
use ulixee_client::{
	api, api::runtime_types::bounded_collections::bounded_vec::BoundedVec, try_until_connected,
	UlxClient, UlxConfig,
};
use ulx_notary_primitives::{
	AccountId, BlockVoteDigest, BlockVoteEligibility, NotaryId, NotebookNumber,
	BLOCK_VOTES_DIGEST_ID, NEXT_VOTE_ELIGIBILITY_DIGEST_ID,
};

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
) -> anyhow::Result<(), Error> {
	sync_finalized_blocks(rpc_url.clone(), notary_id, pool)
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

async fn sync_finalized_blocks(
	url: String,
	notary_id: NotaryId,
	pool: &PgPool,
) -> anyhow::Result<()> {
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

	for block in missing_blocks.into_iter() {
		process_block(&mut *tx, &block, notary_id).await?;
		process_finalized_block(&mut *tx, block, notary_id).await?;
	}
	tx.commit().await?;
	Ok(())
}

async fn process_block(
	db: &mut PgConnection,
	block: &Block<UlxConfig, UlxClient>,
	notary_id: NotaryId,
) -> anyhow::Result<()> {
	let mut next_vote_eligibility: Option<BlockVoteEligibility> = None;
	let mut included_notebook_number: Option<NotebookNumber> = None;
	let mut parent_voting_key: Option<H256> = None;

	for log in block.header().digest.logs.iter() {
		match log {
			PreRuntime(BLOCK_VOTES_DIGEST_ID, data) => {
				let Some(votes_digest) = BlockVoteDigest::decode(&mut &data[..]).ok() else {
					return Err(anyhow::anyhow!("Unable to decode votes digest"))
				};
				parent_voting_key = votes_digest.parent_voting_key;
				for notebook in votes_digest.notebook_numbers.iter() {
					if notebook.notary_id == notary_id {
						included_notebook_number = Some(notebook.notebook_number);
					}
				}
			},
			Consensus(NEXT_VOTE_ELIGIBILITY_DIGEST_ID, data) => {
				let Some(votes_digest) = BlockVoteEligibility::decode(&mut &data[..]).ok() else {
					return Err(anyhow::anyhow!("Unable to decode votes eligibility"))
				};
				next_vote_eligibility = Some(votes_digest);
			},

			_ => (),
		}
	}

	let parent_hash = block.header().parent_hash;
	BlocksStore::record(
		db,
		block.number(),
		block.hash(),
		parent_hash.clone(),
		next_vote_eligibility.ok_or_else(|| anyhow::anyhow!("Missing next work digest"))?,
		parent_voting_key.expect("Missing parent voting key"),
		included_notebook_number,
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
	let missing_blocks = find_missing_blocks(db, client, block.hash()).await?;
	for missing_block in missing_blocks {
		process_block(db, &missing_block, notary_id).await?;
	}
	process_block(db, &block, notary_id).await?;
	Ok(())
}

async fn process_finalized_block(
	db: &mut PgConnection,
	block: Block<UlxConfig, UlxClient>,
	notary_id: NotaryId,
) -> anyhow::Result<()> {
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
				event.as_event::<api::notebook::events::NotebookSubmitted>().transpose()
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
) -> anyhow::Result<bool> {
	let client = try_until_connected(url, 2500).await?;
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
						process_finalized_block(&mut *tx, block, notary_id).await?;
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
