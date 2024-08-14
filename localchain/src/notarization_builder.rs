use anyhow::anyhow;
use argon_notary_audit::{verify_changeset_signatures, verify_notarization_allocation};
use argon_primitives::{
  AccountType, Balance, BalanceChange, BlockVote, DataDomain, Notarization, NotaryId, Note,
  NoteType, DATA_DOMAIN_LEASE_COST, MAX_BALANCE_CHANGES_PER_NOTARIZATION,
  MAX_BLOCK_VOTES_PER_NOTARIZATION, MAX_DOMAINS_PER_NOTARIZATION, TAX_PERCENT_BASE,
  TRANSFER_TAX_CAP,
};
use codec::Decode;
use serde_json::json;
use sp_core::crypto::AccountId32;
use sp_core::ConstU32;
use sp_runtime::traits::Verify;
use sp_runtime::{BoundedVec, MultiSignature};
use sqlx::SqlitePool;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::accounts::AccountStore;
use crate::accounts::LocalAccount;
use crate::argon_file::{ArgonFile, ArgonFileType};
use crate::balance_change_builder::BalanceChangeBuilder;
use crate::balance_changes::BalanceChangeStore;
use crate::data_domain::JsDataDomain;
use crate::keystore::Keystore;
use crate::notarization_tracker::NotarizationTracker;
use crate::notary_client::NotaryClients;
use crate::open_escrows::OpenEscrow;
use crate::transactions::LocalchainTransaction;
use crate::{bail, Error};
use crate::{DataDomainStore, Escrow, LocalchainTransfer, NotaryAccountOrigin};
use crate::{Result, TickerRef};

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone)]
pub struct NotarizationBuilder {
  imported_balance_changes: Arc<Mutex<Vec<BalanceChange>>>,
  balance_changes_by_account: Arc<Mutex<HashMap<i64, BalanceChangeBuilder>>>,
  votes: Arc<Mutex<BoundedVec<BlockVote, ConstU32<MAX_BLOCK_VOTES_PER_NOTARIZATION>>>>,
  data_domains:
    Arc<Mutex<BoundedVec<(DataDomain, AccountId32), ConstU32<MAX_DOMAINS_PER_NOTARIZATION>>>>,
  loaded_accounts: Arc<Mutex<BTreeMap<(String, AccountType), LocalAccount>>>,
  escrows: Arc<Mutex<Vec<OpenEscrow>>>,
  db: SqlitePool,
  is_verified: Arc<Mutex<bool>>,
  is_finalized: Arc<Mutex<bool>>,
  notary_clients: NotaryClients,
  notary_id: Arc<Mutex<Option<u32>>>,
  transaction: Arc<Mutex<Option<LocalchainTransaction>>>,
  keystore: Keystore,
  ticker: TickerRef,
}

impl NotarizationBuilder {
  pub(crate) fn new(
    db: SqlitePool,
    notary_clients: NotaryClients,
    keystore: Keystore,
    ticker: TickerRef,
  ) -> Self {
    NotarizationBuilder {
      notary_clients,
      db,
      imported_balance_changes: Default::default(),
      balance_changes_by_account: Default::default(),
      votes: Default::default(),
      data_domains: Default::default(),
      loaded_accounts: Default::default(),
      escrows: Default::default(),
      is_verified: Default::default(),
      is_finalized: Default::default(),
      notary_id: Arc::new(Mutex::new(Some(1))),
      transaction: Default::default(),
      keystore,
      ticker,
    }
  }

  pub async fn set_notary_id(&self, notary_id: NotaryId) {
    *(self.notary_id.lock().await) = Some(notary_id);
  }

  pub async fn get_notary_id(&self) -> Result<u32> {
    let notary_id = *(self.notary_id.lock().await);
    let Some(notary_id) = notary_id else {
      bail!("No notary id found. Please specify which notary to use.");
    };
    Ok(notary_id)
  }

  pub async fn set_transaction(&self, transaction: LocalchainTransaction) {
    *(self.transaction.lock().await) = Some(transaction);
  }

  pub async fn get_transaction(&self) -> Option<LocalchainTransaction> {
    let transaction = self.transaction.lock().await;
    transaction.clone()
  }

  pub async fn ensure_notary_id(&self, notary_id: NotaryId) -> Result<()> {
    let mut notary_id_lock = self.notary_id.lock().await;
    if (*notary_id_lock).is_none() {
      *notary_id_lock = Some(notary_id);
    } else if *notary_id_lock != Some(notary_id) {
      bail!("Account is not from the same notary as this notarization");
    }
    Ok(())
  }
  pub async fn ensure_balance_change_notary_id(
    &self,
    balance_changes: &Vec<BalanceChange>,
  ) -> Result<()> {
    for change in balance_changes {
      let balance_notary_id = change
        .previous_balance_proof
        .as_ref()
        .map(|x| x.notary_id)
        .ok_or(anyhow!(
          "No previous balance proof found in the requested balance changes",
        ))?;
      self.ensure_notary_id(balance_notary_id).await?;
    }
    Ok(())
  }

  pub async fn is_finalized(&self) -> bool {
    *(self.is_finalized.lock().await)
  }

  pub async fn unclaimed_tax(&self) -> Result<i128> {
    let notarization = self.to_notarization().await?;
    let mut balance = 0i128;
    for change in notarization.balance_changes {
      for note in change.notes {
        if change.account_type == AccountType::Tax {
          match note.note_type {
            NoteType::Claim => balance -= note.milligons as i128,
            NoteType::Send { .. } => balance += note.milligons as i128,
            _ => {}
          }
        } else if note.note_type == NoteType::Tax {
          balance += note.milligons as i128;
        }
      }
    }

    Ok(balance)
  }

  pub async fn escrows(&self) -> Vec<Escrow> {
    let escrows = self.escrows.lock().await;
    let mut result = vec![];
    for escrow in &*escrows {
      result.push(escrow.inner().await);
    }
    result
  }

  pub async fn add_escrow(&self, escrow: &OpenEscrow) {
    self.escrows.lock().await.push(escrow.clone());
  }

  pub async fn accounts(&self) -> Vec<LocalAccount> {
    let accounts = self.loaded_accounts.lock().await;
    (*accounts).values().cloned().collect::<Vec<_>>()
  }

  pub async fn balance_change_builders(&self) -> Vec<BalanceChangeBuilder> {
    let builders = self.balance_changes_by_account.lock().await;
    builders.values().cloned().collect::<Vec<_>>()
  }

  pub async fn unused_vote_funds(&self) -> Result<i128> {
    let notarization = self.to_notarization().await?;
    let mut balance = 0i128;
    for change in notarization.balance_changes {
      if change.account_type == AccountType::Tax {
        for note in change.notes {
          if note.note_type == NoteType::SendToVote {
            balance += note.milligons as i128;
          }
        }
      }
    }
    for vote in notarization.block_votes.iter() {
      balance -= vote.power as i128
    }

    Ok(balance)
  }

