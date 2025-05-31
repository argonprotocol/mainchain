use anyhow::anyhow;
use chrono::NaiveDateTime;
use polkadot_sdk::*;
use sp_core::ByteArray;
use sp_core::crypto::{AccountId32, PublicError, Ss58AddressFormat, Ss58Codec};
use sqlx::{FromRow, SqliteConnection, SqlitePool};

use argon_primitives::AccountOriginUid;
use argon_primitives::NotaryId;
use argon_primitives::NotebookNumber;
use argon_primitives::{ADDRESS_PREFIX, AccountOrigin, AccountType};

use crate::{BalanceChangeStatus, BalanceChangeStore, Result, bail};

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NotaryAccountOrigin {
  pub notary_id: u32,
  pub notebook_number: u32,
  pub account_uid: u32,
}

impl From<NotaryAccountOrigin> for AccountOrigin {
  fn from(val: NotaryAccountOrigin) -> Self {
    AccountOrigin {
      notebook_number: val.notebook_number,
      account_uid: val.account_uid,
    }
  }
}

#[cfg_attr(feature = "napi", napi)]
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
    let account_id32 = hex::decode(&self.account_id32).map_err(|e| anyhow!(e))?;
    Ok(
      AccountId32::from_slice(&account_id32)
        .map_err(|_| anyhow!("Could not decode account id {account_id32:?}"))?,
    )
  }
}

impl From<AccountRow> for LocalAccount {
  fn from(val: AccountRow) -> Self {
    let row = val;
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

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone)]
pub struct AccountStore {
  pool: SqlitePool,
}

pub const DEFAULT_NOTARY_ID: NotaryId = 1;

