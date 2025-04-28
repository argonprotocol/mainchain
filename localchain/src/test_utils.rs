use binary_merkle_tree::{merkle_proof, merkle_root};
use codec::Encode;
use frame_support::BoundedVec;
use jsonrpsee::{
  core::{async_trait, SubscriptionResult},
  server::{PendingSubscriptionSink, Server, ServerHandle, SubscriptionMessage},
  types::error::ErrorObjectOwned,
  RpcModule,
};
use polkadot_sdk::*;
use sc_utils::notification::NotificationSender;
use sp_core::ed25519::Signature;
use sp_core::{Blake2Hasher, LogLevelFilter, Pair, H256};
use sp_keyring::Ed25519Keyring;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{ConnectOptions, SqlitePool};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::notarization_builder::NotarizationBuilder;
use crate::notarization_tracker::NotebookProof;
use crate::{
  AccountStore, CryptoScheme, Keystore, Localchain, LocalchainTransfer, MainchainClient,
  NotaryClient, NotaryClients, TickerRef,
};
use argon_notary::server::NotebookHeaderStream;
use argon_notary::server::{pipe_from_stream_and_drop, NotebookHeaderInfo};
use argon_notary_apis::localchain::{BalanceChangeResult, BalanceTipResult, LocalchainRpcServer};
use argon_notary_apis::notebook::{NotebookRpcServer, NotebookSubscriptionBroadcast};
use argon_notary_apis::system::SystemRpcServer;
use argon_notary_apis::{get_header_url, get_notebook_url};
use argon_primitives::tick::Ticker;
use argon_primitives::{
  AccountId, AccountOrigin, AccountOriginUid, AccountType, BalanceChange, BalanceProof, BalanceTip,
  ChainTransfer, LocalchainAccountId, MerkleProof, NewAccountOrigin, Notarization,
  NotarizationBalanceChangeset, NotarizationBlockVotes, NotarizationDomains, NoteType,
  NotebookHeader, NotebookMeta, NotebookNumber, SignedNotebookHeader,
};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{http::StatusCode, routing::get, Router};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

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
  let _ = tracing_subscriber::FmtSubscriber::builder()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .try_init();
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

pub(crate) async fn mock_localchain(
  pool: &SqlitePool,
  suri: &str,
  crypto_scheme: CryptoScheme,
  notary_clients: &NotaryClients,
) -> Localchain {
  let ticker = TickerRef::new(Ticker::start(Duration::from_secs(60), 2));
  let keystore = Keystore::new(pool.clone());
  let _ = keystore
    .import_suri(suri.to_string(), crypto_scheme, None)
    .await
    .expect("should import");
  Localchain {
    db: pool.clone(),
    keystore: keystore.clone(),
    ticker: ticker.clone(),
    notary_clients: notary_clients.clone(),
    path: ":memory:".to_string(),
    mainchain_client: Default::default(),
  }
}

pub(crate) async fn mock_notary_clients(
  mock_notary: &MockNotary,
  operator: Ed25519Keyring,
) -> anyhow::Result<NotaryClients> {
  let notary_clients = NotaryClients::new(&MainchainClient::mock());
  let notary_client = NotaryClient::connect(
    mock_notary.notary_id,
    operator.pair().public().to_vec(),
    mock_notary.addr.clone(),
    false,
  )
  .await?;
  notary_clients.use_client(&notary_client).await;
  Ok(notary_clients)
}
#[derive(Debug, Default)]
pub struct NotaryState {
  pub balance_tips: BTreeMap<LocalchainAccountId, BalanceTipResult>,
  pub balance_proofs: BTreeMap<(NotebookNumber, H256), BalanceProof>,
  pub accounts: BTreeMap<LocalchainAccountId, AccountOrigin>,
  pub metadata: Option<NotebookMeta>,
  pub notarizations: BTreeMap<(AccountId, AccountType, NotebookNumber, u32), Notarization>,
  pub headers: HashMap<NotebookNumber, SignedNotebookHeader>,
  pub mark_bad_tip: BTreeMap<LocalchainAccountId, argon_notary_apis::Error>,
}

