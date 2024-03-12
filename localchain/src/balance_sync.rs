use crate::accounts::AccountStore;
use crate::balance_changes::{BalanceChangeRow, BalanceChangeStatus, BalanceChangeStore};
use crate::keystore::Keystore;
use crate::notarization_builder::NotarizationBuilder;
use crate::notarization_tracker::NotarizationTracker;
use crate::open_escrows::OpenEscrowsStore;
use crate::transactions::{TransactionType, Transactions};
use crate::{to_js_error, NotaryAccountOrigin, TickerRef};
use crate::{Escrow, MainchainClient};
use crate::{LocalAccount, NOTARIZATION_MAX_BALANCE_CHANGES};
use crate::{Localchain, OpenEscrow};
use crate::{NotaryClient, NotaryClients};
use napi::bindgen_prelude::*;
use serde_json::json;
use sp_core::sr25519::Signature;
use sp_core::Decode;
use sp_core::H256;
use sp_runtime::MultiSignature;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use ulx_primitives::{AccountType, BlockVote};

#[napi]
pub struct BalanceSync {
  db: SqlitePool,
  ticker: TickerRef,
  mainchain_client: Arc<Mutex<Option<MainchainClient>>>,
  notary_clients: NotaryClients,
  lock: Arc<Mutex<()>>,
  open_escrows: OpenEscrowsStore,
  keystore: Keystore,
  tick_counter: Arc<Mutex<(u32, u32)>>,
}

#[napi(object)]
#[derive(Clone)]
pub struct EscrowCloseOptions {
  pub votes_address: Option<String>,
  /// What's the minimum amount of tax we should wait for before voting on blocks
  pub minimum_vote_amount: Option<i64>,
}

#[napi]
pub struct BalanceSyncResult {
  pub(crate) balance_changes: Vec<BalanceChangeRow>,
  pub(crate) escrow_notarizations: Vec<NotarizationBuilder>,
  pub(crate) jump_account_consolidations: Vec<NotarizationTracker>,
}
#[napi]
impl BalanceSyncResult {
  #[napi(getter)]
  pub fn balance_changes(&self) -> Vec<BalanceChangeRow> {
    self.balance_changes.clone()
  }
  #[napi(getter)]
  pub fn escrow_notarizations(&self) -> Vec<NotarizationBuilder> {
    self.escrow_notarizations.clone()
  }

  #[napi(getter)]
  pub fn jump_account_consolidations(&self) -> Vec<NotarizationTracker> {
    self.jump_account_consolidations.clone()
  }
}

#[napi]
impl BalanceSync {
  #[napi(constructor)]
  pub fn new(localchain: &Localchain) -> Self {
    BalanceSync {
      db: localchain.db.clone(),
      ticker: localchain.ticker.clone(),
      mainchain_client: localchain.mainchain_client.clone(),
      notary_clients: localchain.notary_clients.clone(),
      lock: Arc::new(Mutex::new(())),
      open_escrows: localchain.open_escrows(),
      tick_counter: Arc::new(Mutex::new((0, 0))),
      keystore: localchain.keystore.clone(),
    }
  }

  #[napi]
  pub async fn sync(&self, options: Option<EscrowCloseOptions>) -> Result<BalanceSyncResult> {
    let balance_changes = self.sync_unsettled_balances().await?;

    tracing::debug!(
      "Finished processing unsettled balances {}",
      balance_changes.len(),
    );

    let escrow_notarizations = self.process_pending_escrows(options).await?;

    let jump_account_consolidations = self.consolidate_jump_accounts().await?;

    Ok(BalanceSyncResult {
      balance_changes,
      escrow_notarizations,
      jump_account_consolidations,
    })
  }