impl AccountStore {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }

  pub fn parse_address(address: &str) -> Result<AccountId32> {
    Ok(
      AccountId32::from_ss58check_with_version(address)
        .and_then(|(r, v)| match v {
          v if v.prefix() == ADDRESS_PREFIX => Ok(r),
          v => Err(PublicError::UnknownSs58AddressFormat(v)),
        })
        .map_err(|e| anyhow!(e))?,
    )
  }

  pub fn to_address(account_id32: &AccountId32) -> String {
    account_id32.to_ss58check_with_version(Self::address_format())
  }

  pub fn address_format() -> Ss58AddressFormat {
    Ss58AddressFormat::from(ADDRESS_PREFIX)
  }

  pub async fn deposit_account(&self, notary_id: Option<u32>) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await?;
    let res = Self::db_deposit_account(&mut db, notary_id).await?;
    Ok(res)
  }

  pub async fn db_deposit_account(
    db: &mut SqliteConnection,
    notary_id: Option<NotaryId>,
  ) -> Result<LocalAccount> {
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

    bail!("This localchain has not been setup with an address! Import or create a new account.");
  }

  pub async fn tax_account(&self, notary_id: Option<NotaryId>) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await?;
    let res = super::AccountStore::db_tax_account(&mut db, notary_id).await?;
    Ok(res)
  }

  pub async fn db_tax_account(
    db: &mut SqliteConnection,
    notary_id: Option<NotaryId>,
  ) -> Result<LocalAccount> {
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

    bail!("This localchain has not been setup with an address! Import or create a new account.");
  }

  pub async fn get_by_id(&self, id: i64) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await?;
    let res = Self::db_get_by_id(&mut db, id).await?;
    Ok(res)
  }

  pub async fn db_get_by_id(db: &mut SqliteConnection, account_id: i64) -> Result<LocalAccount> {
    let res = sqlx::query_as!(
      AccountRow,
      r#"SELECT * from accounts WHERE id = $1"#,
      account_id,
    )
    .fetch_one(&mut *db)
    .await?
    .into();
    Ok(res)
  }

  pub async fn get(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await?;
    let res = Self::db_get(&mut db, address, account_type, notary_id).await?;
    Ok(res)
  }

  pub async fn db_get(
    db: &mut SqliteConnection,
    address: String,
    account_type: AccountType,
    notary_id: NotaryId,
  ) -> Result<LocalAccount> {
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
    .await?
    .into();
    Ok(res)
  }

  pub async fn has_account(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<bool> {
    let mut db = self.pool.acquire().await?;
    Ok(Self::db_has_account(&mut db, address, account_type, notary_id).await)
  }

  pub async fn db_has_account(
    db: &mut SqliteConnection,
    address: String,
    account_type: AccountType,
    notary_id: u32,
  ) -> bool {
    Self::db_get(&mut *db, address, account_type, notary_id)
      .await
      .ok()
      .is_some()
  }

  /// Finds an account with no balance that is not waiting for a send claim
  pub async fn find_idle_jump_account(
    &self,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<Option<LocalAccount>> {
    let mut db = self.pool.acquire().await?;
    super::AccountStore::db_find_idle_jump_account(&mut db, account_type, notary_id).await
  }

  pub async fn db_get_next_jump_path(
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
    .await?;

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
    Ok(format!("//jump//{}", max_jump_id + 1))
  }

  pub async fn db_find_idle_jump_account(
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
    .await?;

    for row in res {
      let balance = BalanceChangeStore::db_get_latest_for_account(&mut *db, row.id).await?;
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

  pub async fn bootstrap(
    pool: SqlitePool,
    address: String,
    notary_id: Option<NotaryId>,
  ) -> Result<()> {
    let mut db = pool.acquire().await?;

    if let Ok(account) = AccountStore::db_deposit_account(&mut db, notary_id).await {
      if account.address != address {
        bail!("Cannot bootstrap this localchain with a different address");
      }
      return Ok(());
    }

    let notary_id = notary_id.unwrap_or(DEFAULT_NOTARY_ID);
    AccountStore::db_insert(
      &mut db,
      address.clone(),
      AccountType::Deposit,
      notary_id,
      None,
    )
    .await?;
    AccountStore::db_insert(&mut db, address.clone(), AccountType::Tax, notary_id, None).await?;
    Ok(())
  }

  pub async fn insert(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: u32,
    hd_path: Option<String>,
  ) -> Result<LocalAccount> {
    let mut db = self.pool.acquire().await?;
    let res =
      super::AccountStore::db_insert(&mut db, address, account_type, notary_id, hd_path).await?;
    Ok(res)
  }

  pub async fn db_insert(
    db: &mut SqliteConnection,
    address: String,
    account_type: AccountType,
    notary_id: NotaryId,
    hd_path: Option<String>,
  ) -> Result<LocalAccount> {
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
            ?
            .into();
    Ok(res)
  }

  pub async fn db_update_origin(
    db: &mut SqliteConnection,
    account_id: i64,
    notebook_number: NotebookNumber,
    account_uid: AccountOriginUid,
  ) -> Result<()> {
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
  pub async fn list(&self, include_jump_accounts: Option<bool>) -> Result<Vec<LocalAccount>> {
    let mut db = self.pool.acquire().await?;
    Self::db_list(&mut db, include_jump_accounts.unwrap_or(false)).await
  }

  pub async fn db_list(
    db: &mut SqliteConnection,
    include_jump_accounts: bool,
  ) -> Result<Vec<LocalAccount>> {
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
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use super::*;
  use crate::LocalAccount;
  use crate::error::NapiOk;
  use argon_primitives::AccountType;

  #[napi]
  impl AccountStore {
    #[napi(js_name = "getDepositAccount")]
    pub async fn deposit_account_napi(&self, notary_id: Option<u32>) -> napi::Result<LocalAccount> {
      self.deposit_account(notary_id).await.napi_ok()
    }

    #[napi(js_name = "getTaxAccount")]
    pub async fn tax_account_napi(&self, notary_id: Option<u32>) -> napi::Result<LocalAccount> {
      self.tax_account(notary_id).await.napi_ok()
    }
    #[napi(js_name = "get")]
    pub async fn get_napi(
      &self,
      address: String,
      account_type: AccountType,
      notary_id: u32,
    ) -> napi::Result<LocalAccount> {
      self.get(address, account_type, notary_id).await.napi_ok()
    }

    #[napi(js_name = "getById")]
    pub async fn get_by_id_napi(&self, id: i64) -> napi::Result<LocalAccount> {
      self.get_by_id(id).await.napi_ok()
    }

    #[napi(js_name = "hasAccount")]
    pub async fn has_account_napi(
      &self,
      address: String,
      account_type: AccountType,
      notary_id: u32,
    ) -> napi::Result<bool> {
      self
        .has_account(address, account_type, notary_id)
        .await
        .napi_ok()
    }
    /// Finds an account with no balance that is not waiting for a send claim
    #[napi(js_name = "findIdleJumpAccount")]
    pub async fn find_idle_jump_account_napi(
      &self,
      account_type: AccountType,
      notary_id: u32,
    ) -> napi::Result<Option<LocalAccount>> {
      self
        .find_idle_jump_account(account_type, notary_id)
        .await
        .napi_ok()
    }

    #[napi(js_name = "insert")]
    pub async fn insert_napi(
      &self,
      address: String,
      account_type: AccountType,
      notary_id: u32,
      hd_path: Option<String>,
    ) -> napi::Result<LocalAccount> {
      self
        .insert(address, account_type, notary_id, hd_path)
        .await
        .napi_ok()
    }

    #[napi(js_name = "list")]
    pub async fn list_napi(
      &self,
      include_jump_accounts: Option<bool>,
    ) -> napi::Result<Vec<LocalAccount>> {
      self.list(include_jump_accounts).await.napi_ok()
    }
  }
}

#[cfg(test)]
mod test {
  use sp_keyring::Ed25519Keyring::Alice;
  use sp_keyring::Sr25519Keyring::Bob;

  use crate::*;

  use super::*;

  #[sqlx::test]
  async fn accounts_stored_and_retrieved(pool: SqlitePool) -> Result<()> {
    let bob_address = AccountStore::to_address(&Bob.to_account_id());
    let accounts = AccountStore { pool };
    let tax_account = accounts
      .insert(bob_address.clone(), AccountType::Tax, 1, None)
      .await
      .expect("Could not insert account");

    assert_eq!(tax_account.address, bob_address);
    assert_eq!(tax_account.get_account_id32()?, Bob.to_account_id());

    let _ = accounts
      .insert(bob_address.clone(), AccountType::Tax, 2, None)
      .await
      .expect("Could not insert account");

    let account = accounts
      .insert(bob_address.clone(), AccountType::Deposit, 1, None)
      .await
      .unwrap();

    let list = accounts.list(Some(true)).await?;
    assert_eq!(list.len(), 3);
    assert_eq!(accounts.tax_account(Some(1)).await?, tax_account);

    assert_eq!(accounts.get_by_id(account.id).await?, account);
    assert_eq!(
      accounts
        .get(bob_address.clone(), AccountType::Deposit, 1)
        .await?,
      account
    );
    Ok(())
  }

  #[sqlx::test]
  async fn can_update_an_origin(pool: SqlitePool) -> Result<()> {
    let db = &mut *pool.acquire().await?;
    let account = AccountStore::db_insert(
      db,
      AccountStore::to_address(&Bob.to_account_id()),
      AccountType::Deposit,
      1,
      None,
    )
    .await
    .expect("Could not insert account");
    assert_eq!(account.origin, None);

    AccountStore::db_update_origin(db, account.id, 1, 1).await?;

    assert_eq!(
      AccountStore::db_get_by_id(db, account.id).await?.origin,
      Some(NotaryAccountOrigin {
        notary_id: 1,
        notebook_number: 1,
        account_uid: 1,
      })
    );

    Ok(())
  }

  #[test]
  fn can_parse_addresses() -> Result<()> {
    let address = AccountStore::to_address(&Alice.to_account_id());
    assert_eq!(
      AccountStore::parse_address(&address)?,
      Alice.to_account_id()
    );
    Ok(())
  }
}
