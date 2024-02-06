use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use napi::bindgen_prelude::*;
use sp_core::H256;
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use ulx_primitives::{AccountType, BalanceChange, BalanceTip, Notarization};

use crate::accounts::LocalAccount;
use crate::balance_changes::{BalanceChangeRow, BalanceChangeStatus};
use crate::mainchain_client::MainchainClient;
use crate::{to_js_error, BalanceChangeStore, BalanceSync, NotaryAccountOrigin, NotaryClients};

#[napi]
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
}

#[napi]
impl NotarizationTracker {
  pub async fn get_balance_tips(&self) -> anyhow::Result<Vec<BalanceTip>> {
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
        account_type: change.account_type.clone(),
        balance: change.balance,
        account_origin: previous_balance_proof.account_origin,
        change_number: change.change_number,
        escrow_hold_note: change.escrow_hold_note.clone(),
      };
      tips.push(tip);
    }
    for (account_id, balance_change) in (*balance_changes).iter() {
      let account = self.accounts_by_id.get(&account_id).unwrap();
      let tip = balance_change.get_balance_tip(&account)?;
      tips.push(tip);
    }
    Ok(tips)
  }

  #[napi(getter)]
  /// Returns the balance changes that were submitted to the notary indexed by the stringified account id (napi doesn't allow numbers as keys)
  pub async fn balance_changes_by_account(&self) -> HashMap<String, BalanceChangeRow> {
    let balance_changes = self.balance_changes_by_account.lock().await;
    (*balance_changes)
      .iter()
      .map(|(k, v)| (k.to_string(), v.clone()))
      .collect()
  }

  #[napi]
  pub async fn wait_for_notebook(&self) -> Result<()> {
    let notary_client = self
      .notary_clients
      .get(self.notary_id)
      .await
      .map_err(to_js_error)?;
    notary_client
      .wait_for_notebook(self.notebook_number)
      .await
      .map_err(to_js_error)?;
    Ok(())
  }

  /// Asks the notary for proof the transaction was included in a notebook header. If this notebook has not been finalized yet, it will return an error.
  #[napi]
  pub async fn get_notebook_proof(&self) -> Result<Vec<NotebookProof>> {
    self.wait_for_notebook().await?;
    let mut balance_changes = self.balance_changes_by_account.lock().await;
    let mut proofs = vec![];
    for (_, balance_change) in (*balance_changes).iter_mut() {
      if balance_change.status == BalanceChangeStatus::SubmittedToNotary {
        BalanceSync::sync_notebook_proof(&self.db, balance_change, &self.notary_clients)
          .await
          .map_err(to_js_error)?;
      }

      if let Ok(Some(proof)) = balance_change.get_proof() {
        let account = self.accounts_by_id.get(&balance_change.account_id).unwrap();
        let tip = balance_change.get_balance_tip(&account)?;
        let notebook_proof = NotebookProof {
          address: account.address.clone(),
          account_type: account.account_type.clone(),
          notebook_number: self.notebook_number,
          balance_tip: tip.tip().into(),
          balance: tip.balance.into(),
          change_number: tip.change_number,
          account_origin: NotaryAccountOrigin {
            notary_id: self.notary_id,
            account_uid: tip.account_origin.account_uid,
            notebook_number: tip.account_origin.notebook_number,
          },
          escrow_hold_note_json: balance_change.escrow_hold_note_json.clone(),
          leaf_index: proof.leaf_index,
          number_of_leaves: proof.number_of_leaves,
          proof: proof.proof.iter().map(|p| p.0.to_vec().into()).collect(),
        };
        proofs.push(notebook_proof);
      }
    }
    Ok(proofs)
  }

  /// Confirms the root added to the mainchain
  #[napi]
  pub async fn wait_for_finalized(
    &self,
    mainchain_client: &MainchainClient,
  ) -> Result<FinalizedBlock> {
    let _ = self.get_notebook_proof().await?;
    let finalized_block = mainchain_client
      .wait_for_notebook_finalized(self.notary_id, self.notebook_number)
      .await?;
    let account_change_root = mainchain_client
      .get_account_changes_root(self.notary_id, self.notebook_number)
      .await?;
    let change_root = H256::from_slice(&account_change_root.as_ref()[..]);

    let mut balance_changes = self.balance_changes_by_account.lock().await;
    for (account_id, balance_change) in (*balance_changes).iter_mut() {
      let mut tx = self.db.begin().await.map_err(to_js_error)?;
      let account = self
        .accounts_by_id
        .get(&account_id)
        .expect("account not found");

      BalanceChangeStore::save_finalized(
        &mut tx,
        balance_change,
        &account,
        change_root,
        finalized_block,
      )
      .await
      .map_err(to_js_error)?;
      tx.commit().await.map_err(to_js_error)?;
    }

    Ok(FinalizedBlock {
      finalized_block_number: finalized_block,
      account_changes_merkle_root: account_change_root.into(),
    })
  }
}

#[napi(object)]
pub struct FinalizedBlock {
  pub finalized_block_number: u32,
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
  pub escrow_hold_note_json: Option<String>,
  pub leaf_index: u32,
  pub number_of_leaves: u32,
  pub proof: Vec<Uint8Array>,
}
