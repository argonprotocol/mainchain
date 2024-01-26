use jsonrpsee::{
  core::{async_trait, SubscriptionResult},
  server::{PendingSubscriptionSink, Server, ServerHandle, SubscriptionMessage},
  types::ErrorObjectOwned,
  RpcModule,
};
use napi::bindgen_prelude::Uint8Array;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{ConnectOptions, SqlitePool};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{MainchainClient, NotaryClient, NotaryClients};
use sc_utils::notification::NotificationSender;
use sp_core::crypto::key_types::ACCOUNT;
use sp_core::{LogLevelFilter, Pair, H256};
use sp_keyring::Ed25519Keyring;
use sp_keystore::testing::MemoryKeystore;
use sp_keystore::{Keystore, KeystorePtr};
use tokio::sync::Mutex;
use ulx_notary::apis::localchain::{BalanceChangeResult, BalanceTipResult, LocalchainRpcServer};
use ulx_notary::apis::notebook::NotebookRpcServer;
use ulx_notary::server::pipe_from_stream_and_drop;
use ulx_notary::server::NotebookHeaderStream;
use ulx_primitives::{
  AccountId, AccountType, BalanceProof, BalanceTip, Notarization, NotarizationBalanceChangeset,
  NotarizationBlockVotes, NotarizationDataDomains, Notebook, NotebookMeta, NotebookNumber,
  SignedNotebookHeader,
};

/// Debug sqlite connections. This function is for sqlx unit tests. To activate, your test signature
/// should look like this:
///
/// ```
/// use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
///   use crate::test_utils::connect_with_logs;
///
///
///  #[sqlx::test]
///   async fn my_test(pool_options: SqlitePoolOptions, connect_options: SqliteConnectOptions,) -> anyhow::Result<()> {
///     let pool = connect_with_logs(pool_options, connect_options).await?;
/// ```
pub async fn connect_with_logs(
  pool_options: SqlitePoolOptions,
  connect_options: SqliteConnectOptions,
) -> anyhow::Result<SqlitePool> {
  tracing_subscriber::FmtSubscriber::builder()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .try_init()
    .expect("setting default subscriber failed");
  let connect_options = connect_options
    .clone()
    .log_statements(LogLevelFilter::Trace.into());
  let pool = pool_options.connect_with(connect_options).await?;
  Ok(pool)
}

pub(crate) async fn create_mock_notary() -> anyhow::Result<MockNotary> {
  let mut mock_notary = MockNotary::new(1);
  mock_notary.start().await?;
  Ok(mock_notary)
}

#[allow(dead_code)]
pub enum CryptoType {
  Ed25519,
  Sr25519,
  Ecdsa,
}

pub(crate) fn create_keystore(suri: &str, crypt_type: CryptoType) -> anyhow::Result<KeystorePtr> {
  let keystore = MemoryKeystore::new();
  let key = match crypt_type {
    CryptoType::Ed25519 => {
      let pair = sp_core::ed25519::Pair::from_string(suri, None)?;
      let public = pair.public();
      public.0.to_vec()
    }
    CryptoType::Sr25519 => {
      let pair = sp_core::sr25519::Pair::from_string(suri, None)?;
      let public = pair.public();
      public.0.to_vec()
    }
    CryptoType::Ecdsa => {
      let pair = sp_core::ecdsa::Pair::from_string(suri, None)?;
      let public = pair.public();
      public.0.to_vec()
    }
  };
  keystore
    .insert(ACCOUNT, &suri, &key)
    .expect("should insert");
  Ok(keystore.into())
}

pub(crate) async fn mock_notary_clients(
  mock_notary: &MockNotary,
  operator: Ed25519Keyring,
) -> anyhow::Result<NotaryClients> {
  let notary_clients = NotaryClients::new(&MainchainClient::mock());
  let notary_client = NotaryClient::connect(
    mock_notary.notary_id,
    Uint8Array::from(operator.pair().public().to_vec()),
    mock_notary.addr.clone(),
    false,
  )
  .await?;
  notary_clients.use_client(&notary_client).await;
  Ok(notary_clients)
}
#[derive(Debug, Default)]
pub struct NotaryState {
  pub balance_tips: BTreeMap<(AccountId, AccountType), BalanceTipResult>,
  pub balance_proofs: BTreeMap<(NotebookNumber, H256), BalanceProof>,
  pub notarize_result: Option<BalanceChangeResult>,
  pub metadata: Option<NotebookMeta>,
  pub notarizations: BTreeMap<(AccountId, AccountType, NotebookNumber, u32), Notarization>,
  pub headers: HashMap<NotebookNumber, SignedNotebookHeader>,
}

#[derive(Clone)]
pub struct MockNotary {
  server_handle: Option<ServerHandle>,
  pub addr: String,
  pub notary_id: u32,
  pub state: Arc<Mutex<NotaryState>>,
  pub header_channel: (
    NotificationSender<SignedNotebookHeader>,
    NotebookHeaderStream,
  ),
}

impl MockNotary {
  pub fn new(notary_id: u32) -> Self {
    Self {
      notary_id,
      state: Default::default(),
      header_channel: NotebookHeaderStream::channel(),
      server_handle: None,
      addr: String::new(),
    }
  }

  #[allow(dead_code)]
  pub async fn stop(&mut self) {
    if let Some(server_handle) = self.server_handle.take() {
      server_handle.stop().expect("Should be able to stop server");
      server_handle.stopped().await;
    }
  }

