use crate::accounts::AccountStore;
use crate::balance_changes::BalanceChangeStore;
use crate::data_domain::JsDataDomain;
use crate::notary_client::NotaryClients;
use crate::signer::Signer;
use crate::to_js_error;
use crate::TickerRef;
use anyhow::anyhow;
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
  AccountType, BalanceChange, BalanceTip, NoteType, NotebookNumber, CHANNEL_CLAWBACK_TICKS,
  CHANNEL_EXPIRATION_TICKS, MIN_CHANNEL_NOTE_MILLIGONS,
};

lazy_static! {
  pub static ref EMPTY_SIGNATURE: Vec<u8> =
    MultiSignature::from(Signature([0; 64])).encode().to_vec();
}

#[derive(FromRow, Clone)]
#[allow(dead_code)]
struct ChannelRow {
  id: i64,
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
pub struct Channel {
  pub initial_balance_change_json: String,
  pub notary_id: u32,
  hold_amount: u128,
  pub from_address: String,
  pub to_address: String,
  pub data_domain: Option<JsDataDomain>,
  pub expiration_tick: u32,
  pub balance_change_number: u32,
  pub notarization_id: Option<i64>,
  pub is_client: bool,
  pub missed_claim_window: bool,
  pub(crate) settled_amount: u128,
  id: Option<i64>,
  settled_signature: Vec<u8>,
  balance_change: BalanceChange,
}

#[napi]
impl Channel {
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

  #[napi(getter)]
  pub fn id(&self) -> i64 {
    self.id.expect("Channel has not been saved yet!")
  }

  #[napi]
  pub fn is_past_claim_period(&self, current_tick: u32) -> bool {
    current_tick > self.expiration_tick + CHANNEL_CLAWBACK_TICKS
  }

  pub fn get_initial_balance_change(&self) -> BalanceChange {
    self.balance_change.clone()
  }

  pub fn try_from_balance_change_json(balance_change_json: String) -> anyhow::Result<Channel> {
    let balance_change: BalanceChange = serde_json::from_str(&balance_change_json)?;
    let Some(ref channel_hold_note) = balance_change.channel_hold_note else {
      return Err(anyhow!("Balance change has no channel hold note"));
    };

    let (recipient, data_domain) = match &channel_hold_note.note_type {
      NoteType::ChannelHold {
        recipient,
        data_domain,
      } => (recipient, data_domain),
      _ => {
        return Err(anyhow!(
          "Balance change has invalid channel hold note type {:?}",
          channel_hold_note.note_type
        ));
      }
    };

    if balance_change.account_type != AccountType::Deposit {
      return Err(anyhow!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      ));
    }

    if balance_change.balance < MIN_CHANNEL_NOTE_MILLIGONS {
      return Err(anyhow!(
        "Balance change amount {} is less than minimum channel note amount {}",
        balance_change.balance,
        MIN_CHANNEL_NOTE_MILLIGONS
      ));
    }

    if balance_change.notes.len() != 1 {
      return Err(anyhow!(
        "Balance change has {} notes, expected 1",
        balance_change.notes.len()
      ));
    }
    let settle_note = &balance_change.notes[0];
    if settle_note.note_type != NoteType::ChannelSettle {
      return Err(anyhow!(
        "Balance change doesn't have a ChannelSettle note. It is: {:?}",
        settle_note.note_type
      ));
    }

    let Some(proof) = &balance_change.previous_balance_proof else {
      return Err(anyhow!("Balance change has no proof"));
    };

    let data_domain = match data_domain {
      Some(data_domain) => Some(JsDataDomain {
        domain_name: String::from_utf8(data_domain.domain_name.to_vec())?,
        top_level_domain: data_domain.top_level_domain,
      }),
      None => None,
    };

