use crate::accounts::AccountStore;
use crate::balance_changes::BalanceChangeStore;
use crate::notary_client::NotaryClients;
use crate::signer::Signer;
use crate::to_js_error;
use crate::TickerRef;
use anyhow::anyhow;
use bech32::{Bech32m, Hrp};
use chrono::NaiveDateTime;
use codec::Encode;
use lazy_static::lazy_static;
use napi::bindgen_prelude::*;
use sp_core::ed25519::Signature;
use sp_core::Decode;
use sp_runtime::MultiSignature;
use sqlx::{FromRow, SqliteConnection, SqlitePool};
use std::sync::Arc;
use tokio::sync::Mutex;
use ulx_notary_audit::verify_changeset_signatures;
use ulx_primitives::{
  AccountType, BalanceChange, BalanceTip, NoteType, NotebookNumber, ESCROW_CLAWBACK_TICKS,
  ESCROW_EXPIRATION_TICKS, MINIMUM_ESCROW_SETTLEMENT,
};

lazy_static! {
  pub static ref EMPTY_SIGNATURE: Vec<u8> =
    MultiSignature::from(Signature([0; 64])).encode().to_vec();
}

#[derive(FromRow, Clone)]
#[allow(dead_code)]
struct EscrowRow {
  id: String,
  initial_balance_change_json: String,
  from_address: String,
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

#[napi]
#[derive(Clone, Debug, PartialEq)]
pub struct Escrow {
  pub id: String,
  pub initial_balance_change_json: String,
  pub notary_id: u32,
  hold_amount: u128,
  pub from_address: String,
  pub to_address: String,
  pub data_domain_hash: Option<Vec<u8>>,
  pub expiration_tick: u32,
  pub balance_change_number: u32,
  pub notarization_id: Option<i64>,
  pub is_client: bool,
  pub missed_claim_window: bool,
  pub(crate) settled_amount: u128,
  settled_signature: Vec<u8>,
  balance_change: BalanceChange,
}

#[napi]
impl Escrow {
  #[napi(getter)]
  pub fn hold_amount(&self) -> BigInt {
    BigInt::from(self.hold_amount)
  }
  #[napi(getter)]
  pub fn settled_amount(&self) -> BigInt {
    BigInt::from(self.settled_amount)
  }
  #[napi(getter)]
  pub fn settled_signature(&self) -> Uint8Array {
    Uint8Array::from(self.settled_signature.clone())
  }

  #[napi]
  pub fn is_past_claim_period(&self, current_tick: u32) -> bool {
    current_tick > self.expiration_tick + ESCROW_CLAWBACK_TICKS
  }

  pub fn get_initial_balance_change(&self) -> BalanceChange {
    self.balance_change.clone()
  }

  pub fn create_escrow_id(balance_change: &BalanceChange) -> anyhow::Result<String> {
    let mut balance_change = balance_change.clone();
    // set to minimum for id
    balance_change.notes[0].milligons = MINIMUM_ESCROW_SETTLEMENT;
    let Ok(hrp) = Hrp::parse("esc") else {
      return Err(anyhow!("Failed to parse internal bech32 encoding hrp"));
    };
    println!("Creating escrow id {:?}", balance_change);
    let id = bech32::encode::<Bech32m>(hrp, balance_change.hash().as_ref())?;
    Ok(id)
  }

  pub fn try_from_balance_change_json(balance_change_json: String) -> anyhow::Result<Escrow> {
    let balance_change: BalanceChange = serde_json::from_str(&balance_change_json)?;
    let Some(ref escrow_hold_note) = balance_change.escrow_hold_note else {
      return Err(anyhow!("Balance change has no escrow hold note"));
    };
    if escrow_hold_note.milligons < MINIMUM_ESCROW_SETTLEMENT {
      return Err(anyhow!(
        "Escrow hold note {} is less than minimum settlement amount: {}",
        escrow_hold_note.milligons,
        MINIMUM_ESCROW_SETTLEMENT
      ));
    }

    let (recipient, data_domain_hash) = match &escrow_hold_note.note_type {
      NoteType::EscrowHold {
        recipient,
        data_domain_hash,
      } => (recipient, data_domain_hash),
      _ => {
        return Err(anyhow!(
          "Balance change has invalid escrow hold note type {:?}",
          escrow_hold_note.note_type
        ));
      }
    };

    if balance_change.account_type != AccountType::Deposit {
      return Err(anyhow!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      ));
    }

