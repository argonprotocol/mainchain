use binary_merkle_tree::{verify_proof, Leaf};
use chrono::NaiveDateTime;
use codec::Encode;
use napi::bindgen_prelude::*;
use serde_json::{from_value, json};
use sp_core::{bounded_vec, ed25519, H256};
use sp_runtime::traits::BlakeTwo256;
use sqlx::{FromRow, Sqlite, SqliteConnection, SqlitePool, Transaction};

use ulx_primitives::tick::Tick;
use ulx_primitives::{
  BalanceChange, BalanceProof, BalanceTip, MerkleProof, NotaryId, NotebookNumber,
};

use crate::accounts::{AccountStore, LocalAccount};
use crate::to_js_error;

#[derive(FromRow, Clone, Debug)]
#[allow(dead_code)]
#[napi(js_name = "BalanceChange")]
pub struct BalanceChangeRow {
  pub id: i64,
  pub account_id: i64,
  pub change_number: i64,
  pub balance: String,
  pub net_balance_change: String,
  pub escrow_hold_note_json: Option<String>,
  pub notary_id: i64,
  pub notes_json: String,
  pub proof_json: Option<String>,
  pub finalized_block_number: Option<i64>,
  pub status: BalanceChangeStatus,
  pub transaction_id: Option<i64>,
  pub(crate) timestamp: NaiveDateTime,
  pub notarization_id: Option<i64>,
}

impl BalanceChangeRow {
  pub fn get_balance_tip(&self, account: &LocalAccount) -> anyhow::Result<BalanceTip> {
    let origin = account
      .origin
      .clone()
      .ok_or_else(|| anyhow::anyhow!("Account {} has no origin", account.address))?;

    let escrow_hold_note = match &self.escrow_hold_note_json {
      Some(s) => Some(serde_json::from_str(s)?),
      None => None,
    };

    Ok(BalanceTip {
      account_type: account.account_type.clone(),
      account_id: account.get_account_id32()?,
      change_number: self.change_number as u32,
      balance: self.balance.parse()?,
      account_origin: origin.into(),
      escrow_hold_note,
    })
  }

  pub fn get_proof(&self) -> anyhow::Result<Option<MerkleProof>> {
    if let Some(proof_json) = self.proof_json.clone() {
      let proof: MerkleProof = serde_json::from_str(&proof_json)?;
      return Ok(Some(proof));
    }
    Ok(None)
  }

  pub fn verify_balance_proof(
    &self,
    account: &LocalAccount,
    account_change_root: H256,
  ) -> anyhow::Result<bool> {
    let tip = self.get_balance_tip(account)?;

    let notebook_proof = self
      .get_proof()?
      .ok_or_else(|| anyhow::anyhow!("Balance change {} has no proof", self.change_number))?;
    let result = verify_proof::<'_, BlakeTwo256, _, _>(
      &H256::from_slice(&account_change_root[..]),
      notebook_proof.proof.clone().into_inner(),
      notebook_proof.number_of_leaves as usize,
      notebook_proof.leaf_index as usize,
      Leaf::Value(&tip.encode()),
    );
    if !result {
      Err(anyhow::anyhow!(
        "Invalid proof recorded when tried against mainchain change root"
      ))?;
    }
    Ok(result)
  }
}

#[derive(Debug, PartialOrd, PartialEq)]
#[napi(string_enum)]
pub enum BalanceChangeStatus {
  /// The balance change has been submitted, but is not in a known notebook yet.
  SubmittedToNotary,
  /// A balance change that doesn't get final proof because it is one of many in a single notebook. Aka, another balance change superseded it in the notebook.
  SupersededInNotebook,
  /// Proof has been obtained from a notebook
  NotebookPublished,
  /// The mainchain has finalized the notebook with the balance change
  MainchainFinal,
  /// A balance change has been sent to another user to claim. Keep checking until it is claimed.
  WaitingForSendClaim,
  /// A pending balance change that was canceled before being claimed by another user (escrow or send).
  Canceled,
}

impl From<i64> for BalanceChangeStatus {
  fn from(i: i64) -> Self {
    match i {
      0 => BalanceChangeStatus::SubmittedToNotary,
      1 => BalanceChangeStatus::SupersededInNotebook,
      2 => BalanceChangeStatus::NotebookPublished,
      3 => BalanceChangeStatus::MainchainFinal,
      4 => BalanceChangeStatus::WaitingForSendClaim,
      5 => BalanceChangeStatus::Canceled,
      _ => panic!("Unknown balance change status {}", i),
    }
  }
}

#[napi]
pub struct BalanceChangeStore {
  db: SqlitePool,
}

