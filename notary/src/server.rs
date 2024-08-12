use std::net::SocketAddr;

use crate::{
	stores::{
		balance_tip::BalanceTipStore, notarizations::NotarizationsStore, notebook::NotebookStore,
		notebook_header::NotebookHeaderStore,
	},
	Error,
};
use argon_notary_apis::{
	localchain::{BalanceChangeResult, BalanceTipResult, LocalchainRpcServer},
	notebook::NotebookRpcServer,
};
use argon_primitives::{
	tick::Ticker, AccountId, AccountOrigin, AccountType, BalanceProof, BalanceTip, Notarization,
	NotarizationBalanceChangeset, NotarizationBlockVotes, NotarizationDataDomains, NotaryId,
	Notebook, NotebookMeta, NotebookNumber, SignedNotebookHeader,
};
use codec::Encode;
use futures::{Stream, StreamExt};
use jsonrpsee::{
	core::{async_trait, SubscriptionResult},
	server::{
		middleware::rpc::RpcLoggerLayer, PendingSubscriptionSink, PingConfig, RpcServiceBuilder,
		Server, ServerHandle, SubscriptionMessage,
	},
	types::ErrorObjectOwned,
	RpcModule, TrySendError,
};
use sc_utils::notification::{NotificationSender, NotificationStream, TracingKeyStr};
use serde::Serialize;
use sqlx::{pool::PoolConnection, PgPool, Postgres};
use tokio::net::ToSocketAddrs;
use tower::layer::util::{Identity, Stack};

pub type NotebookHeaderStream = NotificationStream<SignedNotebookHeader, NotebookHeaderTracingKey>;

#[derive(Clone)]
pub struct NotebookHeaderTracingKey;
impl TracingKeyStr for NotebookHeaderTracingKey {
	const TRACING_KEY: &'static str = "mpsc_notebook_header_notification_stream";
}

#[derive(Clone)]
pub struct NotaryServer {
	pub addr: SocketAddr,
	notary_id: NotaryId,
	pool: PgPool,
	ticker: Ticker,
	pub(crate) completed_notebook_stream: NotebookHeaderStream,
	pub completed_notebook_sender: NotificationSender<SignedNotebookHeader>,
	server_handle: Option<ServerHandle>,
}

impl NotaryServer {
	pub async fn create_http_server(
		addrs: impl ToSocketAddrs,
	) -> anyhow::Result<Server<Identity, Stack<RpcLoggerLayer, Identity>>> {
		let rpc_middleware = RpcServiceBuilder::new().rpc_logger(1024);

		let server = Server::builder()
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
		server: Server<Identity, Stack<RpcLoggerLayer, Identity>>,
		notary_id: NotaryId,
		ticker: Ticker,
		pool: PgPool,
	) -> anyhow::Result<Self> {
		let (completed_notebook_sender, completed_notebook_stream) =
			NotebookHeaderStream::channel();

		let addr = server.local_addr()?;
		let mut notary_server = Self {
			notary_id,
			ticker,
			completed_notebook_sender,
			completed_notebook_stream,
			pool,
			addr,
			server_handle: None,
		};

		let mut module = RpcModule::new(());
		module.merge(NotebookRpcServer::into_rpc(notary_server.clone()))?;
		module.merge(LocalchainRpcServer::into_rpc(notary_server.clone()))?;

		let handle = server.start(module);
		notary_server.server_handle = Some(handle.clone());

		Ok(notary_server)
	}

	pub async fn wait_for_close(&self) {
		if let Some(handle) = self.server_handle.clone() {
			handle.stopped().await;
		}
	}

	pub async fn start(
		notary_id: NotaryId,
		pool: PgPool,
		ticker: Ticker,
		addrs: impl ToSocketAddrs,
	) -> anyhow::Result<Self> {
		let server = Self::create_http_server(addrs).await?;
		Self::start_with(server, notary_id, ticker, pool).await
	}

	async fn get_conn(&self) -> Result<PoolConnection<Postgres>, ErrorObjectOwned> {
		let conn = self.pool.acquire().await.map_err(|e| Error::Database(e.to_string()))?;
		Ok(conn)
	}
}