#[derive(Clone)]
pub struct MockNotary {
  server_handle: Option<ServerHandle>,
  archive_server_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
  pub addr: String,
  pub archive_host: String,
  pub notary_id: u32,
  pub state: Arc<Mutex<NotaryState>>,
  pub ticker: Arc<Mutex<Ticker>>,
  pub header_channel: (NotificationSender<NotebookHeaderInfo>, NotebookHeaderStream),
}
#[derive(Debug, Deserialize, Serialize)]
struct NotebookParam {
  notebook_number: NotebookNumber,
}

impl MockNotary {
  pub fn new(notary_id: u32) -> Self {
    Self {
      notary_id,
      state: Default::default(),
      header_channel: NotebookHeaderStream::channel(),
      server_handle: None,
      archive_server_handle: Default::default(),
      ticker: Arc::new(Mutex::new(Ticker::start(Duration::from_secs(60), 2))),
      addr: String::new(),
      archive_host: String::new(),
    }
  }

  #[allow(dead_code)]
  pub async fn stop(&mut self) {
    if let Some(server_handle) = self.server_handle.take() {
      server_handle.stop().expect("Should be able to stop server");
      server_handle.stopped().await;
    }

    if let Some(handle) = self.archive_server_handle.lock().await.take() {
      handle.abort();
    }
  }