#[napi]
impl BalanceChangeStore {
  pub fn new(db: SqlitePool) -> Self {
    Self { db }
  }

  #[napi(js_name = "allForAccount")]
  pub async fn all_for_account_js(&self, account_id: i64) -> Result<Vec<BalanceChangeRow>> {
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    Self::all_for_account(&mut *db, account_id)
      .await
      .map_err(to_js_error)
  }

  pub async fn all_for_account(
    db: &mut SqliteConnection,
    account_id: i64,
  ) -> Result<Vec<BalanceChangeRow>> {
    let row = sqlx::query_as!(
      BalanceChangeRow,
      "SELECT * FROM balance_changes WHERE account_id = ? ORDER BY change_number DESC",
      account_id
    )
    .fetch_all(db)
    .await
    .map_err(to_js_error)?;
    Ok(row)
  }

  pub async fn find_account_change(
    db: &mut Transaction<'static, Sqlite>,
    account_id: i64,
    balance_change: &BalanceChange,
  ) -> anyhow::Result<Option<BalanceChangeRow>> {
    let res = sqlx::query_as!(
      BalanceChangeRow,
      "SELECT * FROM balance_changes WHERE account_id = ? AND change_number = ?",
      account_id,
      balance_change.change_number,
    )
    .fetch_optional(&mut **db)
    .await?;
    Ok(res)
  }

  pub async fn get_latest_for_account(
    db: &mut SqliteConnection,
    account_id: i64,
  ) -> anyhow::Result<Option<BalanceChangeRow>> {
    let row = sqlx::query_as!(
      BalanceChangeRow,
      "SELECT * FROM balance_changes WHERE account_id = ? AND status <> ? ORDER BY change_number DESC LIMIT 1",
      account_id,
      BalanceChangeStatus::Canceled as i64,
    )
            .fetch_optional(db)
            .await?;
    Ok(row)
  }

  #[napi(js_name = "getLatestForAccount")]
  pub async fn get_latest_for_account_js(
    &self,
    account_id: i64,
  ) -> Result<Option<BalanceChangeRow>> {
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    Self::get_latest_for_account(&mut *db, account_id)
      .await
      .map_err(to_js_error)
  }

  #[napi]
  pub async fn cancel(&self, id: i64) -> Result<()> {
    let mut tx = self.db.begin().await.map_err(to_js_error)?;
    let status = sqlx::query_scalar!("SELECT status FROM balance_changes WHERE id = ?", id)
      .fetch_one(&mut *tx)
      .await
      .map_err(to_js_error)?;
    if status != BalanceChangeStatus::WaitingForSendClaim as i64 {
      return Err(anyhow::anyhow!(
        "Balance change not in correct state - {:?}",
        status
      ))?;
    }

    sqlx::query!(
      "UPDATE balance_changes SET status = ? WHERE id = ?",
      BalanceChangeStatus::Canceled as i64,
      id
    )
    .execute(&mut *tx)
    .await
    .map_err(to_js_error)?;
    tx.commit().await.map_err(to_js_error)?;
    Ok(())
  }

  #[napi(js_name = "getById")]
  pub async fn get_by_id_js(&self, id: i64) -> Result<BalanceChangeRow> {
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    Self::get_by_id(&mut *db, id).await.map_err(to_js_error)
  }

  pub async fn get_by_id(db: &mut SqliteConnection, id: i64) -> anyhow::Result<BalanceChangeRow> {
    let row = sqlx::query_as!(
      BalanceChangeRow,
      "SELECT * FROM balance_changes WHERE id = ?",
      id
    )
    .fetch_one(&mut *db)
    .await?;
    Ok(row)
  }

  #[napi(js_name = "findUnsettled")]
  pub async fn find_unsettled_js(&self) -> Result<Vec<BalanceChangeRow>> {
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    Self::find_unsettled(&mut *db).await.map_err(to_js_error)
  }

  pub async fn find_unsettled(db: &mut SqliteConnection) -> anyhow::Result<Vec<BalanceChangeRow>> {
    let rows = sqlx::query_as!(
      BalanceChangeRow,
      "SELECT * FROM balance_changes WHERE status in (?,?,?) ORDER BY account_id ASC, change_number DESC",
      BalanceChangeStatus::WaitingForSendClaim as i64,
      BalanceChangeStatus::NotebookPublished as i64,
      BalanceChangeStatus::SubmittedToNotary as i64,
    )
            .fetch_all(&mut *db)
            .await?;
    Ok(rows)
  }

