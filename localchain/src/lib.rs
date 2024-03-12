#![deny(clippy::all)]
#![feature(async_closure)]
#[macro_use]
extern crate napi_derive;

use std::env;
use directories::BaseDirs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::*;
use sqlx::Sqlite;
use sqlx::{migrate::MigrateDatabase, SqlitePool};
use tokio::sync::Mutex;

pub use accounts::*;
pub use balance_changes::*;
pub use balance_sync::*;
pub use constants::*;
pub use data_domain::*;
pub use keystore::Keystore;
pub use mainchain_client::*;
pub use notary_client::*;
pub use open_escrows::*;
use ulx_primitives::tick::Ticker;

mod accounts;
mod balance_change_builder;
mod balance_changes;
mod balance_sync;
mod data_domain;
pub mod keystore;
mod mainchain_client;
mod notarization_builder;
mod notarization_tracker;
mod notary_client;
mod open_escrows;

pub mod embedded_keystore;

use crate::keystore::KeystorePasswordOption;
pub use embedded_keystore::CryptoScheme;

pub mod cli;
pub mod constants;
mod file_transfer;
pub mod macros;
mod overview;
#[cfg(test)]
pub(crate) mod test_utils;
pub mod transactions;

#[napi(custom_finalize)]
pub struct Localchain {
  pub(crate) db: SqlitePool,
  pub(crate) ticker: TickerRef,
  pub(crate) mainchain_client: Arc<Mutex<Option<MainchainClient>>>,
  pub(crate) notary_clients: NotaryClients,
  pub(crate) keystore: Keystore,
  pub path: String,
}

#[napi(object)]
pub struct LocalchainConfig {
  pub path: String,
  pub mainchain_url: String,
  pub ntp_pool_url: Option<String>,
  pub keystore_password: Option<KeystorePasswordOption>,
}

impl ObjectFinalize for Localchain {
  fn finalize(self, _: Env) -> Result<()> {
    spawn(async move {
      let _ = self.close().await;
    });
    Ok(())
  }
}

#[napi]
impl Localchain {
  pub async fn create_db(path: String) -> Result<SqlitePool> {
    if !Sqlite::database_exists(&path).await.unwrap_or(false) {
      Sqlite::create_database(&path)
        .await
        .map_err(|e| Error::from_reason(format!("Error creating database {}", e.to_string())))?;
    }

    let db = SqlitePool::connect(&path).await.map_err(to_js_error)?;
    sqlx::migrate!()
      .run(&db)
      .await
      .map_err(|e| Error::from_reason(format!("Error migrating database {}", e.to_string())))?;
    Ok(db)
  }

  #[napi(factory)]
  pub async fn load(config: LocalchainConfig) -> Result<Localchain> {
    Self::config_logs();
    tracing::info!("Loading localchain at {:?}", config.path);

    let LocalchainConfig {
      keystore_password,
      path,
      mainchain_url,
      ntp_pool_url,
    } = config;

    let db = Self::create_db(path.clone()).await?;

    let mainchain_client = MainchainClient::connect(mainchain_url, 30_000)
      .await
      .map_err(to_js_error)?;
    let mut ticker = mainchain_client.get_ticker().await.map_err(to_js_error)?;
    if let Some(ntp_pool_url) = ntp_pool_url {
      ticker
        .lookup_ntp_offset(&ntp_pool_url)
        .await
        .map_err(to_js_error)?;
    }

    let mainchain_mutex = Arc::new(Mutex::new(Some(mainchain_client.clone())));
    let keystore = Keystore::new(db.clone());
    if let Some(password_option) = keystore_password {
      keystore
        .unlock(Some(password_option))
        .await
        .map_err(to_js_error)?;
    } else {
      // might not unlock, but try in case
      let _ = keystore.unlock(None).await;
    }

    Ok(Localchain {
      db,
      path: path,
      ticker: TickerRef { ticker },
      mainchain_client: mainchain_mutex.clone(),
      notary_clients: NotaryClients::from(mainchain_mutex.clone()),
      keystore,
    })
  }

  #[napi(factory)]
  pub async fn load_without_mainchain(
    path: String,
    ticker_config: TickerConfig,
    keystore_password: Option<KeystorePasswordOption>,
  ) -> Result<Localchain> {
    Self::config_logs();
    tracing::info!("Loading localchain at {:?}", path);

    let mut ticker = Ticker::new(
      ticker_config.tick_duration_millis as u64,
      ticker_config.genesis_utc_time as u64,
    );
    if let Some(ntp_pool_url) = ticker_config.ntp_pool_url {
      ticker
        .lookup_ntp_offset(&ntp_pool_url)
        .await
        .map_err(to_js_error)?;
    }
    let db = Self::create_db(path.clone()).await?;

    let mainchain_mutex = Arc::new(Mutex::new(None));
    let keystore = Keystore::new(db.clone());
    if let Some(password_option) = keystore_password {
      keystore
        .unlock(Some(password_option))
        .await
        .map_err(to_js_error)?;
    }

    Ok(Localchain {
      db,
      path: path,
      ticker: TickerRef { ticker },
      mainchain_client: mainchain_mutex.clone(),
      notary_clients: NotaryClients::from(mainchain_mutex),
      keystore,
    })
  }

