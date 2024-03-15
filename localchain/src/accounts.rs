use anyhow::anyhow;
use chrono::NaiveDateTime;
use napi::bindgen_prelude::*;
use sp_core::ByteArray;
use sp_core::crypto::{
  AccountId32, PublicError, Ss58AddressFormat, Ss58Codec,
};
use sqlx::{FromRow, SqliteConnection, SqlitePool};

use ulx_primitives::{AccountOrigin, AccountType, ADDRESS_PREFIX};
use ulx_primitives::AccountOriginUid;
use ulx_primitives::NotaryId;
use ulx_primitives::NotebookNumber;

use crate::{BalanceChangeStatus, BalanceChangeStore, to_js_error};

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
  pub hd_path: Option<String>,
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
      hd_path: row.hd_path,
      account_id32: hex::encode(row.account_id32),
      address: row.address,
      account_type: row.account_type,
      notary_id: row.notary_id as u32,
      created_at: row.created_at.and_utc().timestamp_millis(),
      updated_at: row.updated_at.and_utc().timestamp_millis(),
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
  hd_path: Option<String>,
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

pub const DEFAULT_NOTARY_ID: NotaryId = 1;

#[napi]
impl AccountStore {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
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

  #[napi(js_name = "getDepositAccount")]
  pub async fn deposit_account_js(&self, notary_id: Option<u32>) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    let res = Self::deposit_account(&mut *db, notary_id).await?;
    Ok(res)
  }

  pub async fn deposit_account(
    db: &mut SqliteConnection,
    notary_id: Option<NotaryId>,
  ) -> anyhow::Result<LocalAccount> {
    let notary_id = notary_id.unwrap_or(DEFAULT_NOTARY_ID) as i32;
    let res = sqlx::query_as!(
      AccountRow,
      r#"SELECT * from accounts WHERE hd_path IS NULL AND account_type = ? AND notary_id = ?"#,
      AccountType::Deposit as i64,
      notary_id
    )
    .fetch_optional(&mut *db)
    .await?;
    if let Some(res) = res {
      return Ok(res.into());
    }

    return Err(anyhow!(
      "This localchain has not been setup with an address! Import or create a new account."
    ));
  }

  #[napi(js_name = "getTaxAccount")]
  pub async fn tax_account_js(&self, notary_id: Option<NotaryId>) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    let res = Self::tax_account(&mut *db, notary_id).await?;
    Ok(res)
  }

  pub async fn tax_account(
    db: &mut SqliteConnection,
    notary_id: Option<NotaryId>,
  ) -> anyhow::Result<LocalAccount> {
    let notary_id = notary_id.unwrap_or(DEFAULT_NOTARY_ID) as i32;
    let res = sqlx::query_as!(
      AccountRow,
      r#"SELECT * from accounts WHERE hd_path IS NULL AND account_type = ? AND notary_id = ?"#,
      AccountType::Tax as i64,
      notary_id
    )
    .fetch_optional(&mut *db)
    .await?;
    if let Some(res) = res {
      return Ok(res.into());
    }

    return Err(anyhow!(
      "This localchain has not been setup with an address! Import or create a new account."
    ));
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

  #[napi(js_name = "hasAccount")]
  pub async fn has_account_js(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<bool> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    Ok(Self::has_account(&mut *db, address, account_type, notary_id).await)
  }

  pub async fn has_account(
    db: &mut SqliteConnection,
    address: String,
    account_type: AccountType,
    notary_id: u32,
  ) -> bool {
    Self::get(&mut *db, address, account_type, notary_id)
      .await
      .ok()
      .is_some()
  }

  pub async fn get_next_jump_path(
    db: &mut SqliteConnection,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<String> {
    let account_type_i64 = account_type as i64;
    let notary_id = notary_id as i64;
    let res = sqlx::query_scalar!(
      r#"SELECT hd_path from accounts WHERE hd_path IS NOT NULL AND account_type = ? AND notary_id = ? ORDER BY hd_path DESC"#,
      account_type_i64,
      notary_id,
    )
    .fetch_all(&mut *db)
    .await
    .map_err(to_js_error)?;

    let mut max_jump_id = 0u32;
    for path in res {
      let Some(last_path) = path else {
        continue;
      };
      let jump_counter = last_path
        .split("//")
        .last()
        .unwrap()
        .parse::<u32>()
        .unwrap_or_default();
      if jump_counter > max_jump_id {
        max_jump_id = jump_counter;
      }
    }
    return Ok(format!("//jump//{}", max_jump_id + 1));
  }

  pub async fn find_idle_jump_account(
    db: &mut SqliteConnection,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<Option<LocalAccount>> {
    let account_type_i64 = account_type as i64;
    let notary_id = notary_id as i64;
    let res = sqlx::query_as!(
      AccountRow,
      r#"SELECT * from accounts WHERE hd_path IS NOT NULL AND account_type = ? AND notary_id = ?"#,
      account_type_i64,
      notary_id,
    )
    .fetch_all(&mut *db)
    .await
    .map_err(to_js_error)?;

    for row in res {
      let balance = BalanceChangeStore::get_latest_for_account(&mut *db, row.id)
        .await
        .map_err(to_js_error)?;
      if let Some(latest_balance) = balance {
        if latest_balance.balance == "0"
          && latest_balance.status != BalanceChangeStatus::WaitingForSendClaim
        {
          return Ok(Some(row.into()));
        }
      } else {
        return Ok(Some(row.into()));
      }
    }
    Ok(None)
  }

  /// Finds an account with no balance that is not waiting for a send claim
  #[napi(js_name = "findIdleJumpAccount")]
  pub async fn find_idle_jump_account_js(
    &self,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<Option<LocalAccount>> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;

    Self::find_idle_jump_account(&mut *db, account_type, notary_id).await
  }

  pub async fn bootstrap(
    pool: SqlitePool,
    address: String,
    notary_id: Option<NotaryId>,
  ) -> anyhow::Result<()> {
    let mut db = pool.acquire().await?;

    if let Ok(account) = AccountStore::deposit_account(&mut db, notary_id).await {
      if account.address != address {
        return Err(anyhow::anyhow!(
          "Cannot bootstrap this localchain with a different address"
        ));
      }
      return Ok(());
    }

    let notary_id = notary_id.unwrap_or(DEFAULT_NOTARY_ID);
    AccountStore::insert(
      &mut db,
      address.clone(),
      AccountType::Deposit,
      notary_id,
      None,
    )
    .await?;
    AccountStore::insert(&mut db, address.clone(), AccountType::Tax, notary_id, None).await?;
    Ok(())
  }

  pub async fn insert(
    db: &mut SqliteConnection,
    address: String,
    account_type: AccountType,
    notary_id: NotaryId,
    hd_path: Option<String>,
  ) -> anyhow::Result<LocalAccount> {
    let account_type_i64 = account_type as i64;

    let account_id32 = AccountStore::parse_address(&address)?;
    let account_id32: &[u8] = account_id32.as_ref();
    let notary_id_i64 = notary_id as i64;

    let res = sqlx::query_as!(
      AccountRow,
      r#"INSERT INTO accounts (address, account_id32, account_type, notary_id,  hd_path) VALUES ($1, $2, $3, $4, $5) RETURNING *"#,
      address,
      account_id32,
      account_type_i64,
      notary_id_i64,
      hd_path,
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
    hd_path: Option<String>,
  ) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    let res = Self::insert(&mut *db, address, account_type, notary_id, hd_path)
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

  pub async fn list(
    db: &mut SqliteConnection,
    include_jump_accounts: bool,
  ) -> anyhow::Result<Vec<LocalAccount>> {
    let query = if include_jump_accounts {
      sqlx::query_as!(AccountRow, "SELECT * from accounts",)
        .fetch_all(&mut *db)
        .await?
    } else {
      sqlx::query_as!(AccountRow, "SELECT * from accounts WHERE hd_path IS NULL",)
        .fetch_all(&mut *db)
        .await?
    };

    let res = query.into_iter().map(|row| row.into()).collect::<Vec<_>>();
    Ok(res)
  }

  #[napi(js_name = "list")]
  pub async fn list_js(&self, include_jump_accounts: Option<bool>) -> Result<Vec<LocalAccount>> {
    let mut db = self.pool.acquire().await.map_err(to_js_error)?;
    Self::list(&mut db, include_jump_accounts.unwrap_or(false))
      .await
      .map_err(to_js_error)
  }
}

#[cfg(test)]
mod test {
  use sp_keyring::AccountKeyring::Bob;
  use sp_keyring::Ed25519Keyring::Alice;

  use crate::*;

  use super::*;

  #[sqlx::test]
  async fn accounts_stored_and_retrieved(pool: SqlitePool) -> anyhow::Result<()> {
    let bob_address = AccountStore::to_address(&Bob.to_account_id());
    let accounts = AccountStore { pool };
    let tax_account = accounts
      .insert_js(bob_address.clone(), AccountType::Tax, 1, None)
      .await
      .expect("Could not insert account");

    assert_eq!(tax_account.address, bob_address);
    assert_eq!(tax_account.get_account_id32()?, Bob.to_account_id());

    let _ = accounts
      .insert_js(bob_address.clone(), AccountType::Tax, 2, None)
      .await
      .expect("Could not insert account");

    let account = accounts
      .insert_js(bob_address.clone(), AccountType::Deposit, 1, None)
      .await
      .unwrap();

    let list = accounts.list_js(Some(true)).await?;
    assert_eq!(list.len(), 3);
    assert_eq!(accounts.tax_account_js(Some(1)).await?, tax_account);

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
      None,
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