  pub async fn get_notarization_notebook(
    db: &mut SqliteConnection,
    notarization_id: i64,
  ) -> anyhow::Result<(Option<NotebookNumber>, Option<Tick>)> {
    let record = sqlx::query!(
      "SELECT notebook_number, tick FROM notarizations WHERE id = ?",
      notarization_id
    )
    .fetch_one(&mut *db)
    .await?;
    Ok((
      record.notebook_number.map(|x| x as NotebookNumber),
      record.tick.map(|x| x as Tick),
    ))
  }

  pub async fn build_for_account(
    db: &mut SqliteConnection,
    account: &LocalAccount,
  ) -> anyhow::Result<(BalanceChange, Option<BalanceChangeStatus>)> {
    let mut balance_change = BalanceChange {
      account_id: AccountStore::parse_address(&account.address)?,
      account_type: account.account_type.clone(),
      change_number: 1,
      balance: 0,
      escrow_hold_note: None,
      notes: bounded_vec![],
      previous_balance_proof: None,
      signature: ed25519::Signature([0; 64]).into(),
    };

    let mut status = None;

    if let Some(latest) = Self::get_latest_for_account(db, account.id).await? {
      balance_change.change_number = latest.change_number as u32;
      balance_change.balance = latest.balance.parse().unwrap();
      status = Some(latest.status);
      if let Some(note_json) = latest.escrow_hold_note_json {
        balance_change.escrow_hold_note = Some(from_value(note_json.parse()?)?);
      }
      let Some(notarization_id) = latest.notarization_id else {
        return Err(anyhow::anyhow!("Balance change not notarized"));
      };

      let notarization = sqlx::query!(
        "SELECT notebook_number, tick FROM notarizations WHERE id = ?",
        notarization_id
      )
      .fetch_one(&mut *db)
      .await?;
      let notebook_number = notarization
        .notebook_number
        .ok_or_else(|| anyhow::anyhow!("Notarization {} not sent to notary", notarization_id))?;

      balance_change.previous_balance_proof = Some(BalanceProof {
        notary_id: latest.notary_id as u32,
        notebook_number: notebook_number as u32,
        tick: notarization.tick.unwrap() as u32,
        notebook_proof: match latest.proof_json.map(|a| serde_json::from_str(&a)) {
          Some(Ok(proof)) => proof,
          // TODO: we should prolly only allow this to be none if the change is in the current notebook
          None => None,
          Some(Err(e)) => Err(anyhow::anyhow!("Invalid proof json - {:?}", e))?,
        },
        account_origin: account
          .origin
          .clone()
          .ok_or_else(|| anyhow::anyhow!("This account doesn't have an origin"))?
          .into(),
        balance: balance_change.balance,
      });
    }
    Ok((balance_change, status))
  }

  pub async fn save_sent(
    db: &mut Transaction<'static, Sqlite>,
    account_id: i64,
    balance_change: BalanceChange,
    notary_id: NotaryId,
    transaction_id: Option<i64>,
  ) -> anyhow::Result<i64> {
    let mut hold_note_json = None;
    for note in balance_change.notes.iter() {
      if matches!(note.note_type, ulx_primitives::NoteType::EscrowHold { .. }) {
        hold_note_json = Some(json!(note));
      }
    }

    let notes_json = json!(balance_change.notes);

    let balance_str = balance_change.balance.to_string();
    let net_balance_change = balance_change.net_balance_change().to_string();

    let res = sqlx::query!(
        r#"INSERT INTO balance_changes (account_id, change_number, balance, net_balance_change, status, escrow_hold_note_json, notes_json, notary_id, transaction_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        account_id,
        balance_change.change_number,
        balance_str,
        net_balance_change,
        BalanceChangeStatus::WaitingForSendClaim as i64,
        hold_note_json,
        notes_json,
        notary_id,
        transaction_id
      )
            .execute(&mut **db)
            .await?;
    if res.rows_affected() != 1 {
      Err(anyhow::anyhow!("Error inserting balance change"))?;
    }

    let change_id = res.last_insert_rowid();
    Ok(change_id)
  }

