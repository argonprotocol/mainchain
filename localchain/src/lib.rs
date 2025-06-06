#[cfg(feature = "napi")]
#[macro_use]
extern crate napi_derive;

use std::fs::create_dir_all;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub use accounts::*;
use anyhow::{Context, anyhow};
use argon_primitives::Chain;
use argon_primitives::tick::{Tick, Ticker};
pub use balance_changes::*;
pub use balance_sync::*;
pub use constants::*;
use directories::BaseDirs;
pub use domain::*;
pub use embedded_keystore::CryptoScheme;
pub use error::Error;
pub use keystore::Keystore;
use lazy_static::lazy_static;
pub use mainchain_client::*;
pub use notary_client::*;
pub use open_channel_holds::*;
use parking_lot::RwLock;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Executor, Sqlite};
use sqlx::{SqlitePool, migrate::MigrateDatabase};
use tokio::sync::RwLock as AsyncRwLock;
use tracing_subscriber::{EnvFilter, fmt};

use crate::cli::EmbeddedKeyPassword;
use crate::mainchain_transfer::MainchainTransferStore;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
#[cfg(feature = "uniffi")]
argon_primitives::uniffi_reexport_scaffolding!();

mod accounts;
mod balance_change_builder;
mod balance_changes;
mod balance_sync;
mod domain;
pub mod keystore;
mod mainchain_client;
mod mainchain_transfer;
mod notarization_builder;
mod notarization_tracker;
mod notary_client;
mod open_channel_holds;

pub mod embedded_keystore;

mod argon_file;
#[cfg(test)]
mod balance_sync_tests;
pub mod cli;
pub mod constants;
mod error;
pub mod macros;
mod overview;
#[cfg(test)]
pub(crate) mod test_utils;
pub mod transactions;

lazy_static! {
  static ref DEFAULT_DATA_DIR: Arc<RwLock<String>> = {
    let base_dirs = BaseDirs::new().unwrap();
    let data_local_dir = base_dirs.data_local_dir();
    let path = PathBuf::from(data_local_dir)
      .join("argon")
      .join("localchain")
      .to_str()
      .unwrap()
      .to_string();
    Arc::new(RwLock::new(path))
  };
}

#[cfg_attr(feature = "napi", napi(custom_finalize))]
pub struct Localchain {
  pub(crate) db: SqlitePool,
  pub(crate) ticker: TickerRef,
  pub(crate) mainchain_client: Arc<AsyncRwLock<Option<MainchainClient>>>,
  pub(crate) notary_clients: NotaryClients,
  pub(crate) keystore: Keystore,
  pub path: String,
}

pub struct LocalchainConfig {
  pub path: String,
  pub mainchain_url: String,
  pub ntp_pool_url: Option<String>,
  pub keystore_password: Option<EmbeddedKeyPassword>,
}
pub type Result<T> = anyhow::Result<T, Error>;

impl Localchain {
  pub async fn create_db(path: String) -> Result<SqlitePool> {
    let db_path = PathBuf::from(path.clone());
    if let Some(dir) = db_path.parent() {
      if !dir.exists() {
        create_dir_all(dir).with_context(|| {
          format!(
            "Could not create the parent directory ({}) for your localchain",
            dir.display()
          )
        })?;
      }
    }

    if !Sqlite::database_exists(&path).await.unwrap_or(false) {
      Sqlite::create_database(&path).await?;
    }

    let options = SqliteConnectOptions::from_str(&path)?
      .journal_mode(SqliteJournalMode::Wal)
      .optimize_on_close(true, None);

    let db = SqlitePoolOptions::new().connect_with(options).await?;
    sqlx::migrate!()
      .run(&db)
      .await
      .map_err(|e| Error::Database(e.into()))?;
    Ok(db)
  }