  #[napi]
  pub async fn consolidate_jump_accounts(&self) -> Result<Vec<NotarizationTracker>> {
    let mut db = self.db.acquire().await.map_err(to_js_error)?;

    let all_accounts = AccountStore::list(&mut db, true)
      .await
      .map_err(to_js_error)?;
    let mut notarizations: Vec<NotarizationTracker> = vec![];
    let mut notarization = NotarizationBuilder::new(
      self.db.clone(),
      self.notary_clients.clone(),
      self.keystore.clone(),
    );
    let transaction = Transactions::create_static(&mut db, TransactionType::Consolidation).await?;
    notarization.set_transaction(transaction).await;
    for jump_account in all_accounts {
      let latest = BalanceChangeStore::get_latest_for_account(&mut db, jump_account.id)
        .await
        .map_err(to_js_error)?;
      if let Some(latest) = latest {
        let balance = latest.balance.parse::<u128>().map_err(to_js_error)?;
        if balance > 0 {
          let claim_account = match jump_account.account_type {
            AccountType::Deposit => notarization.default_deposit_account().await?,
            AccountType::Tax => notarization.default_tax_account().await?,
          };
          if claim_account.local_account_id == jump_account.id {
            continue;
          }
          notarization
            .load_account(&jump_account)
            .await?
            .send(
              BigInt::from(balance),
              Some(vec![claim_account.address.clone()]),
            )
            .await?;
          let claim_result = claim_account.claim(BigInt::from(balance)).await?;
          if claim_result.tax.get_u128().1 > 0 {
            notarization
              .default_tax_account()
              .await?
              .claim(claim_result.tax)
              .await?;
          }
          if notarization.accounts().await.len() >= NOTARIZATION_MAX_BALANCE_CHANGES as usize {
            let tracker = notarization.notarize().await?;
            notarizations.push(tracker);
            notarization = NotarizationBuilder::new(
              self.db.clone(),
              self.notary_clients.clone(),
              self.keystore.clone(),
            );
            let transaction =
              Transactions::create_static(&mut db, TransactionType::Consolidation).await?;
            notarization.set_transaction(transaction).await;
          }
        }
      }
    }
    if notarization.accounts().await.len() > 0 {
      let tracker = notarization.notarize().await?;
      notarizations.push(tracker);
    }

    return Ok(notarizations);
  }

  #[napi]
  pub async fn sync_unsettled_balances(&self) -> Result<Vec<BalanceChangeRow>> {
    let mut db = self.db.acquire().await.map_err(to_js_error)?;

    let pending_changes = BalanceChangeStore::find_unsettled(&mut db)
      .await
      .map_err(to_js_error)?;
    tracing::debug!("Found {} unsettled balance changes", pending_changes.len());

    let mut results = vec![];

    for change in pending_changes.into_iter() {
      let updated = self
        .sync_balance_change(&change)
        .await
        .map_err(to_js_error)?;
      results.push(updated);
    }
    Ok(results)
  }

  #[napi]
  pub async fn sync_balance_change(
    &self,
    balance_change: &BalanceChangeRow,
  ) -> Result<BalanceChangeRow> {
    let _lock = self.lock.lock().await;
    let mut change = balance_change.clone();

    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    tracing::debug!(
      "Checking status of balance change; id={}, change #{} (account={}), in status {:?}.",
      change.id,
      change.change_number,
      change.account_id,
      change.status
    );
    match BalanceChangeStore::check_if_superseded(&mut db, &mut change).await {
      Ok(true) => {
        tracing::debug!(
          "Balance Change superseded by another change; id={}.",
          change.id,
        );
        return Ok(change.clone());
      }
      Ok(false) => (),
      Err(e) => {
        tracing::warn!("Error checking if superseded (#{}): {:?}", change.id, e);
      }
    }

    let mut check_notary_for_tip = change.status == BalanceChangeStatus::WaitingForSendClaim;
    if change.status == BalanceChangeStatus::SubmittedToNotary {
      check_notary_for_tip =
        match Self::sync_notebook_proof(&self.db, &mut change, &self.notary_clients).await {
          Ok(x) => x,
          Err(e) => {
            tracing::warn!("Error syncing notebook proof (#{}): {:?}", change.id, e);
            true
          }
        };
    }

    if check_notary_for_tip {
      match Self::check_notary(&self.db, &mut change, &self.notary_clients).await {
        Ok(_) => {}
        Err(e) => {
          tracing::warn!("Error checking notary (#{}): {:?}", change.id, e);
        }
      }
    }

    if change.status == BalanceChangeStatus::NotebookPublished {
      match self.check_finalized(&mut change).await {
        Ok(_) => {}
        Err(e) => {
          tracing::warn!("Error checking finalized (#{}): {:?}", change.id, e);
        }
      }
    }
    return Ok(change);
  }