  pub async fn unused_domain_funds(&self) -> Result<i128> {
    let notarization = self.to_notarization().await?;
    let mut balance = 0i128;
    for change in notarization.balance_changes {
      if change.account_type == AccountType::Deposit {
        for note in change.notes {
          if note.note_type == NoteType::LeaseDomain {
            balance += note.milligons as i128;
          }
        }
      }
    }

    let domains = notarization.data_domains.len() as i128;
    balance -= domains * DATA_DOMAIN_LEASE_COST as i128;

    Ok(balance)
  }

  pub async fn unclaimed_deposits(&self) -> Result<i128> {
    let notarization = self.to_notarization().await?;
    let mut balance = 0i128;
    for change in notarization.balance_changes {
      if change.account_type != AccountType::Deposit {
        continue;
      }

      for note in change.notes {
        match note.note_type {
          NoteType::Claim { .. } | NoteType::EscrowClaim => balance -= note.milligons as i128,
          NoteType::Send { .. } | NoteType::EscrowSettle => balance += note.milligons as i128,
          _ => {}
        };
      }
    }

    Ok(balance)
  }

  pub async fn get_balance_change(&self, account: &LocalAccount) -> Result<BalanceChangeBuilder> {
    let balance_changes_by_account = self.balance_changes_by_account.lock().await;
    match (*balance_changes_by_account).get(&account.id) {
      Some(balance_change) => Ok(balance_change.clone()),
      None => bail!("Balance change for account {} not found", account.address),
    }
  }

  async fn register_new_account(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: NotaryId,
    hd_path: String,
  ) -> Result<BalanceChangeBuilder> {
    let mut db = self.db.acquire().await?;
    let account = AccountStore::db_insert(
      &mut db,
      address.clone(),
      account_type,
      notary_id,
      Some(hd_path),
    )
    .await?;
    self.ensure_notary_id(notary_id).await?;

    let imports = (*(self.imported_balance_changes.lock().await)).len();
    let mut balance_changes_by_account = self.balance_changes_by_account.lock().await;
    if balance_changes_by_account.len() + imports + 1
      > MAX_BALANCE_CHANGES_PER_NOTARIZATION as usize
    {
      bail!(
        "Max balance changes reached for this notarization. Move this change to a new notarization! ({} change(s) + {} import(s) + 1 > {} max)",
        balance_changes_by_account.len(),
        imports,
        MAX_BALANCE_CHANGES_PER_NOTARIZATION
      );
    }

    let mut accounts = self.loaded_accounts.lock().await;
    accounts.insert((address.clone(), account_type), account.clone());

    let builder = BalanceChangeBuilder::new_account(address, account.id, account_type)?;
    balance_changes_by_account.insert(account.id, builder.clone());
    Ok(builder)
  }

  pub async fn add_account(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: NotaryId,
  ) -> Result<BalanceChangeBuilder> {
    self.ensure_notary_id(notary_id).await?;
    let mut db = self.db.acquire().await?;
    let account = AccountStore::db_get(&mut db, address.clone(), account_type, notary_id).await?;

    self.load_account(&account).await
  }

  pub async fn add_account_by_id(&self, local_account_id: i64) -> Result<BalanceChangeBuilder> {
    let mut db = self.db.acquire().await?;
    let account = AccountStore::db_get_by_id(&mut db, local_account_id).await?;
    self.load_account(&account).await
  }

  pub async fn get_jump_account(&self, account_type: AccountType) -> Result<BalanceChangeBuilder> {
    let notary_id = self.get_notary_id().await?;
    let mut db = self.db.acquire().await?;

    if let Some(account) =
      AccountStore::db_find_idle_jump_account(&mut db, account_type, notary_id).await?
    {
      return self.load_account(&account).await;
    }

    let hd_path = AccountStore::db_get_next_jump_path(&mut db, account_type, notary_id).await?;
    let address = self.keystore.derive_account_id(hd_path.clone()).await?;
    let balance_change = self
      .register_new_account(address, account_type, notary_id, hd_path)
      .await?;
    Ok(balance_change)
  }

  pub async fn default_deposit_account(&self) -> Result<BalanceChangeBuilder> {
    let notary_id = self.get_notary_id().await?;
    let mut db = self.db.acquire().await?;
    let account = AccountStore::db_deposit_account(&mut db, Some(notary_id)).await?;
    self.load_account(&account).await
  }

  pub async fn default_tax_account(&self) -> Result<BalanceChangeBuilder> {
    let notary_id = self.get_notary_id().await?;
    let mut db = self.db.acquire().await?;
    let account = AccountStore::db_tax_account(&mut db, Some(notary_id)).await?;
    self.load_account(&account).await
  }

  pub async fn load_account(&self, account: &LocalAccount) -> Result<BalanceChangeBuilder> {
    self.ensure_notary_id(account.notary_id).await?;

    let mut balance_changes_by_account = self.balance_changes_by_account.lock().await;
    if let Some(balance_change) = balance_changes_by_account.get(&account.id) {
      return Ok(balance_change.clone());
    }

    let imports = self.imported_balance_changes.lock().await;
    let mut accounts = self.loaded_accounts.lock().await;
    if balance_changes_by_account.len() + imports.len() + 1
      > MAX_BALANCE_CHANGES_PER_NOTARIZATION as usize
    {
      bail!(
        "Max balance changes reached for this notarization. Move this change to a new notarization! ({} change(s) + {} import(s) + 1 > {} max)",
        balance_changes_by_account.len(),
        imports.len(),
        MAX_BALANCE_CHANGES_PER_NOTARIZATION
      );
    }
    accounts.insert(
      (account.address.clone(), account.account_type),
      account.clone(),
    );

    let mut db = self.db.acquire().await?;
    let (balance_change, status) =
      BalanceChangeStore::db_build_for_account(&mut db, account).await?;
    let is_new = balance_change.change_number == 1 && status.is_none();

    let builder = match is_new {
      true => BalanceChangeBuilder::new_account(
        account.address.clone(),
        account.id,
        account.account_type,
      )?,
      false => BalanceChangeBuilder::new(balance_change, account.id, status),
    };
    balance_changes_by_account.insert(account.id, builder.clone());
    Ok(builder)
  }

  pub async fn can_add_escrow(&self, escrow: &OpenEscrow) -> bool {
    let balance_changes_by_account = (*(self.balance_changes_by_account.lock().await)).len();
    let imports = (*(self.imported_balance_changes.lock().await)).len();
    let mut added_accounts_needed = 2;
    let escrow = escrow.inner().await;
    let accounts_by_id = self.loaded_accounts.lock().await;
    for (_, account) in accounts_by_id.iter() {
      if account.address == escrow.to_address {
        added_accounts_needed -= 1;
      }
    }
    balance_changes_by_account + added_accounts_needed + imports
      < MAX_BALANCE_CHANGES_PER_NOTARIZATION as usize
  }

  pub async fn cancel_escrow(&self, open_escrow: &OpenEscrow) -> Result<()> {
    let escrow = open_escrow.inner().await;
    (*self.escrows.lock().await).push(open_escrow.clone());
    let balance_change_tx = self
      .add_account(escrow.from_address, AccountType::Deposit, escrow.notary_id)
      .await?;

    let balance_lock = balance_change_tx.balance_change_lock();
    let mut balance_change = balance_lock.lock().await;
    balance_change.push_note(0, NoteType::EscrowSettle);

    Ok(())
  }

