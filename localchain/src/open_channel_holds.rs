use crate::accounts::AccountStore;
use crate::balance_changes::BalanceChangeStore;
use crate::keystore::Keystore;
use crate::notary_client::NotaryClients;
use crate::{bail, BalanceChangeStatus, Error, Result};
use crate::{TickerRef, CHANNEL_HOLD_MINIMUM_SETTLEMENT};
use anyhow::anyhow;
use argon_notary_audit::verify_changeset_signatures;
use argon_primitives::{
  AccountType, Balance, BalanceChange, BalanceTip, MultiSignatureBytes, NoteType, NotebookNumber,
  CHANNEL_HOLD_CLAWBACK_TICKS, MINIMUM_CHANNEL_HOLD_SETTLEMENT,
};
use bech32::{Bech32m, Hrp};
use chrono::NaiveDateTime;
use codec::Encode;
use lazy_static::lazy_static;
use sp_core::ed25519::Signature;
use sp_core::Decode;
use sp_runtime::MultiSignature;
use sqlx::{FromRow, SqliteConnection, SqlitePool};
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static! {
  pub static ref EMPTY_SIGNATURE: Vec<u8> = MultiSignature::from(Signature::from_raw([0; 64]))
    .encode()
    .to_vec();
}

#[derive(FromRow, Clone)]
#[allow(dead_code)]
struct ChannelHoldRow {
  id: String,
  initial_balance_change_json: String,
  from_address: String,
  delegated_signer_address: Option<String>,
  balance_change_number: i64,
  expiration_tick: i64,
  settled_amount: String,
  settled_signature: Vec<u8>,
  notarization_id: Option<i64>,
  is_client: bool,
  missed_claim_window: bool,
  created_at: NaiveDateTime,
  updated_at: NaiveDateTime,
}

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone, Debug, PartialEq)]
pub struct ChannelHold {
  pub id: String,
  pub initial_balance_change_json: String,
  pub notary_id: u32,
  hold_amount: u128,
  pub from_address: String,
  pub delegated_signer_address: Option<String>,
  pub to_address: String,
  pub expiration_tick: u32,
  pub balance_change_number: u32,
  pub notarization_id: Option<i64>,
  pub is_client: bool,
  pub missed_claim_window: bool,
  pub(crate) settled_amount: u128,
  settled_signature: Vec<u8>,
  balance_change: BalanceChange,
}

impl ChannelHold {
  pub fn hold_amount(&self) -> u128 {
    self.hold_amount
  }
  pub fn settled_amount(&self) -> u128 {
    self.settled_amount
  }

  pub fn settled_signature(&self) -> Vec<u8> {
    self.settled_signature.clone()
  }

  pub fn is_past_claim_period(&self, current_tick: u32) -> bool {
    current_tick > self.expiration_tick + CHANNEL_HOLD_CLAWBACK_TICKS
  }

  pub fn get_initial_balance_change(&self) -> BalanceChange {
    self.balance_change.clone()
  }

  pub fn create_channel_hold_id(balance_change: &BalanceChange) -> Result<String> {
    let mut balance_change = balance_change.clone();
    // set to minimum for id
    balance_change.notes[0].milligons = MINIMUM_CHANNEL_HOLD_SETTLEMENT;
    balance_change.balance = balance_change
      .previous_balance_proof
      .as_ref()
      .map(|a| a.balance)
      .unwrap_or_default()
      .saturating_sub(MINIMUM_CHANNEL_HOLD_SETTLEMENT);
    let Ok(hrp) = Hrp::parse("esc") else {
      bail!("Failed to parse internal bech32 encoding hrp");
    };
    let id =
      bech32::encode::<Bech32m>(hrp, balance_change.hash().as_ref()).map_err(|e| anyhow!(e))?;
    Ok(id)
  }

  pub fn try_from_balance_change_json(balance_change_json: String) -> Result<ChannelHold> {
    let balance_change: BalanceChange = serde_json::from_str(&balance_change_json)?;
    let Some(ref channel_hold_note) = balance_change.channel_hold_note else {
      bail!("Balance change has no channel hold note");
    };
    if channel_hold_note.milligons < MINIMUM_CHANNEL_HOLD_SETTLEMENT {
      bail!(
        "Channel hold note {} is less than minimum settlement amount: {}",
        channel_hold_note.milligons,
        MINIMUM_CHANNEL_HOLD_SETTLEMENT
      );
    }

    let (recipient, delegated_signer) = match &channel_hold_note.note_type {
      NoteType::ChannelHold {
        recipient,
        delegated_signer,
      } => (recipient, delegated_signer),
      _ => {
        bail!(
          "Balance change has invalid channel hold note type {:?}",
          channel_hold_note.note_type
        );
      }
    };

    if balance_change.account_type != AccountType::Deposit {
      bail!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      );
    }