  #[napi]
  pub async fn attach_mainchain(&self, mainchain_client: &MainchainClient) {
    let mut mainchain_mutex = self.mainchain_client.lock().await;
    *mainchain_mutex = Some(mainchain_client.clone());
  }

  #[napi]
  pub async fn close(&self) -> Result<()> {
    tracing::trace!("Closing Localchain");
    let mut mainchain_client = self.mainchain_client.lock().await;
    if let Some(mainchain_client) = mainchain_client.take() {
      mainchain_client.close().await.map_err(to_js_error)?;
    }
    self.notary_clients.close().await;
    if !self.db.is_closed() {
      self.db.close().await;
    }
    tracing::trace!("Closed Localchain");
    Ok(())
  }

  #[napi]
  pub async fn account_overview(&self) -> Result<overview::LocalchainOverviewJs> {
    overview::OverviewStore::new(self.db.clone()).get_js().await
  }

  #[napi]
  pub fn get_default_dir() -> String {
    let base_dirs = BaseDirs::new().unwrap();
    let data_local_dir = base_dirs.data_local_dir();
    PathBuf::from(data_local_dir)
      .join("ulixee")
      .join("localchain")
      .to_str()
      .unwrap()
      .to_string()
  }

  #[napi]
  pub fn get_default_path() -> String {
    PathBuf::from(Self::get_default_dir())
      .join("primary.db")
      .to_str()
      .unwrap()
      .to_string()
  }

  #[napi(getter)]
  pub async fn address(&self) -> Result<String> {
    Ok(self.accounts().deposit_account_js(None).await?.address)
  }

  #[napi(getter)]
  pub fn current_tick(&self) -> u32 {
    self.ticker.current()
  }

  pub fn duration_to_next_tick(&self) -> Duration {
    self.ticker.duration_to_next_tick()
  }

  #[napi(getter)]
  pub fn ticker(&self) -> TickerRef {
    self.ticker.clone()
  }

  #[napi(getter)]
  pub fn keystore(&self) -> Keystore {
    self.keystore.clone()
  }

  #[napi(getter)]
  pub async fn mainchain_client(&self) -> Option<MainchainClient> {
    let mainchain_client = self.mainchain_client.lock().await;
    mainchain_client.clone()
  }

  #[napi(getter)]
  pub fn notary_clients(&self) -> NotaryClients {
    self.notary_clients.clone()
  }

  #[napi(getter)]
  pub fn accounts(&self) -> accounts::AccountStore {
    accounts::AccountStore::new(self.db.clone())
  }

  #[napi(getter)]
  pub fn balance_changes(&self) -> balance_changes::BalanceChangeStore {
    balance_changes::BalanceChangeStore::new(self.db.clone())
  }

  #[napi(getter)]
  pub fn data_domains(&self) -> data_domain::DataDomainStore {
    data_domain::DataDomainStore::new(self.db.clone())
  }

  #[napi(getter)]
  pub fn open_escrows(&self) -> open_escrows::OpenEscrowsStore {
    open_escrows::OpenEscrowsStore::new(
      self.db.clone(),
      self.ticker.clone(),
      &self.notary_clients,
      &self.keystore,
    )
  }

  #[napi(getter)]
  pub fn balance_sync(&self) -> balance_sync::BalanceSync {
    balance_sync::BalanceSync::new(&self)
  }
  #[napi(getter)]
  pub fn transactions(&self) -> transactions::Transactions {
    transactions::Transactions::new(
      self.db.clone(),
      self.ticker.clone(),
      &self.notary_clients,
      &self.keystore,
    )
  }

  #[napi]
  pub fn begin_change(&self) -> notarization_builder::NotarizationBuilder {
    notarization_builder::NotarizationBuilder::new(
      self.db.clone(),
      self.notary_clients.clone(),
      self.keystore.clone(),
    )
  }

  fn config_logs() {
    let _ = tracing_subscriber::fmt()
      .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
      // RUST_LOG=trace,soketto=info,sqlx=info,jsonrpsee_core=info
      .try_init();

    env::set_var("RUST_BACKTRACE", "1");
  }
}

pub(crate) fn to_js_error(e: impl std::fmt::Display) -> Error {
  Error::from_reason(format!("{}", e))
}

#[napi]
#[derive(Clone)]
pub struct TickerRef {
  pub(crate) ticker: Ticker,
}

#[napi]
impl TickerRef {
  #[napi(getter)]
  pub fn current(&self) -> u32 {
    self.ticker.current()
  }
  #[napi]
  pub fn tick_for_time(&self, timestamp_millis: i64) -> u32 {
    self.ticker.tick_for_time(timestamp_millis as u64)
  }

  #[napi]
  pub fn time_for_tick(&self, tick: u32) -> u64 {
    self.ticker.time_for_tick(tick)
  }
  #[napi]
  pub fn millis_to_next_tick(&self) -> u64 {
    self.duration_to_next_tick().as_millis() as u64
  }

  pub fn duration_to_next_tick(&self) -> Duration {
    self.ticker.duration_to_next_tick()
  }
}

#[napi(object)]
pub struct TickerConfig {
  pub tick_duration_millis: i64,
  pub genesis_utc_time: i64,
  pub ntp_pool_url: Option<String>,
}
