use crate::accounts::AccountStore;
use crate::balance_changes::{BalanceChangeRow, BalanceChangeStatus, BalanceChangeStore};
use crate::notarization_builder::NotarizationBuilder;
use crate::open_channels::OpenChannelsStore;
use crate::signer::Signer;
use crate::LocalAccount;
use crate::NotaryClients;
use crate::{to_js_error, TickerRef};
use crate::{Channel, MainchainClient};
use crate::{Localchain, OpenChannel};
use codec::Decode;
use napi::bindgen_prelude::*;
use serde_json::json;
use sp_core::sr25519::Signature;
use sp_core::H256;
use sp_runtime::MultiSignature;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use ulx_primitives::{AccountType, BlockVote};

#[napi]
pub struct BalanceSync {
  db: SqlitePool,
  ticker: TickerRef,
  mainchain_client: MainchainClient,
  notary_clients: NotaryClients,
  lock: Arc<Mutex<()>>,
  open_channels: OpenChannelsStore,
  tick_counter: Arc<Mutex<(u32, u32)>>,
}

#[napi(object)]
#[derive(Clone)]
pub struct ChannelCloseOptions {
  pub channel_tax_address: String,
  pub channel_claims_send_to_address: Option<String>,
  pub votes_address: String,
  /// What's the minimum amount of tax we should wait for before voting on blocks
  pub minimum_vote_amount: Option<i64>,
}