    if balance_change.notes.len() != 1 {
      bail!(
        "Balance change has {} notes, expected 1",
        balance_change.notes.len()
      );
    }
    let settle_note = &balance_change.notes[0];
    if settle_note.note_type != NoteType::ChannelHoldSettle {
      bail!(
        "Balance change doesn't have a ChannelHoldSettle note. It is: {:?}",
        settle_note.note_type
      );
    }
    let Some(proof) = &balance_change.previous_balance_proof else {
      bail!("Balance change has no proof");
    };

    let id = ChannelHold::create_channel_hold_id(&balance_change)?;

    Ok(ChannelHold {
      id,
      is_client: false,
      initial_balance_change_json: balance_change_json,
      hold_amount: channel_hold_note.milligons,
      from_address: AccountStore::to_address(&balance_change.account_id),
      to_address: AccountStore::to_address(recipient),
      delegated_signer_address: delegated_signer.as_ref().map(AccountStore::to_address),
      balance_change_number: balance_change.change_number,
      notary_id: proof.notary_id,
      expiration_tick: 0,
      settled_amount: settle_note.milligons,
      settled_signature: balance_change.signature.encode(),
      notarization_id: None,
      missed_claim_window: false,
      balance_change,
    })
  }

  pub fn hold_notebook_number(&self) -> NotebookNumber {
    self
      .balance_change
      .previous_balance_proof
      .as_ref()
      .map(|p| p.notebook_number)
      .unwrap_or_default()
  }

  pub async fn get_final(&self) -> Result<BalanceChange> {
    let mut balance_change = self.get_change_with_settled_amount(self.settled_amount);
    if self.settled_signature.is_empty() || self.settled_signature == *EMPTY_SIGNATURE {
      bail!("ChannelHold settlement has not been signed");
    }
    balance_change.signature = MultiSignatureBytes::decode(&mut self.settled_signature.as_slice())?;
    verify_changeset_signatures(&vec![balance_change.clone()])?;
    Ok(balance_change)
  }

  pub async fn db_insert(&mut self, db: &mut SqliteConnection) -> Result<()> {
    let settled_amount = self.settled_amount.to_string();
    let balance_change_number = self.balance_change_number as i64;
    let from_address = self.from_address.clone();

    let res = sqlx::query!(
      r#"INSERT INTO open_channel_holds
      (id, initial_balance_change_json, from_address, balance_change_number, expiration_tick, settled_amount, settled_signature, is_client)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
      self.id,
      self.initial_balance_change_json,
      from_address,
      balance_change_number,
      self.expiration_tick,
      settled_amount,
      self.settled_signature,
      self.is_client,
    )
    .execute(&mut *db)
    .await?;
    if res.rows_affected() != 1 {
      bail!("Failed to insert channel_hold");
    }

    Ok(())
  }

  fn get_change_with_settled_amount(&self, amount: u128) -> BalanceChange {
    let mut balance_change = self.balance_change.clone();
    balance_change.notes[0].milligons = amount;
    balance_change.balance = balance_change
      .previous_balance_proof
      .as_ref()
      .map(|p| p.balance)
      .unwrap_or_default()
      .saturating_sub(amount);
    balance_change
  }

  pub async fn db_update_signature(
    &mut self,
    db: &mut SqliteConnection,
    milligons: Balance,
    signature: Vec<u8>,
  ) -> Result<()> {
    let mut balance_change = self.get_change_with_settled_amount(milligons);
    balance_change.signature = MultiSignatureBytes::decode(&mut signature.as_slice())?;
    verify_changeset_signatures(&vec![balance_change.clone()])?;

    self.settled_amount = milligons;
    self.settled_signature = signature;
    let settled_amount = milligons.to_string();
    let id = &self.id;
    let res = sqlx::query!(
      "UPDATE open_channel_holds SET settled_amount=?, settled_signature = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
      settled_amount,
      self.settled_signature,
      id,
    )
    .execute(&mut *db)
    .await?;
    if res.rows_affected() != 1 {
      bail!("Failed to update channel_hold");
    }
    Ok(())
  }

  pub async fn db_mark_unable_to_claim(&mut self, db: &mut SqliteConnection) -> Result<()> {
    let res = sqlx::query!(
      "UPDATE open_channel_holds SET missed_claim_window = true, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
      self.id,
    )
        .execute(&mut *db)
        .await?;
    if res.rows_affected() != 1 {
      bail!("Failed to update channel_hold");
    }
    self.missed_claim_window = true;
    Ok(())
  }

  pub async fn db_mark_notarized(
    &mut self,
    db: &mut SqliteConnection,
    notarization_id: i64,
  ) -> Result<()> {
    self.notarization_id = Some(notarization_id);
    let id = &self.id;
    let res = sqlx::query!(
      "UPDATE open_channel_holds SET notarization_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
      notarization_id,
      id,
    )
    .execute(&mut *db)
    .await?;
    if res.rows_affected() != 1 {
      bail!("Failed to update channel_hold");
    }
    Ok(())
  }
}

