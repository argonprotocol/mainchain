use std::future::Future;

use codec::Encode;
use sc_utils::notification::NotificationSender;
use sp_core::{blake2_256, bytes::from_hex, H256};
use sp_keystore::KeystorePtr;
use sqlx::{Executor, PgPool};
use subxt::{
	backend::rpc::{RpcClient, RpcParams},
	rpc_params,
	utils::AccountId32,
};

use ulixee_client::{
	api,
	api::{
		runtime_types,
		runtime_types::{
			bounded_collections::bounded_vec::BoundedVec,
			sp_core::ed25519::{Public, Signature},
			ulx_primitives::block_seal,
		},
	},
	signature_messages::to_notebook_post_hash,
	try_until_connected,
};
use ulx_notary_primitives::{ChainTransfer, NotaryId, NotebookHeader};

use crate::{
	error::Error,
	notary::NOTARY_KEYID,
	stores::{
		balance_change::BalanceChangeStore,
		block_meta::BlockMetaStore,
		notebook::NotebookStore,
		notebook_auditors::NotebookAuditorStore,
		notebook_header::NotebookHeaderStore,
		notebook_status::{NotebookFinalizationStep, NotebookStatusStore},
		registered_key::RegisteredKeyStore,
		BoxFutureResult,
	},
};

pub const MIN_NOTEBOOK_AUDITORS_PCT: f32 = 0.8f32;

#[derive(Clone)]
pub struct NotebookCloser {
	pub pool: PgPool,
	pub keystore: KeystorePtr,
	pub notary_id: NotaryId,
	pub rpc_url: String,

	pub completed_notebook_sender: NotificationSender<NotebookHeader>,
}

impl NotebookCloser {
	pub fn create_task(&self) -> impl Future<Output = anyhow::Result<(), Error>> + Send + 'static {
		let pool = self.pool.clone();
		let notary_id = self.notary_id;
		let url = self.rpc_url.clone();
		let keystore = self.keystore.clone();
		let notification_sender = self.completed_notebook_sender.clone();
		async move {
			loop {
				let _ = try_rotate_notebook(&pool, notary_id).await.map_err(|e| {
					tracing::error!("Error rotating notebooks: {:?}", e);
					e
				});
				let _ = try_close_notebook(&pool).await.map_err(|e| {
					tracing::error!("Error closing open notebook: {:?}", e);
					e
				});

				let _ = try_get_auditors(&pool, &url).await.map_err(|e| {
					tracing::error!("Error getting notebook auditors. {:?}", e);
					e
				});

				let _ = try_get_audit_signatures(&pool).await.map_err(|e| {
					tracing::error!("Error getting audit signatures: {:?}", e);
					e
				});

				let header = try_submit_notebook(&pool, &keystore, &url).await.map_err(|e| {
					tracing::error!("Error getting audit signatures: {:?}", e);
					e
				});

				if let Ok(Some(header)) = header {
					let _ =
						notification_sender.notify(|| Ok(header)).map_err(|e: anyhow::Error| {
							tracing::error!(
								"Error sending completed notebook notification {:?}",
								e
							);
						});
				}

				// wait before resuming
				tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
			}
		}
	}
}

fn try_rotate_notebook(pool: &PgPool, notary_id: NotaryId) -> BoxFutureResult<()> {
	Box::pin(async move {
		let mut tx = pool.begin().await?;
		let notebook_number = match NotebookStatusStore::step_up_expired_open(&mut *tx).await? {
			Some(notebook_number) => notebook_number,
			None => return Ok(()),
		};
		let meta = BlockMetaStore::load(&mut *tx).await?;
		let next_notebook = notebook_number + 1u32;
		NotebookHeaderStore::create(&mut *tx, notary_id, next_notebook, meta.best_block_number)
			.await?;

		tx.commit().await?;
		Ok(())
	})
}
fn try_close_notebook(pool: &PgPool) -> BoxFutureResult<()> {
	Box::pin(async move {
		let mut tx = pool.begin().await?;
		let step = NotebookFinalizationStep::ReadyForClose;
		let notebook_number = match NotebookStatusStore::lock_with_step(&mut *tx, step).await? {
			Some(notebook_number) => notebook_number,
			None => return Ok(()),
		};

		tx.execute("SET statement_timeout = 5000").await?;
		NotebookStatusStore::lock_to_stop_appends(&mut *tx, notebook_number).await?;

		NotebookStore::close_notebook(&mut *tx, notebook_number).await?;

		NotebookStatusStore::next_step(&mut *tx, notebook_number, step).await?;
		tx.commit().await?;
		Ok(())
	})
}