  pub async fn claim_escrow(&self, open_escrow: &OpenEscrow) -> Result<()> {
    let escrow = open_escrow.inner().await;
    {
      let mut notary_id = self.notary_id.lock().await;
      if let Some(notary_id) = *notary_id {
        if escrow.notary_id != notary_id {
          bail!(
            "Escrow is not using the same notary ({:?}) as this notarization ({:?})",
            escrow.notary_id,
            self.notary_id
          );
        }
      } else {
        *notary_id = Some(escrow.notary_id);
      }
    }

    (*self.escrows.lock().await).push(open_escrow.clone());

    let settle_balance_change = escrow.get_final().await?;
    (*self.imported_balance_changes.lock().await).push(settle_balance_change);

    let default_deposit_account = self.default_deposit_account().await?;
    if default_deposit_account.address != escrow.to_address {
      bail!("Escrow claim address doesn't match this localchain address",)
    }

    let claim_result = default_deposit_account
      .claim_escrow(escrow.settled_amount())
      .await?;

    self
      .default_tax_account()
      .await?
      .claim(claim_result.tax)
      .await?;

    Ok(())
  }

  pub async fn add_vote(&self, vote: BlockVote) -> Result<()> {
    let funds = self.unused_vote_funds().await?;
    if vote.power > funds as u128 {
      bail!(
        "Insufficient tax available for this vote (available: {}, vote power {}).",
        funds,
        vote.power
      )
    }

    if !vote.signature.verify(&vote.hash()[..], &vote.account_id) {
      bail!("Invalid vote signature!");
    }

    let mut votes = self.votes.lock().await;
    votes
      .try_push(vote)
      .map_err(|_| anyhow!("Cannot add any more votes to this notarization!"))?;
    Ok(())
  }

  pub async fn lease_data_domain(
    &self,
    data_domain: String,
    register_to_address: String,
  ) -> Result<()> {
    let lease = self
      .default_deposit_account()
      .await?
      .lease_data_domain()
      .await?;

    self.default_tax_account().await?.claim(lease).await?;

    let register_to_account = AccountStore::parse_address(&register_to_address)?;
    let domain = DataDomain::parse(data_domain)?;
    let mut data_domains = self.data_domains.lock().await;
    data_domains.try_push((domain, register_to_account)).map_err(|_| anyhow!(
      "Max domains reached for this notarization. Move this domain to a new notarization! ({} domains + 1 > {} max)",
      data_domains.len(),
      MAX_DOMAINS_PER_NOTARIZATION
    ))?;
    Ok(())
  }

  /// Calculates the transfer tax on the given amount
  pub fn get_transfer_tax_amount(&self, amount: Balance) -> Balance {
    Note::calculate_transfer_tax(amount)
  }

  /// Calculates the total needed to end up with the given balance
  pub fn get_total_for_after_tax_balance(&self, final_balance: Balance) -> Balance {
    let amount = final_balance;
    if amount < 1000 {
      let total_before_tax = (amount * 100) / (100 - TAX_PERCENT_BASE);

      let round = if total_before_tax % 100 == 0 { 0 } else { 1 };

      total_before_tax + round
    } else {
      amount + TRANSFER_TAX_CAP
    }
  }

  pub fn get_escrow_tax_amount(&self, amount: Balance) -> Balance {
    Note::calculate_escrow_tax(amount)
  }

  pub async fn claim_from_mainchain(
    &self,
    transfer: LocalchainTransfer,
  ) -> Result<BalanceChangeBuilder> {
    self.set_notary_id(transfer.notary_id).await;
    let default_deposit_account = self.default_deposit_account().await?;
    if default_deposit_account.address != transfer.address {
      bail!("Mainchain transfer address doesn't match this localchain address",)
    }
    default_deposit_account
      .claim_from_mainchain(transfer)
      .await?;
    Ok(default_deposit_account)
  }

  pub async fn claim_and_pay_tax(
    &self,
    milligons_plus_tax: Balance,
    deposit_account_id: Option<i64>,
    use_default_tax_account: bool,
  ) -> Result<BalanceChangeBuilder> {
    let claim_account = match deposit_account_id {
      Some(id) => self.add_account_by_id(id).await?,
      None => self.default_deposit_account().await?,
    };

    let tax_result = claim_account.claim(milligons_plus_tax).await?;

    let tax = tax_result.tax;

    if tax > 0 {
      match use_default_tax_account {
        true => self.default_tax_account().await?,
        false => self.get_jump_account(AccountType::Tax).await?,
      }
      .claim(tax)
      .await?;
    }
    Ok(claim_account)
  }

  pub async fn fund_jump_account(&self, milligons: Balance) -> Result<BalanceChangeBuilder> {
    let jump_account = self.get_jump_account(AccountType::Deposit).await?;

    self
      .default_deposit_account()
      .await?
      .send(milligons, Some(vec![jump_account.address.clone()]))
      .await?;

    self
      .claim_and_pay_tax(milligons, Some(jump_account.local_account_id), false)
      .await
  }

  pub async fn accept_argon_file_request(&self, argon_file_json: String) -> Result<()> {
    let argon_file = ArgonFile::from_json(&argon_file_json)?;
    let mut balance_changes: Vec<BalanceChange> = argon_file.request.ok_or(anyhow!(
      "No requested balance changes found in the argon file",
    ))?;

    let mut recipients = vec![];
    let mut requested_milligons: u128 = 0;
    let mut paid_tax = false;

    for change in balance_changes.iter() {
      if !change.verify_signature() {
        bail!("Claimed balance change has an invalid signature");
      }
      if let Some(balance_notary_id) = change.previous_balance_proof.as_ref().map(|x| x.notary_id) {
        self.ensure_notary_id(balance_notary_id).await?;
      }
      if change.account_type == AccountType::Tax {
        paid_tax = true;
        continue;
      }
      for note in &change.notes {
        match note.note_type {
          NoteType::Claim => {
            recipients.push(AccountStore::to_address(&change.account_id));
            requested_milligons += note.milligons;
          }
          NoteType::Tax => {
            continue;
          }
          _ => {
            bail!(
              "This api can only accept 'Claim' notes. The note type is {:?}",
              note.note_type
            );
          }
        }
      }
    }
    if !paid_tax {
      bail!("No tax payment found in the request");
    }

    self
      .default_deposit_account()
      .await?
      .send(requested_milligons, Some(recipients))
      .await?;

    let mut imports = self.imported_balance_changes.lock().await;
    imports.append(&mut balance_changes);

    Ok(())
  }

