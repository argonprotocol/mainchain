use argon_primitives::{Domain, DomainHash, DomainTopLevel};
use chrono::NaiveDateTime;
use serde_json::json;
use sp_runtime::RuntimeString;
use sqlx::{FromRow, SqliteConnection, SqlitePool};

use crate::{bail, Result};

#[derive(FromRow, Clone)]
#[allow(dead_code)]
#[cfg_attr(feature = "napi", napi(js_name = "DomainLease"))]
pub struct DomainRow {
  pub id: i64,
  pub name: String,
  pub top_level: String,
  pub registered_to_address: String,
  pub notarization_id: i64,
  pub registered_at_tick: i64,
  created_at: NaiveDateTime,
}

impl DomainRow {}

#[cfg_attr(feature = "napi", napi)]
pub struct DomainStore {
  db: SqlitePool,
}

impl DomainStore {
  pub fn new(db: SqlitePool) -> Self {
    Self { db }
  }

  pub fn tld_from_string(top_level: String) -> Result<DomainTopLevel> {
    let tld_json_str = format!("\"{}\"", top_level);
    let top_level: DomainTopLevel = serde_json::from_str(&tld_json_str)?;
    Ok(top_level)
  }

  pub async fn list(&self) -> Result<Vec<DomainRow>> {
    let mut db = self.db.acquire().await?;
    Ok(
      sqlx::query_as!(DomainRow, "SELECT * FROM domains")
        .fetch_all(&mut *db)
        .await?,
    )
  }

  pub fn hash_domain(&self, domain: JsDomain) -> DomainHash {
    let parsed_domain = Domain {
      name: RuntimeString::Owned(domain.name.clone()),
      top_level: domain.top_level,
    };
    parsed_domain.hash()
  }

  pub fn get_hash(domain: String) -> Result<DomainHash> {
    let domain = Domain::parse(domain)?;
    Ok(domain.hash())
  }

  pub fn parse(domain: String) -> Result<JsDomain> {
    Ok(Domain::parse(domain).map(Into::into)?)
  }

  pub async fn get(&self, id: i64) -> Result<DomainRow> {
    let mut db = self.db.acquire().await?;
    Ok(
      sqlx::query_as!(DomainRow, "SELECT * FROM domains WHERE id = ?", id)
        .fetch_one(&mut *db)
        .await?,
    )
  }

  pub async fn db_insert(
    db: &mut SqliteConnection,
    domain: JsDomain,
    registered_to_address: String,
    notarization_id: i64,
    registered_at_tick: u32,
  ) -> Result<()> {
    let top_level = json!(domain.top_level);
    // remove leading and trailing quote from json
    let top_level = top_level.to_string();
    let top_level = top_level.trim_matches('"');
    let registered_at_tick = registered_at_tick as i64;
    let res = sqlx::query!(
      "INSERT INTO domains (name, top_level, registered_to_address, notarization_id, registered_at_tick) VALUES (?, ?, ?, ?, ?)",
      domain.name,
      top_level,
      registered_to_address,
      notarization_id,
      registered_at_tick,
    )
            .execute(db)
            .await
            ?;
    if res.rows_affected() != 1 {
      bail!("Error inserting data domain {}", domain.name);
    }
    Ok(())
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use super::*;
  use crate::error::NapiOk;
  use crate::{DomainStore, JsDomain};
  use argon_primitives::DomainTopLevel;
  use napi::bindgen_prelude::*;

  #[napi]
  impl DomainStore {
    #[napi(js_name = "tldFromString")]
    pub fn tld_from_string_napi(top_level: String) -> napi::Result<DomainTopLevel> {
      DomainStore::tld_from_string(top_level).napi_ok()
    }
    #[napi(js_name = "list", getter)]
    pub async fn list_napi(&self) -> napi::Result<Vec<DomainRow>> {
      self.list().await.napi_ok()
    }

    #[napi(js_name = "hashDomain")]
    pub fn hash_domain_napi(&self, domain: JsDomain) -> Uint8Array {
      self.hash_domain(domain).0.into()
    }

    #[napi(js_name = "getHash")]
    pub fn get_hash_napi(domain: String) -> napi::Result<Uint8Array> {
      DomainStore::get_hash(domain).map(|a| a.0.into()).napi_ok()
    }

    #[napi(js_name = "parse", ts_return_type = "Domain")]
    pub fn parse_napi(domain: String) -> napi::Result<JsDomain> {
      DomainStore::parse(domain).napi_ok()
    }

    #[napi(js_name = "get")]
    pub async fn get_napi(&self, id: i64) -> napi::Result<DomainRow> {
      self.get(id).await.napi_ok()
    }
  }
}

#[cfg_attr(feature = "napi", napi(object, js_name = "Domain"))]
#[derive(Clone, Debug, PartialEq)]
pub struct JsDomain {
  pub name: String,
  pub top_level: DomainTopLevel,
}

impl From<Domain> for JsDomain {
  fn from(domain: Domain) -> Self {
    Self {
      name: domain.name.to_string(),
      top_level: domain.top_level,
    }
  }
}

impl From<JsDomain> for Domain {
  fn from(val: JsDomain) -> Self {
    Domain {
      name: RuntimeString::Owned(val.name.clone()),
      top_level: val.top_level,
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::AccountStore;
  use sp_keyring::AccountKeyring::Bob;
  use sqlx::SqlitePool;

  #[test]
  fn test_domain_js_conversion() -> anyhow::Result<()> {
    let domain = Domain {
      name: "test".into(),
      top_level: DomainTopLevel::Cars,
    };
    let js_domain: JsDomain = domain.clone().into();
    let domain2: Domain = js_domain.into();
    assert_eq!(domain, domain2);
    Ok(())
  }

  #[sqlx::test]
  async fn test_domain_store(pool: SqlitePool) -> anyhow::Result<()> {
    let store = DomainStore::new(pool);
    let domain = Domain::new("test", DomainTopLevel::Cars);
    let js_domain: JsDomain = domain.into();

    let mut db = store.db.acquire().await?;
    // insert a fake notarization for foreign keys
    sqlx::query!(
      "INSERT into notarizations (json, notary_id, notebook_number, tick) VALUES (?, ?, ?, ?)",
      "{}",
      1,
      1,
      1
    )
    .execute(&mut *db)
    .await?;
    DomainStore::db_insert(
      &mut db,
      js_domain.clone(),
      AccountStore::to_address(&Bob.to_account_id()),
      1,
      1,
    )
    .await?;

    assert_eq!(store.list().await?.len(), 1);
    assert_eq!(store.get(1).await?.name, "test");
    assert_eq!(store.get(1).await?.top_level, "cars");
    Ok(())
  }
}
