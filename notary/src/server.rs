use std::net::SocketAddr;

use futures::{Stream, StreamExt};
use jsonrpsee::{
	core::{async_trait, SubscriptionResult},
	server::{PendingSubscriptionSink, Server, SubscriptionMessage},
	types::ErrorObjectOwned,
	RpcModule, TrySendError,
};
use sc_utils::notification::{NotificationSender, NotificationStream, TracingKeyStr};
use serde::Serialize;
use sqlx::PgPool;
use tokio::net::ToSocketAddrs;

use ulx_notary_primitives::{
	BalanceProof, BalanceTip, NotarizationBalanceChangeset, NotarizationBlockVotes, NotaryId,
	Notebook, NotebookHeader, NotebookNumber,
};

use crate::{
	apis::{
		localchain::{BalanceChangeResult, LocalchainRpcServer},
		notebook::NotebookRpcServer,
	},
	stores::{
		notarizations::NotarizationsStore, notebook::NotebookStore,
		notebook_header::NotebookHeaderStore,
	},
	Error,
};

pub type NotebookHeaderStream = NotificationStream<NotebookHeader, NotebookHeaderTracingKey>;

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
	completed_notebook_stream: NotebookHeaderStream,
	pub completed_notebook_sender: NotificationSender<NotebookHeader>,
}

impl NotaryServer {
	pub async fn start(
		notary_id: NotaryId,
		pool: PgPool,
		addrs: impl ToSocketAddrs,
	) -> anyhow::Result<Self> {
		let (completed_notebook_sender, completed_notebook_stream) =
			NotebookHeaderStream::channel();

		let server = Server::builder().build(addrs).await?;

		let addr = server.local_addr()?;
		let notary_server =
			Self { notary_id, completed_notebook_sender, completed_notebook_stream, pool, addr };

		let mut module = RpcModule::new(());
		module.merge(NotebookRpcServer::into_rpc(notary_server.clone()))?;
		module.merge(LocalchainRpcServer::into_rpc(notary_server.clone()))?;

		let handle = server.start(module);

		tokio::spawn(handle.stopped());

		Ok(notary_server)
	}
}