impl TryFrom<ChannelHoldRow> for ChannelHold {
  type Error = Error;
  fn try_from(row: ChannelHoldRow) -> Result<Self> {
    let mut channel_hold =
      ChannelHold::try_from_balance_change_json(row.initial_balance_change_json)?;

    channel_hold.expiration_tick = row.expiration_tick as u32;
    channel_hold.settled_amount = row.settled_amount.parse()?;
    channel_hold.settled_signature = row.settled_signature;
    channel_hold.notarization_id = row.notarization_id;
    channel_hold.is_client = row.is_client;
    channel_hold.missed_claim_window = row.missed_claim_window;
    Ok(channel_hold)
  }
}

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone)]
pub struct OpenChannelHold {
  db: SqlitePool,
  channel_hold: Arc<Mutex<ChannelHold>>,
  keystore: Keystore,
}

impl OpenChannelHold {
  pub fn new(db: SqlitePool, channel_hold: ChannelHold, keystore: &Keystore) -> Self {
    OpenChannelHold {
      db,
      channel_hold: Arc::new(Mutex::new(channel_hold)),
      keystore: keystore.clone(),
    }
  }

  pub async fn channel_hold(&self) -> ChannelHold {
    self.channel_hold.lock().await.clone()
  }

  pub async fn sign(&self, settled_amount: Balance) -> Result<SignatureResult> {
    if settled_amount < MINIMUM_CHANNEL_HOLD_SETTLEMENT {
      bail!("Settled amount must be greater than the minimum channel_hold settlement amount ({MINIMUM_CHANNEL_HOLD_SETTLEMENT})");
    }
    let mut channel_hold = self.channel_hold.lock().await;
    let mut tx = self.db.begin().await?;
    let balance_change = channel_hold.get_change_with_settled_amount(settled_amount);
    let bytes = balance_change.hash();

    let signature = self
      .keystore
      .sign(channel_hold.from_address.clone(), bytes.as_bytes().to_vec())
      .await?;

    channel_hold
      .db_update_signature(&mut tx, settled_amount, signature.to_vec())
      .await?;

    tx.commit().await?;

    Ok(SignatureResult {
      signature,
      milligons: settled_amount,
    })
  }

  pub async fn export_for_send(&self) -> Result<String> {
    {
      let channel_hold = self.channel_hold.lock().await;
      let is_empty = channel_hold.settled_signature == *EMPTY_SIGNATURE;
      drop(channel_hold);
      if is_empty {
        self.sign(CHANNEL_HOLD_MINIMUM_SETTLEMENT).await?;
      }
    }
    let channel_hold = self.channel_hold.lock().await;
    let balance_change = channel_hold.get_final().await?;
    let json = serde_json::to_string(&balance_change)?;
    Ok(json)
  }

  pub async fn record_updated_settlement(
    &self,
    milligons: Balance,
    signature: Vec<u8>,
  ) -> Result<()> {
    if milligons < MINIMUM_CHANNEL_HOLD_SETTLEMENT {
      bail!("Settled amount is less than minimum channel_hold settlement amount ({MINIMUM_CHANNEL_HOLD_SETTLEMENT})");
    }
    let mut channel_hold = self.channel_hold.lock().await;
    let mut db = self.db.acquire().await?;
    channel_hold
      .db_update_signature(&mut db, milligons, signature)
      .await?;

    Ok(())
  }

  pub async fn inner(&self) -> ChannelHold {
    self.channel_hold.lock().await.clone()
  }

  pub async fn reload(&self) -> Result<()> {
    let id = self.channel_hold.lock().await.id.clone();
    let channel_hold = ChannelHold::try_from(
      sqlx::query_as!(
        ChannelHoldRow,
        "SELECT * FROM open_channel_holds WHERE id = ?",
        id
      )
      .fetch_one(&self.db)
      .await?,
    )?;
    *self.channel_hold.lock().await = channel_hold;
    Ok(())
  }
}

#[cfg_attr(feature = "napi", napi)]
pub struct OpenChannelHoldsStore {
  db: SqlitePool,
  ticker: TickerRef,
  notary_clients: NotaryClients,
  keystore: Keystore,
}

impl OpenChannelHoldsStore {
  pub(crate) fn new(
    db: SqlitePool,
    ticker: TickerRef,
    notary_clients: &NotaryClients,
    keystore: &Keystore,
  ) -> Self {
    Self {
      db,
      ticker,
      notary_clients: notary_clients.clone(),
      keystore: keystore.clone(),
    }
  }

  pub async fn get(&self, id: String) -> Result<OpenChannelHold> {
    let row = sqlx::query_as!(
      ChannelHoldRow,
      "SELECT * FROM open_channel_holds WHERE id = ?",
      id
    )
    .fetch_one(&self.db)
    .await?;

    let channel_hold = ChannelHold::try_from(row)?;

    Ok(self.open(&channel_hold))
  }

  pub fn open(&self, channel_hold: &ChannelHold) -> OpenChannelHold {
    OpenChannelHold::new(self.db.clone(), channel_hold.clone(), &self.keystore)
  }