  pub async fn upsert_notarized(
    db: &mut Transaction<'static, Sqlite>,
    account_id: i64,
    balance_change: &BalanceChange,
    notary_id: NotaryId,
    notarization_id: i64,
    transaction_id: Option<i64>,
  ) -> anyhow::Result<i64> {
    let mut hold_note_json = None;
    for note in balance_change.notes.iter() {
      if matches!(note.note_type, ulx_primitives::NoteType::EscrowHold { .. }) {
        hold_note_json = Some(json!(note));
      }
    }

    let notes_json = json!(balance_change.notes);

    let balance_str = balance_change.balance.to_string();
    let net_balance_change = balance_change.net_balance_change().to_string();

    if let Some(existing) = Self::find_account_change(db, account_id, balance_change).await? {
      if existing.status == BalanceChangeStatus::NotebookPublished
        || existing.status == BalanceChangeStatus::MainchainFinal
      {
        return Ok(existing.id);
      }
      let res = sqlx::query!(
        "UPDATE balance_changes SET notarization_id = ?, balance = ?, net_balance_change = ?, notes_json = ?, escrow_hold_note_json = ?, status = ?, \
        transaction_id = ? WHERE id = ?",
        notarization_id,
        balance_str,
        net_balance_change,
        notes_json,
        hold_note_json,
        BalanceChangeStatus::SubmittedToNotary as i64,
        transaction_id,
        existing.id,
      ).execute(&mut **db).await?;
      if res.rows_affected() != 1 {
        Err(anyhow::anyhow!("Error updating balance change"))?;
      }
      return Ok(existing.id);
    }

    let res = sqlx::query!(
        r#"INSERT INTO balance_changes (account_id, change_number, balance, net_balance_change, status, escrow_hold_note_json, notes_json, notary_id, notarization_id, transaction_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        account_id,
        balance_change.change_number,
        balance_str,
        net_balance_change,
        BalanceChangeStatus::SubmittedToNotary as i64,
        hold_note_json,
        notes_json,
        notary_id,
        notarization_id,
        transaction_id,
      )
            .execute(&mut **db)
            .await?;
    if res.rows_affected() != 1 {
      Err(anyhow::anyhow!("Error inserting balance change"))?;
    }

    let change_id = res.last_insert_rowid();
    Ok(change_id)
  }

  pub async fn check_if_superseded(
    db: &mut SqliteConnection,
    balance_change: &mut BalanceChangeRow,
  ) -> anyhow::Result<bool> {
    let res = sqlx::query_scalar!(
      "SELECT id FROM balance_changes WHERE account_id = ? AND change_number > ? AND status in (?,?) ORDER BY change_number DESC LIMIT 1",
      balance_change.account_id,
      balance_change.change_number,
      BalanceChangeStatus::MainchainFinal as i64,
      BalanceChangeStatus::NotebookPublished as i64,
    )
            .fetch_optional(&mut *db)
            .await?;
    if res.is_some() {
      balance_change.status = BalanceChangeStatus::SupersededInNotebook;
      sqlx::query!(
        "UPDATE balance_changes SET status = ? WHERE id = ?",
        BalanceChangeStatus::SupersededInNotebook as i64,
        balance_change.id
      )
      .execute(&mut *db)
      .await?;
      return Ok(true);
    }
    Ok(false)
  }

  pub async fn save_notebook_proof(
    db: &mut Transaction<'static, Sqlite>,
    balance_change: &mut BalanceChangeRow,
    proof: &BalanceProof,
  ) -> anyhow::Result<()> {
    let proof_json = json!(proof.notebook_proof);
    balance_change.proof_json = Some(proof_json.to_string());
    balance_change.status = BalanceChangeStatus::NotebookPublished;
    sqlx::query!(
      "UPDATE balance_changes SET proof_json = ?, status = ? WHERE id = ?",
      proof_json,
      BalanceChangeStatus::NotebookPublished as i64,
      balance_change.id
    )
    .execute(&mut **db)
    .await
    .map_err(to_js_error)?;
    Ok(())
  }

  pub async fn save_finalized(
    db: &mut Transaction<'static, Sqlite>,
    balance_change: &mut BalanceChangeRow,
    account: &LocalAccount,
    account_change_root: H256,
    finalized_block_number: u32,
  ) -> anyhow::Result<()> {
    balance_change.verify_balance_proof(account, account_change_root)?;
    balance_change.status = BalanceChangeStatus::MainchainFinal;
    balance_change.finalized_block_number = Some(finalized_block_number as i64);

    sqlx::query!(
      "UPDATE balance_changes SET finalized_block_number = ?, status = ? WHERE id = ?",
      balance_change.finalized_block_number,
      BalanceChangeStatus::MainchainFinal as i64,
      balance_change.id
    )
    .execute(&mut **db)
    .await?;

    Ok(())
  }
}

#[cfg(test)]
mod test {
  use sp_keyring::AccountKeyring::Ferdie;
  use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

  use ulx_primitives::{AccountOrigin, AccountType, Note, NoteType};

  use crate::test_utils::connect_with_logs;
  use crate::*;

  use super::*;

