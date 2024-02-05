use chrono::NaiveDateTime;
use napi::bindgen_prelude::*;
use sp_runtime::RuntimeString;
use sqlx::{FromRow, SqliteConnection, SqlitePool};

use ulx_primitives::{DataDomain, DataTLD};

use crate::{to_js_error, MainchainClient};

#[derive(FromRow, Clone)]
#[allow(dead_code)]
#[napi(js_name = "DataDomainLease")]
pub struct DataDomainRow {
  pub id: i64,
  pub name: String,
  pub tld: i64,
  pub registered_to_address: String,
  pub notarization_id: i64,
  pub registered_at_tick: i64,
  created_at: NaiveDateTime,
}

impl DataDomainRow {}

#[napi]
pub struct DataDomainStore {
  db: SqlitePool,
  mainchain_client: MainchainClient,
}

#[napi]
impl DataDomainStore {
  pub(crate) fn new(db: SqlitePool, mainchain_client: MainchainClient) -> Self {
    Self {
      db,
      mainchain_client,
    }
  }

  #[napi(getter)]
  pub async fn list(&self) -> Result<Vec<DataDomainRow>> {
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    sqlx::query_as!(DataDomainRow, "SELECT * FROM data_domains")
      .fetch_all(&mut *db)
      .await
      .map_err(to_js_error)
  }

  #[napi]
  pub fn get_hash(&self, domain_name: String, tld: DataTLD) -> Uint8Array {
    let domain = DataDomain {
      domain_name: RuntimeString::Owned(domain_name),
      top_level_domain: tld,
    };
    domain.hash().0.into()
  }

  #[napi]
  pub async fn is_registered(&self, domain_name: String, tld: DataTLD) -> Result<bool> {
    let registration = self
      .mainchain_client
      .get_data_domain_registration(domain_name, tld)
      .await?;
    Ok(registration.is_some())
  }

  #[napi]
  pub async fn get(&self, id: i64) -> Result<DataDomainRow> {
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    sqlx::query_as!(DataDomainRow, "SELECT * FROM data_domains WHERE id = ?", id)
      .fetch_one(&mut *db)
      .await
      .map_err(to_js_error)
  }

  pub async fn insert(
    db: &mut SqliteConnection,
    data_domain: JsDataDomain,
    registered_to_address: String,
    notarization_id: i64,
    registered_at_tick: u32,
  ) -> Result<()> {
    let tld_id = data_domain.top_level_domain as i64;
    let registered_at_tick = registered_at_tick as i64;
    let res = sqlx::query!(
      "INSERT INTO data_domains (name, tld, registered_to_address, notarization_id, registered_at_tick) VALUES (?, ?, ?, ?, ?)",
      data_domain.domain_name,
      tld_id,
      registered_to_address,
      notarization_id,
      registered_at_tick,
    )
    .execute(db)
    .await
    .map_err(to_js_error)?;
    if res.rows_affected() != 1 {
      return Err(Error::from_reason(format!(
        "Error inserting data domain {}",
        data_domain.domain_name
      )));
    }
    Ok(())
  }
}
#[napi(object, js_name = "DataDomain")]
#[derive(Clone, Debug, PartialEq)]
pub struct JsDataDomain {
  pub domain_name: String,
  pub top_level_domain: DataTLD,
}

impl From<DataDomain> for JsDataDomain {
  fn from(domain: DataDomain) -> Self {
    Self {
      domain_name: domain.domain_name.to_string(),
      top_level_domain: domain.top_level_domain.clone(),
    }
  }
}

impl Into<DataDomain> for JsDataDomain {
  fn into(self) -> DataDomain {
    DataDomain {
      domain_name: RuntimeString::Owned(self.domain_name.clone()),
      top_level_domain: self.top_level_domain,
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
    let domain2: DataDomain = js_domain.try_into()?;
    assert_eq!(domain, domain2);
    Ok(())
  }

  #[sqlx::test]
  async fn test_data_domain_store(pool: SqlitePool) -> anyhow::Result<()> {
    let mainchain_client = MainchainClient::mock();
    let store = DataDomainStore::new(pool, mainchain_client);
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
    DataDomainStore::insert(
      &mut *db,
      js_domain.clone(),
      AccountStore::to_address(&Bob.to_account_id()),
      1,
      1,
    )
    .await?;

    assert_eq!(store.list().await?.len(), 1);
    assert_eq!(store.get(1).await?.name, "test");
    assert_eq!(store.get(1).await?.tld, DataTLD::Cars as i64);
    Ok(())
  }
}