  pub async fn import_argon_file(&self, argon_file_json: String) -> Result<()> {
    let argon_file = ArgonFile::from_json(&argon_file_json)?;
    let mut balance_changes: Vec<BalanceChange> = argon_file.send.ok_or(anyhow!(
      "No balance changes to claim found in the argon file",
    ))?;

    self
      .ensure_balance_change_notary_id(&balance_changes)
      .await?;

    let tax_account = self.default_tax_account().await?;

    let deposit_account = self.default_deposit_account().await?;

    for (i, balance_change) in balance_changes.iter().enumerate() {
      if !balance_change.verify_signature() {
        bail!("Claimed balance change #{i} has an invalid signature");
      }
      for note in balance_change.notes.iter() {
        match &note.note_type {
          NoteType::Send { to } => {
            if let Some(to) = to {
              let claim_addresses = to.iter().map(AccountStore::to_address).collect::<Vec<_>>();
              if (balance_change.account_type == AccountType::Deposit
                && !claim_addresses.contains(&deposit_account.address))
                || (balance_change.account_type == AccountType::Tax
                  && !claim_addresses.contains(&tax_account.address))
              {
                bail!(
                    "Claimed balance change #{i} has an account restriction that doesn't match your localchain (restricted to: {:?}, your account: {:?})",
                    to.iter().map(AccountStore::to_address).collect::<Vec<_>>(),
                    deposit_account.address,
                  );
              }
            }

            let _ = self.claim_and_pay_tax(note.milligons, None, true).await?;
          }
          _ => bail!(
            "This api can only accept 'Send' notes. The note type is {:?}",
            note.note_type
          ),
        }
      }
    }

    let mut imports = self.imported_balance_changes.lock().await;
    imports.append(&mut balance_changes);

    Ok(())
  }

  /// Exports an argon file from this notarization builder with the intention that these will be sent to another
  /// user (who will import into their own localchain).
  pub async fn export_as_file(&self, file_type: ArgonFileType) -> Result<String> {
    self.sign().await?;
    let notarization = self.to_notarization().await?;

    verify_changeset_signatures(notarization.balance_changes.as_ref())?;

    let file = ArgonFile::from_notarization(&notarization, file_type);

    let mut tx = self.db.begin().await?;
    let Some(notary_id) = *(self.notary_id.lock().await) else {
      bail!("Can't determine which notary to use. Please specify which notary to use.",);
    };

    let transaction = self.get_transaction().await;
    let transaction_id = transaction.map(|a| a.id as i64);
    let balance_changes_by_account = self.balance_changes_by_account.lock().await;
    for (account_id, balance_change_tx) in balance_changes_by_account.clone() {
      let balance_change = balance_change_tx.inner().await;

      BalanceChangeStore::tx_save_sent(
        &mut tx,
        account_id,
        balance_change,
        notary_id,
        transaction_id,
      )
      .await?;
    }
    tx.commit().await?;
    *(self.is_finalized.lock().await) = true;
    file.to_json()
  }

  pub async fn to_json(&self) -> Result<String> {
    let notarization = self.to_notarization().await?;
    let json = serde_json::to_string(&notarization)?;
    Ok(json)
  }

  pub(crate) async fn has_items_to_notarize(&self) -> bool {
    let balance_changes_by_account = self.balance_changes_by_account.lock().await;

    let mut balance_changes = 0;
    for balance_change_tx in (*balance_changes_by_account).values() {
      let balance_change = balance_change_tx.inner().await;
      if balance_change.notes.len() > 0 {
        balance_changes += 1;
      }
    }
    balance_changes > 0
  }

  pub(crate) async fn to_notarization(&self) -> Result<Notarization> {
    let imports = self.imported_balance_changes.lock().await;
    let block_votes = self.votes.lock().await;
    let data_domains = self.data_domains.lock().await;
    let mut notarization = Notarization::new(
      imports.clone(),
      (*block_votes).to_vec(),
      (*data_domains)
        .iter()
        .map(|(d, a)| (d.hash(), a.clone()))
        .collect(),
    );

    let mut to_delete = vec![];
    let mut accounts_to_delete = vec![];
    {
      let balance_changes_by_account = self.balance_changes_by_account.lock().await;
      for (key, balance_change_tx) in &*balance_changes_by_account {
        let balance_change = balance_change_tx.inner().await;
        if balance_change.notes.len() == 0 {
          to_delete.push(*key);
          accounts_to_delete.push((
            balance_change_tx.address.clone(),
            balance_change.account_type,
          ))
        }
        notarization
          .balance_changes
          .try_push(balance_change.clone())
          .map_err(|_| Error::Generic("Cannot add any more balance changes!".to_string()))?;
      }
    }
    self
      .balance_changes_by_account
      .lock()
      .await
      .retain(|id, _| !to_delete.contains(id));
    self
      .loaded_accounts
      .lock()
      .await
      .retain(|id, _| !accounts_to_delete.contains(id));

    Ok(notarization)
  }

  pub async fn notarize_and_wait_for_notebook(&self) -> Result<NotarizationTracker> {
    self.sign().await?;
    let tracker = self.notarize().await?;
    tracker.wait_for_notebook().await?;
    tracker.get_notebook_proof().await?;
    Ok(tracker)
  }

  pub async fn notarize(&self) -> Result<NotarizationTracker> {
    if !*self.is_verified.lock().await {
      self.sign().await?;
      self.verify().await?;
    }
    let notarization = self.to_notarization().await?;
    if notarization.balance_changes.is_empty() {
      bail!("No balance changes found in this notarization",);
    }

    let Some(notary_id) = self.get_notary_id().await.ok() else {
      bail!("Can't determine which notary to use. Please specify which notary to use.",);
    };

    let notarizations_json = json!(&notarization);

    let notary_client = self.notary_clients.get(notary_id).await?;
    let notarized_balance_changes = notarization.balance_changes.len() as u32;
    let notarized_votes = notarization.block_votes.len() as u32;
    let result = notary_client.notarize(notarization.clone()).await?;

    let mut tx = self.db.begin().await?;
    let notarization_id = sqlx::query_scalar!(
      "INSERT INTO notarizations (json, notary_id, notebook_number, tick) VALUES (?, ?, ?, ?) RETURNING id",
      notarizations_json,
      notary_client.notary_id,
      result.notebook_number,
      result.tick,
    )
    .fetch_one(&mut *tx)
    .await
    ?;

    let escrows = self.escrows.lock().await;
    for escrow in (*escrows).iter() {
      let mut escrow_inner = escrow.inner().await;
      escrow_inner
        .db_mark_notarized(&mut tx, notarization_id)
        .await?;
    }

    let notary_id = notary_client.notary_id;
    let notebook_number = result.notebook_number;

    let mut tracker = NotarizationTracker {
      db: self.db.clone(),
      notary_clients: self.notary_clients.clone(),
      tick: result.tick,
      notebook_number,
      notary_id,
      notarization_id,
      notarization,
      imports: (*(self.imported_balance_changes.lock().await)).clone(),
      balance_changes_by_account: Default::default(),
      accounts_by_id: Default::default(),
      notarized_balance_changes,
      notarized_votes,
    };
    let mut tracker_balance_changes = tracker.balance_changes_by_account.lock().await;
    let mut loaded_accounts = self.loaded_accounts.lock().await;
    let transaction = self.get_transaction().await;

    let transaction_id = transaction.map(|a| a.id as i64);
    let balance_changes_by_account = self.balance_changes_by_account.lock().await;
    for (account_id, balance_change_tx) in balance_changes_by_account.clone() {
      let balance_change = balance_change_tx.inner().await;
      let new_account = result.new_account_origins.iter().find(|a| {
        a.account_type == balance_change.account_type && a.account_id == balance_change.account_id
      });
      let change_id = BalanceChangeStore::tx_upsert_notarized(
        &mut tx,
        account_id,
        &balance_change,
        notary_id,
        notarization_id,
        transaction_id,
      )
      .await?;

      let mut account = loaded_accounts
        .iter()
        .find_map(|(_, x)| {
          if x.id == account_id {
            return Some(x.clone());
          }
          None
        })
        .expect("should exist");

      if let Some(new_origin) = new_account {
        AccountStore::db_update_origin(
          &mut tx,
          account_id,
          notebook_number,
          new_origin.account_uid,
        )
        .await?;
        account.origin = Some(NotaryAccountOrigin {
          account_uid: new_origin.account_uid,
          notary_id,
          notebook_number,
        });
        loaded_accounts.insert(
          (account.address.clone(), account.account_type),
          account.clone(),
        );
      }

      (*tracker_balance_changes).insert(
        account_id,
        BalanceChangeStore::db_get_by_id(&mut tx, change_id).await?,
      );

      tracker.accounts_by_id.insert(account_id, account);
    }
    let data_domains = self.data_domains.lock().await;
    for (domain, account) in &*data_domains {
      DataDomainStore::db_insert(
        &mut tx,
        JsDataDomain {
          domain_name: domain.domain_name.clone().into(),
          top_level_domain: domain.top_level_domain,
        },
        AccountStore::to_address(account),
        notarization_id,
        result.tick,
      )
      .await?;
    }

    tx.commit().await?;
    *(self.is_finalized.lock().await) = true;

    Ok(tracker.clone())
  }