    if balance_change.notes.len() != 1 {
      return Err(anyhow!(
        "Balance change has {} notes, expected 1",
        balance_change.notes.len()
      ));
    }
    let settle_note = &balance_change.notes[0];
    if settle_note.note_type != NoteType::EscrowSettle {
      return Err(anyhow!(
        "Balance change doesn't have a EscrowSettle note. It is: {:?}",
        settle_note.note_type
      ));
    }
    let Some(proof) = &balance_change.previous_balance_proof else {
      return Err(anyhow!("Balance change has no proof"));
    };

    let id = Escrow::create_escrow_id(&balance_change)?;

    Ok(Escrow {
      id,
      is_client: false,
      initial_balance_change_json: balance_change_json,
      hold_amount: escrow_hold_note.milligons,
      from_address: AccountStore::to_address(&balance_change.account_id),
      to_address: AccountStore::to_address(&recipient),
      balance_change_number: balance_change.change_number,
      data_domain_hash: data_domain_hash.map(|h| h.0.to_vec()).clone(),
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

  pub async fn get_final(&self) -> anyhow::Result<BalanceChange> {
    let mut balance_change = self.get_change_with_settled_amount(self.settled_amount);
    if self.settled_signature.len() == 0 || self.settled_signature == *EMPTY_SIGNATURE {
      return Err(anyhow::anyhow!("Escrow has not been signed"));
    }
    balance_change.signature = MultiSignature::decode(&mut self.settled_signature.as_slice())?;
    verify_changeset_signatures(&vec![balance_change.clone()])?;
    Ok(balance_change)
  }

  pub async fn insert(&mut self, db: &mut SqliteConnection) -> anyhow::Result<()> {
    let settled_amount = self.settled_amount.to_string();
    let balance_change_number = self.balance_change_number as i64;
    let from_address = self.from_address.clone();

    let res = sqlx::query!(
      r#"INSERT INTO open_escrows 
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
      return Err(anyhow!("Failed to insert escrow"));
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

  pub async fn update_signature(
    &mut self,
    db: &mut SqliteConnection,
    milligons: u128,
    signature: Vec<u8>,
  ) -> anyhow::Result<()> {
    let mut balance_change = self.get_change_with_settled_amount(milligons);
    balance_change.signature = MultiSignature::decode(&mut signature.as_slice())?;
    verify_changeset_signatures(&vec![balance_change.clone()])?;

    self.settled_amount = milligons;
    self.settled_signature = signature;
    let settled_amount = milligons.to_string();
    let id = &self.id;
    let res = sqlx::query!(
      "UPDATE open_escrows SET settled_amount=?, settled_signature = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
      settled_amount,
      self.settled_signature,
      id,
    )
    .execute(&mut *db)
    .await?;
    if res.rows_affected() != 1 {
      return Err(anyhow!("Failed to update escrow"));
    }
    Ok(())
  }

  pub async fn mark_unable_to_claim(&mut self, db: &mut SqliteConnection) -> anyhow::Result<()> {
    let res = sqlx::query!(
      "UPDATE open_escrows SET missed_claim_window = true, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
      self.id,
    )
        .execute(&mut *db)
        .await?;
    if res.rows_affected() != 1 {
      return Err(anyhow!("Failed to update escrow"));
    }
    self.missed_claim_window = true;
    Ok(())
  }

  pub async fn mark_notarized(
    &mut self,
    db: &mut SqliteConnection,
    notarization_id: i64,
  ) -> anyhow::Result<()> {
    self.notarization_id = Some(notarization_id);
    let id = &self.id;
    let res = sqlx::query!(
      "UPDATE open_escrows SET notarization_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
      notarization_id,
      id,
    )
    .execute(&mut *db)
    .await?;
    if res.rows_affected() != 1 {
      return Err(anyhow!("Failed to update escrow"));
    }
    Ok(())
  }
}

impl TryFrom<EscrowRow> for Escrow {
  type Error = anyhow::Error;
  fn try_from(row: EscrowRow) -> anyhow::Result<Self> {
    let mut escrow = Escrow::try_from_balance_change_json(row.initial_balance_change_json)?;

    escrow.expiration_tick = row.expiration_tick as u32;
    escrow.settled_amount = row.settled_amount.parse()?;
    escrow.settled_signature = row.settled_signature;
    escrow.notarization_id = row.notarization_id;
    escrow.is_client = row.is_client;
    escrow.missed_claim_window = row.missed_claim_window;
    Ok(escrow)
  }
}

#[napi]
#[derive(Clone)]
pub struct OpenEscrow {
  db: SqlitePool,
  escrow: Arc<Mutex<Escrow>>,
}

#[napi]
impl OpenEscrow {
  pub fn new(db: SqlitePool, escrow: Escrow) -> Self {
    OpenEscrow {
      db,
      escrow: Arc::new(Mutex::new(escrow)),
    }
  }

  #[napi(getter)]
  pub async fn escrow(&self) -> Escrow {
    self.escrow.lock().await.clone()
  }

  #[napi]
  pub async fn sign(&self, settled_amount: BigInt, signer: &Signer) -> Result<SignatureResult> {
    if settled_amount.get_u128().1 < MINIMUM_ESCROW_SETTLEMENT {
      return Err(to_js_error(format!("Settled amount must be greater than the minimum escrow settlement amount ({MINIMUM_ESCROW_SETTLEMENT})")));
    }
    let mut escrow = self.escrow.lock().await;
    let mut tx = self.db.begin().await.map_err(to_js_error)?;
    let (_, milligons, _) = settled_amount.get_u128();
    let balance_change = escrow.get_change_with_settled_amount(milligons);
    let bytes = balance_change.hash();

    let signature = signer
      .sign(
        AccountStore::to_address(&balance_change.account_id),
        Uint8Array::from(bytes.as_bytes().to_vec()),
      )
      .await?;

    escrow
      .update_signature(&mut *tx, milligons, signature.to_vec())
      .await
      .map_err(to_js_error)?;

    tx.commit().await.map_err(to_js_error)?;

    Ok(SignatureResult {
      signature,
      milligons: settled_amount,
    })
  }

  #[napi]
  pub async fn export_for_send(&self) -> Result<String> {
    let escrow = self.escrow.lock().await;
    let balance_change = escrow.get_final().await.map_err(to_js_error)?;
    let json = serde_json::to_string(&balance_change)?;
    Ok(json)
  }

  #[napi]
  pub async fn record_updated_settlement(
    &self,
    milligons: BigInt,
    signature: Uint8Array,
  ) -> Result<()> {
    if milligons.get_u128().1 < MINIMUM_ESCROW_SETTLEMENT {
      return Err(to_js_error(format!("Settled amount is less than minimum escrow settlement amount ({MINIMUM_ESCROW_SETTLEMENT})")));
    }
    let mut escrow = self.escrow.lock().await;
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    let (_, milligons, _) = milligons.get_u128();
    escrow
      .update_signature(&mut *db, milligons, signature.to_vec())
      .await
      .map_err(to_js_error)?;

    Ok(())
  }

  pub async fn inner(&self) -> Escrow {
    self.escrow.lock().await.clone()
  }
}

#[napi]
pub struct OpenEscrowsStore {
  db: SqlitePool,
  ticker: TickerRef,
  notary_clients: NotaryClients,
}
#[napi]
impl OpenEscrowsStore {
  pub(crate) fn new(db: SqlitePool, ticker: TickerRef, notary_clients: &NotaryClients) -> Self {
    Self {
      db,
      ticker,
      notary_clients: notary_clients.clone(),
    }
  }

  #[napi]
  pub async fn get(&self, id: String) -> Result<OpenEscrow> {
    let row = sqlx::query_as!(EscrowRow, "SELECT * FROM open_escrows WHERE id = ?", id)
      .fetch_one(&self.db)
      .await
      .map_err(to_js_error)?;

    let escrow = Escrow::try_from(row).map_err(to_js_error)?;

    Ok(self.open(&escrow))
  }

  #[napi]
  pub fn open(&self, escrow: &Escrow) -> OpenEscrow {
    OpenEscrow::new(self.db.clone(), escrow.clone())
  }

  pub async fn record_notarized(
    db: &mut SqliteConnection,
    balance_change: &BalanceChange,
    notarization_id: i64,
  ) -> anyhow::Result<()> {
    let address = AccountStore::to_address(&balance_change.account_id);
    let res = sqlx::query!(
      r#"UPDATE open_escrows SET notarization_id = ?, updated_at = CURRENT_TIMESTAMP
       WHERE from_address = ? AND balance_change_number = ?"#,
      notarization_id,
      address,
      balance_change.change_number,
    )
    .execute(db)
    .await?;
    if res.rows_affected() != 1 {
      return Err(anyhow!("Failed to update escrow"));
    }
    Ok(())
  }

  #[napi]
  pub async fn get_claimable(&self) -> Result<Vec<OpenEscrow>> {
    let current_tick = self.ticker.current();
    let expired = sqlx::query_as!(
      EscrowRow,
      r#"SELECT * FROM open_escrows WHERE notarization_id IS NULL AND missed_claim_window = false AND expiration_tick <= $1"#,
      current_tick,
    )
    .fetch_all(&self.db)
    .await
    .map_err(to_js_error)?;
    tracing::info!("Found {} claimable escrows", expired.len());

    let mut escrows = vec![];
    for row in expired.into_iter() {
      let escrow = Escrow::try_from(row).map_err(to_js_error)?;
      escrows.push(OpenEscrow::new(self.db.clone(), escrow))
    }
    tracing::info!("return escrows {}", escrows.len());
    Ok(escrows)
  }

  #[napi]
  /// Import an escrow from a JSON string. Verifies with the notary that the escrow hold is valid.
  pub async fn import_escrow(&self, escrow_json: String) -> Result<OpenEscrow> {
    let mut escrow = Escrow::try_from_balance_change_json(escrow_json).map_err(to_js_error)?;
    verify_changeset_signatures(&vec![escrow.balance_change.clone()]).map_err(to_js_error)?;

    let notary_client = self.notary_clients.get(escrow.notary_id).await?;

    let balance_tip = notary_client
      .get_balance_tip(escrow.from_address.clone(), AccountType::Deposit)
      .await?;

    let Some(balance_proof) = &escrow.balance_change.previous_balance_proof else {
      return Err(to_js_error("Balance change has no previous balance proof"));
    };

    let calculated_tip = BalanceTip::compute_tip(
      escrow.balance_change.change_number.saturating_sub(1),
      balance_proof.balance,
      balance_proof.account_origin.clone(),
      escrow.balance_change.escrow_hold_note.clone(),
    );

    let current_tip = balance_tip.balance_tip.as_ref();

    if calculated_tip != current_tip {
      return Err(to_js_error(format!(
        "Balance tip mismatch. Expected {:#x?}, got {:#x?}",
        calculated_tip, current_tip
      )));
    }
    escrow.expiration_tick = balance_tip.tick + ESCROW_EXPIRATION_TICKS;
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    escrow.insert(&mut db).await.map_err(to_js_error)?;
    Ok(OpenEscrow::new(self.db.clone(), escrow))
  }

  #[napi]
  /// Create a new escrow as a client. You must first notarize an escrow hold note to the notary for the `client_address`.
  pub async fn open_client_escrow(&self, account_id: i64) -> Result<OpenEscrow> {
    let mut tx = self.db.begin().await.map_err(to_js_error)?;
    let account = AccountStore::get_by_id(&mut *tx, account_id).await?;
    let mut balance_tip = BalanceChangeStore::build_for_account(&mut *tx, &account).await?;

    let hold_note = &balance_tip
      .escrow_hold_note
      .clone()
      .ok_or(to_js_error(format!(
        "Account {} has no escrow hold note",
        account.address
      )))?;

    let (data_domain_hash, recipient) = match &hold_note.note_type {
      NoteType::EscrowHold {
        recipient,
        data_domain_hash,
      } => (data_domain_hash, recipient),
      _ => {
        return Err(to_js_error(format!(
          "Balance change has invalid escrow hold note type {:?}",
          hold_note.note_type
        )));
      }
    };

    let (notary_id, tick) = &balance_tip
      .previous_balance_proof
      .clone()
      .map(|p| (p.notary_id, p.tick))
      .ok_or(to_js_error(format!(
        "Balance change has no previous balance proof"
      )))?;

    balance_tip.change_number += 1;
    balance_tip.push_note(MINIMUM_ESCROW_SETTLEMENT, NoteType::EscrowSettle);
    balance_tip.balance -= MINIMUM_ESCROW_SETTLEMENT;

    let id = Escrow::create_escrow_id(&balance_tip)?;

    let mut escrow = Escrow {
      id,
      is_client: true,
      initial_balance_change_json: serde_json::to_string(&balance_tip)?,
      balance_change_number: balance_tip.change_number,
      hold_amount: hold_note.milligons,
      from_address: account.address,
      to_address: AccountStore::to_address(recipient),
      data_domain_hash: data_domain_hash.map(|h| h.0.to_vec()).clone(),
      notary_id: *notary_id,
      expiration_tick: tick + ESCROW_EXPIRATION_TICKS,
      settled_amount: MINIMUM_ESCROW_SETTLEMENT,
      settled_signature: EMPTY_SIGNATURE.clone(),
      notarization_id: None,
      balance_change: balance_tip,
      missed_claim_window: false,
    };
    escrow.insert(&mut *tx).await.map_err(to_js_error)?;
    tx.commit().await.map_err(to_js_error)?;

    Ok(OpenEscrow::new(self.db.clone(), escrow))
  }
}

#[napi(object)]
pub struct SignatureResult {
  pub signature: Uint8Array,
  pub milligons: BigInt,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::balance_change_builder::BalanceChangeBuilder;
  use crate::test_utils::CryptoType::Ed25519;
  use crate::test_utils::{create_keystore, create_mock_notary, mock_notary_clients, MockNotary};
  use crate::*;
  use anyhow::bail;
  use serde_json::json;
  use sp_keyring::AccountKeyring::Alice;
  use sp_keyring::Ed25519Keyring::Bob;
  use sp_keyring::Ed25519Keyring::Ferdie;
  use ulx_primitives::tick::Tick;
  use ulx_primitives::{AccountId, Notarization};

  async fn register_account(
    db: &mut SqliteConnection,
    account_id: AccountId,
    origin_uid: u32,
    origin_notebook: u32,
  ) -> anyhow::Result<LocalAccount> {
    let address = AccountStore::to_address(&account_id);
    let account = AccountStore::insert(db, address, AccountType::Deposit, 1).await?;
    AccountStore::update_origin(db, account.id, origin_notebook, origin_uid).await?;
    let account = AccountStore::get_by_id(db, account.id).await?;
    Ok(account)
  }

  async fn create_escrow_hold(
    pool: &SqlitePool,
    account: &LocalAccount,
    localchain_transfer_amount: u128,
    hold_amount: u128,
    data_domain: String,
    recipient: String,
    notebook_number: NotebookNumber,
    tick: Tick,
  ) -> anyhow::Result<BalanceChangeRow> {
    let mut tx = pool.begin().await?;
    let balance_tip = BalanceChangeStore::build_for_account(&mut *tx, account).await?;
    let builder = BalanceChangeBuilder::new(balance_tip);
    builder
      .claim_from_mainchain(LocalchainTransfer {
        address: account.address.clone(),
        notary_id: 1,
        amount: BigInt::from(localchain_transfer_amount),
        expiration_block: 100,
        account_nonce: 1,
      })
      .await?;
    builder
      .create_escrow_hold(BigInt::from(hold_amount), data_domain, recipient.clone())
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
      BalanceChangeStore::upsert_notarized(&mut tx, account.id, &balance_change, 1, id).await?;
    tx.commit().await?;

    let mut db = pool.acquire().await?;
    let balance_change = BalanceChangeStore::get_by_id(&mut db, id).await?;
    Ok(balance_change)
  }

  async fn register_balance_tip(
    account: &LocalAccount,
    mock_notary: &MockNotary,
    balance_change: &BalanceChangeRow,
    notebook_number: NotebookNumber,
    tick: Tick,
  ) -> anyhow::Result<()> {
    let balance_tip = balance_change.get_balance_tip(&account)?;
    println!("got balance tip for account {:?}", balance_tip);
    let mut state = mock_notary.state.lock().await;
    (*state).balance_tips.insert(
      (account.get_account_id32()?, account.account_type.clone()),
      ulx_notary::apis::localchain::BalanceTipResult {
        tick,
        balance_tip: balance_tip.tip().into(),
        notebook_number,
      },
    );
    Ok(())
  }

  #[sqlx::test]
  async fn test_open_escrow(pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;

    let alice_address = AccountStore::to_address(&Alice.to_account_id());
    let mut db = pool.acquire().await?;
    let bob_account = register_account(&mut *db, Bob.to_account_id(), 1, 1).await?;

    let _bob_hold = create_escrow_hold(
      &pool,
      &bob_account,
      20_000,
      1_000,
      "delta.flights".to_string(),
      alice_address.clone(),
      1,
      1,
    )
    .await?;

    let ticker = TickerRef {
      ticker: Ticker::start(Duration::from_secs(60)),
    };
    println!("about to open escrow");
    let store = OpenEscrowsStore::new(pool, ticker, &notary_clients);
    let open_escrow = store.open_client_escrow(bob_account.id).await?;
    println!("opened escrow");
    let escrow = open_escrow.inner().await;
    assert_eq!(escrow.to_address.clone(), alice_address);
    assert_eq!(escrow.expiration_tick, 1 + ESCROW_EXPIRATION_TICKS);

    assert_eq!(store.get_claimable().await?.len(), 0);

    let Err(e) = open_escrow.export_for_send().await else {
      bail!("Expected error");
    };
    assert_eq!(e.reason.to_string(), "Escrow has not been signed");

    let keystore = create_keystore(&Bob.to_seed(), Ed25519)?;

    let signer = Signer::with_keystore(keystore);
    println!("signing");
    open_escrow.sign(BigInt::from(5u128), &signer).await?;
    println!("signed");

    let json = open_escrow.export_for_send().await?;

    println!("escrow {}", &json);
    assert!(json.contains("escrowHoldNote\":{"));

    assert_eq!(
      store
        .get(escrow.id.clone())
        .await?
        .inner()
        .await
        .get_final()
        .await?,
      open_escrow.inner().await.get_final().await?,
      "can reload from db"
    );

    open_escrow.sign(BigInt::from(10u128), &signer).await?;

    assert_eq!(
      store
        .get(escrow.id)
        .await?
        .inner()
        .await
        .settled_amount()
        .get_u128()
        .1,
      10_u128
    );

    Ok(())
  }

  #[sqlx::test]
  async fn test_importing_escrow(bob_pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;

    let alice_pool = SqlitePool::connect(&":memory:")
      .await
      .map_err(to_js_error)?;
    sqlx::migrate!()
      .run(&alice_pool)
      .await
      .map_err(|e| Error::from_reason(format!("Error migrating database {}", e.to_string())))?;
    let mut alice_db = alice_pool.acquire().await?;

    let alice_address = AccountStore::to_address(&Alice.to_account_id());
    let mut bob_db = bob_pool.acquire().await?;
    let bob_account = register_account(&mut *bob_db, Bob.to_account_id(), 1, 1).await?;

    let _alice_account = register_account(&mut *alice_db, Alice.to_account_id(), 1, 1).await?;
    let bob_hold = create_escrow_hold(
      &bob_pool,
      &bob_account,
      20_000,
      1_000,
      "delta.flights".to_string(),
      alice_address.clone(),
      1,
      1,
    )
    .await?;

    let ticker = TickerRef {
      ticker: Ticker::start(Duration::from_secs(60)),
    };
    let bob_store = OpenEscrowsStore::new(bob_pool, ticker.clone(), &notary_clients);
    let open_escrow = bob_store.open_client_escrow(bob_account.id).await?;

    let signer = Signer::with_keystore(create_keystore(&Bob.to_seed(), Ed25519)?);
    open_escrow.sign(BigInt::from(5u128), &signer).await?;
    let json = open_escrow.export_for_send().await?;

    let alice_store = OpenEscrowsStore::new(alice_pool, ticker, &notary_clients);
    // before registered with notary, should fail
    match alice_store.import_escrow(json.clone()).await {
      Err(e) => {
        assert!(e.reason.contains("balance_tip not set"))
      }
      Ok(_) => {
        return Err(anyhow!("Expected error"));
      }
    }
    println!("registering balance tip");
    register_balance_tip(&bob_account, &mock_notary, &bob_hold, 1, 1).await?;

    println!("importing escrow");
    let alice_escrow = alice_store.import_escrow(json).await?;
    println!("imported escrow");
    let imported_escrow = alice_escrow.inner().await;
    let sent_escrow = open_escrow.inner().await;
    assert_eq!(imported_escrow.to_address, sent_escrow.to_address);
    assert_eq!(imported_escrow.from_address, sent_escrow.from_address);
    assert_eq!(imported_escrow.expiration_tick, sent_escrow.expiration_tick);
    assert_eq!(imported_escrow.settled_amount, sent_escrow.settled_amount);
    assert_eq!(
      imported_escrow.settled_signature,
      sent_escrow.settled_signature
    );
    assert_eq!(imported_escrow.hold_amount, sent_escrow.hold_amount);
    assert_eq!(
      imported_escrow.data_domain_hash,
      sent_escrow.data_domain_hash
    );
    assert_eq!(imported_escrow.notary_id, sent_escrow.notary_id);
    assert_eq!(
      imported_escrow.balance_change_number,
      sent_escrow.balance_change_number
    );

    assert_eq!(imported_escrow.expiration_tick, 1 + ESCROW_EXPIRATION_TICKS);
    assert_eq!(imported_escrow.settled_amount, MINIMUM_ESCROW_SETTLEMENT);
    assert_eq!(imported_escrow.id, sent_escrow.id);

    let result = open_escrow.sign(BigInt::from(10u128), &signer).await?;
    alice_escrow
      .record_updated_settlement(result.milligons, result.signature)
      .await?;
    assert_eq!(
      alice_escrow.inner().await.settled_amount().get_u128().1,
      10_u128
    );

    Ok(())
  }
}
