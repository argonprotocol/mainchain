use std::sync::Arc;

use chrono::NaiveDateTime;
use sqlx::{FromRow, SqlitePool};
use tokio::sync::Mutex;

use argon_primitives::{Balance, TransferToLocalchainId};

use crate::{bail, AccountStore, Keystore, LocalchainTransfer, MainchainClient, Result};

#[derive(FromRow, Clone)]
#[allow(dead_code)]
pub struct MainchainTransferIn {
  pub id: i64,
  pub address: String,
  pub amount: String,
  pub transfer_id: i64,
  pub notary_id: i64,
  pub first_block_hash: String,
  pub expiration_tick: Option<i64>,
  pub balance_change_id: Option<i64>,
  pub extrinsic_hash: String,
  pub finalized_block_number: Option<i64>,
  pub created_at: NaiveDateTime,
}

#[cfg_attr(feature = "napi", napi)]
pub struct MainchainTransferStore {
  db: SqlitePool,
  mainchain_client: Arc<Mutex<Option<MainchainClient>>>,
  keystore: Keystore,
}

impl MainchainTransferStore {
  pub fn new(
    db: SqlitePool,
    mainchain_client: Arc<Mutex<Option<MainchainClient>>>,
    keystore: Keystore,
  ) -> Self {
    Self {
      db,
      mainchain_client,
      keystore,
    }
  }

  pub async fn send_to_localchain(
    &self,
    amount: Balance,
    notary_id: Option<u32>,
  ) -> Result<LocalchainTransfer> {
    let Some(ref mainchain_client) = *(self.mainchain_client.lock().await) else {
      bail!("Mainchain client not initialized");
    };
    let mut db = self.db.acquire().await?;
    let account = AccountStore::db_deposit_account(&mut db, notary_id).await?;
    let (transfer, block) = mainchain_client
      .create_transfer_to_localchain(
        account.address.clone(),
        amount,
        account.notary_id,
        &self.keystore,
      )
      .await?;

    let block_hash = hex::encode(block.block_hash().as_ref());
    let ext_hash = hex::encode(block.extrinsic_hash().as_ref());
    let amount_str = amount.to_string();
    let res = sqlx::query!(
      "INSERT INTO mainchain_transfers_in (address, amount, transfer_id, notary_id, expiration_tick, first_block_hash, extrinsic_hash) VALUES (?, ?, ?, ?, ?, ?, ?)",
      account.address,
      amount_str,
      transfer.transfer_id,
      account.notary_id,
      transfer.expiration_tick,
      block_hash,
      ext_hash
    )
    .execute(&mut *db)
    .await
    ?;
    if res.rows_affected() != 1 {
      bail!("Error storing mainchain transfer");
    }
    Ok(transfer)
  }

  pub async fn get(&self, transfer_id: TransferToLocalchainId) -> Result<MainchainTransferIn> {
    let mut db = self.db.acquire().await?;
    let transfer_id = transfer_id as i64;
    let transfer = sqlx::query_as!(
      MainchainTransferIn,
      "SELECT * FROM mainchain_transfers_in WHERE transfer_id = ?",
      transfer_id
    )
    .fetch_one(&mut *db)
    .await?;
    Ok(transfer)
  }

  pub async fn update_finalization(&self) -> Result<()> {
    let Some(ref mainchain_client) = *(self.mainchain_client.lock().await) else {
      bail!("Mainchain client not initialized");
    };
    let mut db = self.db.acquire().await?;
    let transfers = sqlx::query_as!(
      MainchainTransferIn,
      "SELECT * FROM mainchain_transfers_in WHERE finalized_block_number IS NULL ORDER BY created_at LIMIT 1"
    ).fetch_all(&mut *db).await?;

    for transfer in transfers {
      let finalized_block_number = mainchain_client
        .get_transfer_to_localchain_finalized_block(transfer.transfer_id as u32)
        .await?;
      if let Some(finalized_block_number) = finalized_block_number {
        let res = sqlx::query!(
          "UPDATE mainchain_transfers_in SET finalized_block_number = ? WHERE id = ?",
          finalized_block_number,
          transfer.id
        )
        .execute(&mut *db)
        .await?;
        if res.rows_affected() != 1 {
          bail!("Error updating mainchain transfer");
        }
      }
    }

    Ok(())
  }

  pub async fn find_ready_for_claim(&self) -> Result<Vec<MainchainTransferIn>> {
    let mut db = self.db.acquire().await?;
    let res = sqlx::query_as!(
        MainchainTransferIn,
        "SELECT * FROM mainchain_transfers_in WHERE finalized_block_number IS NOT NULL AND balance_change_id IS NULL ORDER BY created_at LIMIT 1",
      )
      .fetch_all(&mut *db)
      .await
      ?;
    Ok(res)
  }

  pub async fn record_balance_change_id(
    &self,
    local_id: i64,
    balance_change_id: i64,
  ) -> Result<()> {
    let mut db = self.db.acquire().await?;
    let res = sqlx::query!(
      "UPDATE mainchain_transfers_in SET balance_change_id = ? WHERE id = ?",
      balance_change_id,
      local_id
    )
    .execute(&mut *db)
    .await?;
    if res.rows_affected() != 1 {
      bail!("Error updating mainchain transfer");
    }
    Ok(())
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use crate::error::NapiOk;
  use napi::bindgen_prelude::BigInt;

  use crate::mainchain_client::napi_ext::LocalchainTransfer;
  use crate::mainchain_transfer::MainchainTransferStore;

  #[napi]
  impl MainchainTransferStore {
    #[napi(js_name = "sendToLocalchain")]
    pub async fn send_to_localchain_napi(
      &self,
      amount: BigInt,
      notary_id: Option<u32>,
    ) -> napi::Result<LocalchainTransfer> {
      let transfer = self
        .send_to_localchain(amount.get_u128().1, notary_id)
        .await
        .napi_ok()?;
      Ok(LocalchainTransfer {
        address: transfer.address,
        amount: transfer.amount.into(),
        notary_id: transfer.notary_id,
        expiration_tick: transfer.expiration_tick,
        transfer_id: transfer.transfer_id,
      })
    }
  }
}
