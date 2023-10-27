use std::future::Future;

use codec::Encode;
use sc_utils::notification::NotificationSender;
use sp_core::{bytes::from_hex, ed25519, H256};
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
			bounded_collections::bounded_vec::BoundedVec as SubxtBoundedVec,
			sp_core::ed25519::{Public, Signature},
		},
	},
	try_until_connected,
};
use ulx_notary_primitives::{ChainTransfer, NotaryId, NotebookHeader, NotebookNumber};

use crate::{
	error::Error,
	stores::{
		block_meta::BlockMetaStore,
		notebook::NotebookStore,
		notebook_auditors::NotebookAuditorStore,
		notebook_header::NotebookHeaderStore,
		notebook_status::{NotebookFinalizationStep, NotebookStatusStore},
		registered_key::RegisteredKeyStore,
		BoxFutureResult,
	},
};

pub const NOTARY_KEYID: sp_core::crypto::KeyTypeId = sp_core::crypto::KeyTypeId(*b"unot");
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
	pub fn spawn_task(&self) -> tokio::task::JoinHandle<anyhow::Result<(), Error>> {
		tokio::spawn(self.create_task())
	}

	fn create_task(&self) -> impl Future<Output = anyhow::Result<(), Error>> + Send + 'static {
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
				let _ = try_close_notebook(&pool, &notification_sender).await.map_err(|e| {
					tracing::error!("Error closing open notebook: {:?}", e);
					e
				});

				let _ = try_get_auditors(&pool, notary_id, &url).await.map_err(|e| {
					tracing::error!("Error getting notebook auditors. {:?}", e);
					e
				});

				let _ = try_get_audit_signatures(&pool, &keystore).await.map_err(|e| {
					tracing::error!("Error getting audit signatures: {:?}", e);
					e
				});

				let _ = try_submit_notebook(&pool, &keystore, &url).await.map_err(|e| {
					tracing::error!("Error getting audit signatures: {:?}", e);
					e
				});

				// wait before resuming
				tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
			}
		}
	}
}

pub(crate) fn try_rotate_notebook(pool: &PgPool, notary_id: NotaryId) -> BoxFutureResult<()> {
	Box::pin(async move {
		let mut tx = pool.begin().await?;
		// NOTE: must rotate existing first. The db has a constraint to only allow a single open
		// notebook at a time.
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
pub(crate) fn try_close_notebook<'a>(
	pool: &'a PgPool,
	notification_sender: &'a NotificationSender<NotebookHeader>,
) -> BoxFutureResult<'a, ()> {
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

		let header = NotebookHeaderStore::load(pool, notebook_number).await?;

		let _ = notification_sender.notify(|| Ok(header)).map_err(|e: anyhow::Error| {
			tracing::error!("Error sending completed notebook notification {:?}", e);
		});

		Ok(())
	})
}

fn try_get_auditors<'a>(
	pool: &'a PgPool,
	notary_id: NotaryId,
	url: &'a str,
) -> BoxFutureResult<'a, ()> {
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
		let auditors_query = api::runtime_apis::localchain_relay_apis::LocalchainRelayApis
			.get_auditors(notary_id, notebook_number, pinned_block_number);

		let auditors = client.runtime_api().at_latest().await?.call(auditors_query).await?;

		for (idx, (_, public, hosts)) in auditors.iter().enumerate() {
			NotebookAuditorStore::insert(
				&mut *tx,
				notebook_number,
				&public.0 .0,
				idx as u16,
				hosts,
			)
			.await?;
		}

		NotebookStatusStore::next_step(&mut *tx, notebook_number, step).await?;
		tx.commit().await?;
		Ok(())
	})
}