  pub async fn db_record_notarized(
    db: &mut SqliteConnection,
    balance_change: &BalanceChange,
    notarization_id: i64,
  ) -> Result<()> {
    let address = AccountStore::to_address(&balance_change.account_id);
    let settled_amount = balance_change.notes[0].milligons.to_string();
    let res = sqlx::query!(
      r#"UPDATE open_channel_holds SET notarization_id = ?, settled_amount = ?, updated_at = CURRENT_TIMESTAMP
       WHERE from_address = ? AND balance_change_number = ?"#,
      notarization_id,
      settled_amount,
      address,
      balance_change.change_number,
    )
    .execute(db)
    .await?;
    if res.rows_affected() != 1 {
      bail!("Failed to update channel_hold");
    }
    Ok(())
  }

  pub async fn get_claimable(&self) -> Result<Vec<OpenChannelHold>> {
    let current_tick = self.ticker.current();
    let expired = sqlx::query_as!(
      ChannelHoldRow,
      r#"SELECT * FROM open_channel_holds WHERE notarization_id IS NULL AND missed_claim_window = false AND expiration_tick <= $1"#,
      current_tick,
    )
    .fetch_all(&self.db)
    .await
    ?;
    tracing::info!("Found {} claimable channel_holds", expired.len());

    let mut channel_holds = vec![];
    for row in expired.into_iter() {
      let channel_hold = ChannelHold::try_from(row)?;
      channel_holds.push(OpenChannelHold::new(
        self.db.clone(),
        channel_hold,
        &self.keystore,
      ))
    }
    tracing::info!("return channel_holds {}", channel_holds.len());
    Ok(channel_holds)
  }

  /// Import a channel_hold from a JSON string. Verifies with the notary that the channel hold is valid.
  pub async fn import_channel_hold(&self, channel_hold_json: String) -> Result<OpenChannelHold> {
    let mut channel_hold = ChannelHold::try_from_balance_change_json(channel_hold_json)?;
    verify_changeset_signatures(&vec![channel_hold.balance_change.clone()])?;
    let mut db = self.db.acquire().await?;
    let default_account =
      AccountStore::db_deposit_account(&mut db, Some(channel_hold.notary_id)).await?;
    if default_account.address != channel_hold.to_address {
      bail!(
        "This localchain is not configured to accept payments addressed to {}",
        channel_hold.to_address,
      );
    }

    let notary_client = self.notary_clients.get(channel_hold.notary_id).await?;

    let balance_tip = notary_client
      .get_balance_tip(channel_hold.from_address.clone(), AccountType::Deposit)
      .await?;

    let Some(balance_proof) = &channel_hold.balance_change.previous_balance_proof else {
      bail!("Balance change has no previous balance proof");
    };

    let calculated_tip = BalanceTip::compute_tip(
      channel_hold.balance_change.change_number.saturating_sub(1),
      balance_proof.balance,
      balance_proof.account_origin.clone(),
      channel_hold.balance_change.channel_hold_note.clone(),
    );

    let current_tip = balance_tip.balance_tip.as_ref();

    if calculated_tip != current_tip {
      bail!(
        "Balance tip mismatch. Expected {:#x?}, got {:#x?}",
        calculated_tip,
        current_tip
      );
    }
    channel_hold.expiration_tick = balance_tip.tick + self.ticker.channel_hold_expiration_ticks();
    channel_hold.db_insert(&mut db).await?;
    Ok(OpenChannelHold::new(
      self.db.clone(),
      channel_hold,
      &self.keystore,
    ))
  }

  /// Create a new channel_hold as a client. You must first notarize a channel hold note to the notary for the `client_address`.
  pub async fn open_client_channel_hold(&self, account_id: i64) -> Result<OpenChannelHold> {
    let mut tx = self.db.begin().await?;
    let account = AccountStore::db_get_by_id(&mut tx, account_id).await?;
    let (mut balance_tip, status) =
      BalanceChangeStore::db_build_for_account(&mut tx, &account).await?;
    if status == Some(BalanceChangeStatus::WaitingForSendClaim) {
      bail!(
        "This balance change is not in a state to open {}: {:?}",
        account.address,
        status
      );
    }

    let hold_note = &balance_tip.channel_hold_note.clone().ok_or(anyhow!(
      "Account {} has no channel hold note",
      account.address
    ))?;

    let (recipient, delegated_signer) = match &hold_note.note_type {
      NoteType::ChannelHold {
        recipient,
        delegated_signer,
      } => (recipient, delegated_signer),
      _ => {
        bail!(
          "Balance change has invalid channel hold note type {:?}",
          hold_note.note_type
        );
      }
    };

    let (notary_id, tick) = &balance_tip
      .previous_balance_proof
      .clone()
      .map(|p| (p.notary_id, p.tick))
      .ok_or(anyhow!("Balance change has no previous balance proof"))?;

    balance_tip.change_number += 1;
    balance_tip.push_note(MINIMUM_CHANNEL_HOLD_SETTLEMENT, NoteType::ChannelHoldSettle);
    balance_tip.balance -= MINIMUM_CHANNEL_HOLD_SETTLEMENT;

    let id = ChannelHold::create_channel_hold_id(&balance_tip)?;

    let mut channel_hold = ChannelHold {
      id,
      is_client: true,
      initial_balance_change_json: serde_json::to_string(&balance_tip)?,
      balance_change_number: balance_tip.change_number,
      hold_amount: hold_note.milligons,
      from_address: account.address,
      delegated_signer_address: delegated_signer.as_ref().map(AccountStore::to_address),
      to_address: AccountStore::to_address(recipient),
      notary_id: *notary_id,
      expiration_tick: tick + self.ticker.channel_hold_expiration_ticks(),
      settled_amount: MINIMUM_CHANNEL_HOLD_SETTLEMENT,
      settled_signature: EMPTY_SIGNATURE.clone(),
      notarization_id: None,
      balance_change: balance_tip,
      missed_claim_window: false,
    };
    channel_hold.db_insert(&mut tx).await?;
    tx.commit().await?;

    Ok(OpenChannelHold::new(
      self.db.clone(),
      channel_hold,
      &self.keystore,
    ))
  }
}

