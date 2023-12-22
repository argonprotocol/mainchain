use std::{future::Future, sync::Arc};

use codec::Encode;
use futures::FutureExt;
use sc_utils::notification::NotificationSender;
use sp_core::{
	bytes::{from_hex, to_hex},
	crypto::Ss58Codec,
	ed25519, H256,
};
use sp_keystore::KeystorePtr;
use sqlx::{Executor, PgConnection, PgPool};
use subxt::{
	backend::rpc::{RpcClient, RpcParams},
	rpc_params,
	utils::AccountId32,
};
use tokio::sync::RwLock;
use tracing::{info, warn};

use ulixee_client::{
	api,
	api::{
		runtime_types,
		runtime_types::{
			bounded_collections::bounded_vec::BoundedVec as SubxtBoundedVec,
			sp_core::ed25519::{Public, Signature},
		},
	},
	UlxClient,
};
use ulx_notary_primitives::{ensure, ChainTransfer, NotaryId, NotebookHeader, NotebookNumber};

use crate::{
	error::Error,
	stores::{
		block_meta::BlockMetaStore,
		blocks::BlocksStore,
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
	pub client: MainchainClient,

	pub completed_notebook_sender: NotificationSender<NotebookHeader>,
}

#[derive(Clone)]
pub struct MainchainClient {
	urls: Vec<String>,
	client: Arc<RwLock<Option<UlxClient>>>,
	current_index: usize,
}

impl MainchainClient {
	pub fn new(urls: Vec<String>) -> Self {
		Self { urls, client: Arc::new(RwLock::new(None)), current_index: 0 }
	}

	pub fn get(&mut self) -> BoxFutureResult<UlxClient> {
		async {
			{
				let lock = self.client.read().await;
				if let Some(client) = &*lock {
					return Ok(client.clone())
				}
			}

			self.current_index += 1;
			if self.current_index >= self.urls.len() {
				self.current_index = 0;
			}
			let url = self.urls[self.current_index].clone();

			let mut lock = self.client.write().await;
			let ulx_client = UlxClient::from_url(url).await?;
			*lock = Some(ulx_client.clone());
			drop(lock);

			Ok(ulx_client)
		}
		.boxed()
	}

	pub fn on_rpc_error(&mut self) {
		let mut lock = self.client.write().now_or_never().unwrap();
		*lock = None;
	}
}

pub fn spawn_notebook_closer(
	pool: PgPool,
	notary_id: NotaryId,
	client: MainchainClient,
	keystore: KeystorePtr,
	completed_notebook_sender: NotificationSender<NotebookHeader>,
) {
	tokio::spawn(async move {
		let mut notebook_closer =
			NotebookCloser { pool, notary_id, client, keystore, completed_notebook_sender };
		let _ = notebook_closer.create_task().await;
	});
}

impl NotebookCloser {
	pub fn create_task(
		&'_ mut self,
	) -> impl Future<Output = anyhow::Result<(), Error>> + Send + '_ {
		async move {
			loop {
				let _ = self.iterate_notebook_close_loop().await;
				// wait before resuming
				tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
			}
		}
	}

	pub(super) async fn iterate_notebook_close_loop(&mut self) {
		let _ = &self.try_rotate_notebook().await.map_err(|e| {
			tracing::error!("Error rotating notebooks: {:?}", e);
			e
		});
		let _ = &self.try_close_notebook().await.map_err(|e| {
			tracing::error!("Error closing open notebook: {:?}", e);
			e
		});

		let _ = &self.try_get_auditors().await.map_err(|e| {
			tracing::error!("Error getting notebook auditors. {:?}", e);
			e
		});

		let _ = &self.try_get_audit_signatures().await.map_err(|e| {
			tracing::error!("Error getting audit signatures: {:?}", e);
			e
		});

		let _ = &self.try_submit_notebook().await.map_err(|e| {
			tracing::error!("Error submitting notebook: {:?}", e);
			e
		});
	}

	pub(super) fn try_rotate_notebook(&self) -> BoxFutureResult<()> {
		async move {
			let mut tx = self.pool.begin().await?;
			// NOTE: must rotate existing first. The db has a constraint to only allow a single open
			// notebook at a time.
			let notebook_number = match NotebookStatusStore::step_up_expired_open(&mut *tx).await? {
				Some(notebook_number) => notebook_number,
				None => return Ok(()),
			};
			let meta = BlockMetaStore::load(&mut *tx).await?;
			let next_notebook = notebook_number + 1u32;
			NotebookHeaderStore::create(
				&mut *tx,
				self.notary_id,
				next_notebook,
				meta.best_block_number,
			)
			.await?;

			tx.commit().await?;
			Ok(())
		}
		.boxed()
	}
	pub(super) fn try_close_notebook<'a>(&'a mut self) -> BoxFutureResult<'a, ()> {
		async move {
			let mut tx = self.pool.begin().await?;
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

			let header = NotebookHeaderStore::load(&self.pool, notebook_number).await?;

			let _ =
				self.completed_notebook_sender
					.notify(|| Ok(header))
					.map_err(|e: anyhow::Error| {
						tracing::error!("Error sending completed notebook notification {:?}", e);
					});

			Ok(())
		}
		.boxed()
	}

	async fn get_runtime_finalized_block(client: &UlxClient) -> anyhow::Result<u32, Error> {
		let finalized_block_number = client
			.storage()
			.at_latest()
			.await?
			.fetch(&api::storage().localchain_relay().finalized_block_number())
			.await?
			.unwrap_or(0);
		Ok(finalized_block_number)
	}

	fn try_get_auditors<'a>(&'a mut self) -> BoxFutureResult<'a, ()> {
		async move {
			let notary_id = self.notary_id;
			let mut tx = self.pool.begin().await?;
			let step = NotebookFinalizationStep::Closed;

			let notebook_number = match NotebookStatusStore::lock_with_step(&mut *tx, step).await? {
				Some(notebook_number) => notebook_number,
				None => return Ok(()),
			};
			let pinned_block_number =
				NotebookHeaderStore::get_pinned_block_number(&mut *tx, notebook_number).await?;
			let client = self.client.get().await?;

			let auditors_query = api::runtime_apis::localchain_relay_apis::LocalchainRelayApis
				.get_auditors(notary_id, notebook_number, pinned_block_number);

			let auditors = client
				.runtime_api()
				.at_latest()
				.await
				.map_err(|e| {
					self.client.on_rpc_error();
					e
				})?
				.call(auditors_query)
				.await?;

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
		}
		.boxed()
	}

	fn get_audit_params<'a>(
		tx: &'a mut PgConnection,
		keystore: &'a KeystorePtr,
		notebook_number: NotebookNumber,
	) -> BoxFutureResult<'a, (RpcParams, u32)> {
		async move {
			let notebook = NotebookStore::load(&mut *tx, notebook_number).await?;
			let notary_public = RegisteredKeyStore::get_valid_public(
				&mut *tx,
				notebook.header.finalized_block_number,
			)
			.await?;

			let header_hash = notebook.header.hash();
			let version = notebook.header.version;
			let notary_id = notebook.header.notary_id;
			let encoded_notebook = notebook.encode();
			let notary_signature = notary_sign(keystore, &notary_public, &header_hash)?;
			let block = BlocksStore::get_hash(&mut *tx, notebook.header.finalized_block_number)
				.await?
				.ok_or(Error::InternalError("Unable to find block for notebook".to_string()))?;

			let block_hash_hex = to_hex(&block.block_hash, false);

			let params = rpc_params![
				block_hash_hex,
				version,
				notary_id,
				notebook_number.clone(),
				notary_signature.clone(),
				&header_hash,
				&encoded_notebook
			];
			Ok((params, notebook.header.finalized_block_number))
		}
		.boxed()
	}

	fn try_get_audit_signatures<'a>(&'a self) -> BoxFutureResult<'a, ()> {
		async move {
			let pool = &self.pool;
			let keystore = self.keystore.clone();
			let mut tx = pool.begin().await?;
			let step = NotebookFinalizationStep::GetAuditors;
			let notebook_number = match NotebookStatusStore::lock_with_step(&mut *tx, step).await? {
				Some(notebook_number) => notebook_number,
				None => return Ok(()),
			};

			let auditors = NotebookAuditorStore::get_auditors(&mut *tx, notebook_number).await?;
			let (params, finalized_block_needed) =
				Self::get_audit_params(&mut *tx, &keystore, notebook_number).await?;

			let results = futures::future::join_all(auditors.iter().map(|auditor| {
				Self::get_audit_signature(
					&pool,
					auditor.public,
					notebook_number,
					auditor.signature.clone(),
					auditor.rpc_urls.clone(),
					finalized_block_needed,
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
		}
		.boxed()
	}

	pub(super) fn get_audit_signature(
		pool: &PgPool,
		public: [u8; 32],
		notebook_number: NotebookNumber,
		existing_signature: Option<[u8; 64]>,
		rpc_urls: Vec<String>,
		runtime_finalized_block_needed: u32,
		params: RpcParams,
	) -> BoxFutureResult<([u8; 32], [u8; 64])> {
		async move {
			if existing_signature.is_some() {
				return Ok((public, existing_signature.unwrap()))
			}

			let mut db = pool.acquire().await?;
			NotebookAuditorStore::increment_attempts(&mut *db, notebook_number, &public).await?;

			let mut params = params.clone();
			params.push(ed25519::Public(public).to_ss58check())?;

			let peer_client = RpcClient::from_url(rpc_urls[0].clone()).await?;
			let ulx_client = UlxClient::from_rpc_client(peer_client.clone()).await?;
			let auditor_finalized_block = Self::get_runtime_finalized_block(&ulx_client).await?;

			ensure!( runtime_finalized_block_needed <= auditor_finalized_block, Error::InternalError(format!(
				"Peer runtime {} is not up to date yet. Need block {} vs auditor runtime finalized {}",
				rpc_urls[0].clone(),
				runtime_finalized_block_needed,
				auditor_finalized_block
			)));

			let signature = peer_client
				.request::<NotebookAuditResponse>(
					&*"notebook_audit",
					params,
				)
				.await
				.map_err(|e| {
					if e.to_string().contains("Invalid") {
						panic!("This notebook was rejected by an auditor. Need to shutdown until we can fix whatever is wrong.");
					}
					warn!("Error getting audit signature for notebook {} from (url={}): {}", notebook_number, rpc_urls[0].clone(), e);
					Error::InternalError(format!("Error getting audit signature: {}", e))
				})?;

			info!(
				"Got signature from auditor {:0x?} at {:?}: {:?}",
				&public,
				rpc_urls[0].clone(),
				&signature
			);

			let signature =
				from_hex(signature.signature.trim_start_matches("0x")).map_err(|e| {
					Error::InternalError(format!("Error decoding seal signature: {}", e))
				})?;

			let signature: [u8; 64] = signature.try_into().map_err(|e| {
				Error::InternalError(format!("Error decoding seal signature: {:?}", e))
			})?;

			NotebookAuditorStore::update_signature(&mut *db, notebook_number, &public, &signature)
				.await?;

			Ok((public, signature))
		}.boxed()
	}

	fn try_submit_notebook<'a>(&'a mut self) -> BoxFutureResult<'a, Option<NotebookHeader>> {
		async move {
			let pool = &self.pool;
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
			let header = NotebookHeaderStore::load(&mut *tx, notebook_number).await?;
			let public =
				RegisteredKeyStore::get_valid_public(&mut *tx, header.finalized_block_number)
					.await?;
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
					tax: header.tax,
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
									ChainTransfer::ToLocalchain { account_id, account_nonce } =>
										runtime_types::ulx_notary_primitives::notebook::ChainTransfer::ToLocalchain {
											account_id: AccountId32::from(Into::<[u8; 32]>::into(account_id.clone())),
											account_nonce: account_nonce.clone(),
										},
								}
							})
							.collect(),
					),
				},
				header_hash,
				auditors: SubxtBoundedVec(auditors),
			};

			let signature = notary_sign(&self.keystore, &public, &header_hash)?;

			let ext = api::tx()
				.localchain_relay()
				.submit_notebook(notebook.clone(), Signature(signature.0));

			let client = self.client.get().await?;
			let ext = client.tx().create_unsigned(&ext)?;
			let submission = ext.submit_and_watch().await.map_err(|e| {
				self.client.on_rpc_error();
				e
			})?;

			NotebookStatusStore::next_step(&mut *tx, notebook_number, step).await?;
			tx.commit().await?;

			let included_in_block =
				submission.wait_for_in_block().await?.wait_for_success().await?;
			info!("Submitted notebook {}. In block {:?}", notebook_number, included_in_block);

			Ok(Some(header))
		}
		.boxed()
	}
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct NotebookAuditResponse {
	pub signature: String,
}