fn try_get_audit_signatures<'a>(
	pool: &'a PgPool,
	keystore: &'a KeystorePtr,
) -> BoxFutureResult<'a, ()> {
	Box::pin(async move {
		let mut tx = pool.begin().await?;
		let step = NotebookFinalizationStep::GetAuditors;
		let notebook_number = match NotebookStatusStore::lock_with_step(&mut *tx, step).await? {
			Some(notebook_number) => notebook_number,
			None => return Ok(()),
		};

		let auditors = NotebookAuditorStore::get_auditors(&mut *tx, notebook_number).await?;
		let meta = BlockMetaStore::load(&mut *tx).await?;
		let notebook = NotebookStore::load(&mut *tx, notebook_number).await?;
		let notary_public =
			RegisteredKeyStore::get_valid_public(&mut *tx, notebook.header.finalized_block_number)
				.await?;
		let header_hash = notebook.header.hash();
		let version = notebook.header.version;
		let notary_id = notebook.header.notary_id;
		let notebook = notebook.encode();
		let notary_signature = notary_sign(keystore, &notary_public, &header_hash)?;

		let params = rpc_params![
			meta.finalized_block_hash,
			version,
			notary_id,
			notebook_number.clone(),
			notary_signature.clone(),
			header_hash,
			notebook.encode()
		];

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
	notebook_number: NotebookNumber,
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

		let signature = from_hex(signature.signature.trim_start_matches("0x"))
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
					return Some((Public(a.public), Signature(sig)))
				}
				None
			})
			.collect();
		let client = try_until_connected(rpc_url.to_string(), 2500).await?;
		let header = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;
		let public =
			RegisteredKeyStore::get_valid_public(&mut *tx, header.finalized_block_number).await?;
		let header_hash = header.hash();

		let notebook = runtime_types::ulx_notary_primitives::notebook::AuditedNotebook {
			header: runtime_types::ulx_notary_primitives::notebook::NotebookHeader {
				version: header.version,
				finalized_block_number: header.finalized_block_number,
				notebook_number: header.notebook_number,
				pinned_to_block_number: header.pinned_to_block_number,
				notary_id: header.notary_id,
				start_time: header.start_time,
				end_time: header.end_time,
				changed_accounts_root: header.changed_accounts_root,
				changed_account_origins: SubxtBoundedVec(
					header
						.changed_account_origins
						.iter()
						.map(|a| {
							runtime_types::ulx_notary_primitives::balance_change::AccountOrigin {
								notebook_number: a.notebook_number,
								account_uid: a.account_uid,
							}
						})
						.collect(),
				),
				chain_transfers: SubxtBoundedVec(
					header
						.chain_transfers
						.iter()
						.map(|t| {
							match t {
							ChainTransfer::ToMainchain { account_id, amount } =>
								runtime_types::ulx_notary_primitives::notebook::ChainTransfer::ToMainchain {
									account_id: AccountId32::from(Into::<[u8; 32]>::into(account_id.clone())),
									amount: amount.clone(),
								},
							ChainTransfer::ToLocalchain { account_id, nonce } =>
								runtime_types::ulx_notary_primitives::notebook::ChainTransfer::ToLocalchain {
									account_id: AccountId32::from(Into::<[u8; 32]>::into(account_id.clone())),
									nonce: nonce.clone(),
								},
						}
						})
						.collect(),
				),
			},
			header_hash,
			auditors: SubxtBoundedVec(auditors),
		};

		let signature = notary_sign(keystore, &public, &header_hash)?;

		let ext = api::tx().localchain_relay().submit_notebook(
			header_hash,
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

fn notary_sign(
	keystore: &KeystorePtr,
	public: &ed25519::Public,
	hash: &H256,
) -> anyhow::Result<ed25519::Signature, Error> {
	let sig = keystore
		.ed25519_sign(NOTARY_KEYID, &public, &hash[..])
		.map_err(|e| {
			Error::InternalError(format!(
				"Unable to sign notebook header for submission to mainchain {}",
				e
			))
		})?
		.unwrap();
	Ok(sig)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct NotebookAuditResponse {
	pub signature: String,
}