  pub async fn verify(&self) -> Result<()> {
    let mut is_verified = self.is_verified.lock().await;
    let notarization = self.to_notarization().await?;

    verify_notarization_allocation(
      &notarization.balance_changes,
      &notarization.block_votes,
      &notarization.data_domains,
      None,
      self.ticker.escrow_expiration_ticks(),
    )?;
    verify_changeset_signatures(&notarization.balance_changes)?;

    *is_verified = true;
    Ok(())
  }

  pub async fn sign(&self) -> Result<()> {
    if self.keystore.is_unlocked().await {
      let accounts = self.loaded_accounts.lock().await;
      for (_, account) in accounts.iter() {
        if let Some(hd_path) = &account.hd_path {
          // load derived
          self.keystore.derive_account_id(hd_path.clone()).await?;
        }
      }
    }

    let mut balance_changes_by_account = self.balance_changes_by_account.lock().await;
    for (_, balance_change_tx) in balance_changes_by_account.iter_mut() {
      if balance_change_tx.is_empty_signature().await {
        let balance_lock = balance_change_tx.balance_change_lock();
        let mut balance_change = balance_lock.lock().await;
        let bytes = balance_change.hash();
        let signature = self
          .keystore
          .sign(
            AccountStore::to_address(&balance_change.account_id),
            bytes.as_bytes().to_vec(),
          )
          .await?;

        let multi_signature = MultiSignature::decode(&mut signature.as_ref())?;

        balance_change.signature = multi_signature.into();

        if !balance_change.verify_signature() {
          bail!("Invalid signature for balance change {:?}", balance_change);
        }
      }
    }

    Ok(())
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use super::NotarizationBuilder;
  use crate::argon_file::ArgonFileType;
  use crate::balance_change_builder::BalanceChangeBuilder;
  use crate::error::NapiOk;
  use crate::mainchain_client::napi_ext::LocalchainTransfer;
  use crate::open_escrows::OpenEscrow;
  use crate::transactions::LocalchainTransaction;
  use crate::Escrow;
  use crate::LocalAccount;
  use crate::{notarization_tracker::NotarizationTracker, AccountStore};
  use argon_primitives::{AccountType, BlockVote};
  use codec::Decode;
  use napi::bindgen_prelude::BigInt;
  use sp_core::H256;
  use sp_runtime::MultiSignature;

  #[napi]
  impl NotarizationBuilder {
    #[napi(setter, js_name = "notaryId")]
    pub async fn set_notary_id_napi(&self, notary_id: u32) {
      self.set_notary_id(notary_id).await;
    }
    #[napi(getter, js_name = "notaryId")]
    pub async fn get_notary_id_napi(&self) -> napi::Result<u32> {
      self.get_notary_id().await.napi_ok()
    }
    #[napi(setter, js_name = "transaction")]
    pub async fn set_transaction_napi(&self, transaction: LocalchainTransaction) {
      self.set_transaction(transaction).await;
    }

    #[napi(getter, js_name = "transaction")]
    pub async fn get_transaction_napi(&self) -> Option<LocalchainTransaction> {
      self.get_transaction().await
    }

    #[napi(getter, js_name = "isFinalized")]
    pub async fn is_finalized_napi(&self) -> bool {
      self.is_finalized().await
    }

    #[napi(getter, js_name = "unclaimedTax")]
    pub async fn unclaimed_tax_napi(&self) -> napi::Result<BigInt> {
      self.unclaimed_tax().await.map(Into::into).napi_ok()
    }

    #[napi(getter, js_name = "escrows")]
    pub async fn escrows_napi(&self) -> Vec<Escrow> {
      self.escrows().await
    }

    #[napi(getter, js_name = "accounts")]
    pub async fn accounts_napi(&self) -> Vec<LocalAccount> {
      self.accounts().await
    }

    #[napi(getter, js_name = "balanceChangeBuilders")]
    pub async fn balance_change_builders_napi(&self) -> Vec<BalanceChangeBuilder> {
      self.balance_change_builders().await
    }

    #[napi(getter, js_name = "unusedVoteFunds")]
    pub async fn unused_vote_funds_napi(&self) -> napi::Result<BigInt> {
      self.unused_vote_funds().await.map(Into::into).napi_ok()
    }

    #[napi(getter, js_name = "unusedDomainFunds")]
    pub async fn unused_domain_funds_napi(&self) -> napi::Result<BigInt> {
      self.unused_domain_funds().await.map(Into::into).napi_ok()
    }

    #[napi(getter, js_name = "unclaimedDeposits")]
    pub async fn unclaimed_deposits_napi(&self) -> napi::Result<BigInt> {
      self.unclaimed_deposits().await.map(Into::into).napi_ok()
    }

    #[napi(js_name = "getBalanceChange")]
    pub async fn get_balance_change_napi(
      &self,
      account: &LocalAccount,
    ) -> napi::Result<BalanceChangeBuilder> {
      self.get_balance_change(account).await.napi_ok()
    }

    #[napi(js_name = "addAccount")]
    pub async fn add_account_napi(
      &self,
      address: String,
      account_type: AccountType,
      notary_id: u32,
    ) -> napi::Result<BalanceChangeBuilder> {
      self
        .add_account(address, account_type, notary_id)
        .await
        .napi_ok()
    }

    #[napi(js_name = "addAccountById")]
    pub async fn add_account_by_id_napi(
      &self,
      local_account_id: i64,
    ) -> napi::Result<BalanceChangeBuilder> {
      self.add_account_by_id(local_account_id).await.napi_ok()
    }

    #[napi(js_name = "getJumpAccount")]
    pub async fn get_jump_account_napi(
      &self,
      account_type: AccountType,
    ) -> napi::Result<BalanceChangeBuilder> {
      self.get_jump_account(account_type).await.napi_ok()
    }

    #[napi(js_name = "defaultDepositAccount")]
    pub async fn default_deposit_account_napi(&self) -> napi::Result<BalanceChangeBuilder> {
      self.default_deposit_account().await.napi_ok()
    }

    #[napi(js_name = "defaultTaxAccount")]
    pub async fn default_tax_account_napi(&self) -> napi::Result<BalanceChangeBuilder> {
      self.default_tax_account().await.napi_ok()
    }

    #[napi(js_name = "loadAccount")]
    pub async fn load_account_napi(
      &self,
      account: &LocalAccount,
    ) -> napi::Result<BalanceChangeBuilder> {
      self.load_account(account).await.napi_ok()
    }

    #[napi(js_name = "canAddEscrow")]
    pub async fn can_add_escrow_napi(&self, escrow: &OpenEscrow) -> bool {
      self.can_add_escrow(escrow).await
    }

    #[napi(js_name = "cancelEscrow")]
    pub async fn cancel_escrow_napi(&self, open_escrow: &OpenEscrow) -> napi::Result<()> {
      self.cancel_escrow(open_escrow).await.napi_ok()
    }

    #[napi(js_name = "claimEscrow")]
    pub async fn claim_escrow_napi(&self, open_escrow: &OpenEscrow) -> napi::Result<()> {
      self.claim_escrow(open_escrow).await.napi_ok()
    }

    #[napi(js_name = "addVote", ts_args_type = "vote: BlockVote")]
    pub async fn add_vote_napi(&self, vote: JsBlockVote) -> napi::Result<()> {
      let vote = vote.try_into().napi_ok()?;
      self.add_vote(vote).await.napi_ok()
    }

    #[napi(js_name = "leaseDataDomain")]
    pub async fn lease_data_domain_napi(
      &self,
      data_domain: String,
      register_to_address: String,
    ) -> napi::Result<()> {
      self
        .lease_data_domain(data_domain, register_to_address)
        .await
        .napi_ok()
    }

    /// Calculates the transfer tax on the given amount
    #[napi(js_name = "getTransferTaxAmount")]
    pub fn get_transfer_tax_amount_napi(&self, amount: BigInt) -> BigInt {
      self.get_transfer_tax_amount(amount.get_u128().1).into()
    }

    /// Calculates the total needed to end up with the given balance
    #[napi(js_name = "getTotalForAfterTaxBalance")]
    pub fn get_total_for_after_tax_balance_napi(&self, final_balance: BigInt) -> BigInt {
      self
        .get_total_for_after_tax_balance(final_balance.get_u128().1)
        .into()
    }

    #[napi(js_name = "getEscrowTaxAmount")]
    pub fn get_escrow_tax_amount_napi(&self, amount: BigInt) -> BigInt {
      self.get_escrow_tax_amount(amount.get_u128().1).into()
    }

    #[napi(js_name = "claimFromMainchain")]
    pub async fn claim_from_mainchain_napi(
      &self,
      transfer: LocalchainTransfer,
    ) -> napi::Result<BalanceChangeBuilder> {
      let transfer = super::LocalchainTransfer {
        address: transfer.address,
        amount: transfer.amount.get_u128().1,
        notary_id: transfer.notary_id,
        transfer_id: transfer.transfer_id,
        expiration_tick: transfer.expiration_tick,
      };
      self.claim_from_mainchain(transfer).await.napi_ok()
    }

    #[napi(js_name = "claimAndPayTax")]
    pub async fn claim_and_pay_tax_napi(
      &self,
      milligons_plus_tax: BigInt,
      deposit_account_id: Option<i64>,
      use_default_tax_account: bool,
    ) -> napi::Result<BalanceChangeBuilder> {
      self
        .claim_and_pay_tax(
          milligons_plus_tax.get_u128().1,
          deposit_account_id,
          use_default_tax_account,
        )
        .await
        .napi_ok()
    }

    #[napi(js_name = "fundJumpAccount")]
    pub async fn fund_jump_account_napi(
      &self,
      milligons: BigInt,
    ) -> napi::Result<BalanceChangeBuilder> {
      self
        .fund_jump_account(milligons.get_u128().1)
        .await
        .napi_ok()
    }

    #[napi(js_name = "acceptArgonFileRequest")]
    pub async fn accept_argon_file_request_napi(
      &self,
      argon_file_json: String,
    ) -> napi::Result<()> {
      self
        .accept_argon_file_request(argon_file_json)
        .await
        .napi_ok()
    }

    #[napi(js_name = "importArgonFile")]
    pub async fn import_argon_file_napi(&self, argon_file_json: String) -> napi::Result<()> {
      self.import_argon_file(argon_file_json).await.napi_ok()
    }

    /// Exports an argon file from this notarization builder with the intention that these will be sent to another
    /// user (who will import into their own localchain).
    #[napi(js_name = "exportAsFile")]
    pub async fn export_as_file_napi(&self, file_type: ArgonFileType) -> napi::Result<String> {
      self.export_as_file(file_type).await.napi_ok()
    }

    #[napi(js_name = "toJSON")]
    pub async fn to_json_napi(&self) -> napi::Result<String> {
      self.to_json().await.napi_ok()
    }

    #[napi(js_name = "notarizeAndWaitForNotebook")]
    pub async fn notarize_and_wait_for_notebook_napi(&self) -> napi::Result<NotarizationTracker> {
      self.notarize_and_wait_for_notebook().await.napi_ok()
    }

    #[napi(js_name = "notarize")]
    pub async fn notarize_napi(&self) -> napi::Result<NotarizationTracker> {
      self.notarize().await.napi_ok()
    }

    #[napi(js_name = "verify")]
    pub async fn verify_napi(&self) -> napi::Result<()> {
      self.verify().await.napi_ok()
    }

    #[napi(js_name = "sign")]
    pub async fn sign_napi(&self) -> napi::Result<()> {
      self.sign().await.napi_ok()
    }
  }

  #[napi(object, js_name = "BlockVote")]
  pub struct JsBlockVote {
    /// The creator of the seal
    pub address: String,
    /// The block hash being voted on. Must be in last 2 ticks.
    pub block_hash: Vec<u8>,
    /// A unique index per account for this notebook
    pub index: u32,
    /// The voting power of this vote, determined from the amount of tax
    pub power: BigInt,
    /// The data domain used to create this vote
    pub data_domain_hash: Vec<u8>,
    /// The data domain payment address used to create this vote
    pub data_domain_address: String,
    /// The mainchain address where rewards will be sent
    pub block_rewards_address: String,
    /// A signature of the vote by the account_id
    pub signature: Vec<u8>,
  }

  impl TryInto<BlockVote> for JsBlockVote {
    type Error = crate::Error;
    fn try_into(self) -> crate::Result<BlockVote> {
      let (_, power, _) = self.power.get_u128();
      Ok(BlockVote {
        account_id: AccountStore::parse_address(&self.address)?,
        block_hash: H256::from_slice(self.block_hash.as_slice()),
        index: self.index,
        power,
        data_domain_hash: H256::from_slice(self.data_domain_hash.as_slice()),
        data_domain_account: AccountStore::parse_address(&self.data_domain_address)?,
        block_rewards_account_id: AccountStore::parse_address(&self.block_rewards_address)?,
        signature: MultiSignature::decode(&mut self.signature.as_slice())?,
      })
    }
  }
}

#[cfg(test)]
mod test {
  use argon_primitives::{AccountOrigin, BalanceProof, ChainTransfer, MerkleProof, Note};
  use frame_support::assert_ok;
  use sp_core::bounded_vec;
  use sp_core::ed25519::Signature;
  use sp_keyring::Ed25519Keyring::Ferdie;
  use sp_keyring::Sr25519Keyring::Alice;
  use sp_keyring::Sr25519Keyring::Bob;
  use sqlx::SqlitePool;

