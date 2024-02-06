#![deny(clippy::all)]
#![feature(async_closure)]
#[macro_use]
extern crate napi_derive;

use napi::bindgen_prelude::*;
use sc_service::BasePath;
use sqlx::Sqlite;
use sqlx::{migrate::MigrateDatabase, SqlitePool};
use std::time::Duration;
use ulx_primitives::tick::Ticker;

pub use accounts::*;
pub use balance_changes::*;
pub use balance_sync::*;
pub use data_domain::*;
pub use mainchain_client::*;
pub use notary_client::*;
pub use open_escrows::*;
pub use signer::Signer;

mod accounts;
mod balance_change_builder;
mod balance_changes;
mod balance_sync;
mod data_domain;
mod mainchain_client;
mod notarization_builder;
mod notarization_tracker;
mod notary_client;
mod open_escrows;
pub mod signer;

#[cfg(test)]
pub(crate) mod test_utils;

#[napi(custom_finalize)]
pub struct Localchain {
  pub(crate) db: SqlitePool,
  pub(crate) ticker: TickerRef,
  pub(crate) mainchain_client: MainchainClient,
  pub(crate) notary_clients: NotaryClients,
}

#[napi(object)]
pub struct LocalchainConfig {
  pub path: String,
  pub mainchain_url: String,
  pub ntp_pool_url: Option<String>,
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
  #[napi(factory)]
  pub async fn load(config: LocalchainConfig) -> Result<Localchain> {
    let _ = tracing_subscriber::fmt()
      .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
      // RUST_LOG=trace,soketto=info,sqlx=info,jsonrpsee_core=info
      .try_init();
    tracing::info!("Loading localchain at {:?}", config.path);

    let LocalchainConfig {
      path,
      mainchain_url,
      ntp_pool_url,
    } = config;
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

    Ok(Localchain {
      db,
      ticker: TickerRef { ticker },
      mainchain_client: mainchain_client.clone(),
      notary_clients: NotaryClients::new(&mainchain_client),
    })
  }

  #[napi]
  pub async fn close(&self) -> Result<()> {
    tracing::trace!("Closing Localchain");
    self.mainchain_client.close().await.map_err(to_js_error)?;
    self.notary_clients.close().await;
    if !self.db.is_closed() {
      self.db.close().await;
    }
    tracing::trace!("Closed Localchain");
    Ok(())
  }

  #[napi]
  pub fn get_default_path() -> String {
    BasePath::from_project("org", "ulixee", "localchain")
      .path()
      .to_str()
      .unwrap()
      .to_string()
  }

  #[napi(getter)]
  pub fn current_tick(&self) -> u32 {
    self.ticker.current()
  }
  pub fn duration_to_next_tick(&self) -> Duration {
    self.ticker.duration_to_next_tick()
  }

  #[napi(getter)]
  pub fn constants(&self) -> Constants {
    Constants::default()
  }

  #[napi(getter)]
  pub fn ticker(&self) -> TickerRef {
    self.ticker.clone()
  }

  #[napi(getter)]
  pub fn mainchain_client(&self) -> MainchainClient {
    self.mainchain_client.clone()
  }

  #[napi(getter)]
  pub fn notary_clients(&self) -> NotaryClients {
    self.notary_clients.clone()
  }

  #[napi(getter)]
  pub fn accounts(&self) -> accounts::AccountStore {
    accounts::AccountStore::new(self)
  }

  #[napi(getter)]
  pub fn balance_changes(&self) -> balance_changes::BalanceChangeStore {
    balance_changes::BalanceChangeStore::new(self.db.clone())
  }

  #[napi(getter)]
  pub fn data_domains(&self) -> data_domain::DataDomainStore {
    data_domain::DataDomainStore::new(self.db.clone(), self.mainchain_client.clone())
  }

  #[napi(getter)]
  pub fn open_escrows(&self) -> open_escrows::OpenEscrowsStore {
    open_escrows::OpenEscrowsStore::new(self.db.clone(), self.ticker.clone(), &self.notary_clients)
  }

  #[napi(getter)]
  pub fn balance_sync(&self) -> balance_sync::BalanceSync {
    balance_sync::BalanceSync::new(&self)
  }

  #[napi]
  pub fn begin_change(&self) -> notarization_builder::NotarizationBuilder {
    notarization_builder::NotarizationBuilder::new(self.db.clone(), self.notary_clients.clone())
  }
}

#[napi(object)]
#[derive(Default)]
pub struct Constants {
  pub notarization_constants: NotarizationConstants,
  pub escrow_constants: EscrowConstants,
  pub data_domain_constants: DataDomainConstants,
}

#[napi(object)]
pub struct NotarizationConstants {
  pub max_balance_changes: u32,
  pub max_data_domains: u32,
  pub max_block_votes: u32,
}

impl Default for NotarizationConstants {
  fn default() -> Self {
    Self {
      max_balance_changes: ulx_primitives::MAX_BALANCE_CHANGES_PER_NOTARIZATION,
      max_data_domains: ulx_primitives::MAX_DOMAINS_PER_NOTARIZATION,
      max_block_votes: ulx_primitives::MAX_BLOCK_VOTES_PER_NOTARIZATION,
    }
  }
}

#[napi(object)]
pub struct EscrowConstants {
  pub expiration_ticks: u32,
  pub ticks_to_claim: u32,
}
impl Default for EscrowConstants {
  fn default() -> Self {
    Self {
      expiration_ticks: ulx_primitives::ESCROW_EXPIRATION_TICKS,
      ticks_to_claim: ulx_primitives::ESCROW_CLAWBACK_TICKS,
    }
  }
}

#[napi(object)]
pub struct DataDomainConstants {
  pub max_datastore_versions: u32,
  pub min_domain_name_length: u32,
  pub max_domain_name_length: u32,
  pub lease_cost: BigInt,
}

impl Default for DataDomainConstants {
  fn default() -> Self {
    Self {
      max_datastore_versions: ulx_primitives::MAX_DATASTORE_VERSIONS,
      min_domain_name_length: ulx_primitives::MIN_DATA_DOMAIN_NAME_LENGTH as u32,
      max_domain_name_length: ulx_primitives::MAX_DOMAIN_NAME_LENGTH,
      lease_cost: ulx_primitives::DATA_DOMAIN_LEASE_COST.into(),
    }
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
