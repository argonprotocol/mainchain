use std::collections::BTreeMap;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use ulx_primitives::{AccountType, Note, NoteType};

use crate::mainchain_transfer::MainchainTransferIn;
use crate::transactions::TransactionType;
use crate::Result;
use crate::{AccountStore, BalanceChangeRow};
use crate::{BalanceChangeStatus, LocalAccount, MainchainClient};

#[derive(Clone, Debug, Default)]
pub struct LocalchainOverview {
  /// The name of this localchain
  pub name: String,
  /// The primary localchain address
  pub address: String,
  /// The current account balance
  pub balance: i128,
  /// The net pending balance change acceptance/confirmation
  pub pending_balance_change: i128,
  /// Balance held in escrow
  pub held_balance: i128,
  /// Tax accumulated for the account
  pub tax: i128,
  /// The net pending tax balance change
  pub pending_tax_change: i128,
  /// Changes to the account ordered from most recent to oldest
  pub changes: Vec<BalanceChangeGroup>,
  /// The mainchain balance
  pub mainchain_balance: i128,
  /// The net pending mainchain balance pending movement in/out of the localchain
  pub pending_mainchain_balance_change: i128,
}

#[derive(Clone, Debug)]
pub struct BalanceChangeGroup {
  pub net_balance_change: i128,
  pub net_tax: i128,
  pub held_balance: i128,
  pub timestamp: i64,
  pub notes: Vec<String>,
  pub finalized_block_number: Option<u32>,
  pub status: BalanceChangeStatus,
  pub notarization_id: Option<i64>,
  pub transaction_id: Option<i64>,
  pub transaction_type: Option<TransactionType>,
  pub balance_changes: Vec<BalanceChangeSummary>,
  pub notebook_number: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct BalanceChangeSummary {
  pub id: i64,
  pub final_balance: i128,
  pub hold_balance: i128,
  pub net_balance_change: i128,
  pub change_number: u32,
  pub account_id: i64,
  pub account_type: AccountType,
  pub is_jump_account: bool,
  pub notes: Vec<String>,
  pub status: BalanceChangeStatus,
  pub notebook_number: Option<u32>,
  pub finalized_block_number: Option<u32>,
}

fn get_note_descriptions(change: &BalanceChangeRow) -> Vec<String> {
  get_notes(&change)
    .iter()
    .map(|n| format!("{}", n))
    .collect()
}

fn get_notes(change: &BalanceChangeRow) -> Vec<Note> {
  serde_json::from_str(&change.notes_json).unwrap_or_default()
}

#[cfg_attr(feature = "napi", napi)]
pub struct OverviewStore {
  db: SqlitePool,
  name: String,
  mainchain_client: Arc<Mutex<Option<MainchainClient>>>,
}

impl OverviewStore {
  pub fn new(
    db: SqlitePool,
    name: String,
    mainchain_client: Arc<Mutex<Option<MainchainClient>>>,
  ) -> Self {
    Self {
      db,
      name,
      mainchain_client,
    }
  }

