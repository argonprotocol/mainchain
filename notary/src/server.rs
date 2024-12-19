use crate::{
	middleware::{register_prometheus_metrics, MiddlewareLayer},
	notary_metrics::NotaryMetrics,
	stores::{
		balance_tip::BalanceTipStore,
		notarizations::NotarizationsStore,
		notebook::NotebookStore,
		notebook_audit_failure::{
			AuditFailureListener, AuditFailureStream, NotebookAuditFailureStore,
		},
		notebook_header::NotebookHeaderStore,
	},
	Error,
};
use argon_notary_apis::{
	get_header_url, get_notebook_url,
	localchain::{BalanceChangeResult, BalanceTipResult, LocalchainRpcServer},
	notebook::{NotebookRpcServer, NotebookSubscriptionBroadcast},
	system::SystemRpcServer,
};
use argon_primitives::{
	tick::Ticker, AccountId, AccountOrigin, AccountType, BalanceProof, BalanceTip, Notarization,
	NotarizationBalanceChangeset, NotarizationBlockVotes, NotarizationDomains, NotaryId,
	NotebookMeta, NotebookNumber, SignedNotebookHeader,
};
use futures::{Stream, StreamExt};
use jsonrpsee::{
	core::{async_trait, SubscriptionResult},
	server::{
		middleware::{http::ProxyGetRequestLayer, rpc::either::Either},
		BatchRequestConfig, PendingSubscriptionSink, PingConfig, RpcServiceBuilder, Server,
		ServerBuilder, ServerHandle, SubscriptionMessage,
	},
	types::ErrorObjectOwned,
	RpcModule, TrySendError,
};
use prometheus::Registry;
use sc_utils::notification::{NotificationSender, NotificationStream, TracingKeyStr};
use serde::Serialize;
use sp_core::H256;
use sqlx::{pool::PoolConnection, PgPool, Postgres};
use std::{net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};
use tokio::{net::ToSocketAddrs, sync::Mutex, task::JoinHandle};
use tower::layer::util::{Identity, Stack};
use tower_http::cors::{Any, CorsLayer};

pub type NotebookHeaderInfo = (SignedNotebookHeader, H256);
pub type NotebookHeaderStream = NotificationStream<NotebookHeaderInfo, NotebookHeaderTracingKey>;

#[derive(Clone)]
pub struct NotebookHeaderTracingKey;
impl TracingKeyStr for NotebookHeaderTracingKey {
	const TRACING_KEY: &'static str = "mpsc_notebook_header_notification_stream";
}

#[derive(Clone)]
pub struct NotaryServer {
	pub addr: SocketAddr,
	notary_id: NotaryId,
	operator_account_id: AccountId,
	pool: PgPool,
	ticker: Ticker,
	pub audit_failure_stream: AuditFailureStream,
	pub(crate) completed_notebook_stream: NotebookHeaderStream,
	pub completed_notebook_sender: NotificationSender<NotebookHeaderInfo>,
	audit_failure_number: Arc<Mutex<Option<NotebookNumber>>>,
	server_handle: Option<ServerHandle>,
	audit_handle: Arc<JoinHandle<()>>,

	latest_metadata: Arc<Mutex<NotebookMeta>>,
	archive_settings: ArchiveSettings,
	pub notary_metrics: Arc<NotaryMetrics>,
}

impl Drop for NotaryServer {
	fn drop(&mut self) {
		self.audit_handle.abort();
		if let Some(server) = self.server_handle.clone() {
			server.stop().expect("Should be able to stop server");
		}
	}
}

#[derive(Clone, PartialEq, Eq)]
pub struct ArchiveSettings {
	/// The archive server that this notary guarantees uploads to
	pub archive_host: String,
}

pub struct RpcConfig {
	pub max_payload_in_mb: u32,
	pub max_payload_out_mb: u32,
	pub max_connections: u32,
	pub max_subscriptions_per_connection: u32,
	pub max_buffer_capacity_per_connection: u32,
	pub batch_config: Option<BatchRequestConfig>,
	pub rate_limit_per_minute: NonZeroU32,
	pub prometheus_port: Option<u16>,
}