#[cfg(test)]
mod tests {
	use std::{env, net::Ipv4Addr};

	use codec::Decode;
	use frame_support::assert_ok;
	use futures::StreamExt;
	use sp_core::{bounded_vec, ed25519::Public};
	use sp_keyring::Sr25519Keyring::Bob;
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sqlx::PgPool;
	use subxt::utils::AccountId32;
	use subxt_signer::sr25519::dev;

	use ulixee_client::{
		api::{
			runtime_apis::account_nonce_api::AccountNonceApi,
			runtime_types::{
				bounded_collections::bounded_vec::BoundedVec,
				pallet_notaries::pallet::Call as NotaryCall,
				sp_core::ed25519,
				ulx_node_runtime::RuntimeCall,
				ulx_notary_primitives::balance_change::AccountOrigin,
				ulx_primitives::{block_seal::Host, notary::NotaryMeta},
			},
			storage, tx,
		},
		UlxClient,
	};
	use ulx_notary_primitives::{AccountType::Deposit, BalanceChange, Note, Notebook};
	use ulx_testing::{test_context, test_context_from_url};

	use crate::{
		block_watch::track_blocks,
		notebook_closer::NOTARY_KEYID,
		server::NotebookHeaderStream,
		stores::{
			balance_change::BalanceChangeStore, block_meta::BlockMetaStore,
			chain_transfer::ChainTransferStore, notebook_status::NotebookStatusStore,
		},
	};

