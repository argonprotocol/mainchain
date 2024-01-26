use crate::{to_js_error, Localchain};
use chrono::NaiveDateTime;
use napi::bindgen_prelude::*;
use sp_core::crypto::{
  AccountId32, PublicError, Ss58AddressFormat, Ss58AddressFormatRegistry, Ss58Codec,
};
use sp_core::ByteArray;
use sqlx::{FromRow, SqliteConnection, SqlitePool};
use ulx_primitives::AccountOriginUid;
use ulx_primitives::NotaryId;
use ulx_primitives::NotebookNumber;
use ulx_primitives::{AccountOrigin, AccountType};

#[napi(object)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NotaryAccountOrigin {
  pub notary_id: u32,
  pub notebook_number: u32,
  pub account_uid: u32,
}
impl Into<AccountOrigin> for NotaryAccountOrigin {
  fn into(self) -> AccountOrigin {
    AccountOrigin {
      notebook_number: self.notebook_number as u32,
      account_uid: self.account_uid as u32,
    }
  }
}

#[napi]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAccount {
  pub id: i64,
  pub address: String,
  pub account_id32: String,
  pub notary_id: u32,
  pub account_type: AccountType,
  pub created_at: i64,
  pub updated_at: i64,
  pub origin: Option<NotaryAccountOrigin>,
}

impl LocalAccount {
  pub fn get_account_id32(&self) -> Result<AccountId32> {
    let account_id32 = hex::decode(&self.account_id32).map_err(to_js_error)?;
    Ok(
      AccountId32::from_slice(&account_id32)
        .map_err(|_| to_js_error(format!("Could not decode account id {account_id32:?}")))?,
    )
  }
}

impl Into<LocalAccount> for AccountRow {
  fn into(self) -> LocalAccount {
    let row = self;
    LocalAccount {
      id: row.id,
      account_id32: hex::encode(row.account_id32),
      address: row.address,
      account_type: row.account_type,
      notary_id: row.notary_id as u32,
      created_at: row.created_at.timestamp_millis(),
      updated_at: row.updated_at.timestamp_millis(),
      origin: match row.origin_notebook_number {
        Some(notebook_number) => Some(NotaryAccountOrigin {
          notary_id: row.notary_id as u32,
          notebook_number: notebook_number as u32,
          account_uid: row.origin_uid.unwrap_or_default() as u32,
        }),
        None => None,
      },
    }
  }
}

#[derive(FromRow)]
struct AccountRow {
  id: i64,
  address: String,
  account_id32: Vec<u8>,
  account_type: AccountType,
  notary_id: i64,
  origin_uid: Option<i64>,
  origin_notebook_number: Option<i64>,
  created_at: NaiveDateTime,
  updated_at: NaiveDateTime,
}
#[napi]
#[derive(Clone)]
pub struct AccountStore {
  pool: SqlitePool,
}

pub const ADDRESS_PREFIX: u16 = Ss58AddressFormatRegistry::SubstrateAccount as u16;

#[napi]
impl AccountStore {
  #[napi(constructor)]
  pub fn new(localchain: &Localchain) -> Self {
    AccountStore {
      pool: localchain.db.clone(),
    }
  }

  pub fn parse_address(address: &str) -> Result<AccountId32> {
    AccountId32::from_ss58check_with_version(address)
      .and_then(|(r, v)| match v {
        v if v.prefix() == ADDRESS_PREFIX => Ok(r),
        v => Err(PublicError::UnknownSs58AddressFormat(v)),
      })
      .map_err(to_js_error)
  }

  pub fn to_address(account_id32: &AccountId32) -> String {
    account_id32.to_ss58check_with_version(Self::address_format())
  }

  pub fn address_format() -> Ss58AddressFormat {
    Ss58AddressFormat::from(ADDRESS_PREFIX)
  }

  pub async fn get(
    db: &mut SqliteConnection,
    address: String,
    account_type: AccountType,
    notary_id: NotaryId,
  ) -> anyhow::Result<LocalAccount> {
    let account_type_i64 = account_type as i64;
    let notary_id_i64 = notary_id as i64;

    let res = sqlx::query_as!(
      AccountRow,
      r#"SELECT * from accounts WHERE address = $1 AND account_type = $2 AND notary_id = $3"#,
      address,
      account_type_i64,
      notary_id_i64,
    )
    .fetch_one(&mut *db)
    .await
    .map_err(to_js_error)?
    .into();
    Ok(res)
  }

  #[napi(js_name = "get")]
  pub async fn get_js(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    let res = Self::get(&mut *db, address, account_type, notary_id).await?;
    Ok(res)
  }
  pub async fn get_by_id(
    db: &mut SqliteConnection,
    account_id: i64,
  ) -> anyhow::Result<LocalAccount> {
    let res = sqlx::query_as!(
      AccountRow,
      r#"SELECT * from accounts WHERE id = $1"#,
      account_id,
    )
    .fetch_one(&mut *db)
    .await
    .map_err(to_js_error)?
    .into();
    Ok(res)
  }