  pub async fn get(&self) -> Result<LocalchainOverview> {
    let mut overview = LocalchainOverview::default();
    overview.name = self.name.clone();

    let transactions_by_id: BTreeMap<i64, TransactionType> =
      sqlx::query!("SELECT * from transactions")
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(|t| (t.id, TransactionType::from(t.transaction_type)))
        .collect();

    let notarization_notebooks: BTreeMap<i64, u32> =
      sqlx::query!("SELECT notebook_number, id from notarizations")
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .filter_map(|n| n.notebook_number.map(|nb| (n.id, nb as u32)))
        .collect();

    let balance_changes = sqlx::query_as!(
      BalanceChangeRow,
      "SELECT * FROM balance_changes ORDER BY id DESC"
    )
    .fetch_all(&self.db)
    .await?;

    let pending_mainchain_transfers = sqlx::query_as!(
      MainchainTransferIn,
      "SELECT * FROM mainchain_transfers_in where balance_change_id IS NULL ORDER BY id DESC"
    )
    .fetch_all(&self.db)
    .await?;

    let mut db = self.db.acquire().await?;
    let accounts_by_id: BTreeMap<i64, LocalAccount> = AccountStore::db_list(&mut db, true)
      .await?
      .into_iter()
      .map(|a| (a.id, a))
      .collect();

    overview.address = accounts_by_id
      .values()
      .find(|a| a.account_type == AccountType::Deposit && a.hd_path.is_none())
      .map(|a| a.address.clone())
      .unwrap_or_default();

    if overview.address.is_empty() {
      return Ok(overview);
    }

    for transfer in pending_mainchain_transfers {
      overview.pending_mainchain_balance_change -= transfer.amount.parse::<i128>()?;
    }

    if let Some(mainchain_client) = self.mainchain_client.lock().await.as_ref() {
      if let Ok(account) = mainchain_client.get_account(overview.address.clone()).await {
        overview.mainchain_balance = account.data.free as i128;
      }
    }

    for change in balance_changes {
      if change.status == BalanceChangeStatus::Canceled {
        continue;
      }

      let account = accounts_by_id
        .get(&change.account_id)
        .expect("Account should be present");

      let mut balance_change = BalanceChangeSummary {
        id: change.id,
        final_balance: change.balance.parse::<i128>()?.into(),
        hold_balance: 0i128,
        net_balance_change: change.net_balance_change.parse::<i128>()?.into(),
        change_number: change.change_number as u32,
        account_id: change.account_id,
        account_type: account.account_type.clone(),
        is_jump_account: account.hd_path.is_some(),
        notes: get_note_descriptions(&change),
        status: change.status,
        notebook_number: change
          .notarization_id
          .map(|id| notarization_notebooks.get(&id).unwrap())
          .copied(),
        finalized_block_number: change.finalized_block_number.map(|n| n as u32),
      };

      let notes = get_notes(&change);
      if let Some(note) = notes
        .iter()
        .find(|n| matches!(n.note_type, NoteType::EscrowHold { .. }))
      {
        overview.held_balance += note.milligons as i128;
        balance_change.hold_balance = note.milligons as i128;
      }

      let net_balance_change = change.net_balance_change.parse::<i128>()?;
      let change_group = BalanceChangeGroup {
        net_balance_change: net_balance_change.into(),
        net_tax: 0i128,
        held_balance: balance_change.hold_balance.clone(),
        notes: get_note_descriptions(&change),
        finalized_block_number: change.finalized_block_number.map(|n| n as u32),
        status: change.status,
        timestamp: change.timestamp.and_utc().timestamp_millis(),
        notarization_id: change.notarization_id,
        transaction_id: change.transaction_id,
        transaction_type: change
          .transaction_id
          .map(|id| transactions_by_id.get(&id).cloned().unwrap()),
        notebook_number: change
          .notarization_id
          .map(|id| notarization_notebooks.get(&id).unwrap())
          .copied(),
        balance_changes: vec![balance_change.clone()],
      };

      let existing = if let Some(id) = change.transaction_id {
        overview
          .changes
          .iter_mut()
          .find(|c| c.transaction_id == Some(id))
      } else if let Some(id) = change.notarization_id {
        overview
          .changes
          .iter_mut()
          .find(|c| c.notarization_id == Some(id))
      } else {
        None
      };

      if let Some(existing) = existing {
        existing.balance_changes.push(balance_change.clone());
        let existing_hold = existing.held_balance;
        if existing_hold > 0i128 {
          existing.held_balance = existing_hold + balance_change.hold_balance;
        }
      } else {
        overview.changes.push(change_group);
      }

      let is_pending = is_pending(&change.status);
      if account.account_type == AccountType::Tax {
        if is_pending {
          overview.pending_tax_change += net_balance_change;
        } else {
          overview.tax += net_balance_change;
        }
      } else {
        if is_pending {
          overview.pending_balance_change += net_balance_change;
        } else {
          overview.balance += net_balance_change;
        }
      }
    }

    for group in overview.changes.iter_mut() {
      group.balance_changes.sort_by(|a, b| b.id.cmp(&a.id));

      let is_transaction = group.transaction_id.is_some();
      for change in &group.balance_changes {
        if change.account_type == AccountType::Tax {
          group.net_tax += change.net_balance_change;
        } else {
          group.net_balance_change += change.net_balance_change;
        }
      }

      let change = group
        .balance_changes
        .iter()
        .find(|change| {
          if is_transaction {
            match group.transaction_type {
              Some(TransactionType::OpenEscrow) => {
                return change.is_jump_account
                  && change.account_type == AccountType::Deposit
                  && change.notes[0].starts_with("Escrow");
              }
              Some(TransactionType::Consolidation) => {
                return !change.is_jump_account;
              }
              _ => {}
            }
            return change.is_jump_account && change.account_type == AccountType::Deposit;
          } else {
            return change.account_type == AccountType::Deposit;
          }
        })
        .unwrap_or(group.balance_changes.first().unwrap());

      group.status = change.status;
      group.notes = change.notes.clone();
    }

    Ok(overview)
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use super::OverviewStore;
  use napi::bindgen_prelude::*;

  use crate::transactions::TransactionType;
  use crate::BalanceChangeStatus;
  use ulx_primitives::AccountType;
  use crate::error::NapiOk;

  #[napi(object)]
  #[derive(Clone, Debug)]
  pub struct LocalchainOverview {
    /// The name of this localchain
    pub name: String,
    /// The primary localchain address
    pub address: String,
    /// The current account balance
    pub balance: BigInt,
    /// The net pending balance change acceptance/confirmation
    pub pending_balance_change: BigInt,
    /// Balance held in escrow
    pub held_balance: BigInt,
    /// Tax accumulated for the account
    pub tax: BigInt,
    /// The net pending tax balance change
    pub pending_tax_change: BigInt,
    /// Changes to the account ordered from most recent to oldest
    pub changes: Vec<BalanceChangeGroup>,
    /// The mainchain balance
    pub mainchain_balance: BigInt,
    /// The net pending mainchain balance pending movement in/out of the localchain
    pub pending_mainchain_balance_change: BigInt,
  }

  #[napi(object)]
  #[derive(Clone, Debug)]
  pub struct BalanceChangeGroup {
    pub net_balance_change: BigInt,
    pub net_tax: BigInt,
    pub held_balance: BigInt,
    pub timestamp: i64,
    pub notes: Vec<String>,
    pub finalized_block_number: Option<u32>,
    pub status: BalanceChangeStatus,
    pub notarization_id: Option<i64>,
    pub transaction_id: Option<i64>,
    pub transaction_type: Option<TransactionType>,
    pub balance_changes: Vec<BalanceChangeSummary>,
    pub notebook_number: Option<u32>,
  }

  #[napi(object)]
  #[derive(Clone, Debug)]
  pub struct BalanceChangeSummary {
    pub id: i64,
    pub final_balance: BigInt,
    pub hold_balance: BigInt,
    pub net_balance_change: BigInt,
    pub change_number: u32,
    pub account_id: i64,
    pub account_type: AccountType,
    pub is_jump_account: bool,
    pub notes: Vec<String>,
    pub status: BalanceChangeStatus,
    pub notebook_number: Option<u32>,
    pub finalized_block_number: Option<u32>,
  }

  impl Into<LocalchainOverview> for super::LocalchainOverview {
    fn into(self) -> LocalchainOverview {
      LocalchainOverview {
        name: self.name,
        address: self.address,
        balance: self.balance.into(),
        pending_balance_change: self.pending_balance_change.into(),
        held_balance: self.held_balance.into(),
        tax: self.tax.into(),
        pending_tax_change: self.pending_tax_change.into(),
        changes: self.changes.into_iter().map(|c| c.into()).collect::<Vec<_>>(),
        mainchain_balance: self.mainchain_balance.into(),
        pending_mainchain_balance_change: self.pending_mainchain_balance_change.into(),
      }
    }
  }
  impl Into<BalanceChangeGroup> for super::BalanceChangeGroup {
    fn into(self) -> BalanceChangeGroup {
      BalanceChangeGroup {
        net_balance_change: self.net_balance_change.into(),
        net_tax: self.net_tax.into(),
        held_balance: self.held_balance.into(),
        timestamp: self.timestamp,
        notes: self.notes,
        finalized_block_number: self.finalized_block_number,
        status: self.status,
        notarization_id: self.notarization_id,
        transaction_id: self.transaction_id,
        transaction_type: self.transaction_type,
        balance_changes: self
          .balance_changes
          .into_iter()
          .map(|c| BalanceChangeSummary {
            id: c.id,
            final_balance: c.final_balance.into(),
            hold_balance: c.hold_balance.into(),
            net_balance_change: c.net_balance_change.into(),
            change_number: c.change_number,
            account_id: c.account_id,
            account_type: c.account_type,
            is_jump_account: c.is_jump_account,
            notes: c.notes,
            status: c.status,
            notebook_number: c.notebook_number,
            finalized_block_number: c.finalized_block_number,
          })
          .collect::<Vec<_>>(),
        notebook_number: self.notebook_number,
      }
    }
  }

  #[napi]
  impl OverviewStore {
    #[napi(js_name = "get")]
    pub async fn get_napi(&self) -> napi::Result<LocalchainOverview> {
      self.get().await.map(Into::into).napi_ok()
    }
  }
}

fn is_pending(status: &BalanceChangeStatus) -> bool {
  matches!(
    status,
    BalanceChangeStatus::SubmittedToNotary | BalanceChangeStatus::WaitingForSendClaim
  )
}

#[cfg(test)]
mod tests {
  use sp_keyring::AccountKeyring::{Alice, Bob};
  use sp_keyring::Ed25519Keyring::Ferdie;