	use super::*;

	#[sqlx::test]
	#[should_panic]
	async fn test_bad_notebook_audit(pool: PgPool) {
		let _ = tracing_subscriber::fmt::try_init();
		let ctx = test_context().await;
		let genesis = ctx.client.backend().genesis_hash().await.expect("Get genesis hash");
		let keystore = MemoryKeystore::new();
		let keystore = KeystoreExt::new(keystore);
		let notary_key =
			keystore.ed25519_generate_new(NOTARY_KEYID, None).expect("should have a key");

		BlockMetaStore::start(&pool, genesis).await.expect("Start block meta");

		{
			let mut tx = pool.begin().await.expect("Begin tx");
			RegisteredKeyStore::store_public(&mut *tx, notary_key, 0)
				.await
				.expect("Store key");
			NotebookHeaderStore::create(&mut *tx, 1, 1, 0).await.expect("Create notebook");
			tx.commit().await.expect("Commit");
		}
		ChainTransferStore::record_transfer_to_local_from_block(
			&pool,
			0,
			&Bob.to_account_id(),
			1,
			1000,
		)
		.await
		.expect("Record transfer");
		ChainTransferStore::take_and_record_transfer_local(
			&mut *pool.acquire().await.expect("should get db"),
			1,
			&Bob.to_account_id(),
			1,
			1000,
			1,
			0,
			100,
		)
		.await
		.expect("Take transfer");

		sqlx::query("update notebook_status set open_time = now() - interval '2 minutes' where notebook_number = 1")
			.execute(&pool)
			.await.expect("Update notebook status");

		let notary_id = 1;
		let (completed_notebook_sender, _) = NotebookHeaderStream::channel();

		let mut closer = NotebookCloser {
			pool: pool.clone(),
			keystore: keystore.clone(),
			notary_id,
			client: MainchainClient::new(vec![ctx.ws_url.clone()]),
			completed_notebook_sender,
		};
		assert_ok!(&closer.try_rotate_notebook().await);

		assert_ok!(&closer.try_close_notebook().await);

		assert_ok!(&closer.try_get_auditors().await);
		let mut db = pool.acquire().await.expect("should get db");
		let auditors = NotebookAuditorStore::get_auditors(&mut *db, 1)
			.await
			.expect("should get auditors");

		let notebook = NotebookStore::load(&mut *db, 1).await.expect("should load notebook");
		assert_eq!(notebook.header.chain_transfers.len(), 1);
		let bytes = notebook.encode();
		assert_eq!(Notebook::decode(&mut bytes.as_slice()).expect("should decode"), notebook);

		let (params, finalized_block_needed) =
			NotebookCloser::get_audit_params(&mut *db, &closer.keystore, 1)
				.await
				.expect("should get params");

		assert!(NotebookCloser::get_audit_signature(
			&pool,
			auditors[0].public,
			1,
			None,
			vec![ctx.ws_url.clone()],
			finalized_block_needed,
			params
		)
		.await
		.unwrap_err()
		.to_string()
		.contains("InvalidNotarySignature"));
	}