    Ok(Channel {
      id: None,
      is_client: false,
      initial_balance_change_json: balance_change_json,
      hold_amount: channel_hold_note.milligons,
      from_address: AccountStore::to_address(&balance_change.account_id),
      to_address: AccountStore::to_address(&recipient),
      balance_change_number: balance_change.change_number,
      data_domain,
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
      return Err(anyhow::anyhow!("Channel has not been signed"));
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
      r#"INSERT INTO open_channels 
      (initial_balance_change_json, from_address, balance_change_number, expiration_tick, settled_amount, settled_signature, is_client) 
      VALUES (?, ?, ?, ?, ?, ?, ?)"#,
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
      return Err(anyhow!("Failed to insert channel"));
    }
    self.id = Some(res.last_insert_rowid());
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
    let id = self.id.ok_or(anyhow!("Channel has not been saved yet!"))?;
    let res = sqlx::query!(
      "UPDATE open_channels SET settled_amount=?, settled_signature = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
      settled_amount,
      self.settled_signature,
      id,
    )
    .execute(&mut *db)
    .await?;
    if res.rows_affected() != 1 {
      return Err(anyhow!("Failed to update channel"));
    }
    Ok(())
  }

  pub async fn mark_unable_to_claim(&mut self, db: &mut SqliteConnection) -> anyhow::Result<()> {
    let res = sqlx::query!(
      "UPDATE open_channels SET missed_claim_window = true, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
      self.id,
    )
        .execute(&mut *db)
        .await?;
    if res.rows_affected() != 1 {
      return Err(anyhow!("Failed to update channel"));
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
    let id = self.id.ok_or(anyhow!("Channel has not been saved yet!"))?;
    let res = sqlx::query!(
      "UPDATE open_channels SET notarization_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
      notarization_id,
      id,
    )
    .execute(&mut *db)
    .await?;
    if res.rows_affected() != 1 {
      return Err(anyhow!("Failed to update channel"));
    }
    Ok(())
  }
}

impl TryFrom<ChannelRow> for Channel {
  type Error = anyhow::Error;
  fn try_from(row: ChannelRow) -> anyhow::Result<Self> {
    let mut channel = Channel::try_from_balance_change_json(row.initial_balance_change_json)?;

    channel.id = Some(row.id);
    channel.expiration_tick = row.expiration_tick as u32;
    channel.settled_amount = row.settled_amount.parse()?;
    channel.settled_signature = row.settled_signature;
    channel.notarization_id = row.notarization_id;
    channel.is_client = row.is_client;
    channel.missed_claim_window = row.missed_claim_window;
    Ok(channel)
  }
}

#[napi]
#[derive(Clone)]
pub struct OpenChannel {
  db: SqlitePool,
  channel: Arc<Mutex<Channel>>,
}

#[napi]
impl OpenChannel {
  pub fn new(db: SqlitePool, channel: Channel) -> Self {
    OpenChannel {
      db,
      channel: Arc::new(Mutex::new(channel)),
    }
  }

  #[napi(getter)]
  pub async fn channel(&self) -> Channel {
    self.channel.lock().await.clone()
  }

  #[napi]
  pub async fn sign(&self, settled_amount: BigInt, signer: &Signer) -> Result<SignatureResult> {
    let mut channel = self.channel.lock().await;
    let mut tx = self.db.begin().await.map_err(to_js_error)?;
    let (_, milligons, _) = settled_amount.get_u128();
    let balance_change = channel.get_change_with_settled_amount(milligons);
    let bytes = balance_change.hash();

    let signature = signer
      .sign(
        AccountStore::to_address(&balance_change.account_id),
        Uint8Array::from(bytes.as_bytes().to_vec()),
      )
      .await?;

    channel
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
  pub async fn export_for_send(&self) -> Result<Buffer> {
    let channel = self.channel.lock().await;
    let balance_change = channel.get_final().await.map_err(to_js_error)?;
    let json = serde_json::to_vec(&balance_change)?;
    Ok(json.into())
  }

  #[napi]
  pub async fn record_updated_settlement(
    &self,
    milligons: BigInt,
    signature: Uint8Array,
  ) -> Result<()> {
    let mut channel = self.channel.lock().await;
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    let (_, milligons, _) = milligons.get_u128();
    channel
      .update_signature(&mut *db, milligons, signature.to_vec())
      .await
      .map_err(to_js_error)?;

    Ok(())
  }

  pub async fn inner(&self) -> Channel {
    self.channel.lock().await.clone()
  }
}

#[napi]
pub struct OpenChannelsStore {
  db: SqlitePool,
  ticker: TickerRef,
  notary_clients: NotaryClients,
}
#[napi]
impl OpenChannelsStore {
  pub(crate) fn new(db: SqlitePool, ticker: TickerRef, notary_clients: &NotaryClients) -> Self {
    Self {
      db,
      ticker,
      notary_clients: notary_clients.clone(),
    }
  }