fn try_get_auditors<'a>(pool: &'a PgPool, url: &'a str) -> BoxFutureResult<'a, ()> {
	Box::pin(async move {
		let mut tx = pool.begin().await?;
		let step = NotebookFinalizationStep::Closed;
		let notebook_number = match NotebookStatusStore::lock_with_step(&mut *tx, step).await? {
			Some(notebook_number) => notebook_number,
			None => return Ok(()),
		};

		let pinned_block_number =
			NotebookHeaderStore::get_pinned_block_number(&mut *tx, notebook_number).await?;

		let client = try_until_connected(url.to_string(), 2500).await?;
		let query = api::storage().system().block_hash(pinned_block_number);
		let block_hash = client.storage().at_latest().await?.fetch(&query).await?.ok_or(
			Error::BlockSyncError(format!(
				"Block at height {pinned_block_number} could not be retrieved",
			)),
		)?;

		let block_sealers = api::storage()
			.block_seal()
			.historical_block_seal_authorities(pinned_block_number);

		let sealers = client.storage().at_latest().await?.fetch(&block_sealers).await?.ok_or(
			Error::MainchainApiError(format!(
				"Unable to retrieve the block sealers for block {pinned_block_number}",
			)),
		)?;

		for (idx, (index, public)) in sealers.0.iter().enumerate() {
			let miner_query = api::storage().mining_slots().active_miners_by_index(*index as u32);
			let miner = client.storage().at(block_hash).fetch(&miner_query).await?.ok_or(
				Error::MainchainApiError(format!(
					"Unable to retrieve the miners for block {pinned_block_number}",
				)),
			)?;
			NotebookAuditorStore::insert(
				&mut *tx,
				notebook_number,
				&public.0 .0,
				idx as u16,
				&miner.rpc_hosts.0,
			)
			.await?;
		}

		NotebookStatusStore::next_step(&mut *tx, notebook_number, step).await?;
		tx.commit().await?;
		Ok(())
	})
}

fn try_get_audit_signatures(pool: &PgPool) -> BoxFutureResult<()> {
	Box::pin(async move {
		let mut tx = pool.begin().await?;
		let step = NotebookFinalizationStep::GetAuditors;
		let notebook_number = match NotebookStatusStore::lock_with_step(&mut *tx, step).await? {
			Some(notebook_number) => notebook_number,
			None => return Ok(()),
		};

		let auditors = NotebookAuditorStore::get_auditors(&mut *tx, notebook_number).await?;
		let header = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;
		let changes = BalanceChangeStore::get_for_notebook(&mut *tx, notebook_number).await?;
		let params = rpc_params![header.clone(), changes.clone()];

		let results = futures::future::join_all(auditors.iter().map(|auditor| {
			get_audit_signature(
				&pool,
				auditor.public,
				notebook_number,
				auditor.signature.clone(),
				auditor.rpc_urls.clone(),
				params.clone(),
			)
		}))
		.await;

		let count_successful = results.iter().filter(|r| r.is_ok()).count();
		if count_successful as f32 >= auditors.len() as f32 * MIN_NOTEBOOK_AUDITORS_PCT {
			NotebookStatusStore::next_step(&mut *tx, notebook_number, step).await?;
		}

		tx.commit().await?;
		Ok(())
	})
}