	#[sqlx::test]
	async fn test_notebook_cycle(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let use_live = env::var("USE_LIVE").unwrap_or(String::from("false")).parse::<bool>()?;
		let ctx = if use_live {
			test_context_from_url("ws://localhost:9944").await
		} else {
			test_context().await
		};
		let genesis = ctx.client.backend().genesis_hash().await.expect("Get genesis hash");
		BlockMetaStore::start(&pool, genesis).await?;
		let bob_id = dev::bob().public_key().to_account_id();

		let ws_url = ctx.ws_url.clone();
		let keystore = MemoryKeystore::new();
		let keystore = KeystoreExt::new(keystore);
		let notary_key =
			keystore.ed25519_generate_new(NOTARY_KEYID, None).expect("should have a key");

		track_blocks(ctx.ws_url, 1, &pool.clone());

		propose_bob_as_notary(&ctx.client, notary_key).await?;

		activate_notary(&pool, &ctx.client, &bob_id).await?;

		{
			let mut tx = pool.begin().await?;
			assert_eq!(
				NotebookStatusStore::lock_open_for_appending(&mut *tx).await?,
				1,
				"There should be a notebook active now"
			);
		}

		let bob_nonce = get_bob_nonce(&ctx.client, bob_id).await?;

		// Submit a transfer to localchain and wait for result
		create_localchain_transfer(&pool, &ctx.client, bob_nonce).await?;

		// Record the balance change
		submit_balance_change_to_notary(&pool, bob_nonce).await?;

		sqlx::query("update notebook_status set open_time = now() - interval '2 minutes' where notebook_number = 1")
			.execute(&pool)
			.await?;

		let (completed_notebook_sender, completed_notebook_stream) =
			NotebookHeaderStream::channel();

		let mut closer = NotebookCloser {
			pool: pool.clone(),
			keystore: keystore.clone(),
			notary_id: 1,
			client: MainchainClient::new(vec![ws_url.clone()]),
			completed_notebook_sender,
		};

		let mut subscription = completed_notebook_stream.subscribe(100);

		loop {
			let _ = &closer.iterate_notebook_close_loop().await;
			let status = NotebookStatusStore::get(&pool, 1).await?;
			if status.finalized_time.is_some() {
				break
			}
			// yield
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		let next_header = subscription.next().await;
		assert_eq!(next_header.is_some(), true);
		let next_header = next_header.expect("Should have a header");

		assert_eq!(
			ctx.client
				.storage()
				.at_latest()
				.await?
				.fetch(
					&storage().localchain_relay().account_origin_last_changed_notebook_by_notary(
						1,
						AccountOrigin { notebook_number: 1, account_uid: 1 }
					)
				)
				.await?,
			Some(1),
			"Should have updated Bob's last change notebook"
		);

		assert_eq!(
			ctx.client
				.storage()
				.at_latest()
				.await?
				.fetch(&storage().localchain_relay().notebook_changed_accounts_root_by_notary(1, 1))
				.await?,
			next_header.changed_accounts_root.into(),
			"Should have updated Bob's last change notebook"
		);

		assert_eq!(
			ctx.client
				.storage()
				.at_latest()
				.await?
				.fetch(&storage().localchain_relay().last_notebook_number_by_notary(1,))
				.await?,
			Some((1, next_header.pinned_to_block_number)),
			"Should have updated the last notebook number"
		);

		Ok(())
	}

	async fn propose_bob_as_notary(client: &UlxClient, notary_key: Public) -> anyhow::Result<()> {
		let notary_proposal = tx().notaries().propose(NotaryMeta {
			hosts: BoundedVec(vec![Host {
				is_secure: false,
				ip: Ipv4Addr::LOCALHOST.into(),
				port: 0u16,
			}]),
			public: ed25519::Public(notary_key.0),
		});

		let result = client
			.tx()
			.sign_and_submit_then_watch_default(&notary_proposal, &dev::bob())
			.await?
			.wait_for_in_block()
			.await?
			.wait_for_success()
			.await;

		assert_ok!(result);
		Ok(())
	}

	async fn submit_balance_change_to_notary(pool: &PgPool, bob_nonce: u32) -> anyhow::Result<()> {
		let result = BalanceChangeStore::apply_balance_changes(
			&pool,
			1,
			vec![BalanceChange {
				account_id: Bob.to_account_id(),
				account_type: Deposit,
				change_number: 1,
				balance: 1000,
				previous_balance_proof: None,
				notes: bounded_vec![Note::create(
					1000,
					ulx_notary_primitives::NoteType::ClaimFromMainchain {
						account_nonce: bob_nonce
					},
				)],
				channel_hold_note: None,
				signature: sp_core::ed25519::Signature([0u8; 64]).into(),
			}
			.sign(Bob.pair())
			.clone()],
		)
		.await?;
		assert_eq!(result.notebook_number, 1);

		Ok(())
	}

	async fn get_bob_nonce(client: &UlxClient, bob_id: AccountId32) -> anyhow::Result<u32> {
		let bob_nonce = client
			.runtime_api()
			.at_latest()
			.await?
			.call(AccountNonceApi.account_nonce(bob_id.clone()))
			.await?;
		Ok(bob_nonce)
	}

	async fn create_localchain_transfer(
		pool: &PgPool,
		client: &UlxClient,
		bob_nonce: u32,
	) -> anyhow::Result<()> {
		client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().localchain_relay().send_to_localchain(1000u32.into(), 1, bob_nonce),
				&dev::bob(),
			)
			.await?
			.wait_for_finalized_success()
			.await?;

		let mut found = false;
		for _ in 0..5 {
			let row = sqlx::query!("select * from chain_transfers").fetch_optional(pool).await?;
			if let Some(_) = row {
				found = true;
				break
			}
			// wait for 500 ms
			tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		}
		assert_eq!(found, true, "Should have recorded a chain transfer");

		Ok(())
	}

	async fn activate_notary(
		pool: &PgPool,
		client: &UlxClient,
		bob_id: &AccountId32,
	) -> anyhow::Result<()> {
		let notary_activated_finalized_block = client
			.tx()
			.sign_and_submit_then_watch_default(
				&tx().sudo().sudo(RuntimeCall::Notaries(NotaryCall::activate {
					operator_account: bob_id.clone(),
				})),
				&dev::alice(),
			)
			.await?
			.wait_for_finalized_success()
			.await;

		assert_ok!(&notary_activated_finalized_block);
		let notary_activated_finalized_block = notary_activated_finalized_block.unwrap();

		let mut found = false;
		for _ in 0..5 {
			let meta = sqlx::query!(
				"select * from blocks where block_hash=$1",
				&notary_activated_finalized_block.block_hash().0
			)
			.fetch_optional(pool)
			.await?;
			if let Some(_) = meta {
				found = true;
				break
			}
			// wait for 500 ms
			tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		}
		assert_eq!(found, true, "Should have found the finalized block");
		Ok(())
	}
}