  use crate::test_utils::{
    create_mock_notary, create_pool, get_balance_tip, mock_mainchain_transfer, mock_notary_clients,
  };
  use crate::AccountStore;
  use crate::CryptoScheme::{Ed25519, Sr25519};
  use crate::*;

  use super::*;

  #[sqlx::test]
  async fn test_transfer_from_mainchain(pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;

    let alice_signer = Keystore::new(pool.clone());
    let alice_address = alice_signer
      .import_suri(Alice.to_seed(), Sr25519, None)
      .await?;
    let ticker = Ticker::start(Duration::from_secs(1), 2);

    let alice_builder = NotarizationBuilder::new(
      pool,
      notary_clients.clone(),
      alice_signer,
      TickerRef::new(ticker),
    );
    let default_account = alice_builder.default_deposit_account().await?;
    assert_eq!(default_account.address, alice_address.clone());
    assert_eq!(default_account.change_number, 0);

    let _ = alice_builder
      .claim_from_mainchain(mock_mainchain_transfer(&alice_address, 10_000u128))
      .await?;
    alice_builder.sign().await?;

    let test_notarization = alice_builder.to_notarization().await?;
    assert_eq!(test_notarization.balance_changes.len(), 1);
    assert_eq!(test_notarization.balance_changes[0].notes.len(), 1);
    assert_eq!(
      test_notarization.balance_changes[0].notes[0].milligons,
      10_000
    );
    assert_eq!(test_notarization.balance_changes[0].balance, 10_000);

    assert!(test_notarization.balance_changes[0].verify_signature());

    let _ = alice_builder.notarize().await?;
    assert!(alice_builder.is_finalized().await);
    assert_eq!(alice_builder.unclaimed_deposits().await?, 0);

    Ok(())
  }