  #[napi]
  pub async fn get(&self, id: i64) -> Result<OpenChannel> {
    let row = sqlx::query_as!(ChannelRow, "SELECT * FROM open_channels WHERE id = ?", id)
      .fetch_one(&self.db)
      .await
      .map_err(to_js_error)?;

    let channel = Channel::try_from(row).map_err(to_js_error)?;

    Ok(self.open(&channel))
  }

  #[napi]
  pub fn open(&self, channel: &Channel) -> OpenChannel {
    OpenChannel::new(self.db.clone(), channel.clone())
  }

  pub async fn record_notarized(
    db: &mut SqliteConnection,
    balance_change: &BalanceChange,
    notarization_id: i64,
  ) -> anyhow::Result<()> {
    let address = AccountStore::to_address(&balance_change.account_id);
    let res = sqlx::query!(
      r#"UPDATE open_channels SET notarization_id = ?, updated_at = CURRENT_TIMESTAMP
       WHERE from_address = ? AND balance_change_number = ?"#,
      notarization_id,
      address,
      balance_change.change_number,
    )
    .execute(db)
    .await?;
    if res.rows_affected() != 1 {
      return Err(anyhow!("Failed to update channel"));
    }
    Ok(())
  }

  #[napi]
  pub async fn get_claimable(&self) -> Result<Vec<OpenChannel>> {
    let current_tick = self.ticker.current();
    let expired = sqlx::query_as!(
      ChannelRow,
      r#"SELECT * FROM open_channels WHERE notarization_id IS NULL AND missed_claim_window = false AND expiration_tick <= $1"#,
      current_tick,
    )
    .fetch_all(&self.db)
    .await
    .map_err(to_js_error)?;
    tracing::info!("Found {} claimable channels", expired.len());

    let mut channels = vec![];
    for row in expired.into_iter() {
      let channel = Channel::try_from(row).map_err(to_js_error)?;
      channels.push(OpenChannel::new(self.db.clone(), channel))
    }
    tracing::info!("return channels {}", channels.len());
    Ok(channels)
  }

  #[napi]
  /// Import a channel from a JSON string. Verifies with the notary that the channel hold is valid.
  pub async fn import_channel(&self, channel_json: Buffer) -> Result<OpenChannel> {
    let json_string = String::from_utf8(channel_json.to_vec()).map_err(to_js_error)?;
    let mut channel = Channel::try_from_balance_change_json(json_string).map_err(to_js_error)?;
    verify_changeset_signatures(&vec![channel.balance_change.clone()]).map_err(to_js_error)?;

    let notary_client = self.notary_clients.get(channel.notary_id).await?;

    let balance_tip = notary_client
      .get_balance_tip(channel.from_address.clone(), AccountType::Deposit)
      .await?;

    let Some(balance_proof) = &channel.balance_change.previous_balance_proof else {
      return Err(to_js_error("Balance change has no previous balance proof"));
    };

    let calculated_tip = BalanceTip::compute_tip(
      channel.balance_change.change_number.saturating_sub(1),
      balance_proof.balance,
      balance_proof.account_origin.clone(),
      channel.balance_change.channel_hold_note.clone(),
    );

    let current_tip = balance_tip.balance_tip.as_ref();

    if calculated_tip != current_tip {
      return Err(to_js_error(format!(
        "Balance tip mismatch. Expected {:#x?}, got {:#x?}",
        calculated_tip, current_tip
      )));
    }
    channel.expiration_tick = balance_tip.tick + CHANNEL_EXPIRATION_TICKS;
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    channel.insert(&mut db).await.map_err(to_js_error)?;
    Ok(OpenChannel::new(self.db.clone(), channel))
  }