#[async_trait]
impl NotebookRpcServer for NotaryServer {
	async fn get_balance_proof(
		&self,
		notebook_number: NotebookNumber,
		balance_tip: BalanceTip,
	) -> Result<BalanceProof, ErrorObjectOwned> {
		NotebookStore::get_balance_proof(&self.pool, self.notary_id, notebook_number, &balance_tip)
			.await
			.map_err(from_crate_error)
			.map(|a| BalanceProof {
				notebook_number,
				notary_id: self.notary_id,
				notebook_proof: a.into(),
				account_origin: balance_tip.account_origin,
				balance: balance_tip.balance,
			})
	}
	async fn get_header(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<NotebookHeader, ErrorObjectOwned> {
		NotebookHeaderStore::load(&self.pool, notebook_number)
			.await
			.map_err(from_crate_error)
	}

	async fn get(&self, notebook_number: NotebookNumber) -> Result<Notebook, ErrorObjectOwned> {
		let mut db = self
			.pool
			.acquire()
			.await
			.map_err(|e| from_crate_error(Error::Database(e.to_string())))?;

		Ok(NotebookStore::load(&mut *db, notebook_number).await.map_err(from_crate_error)?)
	}

	async fn get_raw_body(
		&self,
		notebook_number: NotebookNumber,
	) -> Result<Vec<u8>, ErrorObjectOwned> {
		let mut db = self
			.pool
			.acquire()
			.await
			.map_err(|e| from_crate_error(Error::Database(e.to_string())))?;

		Ok(NotebookStore::load_raw(&mut *db, notebook_number)
			.await
			.map_err(from_crate_error)?)
	}

	async fn subscribe_headers(&self, pending: PendingSubscriptionSink) -> SubscriptionResult {
		let stream = self.completed_notebook_stream.subscribe(1_000);

		pipe_from_stream_and_drop(pending, stream).await.map_err(Into::into)
	}
}

#[async_trait]
impl LocalchainRpcServer for NotaryServer {
	async fn notarize(
		&self,
		balance_changeset: NotarizationBalanceChangeset,
		block_votes: NotarizationBlockVotes,
	) -> Result<BalanceChangeResult, ErrorObjectOwned> {
		NotarizationsStore::apply(
			&self.pool,
			self.notary_id,
			balance_changeset.into_inner(),
			block_votes.into_inner(),
		)
		.await
		.map_err(from_crate_error)
	}
}

async fn pipe_from_stream_and_drop<T: Serialize>(
	pending: PendingSubscriptionSink,
	mut stream: impl Stream<Item = T> + Unpin,
) -> Result<(), anyhow::Error> {
	let mut sink = pending.accept().await?;

	loop {
		tokio::select! {
			_ = sink.closed() => break Err(anyhow::anyhow!("Subscription was closed")),
			maybe_item = stream.next() => {
				let item = match maybe_item {
					Some(item) => item,
					None => break Err(anyhow::anyhow!("Subscription was closed")),
				};
				let msg = SubscriptionMessage::from_json(&item)?;
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

fn from_crate_error(e: crate::Error) -> ErrorObjectOwned {
	let msg = e.to_string();
	let code: i32 = Into::<i32>::into(e);
	ErrorObjectOwned::owned(code, msg, None::<String>)
}

#[cfg(test)]
mod tests {
	use binary_merkle_tree::verify_proof;
	use codec::Encode;
	use futures::{StreamExt, TryStreamExt};
	use jsonrpsee::ws_client::WsClientBuilder;
	use sp_core::{bounded_vec, ed25519::Signature, Blake2Hasher};
	use sp_keyring::Ed25519Keyring::Bob;
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
	use sqlx::PgPool;

	use ulx_notary_primitives::{
		AccountOrigin, AccountType::Deposit, BalanceChange, BalanceTip, BlockVoteEligibility,
		ChainTransfer, NewAccountOrigin, Note, NoteType,
	};

	use crate::{
		apis::{
			localchain::{BalanceChangeResult, LocalchainRpcClient},
			notebook::NotebookRpcClient,
		},
		notebook_closer::{MainchainClient, NotebookCloser, NOTARY_KEYID},
		stores::{
			blocks::BlocksStore, chain_transfer::ChainTransferStore,
			notebook_header::NotebookHeaderStore, registered_key::RegisteredKeyStore,
		},
	};

	use super::NotaryServer;

	#[sqlx::test]
	async fn test_balance_change_and_get_proof(pool: PgPool) -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let notary = NotaryServer::start(1, pool.clone(), "127.0.0.1:0").await?;
		assert!(notary.addr.port() > 0);

		let mut db = notary.pool.acquire().await?;
		BlocksStore::record(
			&mut *db,
			0,
			[1u8; 32].into(),
			[0u8; 32].into(),
			BlockVoteEligibility::new(100, Default::default()),
			[1u8; 32].into(),
			None,
		)
		.await?;
		BlocksStore::record_finalized(&mut *db, [1u8; 32].into()).await?;
		NotebookHeaderStore::create(&mut *db, notary.notary_id, 1, 0).await?;
		ChainTransferStore::record_transfer_to_local_from_block(
			&mut *db,
			0,
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
				NoteType::ClaimFromMainchain { account_nonce: 1 }
			)],
			channel_hold_note: None,
			signature: Signature([0; 64]).into(),
		}
		.sign(Bob.pair())
		.clone();

		assert_eq!(
			client.notarize(bounded_vec![balance_change], bounded_vec![]).await?,
			BalanceChangeResult {
				notebook_number: 1,
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
			completed_notebook_sender: notary.completed_notebook_sender.clone(),
			keystore: keystore.clone(),
			client: MainchainClient::new(vec![format!("ws://{}", notary.addr)]),
		};
		sqlx::query("update notebook_status set open_time = now() - interval '2 minutes' where notebook_number = 1")
			.execute(&mut *db)
			.await?;

		closer.try_rotate_notebook().await?;
		closer.try_close_notebook().await?;

		let mut stream = subscription.into_stream();
		let header = stream.next().await.unwrap()?;

		assert_eq!(header.notebook_number, 1);
		assert_eq!(
			header.chain_transfers[0],
			ChainTransfer::ToLocalchain { account_id: Bob.to_account_id(), account_nonce: 1 }
		);

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
}