pub struct SignatureResult {
  pub signature: Vec<u8>,
  pub milligons: Balance,
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use super::{ChannelHold, OpenChannelHold, OpenChannelHoldsStore};
  use crate::error::NapiOk;
  use napi::bindgen_prelude::{BigInt, Uint8Array};

  #[napi]
  impl ChannelHold {
    #[napi(getter, js_name = "holdAmount")]
    pub fn hold_amount_napi(&self) -> BigInt {
      BigInt::from(self.hold_amount)
    }
    #[napi(getter, js_name = "settledAmount")]
    pub fn settled_amount_napi(&self) -> BigInt {
      BigInt::from(self.settled_amount)
    }
    #[napi(getter, js_name = "settledSignature")]
    pub fn settled_signature_napi(&self) -> Uint8Array {
      Uint8Array::from(self.settled_signature.clone())
    }
    #[napi(js_name = "isPastClaimPeriod")]
    pub fn is_past_claim_period_napi(&self, current_tick: u32) -> bool {
      self.is_past_claim_period(current_tick)
    }
  }

  #[napi(object)]
  pub struct SignatureResult {
    pub signature: Uint8Array,
    pub milligons: BigInt,
  }

  #[napi]
  impl OpenChannelHold {
    #[napi(getter, js_name = "channelHold")]
    pub async fn channel_hold_napi(&self) -> ChannelHold {
      self.channel_hold().await
    }
    #[napi(js_name = "sign")]
    pub async fn sign_napi(&self, settled_amount: BigInt) -> napi::Result<SignatureResult> {
      let result = self.sign(settled_amount.get_u128().1).await.napi_ok()?;
      Ok(SignatureResult {
        signature: result.signature.into(),
        milligons: result.milligons.into(),
      })
    }
    #[napi(js_name = "exportForSend")]
    pub async fn export_for_send_napi(&self) -> napi::Result<String> {
      self.export_for_send().await.napi_ok()
    }

    #[napi(js_name = "recordUpdatedSettlement")]
    pub async fn record_updated_settlement_napi(
      &self,
      milligons: BigInt,
      signature: Uint8Array,
    ) -> napi::Result<()> {
      self
        .record_updated_settlement(milligons.get_u128().1, signature.to_vec())
        .await
        .napi_ok()
    }
  }

  #[napi]
  impl OpenChannelHoldsStore {
    #[napi(js_name = "get")]
    pub async fn get_napi(&self, id: String) -> napi::Result<OpenChannelHold> {
      self.get(id).await.napi_ok()
    }
    #[napi(js_name = "open")]
    pub fn open_napi(&self, channel_hold: &ChannelHold) -> OpenChannelHold {
      self.open(channel_hold)
    }