  #[sqlx::test]
  async fn test_exchange(bob_pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;
    let bob_address = AccountStore::to_address(&Bob.to_account_id());
    let ticker = Ticker::start(Duration::from_secs(1), 2);
    let ticker = TickerRef::new(ticker);

    let alice_pool = create_pool().await?;
    let alice_signer = Keystore::new(alice_pool.clone());
    let alice_address = alice_signer
      .import_suri(Alice.to_seed(), Sr25519, None)
      .await?;

    let mut alice_db = alice_pool.acquire().await?;

    let alice_id = {
      let alice_builder = NotarizationBuilder::new(
        alice_pool.clone(),
        notary_clients.clone(),
        alice_signer.clone(),
        ticker.clone(),
      );
      let alice_account = alice_builder
        .claim_from_mainchain(mock_mainchain_transfer(&alice_address, 10_000u128))
        .await?;

      let alice_notarization = alice_builder.notarize().await?;

      let balance_tip = get_balance_tip(alice_account.inner().await, 1, 1);
      let mut notebook_header = mock_notary.create_notebook_header(vec![balance_tip]).await;
      notebook_header
        .chain_transfers
        .try_push(ChainTransfer::ToLocalchain { transfer_id: 1 })
        .expect("should be able to push");

      alice_notarization.get_notebook_proof().await?;
      let latest = BalanceChangeStore::db_get_latest_for_account(
        &mut alice_db,
        alice_account.local_account_id,
      )
      .await?
      .unwrap();
      assert_eq!(latest.balance, "10000");
      assert_eq!(latest.status, BalanceChangeStatus::NotebookPublished);
      assert_ne!(latest.proof_json, None);
      let merkle_proof: MerkleProof = serde_json::from_str(&latest.proof_json.unwrap())?;
      assert_eq!(merkle_proof.number_of_leaves, 1);
      assert_eq!(merkle_proof.leaf_index, 0);
      alice_account.local_account_id
    };

    println!("Alice has mainchain funds with proof");

    // 2. Load up funds to send for alice
    let alice_balance_changes = {
      let notarization = NotarizationBuilder::new(
        alice_pool.clone(),
        notary_clients.clone(),
        alice_signer,
        ticker.clone(),
      );
      notarization
        .default_deposit_account()
        .await?
        .send(1000u128, Some(vec![bob_address.clone()]))
        .await?;
      notarization.export_as_file(ArgonFileType::Send).await?
    };
    println!("Alice exported a balance change");

    let bob_signer = Keystore::new(bob_pool.clone());
    bob_signer.import_suri(Bob.to_seed(), Sr25519, None).await?;
    let bob_builder = NotarizationBuilder::new(
      bob_pool.clone(),
      notary_clients.clone(),
      bob_signer,
      ticker.clone(),
    );
    println!("Bob importing the balance change");
    bob_builder.import_argon_file(alice_balance_changes).await?;
    println!("Bob claimed the balance change");

    let bob_notarization = bob_builder.notarize().await?;
    println!("Bob notarized the balance change");

    let mut bob_db = bob_pool.acquire().await?;
    let bob_account =
      AccountStore::db_get(&mut bob_db, bob_address.clone(), AccountType::Deposit, 1).await?;
    let bob_tax_account =
      AccountStore::db_get(&mut bob_db, bob_address.clone(), AccountType::Tax, 1).await?;
    assert_eq!(bob_notarization.accounts_by_id.len(), 2);
    assert!(bob_notarization
      .accounts_by_id
      .contains_key(&bob_account.id));
    assert_eq!(bob_notarization.notarized_balance_changes, 3);
    assert_eq!(bob_notarization.notarized_votes, 0);

    let alice_latest = BalanceChangeStore::db_get_latest_for_account(&mut alice_db, alice_id)
      .await?
      .unwrap();
    assert_eq!(alice_latest.balance, "9000");
    assert_eq!(
      alice_latest.status,
      BalanceChangeStatus::WaitingForSendClaim
    );
    assert_eq!(alice_latest.proof_json, None, "Has not bee notarized yet");

    assert_eq!(bob_account.origin.unwrap().notebook_number, 2);
    assert_eq!(bob_tax_account.origin.unwrap().notebook_number, 2);

    let bob_latest = BalanceChangeStore::db_get_latest_for_account(&mut bob_db, bob_account.id)
      .await?
      .unwrap();
    assert_eq!(bob_latest.balance, "800");
    assert_eq!(bob_latest.status, BalanceChangeStatus::Notarized);
    assert_eq!(bob_latest.proof_json, None);
    let bob_tax_latest =
      BalanceChangeStore::db_get_latest_for_account(&mut bob_db, bob_tax_account.id)
        .await?
        .unwrap();
    assert_eq!(bob_tax_latest.balance, "200");
    println!("Notebook 2 is closing");

    let header = mock_notary
      .create_notebook_header(bob_notarization.get_balance_tips().await?)
      .await;
    mock_notary
      .add_notarization(
        header.notebook_number,
        bob_notarization.notarization.clone(),
      )
      .await;
    println!("Bob is getting proof for notebook 2");
    bob_notarization.get_notebook_proof().await?;
    println!("Bob got proof for notebook 2");

    let bob_latest = BalanceChangeStore::db_get_latest_for_account(&mut bob_db, bob_account.id)
      .await?
      .unwrap();
    assert_eq!(bob_latest.status, BalanceChangeStatus::NotebookPublished);
    assert_ne!(bob_latest.proof_json, None);
    let bob_tax_latest =
      BalanceChangeStore::db_get_latest_for_account(&mut bob_db, bob_tax_account.id)
        .await?
        .unwrap();
    assert_eq!(
      bob_tax_latest.status,
      BalanceChangeStatus::NotebookPublished
    );
    assert_ne!(bob_tax_latest.proof_json, None);

    // Simulate alice sync
    {
      let pending_changes = BalanceChangeStore::db_find_unsettled(&mut alice_db).await?;
      assert_eq!(pending_changes.len(), 2);
      let waiting_for_send = pending_changes
        .iter()
        .find(|x| x.status == BalanceChangeStatus::WaitingForSendClaim)
        .unwrap();
      let mut waiting_for_send = waiting_for_send.clone();
      assert_eq!(
        waiting_for_send.status,
        BalanceChangeStatus::WaitingForSendClaim
      );

      BalanceSync::check_notary(&alice_pool, &mut waiting_for_send, &notary_clients).await?;
      assert_eq!(
        waiting_for_send.status,
        BalanceChangeStatus::NotebookPublished
      );
      assert_ne!(waiting_for_send.proof_json, None);
    }

    Ok(())
  }