  pub async fn start(&mut self) -> anyhow::Result<()> {
    // build our application with a route
    let archive_server = Router::new()
      // `GET /` goes to `root`
      .route("/notary/1/header/{notebook_number}", get(Self::get_header))
      .with_state(Arc::new(self.clone()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    self.archive_host = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let archive_server_handle = tokio::spawn(async move {
      axum::serve(listener, archive_server)
        .await
        .expect("Should be able to serve");
    });

    self.archive_server_handle = Arc::new(Mutex::new(Some(archive_server_handle)));

    let server = Server::builder().build("127.0.0.1:0").await?;
    self.addr = format!("ws://{:?}", server.local_addr()?);

    let mut module = RpcModule::new(());
    module.merge(NotebookRpcServer::into_rpc(self.clone()))?;
    module.merge(LocalchainRpcServer::into_rpc(self.clone()))?;
    module.merge(SystemRpcServer::into_rpc(self.clone()))?;

    let handle = server.start(module);
    self.server_handle = Some(handle.clone());

    // handle in background
    tokio::spawn(handle.stopped());

    Ok(())
  }

  async fn get_header(
    State(state): State<Arc<Self>>,
    Path(notebook_number): Path<String>,
  ) -> impl IntoResponse {
    let notebook_number =
      NotebookNumber::from_str(&notebook_number.replace(".scale", "")).expect("should parse");
    let state = state.state.lock().await;
    if let Some(body) = state.headers.get(&notebook_number).map(|x| x.encode()) {
      return (StatusCode::OK, Bytes::from(body));
    }
    (StatusCode::NOT_FOUND, Bytes::from_static(b"Not found"))
  }

  pub async fn set_bad_tip_error(
    &self,
    account_id: AccountId,
    account_type: AccountType,
    error: argon_notary_apis::Error,
  ) {
    let mut state = self.state.lock().await;
    state
      .mark_bad_tip
      .insert(LocalchainAccountId::new(account_id, account_type), error);
  }

  pub async fn add_notebook_header(&self, header: SignedNotebookHeader) {
    let hash = header.header.hash();
    let mut state = self.state.lock().await;
    state
      .headers
      .insert(header.header.notebook_number, header.clone());
    state.metadata = Some(NotebookMeta {
      last_closed_notebook_tick: header.header.tick,
      last_closed_notebook_number: header.header.notebook_number,
    });
    drop(state);
    let _ = self
      .header_channel
      .0
      .notify(|| Ok::<_, anyhow::Error>((header, hash)));
  }

  pub async fn add_notarization(
    &self,
    notebook_number: NotebookNumber,
    notarization: Notarization,
  ) -> anyhow::Result<()> {
    let mut state = self.state.lock().await;

    for change in &notarization.balance_changes {
      state.notarizations.insert(
        (
          change.account_id.clone(),
          change.account_type,
          notebook_number,
          change.change_number,
        ),
        notarization.clone(),
      );
      let local_account_id =
        LocalchainAccountId::new(change.account_id.clone(), change.account_type);
      if let Some(error) = state.mark_bad_tip.remove(&local_account_id) {
        return Err(error.into());
      }
    }
    Ok(())
  }

  pub async fn next_notebook_number(&self) -> NotebookNumber {
    let state = self.state.lock().await;
    let mut notebook_number = 0u32;
    for (num, _) in state.headers.iter() {
      if num > &notebook_number {
        notebook_number = *num;
      }
    }
    notebook_number += 1u32;
    notebook_number
  }

  pub async fn create_claim_from_mainchain(
    &self,
    notarization_builder: NotarizationBuilder,
    amount: u128,
    account_id: AccountId,
  ) -> anyhow::Result<Vec<NotebookProof>> {
    let notebook_number = self.next_notebook_number().await;
    let change_builder = notarization_builder
      .claim_from_mainchain(LocalchainTransfer {
        amount,
        notary_id: 1,
        expiration_tick: self.ticker.lock().await.current() + 10,
        address: AccountStore::to_address(&account_id),
        transfer_id: 1,
      })
      .await?;

    let notarization = notarization_builder.notarize().await?;
    let uid = {
      let accounts = self.state.lock().await;
      let key = LocalchainAccountId {
        account_id: account_id.clone(),
        account_type: AccountType::Deposit,
      };
      if accounts.accounts.contains_key(&key) {
        accounts.accounts.get(&key).unwrap().account_uid
      } else {
        accounts.accounts.len() as AccountOriginUid + 1
      }
    };
    let balance_tip = get_balance_tip(change_builder.inner().await, uid, notebook_number);
    let mut notebook_header = self.create_notebook_header(vec![balance_tip]).await;
    notebook_header
      .chain_transfers
      .try_push(ChainTransfer::ToLocalchain { transfer_id: 1 })
      .expect("should be able to push");

    self
      .add_notebook_header(SignedNotebookHeader {
        header: notebook_header,
        signature: Signature::from_raw([0u8; 64]),
      })
      .await;

    let proof = notarization.get_notebook_proof().await?;
    Ok(proof)
  }

  pub async fn get_pending_tips(&self) -> Vec<BalanceTip> {
    let mut change_by_account = BTreeMap::<LocalchainAccountId, u32>::new();
    let mut pending_tips = vec![];
    let next_notebook_number = self.next_notebook_number().await;
    let notary_state = self.state.lock().await;
    for ((account_id, account_type, notebook_number, change_number), notarization) in
      notary_state.notarizations.iter()
    {
      if *notebook_number == next_notebook_number {
        let key = LocalchainAccountId::new(account_id.clone(), *account_type);
        let should_use = match change_by_account.get(&key) {
          Some(x) => x < change_number,
          None => true,
        };
        if should_use {
          change_by_account.insert(key.clone(), *change_number);
          let balance_changes = notarization.balance_changes.clone().into_inner();
          let balance_change = balance_changes
            .iter()
            .find(|a| {
              a.change_number == *change_number
                && a.account_type == *account_type
                && &a.account_id == account_id
            })
            .expect("");
          let account = notary_state.accounts.get(&key).unwrap();
          pending_tips.push(BalanceTip {
            account_type: *account_type,
            account_id: account_id.clone(),
            account_origin: account.clone(),
            change_number: *change_number,
            balance: balance_change.balance,
            channel_hold_note: balance_change
              .notes
              .iter()
              .find(|a| matches!(a.note_type, NoteType::ChannelHold { .. }))
              .cloned(),
          });
        }
      }
    }

    pending_tips
  }

  pub async fn create_notebook_header(&self, balance_tips: Vec<BalanceTip>) -> NotebookHeader {
    let merkle_leafs = balance_tips.iter().map(|x| x.encode()).collect::<Vec<_>>();
    let changed_accounts_root = merkle_root::<Blake2Hasher, _>(&merkle_leafs.clone());
    let notebook_number = self.next_notebook_number().await;
    let mut notary_state = self.state.lock().await;

    let tick = self.ticker.lock().await.current();
    for (leaf_index, balance_tip) in balance_tips.iter().enumerate() {
      let proof = merkle_proof::<Blake2Hasher, _, _>(merkle_leafs.clone(), leaf_index as u32);

      notary_state.balance_proofs.insert(
        (notebook_number, balance_tip.tip().into()),
        BalanceProof {
          notary_id: 1,
          balance: balance_tip.balance,
          account_origin: balance_tip.account_origin.clone(),
          notebook_number,
          tick,
          notebook_proof: Some(MerkleProof {
            proof: BoundedVec::truncate_from(proof.proof),
            number_of_leaves: proof.number_of_leaves as u32,
            leaf_index: proof.leaf_index as u32,
          }),
        },
      );

      notary_state.balance_tips.insert(
        LocalchainAccountId::new(balance_tip.account_id.clone(), balance_tip.account_type),
        BalanceTipResult {
          tick,
          balance_tip: balance_tip.tip().into(),
          notebook_number,
        },
      );
    }
    drop(notary_state);

    let changed_account_origins = BoundedVec::truncate_from(
      balance_tips
        .iter()
        .map(|x| x.account_origin.clone())
        .collect::<Vec<_>>(),
    );

    let notebook_header = NotebookHeader {
      version: 1,
      notary_id: 1,
      notebook_number,
      tick,
      tax: 0,
      domains: Default::default(),
      block_votes_count: 0,
      block_voting_power: 0,
      parent_secret: None,
      block_votes_root: H256([0u8; 32]),
      changed_account_origins,
      blocks_with_votes: Default::default(),
      secret_hash: H256::random(),
      chain_transfers: Default::default(),
      changed_accounts_root,
    };
    self
      .add_notebook_header(SignedNotebookHeader {
        header: notebook_header.clone(),
        signature: Signature::from_raw([0u8; 64]),
      })
      .await;
    notebook_header
  }
}
pub async fn create_pool() -> anyhow::Result<SqlitePool> {
  let pool = SqlitePool::connect_with(
    SqliteConnectOptions::from_str(":memory:")?
      .clone()
      .log_statements(LogLevelFilter::Debug.into()),
  )
  .await?;
  let _ = tracing_subscriber::FmtSubscriber::builder()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .try_init();
  sqlx::migrate!().run(&pool).await?;
  Ok(pool)
}
pub fn get_balance_tip(
  balance_change: BalanceChange,
  account_origin_uid: AccountOriginUid,
  account_origin_notebook_number: NotebookNumber,
) -> BalanceTip {
  BalanceTip {
    account_type: balance_change.account_type,
    account_id: balance_change.account_id,
    balance: balance_change.balance,
    channel_hold_note: balance_change.channel_hold_note.clone(),
    account_origin: AccountOrigin {
      account_uid: account_origin_uid,
      notebook_number: account_origin_notebook_number,
    },
    change_number: balance_change.change_number,
  }
}

pub fn mock_mainchain_transfer(address: &str, amount: u128) -> LocalchainTransfer {
  LocalchainTransfer {
    amount,
    notary_id: 1,
    expiration_tick: 1,
    address: address.to_string(),
    transfer_id: 1,
  }
}

#[async_trait]
impl LocalchainRpcServer for MockNotary {
  async fn notarize(
    &self,
    balance_changeset: NotarizationBalanceChangeset,
    block_votes: NotarizationBlockVotes,
    domains: NotarizationDomains,
  ) -> Result<BalanceChangeResult, ErrorObjectOwned> {
    let notebook_number = self.next_notebook_number().await;
    let tick = self.ticker.lock().await.current();
    self
      .add_notarization(
        notebook_number,
        Notarization {
          domains,
          block_votes,
          balance_changes: balance_changeset.clone(),
        },
      )
      .await
      .map_err(|e| ErrorObjectOwned::owned(-32000, e.to_string(), None::<String>))?;
    let mut state = self.state.lock().await;
    let mut new_origins = vec![];
    for change in balance_changeset {
      if change.change_number == 1 {
        let id = state.accounts.len() + 1;
        let account_id = LocalchainAccountId::new(change.account_id.clone(), change.account_type);
        let not = NewAccountOrigin {
          account_id: change.account_id,
          account_type: change.account_type,
          account_uid: id as u32,
        };
        state.accounts.insert(
          account_id,
          AccountOrigin {
            notebook_number,
            account_uid: id as u32,
          },
        );
        new_origins.push(not);
      }
    }

    Ok(BalanceChangeResult {
      new_account_origins: new_origins,
      notebook_number,
      tick: state
        .headers
        .get(&notebook_number)
        .map(|x| x.header.tick)
        .unwrap_or(tick),
    })
  }

  async fn get_origin(
    &self,
    account_id: AccountId,
    account_type: AccountType,
  ) -> Result<AccountOrigin, ErrorObjectOwned> {
    let state = self.state.lock().await;
    let origin = state
      .accounts
      .get(&LocalchainAccountId {
        account_id,
        account_type,
      })
      .ok_or_else(|| {
        ErrorObjectOwned::owned(
          -32000,
          "MockNotary account not found".to_string(),
          None::<String>,
        )
      })?;
    Ok(origin.clone())
  }

  async fn get_tip(
    &self,
    account_id: AccountId,
    account_type: AccountType,
  ) -> Result<BalanceTipResult, ErrorObjectOwned> {
    let state = self.state.lock().await;
    state
      .balance_tips
      .get(&LocalchainAccountId::new(account_id, account_type))
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
    state
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
    state
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
    state.metadata.clone().ok_or_else(|| {
      ErrorObjectOwned::owned(
        -32000,
        "MockNotary metadata not set".to_string(),
        None::<String>,
      )
    })
  }

  async fn get_header_download_url(
    &self,
    notebook_number: NotebookNumber,
  ) -> Result<String, ErrorObjectOwned> {
    Ok(get_header_url(
      &self.archive_host,
      self.notary_id,
      notebook_number,
    ))
  }

  async fn get_notebook_download_url(
    &self,
    notebook_number: NotebookNumber,
  ) -> Result<String, ErrorObjectOwned> {
    Ok(get_notebook_url(
      &self.archive_host,
      self.notary_id,
      notebook_number,
    ))
  }

  async fn subscribe_headers(
    &self,
    subscription_sink: PendingSubscriptionSink,
  ) -> SubscriptionResult {
    let stream = self.header_channel.1.subscribe(1_000);
    pipe_from_stream_and_drop(subscription_sink, stream, |a| {
      let a = NotebookSubscriptionBroadcast::build(a, &self.archive_host);
      SubscriptionMessage::from_json(&a).map_err(Into::into)
    })
    .await
    .map_err(Into::into)
  }
}

#[async_trait]
impl SystemRpcServer for MockNotary {
  async fn get_archive_base_url(&self) -> Result<String, ErrorObjectOwned> {
    Ok(self.archive_host.clone())
  }
  async fn health(&self) -> Result<(), ErrorObjectOwned> {
    Ok(())
  }
}