#[async_trait]
impl NotebookRpcServer for NotaryServer {
	async fn get_balance_proof(
		&self,
		notebook_number: NotebookNumber,
		balance_tip: BalanceTip,
	) -> Result<BalanceProof, ErrorObjectOwned> {
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

	async fn get_header(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<SignedNotebookHeader, ErrorObjectOwned> {
		Ok(NotebookHeaderStore::load_with_signature(&self.pool, notebook_number).await?)
	}

	async fn get_raw_headers(
		&self,
		since_notebook: Option<NotebookNumber>,
		or_specific_notebooks: Option<Vec<NotebookNumber>>,
	) -> Result<Vec<(NotebookNumber, Vec<u8>)>, ErrorObjectOwned> {
		Ok(NotebookHeaderStore::load_raw_signed_headers(
			&self.pool,
			since_notebook,
			or_specific_notebooks,
		)
		.await?)
	}

	async fn metadata(&self) -> Result<NotebookMeta, ErrorObjectOwned> {
		Ok(NotebookHeaderStore::latest(&self.pool).await?)
	}

	async fn get(&self, notebook_number: NotebookNumber) -> Result<Notebook, ErrorObjectOwned> {
		let mut db = self.get_conn().await?;

		Ok(NotebookStore::load_finalized(&mut db, notebook_number).await?)
	}

	async fn get_raw_body(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<Vec<u8>, ErrorObjectOwned> {
		let mut db = self.get_conn().await?;

		Ok(NotebookStore::load_raw(&mut db, notebook_number).await?)
	}

	async fn subscribe_headers(&self, pending: PendingSubscriptionSink) -> SubscriptionResult {
		let stream = self.completed_notebook_stream.subscribe(1_000);

		pipe_from_stream_and_drop(pending, stream, |a| {
			SubscriptionMessage::from_json(&a).map_err(Into::into)
		})
		.await
		.map_err(Into::into)
	}

	async fn subscribe_raw_headers(&self, pending: PendingSubscriptionSink) -> SubscriptionResult {
		let stream = self.completed_notebook_stream.subscribe(1_000);

		pipe_from_stream_and_drop(pending, stream, |item| {
			SubscriptionMessage::from_json(&(item.header.notebook_number, item.encode()))
				.map_err(Into::into)
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
		data_domains: NotarizationDataDomains,
	) -> Result<BalanceChangeResult, ErrorObjectOwned> {
		Ok(NotarizationsStore::apply(
			&self.pool,
			self.notary_id,
			&self.ticker,
			balance_changeset.into_inner(),
			block_votes.into_inner(),
			data_domains.into_inner(),
		)
		.await?)
	}

	async fn get_tip(
		&self,
		account_id: AccountId,
		account_type: AccountType,
	) -> Result<BalanceTipResult, ErrorObjectOwned> {
		let mut db = self.get_conn().await?;
		Ok(BalanceTipStore::get_tip(&mut db, &account_id, account_type).await?)
	}

	async fn get_origin(
		&self,
		account_id: AccountId,
		account_type: AccountType,
	) -> Result<AccountOrigin, ErrorObjectOwned> {
		let mut db = self.get_conn().await?;
		Ok(NotebookStore::get_account_origin(&mut db, account_id.clone(), account_type).await?)
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
	use sp_core::{bounded_vec, ed25519::Signature, Blake2Hasher};
	use sp_keyring::Ed25519Keyring::Bob;
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sqlx::PgPool;

	use argon_primitives::{
		tick::Ticker, AccountOrigin, AccountType::Deposit, BalanceChange, BalanceTip,
		ChainTransfer, NewAccountOrigin, Note, NoteType,
	};

	use crate::{
		notebook_closer::{FinalizedNotebookHeaderListener, NotebookCloser, NOTARY_KEYID},
		stores::{
			blocks::BlocksStore, chain_transfer::ChainTransferStore,
			notebook_header::NotebookHeaderStore, registered_key::RegisteredKeyStore,
		},
	};
	use argon_notary_apis::{
		localchain::{BalanceChangeResult, LocalchainRpcClient},
		notebook::NotebookRpcClient,
	};

	use super::NotaryServer;

	#[sqlx::test]
	async fn test_balance_change_and_get_proof(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let ticker = Ticker::new(60_000, Utc::now().timestamp_millis() as u64, 2);
		let notary = NotaryServer::start(1, pool.clone(), ticker.clone(), "127.0.0.1:0").await?;
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
			escrow_hold_note: None,
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
			ticker,
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
		let header = stream.next().await.unwrap()?.header;

		assert_eq!(header.notebook_number, 1);
		assert_eq!(header.chain_transfers[0], ChainTransfer::ToLocalchain { transfer_id: 1 });

		let tip = BalanceTip {
			account_id: Bob.to_account_id(),
			account_type: Deposit,
			change_number: 1,
			balance: 1000,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			escrow_hold_note: None,
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
}