  pub async fn confirm_chain(db: &SqlitePool, mainchain_client: &MainchainClient) -> Result<()> {
    let identity = mainchain_client.get_chain_identity().await?;
    let existing = sqlx::query!("SELECT * FROM mainchain_identity")
      .fetch_optional(db)
      .await?;
    if let Some(existing) = existing {
      let existing_chain: Chain = existing.chain.try_into()?;
      if existing_chain != identity.chain {
        bail!(
          "The connected mainchain chain ({}) does not match this Localchain ({})",
          identity.chain,
          existing_chain
        );
      }
      if existing.genesis_hash != identity.genesis_hash.as_ref().to_vec() {
        bail!(
          "The connected mainchain genesis hash ({}) does not match this Localchain ({})",
          identity.genesis_hash,
          hex::encode(existing.genesis_hash)
        );
      }
    } else {
      let identity_chain = identity.chain.to_string();
      let genesis_hash = identity.genesis_hash.as_ref().to_vec();
      sqlx::query!(
        "INSERT INTO mainchain_identity (chain, genesis_hash) VALUES (?, ?)",
        identity_chain,
        genesis_hash
      )
      .execute(db)
      .await?;
    }
    Ok(())
  }

  pub async fn load(config: LocalchainConfig) -> Result<Localchain> {
    Self::config_logs();
    tracing::info!("Loading localchain at {:?}", config.path);

    let LocalchainConfig {
      keystore_password,
      path,
      mainchain_url,
      ntp_pool_url,
    } = config;

    let db = Self::create_db(path.clone())
      .await
      .with_context(|| format!("Creating database at {}", path.clone()))?;

    let mainchain_client = MainchainClient::connect(mainchain_url.clone(), 30_000)
      .await
      .with_context(|| format!("Connecting to mainchain at ({})", mainchain_url.clone()))?;
    let mut ticker = mainchain_client.get_ticker().await?;
    if let Some(ntp_pool_url) = ntp_pool_url {
      ticker.lookup_ntp_offset(&ntp_pool_url).await?;
    }
    Self::confirm_chain(&db, &mainchain_client).await?;
    let mainchain_mutex = Arc::new(AsyncRwLock::new(Some(mainchain_client.clone())));
    let keystore = Keystore::new(db.clone());
    if let Some(password_option) = keystore_password {
      keystore.unlock(Some(password_option)).await?;
    } else {
      // might not unlock, but try in case
      let _ = keystore.unlock(None).await;
    }

    Ok(Localchain {
      db,
      path,
      ticker: TickerRef::new(ticker),
      mainchain_client: mainchain_mutex.clone(),
      notary_clients: NotaryClients::from(mainchain_mutex.clone()),
      keystore,
    })
  }

  pub async fn load_without_mainchain(
    path: String,
    ticker_config: TickerConfig,
    keystore_password: Option<EmbeddedKeyPassword>,
  ) -> Result<Localchain> {
    Self::config_logs();
    tracing::info!("Loading localchain at {:?}", path);

    let mut ticker = Ticker::new(
      ticker_config.tick_duration_millis as u64,
      ticker_config.channel_hold_expiration_ticks as Tick,
    );
    if let Some(ntp_pool_url) = ticker_config.ntp_pool_url {
      ticker.lookup_ntp_offset(&ntp_pool_url).await?;
    }
    let db = Self::create_db(path.clone()).await?;

    let mainchain_mutex = Arc::new(AsyncRwLock::new(None));
    let keystore = Keystore::new(db.clone());
    if let Some(password_option) = keystore_password {
      keystore.unlock(Some(password_option)).await?;
    } else {
      // might not unlock, but try in case
      let _ = keystore.unlock(None).await;
    }

    Ok(Localchain {
      db,
      path,
      ticker: TickerRef::new(ticker),
      mainchain_client: mainchain_mutex.clone(),
      notary_clients: NotaryClients::from(mainchain_mutex),
      keystore,
    })
  }

  pub async fn attach_mainchain(&self, mainchain_client: &MainchainClient) -> Result<()> {
    Self::confirm_chain(&self.db, mainchain_client).await?;

    self
      .mainchain_client
      .write()
      .await
      .replace(mainchain_client.clone());
    Ok(())
  }

  pub async fn update_ticker(&self, ntp_sync_pool_url: Option<String>) -> Result<()> {
    let Some(ref mainchain_client) = *(self.mainchain_client.read().await) else {
      bail!("No mainchain client attached");
    };
    let mut ticker = mainchain_client.get_ticker().await?;
    if let Some(ntp_pool_url) = ntp_sync_pool_url {
      ticker.lookup_ntp_offset(&ntp_pool_url).await?;
    }
    self.ticker.set(ticker);
    Ok(())
  }