  #[napi]
  /// Create a new channel as a client. You must first notarize a channel hold note to the notary for the `client_address`.
  pub async fn open_client_channel(&self, account_id: i64) -> Result<OpenChannel> {
    let mut tx = self.db.begin().await.map_err(to_js_error)?;
    let account = AccountStore::get_by_id(&mut *tx, account_id).await?;
    let mut balance_tip = BalanceChangeStore::build_for_account(&mut *tx, &account).await?;

    let hold_note = &balance_tip
      .channel_hold_note
      .clone()
      .ok_or(to_js_error(format!(
        "Account {} has no channel hold note",
        account.address
      )))?;

    let (data_domain, recipient) = match &hold_note.note_type {
      NoteType::ChannelHold {
        recipient,
        data_domain,
      } => (data_domain, recipient),
      _ => {
        return Err(to_js_error(format!(
          "Balance change has invalid channel hold note type {:?}",
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
    balance_tip.push_note(0, NoteType::ChannelSettle);

    let data_domain = match data_domain {
      Some(x) => Some(x.try_into().map_err(to_js_error)?),
      None => None,
    };

    let mut channel = Channel {
      id: None,
      is_client: true,
      initial_balance_change_json: serde_json::to_string(&balance_tip)?,
      balance_change_number: balance_tip.change_number,
      hold_amount: hold_note.milligons,
      from_address: account.address,
      to_address: AccountStore::to_address(recipient),
      data_domain,
      notary_id: *notary_id,
      expiration_tick: tick + CHANNEL_EXPIRATION_TICKS,
      settled_amount: 0,
      settled_signature: EMPTY_SIGNATURE.clone(),
      notarization_id: None,
      balance_change: balance_tip,
      missed_claim_window: false,
    };
    channel.insert(&mut *tx).await.map_err(to_js_error)?;
    tx.commit().await.map_err(to_js_error)?;

    Ok(OpenChannel::new(self.db.clone(), channel))
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
  use ulx_primitives::{AccountId, DataTLD, Notarization};

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

  async fn create_channel_hold(
    pool: &SqlitePool,
    account: &LocalAccount,
    localchain_transfer_amount: u128,
    hold_amount: u128,
    data_domain: JsDataDomain,
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
      .create_channel_hold(BigInt::from(hold_amount), data_domain, recipient.clone())
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
  async fn test_open_channel(pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;

    let alice_address = AccountStore::to_address(&Alice.to_account_id());
    let mut db = pool.acquire().await?;
    let bob_account = register_account(&mut *db, Bob.to_account_id(), 1, 1).await?;

    let _bob_hold = create_channel_hold(
      &pool,
      &bob_account,
      20_000,
      1_000,
      JsDataDomain {
        domain_name: "Delta".to_string(),
        top_level_domain: DataTLD::Flights,
      },
      alice_address.clone(),
      1,
      1,
    )
    .await?;

    let ticker = TickerRef {
      ticker: Ticker::start(Duration::from_secs(60)),
    };
    println!("about to open channel");
    let store = OpenChannelsStore::new(pool, ticker, &notary_clients);
    let open_channel = store.open_client_channel(bob_account.id).await?;
    println!("opened channel");
    let channel = open_channel.inner().await;
    assert_eq!(channel.to_address.clone(), alice_address);
    assert_eq!(channel.expiration_tick, 1 + CHANNEL_EXPIRATION_TICKS);

    assert_eq!(store.get_claimable().await?.len(), 0);

    let Err(e) = open_channel.export_for_send().await else {
      bail!("Expected error");
    };
    assert_eq!(e.reason.to_string(), "Channel has not been signed");

    let keystore = create_keystore(&Bob.to_seed(), Ed25519)?;

    let signer = Signer::with_keystore(keystore);
    println!("signing");
    open_channel.sign(BigInt::from(0u128), &signer).await?;
    println!("signed");

    let json = open_channel.export_for_send().await?;
    let json_string = String::from_utf8(json.to_vec())?;
    println!("channel {}", &json_string);
    assert!(json_string.contains("channelHoldNote\":{"));

    assert_eq!(
      store
        .get(channel.id())
        .await?
        .inner()
        .await
        .get_final()
        .await?,
      open_channel.inner().await.get_final().await?,
      "can reload from db"
    );

    open_channel.sign(BigInt::from(10u128), &signer).await?;

    assert_eq!(
      store
        .get(channel.id())
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
  async fn test_importing_channel(bob_pool: SqlitePool) -> anyhow::Result<()> {
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
    let bob_hold = create_channel_hold(
      &bob_pool,
      &bob_account,
      20_000,
      1_000,
      JsDataDomain {
        domain_name: "Delta".to_string(),
        top_level_domain: DataTLD::Flights,
      },
      alice_address.clone(),
      1,
      1,
    )
    .await?;

    let ticker = TickerRef {
      ticker: Ticker::start(Duration::from_secs(60)),
    };
    let bob_store = OpenChannelsStore::new(bob_pool, ticker.clone(), &notary_clients);
    let open_channel = bob_store.open_client_channel(bob_account.id).await?;

    let signer = Signer::with_keystore(create_keystore(&Bob.to_seed(), Ed25519)?);
    open_channel.sign(BigInt::from(0u128), &signer).await?;
    let json = open_channel.export_for_send().await?;

    let alice_store = OpenChannelsStore::new(alice_pool, ticker, &notary_clients);
    // before registered with notary, should fail
    match alice_store.import_channel(json.clone()).await {
      Err(e) => {
        assert!(e.reason.contains("balance_tip not set"))
      }
      Ok(_) => {
        return Err(anyhow!("Expected error"));
      }
    }
    println!("registering balance tip");
    register_balance_tip(&bob_account, &mock_notary, &bob_hold, 1, 1).await?;

    println!("importing channel");
    let alice_channel = alice_store.import_channel(json).await?;
    println!("imported channel");
    let imported_channel = alice_channel.inner().await;
    let sent_channel = open_channel.inner().await;
    assert_eq!(imported_channel.to_address, sent_channel.to_address);
    assert_eq!(imported_channel.from_address, sent_channel.from_address);
    assert_eq!(
      imported_channel.expiration_tick,
      sent_channel.expiration_tick
    );
    assert_eq!(imported_channel.settled_amount, sent_channel.settled_amount);
    assert_eq!(
      imported_channel.settled_signature,
      sent_channel.settled_signature
    );
    assert_eq!(imported_channel.hold_amount, sent_channel.hold_amount);
    assert_eq!(imported_channel.data_domain, sent_channel.data_domain);
    assert_eq!(imported_channel.notary_id, sent_channel.notary_id);
    assert_eq!(
      imported_channel.balance_change_number,
      sent_channel.balance_change_number
    );

    assert_eq!(
      imported_channel.expiration_tick,
      1 + CHANNEL_EXPIRATION_TICKS
    );
    assert_eq!(imported_channel.settled_amount, 0);

    let result = open_channel.sign(BigInt::from(10u128), &signer).await?;
    alice_channel
      .record_updated_settlement(result.milligons, result.signature)
      .await?;
    assert_eq!(
      alice_channel.inner().await.settled_amount().get_u128().1,
      10_u128
    );

    Ok(())
  }
}