impl Default for RpcConfig {
	fn default() -> Self {
		Self {
			max_payload_in_mb: 1,
			max_payload_out_mb: 10,
			max_connections: 10000,
			max_subscriptions_per_connection: 10,
			max_buffer_capacity_per_connection: 64,
			batch_config: Some(BatchRequestConfig::Limit(100)),
			rate_limit_per_minute: NonZeroU32::new(500).unwrap(),
			prometheus_port: Some(9116),
		}
	}
}
const MEGABYTE: u32 = 1024 * 1024;

type NotaryServerT = Server<
	Stack<CorsLayer, Stack<ProxyGetRequestLayer, Identity>>,
	Stack<Either<MiddlewareLayer, Identity>, Identity>,
>;

impl NotaryServer {
	pub async fn create_http_server(
		addrs: impl ToSocketAddrs,
		rpc_config: RpcConfig,
		registry: Registry,
	) -> anyhow::Result<NotaryServerT> {
		let metrics = register_prometheus_metrics(registry, &rpc_config)?;
		let RpcConfig {
			max_payload_in_mb,
			max_payload_out_mb,
			max_connections,
			max_subscriptions_per_connection,
			max_buffer_capacity_per_connection,
			batch_config,
			..
		} = rpc_config;
		let rpc_middleware = RpcServiceBuilder::new().option_layer(metrics);
		let cors = CorsLayer::new().allow_methods(Any).allow_origin(Any).allow_headers(Any);
		let http_middleware = tower::ServiceBuilder::new()
			.layer(ProxyGetRequestLayer::new("/health", "system_health")?)
			.layer(cors);

		let server = ServerBuilder::default()
			.max_request_body_size(max_payload_in_mb.saturating_mul(MEGABYTE))
			.max_response_body_size(max_payload_out_mb.saturating_mul(MEGABYTE))
			.max_connections(max_connections)
			.max_subscriptions_per_connection(max_subscriptions_per_connection)
			.enable_ws_ping(
				PingConfig::new()
					.ping_interval(Duration::from_secs(30))
					.inactive_limit(Duration::from_secs(60))
					.max_failures(3),
			)
			.set_http_middleware(http_middleware)
			.set_message_buffer_capacity(max_buffer_capacity_per_connection)
			.set_batch_request_config(batch_config.unwrap_or(BatchRequestConfig::Disabled))
			.set_rpc_middleware(rpc_middleware)
			.enable_ws_ping(PingConfig::default())
			.build(addrs)
			.await?;
		Ok(server)
	}

	pub async fn stop(&mut self) {
		if let Some(server_handle) = self.server_handle.take() {
			server_handle.stop().expect("Should be able to stop server");
			server_handle.stopped().await;
		}
	}

	pub async fn start_with(
		server: NotaryServerT,
		notary_id: NotaryId,
		operator_account_id: AccountId,
		archive_settings: ArchiveSettings,
		ticker: Ticker,
		pool: PgPool,
		notary_metrics: Arc<NotaryMetrics>,
	) -> anyhow::Result<Self> {
		let (completed_notebook_sender, completed_notebook_stream) =
			NotebookHeaderStream::channel();
		let (audit_failure_sender, audit_failure_stream) = AuditFailureStream::channel();

		let (audit_failure_number, audit_handle) =
			Self::listen_for_audit_failure(&pool, audit_failure_sender).await?;
		let addr = server.local_addr()?;
		let latest_metadata = Arc::new(Mutex::new(NotebookHeaderStore::latest(&pool).await?));
		Self::listen_for_latest_notebook(
			completed_notebook_stream.clone(),
			latest_metadata.clone(),
		);
		let mut notary_server = Self {
			notary_id,
			ticker,
			completed_notebook_sender,
			completed_notebook_stream,
			operator_account_id,
			pool,
			addr,
			server_handle: None,
			audit_failure_stream,
			audit_failure_number,
			audit_handle: Arc::new(audit_handle),
			latest_metadata,
			archive_settings,
			notary_metrics,
		};

		let mut module = RpcModule::new(());
		module.merge(NotebookRpcServer::into_rpc(notary_server.clone()))?;
		module.merge(LocalchainRpcServer::into_rpc(notary_server.clone()))?;
		module.merge(SystemRpcServer::into_rpc(notary_server.clone()))?;

		let handle = server.start(module);
		notary_server.server_handle = Some(handle.clone());

		Ok(notary_server)
	}