  #[sqlx::test]
  async fn test_storage(
    pool_options: SqlitePoolOptions,
    connect_options: SqliteConnectOptions,
  ) -> anyhow::Result<()> {
    let pool = connect_with_logs(pool_options, connect_options).await?;

    let mut db = pool.acquire().await?;
    let account = AccountStore::insert(
      &mut *db,
      AccountStore::to_address(&Ferdie.to_account_id()),
      AccountType::Tax,
      1,
      None,
    )
    .await?;
    // need to set the id and get the updated origin
    AccountStore::update_origin(&mut *db, account.id, 1, 1).await?;
    let account = AccountStore::get_by_id(&mut *db, account.id).await?;

    let (mut balance_change, _) = BalanceChangeStore::build_for_account(&mut *db, &account).await?;
    assert_eq!(balance_change.balance, 0);
    assert_eq!(balance_change.change_number, 1);

    balance_change.balance = 100;
    balance_change.change_number = 2;
    balance_change.push_note(100, NoteType::Claim);
    balance_change.previous_balance_proof = Some(BalanceProof {
      notary_id: 1,
      notebook_number: 1,
      tick: 1,
      notebook_proof: None,
      account_origin: AccountOrigin {
        account_uid: 1,
        notebook_number: 1,
      },
      balance: 0,
    });
    let mut tx = pool.begin().await?;
    let id =
      BalanceChangeStore::save_sent(&mut tx, account.id, balance_change.clone(), 1, None).await?;
    tx.commit().await?;

    assert_eq!(
      BalanceChangeStore::build_for_account(&mut *db, &account)
        .await
        .unwrap_err()
        .to_string(),
      "Balance change not notarized",
      "Should not be able to load account with no notarization"
    );

    let by_id = BalanceChangeStore::get_by_id(&mut *db, id).await?;
    println!("{:?}", by_id);
    assert_eq!(by_id.balance, "100");
    assert_eq!(by_id.status, BalanceChangeStatus::WaitingForSendClaim);

    let unsettled = BalanceChangeStore::find_unsettled(&mut *db).await?;
    assert_eq!(unsettled.len(), 1);
    assert_eq!(unsettled[0].id, id);

    let for_account = BalanceChangeStore::get_latest_for_account(&mut *db, account.id).await?;
    assert_eq!(for_account.unwrap().id, id);

    sqlx::query!(
      "INSERT into notarizations (json, notary_id, notebook_number, tick) VALUES (?, ?, ?, ?)",
      "{}",
      1,
      1,
      1
    )
    .execute(&mut *db)
    .await?;

    let mut tx = pool.begin().await?;
    BalanceChangeStore::upsert_notarized(&mut tx, account.id, &balance_change, 1, 1, None).await?;
    tx.commit().await?;

    let (reloaded, _) = BalanceChangeStore::build_for_account(&mut *db, &account).await?;
    assert_eq!(reloaded.balance, 100);
    assert_eq!(reloaded.change_number, 2);

    assert_eq!(
      BalanceChangeStore::get_by_id(&mut *db, id).await?.status,
      BalanceChangeStatus::SubmittedToNotary
    );

    let mut next = balance_change.clone();
    next.change_number += 1;
    next.balance = 200;
    next.notes = bounded_vec![Note {
      note_type: NoteType::Claim,
      milligons: 100
    }];
    let mut tx = pool.begin().await?;
    let id2 = BalanceChangeStore::upsert_notarized(&mut tx, account.id, &next, 1, 1, None).await?;
    tx.commit().await?;

    let (reloaded, _) = BalanceChangeStore::build_for_account(&mut *db, &account).await?;
    assert_eq!(reloaded.balance, 200);
    assert_eq!(reloaded.change_number, 3);

    let for_account = BalanceChangeStore::get_latest_for_account(&mut *db, account.id).await?;
    assert_eq!(for_account.unwrap().id, id2);

    let mut unsettled = BalanceChangeStore::find_unsettled(&mut *db).await?;
    assert_eq!(unsettled.len(), 2);

    let mut tx = pool.begin().await?;
    BalanceChangeStore::save_notebook_proof(
      &mut tx,
      &mut unsettled[0],
      &BalanceProof {
        notary_id: 1,
        notebook_number: 1,
        tick: 1,
        notebook_proof: None,
        account_origin: AccountOrigin {
          account_uid: 1,
          notebook_number: 1,
        },
        balance: 0,
      },
    )
    .await?;
    tx.commit().await?;

    assert_ne!(
      BalanceChangeStore::get_by_id(&mut *db, unsettled[0].id)
        .await?
        .proof_json,
      None
    );

    Ok(())
  }
}