    #[napi(js_name = "getClaimable")]
    pub async fn get_claimable_napi(&self) -> napi::Result<Vec<OpenChannelHold>> {
      self.get_claimable().await.napi_ok()
    }
    #[napi(js_name = "importChannelHold")]
    /// Import a channel_hold from a JSON string. Verifies with the notary that the channel hold is valid.
    pub async fn import_channel_hold_napi(
      &self,
      channel_hold_json: String,
    ) -> napi::Result<OpenChannelHold> {
      self.import_channel_hold(channel_hold_json).await.napi_ok()
    }
    #[napi(js_name = "openClientChannelHold")]
    /// Create a new channel_hold as a client. You must first notarize a channel hold note to the notary for the `client_address`.
    pub async fn open_client_channel_hold_napi(
      &self,
      account_id: i64,
    ) -> napi::Result<OpenChannelHold> {
      self.open_client_channel_hold(account_id).await.napi_ok()
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::balance_change_builder::BalanceChangeBuilder;
  use crate::notarization_builder::NotarizationBuilder;
  use crate::test_utils::{create_mock_notary, mock_notary_clients, MockNotary};
  use crate::transactions::Transactions;
  use crate::*;
  use argon_primitives::tick::Tick;
  use argon_primitives::{AccountId, LocalchainAccountId, Notarization};
  use serde_json::json;
  use sp_core::Pair;
  use sp_keyring::AccountKeyring::{Alice, Charlie};
  use sp_keyring::Ed25519Keyring::Bob;
  use sp_keyring::Ed25519Keyring::Ferdie;

  async fn register_account(
    db: &mut SqliteConnection,
    account_id: AccountId,
    origin_uid: u32,
    origin_notebook: u32,
  ) -> Result<LocalAccount> {
    let address = AccountStore::to_address(&account_id);
    let account = AccountStore::db_insert(db, address, AccountType::Deposit, 1, None).await?;
    AccountStore::db_update_origin(db, account.id, origin_notebook, origin_uid).await?;
    let account = AccountStore::db_get_by_id(db, account.id).await?;
    Ok(account)
  }

  #[allow(clippy::too_many_arguments)]
  async fn create_channel_hold(
    pool: &SqlitePool,
    account: &LocalAccount,
    localchain_transfer_amount: u128,
    hold_amount: u128,
    recipient: String,
    notebook_number: NotebookNumber,
    tick: Tick,
    delegated_signer: Option<String>,
  ) -> Result<BalanceChangeRow> {
    let mut tx = pool.begin().await?;
    let (balance_tip, status) = BalanceChangeStore::db_build_for_account(&mut tx, account).await?;
    let builder = BalanceChangeBuilder::new(balance_tip, account.id, status);
    builder
      .claim_from_mainchain(LocalchainTransfer {
        address: account.address.clone(),
        notary_id: 1,
        amount: localchain_transfer_amount,
        expiration_tick: 100,
        transfer_id: 1,
      })
      .await?;
    builder
      .create_channel_hold(hold_amount, recipient.clone(), delegated_signer)
      .await?;

    let balance_change = builder.inner().await;
    let notarization = Notarization::new(vec![balance_change.clone()], vec![], vec![]);

    let json_notarization = json!(notarization);
    let id = sqlx::query_scalar!(
      "INSERT into notarizations (json, notary_id, notebook_number, tick) VALUES (?, ?, ?, ?) RETURNING id",
      json_notarization,
      1,
      notebook_number,
      tick
    )
        .fetch_one(&mut *tx)
        .await?;
    let id =
      BalanceChangeStore::tx_upsert_notarized(&mut tx, account.id, &balance_change, 1, id, None)
        .await?;
    tx.commit().await?;

    let mut db = pool.acquire().await?;
    let balance_change = BalanceChangeStore::db_get_by_id(&mut db, id).await?;
    Ok(balance_change)
  }

  async fn register_balance_tip(
    account: &LocalAccount,
    mock_notary: &MockNotary,
    balance_change: &BalanceChangeRow,
    notebook_number: NotebookNumber,
    tick: Tick,
  ) -> Result<()> {
    let balance_tip = balance_change.get_balance_tip(account)?;
    println!("got balance tip for account {:?}", balance_tip);
    let mut state = mock_notary.state.lock().await;
    state.balance_tips.insert(
      LocalchainAccountId::new(account.get_account_id32()?, account.account_type),
      argon_notary_apis::localchain::BalanceTipResult {
        tick,
        balance_tip: balance_tip.tip().into(),
        notebook_number,
      },
    );
    Ok(())
  }

  #[sqlx::test]
  async fn test_open_channel_hold(pool: SqlitePool) -> Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;

    let alice_address = AccountStore::to_address(&Alice.to_account_id());
    let mut db = pool.acquire().await?;
    let bob_account = register_account(&mut db, Bob.to_account_id(), 1, 1).await?;

    let _bob_hold = create_channel_hold(
      &pool,
      &bob_account,
      20_000,
      1_000,
      alice_address.clone(),
      1,
      1,
      None,
    )
    .await?;

    let channel_hold_expiration_ticks: u32 = 2;
    let ticker = TickerRef::new(Ticker::start(
      Duration::from_secs(60),
      channel_hold_expiration_ticks,
    ));
    println!("about to open channel_hold");
    let keystore = Keystore::new(pool.clone());
    let _ = keystore
      .import_suri(Bob.to_seed(), CryptoScheme::Ed25519, None)
      .await?;

    let store = OpenChannelHoldsStore::new(pool, ticker, &notary_clients, &keystore);
    let open_channel_hold = store.open_client_channel_hold(bob_account.id).await?;
    println!("opened channel_hold");
    let channel_hold = open_channel_hold.inner().await;
    assert_eq!(channel_hold.to_address.clone(), alice_address);
    assert_eq!(
      channel_hold.expiration_tick,
      1 + channel_hold_expiration_ticks
    );

    assert_eq!(store.get_claimable().await?.len(), 0);
    let json = open_channel_hold.export_for_send().await?;

    println!("channel_hold {}", &json);
    assert!(json.contains("channelHoldNote\":{"));

    assert_eq!(
      store
        .get(channel_hold.id.clone())
        .await?
        .inner()
        .await
        .get_final()
        .await?,
      open_channel_hold.inner().await.get_final().await?,
      "can reload from db"
    );

    open_channel_hold.sign(10u128).await?;

    assert_eq!(
      store
        .get(channel_hold.id)
        .await?
        .inner()
        .await
        .settled_amount(),
      10_u128
    );

    Ok(())
  }