	pub async fn wait_for_close(&self) {
		if let Some(handle) = self.server_handle.clone() {
			handle.stopped().await;
		}
	}

	#[allow(clippy::too_many_arguments)]
	pub async fn start(
		notary_id: NotaryId,
		operator_account_id: AccountId,
		pool: PgPool,
		archive_settings: ArchiveSettings,
		rpc_config: RpcConfig,
		ticker: Ticker,
		addrs: impl ToSocketAddrs,
		registry: Registry,
	) -> anyhow::Result<Self> {
		let server = Self::create_http_server(addrs, rpc_config, registry.clone()).await?;
		let notary_metrics = Arc::new(NotaryMetrics::new(&registry)?);
		Self::start_with(
			server,
			notary_id,
			operator_account_id,
			archive_settings,
			ticker,
			pool,
			notary_metrics,
		)
		.await
	}

	fn listen_for_latest_notebook(
		completed_notebook_stream: NotebookHeaderStream,
		latest_metadata: Arc<Mutex<NotebookMeta>>,
	) {
		let mut subscription = completed_notebook_stream.subscribe(10);
		tokio::spawn(async move {
			while let Some((header, _hash)) = subscription.next().await {
				let mut latest = latest_metadata.lock().await;
				if header.header.notebook_number > latest.finalized_notebook_number {
					*latest = NotebookMeta {
						finalized_notebook_number: header.header.notebook_number,
						finalized_tick: header.header.tick,
					}
				}
			}
		});
	}

	async fn listen_for_audit_failure(
		pool: &PgPool,
		audit_failure_sender: NotificationSender<NotebookNumber>,
	) -> Result<(Arc<Mutex<Option<NotebookNumber>>>, JoinHandle<()>), Error> {
		let mut audit_failure_listener =
			AuditFailureListener::connect(pool.clone(), audit_failure_sender)
				.await
				.map_err(|e| {
					Error::InternalError(format!(
						"An error occurred creating a Notebook Audit Failure listener {}",
						e
					))
				})?;
		let audit_failure_number: Arc<Mutex<Option<NotebookNumber>>> = Default::default();
		if let Some(is_failed) =
			NotebookAuditFailureStore::has_unresolved_audit_failure(pool).await?
		{
			*audit_failure_number.lock().await = Some(is_failed.notebook_number as NotebookNumber);
		}

		let audit_failure_number_copy = audit_failure_number.clone();
		let handle = tokio::spawn(async move {
			loop {
				while let Ok(notebook_number) = audit_failure_listener.next().await {
					tracing::error!("Audit failure for notebook {}", notebook_number);
					*audit_failure_number_copy.lock().await = Some(notebook_number);
				}
			}
		});
		Ok((audit_failure_number, handle))
	}

	async fn ensure_active(&self) -> Result<(), Error> {
		if let Some(notebook_number) = *self.audit_failure_number.lock().await {
			Err(Error::NotaryFailedAudit(notebook_number))
		} else {
			Ok(())
		}
	}

	async fn disallow_notebook_after_audit_failure(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<(), Error> {
		if let Some(failed_notebook_number) = *self.audit_failure_number.lock().await {
			if notebook_number >= failed_notebook_number {
				return Err(Error::NotaryFailedAudit(notebook_number));
			}
		}
		Ok(())
	}

	async fn get_conn(&self) -> Result<PoolConnection<Postgres>, ErrorObjectOwned> {
		let conn = self.pool.acquire().await.map_err(|e| Error::Database(e.to_string()))?;
		Ok(conn)
	}
}

#[async_trait]
impl SystemRpcServer for NotaryServer {
	async fn get_archive_base_url(&self) -> Result<String, ErrorObjectOwned> {
		Ok(self.archive_settings.archive_host.clone())
	}

	async fn health(&self) -> Result<(), ErrorObjectOwned> {
		self.pool.acquire().await.map_err(|e| Error::Database(e.to_string()))?;
		Ok(())
	}
}

#[async_trait]
impl NotebookRpcServer for NotaryServer {
	async fn get_balance_proof(
		&self,
		notebook_number: NotebookNumber,
		balance_tip: BalanceTip,
	) -> Result<BalanceProof, ErrorObjectOwned> {
		self.disallow_notebook_after_audit_failure(notebook_number).await?;
		let mut db = self.get_conn().await?;

		let merkle_proof = NotebookStore::get_balance_proof(
			&mut *db,
			self.notary_id,
			notebook_number,
			&balance_tip,
		)
		.await?;

		let tick = NotebookHeaderStore::get_notebook_tick(&mut db, notebook_number).await?;
		Ok(BalanceProof {
			notebook_number,
			notary_id: self.notary_id,
			notebook_proof: merkle_proof.into(),
			account_origin: balance_tip.account_origin,
			balance: balance_tip.balance,
			tick,
		})
	}

