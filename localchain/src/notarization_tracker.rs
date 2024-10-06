use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use sp_core::H256;
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use argon_primitives::{AccountType, Balance, BalanceChange, BalanceTip, Notarization};

use crate::accounts::LocalAccount;
use crate::balance_changes::{BalanceChangeRow, BalanceChangeStatus};
use crate::mainchain_client::MainchainClient;
use crate::{BalanceChangeStore, BalanceSync, NotaryAccountOrigin, NotaryClients};
use crate::{ChannelHold, Result};

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone)]
pub struct NotarizationTracker {
  pub notebook_number: u32,
  pub tick: u32,
  pub notary_id: u32,
  pub notarization_id: i64,
  pub notarized_balance_changes: u32,
  pub notarized_votes: u32,
  #[allow(dead_code)]
  pub(crate) notarization: Notarization,
  pub(crate) imports: Vec<BalanceChange>,
  pub(crate) balance_changes_by_account: Arc<Mutex<HashMap<i64, BalanceChangeRow>>>,
  pub(crate) accounts_by_id: HashMap<i64, LocalAccount>,
  pub(crate) notary_clients: NotaryClients,
  pub(crate) db: SqlitePool,
  pub channel_holds: Vec<ChannelHold>,
}

impl NotarizationTracker {
  pub async fn get_balance_tips(&self) -> Result<Vec<BalanceTip>> {
    let balance_changes = self.balance_changes_by_account.lock().await;
    let mut tips: Vec<BalanceTip> = vec![];
    for change in &self.imports {
      tips
        .retain(|t| !(t.account_id == change.account_id && t.account_type == change.account_type));
      let previous_balance_proof = change.previous_balance_proof.clone().ok_or(anyhow!(
        "Balance change {:?} is missing previous balance proof",
        change
      ))?;
      let tip = BalanceTip {
        account_id: change.account_id.clone(),
        account_type: change.account_type,
        balance: change.balance,
        account_origin: previous_balance_proof.account_origin,
        change_number: change.change_number,
        channel_hold_note: change.channel_hold_note.clone(),
      };
      tips.push(tip);
    }
    for (account_id, balance_change) in (*balance_changes).iter() {
      let account = self.accounts_by_id.get(account_id).unwrap();
      let tip = balance_change.get_balance_tip(account)?;
      tips.push(tip);
    }
    Ok(tips)
  }

  pub async fn get_changed_accounts(&self) -> Vec<(LocalAccount, BalanceChangeRow)> {
    let balance_changes = self.balance_changes_by_account.lock().await;
    let mut changed_accounts = vec![];
    for (account_id, balance_change) in (*balance_changes).iter() {
      let account = self.accounts_by_id.get(account_id).unwrap();
      changed_accounts.push((account.clone(), balance_change.clone()));
    }
    changed_accounts
  }

  /// Returns the balance changes that were submitted to the notary indexed by the stringified account id (napi doesn't allow numbers as keys)
  pub async fn balance_changes_by_account_id(&self) -> HashMap<String, BalanceChangeRow> {
    let balance_changes = self.balance_changes_by_account.lock().await;
    (*balance_changes)
      .iter()
      .map(|(k, v)| (k.to_string(), v.clone()))
      .collect()
  }

  pub async fn wait_for_notebook(&self) -> Result<()> {
    let notary_client = self.notary_clients.get(self.notary_id).await?;
    notary_client
      .wait_for_notebook(self.notebook_number)
      .await?;
    Ok(())
  }

  /// Asks the notary for proof the transaction was included in a notebook header. If this notebook has not been finalized yet, it will return an error.
  pub async fn get_notebook_proof(&self) -> Result<Vec<NotebookProof>> {
    self.wait_for_notebook().await?;
    let mut balance_changes = self.balance_changes_by_account.lock().await;
    let mut proofs = vec![];
    for (_, balance_change) in (*balance_changes).iter_mut() {
      if balance_change.status == BalanceChangeStatus::Notarized {
        BalanceSync::sync_notebook_proof(&self.db, balance_change, &self.notary_clients).await?;
      }

      if let Ok(Some(proof)) = balance_change.get_proof() {
        let account = self.accounts_by_id.get(&balance_change.account_id).unwrap();
        let tip = balance_change.get_balance_tip(account)?;
        let notebook_proof = NotebookProof {
          address: account.address.clone(),
          account_type: account.account_type,
          notebook_number: self.notebook_number,
          balance_tip: tip.tip().into(),
          balance: tip.balance,
          change_number: tip.change_number,
          account_origin: NotaryAccountOrigin {
            notary_id: self.notary_id,
            account_uid: tip.account_origin.account_uid,
            notebook_number: tip.account_origin.notebook_number,
          },
          channel_hold_note_json: balance_change.channel_hold_note_json.clone(),
          leaf_index: proof.leaf_index,
          number_of_leaves: proof.number_of_leaves,
          proof: proof.proof.to_vec(),
        };
        proofs.push(notebook_proof);
      }
    }
    Ok(proofs)
  }