  pub async fn close(&self) -> Result<()> {
    tracing::trace!("Closing Localchain");
    {
      if let Some(mainchain_client) = self.mainchain_client.write().await.take() {
        mainchain_client.close().await?;
      }
    }
    self.notary_clients.close().await;
    if !self.db.is_closed() {
      self.db.execute("PRAGMA wal_checkpoint(FULL);").await?;
      self.db.close().await;
    }
    tracing::trace!("Closed Localchain");
    Ok(())
  }

  pub async fn account_overview(&self) -> Result<overview::LocalchainOverview> {
    overview::OverviewStore::new(self.db.clone(), self.name(), self.mainchain_client.clone())
      .get()
      .await
  }

  pub fn get_default_dir() -> String {
    DEFAULT_DATA_DIR.read().clone()
  }

  pub fn set_default_dir(value: String) {
    *DEFAULT_DATA_DIR.write() = value;
  }

  pub fn get_default_path() -> String {
    PathBuf::from(Self::get_default_dir())
      .join("primary.db")
      .to_str()
      .unwrap()
      .to_string()
  }

  pub async fn address(&self) -> Result<String> {
    Ok(self.accounts().deposit_account(None).await?.address)
  }

  pub fn name(&self) -> String {
    PathBuf::from(&self.path)
      .file_stem()
      .unwrap()
      .to_str()
      .unwrap()
      .to_string()
  }

  pub fn current_tick(&self) -> Tick {
    self.ticker.current()
  }

  pub fn duration_to_next_tick(&self) -> Duration {
    self.ticker.duration_to_next_tick()
  }

  pub fn ticker(&self) -> TickerRef {
    self.ticker.clone()
  }

  pub fn keystore(&self) -> Keystore {
    self.keystore.clone()
  }

  pub async fn mainchain_client(&self) -> Option<MainchainClient> {
    self.mainchain_client.read().await.clone()
  }

  pub fn mainchain_transfers(&self) -> MainchainTransferStore {
    MainchainTransferStore::new(
      self.db.clone(),
      self.mainchain_client.clone(),
      self.keystore.clone(),
    )
  }

  pub fn notary_clients(&self) -> NotaryClients {
    self.notary_clients.clone()
  }

  pub fn accounts(&self) -> accounts::AccountStore {
    accounts::AccountStore::new(self.db.clone())
  }

  pub fn balance_changes(&self) -> balance_changes::BalanceChangeStore {
    balance_changes::BalanceChangeStore::new(self.db.clone())
  }

  pub fn domains(&self) -> domain::DomainStore {
    domain::DomainStore::new(self.db.clone())
  }

  pub fn open_channel_holds(&self) -> open_channel_holds::OpenChannelHoldsStore {
    open_channel_holds::OpenChannelHoldsStore::new(
      self.db.clone(),
      self.ticker.clone(),
      &self.notary_clients,
      &self.keystore,
    )
  }

  pub fn balance_sync(&self) -> balance_sync::BalanceSync {
    balance_sync::BalanceSync::new(self)
  }

  pub fn transactions(&self) -> transactions::Transactions {
    transactions::Transactions::new(
      self.db.clone(),
      self.ticker.clone(),
      &self.notary_clients,
      &self.keystore,
    )
  }

  pub fn begin_change(&self) -> notarization_builder::NotarizationBuilder {
    notarization_builder::NotarizationBuilder::new(
      self.db.clone(),
      self.notary_clients.clone(),
      self.keystore.clone(),
      self.ticker.clone(),
    )
  }

  pub fn config_logs() {
    color_backtrace::install();
    // RUST_LOG=trace,soketto=info,sqlx=info,jsonrpsee_core=info
    let filter = EnvFilter::try_from_default_env()
      .or_else(|_| EnvFilter::try_new("info,secp256k1=warn"))
      .expect("Failed to parse RUST_LOG environment variable");

    let trace = fmt::Subscriber::builder()
      .with_env_filter(filter)
      .with_line_number(true)
      .with_file(true)
      .with_thread_names(true);
    #[cfg(feature = "uniffi")]
    let trace = trace.with_ansi(false);

    let _ = trace.try_init();
  }
}

#[cfg(feature = "uniffi")]
pub mod uniffi_ext {
  use super::{balance_sync, transactions};
  use crate::CryptoScheme;
  use crate::MainchainClient;
  use crate::cli::EmbeddedKeyPassword;
  use crate::error::UniffiResult;