  use crate::overview::OverviewStore;
  use crate::test_utils::{create_mock_notary, create_pool, mock_localchain, mock_notary_clients};
  use crate::transactions::TransactionType;
  use crate::CryptoScheme::{Ed25519, Sr25519};
  use crate::*;

  #[sqlx::test]
  async fn test_overview_of_send_transaction_flow(bob_pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;

    let bob_localchain = mock_localchain(&bob_pool, &Bob.to_seed(), Ed25519, &notary_clients).await;

    let alice_pool = create_pool().await?;
    let alice_localchain =
      mock_localchain(&alice_pool, &Alice.to_seed(), Sr25519, &notary_clients).await;

    mock_notary
      .create_claim_from_mainchain(
        alice_localchain.begin_change(),
        5_000u128,
        Alice.to_account_id(),
      )
      .await?;

    let alice_overview =
      OverviewStore::new(alice_pool.clone(), "alice".to_string(), Default::default());
    {
      let overview = alice_overview.get().await?;
      println!("Alice {:#?}", overview);
      assert_eq!(overview.balance, 5000);
      assert_eq!(overview.pending_balance_change, 0);
      assert_eq!(overview.tax, 0);
      assert_eq!(overview.pending_tax_change, 0);
      assert_eq!(overview.changes.len(), 1);
      assert_eq!(
        overview.changes[0].status,
        BalanceChangeStatus::NotebookPublished
      );
      assert_eq!(
        overview.changes[0].notes,
        vec!["ClaimFromMainchain(nonce=1) ₳5.0"]
      );
    }

    let alice_json = alice_localchain
      .transactions()
      .send(
        3500_u128.into(),
        Some(vec![bob_localchain.address().await?]),
      )
      .await?;
    {
      let overview = alice_overview.get().await?;
      println!("Alice {:#?}", overview);
      assert_eq!(overview.balance, 5000);
      assert_eq!(overview.pending_balance_change, -3500);
      assert_eq!(overview.tax, 0);
      assert_eq!(overview.pending_tax_change, 200);
      assert_eq!(overview.changes.len(), 2);
      assert_eq!(
        overview.changes[0].status,
        BalanceChangeStatus::WaitingForSendClaim
      );
      assert_eq!(
        overview.changes[0].notes,
        vec![format!(
          "Send(restrictedTo: [{:?}]) ₳3.3",
          bob_localchain.address().await?
        )]
      );
    }

    let bob_builder = bob_localchain.begin_change();
    bob_builder.import_argon_file(alice_json).await?;
    let _ = bob_builder.notarize().await?;
    let bob_overview = OverviewStore::new(bob_pool.clone(), "bob".to_string(), Default::default());
    {
      let overview = bob_overview.get().await?;
      println!("Bob {:#?}", overview);
      assert_eq!(overview.balance, 0);
      assert_eq!(overview.pending_balance_change, 3100);
      assert_eq!(overview.tax, 0);
      assert_eq!(overview.pending_tax_change, 200);
      assert_eq!(overview.changes.len(), 1);
      assert_eq!(
        overview.changes[0].status,
        BalanceChangeStatus::SubmittedToNotary
      );
      assert_eq!(overview.changes[0].notes, vec!["Claim ₳3.3", "Tax ₳0.2"]);
    }

    let pending_tips = mock_notary.get_pending_tips().await;
    mock_notary.create_notebook_header(pending_tips).await;

    {
      bob_localchain.balance_sync().sync(None).await?;
      let overview = bob_overview.get().await?;
      println!("Bob {:#?}", overview);
      assert_eq!(overview.balance, 3100);
      assert_eq!(overview.pending_balance_change, 0);
      assert_eq!(overview.tax, 200);
      assert_eq!(overview.pending_tax_change, 0);
      assert_eq!(overview.changes.len(), 1);
      assert_eq!(
        overview.changes[0].status,
        BalanceChangeStatus::NotebookPublished
      );
      assert_eq!(overview.changes[0].notebook_number, Some(2));
      assert_eq!(overview.changes[0].notes, vec!["Claim ₳3.3", "Tax ₳0.2"]);
    }
    {
      alice_localchain.balance_sync().sync(None).await?;
      let overview = alice_overview.get().await?;
      println!("Alice {:#?}", overview);
      assert_eq!(overview.balance, 1500);
      assert_eq!(overview.pending_balance_change, 0);
      assert_eq!(overview.tax, 200);
      assert_eq!(overview.pending_tax_change, 0);
      assert_eq!(overview.changes.len(), 3);
      assert_eq!(
        overview.changes[0].status,
        BalanceChangeStatus::SubmittedToNotary
      );
      assert_eq!(
        overview.changes[0].transaction_type,
        Some(TransactionType::Consolidation)
      );
      assert_eq!(overview.changes[0].notebook_number, Some(3));
      assert_eq!(
        overview.changes[1].status,
        BalanceChangeStatus::NotebookPublished
      );
      assert_eq!(overview.changes[1].notebook_number, Some(2));
    }
    Ok(())
  }
}