  /// Confirms the root added to the mainchain
  pub async fn wait_for_immortalized(
    &self,
    mainchain_client: &MainchainClient,
  ) -> Result<ImmortalizedBlock> {
    let _ = self.get_notebook_proof().await?;
    let immortalized_block = mainchain_client
      .wait_for_notebook_immortalized(self.notary_id, self.notebook_number)
      .await?;
    let account_change_root = mainchain_client
      .get_account_changes_root(self.notary_id, self.notebook_number)
      .await?
      .ok_or(anyhow!("Account changes root not found"))?;
    let change_root = H256::from_slice(account_change_root.as_ref());

    let mut balance_changes = self.balance_changes_by_account.lock().await;
    for (account_id, balance_change) in (*balance_changes).iter_mut() {
      let mut tx = self.db.begin().await?;
      let account = self
        .accounts_by_id
        .get(account_id)
        .expect("account not found");

      BalanceChangeStore::tx_save_immortalized(
        &mut tx,
        balance_change,
        account,
        change_root,
        immortalized_block,
      )
      .await?;
      tx.commit().await?;
    }

    Ok(ImmortalizedBlock {
      immortalized_block,
      account_changes_merkle_root: account_change_root,
    })
  }
}
pub struct ImmortalizedBlock {
  pub immortalized_block: u32,
  pub account_changes_merkle_root: H256,
}

pub struct NotebookProof {
  pub address: String,
  pub account_type: AccountType,
  pub notebook_number: u32,
  pub balance: Balance,
  /// H256 hash of the balance tip
  pub balance_tip: H256,
  pub change_number: u32,
  pub account_origin: NotaryAccountOrigin,
  pub channel_hold_note_json: Option<String>,
  pub leaf_index: u32,
  pub number_of_leaves: u32,
  pub proof: Vec<H256>,
}

#[cfg(feature = "uniffi")]
pub mod uniffi_ext {
  use argon_primitives::AccountType;

  use crate::error::UniffiResult;
  use crate::BalanceChangeStatus;
  use crate::NotaryAccountOrigin;
  use std::collections::HashMap;

  #[derive(uniffi::Object, Debug)]
  #[uniffi::export(Debug)]
  pub struct ImmortalizedBlock {
    pub immortalized_block: u32,
    pub account_changes_merkle_root: Vec<u8>,
  }

  #[derive(uniffi::Record, Debug)]
  pub struct NotebookProof {
    pub address: String,
    pub account_type: AccountType,
    pub notebook_number: u32,
    pub balance: String,
    /// H256 hash of the balance tip
    pub balance_tip: Vec<u8>,
    pub change_number: u32,
    pub account_origin: NotaryAccountOrigin,
    pub channel_hold_note_json: Option<String>,
    pub leaf_index: u32,
    pub number_of_leaves: u32,
    pub proof: Vec<Vec<u8>>,
  }

  #[derive(uniffi::Object)]
  pub struct NotarizationTracker {
    inner: super::NotarizationTracker,
  }

  impl NotarizationTracker {
    pub fn new(inner: super::NotarizationTracker) -> Self {
      NotarizationTracker { inner }
    }
  }

  impl From<super::NotarizationTracker> for NotarizationTracker {
    fn from(inner: super::NotarizationTracker) -> Self {
      NotarizationTracker::new(inner)
    }
  }

  #[derive(uniffi::Record, Debug)]
  pub struct BalanceChange {
    pub id: i64,
    pub account_id: i64,
    pub change_number: i64,
    pub balance: String,
    pub net_balance_change: String,
    pub channel_hold_note_json: Option<String>,
    pub notary_id: i64,
    pub notes_json: String,
    pub proof_json: Option<String>,
    pub finalized_block_number: Option<i64>,
    pub status: BalanceChangeStatus,
    pub transaction_id: Option<i64>,
    pub notarization_id: Option<i64>,
  }

  impl From<crate::BalanceChangeRow> for BalanceChange {
    fn from(row: crate::BalanceChangeRow) -> Self {
      BalanceChange {
        id: row.id,
        account_id: row.account_id,
        change_number: row.change_number,
        balance: row.balance.to_string(),
        net_balance_change: row.net_balance_change.to_string(),
        channel_hold_note_json: row.channel_hold_note_json,
        notary_id: row.notary_id,
        notes_json: row.notes_json,
        proof_json: row.proof_json,
        finalized_block_number: row.finalized_block_number,
        status: row.status,
        transaction_id: row.transaction_id,
        notarization_id: row.notarization_id,
      }
    }
  }