  #[sqlx::test]
  async fn test_open_channel_hold_with_delegated_signer(pool: SqlitePool) -> Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;

    let alice_address = AccountStore::to_address(&Alice.to_account_id());
    let mut db = pool.acquire().await?;
    let bob_account = register_account(&mut db, Bob.to_account_id(), 1, 1).await?;
    let signer_address = AccountStore::to_address(&Charlie.to_account_id());

    let bob_hold = create_channel_hold(
      &pool,
      &bob_account,
      20_000,
      1_000,
      alice_address.clone(),
      1,
      1,
      Some(signer_address.clone()),
    )
    .await?;

    let ticker = TickerRef::new(Ticker::start(Duration::from_secs(60), 2));

    let keystore = Keystore::new(pool.clone());
    let _ = keystore
      .import_suri(Bob.to_seed(), CryptoScheme::Ed25519, None)
      .await?;
    let store = OpenChannelHoldsStore::new(pool, ticker.clone(), &notary_clients, &keystore);
    let open_channel_hold = store.open_client_channel_hold(bob_account.id).await?;
    let channel_hold = open_channel_hold.inner().await;
    assert_eq!(
      channel_hold.delegated_signer_address,
      Some(signer_address.clone())
    );

    let json = open_channel_hold.export_for_send().await?;

    let alice_pool = SqlitePool::connect(":memory:").await?;
    sqlx::migrate!()
      .run(&alice_pool)
      .await
      .map_err(|e| anyhow!("Error migrating database {:?}", e))?;
    let mut alice_db = alice_pool.acquire().await?;

    let alice_store = OpenChannelHoldsStore::new(alice_pool, ticker, &notary_clients, &keystore);
    let _alice_account = register_account(&mut alice_db, Alice.to_account_id(), 1, 1).await?;
    // before registered with notary, should fail
    register_balance_tip(&bob_account, &mock_notary, &bob_hold, 1, 1).await?;

    let alice_channel_hold = alice_store.import_channel_hold(json.clone()).await?;
    let imported_channel_hold = alice_channel_hold.inner().await;
    assert_eq!(
      imported_channel_hold.delegated_signer_address,
      Some(signer_address.clone())
    );

    // simulate signing
    let (updated_signature, updated_total) = {
      let mut balance_change: BalanceChange = serde_json::from_str(&json)?;
      balance_change.balance -= 100;
      assert_eq!(
        balance_change.notes[0].note_type,
        NoteType::ChannelHoldSettle
      );
      balance_change.notes[0].milligons += 100;
      let encoded = balance_change.hash().0;
      let charlie_pair = sp_core::sr25519::Pair::from_string(&Charlie.to_seed(), None)?;
      let signature = charlie_pair.sign(&encoded);
      balance_change.signature = signature.into();
      (balance_change.signature, balance_change.notes[0].milligons)
    };
    assert!(alice_channel_hold
      .record_updated_settlement(updated_total, updated_signature.encode())
      .await
      .is_ok());

    Ok(())
  }
  #[sqlx::test]
  async fn test_importing_channel_hold(bob_pool: SqlitePool) -> Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;

    let alice_pool = SqlitePool::connect(":memory:").await?;
    sqlx::migrate!()
      .run(&alice_pool)
      .await
      .map_err(|e| anyhow!("Error migrating database {:?}", e))?;
    let mut alice_db = alice_pool.acquire().await?;

    let alice_address = AccountStore::to_address(&Alice.to_account_id());
    let mut bob_db = bob_pool.acquire().await?;
    let bob_account = register_account(&mut bob_db, Bob.to_account_id(), 1, 1).await?;

    let _alice_account = register_account(&mut alice_db, Alice.to_account_id(), 1, 1).await?;
    let bob_hold = create_channel_hold(
      &bob_pool,
      &bob_account,
      20_000,
      1_000,
      alice_address.clone(),
      1,
      1,
      None,
    )
    .await?;

    let channel_hold_expiration_ticks: u32 = 2;
    let ticker: TickerRef =
      Ticker::start(Duration::from_secs(60), channel_hold_expiration_ticks).into();
    let keystore = Keystore::new(bob_pool.clone());
    keystore
      .import_suri("//Bob".to_string(), CryptoScheme::Ed25519, None)
      .await?;
    let bob_store =
      OpenChannelHoldsStore::new(bob_pool, ticker.clone(), &notary_clients, &keystore);
    let open_channel_hold = bob_store.open_client_channel_hold(bob_account.id).await?;