#[napi]
pub struct BalanceSyncResult {
  balance_changes: Vec<BalanceChangeRow>,
  channel_notarizations: Vec<NotarizationBuilder>,
}
#[napi]
impl BalanceSyncResult {
  #[napi(getter)]
  pub fn balance_changes(&self) -> Vec<BalanceChangeRow> {
    self.balance_changes.clone()
  }
  #[napi(getter)]
  pub fn channel_notarizations(&self) -> Vec<NotarizationBuilder> {
    self.channel_notarizations.clone()
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
      open_channels: localchain.open_channels(),
      tick_counter: Arc::new(Mutex::new((0, 0))),
    }
  }

  #[napi]
  pub async fn sync(
    &self,
    options: Option<ChannelCloseOptions>,
    signer: Option<&Signer>,
  ) -> Result<BalanceSyncResult> {
    let balance_changes = self.sync_unsettled_balances().await?;

    tracing::debug!(
      "Finished processing unsettled balances {}. Has options? {:?} {:?}",
      balance_changes.len(),
      options.is_some(),
      signer.is_some()
    );
    let channel_notarizations = match (options, signer) {
      (Some(options), Some(signer)) => self.process_pending_channels(options, signer).await?,
      _ => vec![],
    };

    Ok(BalanceSyncResult {
      balance_changes,
      channel_notarizations,
    })
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

    if change.status == BalanceChangeStatus::InNotebook {
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
  pub async fn process_pending_channels(
    &self,
    options: ChannelCloseOptions,
    signer: &Signer,
  ) -> Result<Vec<NotarizationBuilder>> {
    let _lock = self.lock.lock().await;
    let open_channels = self.open_channels.get_claimable().await?;
    tracing::debug!(
      "Processing pending channels. Found {} to check for claims.",
      open_channels.len(),
    );

    let mut builder_by_notary = HashMap::new();

    let mut notarizations = vec![];

    for open_channel in open_channels {
      let mut channel = open_channel.inner().await;

      let notary_id = channel.notary_id as u32;

      let notarization = builder_by_notary
        .entry(notary_id)
        .or_insert_with(|| NotarizationBuilder::new(self.db.clone(), self.notary_clients.clone()));

      if channel.is_client {
        if let Err(e) = self
          .sync_client_channel(notary_id, &open_channel, &mut channel, notarization)
          .await
        {
          tracing::warn!("Error syncing client channel (#{}): {:?}", channel.id(), e);
        }
      } else {
        if let Err(e) = self
          .sync_server_channel(&open_channel, &mut channel, &options, notarization, signer)
          .await
        {
          tracing::warn!("Error syncing server channel (#{}): {:?}", channel.id(), e);
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
        .finalize_channel_notarization(&mut notarization, signer, &options)
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
      if account.account_type == AccountType::Deposit {
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
    signer: &Signer,
    options: &ChannelCloseOptions,
  ) -> anyhow::Result<()> {
    let (total_available_tax, tax_accounts) = self.get_available_tax_by_account(notarization).await;

    let current_tick = self.ticker.current();
    let Some(best_block_for_vote) = self
      .mainchain_client
      .get_vote_block_hash(current_tick)
      .await?
    else {
      return Ok(());
    };

    if total_available_tax < options.minimum_vote_amount.unwrap_or_default() as u128
      || total_available_tax < best_block_for_vote.vote_minimum.get_u128().1
    {
      return Ok(());
    }

    let channels = notarization.channels().await;
    let Some((data_domain, data_domain_address)) = channels.into_iter().find_map(|c| {
      if c.is_client == true {
        return None;
      }
      if let Some(domain) = c.data_domain {
        Some((domain, c.to_address))
      } else {
        None
      }
    }) else {
      return Ok(());
    };

    for (_, (account, tax)) in tax_accounts {
      let balance_change = notarization.get_balance_change(&account).await?;
      balance_change.send_to_vote(BigInt::from(tax)).await?;
    }

    let mut tick_counter = self.tick_counter.lock().await;
    if (*tick_counter).0 == current_tick {
      (*tick_counter).1 += 1;
    } else {
      *tick_counter = (current_tick, 0);
    }
    let vote_address = options.votes_address.clone();
    let mut vote = BlockVote {
      account_id: AccountStore::parse_address(&vote_address)?,
      power: total_available_tax,
      data_domain: data_domain.try_into()?,
      data_domain_account: AccountStore::parse_address(&data_domain_address)?,
      signature: Signature([0; 64]).into(),
      index: (*tick_counter).1,
      block_hash: H256::from_slice(best_block_for_vote.block_hash.as_ref()),
    };
    let signature = signer
      .sign(
        vote_address,
        Uint8Array::from(vote.hash().as_bytes().to_vec()),
      )
      .await?;
    vote.signature = MultiSignature::decode(&mut signature.as_ref())?;
    notarization.add_vote(vote).await?;

    Ok(())
  }

  pub async fn finalize_channel_notarization(
    &self,
    notarization: &mut NotarizationBuilder,
    signer: &Signer,
    options: &ChannelCloseOptions,
  ) {
    if let Err(e) = self
      .convert_tax_to_votes(notarization, signer, options)
      .await
    {
      tracing::error!(
        "Error converting tax to votes: {:?}. Continuing with notarization",
        e
      );
    }

    if let Err(e) = notarization.sign(signer).await {
      tracing::error!("Could not sign a channel closing notarization -> {:?}", e);
      return;
    }

    for i in 0..3 {
      if i > 0 {
        tracing::debug!("Retrying notarization finalization. Attempt #{}", i);
      }

      match notarization.notarize().await {
        Ok(tracker) => {
          tracing::info!(
            "Finalized channel notarization. id={}, balance_changes={}, votes={}",
            tracker.notarization_id,
            tracker.notarized_balance_changes,
            tracker.notarized_votes
          );
          break;
        }
        Err(e) => {
          if e.reason.contains("Channel hold not ready for claim") {
            let delay = 2 + i ^ 5;
            tracing::debug!("Channel hold not ready for claim. Waiting {delay} seconds.");
            tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
            continue;
          }
          tracing::warn!("Error finalizing channel notarization: {:?}", e);
        }
      }
    }
  }

  pub async fn sync_server_channel(
    &self,
    open_channel: &OpenChannel,
    channel: &mut Channel,
    options: &ChannelCloseOptions,
    notarization: &mut NotarizationBuilder,
    signer: &Signer,
  ) -> anyhow::Result<()> {
    let current_tick = self.ticker.current();

    if channel.is_past_claim_period(current_tick) {
      tracing::warn!(
        "Channel expired and we missed claim window, marking unable to claim. id={}",
        channel.id()
      );
      let mut db = self.db.acquire().await?;
      channel.mark_unable_to_claim(&mut db).await?;
      return Ok(());
    }

    tracing::debug!(
      "Server channel {} ready for claim. channel address={}, change number={}",
      channel.id(),
      channel.from_address,
      channel.balance_change_number
    );
    if !notarization
      .can_add_channel(&open_channel, options.channel_tax_address.clone())
      .await
    {
      if let Some(address) = options.channel_claims_send_to_address.clone() {
        notarization
          .move_claims_to_address(
            address,
            AccountType::Deposit,
            options.channel_tax_address.clone(),
          )
          .await?;
      }
      self
        .finalize_channel_notarization(notarization, signer, options)
        .await;
      return Ok(());
    }
    notarization
      .claim_channel(&open_channel, options.channel_tax_address.clone())
      .await?;
    Ok(())
  }

  pub async fn sync_client_channel(
    &self,
    notary_id: u32,
    open_channel: &OpenChannel,
    channel: &mut Channel,
    notarization: &mut NotarizationBuilder,
  ) -> anyhow::Result<()> {
    let tip = self
      .notary_clients
      .get(notary_id)
      .await?
      .get_balance_tip(channel.from_address.clone(), AccountType::Deposit)
      .await?;

    let hold_notebook = channel.hold_notebook_number();
    // hasn't changed - aka, nothing synced
    if tip.notebook_number == hold_notebook {
      let current_tick = self.ticker.current();
      if channel.is_past_claim_period(current_tick) {
        tracing::info!(
          "A channel was not claimed by the recipient. We're taking it back. id={}",
          channel.id()
        );
        notarization.cancel_channel(&open_channel).await?;
      }
      return Ok(());
    }

    tracing::debug!(
      "Trying to sync a client channel that appears to have been updated by the recipient. channel address={}, change number={}",
      channel.from_address,
      channel.balance_change_number
    );
    // will handle notarization
    let _ = self
      .sync_notarization(
        channel.from_address.clone(),
        AccountType::Deposit,
        notary_id,
        tip.notebook_number,
        channel.balance_change_number,
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

    for balance_change in notarization.balance_changes.iter() {
      let _ = OpenChannelsStore::record_notarized(&mut *tx, &balance_change, notarization_id).await;

      BalanceChangeStore::upsert_notarized(
        &mut tx,
        account.id,
        &balance_change,
        notary_id,
        notarization_id,
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
    let account = AccountStore::get_by_id(&mut *tx, balance_change.account_id).await?;
    let notary_id = balance_change.notary_id as u32;
    let notary_client = notary_clients.get(notary_id).await?;

    let expected_tip = balance_change.get_balance_tip(&account)?;
    let tip = notary_client
      .get_balance_tip(account.address.clone(), account.account_type)
      .await?;

    if tip.balance_tip.as_ref() != expected_tip.tip().as_slice() {
      return Ok(());
    }

    let notarization = notary_client
      .get_notarization(
        account.get_account_id32()?,
        account.account_type,
        tip.notebook_number,
        balance_change.change_number as u32,
      )
      .await?;
    tracing::debug!(
      "Downloaded notarization for balance change. id={}, change={}. In notebook #{}, tick {}.",
      balance_change.id,
      balance_change.change_number,
      tip.notebook_number,
      tip.tick
    );

    let json = json!(notarization);
    let notarization_id = sqlx::query_scalar!(
          "INSERT INTO notarizations (json, notary_id, notebook_number, tick) VALUES (?, ?, ?, ?) RETURNING id",
            json,
            balance_change.notary_id,
            tip.notebook_number,
            tip.tick,
          )
          .fetch_one(&mut *tx)
          .await?;
    balance_change.notarization_id = Some(notarization_id);
    balance_change.status = BalanceChangeStatus::InNotebook;
    sqlx::query!(
      "UPDATE balance_changes SET notarization_id = ?, status = ? WHERE id = ?",
      notarization_id,
      BalanceChangeStatus::InNotebook as i64,
      balance_change.id
    )
    .execute(&mut *tx)
    .await?;

    let result = notary_client
      .get_balance_proof(tip.notebook_number, expected_tip)
      .await?;

    BalanceChangeStore::save_notebook_proof(&mut tx, balance_change, &result).await?;
    tx.commit().await?;

    Ok(())
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
    let account = AccountStore::get_by_id(&mut tx, balance_change.account_id).await?;

    let tip = balance_change.get_balance_tip(&account)?;

    let notary_client = notary_clients.get(balance_change.notary_id as u32).await?;
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
    let latest_notebook = self
      .mainchain_client
      .get_latest_notebook(balance_change.notary_id as u32)
      .await?;

    let latest_finalized = self.mainchain_client.latest_finalized_number().await?;

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

    let account_change_root = self
      .mainchain_client
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