  #[derive(uniffi::Record)]
  pub struct LocalchainConfig {
    pub path: String,
    pub mainchain_url: String,
    pub ntp_pool_url: Option<String>,
    pub keystore_password: Option<String>,
  }

  impl Into<super::LocalchainConfig> for LocalchainConfig {
    fn into(self) -> super::LocalchainConfig {
      super::LocalchainConfig {
        path: self.path,
        mainchain_url: self.mainchain_url,
        ntp_pool_url: self.ntp_pool_url,
        keystore_password: self.keystore_password.map(|a| EmbeddedKeyPassword {
          key_password: Some(a),
          key_password_filename: None,
          key_password_interactive: false,
        }),
      }
    }
  }

  #[derive(uniffi::Object)]
  pub struct Localchain {
    pub path: String,
    pub name: String,
    inner: super::Localchain,
  }

  #[uniffi::export(async_runtime = "tokio")]
  impl Localchain {
    #[uniffi::constructor(name = "newLive")]
    pub async fn load_uniffi(config: LocalchainConfig) -> UniffiResult<Self> {
      let inner = super::Localchain::load(config.into()).await?;
      Ok(Self {
        path: inner.path.clone(),
        name: inner.name(),
        inner,
      })
    }

    #[uniffi::constructor(name = "newWithoutMainchain")]
    pub async fn load_without_mainchain_uniffi(
      path: String,
      ticker_config: crate::TickerConfig,
      keystore_password: Option<String>,
    ) -> UniffiResult<Self> {
      let keystore_password = keystore_password.map(|a| EmbeddedKeyPassword {
        key_password: Some(a),
        key_password_filename: None,
        key_password_interactive: false,
      });
      let inner =
        super::Localchain::load_without_mainchain(path, ticker_config, keystore_password).await?;
      Ok(Self {
        path: inner.path.clone(),
        name: inner.name(),
        inner,
      })
    }

    #[uniffi::method(name = "isConnectedToMainchain")]
    pub async fn is_connected_to_mainchain_napi(&self) -> bool {
      self.inner.mainchain_client().await.is_some()
    }

    #[uniffi::method(name = "connectMainchain", default(timeout_millis = 30_000))]
    pub async fn connect_mainchain_uniffi(
      &self,
      mainchain_url: String,
      timeout_millis: u64,
    ) -> UniffiResult<()> {
      let mainchain_client = MainchainClient::connect(mainchain_url, timeout_millis as i64).await?;
      self.inner.attach_mainchain(&mainchain_client).await?;
      self.inner.update_ticker(None).await?;
      Ok(())
    }

    #[uniffi::method(name = "close")]
    pub async fn close_uniffi(&self) -> UniffiResult<()> {
      Ok(self.inner.close().await?)
    }

    #[uniffi::method(name = "accountOverview")]
    pub async fn account_overview_uniffi(
      &self,
    ) -> UniffiResult<crate::overview::uniffi_ext::LocalchainOverview> {
      Ok(self.inner.account_overview().await?.into())
    }

    #[uniffi::method(name = "address")]
    pub async fn address_uniffi(&self) -> UniffiResult<String> {
      Ok(self.inner.address().await?)
    }

    #[uniffi::method(name = "currentTick")]
    pub fn current_tick_uniffi(&self) -> u64 {
      self.inner.current_tick()
    }

    #[uniffi::method(name = "durationToNextTick")]
    pub fn duration_to_next_tick_uniffi(&self) -> u64 {
      self.inner.duration_to_next_tick().as_millis() as u64
    }

    #[uniffi::method(name = "useAccount")]
    pub async fn use_account(
      &self,
      suri: Option<String>,
      password: Option<String>,
      crypto_scheme: Option<CryptoScheme>,
    ) -> UniffiResult<String> {
      if let Some(deposit_account) = self.inner.accounts().deposit_account(None).await.ok() {
        return Ok(deposit_account.address);
      }
      let keystore = self.inner.keystore();
      let password = password.map(|a| EmbeddedKeyPassword {
        key_password: Some(a),
        key_password_filename: None,
        key_password_interactive: false,
      });
      if let Some(suri) = suri {
        Ok(
          keystore
            .import_suri(
              suri,
              crypto_scheme.unwrap_or(CryptoScheme::Sr25519),
              password,
            )
            .await?,
        )
      } else {
        Ok(keystore.bootstrap(crypto_scheme, password).await?)
      }
    }