    let json = open_channel_hold.export_for_send().await?;

    let alice_store =
      OpenChannelHoldsStore::new(alice_pool, ticker.clone(), &notary_clients, &keystore);
    // before registered with notary, should fail
    match alice_store.import_channel_hold(json.clone()).await {
      Err(e) => {
        assert!(e.to_string().contains("balance_tip not set"))
      }
      Ok(_) => {
        bail!("Expected error");
      }
    }
    println!("registering balance tip");
    register_balance_tip(&bob_account, &mock_notary, &bob_hold, 1, 1).await?;

    println!("importing channel_hold");
    let alice_channel_hold = alice_store.import_channel_hold(json).await?;
    println!("imported channel_hold");
    let imported_channel_hold = alice_channel_hold.inner().await;
    let sent_channel_hold = open_channel_hold.inner().await;
    assert_eq!(
      imported_channel_hold.to_address,
      sent_channel_hold.to_address
    );
    assert_eq!(
      imported_channel_hold.from_address,
      sent_channel_hold.from_address
    );
    assert_eq!(
      imported_channel_hold.expiration_tick,
      sent_channel_hold.expiration_tick
    );
    assert_eq!(
      imported_channel_hold.settled_amount,
      sent_channel_hold.settled_amount
    );
    assert_eq!(
      imported_channel_hold.settled_signature,
      sent_channel_hold.settled_signature
    );
    assert_eq!(
      imported_channel_hold.hold_amount,
      sent_channel_hold.hold_amount
    );
    assert_eq!(imported_channel_hold.notary_id, sent_channel_hold.notary_id);
    assert_eq!(
      imported_channel_hold.balance_change_number,
      sent_channel_hold.balance_change_number
    );

    assert_eq!(
      imported_channel_hold.expiration_tick,
      1 + channel_hold_expiration_ticks
    );
    assert_eq!(
      imported_channel_hold.settled_amount,
      MINIMUM_CHANNEL_HOLD_SETTLEMENT
    );
    assert_eq!(imported_channel_hold.id, sent_channel_hold.id);

    let result = open_channel_hold.sign(10u128).await?;
    alice_channel_hold
      .record_updated_settlement(result.milligons, result.signature)
      .await?;
    assert_eq!(alice_channel_hold.inner().await.settled_amount(), 10_u128);

    Ok(())
  }

  #[sqlx::test]
  async fn test_will_not_import_if_not_own_account(bob_pool: SqlitePool) -> Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;

    let alice_pool = SqlitePool::connect(":memory:").await?;
    sqlx::migrate!()
      .run(&alice_pool)
      .await
      .map_err(|e| anyhow!("Error migrating database {:?}", e))?;

    let not_alice = AccountStore::to_address(&Ferdie.to_account_id());

    let bob_signer = Keystore::new(bob_pool.clone());
    let bob_address = bob_signer
      .import_suri("//Bob".to_string(), CryptoScheme::Ed25519, None)
      .await?;

    let ticker = TickerRef::new(Ticker::start(Duration::from_secs(1), 2));
    let builder = NotarizationBuilder::new(
      bob_pool.clone(),
      notary_clients.clone(),
      bob_signer.clone(),
      ticker.clone(),
    );
    builder
      .claim_from_mainchain(LocalchainTransfer {
        address: bob_address.clone(),
        notary_id: 1,
        amount: 2_000u128,
        expiration_tick: 100,
        transfer_id: 1,
      })
      .await?;

    builder.notarize().await?;
    let bob_account = builder.default_deposit_account().await?;
    let accounts = builder.accounts().await;
    let bob_account = accounts
      .iter()
      .find(|a| a.id == bob_account.local_account_id)
      .expect("should get");

    let transactions = Transactions::new(
      bob_pool.clone(),
      ticker.clone(),
      &notary_clients,
      &bob_signer,
    );

    let channel_hold = transactions
      .create_channel_hold(800u128, not_alice.clone(), None, None)
      .await?;
    let json = channel_hold.export_for_send().await?;

    let mut db = bob_pool.acquire().await?;
    let bob_hold = BalanceChangeStore::db_get_latest_for_account(&mut db, bob_account.id)
      .await?
      .expect("should have a latest");
    register_balance_tip(bob_account, &mock_notary, &bob_hold, 1, 1).await?;

    let alice_signer = Keystore::new(alice_pool.clone());
    let _ = alice_signer
      .import_suri("//Alice".to_string(), CryptoScheme::Sr25519, None)
      .await?;
    let alice_store =
      OpenChannelHoldsStore::new(alice_pool, ticker, &notary_clients, &alice_signer);

    let result = alice_store.import_channel_hold(json.clone()).await;
    assert!(result.is_err());
    println!("{:?}", result.as_ref().err());
    assert!(result
      .err()
      .expect("")
      .to_string()
      .contains("This localchain is not configured to accept payments addressed "));
    Ok(())
  }
}