  #[napi]
  pub async fn process_pending_escrows(
    &self,
    options: Option<EscrowCloseOptions>,
  ) -> Result<Vec<NotarizationBuilder>> {
    let _lock = self.lock.lock().await;
    let open_escrows = self.open_escrows.get_claimable().await?;
    tracing::debug!(
      "Processing pending escrows. Found {} to check for claims.",
      open_escrows.len(),
    );

    let mut builder_by_notary = HashMap::new();

    let mut notarizations = vec![];

    for open_escrow in open_escrows {
      let mut escrow = open_escrow.inner().await;

      let notary_id = escrow.notary_id as u32;

      let notarization = builder_by_notary.entry(notary_id).or_insert_with(|| {
        NotarizationBuilder::new(
          self.db.clone(),
          self.notary_clients.clone(),
          self.keystore.clone(),
        )
      });

      if escrow.is_client {
        if let Err(e) = self
          .sync_client_escrow(notary_id, &open_escrow, &mut escrow, notarization)
          .await
        {
          tracing::warn!("Error syncing client escrow (#{}): {:?}", escrow.id, e);
        }
      } else {
        if let Err(e) = self
          .sync_server_escrow(&open_escrow, &mut escrow, &options, notarization)
          .await
        {
          tracing::warn!("Error syncing server escrow (#{}): {:?}", escrow.id, e);
        }
        if notarization.is_finalized().await {
          if let Some(n) = builder_by_notary.remove(&notary_id) {
            notarizations.push(n);
          }
        }
      }
    }

    for (_, mut notarization) in builder_by_notary {
      self
        .finalize_escrow_notarization(&mut notarization, &options)
        .await;
      if notarization.is_finalized().await {
        notarizations.push(notarization.clone());
      }
    }

    Ok(notarizations)
  }

  pub async fn get_available_tax_by_account(
    &self,
    notarization: &mut NotarizationBuilder,
  ) -> (u128, HashMap<i64, (LocalAccount, u128)>) {
    let accounts = notarization.accounts().await;
    let mut tax_accounts = HashMap::new();
    let mut total_available_tax = 0;

    for account in accounts {
      if account.account_type == AccountType::Deposit || account.hd_path.is_some() {
        continue;
      }
      if let Ok(balance) = notarization.get_balance_change(&account).await {
        let (_, tax, _) = balance.balance().await.get_u128();
        if tax > 0 {
          total_available_tax += tax;
          tax_accounts.insert(account.id, (account, tax));
        }
      }
    }
    (total_available_tax, tax_accounts)
  }

  pub async fn convert_tax_to_votes(
    &self,
    notarization: &mut NotarizationBuilder,
    options: &Option<EscrowCloseOptions>,
  ) -> anyhow::Result<()> {
    let Some(options) = options else {
      return Err(anyhow::anyhow!(
        "No options provided to create votes with tax"
      ));
    };
    let Some(votes_address) = options.votes_address.as_ref() else {
      return Err(anyhow::anyhow!(
        "No votes address provided to create votes with tax"
      ));
    };

    let mainchain_mutex = self.mainchain_client.lock().await;
    let Some(ref mainchain_client) = *mainchain_mutex else {
      return Err(anyhow::anyhow!(
        "Cannot create votes.. No mainchain client available!"
      ));
    };

    let (total_available_tax, tax_accounts) = self.get_available_tax_by_account(notarization).await;

    let current_tick = self.ticker.current();
    let Some(best_block_for_vote) = mainchain_client.get_vote_block_hash(current_tick).await?
    else {
      return Ok(());
    };

    if total_available_tax < options.minimum_vote_amount.unwrap_or_default() as u128
      || total_available_tax < best_block_for_vote.vote_minimum.get_u128().1
    {
      return Ok(());
    }

    let escrows = notarization.escrows().await;
    let Some((data_domain_hash, data_domain_address)) = escrows.into_iter().find_map(|c| {
      if c.is_client == true {
        return None;
      }
      if let Some(domain) = c.data_domain_hash {
        Some((domain, c.to_address))
      } else {
        None
      }
    }) else {
      return Ok(());
    };

    let mut tax_account = None;
    for (_, (account, tax)) in tax_accounts {
      let balance_change = notarization.get_balance_change(&account).await?;
      balance_change.send_to_vote(BigInt::from(tax)).await?;
      if tax_account.is_none() {
        tax_account = Some(account.clone());
      }
    }

    let mut tick_counter = self.tick_counter.lock().await;
    if (*tick_counter).0 == current_tick {
      (*tick_counter).1 += 1;
    } else {
      *tick_counter = (current_tick, 0);
    }
    let vote_address = votes_address.clone();
    let tax_account = tax_account.unwrap();
    let mut vote = BlockVote {
      account_id: tax_account.get_account_id32()?,
      power: total_available_tax,
      data_domain_hash: H256::from_slice(data_domain_hash.as_ref()),
      data_domain_account: AccountStore::parse_address(&data_domain_address)?,
      index: (*tick_counter).1,
      block_hash: H256::from_slice(best_block_for_vote.block_hash.as_ref()),
      block_rewards_account_id: AccountStore::parse_address(&vote_address)?,
      signature: Signature([0; 64]).into(),
    };
    let signature = self
      .keystore
      .sign(
        tax_account.address,
        Uint8Array::from(vote.hash().as_bytes().to_vec()),
      )
      .await?;
    vote.signature = MultiSignature::decode(&mut signature.as_ref())?;
    notarization.add_vote(vote).await?;

    Ok(())
  }