    #[uniffi::method(name = "sync")]
    pub async fn sync_uniffi(&self) -> UniffiResult<balance_sync::uniffi_ext::BalanceSyncResult> {
      let balance_sync = self.inner.balance_sync();
      Ok(balance_sync.sync(None).await?.into())
    }

    #[uniffi::method(name = "transactions")]
    pub fn transactions_uniffi(&self) -> transactions::Transactions {
      self.inner.transactions()
    }
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use crate::error::NapiOk;
  use napi::bindgen_prelude::*;

  use super::*;
  pub use crate::TickerRef;
  use crate::keystore::napi_ext::KeystorePasswordOption;

  impl ObjectFinalize for Localchain {
    fn finalize(self, _: Env) -> napi::Result<()> {
      spawn(async move {
        let _ = self.close().await;
      });
      Ok(())
    }
  }
  #[napi(object, js_name = "LocalchainConfig")]
  pub struct LocalchainConfigJs {
    pub path: String,
    pub mainchain_url: String,
    pub ntp_pool_url: Option<String>,
    pub keystore_password: Option<KeystorePasswordOption>,
  }

  impl Into<LocalchainConfig> for LocalchainConfigJs {
    fn into(self) -> LocalchainConfig {
      LocalchainConfig {
        path: self.path,
        mainchain_url: self.mainchain_url,
        ntp_pool_url: self.ntp_pool_url,
        keystore_password: self.keystore_password.map(|v| v.into()),
      }
    }
  }

  #[napi]
  impl Localchain {
    #[napi(factory, js_name = "load", ts_args_type = "config: LocalchainConfig")]
    pub async fn load_napi(config: LocalchainConfigJs) -> napi::Result<Localchain> {
      Localchain::load(config.into()).await.napi_ok()
    }
    #[napi(factory, js_name = "loadWithoutMainchain")]
    pub async fn load_without_mainchain_napi(
      path: String,
      ticker_config: TickerConfig,
      keystore_password: Option<KeystorePasswordOption>,
    ) -> napi::Result<Localchain> {
      Localchain::load_without_mainchain(path, ticker_config, keystore_password.map(Into::into))
        .await
        .napi_ok()
    }

    #[napi(js_name = "attachMainchain")]
    pub async fn attach_mainchain_napi(
      &self,
      mainchain_client: &MainchainClient,
    ) -> napi::Result<()> {
      self.attach_mainchain(mainchain_client).await.napi_ok()
    }

    #[napi(js_name = "updateTicker")]
    pub async fn update_ticker_napi(&self, ntp_sync_pool_url: Option<String>) -> napi::Result<()> {
      self.update_ticker(ntp_sync_pool_url).await.napi_ok()
    }

    #[napi(js_name = "close")]
    pub async fn close_napi(&self) -> napi::Result<()> {
      self.close().await.napi_ok()
    }

    #[napi(js_name = "accountOverview")]
    pub async fn account_overview_napi(
      &self,
    ) -> napi::Result<crate::overview::napi_ext::LocalchainOverview> {
      self.account_overview().await.map(Into::into).napi_ok()
    }

    #[napi(js_name = "getDefaultDir")]
    pub fn get_default_dir_napi() -> String {
      Localchain::get_default_dir()
    }

    #[napi(js_name = "setDefaultDir")]
    pub fn set_default_dir_napi(value: String) {
      Localchain::set_default_dir(value)
    }

    #[napi(js_name = "getDefaultPath")]
    pub fn get_default_path_napi() -> String {
      Localchain::get_default_path()
    }

    #[napi(getter, js_name = "address")]
    pub async fn address_napi(&self) -> napi::Result<String> {
      self.address().await.napi_ok()
    }

    #[napi(js_name = "name", getter)]
    pub fn name_napi(&self) -> String {
      self.name()
    }

    #[napi(js_name = "currentTick", getter)]
    pub fn current_tick_napi(&self) -> i64 {
      self.current_tick() as i64
    }

    #[napi(js_name = "durationToNextTick")]
    pub fn duration_to_next_tick_napi(&self) -> u64 {
      self.duration_to_next_tick().as_millis() as u64
    }

    #[napi(js_name = "ticker", getter)]
    pub fn ticker_napi(&self) -> TickerRef {
      self.ticker()
    }