  pub async fn start(&mut self) -> anyhow::Result<()> {
    let server = Server::builder().build("127.0.0.1:0").await?;
    self.addr = format!("ws://{:?}", server.local_addr()?);

    let mut module = RpcModule::new(());
    module.merge(NotebookRpcServer::into_rpc(self.clone()))?;
    module.merge(LocalchainRpcServer::into_rpc(self.clone()))?;

    let handle = server.start(module);
    self.server_handle = Some(handle.clone());
    // handle in background
    tokio::spawn(handle.stopped());

    Ok(())
  }

  pub async fn add_notebook_header(&mut self, header: SignedNotebookHeader) {
    let mut state = self.state.lock().await;
    (*state)
      .headers
      .insert(header.header.notebook_number, header.clone());
    state.metadata = Some(NotebookMeta {
      finalized_tick: header.header.tick,
      finalized_notebook_number: header.header.notebook_number,
    });
    drop(state);
    let _ = self
      .header_channel
      .0
      .notify(|| Ok::<_, anyhow::Error>(header));
  }
  pub async fn set_notarization_result(&self, result: BalanceChangeResult) {
    let mut state = self.state.lock().await;
    (*state).notarize_result = Some(result);
  }
  pub async fn add_notarization(
    &self,
    notebook_number: NotebookNumber,
    notarization: Notarization,
  ) {
    let mut state = self.state.lock().await;
    for change in &notarization.balance_changes {
      (*state).notarizations.insert(
        (
          change.account_id.clone(),
          change.account_type.clone(),
          notebook_number,
          change.change_number,
        ),
        notarization.clone(),
      );
    }
  }
}

#[async_trait]
impl LocalchainRpcServer for MockNotary {
  async fn notarize(
    &self,
    _balance_changeset: NotarizationBalanceChangeset,
    _block_votes: NotarizationBlockVotes,
    _data_domains: NotarizationDataDomains,
  ) -> Result<BalanceChangeResult, ErrorObjectOwned> {
    let mut state = self.state.lock().await;
    (*state).notarize_result.take().ok_or_else(|| {
      ErrorObjectOwned::owned(
        -32000,
        "MockNotary notarize_result not set".to_string(),
        None::<String>,
      )
    })
  }
  async fn get_tip(
    &self,
    account_id: AccountId,
    account_type: AccountType,
  ) -> Result<BalanceTipResult, ErrorObjectOwned> {
    let state = self.state.lock().await;
    (*state)
      .balance_tips
      .get(&(account_id, account_type))
      .cloned()
      .ok_or_else(|| {
        ErrorObjectOwned::owned(
          -32000,
          "MockNotary balance_tip not set".to_string(),
          None::<String>,
        )
      })
  }
}

#[async_trait]
impl NotebookRpcServer for MockNotary {
  async fn get_balance_proof(
    &self,
    notebook_number: NotebookNumber,
    balance_tip: BalanceTip,
  ) -> Result<BalanceProof, ErrorObjectOwned> {
    let hash = balance_tip.tip();
    let state = self.state.lock().await;
    (*state)
      .balance_proofs
      .get(&(notebook_number, hash.into()))
      .cloned()
      .ok_or_else(|| {
        ErrorObjectOwned::owned(
          -32000,
          "MockNotary balance_proof not set".to_string(),
          None::<String>,
        )
      })
  }

  async fn get_notarization(
    &self,
    account_id: AccountId,
    account_type: AccountType,
    notebook_number: NotebookNumber,
    change_number: u32,
  ) -> Result<Notarization, ErrorObjectOwned> {
    let state = self.state.lock().await;
    (*state)
      .notarizations
      .get(&(account_id, account_type, notebook_number, change_number))
      .cloned()
      .ok_or_else(|| {
        ErrorObjectOwned::owned(
          -32000,
          "MockNotary notarization not set".to_string(),
          None::<String>,
        )
      })
  }

  async fn metadata(&self) -> Result<NotebookMeta, ErrorObjectOwned> {
    let state = self.state.lock().await;
    (*state).metadata.clone().ok_or_else(|| {
      ErrorObjectOwned::owned(
        -32000,
        "MockNotary metadata not set".to_string(),
        None::<String>,
      )
    })
  }

  async fn get_header(
    &self,
    notebook_number: NotebookNumber,
  ) -> Result<SignedNotebookHeader, ErrorObjectOwned> {
    let state = self.state.lock().await;
    (*state)
      .headers
      .get(&notebook_number)
      .cloned()
      .ok_or_else(|| {
        ErrorObjectOwned::owned(
          -32000,
          "MockNotary header not set".to_string(),
          None::<String>,
        )
      })
  }

  async fn get_raw_headers(
    &self,
    _since_notebook: NotebookNumber,
  ) -> Result<Vec<(NotebookNumber, Vec<u8>)>, ErrorObjectOwned> {
    todo!()
  }

  async fn get(&self, _notebook_number: NotebookNumber) -> Result<Notebook, ErrorObjectOwned> {
    todo!()
  }

  async fn get_raw_body(
    &self,
    _notebook_number: NotebookNumber,
  ) -> Result<Vec<u8>, ErrorObjectOwned> {
    todo!()
  }

  async fn subscribe_headers(
    &self,
    subscription_sink: PendingSubscriptionSink,
  ) -> SubscriptionResult {
    let stream = self.header_channel.1.subscribe(1_000);
    pipe_from_stream_and_drop(subscription_sink, stream, |a| {
      SubscriptionMessage::from_json(&a).map_err(Into::into)
    })
    .await
    .map_err(Into::into)
  }

  async fn subscribe_raw_headers(
    &self,
    _subscription_sink: PendingSubscriptionSink,
  ) -> SubscriptionResult {
    todo!()
  }
}