  pub async fn finalize_escrow_notarization(
    &self,
    notarization: &mut NotarizationBuilder,
    options: &Option<EscrowCloseOptions>,
  ) {
    if let Err(e) = self.convert_tax_to_votes(notarization, options).await {
      tracing::error!(
        "Error converting tax to votes: {:?}. Continuing with notarization",
        e
      );
    }

    if let Err(e) = notarization.sign().await {
      tracing::error!("Could not claim an escrow -> {:?}", e);
      return;
    }

    for i in 0..3 {
      if i > 0 {
        tracing::debug!("Retrying notarization finalization. Attempt #{}", i);
      }

      match notarization.notarize().await {
        Ok(tracker) => {
          tracing::info!(
            "Finalized escrow notarization. id={}, balance_changes={}, votes={}",
            tracker.notarization_id,
            tracker.notarized_balance_changes,
            tracker.notarized_votes
          );
          break;
        }
        Err(e) => {
          if e.reason.contains("Escrow hold not ready for claim") {
            let delay = 2 + i ^ 5;
            tracing::debug!("Escrow hold not ready for claim. Waiting {delay} seconds.");
            tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
            continue;
          }
          tracing::warn!("Error finalizing escrow notarization: {:?}", e);
        }
      }
    }
  }

  pub async fn sync_server_escrow(
    &self,
    open_escrow: &OpenEscrow,
    escrow: &mut Escrow,
    options: &Option<EscrowCloseOptions>,
    notarization: &mut NotarizationBuilder,
  ) -> anyhow::Result<()> {
    let current_tick = self.ticker.current();

    if escrow.is_past_claim_period(current_tick) {
      tracing::warn!(
        "Escrow expired and we missed claim window, marking unable to claim. id={}",
        escrow.id
      );
      let mut db = self.db.acquire().await?;
      escrow.mark_unable_to_claim(&mut db).await?;
      return Ok(());
    }

    tracing::debug!(
      "Server escrow {} ready for claim. escrow address={}, change number={}",
      escrow.id,
      escrow.from_address,
      escrow.balance_change_number
    );
    if !notarization.can_add_escrow(&open_escrow).await {
      self
        .finalize_escrow_notarization(notarization, options)
        .await;
      return Ok(());
    }
    notarization.claim_escrow(&open_escrow).await?;
    Ok(())
  }

  pub async fn sync_client_escrow(
    &self,
    notary_id: u32,
    open_escrow: &OpenEscrow,
    escrow: &mut Escrow,
    notarization: &mut NotarizationBuilder,
  ) -> anyhow::Result<()> {
    let tip = self
      .notary_clients
      .get(notary_id)
      .await?
      .get_balance_tip(escrow.from_address.clone(), AccountType::Deposit)
      .await?;

    let hold_notebook = escrow.hold_notebook_number();
    // hasn't changed - aka, nothing synced
    if tip.notebook_number == hold_notebook {
      let current_tick = self.ticker.current();
      if escrow.is_past_claim_period(current_tick) {
        tracing::info!(
          "An escrow was not claimed by the recipient. We're taking it back. id={}",
          escrow.id
        );
        notarization.cancel_escrow(&open_escrow).await?;
      }
      return Ok(());
    }

    tracing::debug!(
      "Trying to sync a client escrow that appears to have been updated by the recipient. escrow address={}, change number={}",
      escrow.from_address,
      escrow.balance_change_number
    );
    // will handle notarization
    let _ = self
      .sync_notarization(
        escrow.from_address.clone(),
        AccountType::Deposit,
        notary_id,
        tip.notebook_number,
        escrow.balance_change_number,
        tip.tick,
      )
      .await?;
    Ok(())
  }

