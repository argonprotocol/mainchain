use argon_primitives::{DataDomain, DataDomainHash, DataTLD};
use chrono::NaiveDateTime;
use serde_json::json;
use sp_runtime::RuntimeString;
use sqlx::{FromRow, SqliteConnection, SqlitePool};

use crate::{bail, Result};

#[derive(FromRow, Clone)]
#[allow(dead_code)]
#[cfg_attr(feature = "napi", napi(js_name = "DataDomainLease"))]
pub struct DataDomainRow {
  pub id: i64,
  pub name: String,
  pub tld: String,
  pub registered_to_address: String,
  pub notarization_id: i64,
  pub registered_at_tick: i64,
  created_at: NaiveDateTime,
}

impl DataDomainRow {}

#[cfg_attr(feature = "napi", napi)]
pub struct DataDomainStore {
  db: SqlitePool,
}

impl DataDomainStore {
  pub fn new(db: SqlitePool) -> Self {
    Self { db }
  }

  pub fn tld_from_string(tld: String) -> Result<DataTLD> {
    let tld_json_str = format!("\"{}\"", tld);
    let tld: DataTLD = serde_json::from_str(&tld_json_str)?;
    Ok(tld)
  }

  pub async fn list(&self) -> Result<Vec<DataDomainRow>> {
    let mut db = self.db.acquire().await?;
    Ok(
      sqlx::query_as!(DataDomainRow, "SELECT * FROM data_domains")
        .fetch_all(&mut *db)
        .await?,
    )
  }

  pub fn hash_domain(&self, domain: JsDataDomain) -> DataDomainHash {
    let parsed_domain = DataDomain {
      domain_name: RuntimeString::Owned(domain.domain_name.clone()),
      top_level_domain: domain.top_level_domain,
    };
    parsed_domain.hash()
  }

  pub fn get_hash(domain: String) -> Result<DataDomainHash> {
    let domain = DataDomain::parse(domain)?;
    Ok(domain.hash())
  }

  pub fn parse(domain: String) -> Result<JsDataDomain> {
    Ok(DataDomain::parse(domain).map(Into::into)?)
  }

  pub async fn get(&self, id: i64) -> Result<DataDomainRow> {
    let mut db = self.db.acquire().await?;
    Ok(
      sqlx::query_as!(DataDomainRow, "SELECT * FROM data_domains WHERE id = ?", id)
        .fetch_one(&mut *db)
        .await?,
    )
  }

  pub async fn db_insert(
    db: &mut SqliteConnection,
    data_domain: JsDataDomain,
    registered_to_address: String,
    notarization_id: i64,
    registered_at_tick: u32,
  ) -> Result<()> {
    let tld = json!(data_domain.top_level_domain);
    // remove leading and trailing quote from json
    let tld = tld.to_string();
    let tld = tld.trim_matches('"');
    let registered_at_tick = registered_at_tick as i64;
    let res = sqlx::query!(
      "INSERT INTO data_domains (name, tld, registered_to_address, notarization_id, registered_at_tick) VALUES (?, ?, ?, ?, ?)",
      data_domain.domain_name,
      tld,
      registered_to_address,
      notarization_id,
      registered_at_tick,
    )
            .execute(db)
            .await
            ?;
    if res.rows_affected() != 1 {
      bail!("Error inserting data domain {}", data_domain.domain_name);
    }
    Ok(())
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use super::*;
  use crate::error::NapiOk;
  use crate::{DataDomainStore, JsDataDomain};
  use argon_primitives::DataTLD;
  use napi::bindgen_prelude::*;

  #[napi]
  impl DataDomainStore {
    #[napi(js_name = "tldFromString")]
    pub fn tld_from_string_napi(tld: String) -> napi::Result<DataTLD> {
      DataDomainStore::tld_from_string(tld).napi_ok()
    }
    #[napi(js_name = "list", getter)]
    pub async fn list_napi(&self) -> napi::Result<Vec<DataDomainRow>> {
      self.list().await.napi_ok()
    }

    #[napi(js_name = "hashDomain")]
    pub fn hash_domain_napi(&self, domain: JsDataDomain) -> Uint8Array {
      self.hash_domain(domain).0.into()
    }

    #[napi(js_name = "getHash")]
    pub fn get_hash_napi(domain: String) -> napi::Result<Uint8Array> {
      DataDomainStore::get_hash(domain)
        .map(|a| a.0.into())
        .napi_ok()
    }

    #[napi(js_name = "parse", ts_return_type = "DataDomain")]
    pub fn parse_napi(domain: String) -> napi::Result<JsDataDomain> {
      DataDomainStore::parse(domain).napi_ok()
    }

    #[napi(js_name = "get")]
    pub async fn get_napi(&self, id: i64) -> napi::Result<DataDomainRow> {
      self.get(id).await.napi_ok()
    }
  }
}

#[cfg_attr(feature = "napi", napi(object, js_name = "DataDomain"))]
#[derive(Clone, Debug, PartialEq)]
pub struct JsDataDomain {
  pub domain_name: String,
  pub top_level_domain: DataTLD,
}

impl From<DataDomain> for JsDataDomain {
  fn from(domain: DataDomain) -> Self {
    Self {
      domain_name: domain.domain_name.to_string(),
      top_level_domain: domain.top_level_domain,
    }
  }
}

impl From<JsDataDomain> for DataDomain {
  fn from(val: JsDataDomain) -> Self {
    DataDomain {
      domain_name: RuntimeString::Owned(val.domain_name.clone()),
      top_level_domain: val.top_level_domain,
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
  fn test_data_domain_js_conversion() -> anyhow::Result<()> {
    let domain = DataDomain {
      domain_name: "test".into(),
      top_level_domain: DataTLD::Cars,
    };
    let js_domain: JsDataDomain = domain.clone().into();
    let domain2: DataDomain = js_domain.into();
    assert_eq!(domain, domain2);
    Ok(())
  }

  #[sqlx::test]
  async fn test_data_domain_store(pool: SqlitePool) -> anyhow::Result<()> {
    let store = DataDomainStore::new(pool);
    let domain = DataDomain::new("test", DataTLD::Cars);
    let js_domain: JsDataDomain = domain.into();

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
    DataDomainStore::db_insert(
      &mut db,
      js_domain.clone(),
      AccountStore::to_address(&Bob.to_account_id()),
      1,
      1,
    )
    .await?;

    assert_eq!(store.list().await?.len(), 1);
    assert_eq!(store.get(1).await?.name, "test");
    assert_eq!(store.get(1).await?.tld, "cars");
    Ok(())
  }
}