  #[uniffi::export(async_runtime = "tokio")]
  impl NotarizationTracker {
    #[uniffi::method(name = "balanceChangesByAccountId")]
    /// Returns the balance changes that were submitted to the notary indexed by the stringified account id (napi doesn't allow numbers as keys)
    pub async fn balance_changes_by_account_id_uniffi(&self) -> HashMap<String, BalanceChange> {
      let result = self.inner.balance_changes_by_account_id().await;
      result.into_iter().map(|(k, v)| (k, v.into())).collect()
    }
    #[uniffi::method(name = "waitForNotebook")]
    pub async fn wait_for_notebook_uniffi(&self) -> UniffiResult<()> {
      Ok(self.inner.wait_for_notebook().await?)
    }
    /// Asks the notary for proof the transaction was included in a notebook header. If this notebook has not been finalized yet, it will return an error.
    #[uniffi::method(name = "getNotebookProof")]
    pub async fn get_notebook_proof_uniffi(&self) -> UniffiResult<Vec<NotebookProof>> {
      let proofs = self.inner.get_notebook_proof().await?;
      Ok(
        proofs
          .into_iter()
          .map(|p| NotebookProof {
            address: p.address,
            account_type: p.account_type,
            notebook_number: p.notebook_number,
            balance: p.balance.to_string(),
            balance_tip: p.balance_tip.0.to_vec(),
            change_number: p.change_number,
            account_origin: p.account_origin,
            channel_hold_note_json: p.channel_hold_note_json,
            leaf_index: p.leaf_index,
            number_of_leaves: p.number_of_leaves,
            proof: p
              .proof
              .into_iter()
              .map(|p| p.0.to_vec())
              .collect::<Vec<_>>(),
          })
          .collect::<Vec<_>>(),
      )
    }
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use std::collections::HashMap;

  use napi::bindgen_prelude::{BigInt, Uint8Array};

  use argon_primitives::AccountType;

  use crate::error::NapiOk;
  use crate::MainchainClient;
  use crate::NotaryAccountOrigin;
  use crate::{BalanceChangeRow, ChannelHold};

  use super::NotarizationTracker;

  #[napi(object)]
  pub struct ImmortalizedBlock {
    pub immortalized_block: u32,
    pub account_changes_merkle_root: Uint8Array,
  }

  #[napi(object)]
  pub struct NotebookProof {
    pub address: String,
    pub account_type: AccountType,
    pub notebook_number: u32,
    pub balance: BigInt,
    /// H256 hash of the balance tip
    pub balance_tip: Uint8Array,
    pub change_number: u32,
    pub account_origin: NotaryAccountOrigin,
    pub channel_hold_note_json: Option<String>,
    pub leaf_index: u32,
    pub number_of_leaves: u32,
    pub proof: Vec<Uint8Array>,
  }

  #[napi]
  impl NotarizationTracker {
    #[napi(getter, js_name = "balanceChangesByAccountId")]
    /// Returns the balance changes that were submitted to the notary indexed by the stringified account id (napi doesn't allow numbers as keys)
    pub async fn balance_changes_by_account_id_napi(&self) -> HashMap<String, BalanceChangeRow> {
      self.balance_changes_by_account_id().await
    }
    #[napi(js_name = "waitForNotebook")]
    pub async fn wait_for_notebook_napi(&self) -> napi::Result<()> {
      self.wait_for_notebook().await.napi_ok()
    }
    #[napi(getter, js_name = "channelHolds")]
    pub async fn channel_holds_napi(&self) -> Vec<ChannelHold> {
      self.channel_holds().await
    }

    /// Asks the notary for proof the transaction was included in a notebook header. If this notebook has not been finalized yet, it will return an error.
    #[napi(js_name = "getNotebookProof")]
    pub async fn get_notebook_proof_napi(&self) -> napi::Result<Vec<NotebookProof>> {
      self
        .get_notebook_proof()
        .await
        .napi_ok()?
        .into_iter()
        .map(|p| {
          Ok(NotebookProof {
            address: p.address,
            account_type: p.account_type,
            notebook_number: p.notebook_number,
            balance: p.balance.into(),
            balance_tip: p.balance_tip.0.to_vec().into(),
            change_number: p.change_number,
            account_origin: p.account_origin,
            channel_hold_note_json: p.channel_hold_note_json,
            leaf_index: p.leaf_index,
            number_of_leaves: p.number_of_leaves,
            proof: p
              .proof
              .into_iter()
              .map(|p| p.0.to_vec().into())
              .collect::<Vec<_>>(),
          })
        })
        .collect::<napi::Result<Vec<NotebookProof>>>()
    }
    /// Confirms the root added to the mainchain
    #[napi(js_name = "waitForImmortalized")]
    pub async fn wait_for_immortalized_napi(
      &self,
      mainchain_client: &MainchainClient,
    ) -> napi::Result<ImmortalizedBlock> {
      let result = self
        .wait_for_immortalized(mainchain_client)
        .await
        .napi_ok()?;
      Ok(ImmortalizedBlock {
        immortalized_block: result.immortalized_block,
        account_changes_merkle_root: result.account_changes_merkle_root.0.to_vec().into(),
      })
    }
  }
}