  pub async fn sync_notarization(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: u32,
    notebook_number: u32,
    change_number: u32,
    tick: u32,
  ) -> anyhow::Result<i64> {
    let mut tx = self.db.begin().await?;
    let account = AccountStore::get(&mut tx, address, account_type, notary_id).await?;
    let notary_client = self.notary_clients.get(notary_id).await?;

    let notarization = notary_client
      .get_notarization(
        account.get_account_id32()?,
        account_type,
        notebook_number,
        change_number,
      )
      .await?;

    let json = json!(&notarization);

    let notarization_id = sqlx::query_scalar!(
      "INSERT INTO notarizations (json, notary_id, notebook_number, tick) VALUES (?, ?, ?, ?) RETURNING id",
        json,
        notary_id,
        notebook_number,
        tick,
      )
      .fetch_one(&mut *tx)
      .await?;

    let transaction_id = sqlx::query_scalar!("SELECT transaction_id FROM balance_changes WHERE notarization_id = ? AND transaction_id IS NOT NULL", notarization_id)
        .fetch_optional(&mut *tx)
        .await?
        .map(|a| a.unwrap());

    for balance_change in notarization.balance_changes.iter() {
      let _ = OpenEscrowsStore::record_notarized(&mut *tx, &balance_change, notarization_id).await;

      BalanceChangeStore::upsert_notarized(
        &mut tx,
        account.id,
        &balance_change,
        notary_id,
        notarization_id,
        transaction_id,
      )
      .await?;
    }

    tx.commit().await?;

    Ok(notarization_id)
  }

  pub async fn check_notary(
    db: &SqlitePool,
    balance_change: &mut BalanceChangeRow,
    notary_clients: &NotaryClients,
  ) -> anyhow::Result<()> {
    let mut tx = db.begin().await?;
    let mut account = AccountStore::get_by_id(&mut *tx, balance_change.account_id).await?;
    let notary_id = balance_change.notary_id as u32;
    let notary_client = notary_clients.get(notary_id).await?;
    if account.origin.is_none() {
      Self::sync_account_origin(&mut tx, &mut account, &notary_client).await?;
    }

    let mut needs_notarization_download = true;
    let mut notebook_number = None;
    let expected_tip = balance_change.get_balance_tip(&account)?;

    if let Some(notarization_id) = balance_change.notarization_id {
      let record = sqlx::query!(
        "SELECT notebook_number, json IS NOT NULL as json FROM notarizations WHERE id = ?",
        notarization_id
      )
      .fetch_one(&mut *tx)
      .await?;

      println!("record: {:?}", record);

      notebook_number = record.notebook_number.map(|a| a as u32);
      needs_notarization_download = record.json == 0;
    }

    if needs_notarization_download {
      let tip = notary_client
        .get_balance_tip(account.address.clone(), account.account_type)
        .await?;

      if tip.balance_tip.as_ref() != expected_tip.tip().as_slice() {
        return Ok(());
      }

      notebook_number = Some(tip.notebook_number);

      let notarization = notary_client
        .get_notarization(
          account.get_account_id32()?,
          account.account_type,
          tip.notebook_number,
          balance_change.change_number as u32,
        )
        .await?;
      tracing::debug!(
        "Downloaded notarization for balance change. id={}, notarization_id={:?}, change={}. In notebook #{}, tick {}.",
          balance_change.id,
          balance_change.notarization_id,
          balance_change.change_number,
          tip.notebook_number,
          tip.tick
      );

      let json = json!(notarization);

      let notarization_id = match balance_change.notarization_id {
        Some(id) => {
          sqlx::query!(
              "UPDATE notarizations SET json = ?, notebook_number = ?, tick = ? WHERE id = ?",
              json,
              tip.notebook_number,
              tip.tick,
              id
            )
              .execute(&mut *tx)
              .await?;
          id
        },
        None =>
          sqlx::query_scalar!(
              "INSERT INTO notarizations (json, notary_id, notebook_number, tick) VALUES (?, ?, ?, ?) RETURNING id",
                json,
                balance_change.notary_id,
                tip.notebook_number,
                tip.tick,
              )
              .fetch_one(&mut *tx)
              .await?
      };
      balance_change.notarization_id = Some(notarization_id);
    }

    balance_change.status = BalanceChangeStatus::NotebookPublished;
    sqlx::query!(
      "UPDATE balance_changes SET notarization_id = ?, status = ? WHERE id = ?",
      balance_change.notarization_id,
      BalanceChangeStatus::NotebookPublished as i64,
      balance_change.id
    )
    .execute(&mut *tx)
    .await?;

    if let Some(notebook_number) = notebook_number {
      let result = notary_client
        .get_balance_proof(notebook_number, expected_tip)
        .await?;

      BalanceChangeStore::save_notebook_proof(&mut tx, balance_change, &result).await?;
    }
    tx.commit().await?;

    Ok(())
  }