  #[napi(js_name = "getById")]
  pub async fn get_by_id_js(&self, id: i64) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    let res = Self::get_by_id(&mut *db, id).await?;
    Ok(res)
  }

  pub async fn insert(
    db: &mut SqliteConnection,
    address: String,
    account_type: AccountType,
    notary_id: NotaryId,
  ) -> anyhow::Result<LocalAccount> {
    let account_type_i64 = account_type as i64;

    let account_id32 = AccountStore::parse_address(&address)?;
    let account_id32: &[u8] = account_id32.as_ref();
    let notary_id_i64 = notary_id as i64;

    let res = sqlx::query_as!(
      AccountRow,
      r#"INSERT INTO accounts (address, account_id32, account_type, notary_id) VALUES ($1, $2, $3, $4) RETURNING *"#,
      address,
      account_id32,
      account_type_i64,
      notary_id_i64,
    )
    .fetch_one(&mut *db)
    .await
    .map_err(to_js_error)?
    .into();
    Ok(res)
  }

  #[napi(js_name = "insert")]
  pub async fn insert_js(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    let res = Self::insert(&mut *db, address, account_type, notary_id)
      .await
      .map_err(to_js_error)?;
    Ok(res)
  }

  pub async fn update_origin(
    db: &mut SqliteConnection,
    account_id: i64,
    notebook_number: NotebookNumber,
    account_uid: AccountOriginUid,
  ) -> anyhow::Result<()> {
    let uid_i64 = account_uid as i64;
    let notebook_i64 = notebook_number as i64;
    let res = sqlx::query!(
      r#"UPDATE accounts SET origin_uid = ?, origin_notebook_number = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#,
      uid_i64,
      notebook_i64,
      account_id,
    )
    .execute(&mut *db)
    .await?;
    if res.rows_affected() != 1 {
      Err(anyhow::anyhow!("Error updating account"))?;
    }
    Ok(())
  }

  #[napi]
  pub async fn list(&self) -> Result<Vec<LocalAccount>> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    let res = sqlx::query_as!(AccountRow, r#"SELECT * from accounts"#,)
      .fetch_all(&mut *db)
      .await
      .map_err(to_js_error)?
      .into_iter()
      .map(|row| row.into())
      .collect::<Vec<_>>();
    Ok(res)
  }

  #[napi]
  pub async fn tax_accounts(&self, notary_id: u32) -> Result<Vec<LocalAccount>> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    let notary_id = notary_id as i64;
    let res = sqlx::query_as!(
      AccountRow,
      r#"SELECT * from accounts WHERE notary_id=? and account_type=?"#,
      notary_id,
      AccountType::Tax as i64,
    )
    .fetch_all(&mut *db)
    .await
    .map_err(to_js_error)?
    .into_iter()
    .map(|row| row.into())
    .collect::<Vec<_>>();

    Ok(res)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::*;
  use sp_keyring::AccountKeyring::Bob;
  use sp_keyring::Ed25519Keyring::Alice;

  #[sqlx::test]
  async fn accounts_stored_and_retrieved(pool: SqlitePool) -> anyhow::Result<()> {
    let bob_address = AccountStore::to_address(&Bob.to_account_id());
    let accounts = AccountStore { pool };
    let tax_account = accounts
      .insert_js(bob_address.clone(), AccountType::Tax, 1)
      .await
      .expect("Could not insert account");

    assert_eq!(tax_account.address, bob_address);
    assert_eq!(tax_account.get_account_id32()?, Bob.to_account_id());

    let _ = accounts
      .insert_js(bob_address.clone(), AccountType::Tax, 2)
      .await
      .expect("Could not insert account");

    let account = accounts
      .insert_js(bob_address.clone(), AccountType::Deposit, 1)
      .await
      .unwrap();

    let list = accounts.list().await?;
    assert_eq!(list.len(), 3);
    assert_eq!(accounts.tax_accounts(1).await?[0], tax_account);

    assert_eq!(accounts.get_by_id_js(account.id).await?, account);
    assert_eq!(
      accounts
        .get_js(bob_address.clone(), AccountType::Deposit, 1)
        .await?,
      account
    );
    Ok(())
  }

  #[sqlx::test]
  async fn can_update_an_origin(pool: SqlitePool) -> anyhow::Result<()> {
    let mut db = &mut *pool.acquire().await?;
    let account = AccountStore::insert(
      &mut db,
      AccountStore::to_address(&Bob.to_account_id()),
      AccountType::Deposit,
      1,
    )
    .await
    .expect("Could not insert account");
    assert_eq!(account.origin, None);

    AccountStore::update_origin(&mut db, account.id, 1, 1).await?;

    assert_eq!(
      AccountStore::get_by_id(&mut db, account.id).await?.origin,
      Some(NotaryAccountOrigin {
        notary_id: 1,
        notebook_number: 1,
        account_uid: 1,
      })
    );

    Ok(())
  }

  #[test]
  fn can_parse_addresses() -> anyhow::Result<()> {
    let address = AccountStore::to_address(&Alice.to_account_id());
    assert_eq!(
      AccountStore::parse_address(&address)?,
      Alice.to_account_id()
    );
    Ok(())
  }
}