	async fn get_notarization(
		&self,
		account_id: AccountId,
		account_type: AccountType,
		notebook_number: NotebookNumber,
		change_number: u32,
	) -> Result<Notarization, ErrorObjectOwned> {
		self.disallow_notebook_after_audit_failure(notebook_number).await?;
		self.ensure_active().await?;
		let mut db = self.get_conn().await?;
		let notarization = NotarizationsStore::get_account_change(
			&mut db,
			notebook_number,
			account_id,
			account_type,
			change_number,
		)
		.await?;
		Ok(notarization)
	}

	async fn metadata(&self) -> Result<NotebookMeta, ErrorObjectOwned> {
		Ok(self.latest_metadata.lock().await.clone())
	}

	async fn get_header_download_url(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<String, ErrorObjectOwned> {
		let archive_host = self.archive_settings.archive_host.clone();
		Ok(get_header_url(&archive_host, self.notary_id, notebook_number))
	}

	async fn get_notebook_download_url(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<String, ErrorObjectOwned> {
		let archive_host = self.archive_settings.archive_host.clone();
		Ok(get_notebook_url(&archive_host, self.notary_id, notebook_number))
	}

	async fn subscribe_headers(&self, pending: PendingSubscriptionSink) -> SubscriptionResult {
		self.ensure_active().await?;
		let stream = self.completed_notebook_stream.subscribe(1_000);

		pipe_from_stream_and_drop(pending, stream, |a| {
			let obj = NotebookSubscriptionBroadcast::build(a, &self.archive_settings.archive_host);
			SubscriptionMessage::from_json(&obj).map_err(Into::into)
		})
		.await
		.map_err(Into::into)
	}
}

#[async_trait]
impl LocalchainRpcServer for NotaryServer {
	async fn notarize(
		&self,
		balance_changeset: NotarizationBalanceChangeset,
		block_votes: NotarizationBlockVotes,
		domains: NotarizationDomains,
	) -> Result<BalanceChangeResult, ErrorObjectOwned> {
		self.ensure_active().await?;
		Ok(NotarizationsStore::apply(
			&self.pool,
			self.notary_id,
			&self.operator_account_id,
			&self.ticker,
			&self.notary_metrics,
			balance_changeset.into_inner(),
			block_votes.into_inner(),
			domains.into_inner(),
		)
		.await
		.inspect_err(|_| {
			self.notary_metrics.on_notarization_error();
		})?)
	}

	async fn get_tip(
		&self,
		account_id: AccountId,
		account_type: AccountType,
	) -> Result<BalanceTipResult, ErrorObjectOwned> {
		let mut db = self.get_conn().await?;
		let tip = BalanceTipStore::get_tip(&mut db, &account_id, account_type).await?;
		self.disallow_notebook_after_audit_failure(tip.notebook_number).await?;
		Ok(tip)
	}

	async fn get_origin(
		&self,
		account_id: AccountId,
		account_type: AccountType,
	) -> Result<AccountOrigin, ErrorObjectOwned> {
		let mut db = self.get_conn().await?;
		let origin =
			NotebookStore::get_account_origin(&mut db, account_id.clone(), account_type).await?;
		self.disallow_notebook_after_audit_failure(origin.notebook_number).await?;
		Ok(origin)
	}
}

pub async fn pipe_from_stream_and_drop<T: Serialize>(
	pending: PendingSubscriptionSink,
	mut stream: impl Stream<Item = T> + Unpin,
	transform: impl Fn(T) -> Result<SubscriptionMessage, anyhow::Error>,
) -> Result<(), anyhow::Error> {
	let mut sink = pending.accept().await?;

	loop {
		tokio::select! {
			_ = sink.closed() => break Err(anyhow::anyhow!("Subscription was closed")),
			maybe_item = stream.next() => {
				let msg = match maybe_item {
					Some(item) => transform(item)?,
					None => break Err(anyhow::anyhow!("Subscription was closed")),
				};
				match sink.try_send(msg) {
					Ok(_) => (),
					Err(TrySendError::Closed(_)) => break Err(anyhow::anyhow!("Subscription was closed")),
					// BAB - copied this message.. don't know better option. "channel is full, let's be naive an just drop the message."
					Err(TrySendError::Full(_)) => (),
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use binary_merkle_tree::verify_proof;
	use chrono::Utc;
	use codec::Encode;
	use futures::{StreamExt, TryStreamExt};
	use jsonrpsee::ws_client::WsClientBuilder;
	use prometheus::Registry;
	use sp_core::{bounded_vec, ed25519::Signature, Blake2Hasher};
	use sp_keyring::{AccountKeyring::Ferdie, Ed25519Keyring::Bob};
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sqlx::PgPool;

	use argon_primitives::{
		tick::Ticker, AccountOrigin, AccountType::Deposit, BalanceChange, BalanceTip,
		ChainTransfer, NewAccountOrigin, Note, NoteType,
	};

	use super::NotaryServer;
	use crate::{
		notebook_closer::{FinalizedNotebookHeaderListener, NotebookCloser, NOTARY_KEYID},
		s3_archive::S3Archive,
		stores::{
			blocks::BlocksStore, chain_transfer::ChainTransferStore,
			notebook_audit_failure::NotebookAuditFailureStore,
			notebook_header::NotebookHeaderStore, registered_key::RegisteredKeyStore,
		},
	};
	use argon_notary_apis::{
		download_notebook, download_notebook_header,
		localchain::{BalanceChangeResult, LocalchainRpcClient},
		notebook::NotebookRpcClient,
	};

	#[sqlx::test]
	async fn test_balance_change_and_get_proof(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let ticker = Ticker::new(60_000, 2);
		let operator = Ferdie.to_account_id();
		let (s3_buckets, archive_settings) =
			S3Archive::rand_minio_test_bucket(1, None, None).await?;

		let notary = NotaryServer::start(
			1,
			operator.clone(),
			pool.clone(),
			archive_settings,
			Default::default(),
			ticker,
			"127.0.0.1:0",
			Registry::new(),
		)
		.await?;
		assert!(notary.addr.port() > 0);

		let mut db = notary.pool.acquire().await?;
		BlocksStore::record(&mut db, 0, [1u8; 32].into(), [0u8; 32].into(), 100, vec![]).await?;
		BlocksStore::record_finalized(&mut db, [1u8; 32].into()).await?;
		NotebookHeaderStore::create(&mut db, notary.notary_id, 1, 1, ticker.time_for_tick(1))
			.await?;
		ChainTransferStore::record_transfer_to_local_from_block(
			&mut *db,
			0,
			10,
			&Bob.to_account_id(),
			1,
			1000,
		)
		.await?;

		let client = WsClientBuilder::default().build(format!("ws://{}", notary.addr)).await?;

		let balance_change = BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: Deposit,
			change_number: 1,
			balance: 1000,
			previous_balance_proof: None,
			notes: bounded_vec![Note::create(
				1000,
				NoteType::ClaimFromMainchain { transfer_id: 1 }
			)],
			channel_hold_note: None,
			signature: Signature::from_raw([0; 64]).into(),
		}
		.sign(Bob.pair())
		.clone();

		assert_eq!(
			client
				.notarize(bounded_vec![balance_change], bounded_vec![], bounded_vec![])
				.await?,
			BalanceChangeResult {
				notebook_number: 1,
				tick: 1,
				new_account_origins: vec![NewAccountOrigin::new(Bob.to_account_id(), Deposit, 1)],
			}
		);

		let subscription = client.subscribe_headers().await?;
		let keystore = MemoryKeystore::new();
		let keystore = KeystoreExt::new(keystore);
		let key = keystore
			.ed25519_generate_new(NOTARY_KEYID, None)
			.expect("Should be able to create a key");
		RegisteredKeyStore::store_public(&mut *db, key, 0).await?;

		let mut closer = NotebookCloser {
			pool: pool.clone(),
			notary_id: notary.notary_id,
			keystore: keystore.clone(),
			operator_account_id: operator.clone(),
			ticker,
			s3_buckets,
			notary_metrics: notary.notary_metrics.clone(),
		};
		let mut header_listener = FinalizedNotebookHeaderListener::connect(
			pool.clone(),
			notary.completed_notebook_sender.clone(),
		)
		.await?;

		sqlx::query("update notebook_status set end_time = $1 where notebook_number = 1")
			.bind(Utc::now())
			.execute(&mut *db)
			.await?;

		closer.try_rotate_notebook().await?;
		closer.try_close_notebook().await?;
		let _ = header_listener.next().await;

		let mut stream = subscription.into_stream();
		let notebook_number = stream.next().await.unwrap()?.notebook_number;
		let header = download_notebook_header(&client, notebook_number).await?.header;

		assert_eq!(header.notebook_number, 1);
		assert_eq!(header.chain_transfers[0], ChainTransfer::ToLocalchain { transfer_id: 1 });

		let tip = BalanceTip {
			account_id: Bob.to_account_id(),
			account_type: Deposit,
			change_number: 1,
			balance: 1000,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			channel_hold_note: None,
		};

		let proof = client.get_balance_proof(header.notebook_number, tip.clone()).await?;

		let notebook_proof = proof.notebook_proof.expect("Should have notebook proof");
		assert!(verify_proof::<Blake2Hasher, _, _>(
			&header.changed_accounts_root,
			notebook_proof.proof,
			notebook_proof.number_of_leaves as usize,
			notebook_proof.leaf_index as usize,
			&tip.encode(),
		));

		Ok(())
	}

	#[sqlx::test]
	async fn test_should_block_apis_if_audit_fails(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let ticker = Ticker::new(2_000, 2);
		let start_tick = ticker.current();
		let operator = Ferdie.to_account_id();
		let (s3_buckets, archive_settings) =
			S3Archive::rand_minio_test_bucket(1, None, None).await?;
		let notary = NotaryServer::start(
			1,
			operator.clone(),
			pool.clone(),
			archive_settings.clone(),
			Default::default(),
			ticker,
			"127.0.0.1:0",
			Registry::new(),
		)
		.await?;
		assert!(notary.addr.port() > 0);

		let mut db = notary.pool.acquire().await?;
		BlocksStore::record(&mut db, 0, [1u8; 32].into(), [0u8; 32].into(), 100, vec![]).await?;
		BlocksStore::record_finalized(&mut db, [1u8; 32].into()).await?;
		NotebookHeaderStore::create(
			&mut db,
			notary.notary_id,
			1,
			start_tick,
			ticker.time_for_tick(start_tick),
		)
		.await?;

		let client = WsClientBuilder::default().build(format!("ws://{}", notary.addr)).await?;

		let keystore = MemoryKeystore::new();
		let keystore = KeystoreExt::new(keystore);
		let notary_key =
			keystore.ed25519_generate_new(NOTARY_KEYID, None).expect("should have a key");
		RegisteredKeyStore::store_public(&mut *db, notary_key, 1).await?;

		let mut notebook_closer = NotebookCloser {
			pool: pool.clone(),
			notary_id: notary.notary_id,
			keystore: keystore.clone(),
			operator_account_id: operator.clone(),
			ticker,
			s3_buckets,
			notary_metrics: notary.notary_metrics.clone(),
		};

		notebook_closer
			.try_rotate_notebook()
			.await
			.expect("Should be able to rotate notebook");
		notebook_closer
			.try_close_notebook()
			.await
			.expect("Should be able to close notebook");
		// now we have an audit failure

		let notebook1 = download_notebook(&client, 1)
			.await
			.expect("Should be able to download notebook");
		NotebookAuditFailureStore::record(&mut db, 1, notebook1.hash, "failure".to_string(), 1)
			.await
			.expect("Should be able to record audit failure");
		{
			let mut stream = notary.audit_failure_stream.subscribe(2);
			stream.next().await.expect("Should get audit failure");
		}

		assert_eq!(*notary.audit_failure_number.lock().await, Some(1));
		assert_eq!(
			client.get_header_download_url(1).await?,
			format!("{}/notary/1/header/{}.scale", &archive_settings.archive_host, 1),
			"should NOT cut off access to headers"
		);
		client
			.get_tip(Bob.to_account_id(), Deposit)
			.await
			.expect_err("Should block access");
		Ok(())
	}
}