    #[napi(js_name = "keystore", getter)]
    pub fn keystore_napi(&self) -> Keystore {
      self.keystore()
    }

    #[napi(js_name = "mainchainClient", getter)]
    pub async fn mainchain_client_napi(&self) -> Option<MainchainClient> {
      self.mainchain_client().await
    }

    #[napi(js_name = "mainchainTransfers", getter)]
    pub fn mainchain_transfers_napi(&self) -> MainchainTransferStore {
      self.mainchain_transfers()
    }

    #[napi(js_name = "notaryClients", getter)]
    pub fn notary_clients_napi(&self) -> NotaryClients {
      self.notary_clients()
    }

    #[napi(js_name = "accounts", getter)]
    pub fn accounts_napi(&self) -> AccountStore {
      self.accounts()
    }

    #[napi(js_name = "balanceChanges", getter)]
    pub fn balance_changes_napi(&self) -> balance_changes::BalanceChangeStore {
      self.balance_changes()
    }

    #[napi(js_name = "domains", getter)]
    pub fn domains_napi(&self) -> domain::DomainStore {
      self.domains()
    }

    #[napi(js_name = "openChannelHolds", getter)]
    pub fn open_channel_holds_napi(&self) -> open_channel_holds::OpenChannelHoldsStore {
      self.open_channel_holds()
    }

    #[napi(js_name = "balanceSync", getter)]
    pub fn balance_sync_napi(&self) -> balance_sync::BalanceSync {
      Localchain::balance_sync(self)
    }

    #[napi(js_name = "transactions", getter)]
    pub fn transactions_napi(&self) -> transactions::Transactions {
      self.transactions()
    }

    #[napi(js_name = "beginChange")]
    pub fn begin_change_napi(&self) -> notarization_builder::NotarizationBuilder {
      self.begin_change()
    }
  }

  #[napi]
  impl TickerRef {
    #[napi(getter, js_name = "current")]
    pub fn current_napi(&self) -> i64 {
      self.current() as i64
    }

    #[napi(js_name = "tickForTime")]
    pub fn tick_for_time_napi(&self, timestamp_millis: i64) -> u64 {
      self.tick_for_time(timestamp_millis)
    }

    #[napi(js_name = "timeForTick")]
    pub fn time_for_tick_napi(&self, tick: i64) -> i64 {
      self.time_for_tick(tick as u64) as i64
    }

    #[napi(js_name = "millisToNextTick")]
    pub fn millis_to_next_tick_napi(&self) -> i64 {
      self.millis_to_next_tick() as i64
    }

    #[napi(js_name = "channelHoldExpirationTicks", getter)]
    pub fn channel_hold_expiration_ticks_napi(&self) -> i64 {
      self.channel_hold_expiration_ticks() as i64
    }
  }
}

#[derive(Clone)]
#[cfg_attr(feature = "napi", napi)]
pub struct TickerRef {
  pub(crate) ticker: Arc<RwLock<Ticker>>,
}

impl From<Ticker> for TickerRef {
  fn from(ticker: Ticker) -> Self {
    Self::new(ticker)
  }
}

impl TickerRef {
  pub fn new(ticker: Ticker) -> Self {
    Self {
      ticker: Arc::new(RwLock::new(ticker)),
    }
  }

  pub fn set(&self, ticker: Ticker) {
    *self.ticker.write() = ticker;
  }

  pub fn current(&self) -> Tick {
    self.ticker.read().current()
  }

  pub fn tick_for_time(&self, timestamp_millis: i64) -> Tick {
    self.ticker.read().tick_for_time(timestamp_millis as u64)
  }

  pub fn time_for_tick(&self, tick: Tick) -> u64 {
    self.ticker.read().time_for_tick(tick)
  }

  pub fn millis_to_next_tick(&self) -> u64 {
    self.duration_to_next_tick().as_millis() as u64
  }

  pub fn duration_to_next_tick(&self) -> Duration {
    self.ticker.read().duration_to_next_tick()
  }

  pub fn channel_hold_expiration_ticks(&self) -> Tick {
    self.ticker.read().channel_hold_expiration_ticks
  }
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TickerConfig {
  pub tick_duration_millis: i64,
  pub channel_hold_expiration_ticks: i64,
  pub ntp_pool_url: Option<String>,
}