  pub async fn sync_account_origin(
    tx: &mut Transaction<'static, Sqlite>,
    account: &mut LocalAccount,
    notary_client: &NotaryClient,
  ) -> anyhow::Result<bool> {
    let Ok(result) = notary_client
      .get_account_origin(account.address.clone(), account.account_type)
      .await
    else {
      return Ok(false);
    };
    AccountStore::update_origin(
      &mut *tx,
      account.id,
      result.notebook_number,
      result.account_uid,
    )
    .await?;
    account.origin = Some(NotaryAccountOrigin {
      account_uid: result.account_uid,
      notary_id: account.notary_id,
      notebook_number: result.notebook_number,
    });
    Ok(true)
  }

  /// Asks the notary to supply their own proof
  pub async fn sync_notebook_proof(
    db: &SqlitePool,
    balance_change: &mut BalanceChangeRow,
    notary_clients: &NotaryClients,
  ) -> anyhow::Result<bool> {
    let mut tx = db.begin().await?;

    let notebook_number = sqlx::query_scalar!(
      "SELECT notebook_number FROM notarizations WHERE id = ?",
      balance_change.notarization_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let Some(notebook_number) = notebook_number else {
      return Ok(false);
    };
    let mut account = AccountStore::get_by_id(&mut tx, balance_change.account_id).await?;
    let notary_client = notary_clients.get(balance_change.notary_id as u32).await?;

    if account.origin.is_none() {
      Self::sync_account_origin(&mut tx, &mut account, &notary_client).await?;
    }

    let tip = balance_change.get_balance_tip(&account)?;

    let result = notary_client
      .get_balance_proof(notebook_number as u32, tip)
      .await?;

    BalanceChangeStore::save_notebook_proof(&mut tx, balance_change, &result).await?;
    tx.commit().await?;
    tracing::debug!(
      "Balance Change synched notebook proof; id={}. Notebook={}, tick={}",
      balance_change.id,
      notebook_number,
      result.tick
    );
    Ok(true)
  }

  pub async fn check_finalized(&self, balance_change: &mut BalanceChangeRow) -> anyhow::Result<()> {
    let mut tx = self.db.begin().await?;

    let mainchain_mutex = self.mainchain_client.lock().await;
    let Some(ref mainchain_client) = *mainchain_mutex else {
      tracing::warn!(
        "Cannot synchronize finalization of balance change; id={}. No mainchain client available.",
        balance_change.id,
      );
      return Ok(());
    };

    let latest_notebook = mainchain_client
      .get_latest_notebook(balance_change.notary_id as u32)
      .await?;

    let latest_finalized = mainchain_client.latest_finalized_number().await?;

    let notebook_number = sqlx::query_scalar!(
      "SELECT notebook_number FROM notarizations WHERE id = ?",
      balance_change.notarization_id
    )
    .fetch_one(&mut *tx)
    .await?;

    let Some(notebook_number) = notebook_number else {
      return Ok(());
    };
    let notebook_number = notebook_number as u32;
    let notary_id = balance_change.notary_id as u32;

    if latest_notebook.notebook_number < notebook_number {
      return Ok(());
    }

    let account_change_root = mainchain_client
      .get_account_changes_root(notary_id, notebook_number)
      .await?;

    let account = AccountStore::get_by_id(&mut tx, balance_change.account_id).await?;
    let change_root = H256::from_slice(&account_change_root.as_ref()[..]);
    BalanceChangeStore::save_finalized(
      &mut tx,
      balance_change,
      &account,
      change_root,
      latest_finalized,
    )
    .await?;
    tx.commit().await?;
    tracing::debug!(
      "Balance Change finalized and proof verified in mainchain; id={}. Block #{}",
      balance_change.id,
      latest_finalized
    );

    Ok(())
  }
}