  #[sqlx::test]
  async fn it_cannot_accept_funds_sent_to_another_address(pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;
    let ticker = Ticker::start(Duration::from_secs(1), 2);
    let ticker = TickerRef::new(ticker);

    let mut balance_change = BalanceChange {
      account_id: Bob.to_account_id(),
      account_type: AccountType::Deposit,
      balance: 10_000,
      previous_balance_proof: Some(BalanceProof {
        notary_id: 1,
        balance: 11_000,
        account_origin: AccountOrigin {
          account_uid: 1,
          notebook_number: 1,
        },
        notebook_number: 1,
        tick: 1,
        notebook_proof: None,
      }),
      notes: bounded_vec![Note {
        milligons: 1000,
        note_type: NoteType::Send {
          to: Some(bounded_vec![Ferdie.to_account_id()])
        }
      }],
      signature: Signature::from_raw([0u8; 64]).into(),
      change_number: 2,
      escrow_hold_note: None,
    };
    let balance_change = balance_change.sign(Bob.pair()).clone();
    let keystore = Keystore::new(pool.clone());
    let _ = keystore.import_suri(Alice.to_seed(), Ed25519, None).await?;
    let builder = NotarizationBuilder::new(
      pool.clone(),
      notary_clients.clone(),
      keystore.clone(),
      ticker.clone(),
    );
    let res = builder
      .import_argon_file(ArgonFile::create(vec![balance_change], ArgonFileType::Send).to_json()?)
      .await;
    let error_message = res.unwrap_err().to_string();
    println!("Error accepting funds {:?}", error_message);
    assert!(error_message.contains("account restriction"));
    Ok(())
  }

  #[sqlx::test]
  async fn it_informs_user_if_not_setup(pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;
    let ticker = Ticker::start(Duration::from_secs(1), 2);
    let ticker = TickerRef::new(ticker);

    let mut balance_change = BalanceChange {
      account_id: Bob.to_account_id(),
      account_type: AccountType::Deposit,
      balance: 10_000,
      previous_balance_proof: Some(BalanceProof {
        notary_id: 1,
        balance: 11_000,
        account_origin: AccountOrigin {
          account_uid: 1,
          notebook_number: 1,
        },
        notebook_number: 1,
        tick: 1,
        notebook_proof: None,
      }),
      notes: bounded_vec![Note {
        milligons: 1000,
        note_type: NoteType::Send {
          to: Some(bounded_vec![Ferdie.to_account_id()])
        }
      }],
      signature: Signature::from_raw([0u8; 64]).into(),
      change_number: 2,
      escrow_hold_note: None,
    };
    let balance_change = balance_change.sign(Bob.pair()).clone();
    let keystore = Keystore::new(pool.clone());
    let builder = NotarizationBuilder::new(
      pool.clone(),
      notary_clients.clone(),
      keystore.clone(),
      ticker.clone(),
    );
    let res = builder
      .import_argon_file(
        ArgonFile::create(vec![balance_change.clone()], ArgonFileType::Send).to_json()?,
      )
      .await;
    assert!(res.unwrap_err().to_string().contains("has not been setup"));

    Ok(())
  }
  #[sqlx::test]
  async fn it_can_read_json() -> anyhow::Result<()> {
    let balance_change = r#"{
  "balanceChanges": [
    {
      "accountId": "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL",
      "accountType": "deposit",
      "changeNumber": 1,
      "balance": 4000,
      "previousBalanceProof": null,
      "escrowHoldNote": null,
      "notes": [
        {
          "milligons": 5000,
          "noteType": {
            "action": "claimFromMainchain",
            "transferId": 1
          }
        },
        {
          "milligons": 1000,
          "noteType": {
            "action": "leaseDomain"
          }
        }
      ],
      "signature": "0x01804acb1551182297e77da0afa3250c1ec6a034279d5cdb853ee89be38d09b61ce4afc347f9f9aa77f738babb0b96ece810caae1a46a9d34f6e218d94fd092c8a"
    },
    {
      "accountId": "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL",
      "accountType": "tax",
      "changeNumber": 1,
      "balance": 1000,
      "previousBalanceProof": null,
      "escrowHoldNote": null,
      "notes": [
        {
          "milligons": 1000,
          "noteType": {
            "action": "claim"
          }
        }
      ],
      "signature": "0x017214cf11f8e3fdfe62625aaf7c7a5aab93ed9707cec5a6aa7b75e05a36b9f23290da323d9c5ba5be9db6836631d538e07550705f45c1c1e1e9103d572677ea8f"
    }
  ],
  "blockVotes": [],
  "dataDomains": [
    [
      "0x653a9ab2d0648508094d117cff1dcb474a2c2cda8f5b94882678e9c447458fc1",
      "5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL"
    ]
  ]
  }"#;
    let balance_change: Notarization = serde_json::from_str(balance_change)?;
    assert_ok!(verify_notarization_allocation(
      &balance_change.balance_changes,
      &balance_change.block_votes,
      &balance_change.data_domains,
      None,
      2,
    ));
    Ok(())
  }
}