fn get_audit_signature(
	pool: &PgPool,
	public: [u8; 32],
	notebook_number: u32,
	existing_signature: Option<[u8; 64]>,
	rpc_urls: Vec<String>,
	params: RpcParams,
) -> BoxFutureResult<([u8; 32], [u8; 64])> {
	Box::pin(async move {
		if existing_signature.is_some() {
			return Ok((public, existing_signature.unwrap()))
		}

		let mut db = pool.acquire().await?;
		NotebookAuditorStore::increment_attempts(&mut *db, notebook_number, &public).await?;

		let mut params = params.clone();
		params.push(public.clone())?;
		let peer_client = RpcClient::from_url(rpc_urls[0].clone()).await?;
		let signature = peer_client
			.request::<NotebookAuditResponse>(
				&*"notebook_audit",
				params,
			)
			.await
			.map_err(|e| {
				if e.to_string().contains("Rejected") {
					panic!("This notebook was rejected by an auditor. Need to shutdown until we can fix whatever is wrong.");
				}
				Error::InternalError(format!("Error getting audit signature: {}", e))
			})?;

		let signature = if signature.signature.starts_with("0x") {
			&signature.signature[2..]
		} else {
			&signature.signature
		};
		let signature = from_hex(signature)
			.map_err(|e| Error::InternalError(format!("Error decoding seal signature: {}", e)))?;

		let signature: [u8; 64] = signature
			.try_into()
			.map_err(|e| Error::InternalError(format!("Error decoding seal signature: {:?}", e)))?;

		NotebookAuditorStore::update_signature(&mut *db, notebook_number, &public, &signature)
			.await?;

		Ok((public, signature))
	})
}

fn try_submit_notebook<'a>(
	pool: &'a PgPool,
	keystore: &'a KeystorePtr,
	rpc_url: &'a str,
) -> BoxFutureResult<'a, Option<NotebookHeader>> {
	Box::pin(async move {
		let mut tx = pool.begin().await?;
		let step = NotebookFinalizationStep::Audited;
		let notebook_number = match NotebookStatusStore::lock_with_step(&mut *tx, step).await? {
			Some(notebook_number) => notebook_number,
			None => return Ok(None),
		};

		let auditors = NotebookAuditorStore::get_auditors(&mut *tx, notebook_number).await?;
		let auditors = auditors
			.into_iter()
			.filter_map(|a| {
				if let Some(sig) = a.signature {
					return Some((
						block_seal::app::Public(Public(a.public)),
						block_seal::app::Signature(Signature(sig)),
					))
				}
				None
			})
			.collect();
		let client = try_until_connected(rpc_url.to_string(), 2500).await?;
		let header = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;
		let public =
			RegisteredKeyStore::get_valid_public(&mut *tx, header.finalized_block_number).await?;

		let notebook = runtime_types::ulx_primitives::notebook::Notebook {
			notebook_number: header.notebook_number,
			pinned_to_block_number: header.pinned_to_block_number,
			notary_id: header.notary_id,
			auditors: BoundedVec(auditors),
			transfers: BoundedVec(
				header
					.chain_transfers
					.clone()
					.into_iter()
					.map(|t| match t {
						ChainTransfer::ToMainchain { account_id, amount } =>
							runtime_types::ulx_primitives::notebook::ChainTransfer::ToMainchain {
								account_id: AccountId32::from(Into::<[u8; 32]>::into(account_id)),
								amount,
							},
						ChainTransfer::ToLocalchain { account_id, nonce } =>
							runtime_types::ulx_primitives::notebook::ChainTransfer::ToLocalchain {
								account_id: AccountId32::from(Into::<[u8; 32]>::into(account_id)),
								nonce,
							},
					})
					.collect(),
			),
		};

		let notebook_hash = H256(to_notebook_post_hash(&notebook).using_encoded(blake2_256));
		let signature = keystore
			.ed25519_sign(NOTARY_KEYID, &public, &notebook_hash[..])
			.map_err(|e| {
				Error::InternalError(format!(
					"Unable to sign notebook header for submission to mainchain {}",
					e
				))
			})?
			.unwrap();

		let ext = api::tx().localchain_relay().submit_notebook(
			notebook_hash.clone(),
			notebook.clone(),
			Signature(signature.0),
		);

		let ext = client.tx().create_unsigned(&ext)?;
		ext.submit().await?;

		NotebookStatusStore::next_step(&mut *tx, notebook_number, step).await?;
		tx.commit().await?;

		Ok(Some(header))
	})
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NotebookAuditResponse {
	pub signature: String,
}
