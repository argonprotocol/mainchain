use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use codec::Decode;
use napi::bindgen_prelude::*;
use serde_json::json;
use sp_core::crypto::AccountId32;
use sp_core::{ConstU32, H256};
use sp_runtime::{BoundedVec, MultiSignature};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use ulx_notary_audit::{verify_changeset_signatures, verify_notarization_allocation};
use ulx_primitives::{
  AccountType, BalanceChange, BlockVote, DataDomain, Notarization, NotaryId, NoteType,
  DATA_DOMAIN_LEASE_COST, MAX_BALANCE_CHANGES_PER_NOTARIZATION, MAX_BLOCK_VOTES_PER_NOTARIZATION,
  MAX_DOMAINS_PER_NOTARIZATION,
};

use crate::accounts::AccountStore;
use crate::accounts::LocalAccount;
use crate::balance_change_builder::BalanceChangeBuilder;
use crate::balance_changes::BalanceChangeStore;
use crate::data_domain::JsDataDomain;
use crate::notarization_tracker::NotarizationTracker;
use crate::notary_client::NotaryClients;
use crate::open_escrows::OpenEscrow;
use crate::signer::Signer;
use crate::{to_js_error, DataDomainStore, Escrow, LocalchainTransfer, NotaryAccountOrigin};

#[napi]
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
}

#[napi]
impl NotarizationBuilder {
  pub(crate) fn new(db: SqlitePool, notary_clients: NotaryClients) -> Self {
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
      notary_id: Default::default(),
    }
  }

  #[napi(setter, js_name = "notaryId")]
  pub async fn set_notary_id(&self, notary_id: u32) {
    *(self.notary_id.lock().await) = Some(notary_id);
  }

  #[napi(getter, js_name = "notaryId")]
  pub async fn get_notary_id(&self) -> Result<u32> {
    let notary_id = *(self.notary_id.lock().await);
    notary_id.ok_or(Error::from_reason(
      "No notary id found. Please specify which notary to use.",
    ))
  }

  pub async fn ensure_notary_id(&self, notary_id: u32) -> Result<()> {
    let mut notary_id_lock = self.notary_id.lock().await;
    if *notary_id_lock == None {
      *notary_id_lock = Some(notary_id);
    } else if *notary_id_lock != Some(notary_id) {
      return Err(Error::from_reason(format!(
        "Account is not from the same notary as this notarization"
      )));
    }
    Ok(())
  }

  #[napi(getter)]
  pub async fn is_finalized(&self) -> bool {
    *(self.is_finalized.lock().await)
  }

  #[napi(getter)]
  pub async fn unclaimed_tax(&self) -> Result<BigInt> {
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
        } else {
          if note.note_type == NoteType::Tax {
            balance += note.milligons as i128;
          }
        }
      }
    }

    Ok(BigInt::from(balance))
  }

  #[napi(getter)]
  pub async fn escrows(&self) -> Vec<Escrow> {
    let escrows = self.escrows.lock().await;
    let mut result = vec![];
    for escrow in &*escrows {
      result.push(escrow.inner().await);
    }
    result
  }

  #[napi(getter)]
  pub async fn accounts(&self) -> Vec<LocalAccount> {
    let accounts = self.loaded_accounts.lock().await;
    (*accounts).values().cloned().collect::<Vec<_>>()
  }

  #[napi(getter)]
  pub async fn balance_change_builders(&self) -> Vec<BalanceChangeBuilder> {
    let builders = self.balance_changes_by_account.lock().await;
    builders.values().cloned().collect::<Vec<_>>()
  }

  #[napi(getter)]
  pub async fn unused_vote_funds(&self) -> Result<BigInt> {
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

    Ok(BigInt::from(balance))
  }

  #[napi(getter)]
  pub async fn unused_domain_funds(&self) -> Result<BigInt> {
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

    Ok(BigInt::from(balance))
  }

  #[napi(getter)]
  pub async fn unclaimed_deposits(&self) -> Result<BigInt> {
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

    Ok(BigInt::from(balance))
  }

  #[napi]
  pub async fn get_balance_change(&self, account: &LocalAccount) -> Result<BalanceChangeBuilder> {
    let balance_changes_by_account = self.balance_changes_by_account.lock().await;
    match (*balance_changes_by_account).get(&account.id) {
      Some(balance_change) => Ok(balance_change.clone()),
      None => Err(Error::from_reason(format!(
        "Balance change for account {} not found",
        account.address
      ))),
    }
  }

  async fn register_new_account(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: NotaryId,
  ) -> Result<LocalAccount> {
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    let account =
      AccountStore::insert(&mut db, address.clone(), account_type.clone(), notary_id).await?;
    self.ensure_notary_id(notary_id).await?;

    let imports = (*(self.imported_balance_changes.lock().await)).len();
    let mut balance_changes_by_account = self.balance_changes_by_account.lock().await;
    if balance_changes_by_account.len() + imports + 1
      > MAX_BALANCE_CHANGES_PER_NOTARIZATION as usize
    {
      return Err(Error::from_reason(format!(
        "Max balance changes reached for this notarization. Move this change to a new notarization! ({} change(s) + {} import(s) + 1 > {} max)",
        balance_changes_by_account.len(),
        imports,
        MAX_BALANCE_CHANGES_PER_NOTARIZATION
      )));
    }

    let mut accounts = self.loaded_accounts.lock().await;
    accounts.insert((address.clone(), account_type.clone()), account.clone());

    balance_changes_by_account.insert(
      account.id,
      BalanceChangeBuilder::new_account(address, account_type)?,
    );
    Ok(account)
  }

  #[napi]
  pub async fn add_account(
    &self,
    address: String,
    account_type: AccountType,
    notary_id: u32,
  ) -> Result<LocalAccount> {
    self.ensure_notary_id(notary_id).await?;
    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    return match AccountStore::get(&mut db, address.clone(), account_type, notary_id).await {
      Ok(account) => {
        self.load_account(&account).await.map_err(to_js_error)?;
        Ok(account)
      }
      Err(_) => self
        .register_new_account(address.clone(), account_type, notary_id)
        .await
        .map_err(to_js_error),
    };
  }

  #[napi]
  pub async fn load_account(&self, account: &LocalAccount) -> Result<()> {
    self.ensure_notary_id(account.notary_id).await?;
    let mut balance_changes_by_account = self.balance_changes_by_account.lock().await;
    if balance_changes_by_account.contains_key(&account.id) {
      return Ok(());
    }
    let imports = self.imported_balance_changes.lock().await;
    let mut accounts = self.loaded_accounts.lock().await;
    if balance_changes_by_account.len() + imports.len() + 1
      > MAX_BALANCE_CHANGES_PER_NOTARIZATION as usize
    {
      return Err(Error::from_reason(format!(
        "Max balance changes reached for this notarization. Move this change to a new notarization! ({} change(s) + {} import(s) + 1 > {} max)",
        balance_changes_by_account.len(),
        imports.len(),
        MAX_BALANCE_CHANGES_PER_NOTARIZATION
      )));
    }
    accounts.insert(
      (account.address.clone(), account.account_type.clone()),
      account.clone(),
    );

    let mut db = self.db.acquire().await.map_err(to_js_error)?;
    let balance_change = BalanceChangeStore::build_for_account(&mut *db, &account)
      .await
      .map_err(to_js_error)?;
    balance_changes_by_account.insert(account.id, BalanceChangeBuilder::new(balance_change));
    Ok(())
  }

  #[napi]
  pub async fn can_add_escrow(&self, escrow: &OpenEscrow, tax_address: String) -> bool {
    let balance_changes_by_account = (*(self.balance_changes_by_account.lock().await)).len();
    let imports = (*(self.imported_balance_changes.lock().await)).len();
    let mut added_accounts_needed = 2;
    let escrow = escrow.inner().await;
    let accounts_by_id = self.loaded_accounts.lock().await;
    for (_, account) in accounts_by_id.iter() {
      if account.address == escrow.to_address || account.address == tax_address {
        added_accounts_needed -= 1;
      }
    }
    balance_changes_by_account + added_accounts_needed + imports + 1
      <= MAX_BALANCE_CHANGES_PER_NOTARIZATION as usize
  }

  #[napi]
  pub async fn cancel_escrow(&self, open_escrow: &OpenEscrow) -> Result<()> {
    let escrow = open_escrow.inner().await;
    (*self.escrows.lock().await).push(open_escrow.clone());
    let sender = self
      .add_account(escrow.from_address, AccountType::Deposit, escrow.notary_id)
      .await?;

    let balance_change_tx = self.get_balance_change(&sender).await?;
    let balance_lock = balance_change_tx.balance_change_lock();
    let mut balance_change = balance_lock.lock().await;
    balance_change.push_note(0, NoteType::EscrowSettle);

    Ok(())
  }

  #[napi]
  pub async fn claim_escrow(&self, open_escrow: &OpenEscrow, tax_address: String) -> Result<()> {
    let escrow = open_escrow.inner().await;
    {
      let mut notary_id = self.notary_id.lock().await;
      if let Some(notary_id) = *notary_id {
        if escrow.notary_id != notary_id {
          return Err(to_js_error(format!(
            "Escrow is not using the same notary ({:?}) as this notarization ({:?})",
            escrow.notary_id, self.notary_id
          )));
        }
      } else {
        *notary_id = Some(escrow.notary_id);
      }
    }
    let notary_id = escrow.notary_id;

    (*self.escrows.lock().await).push(open_escrow.clone());

    let settle_balance_change = escrow.get_final().await.map_err(to_js_error)?;
    (*self.imported_balance_changes.lock().await).push(settle_balance_change);

    let claim_account = self
      .add_account(escrow.to_address.clone(), AccountType::Deposit, notary_id)
      .await?;

    let claim_balance_change = self.get_balance_change(&claim_account).await?;
    let claim_result = claim_balance_change
      .claim_escrow(escrow.settled_amount())
      .await?;

    let tax_account = self
      .add_account(tax_address.clone(), AccountType::Tax, notary_id)
      .await?;

    let tax_balance_change = self.get_balance_change(&tax_account).await?;

    tax_balance_change.claim(claim_result.tax).await?;

    Ok(())
  }

  #[napi(js_name = "addVote", ts_args_type = "vote: BlockVote")]
  pub async fn add_vote_js(&self, vote: JsBlockVote) -> Result<()> {
    let vote: BlockVote = vote.try_into()?;
    self.add_vote(vote).await
  }

  pub async fn add_vote(&self, vote: BlockVote) -> Result<()> {
    let funds = self.unused_vote_funds().await?;
    let (_, funds, _) = funds.get_u128();
    if vote.power > funds {
      return Err(Error::from_reason(format!(
        "Insufficient tax available for this vote (available: {}, vote power {}).",
        funds, vote.power
      )));
    }

    let mut votes = self.votes.lock().await;
    votes
      .try_push(vote)
      .map_err(|_| Error::from_reason("Cannot add any more votes to this notarization!"))?;
    Ok(())
  }

  #[napi]
  pub async fn lease_data_domain(
    &self,
    use_funds_from_address: String,
    tax_address: String,
    data_domain: JsDataDomain,
    register_to_address: String,
  ) -> Result<()> {
    let notary_id = self.get_notary_id().await?;
    let from_account = self
      .add_account(
        use_funds_from_address.clone(),
        AccountType::Deposit,
        notary_id,
      )
      .await?;

    let balance_change = self.get_balance_change(&from_account).await?;
    let lease = balance_change.lease_data_domain().await?;

    let tax_account = self
      .add_account(tax_address.clone(), AccountType::Tax, notary_id)
      .await?;
    self
      .get_balance_change(&tax_account)
      .await?
      .claim(lease)
      .await?;

    let register_to_account = AccountStore::parse_address(&register_to_address)?;
    let domain = data_domain.into();
    let mut data_domains = self.data_domains.lock().await;
    data_domains.try_push((domain, register_to_account)).map_err(|_| Error::from_reason(format!(
      "Max domains reached for this notarization. Move this domain to a new notarization! ({} domains + 1 > {} max)",
      data_domains.len(),
      MAX_DOMAINS_PER_NOTARIZATION
    )))?;
    Ok(())
  }

  #[napi]
  pub async fn can_add_balance_change(&self, claim_address: String, tax_address: String) -> bool {
    let balance_changes = (*(self.balance_changes_by_account.lock().await)).len();
    let mut added_accounts_needed = 2;
    let accounts_by_id = self.loaded_accounts.lock().await;
    for (_, account) in accounts_by_id.iter() {
      if account.address == claim_address || account.address == tax_address {
        added_accounts_needed -= 1;
      }
    }
    let imports = self.imported_balance_changes.lock().await;
    balance_changes + added_accounts_needed + imports.len() + 1
      <= MAX_BALANCE_CHANGES_PER_NOTARIZATION as usize
  }

  #[napi]
  pub async fn move_to_sub_address(
    &self,
    from_address: String,
    to_sub_address: String,
    account_type: AccountType,
    amount: BigInt,
    tax_address: String,
  ) -> Result<()> {
    let notary_id = self.get_notary_id().await?;
    let from_account = self
      .add_account(from_address.clone(), account_type, notary_id)
      .await?;
    let to_account = self
      .add_account(to_sub_address.clone(), account_type, notary_id)
      .await?;

    let balance_change = self.get_balance_change(&from_account).await?;
    balance_change.send(amount.clone(), None).await?;

    let to_balance_change = self.get_balance_change(&to_account).await?;
    let result = to_balance_change.claim(amount).await?;

    let (_, tax, _) = result.tax.get_u128();

    if account_type == AccountType::Deposit && tax > 0 {
      let tax_account = self
        .add_account(tax_address.clone(), AccountType::Tax, notary_id)
        .await?;
      let tax_balance = self.get_balance_change(&tax_account).await?;
      tax_balance.claim(result.tax).await?;
    }

    Ok(())
  }

  #[napi]
  pub async fn move_claims_to_address(
    &self,
    address: String,
    account_type: AccountType,
    tax_address: String,
  ) -> Result<()> {
    let balance_changes_by_account = self.balance_changes_by_account.lock().await;
    let mut net_claim_amount = 0u128;
    for (_, balance_change_tx) in &*balance_changes_by_account {
      let balance_lock = balance_change_tx.balance_change_lock();
      let mut balance_change = balance_lock.lock().await;

      if balance_change.account_type == account_type {
        if balance_change.account_id == AccountStore::parse_address(&address)? {
          continue;
        }

        let claim_amount = balance_change.balance.saturating_sub(
          balance_change
            .previous_balance_proof
            .as_ref()
            .map(|x| x.balance)
            .unwrap_or_default(),
        );
        if claim_amount > 0 {
          net_claim_amount += claim_amount;
          (*balance_change).balance -= claim_amount;
          (*balance_change).push_note(claim_amount, NoteType::Send { to: None });
        }
      }
    }
    let notary_id = self.get_notary_id().await?;
    drop(balance_changes_by_account);
    let account = self.add_account(address, account_type, notary_id).await?;

    let balance_change = self.get_balance_change(&account).await?;
    let result = balance_change.claim(BigInt::from(net_claim_amount)).await?;

    let (_, tax, _) = result.tax.get_u128();

    if account_type == AccountType::Deposit && tax > 0 {
      let tax_account = self
        .add_account(tax_address.clone(), AccountType::Tax, notary_id)
        .await?;
      let tax_balance = self.get_balance_change(&tax_account).await?;
      tax_balance.claim(result.tax).await?;
    }

    Ok(())
  }

  #[napi]
  pub async fn claim_from_mainchain(
    &self,
    transfer: LocalchainTransfer,
  ) -> Result<BalanceChangeBuilder> {
    let claim_account = self
      .add_account(
        transfer.address.clone(),
        AccountType::Deposit,
        transfer.notary_id,
      )
      .await?;
    let balance_change = self.get_balance_change(&claim_account).await?;
    balance_change.claim_from_mainchain(transfer).await?;
    Ok(balance_change)
  }

  #[napi]
  pub async fn claim_received_balance(
    &self,
    balance_changes_json: String,
    claim_address: String,
    tax_address: String,
  ) -> Result<()> {
    let mut balance_changes: Vec<BalanceChange> =
      serde_json::from_slice(balance_changes_json.as_ref())?;
    let notary_id = {
      let mut notary_id = self.notary_id.lock().await;

      for change in balance_changes.iter() {
        let balance_notary_id = change
          .previous_balance_proof
          .as_ref()
          .map(|x| x.notary_id.clone());

        if *notary_id == None {
          *notary_id = balance_notary_id;
        } else if *notary_id != balance_notary_id {
          return Err(Error::from_reason(
            "All balance changes must use the same notary",
          ));
        }
      }
      notary_id.ok_or(Error::new(
        Status::GenericFailure,
        "No notary id found in balance changes",
      ))?
    };

    let mut amount = 0;

    let claim_account = self
      .add_account(claim_address.clone(), AccountType::Deposit, notary_id)
      .await?;

    let tax_account = self
      .add_account(tax_address.clone(), AccountType::Tax, notary_id)
      .await?;

    let claim_account_id32 = AccountStore::parse_address(&claim_account.address)?;
    for (i, balance_change) in balance_changes.iter().enumerate() {
      if !balance_change.verify_signature() {
        return Err(Error::from_reason(format!(
          "Claimed balance change #{i} has an invalid signature"
        )));
      }
      if balance_change.account_type != claim_account.account_type {
        return Err(Error::from_reason(
          format!(
            "Claimed balance change #{i} is not the same account type ({:?}) as the claim account ({:?})",
            balance_change.account_type,
            claim_account.account_type,
          ),
        ));
      }
      for note in balance_change.notes.iter() {
        match &note.note_type {
          &NoteType::Send { ref to } => {
            if let Some(to) = to {
              if !to.iter().any(|a| a == &claim_account_id32) {
                return Err(Error::new(
                  Status::GenericFailure,
                  format!(
                    "Claimed balance change #{i} has an account restriction that doesn't match the supplied claim account (restricted to: {:?}, claim_account: {:?})",
                    to.iter().map(|a| AccountStore::to_address(a)).collect::<Vec<_>>(),
                    claim_address,
                  ),
                ));
              }
            }
          }
          _ => Err(Error::new(
            Status::GenericFailure,
            format!(
              "This api can only accept 'Send' notes. The note type is {:?}",
              note.note_type
            ),
          ))?,
        }
        amount += note.milligons;
      }
    }

    let mut imports = self.imported_balance_changes.lock().await;
    imports.append(&mut balance_changes);

    let balance_changes_by_account = self.balance_changes_by_account.lock().await;
    let claim_result = balance_changes_by_account
      .get(&claim_account.id)
      .unwrap()
      .claim(BigInt::from(amount))
      .await?;

    balance_changes_by_account
      .get(&tax_account.id)
      .unwrap()
      .claim(claim_result.tax)
      .await?;

    Ok(())
  }

  /// Exports balance changes (only) from this notarization builder with the intention that these will be sent to another
  /// user (who will import them into their own localchain). The returned byffer is utf8 encoded json.
  #[napi]
  pub async fn export_for_send(&self) -> Result<String> {
    let notarization = self.to_notarization().await?;

    verify_changeset_signatures(&notarization.balance_changes.to_vec()).map_err(to_js_error)?;

    let json = serde_json::to_string(&notarization.balance_changes).map_err(to_js_error)?;

    let mut tx = self.db.begin().await.map_err(to_js_error)?;
    let Some(notary_id) = *(self.notary_id.lock().await) else {
      return Err(Error::from_reason(
        "Can't determine which notary to use. Please specify which notary to use.",
      ));
    };

    let balance_changes_by_account = self.balance_changes_by_account.lock().await;
    for (account_id, balance_change_tx) in balance_changes_by_account.clone() {
      let balance_change = balance_change_tx.inner().await;

      BalanceChangeStore::save_sent(&mut tx, account_id, balance_change, notary_id).await?;
    }
    tx.commit().await.map_err(to_js_error)?;
    *(self.is_finalized.lock().await) = true;
    Ok(json)
  }

  #[napi]
  pub async fn to_json(&self) -> Result<String> {
    let notarization = self.to_notarization().await?;
    let json = serde_json::to_string(&notarization).map_err(to_js_error)?;
    Ok(json)
  }

  pub(crate) async fn to_notarization(&self) -> Result<Notarization> {
    let balance_changes_by_account = self.balance_changes_by_account.lock().await;
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
    let mut notary_id = None;
    for (_, balance_change_tx) in &*balance_changes_by_account {
      let balance_change = balance_change_tx.inner().await;
      let change_notary_id = balance_change
        .previous_balance_proof
        .as_ref()
        .map(|x| x.notary_id.clone());
      if notary_id == None {
        notary_id = change_notary_id;
      } else if notary_id != change_notary_id {
        return Err(Error::from_reason(
          "All balance changes must use the same notary",
        ));
      }
      notarization
        .balance_changes
        .try_push(balance_change.clone())
        .map_err(|_| {
          Error::new(
            Status::GenericFailure,
            "Cannot add any more balance changes!",
          )
        })?;
    }

    Ok(notarization)
  }

  #[napi]
  pub async fn notarize_and_wait_for_notebook(
    &self,
    signer: &Signer,
  ) -> Result<NotarizationTracker> {
    self.sign(signer).await?;
    let tracker = self.notarize().await?;
    tracker.wait_for_notebook().await?;
    tracker.get_notebook_proof().await?;
    Ok(tracker)
  }

  #[napi]
  pub async fn notarize(&self) -> Result<NotarizationTracker> {
    let is_verified = self.is_verified.lock().await;
    if !*is_verified {
      drop(is_verified);
      self.verify().await?;
    }
    let notarization = self.to_notarization().await?;

    let Some(notary_id) = self.get_notary_id().await.ok() else {
      return Err(Error::from_reason(
        "Can't determine which notary to use. Please specify which notary to use.",
      ));
    };

    let notarizations_json = json!(&notarization);

    let notary_client = self.notary_clients.get(notary_id).await?;
    let notarized_balance_changes = notarization.balance_changes.len() as u32;
    let notarized_votes = notarization.block_votes.len() as u32;
    let result = notary_client
      .notarize(notarization.clone())
      .await
      .map_err(to_js_error)?;

    let mut tx = self.db.begin().await.map_err(to_js_error)?;
    let notarization_id = sqlx::query_scalar!(
      "INSERT INTO notarizations (json, notary_id, notebook_number, tick) VALUES (?, ?, ?, ?) RETURNING id",
      notarizations_json,
      notary_client.notary_id,
      result.notebook_number,
      result.tick,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(to_js_error)?;

    let escrows = self.escrows.lock().await;
    for escrow in (*escrows).iter() {
      let mut escrow_inner = escrow.inner().await;
      escrow_inner
        .mark_notarized(&mut *tx, notarization_id)
        .await
        .map_err(to_js_error)?;
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

    let balance_changes_by_account = self.balance_changes_by_account.lock().await;
    for (account_id, balance_change_tx) in balance_changes_by_account.clone() {
      let balance_change = balance_change_tx.inner().await;
      let new_account = result.new_account_origins.iter().find(|a| {
        a.account_type == balance_change.account_type && a.account_id == balance_change.account_id
      });
      let change_id = BalanceChangeStore::upsert_notarized(
        &mut tx,
        account_id,
        &balance_change,
        notary_id,
        notarization_id,
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
        AccountStore::update_origin(&mut tx, account_id, notebook_number, new_origin.account_uid)
          .await?;
        account.origin = Some(NotaryAccountOrigin {
          account_uid: new_origin.account_uid,
          notary_id,
          notebook_number,
        });
        loaded_accounts.insert(
          (account.address.clone(), account.account_type.clone()),
          account.clone(),
        );
      }

      (*tracker_balance_changes).insert(
        account_id,
        BalanceChangeStore::get_by_id(&mut tx, change_id).await?,
      );

      tracker.accounts_by_id.insert(account_id, account);
    }
    let data_domains = self.data_domains.lock().await;
    for (domain, account) in &*data_domains {
      DataDomainStore::insert(
        &mut *tx,
        JsDataDomain {
          domain_name: domain.domain_name.clone().into(),
          top_level_domain: domain.top_level_domain.clone(),
        },
        AccountStore::to_address(account),
        notarization_id,
        result.tick,
      )
      .await
      .map_err(to_js_error)?;
    }

    tx.commit().await.map_err(to_js_error)?;
    *(self.is_finalized.lock().await) = true;

    Ok(tracker.clone())
  }

  #[napi]
  pub async fn verify(&self) -> Result<()> {
    let mut is_verified = self.is_verified.lock().await;
    let notarization = self.to_notarization().await?;
    verify_notarization_allocation(
      &notarization.balance_changes,
      &notarization.block_votes,
      &notarization.data_domains,
      None,
    )
    .map_err(to_js_error)?;
    verify_changeset_signatures(&notarization.balance_changes).map_err(to_js_error)?;

    *is_verified = true;
    Ok(())
  }

  #[napi]
  pub async fn sign(&self, signer: &Signer) -> Result<()> {
    let mut balance_changes_by_account = self.balance_changes_by_account.lock().await;
    for (_, balance_change_tx) in balance_changes_by_account.iter_mut() {
      if balance_change_tx.is_empty_signature().await {
        let balance_lock = balance_change_tx.balance_change_lock();
        let mut balance_change = balance_lock.lock().await;
        let bytes = balance_change.hash();

        let signature = signer
          .sign(
            AccountStore::to_address(&balance_change.account_id),
            Uint8Array::from(bytes.as_bytes().to_vec()),
          )
          .await?;
        balance_change.signature =
          MultiSignature::decode(&mut signature.as_ref()).map_err(to_js_error)?;

        if !balance_change.verify_signature() {
          return Err(Error::from_reason(format!(
            "Invalid signature for balance change {:?}",
            balance_change
          )));
        }
      }
    }

    Ok(())
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
  /// A signature of the vote by the account_id
  pub signature: Vec<u8>,
}

impl TryInto<BlockVote> for JsBlockVote {
  type Error = anyhow::Error;
  fn try_into(self) -> anyhow::Result<BlockVote> {
    let (_, power, _) = self.power.get_u128();
    Ok(BlockVote {
      account_id: AccountStore::parse_address(&self.address)?,
      block_hash: H256::from_slice(self.block_hash.as_slice()),
      index: self.index,
      power,
      data_domain_hash: H256::from_slice(self.data_domain_hash.as_slice()),
      data_domain_account: AccountStore::parse_address(&self.data_domain_address)?,
    })
  }
}

#[cfg(test)]
mod test {
  use std::str::FromStr;

  use binary_merkle_tree::{merkle_proof, merkle_root};
  use codec::Encode;
  use sp_core::ed25519::Signature;
  use sp_core::sr25519::Pair as SrPair;
  use sp_core::{bounded_vec, Blake2Hasher, LogLevelFilter, Pair};
  use sp_keyring::AccountKeyring::Alice;
  use sp_keyring::Ed25519Keyring::Ferdie;
  use sp_keyring::Sr25519Keyring::Bob;
  use sqlx::sqlite::SqliteConnectOptions;
  use sqlx::{ConnectOptions, SqlitePool};

  use ulx_notary::apis::localchain::BalanceChangeResult;
  use ulx_primitives::{
    AccountOrigin, AccountOriginUid, BalanceProof, BalanceTip, ChainTransfer, MerkleProof,
    NewAccountOrigin, Note, NotebookHeader, NotebookNumber, SignedNotebookHeader,
  };

  use crate::test_utils::CryptoType::Sr25519;
  use crate::test_utils::{create_keystore, create_mock_notary, mock_notary_clients, MockNotary};
  use crate::AccountStore;
  use crate::*;

  use super::*;

  #[sqlx::test]
  async fn test_transfer_from_mainchain(pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;
    let alice_address = AccountStore::to_address(&Alice.to_account_id());

    let alice_keystore = create_keystore(&Alice.to_seed(), Sr25519)?;
    let alice_signer = Signer::with_keystore(alice_keystore);

    let alice_builder = NotarizationBuilder::new(pool, notary_clients.clone());
    let alice_account = alice_builder
      .register_new_account(
        alice_address.clone(),
        AccountType::Deposit,
        mock_notary.notary_id,
      )
      .await?;
    let balance = alice_builder.get_balance_change(&alice_account).await?;
    balance
      .claim_from_mainchain(mainchain_transfer(&alice_address, 10_000u128))
      .await?;

    alice_builder.sign(&alice_signer).await?;
    let test_notarization = alice_builder.to_notarization().await?;
    assert_eq!(test_notarization.balance_changes.len(), 1);
    assert_eq!(test_notarization.balance_changes[0].notes.len(), 1);
    assert_eq!(
      test_notarization.balance_changes[0].notes[0].milligons,
      10_000
    );
    assert_eq!(test_notarization.balance_changes[0].balance, 10_000);
    println!(
      "Signature after to_notarization is {:?}",
      test_notarization.balance_changes[0].signature
    );
    assert!(test_notarization.balance_changes[0].verify_signature());
    mock_notary
      .set_notarization_result(BalanceChangeResult {
        new_account_origins: vec![],
        notebook_number: 1,
        tick: 1,
      })
      .await;
    let _ = alice_builder.notarize().await?;
    assert!(alice_builder.is_finalized().await);
    assert_eq!(alice_builder.unclaimed_deposits().await?.get_u128().1, 0);

    Ok(())
  }

  #[sqlx::test]
  async fn test_exchange(bob_pool: SqlitePool) -> anyhow::Result<()> {
    let mut mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;
    let alice_account_id = Alice.to_account_id();
    let alice_address = AccountStore::to_address(&alice_account_id);
    let bob_address = AccountStore::to_address(&Bob.to_account_id());

    let alice_pool = create_pool().await?;
    let mut alice_db = alice_pool.acquire().await?;
    let alice_keystore = create_keystore(&Alice.to_seed(), Sr25519)?;
    let alice_signer = Signer::with_keystore(alice_keystore);

    let alice_id = {
      let alice_builder = NotarizationBuilder::new(alice_pool.clone(), notary_clients.clone());
      let alice_account = alice_builder
        .register_new_account(alice_address.clone(), AccountType::Deposit, 1)
        .await?;
      let balance = alice_builder.get_balance_change(&alice_account).await?;
      balance
        .claim_from_mainchain(mainchain_transfer(&alice_address, 10_000u128))
        .await?;
      alice_builder.sign(&alice_signer).await?;
      mock_notary
        .set_notarization_result(BalanceChangeResult {
          new_account_origins: vec![NewAccountOrigin {
            account_type: AccountType::Deposit,
            account_id: alice_account_id.clone(),
            account_uid: 1,
          }],
          notebook_number: 1,
          tick: 1,
        })
        .await;
      let alice_notarization = alice_builder.notarize().await?;

      let balance_tip = get_balance_tip(balance.inner().await, 1, 1);
      let mut notebook_header = create_notebook_header(1, vec![balance_tip], &mock_notary).await;
      notebook_header
        .chain_transfers
        .try_push(ChainTransfer::ToLocalchain {
          account_id: alice_account_id.clone(),
          account_nonce: 1,
        })
        .expect("should be able to push");

      mock_notary
        .add_notebook_header(SignedNotebookHeader {
          header: notebook_header,
          signature: Signature([0u8; 64]),
        })
        .await;

      alice_notarization.get_notebook_proof().await?;
      let latest = BalanceChangeStore::get_latest_for_account(&mut alice_db, alice_account.id)
        .await?
        .unwrap();
      assert_eq!(latest.balance, "10000");
      assert_eq!(latest.status, BalanceChangeStatus::InNotebook);
      assert_ne!(latest.proof_json, None);
      let merkle_proof: MerkleProof = serde_json::from_str(&latest.proof_json.unwrap())?;
      assert_eq!(merkle_proof.number_of_leaves, 1);
      assert_eq!(merkle_proof.leaf_index, 0);
      alice_account.id
    };

    println!("Alice has mainchain funds with proof");

    // 2. Load up funds to send for alice
    let alice_balance_changes = {
      let notarization = NotarizationBuilder::new(alice_pool.clone(), notary_clients.clone());
      let alice_account = notarization
        .add_account(alice_address.clone(), AccountType::Deposit, 1)
        .await?;
      let balance = notarization.get_balance_change(&alice_account).await?;
      balance
        .send(1000u128.into(), Some(vec![bob_address.clone()]))
        .await?;
      notarization.sign(&alice_signer).await?;
      notarization.export_for_send().await?
    };
    println!("Alice exported a balance change");

    let bob_keystore = create_keystore(&Bob.to_seed(), Sr25519)?;
    let bob_signer = Signer::with_keystore(bob_keystore);
    let bob_builder = NotarizationBuilder::new(bob_pool.clone(), notary_clients.clone());
    println!("Bob importing the balance change");
    bob_builder
      .claim_received_balance(
        alice_balance_changes,
        bob_address.clone(),
        bob_address.clone(),
      )
      .await?;
    println!("Bob claimed the balance change");
    bob_builder.sign(&bob_signer).await?;
    mock_notary
      .set_notarization_result(BalanceChangeResult {
        new_account_origins: vec![
          NewAccountOrigin {
            account_type: AccountType::Deposit,
            account_id: Bob.to_account_id(),
            account_uid: 2,
          },
          NewAccountOrigin {
            account_type: AccountType::Tax,
            account_id: Bob.to_account_id(),
            account_uid: 3,
          },
        ],
        notebook_number: 2,
        tick: 2,
      })
      .await;

    let bob_notarization = bob_builder.notarize().await?;
    println!("Bob notarized the balance change");

    let mut bob_db = bob_pool.acquire().await?;
    let bob_account =
      AccountStore::get(&mut bob_db, bob_address.clone(), AccountType::Deposit, 1).await?;
    let bob_tax_account =
      AccountStore::get(&mut bob_db, bob_address.clone(), AccountType::Tax, 1).await?;
    assert_eq!(bob_notarization.accounts_by_id.len(), 2);
    assert_eq!(
      bob_notarization
        .accounts_by_id
        .get(&bob_account.id)
        .unwrap()
        .origin,
      Some(NotaryAccountOrigin {
        notary_id: 1,
        notebook_number: 2,
        account_uid: 2,
      }),
    );
    assert_eq!(bob_notarization.notarized_balance_changes, 3);
    assert_eq!(bob_notarization.notarized_votes, 0);

    let alice_latest = BalanceChangeStore::get_latest_for_account(&mut alice_db, alice_id)
      .await?
      .unwrap();
    assert_eq!(alice_latest.balance, "9000");
    assert_eq!(
      alice_latest.status,
      BalanceChangeStatus::WaitingForSendClaim
    );
    assert_eq!(alice_latest.proof_json, None, "Has not bee notarized yet");

    assert_eq!(
      bob_account.origin,
      Some(NotaryAccountOrigin {
        notary_id: 1,
        notebook_number: 2,
        account_uid: 2,
      }),
    );
    assert_eq!(
      bob_tax_account.origin,
      Some(NotaryAccountOrigin {
        notary_id: 1,
        notebook_number: 2,
        account_uid: 3,
      }),
    );

    let bob_latest = BalanceChangeStore::get_latest_for_account(&mut bob_db, bob_account.id)
      .await?
      .unwrap();
    assert_eq!(bob_latest.balance, "800");
    assert_eq!(bob_latest.status, BalanceChangeStatus::SubmittedToNotary);
    assert_eq!(bob_latest.proof_json, None);
    let bob_tax_latest =
      BalanceChangeStore::get_latest_for_account(&mut bob_db, bob_tax_account.id)
        .await?
        .unwrap();
    assert_eq!(bob_tax_latest.balance, "200");
    println!("Notebook 2 is closing");

    let header =
      create_notebook_header(2, bob_notarization.get_balance_tips().await?, &mock_notary).await;
    mock_notary
      .add_notarization(
        header.notebook_number,
        bob_notarization.notarization.clone(),
      )
      .await;
    mock_notary
      .add_notebook_header(SignedNotebookHeader {
        header,
        signature: Signature([0u8; 64]),
      })
      .await;
    println!("Bob is getting proof for notebook 2");
    bob_notarization.get_notebook_proof().await?;
    println!("Bob got proof for notebook 2");

    let bob_latest = BalanceChangeStore::get_latest_for_account(&mut bob_db, bob_account.id)
      .await?
      .unwrap();
    assert_eq!(bob_latest.status, BalanceChangeStatus::InNotebook);
    assert_ne!(bob_latest.proof_json, None);
    let bob_tax_latest =
      BalanceChangeStore::get_latest_for_account(&mut bob_db, bob_tax_account.id)
        .await?
        .unwrap();
    assert_eq!(bob_tax_latest.status, BalanceChangeStatus::InNotebook);
    assert_ne!(bob_tax_latest.proof_json, None);

    // Simulate alice sync
    {
      let pending_changes = BalanceChangeStore::find_unsettled(&mut alice_db).await?;
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
      assert_eq!(waiting_for_send.status, BalanceChangeStatus::InNotebook);
      assert_ne!(waiting_for_send.proof_json, None);
    }

    Ok(())
  }

  #[sqlx::test]
  async fn it_cannot_accept_funds_sent_to_another_address(pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;
    let alice_account_id = Alice.to_account_id();
    let alice_address = AccountStore::to_address(&alice_account_id);

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
      signature: Signature([0u8; 64]).into(),
      change_number: 2,
      escrow_hold_note: None,
    };
    let balance_change = balance_change.sign(Bob.pair()).clone();
    let builder = NotarizationBuilder::new(pool.clone(), notary_clients.clone());
    let res = builder
      .claim_received_balance(
        serde_json::to_string(&vec![balance_change])?,
        alice_address.clone(),
        alice_address.clone(),
      )
      .await;
    assert!(res
      .unwrap_err()
      .reason
      .contains("account restriction that doesn't match the supplied claim account"));
    Ok(())
  }

  #[sqlx::test]
  async fn it_can_move_claims(pool: SqlitePool) -> anyhow::Result<()> {
    let mock_notary = create_mock_notary().await?;
    let notary_clients = mock_notary_clients(&mock_notary, Ferdie).await?;
    let alice_account_id = Alice.to_account_id();
    let alice_address = AccountStore::to_address(&alice_account_id);
    println!("deriving");
    let alice_1 = SrPair::from_string("//Alice//1", None).expect("can derive");
    println!("derived {:?}", alice_1.public());
    let alice_1_address = AccountStore::to_address(&AccountId32::from(alice_1.public()));

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
          to: Some(bounded_vec![Alice.to_account_id()])
        }
      }],
      signature: Signature([0u8; 64]).into(),
      change_number: 2,
      escrow_hold_note: None,
    };
    let balance_change = balance_change.sign(Bob.pair()).clone();
    let builder = NotarizationBuilder::new(pool.clone(), notary_clients.clone());
    let _ = builder
      .claim_received_balance(
        serde_json::to_string(&vec![balance_change])?,
        alice_address.clone(),
        alice_address.clone(),
      )
      .await?;
    assert_eq!(builder.unclaimed_deposits().await?.get_u128().1, 0);
    println!("Claimed funds");
    builder
      .move_claims_to_address(
        alice_1_address.clone(),
        AccountType::Deposit,
        alice_1_address.clone(),
      )
      .await?;
    println!("Moved funds");
    assert_eq!(builder.unclaimed_deposits().await?.get_u128().1, 0);
    let accounts = builder.accounts().await;
    assert_eq!(accounts.len(), 4);
    let alice1_account = accounts
      .iter()
      .find(|x| x.address == alice_1_address && x.account_type == AccountType::Deposit)
      .unwrap();
    let alice1_balance = builder.get_balance_change(alice1_account).await?;
    assert_eq!(alice1_balance.inner().await.balance, 640);

    let original_recipient = accounts
      .iter()
      .find(|x| x.address == alice_address && x.account_type == AccountType::Deposit)
      .unwrap();
    let original_recipient_balance = builder.get_balance_change(original_recipient).await?;
    assert_eq!(original_recipient_balance.inner().await.balance, 0);

    Ok(())
  }

  fn get_balance_tip(
    balance_change: BalanceChange,
    account_origin_uid: AccountOriginUid,
    account_origin_notebook_number: NotebookNumber,
  ) -> BalanceTip {
    BalanceTip {
      account_type: balance_change.account_type,
      account_id: balance_change.account_id,
      balance: balance_change.balance,
      escrow_hold_note: balance_change.escrow_hold_note.clone(),
      account_origin: AccountOrigin {
        account_uid: account_origin_uid,
        notebook_number: account_origin_notebook_number,
      },
      change_number: balance_change.change_number,
    }
  }

  async fn create_pool() -> anyhow::Result<SqlitePool> {
    let pool = SqlitePool::connect_with(
      SqliteConnectOptions::from_str(&":memory:")?
        .clone()
        .log_statements(LogLevelFilter::Debug.into()),
    )
    .await?;
    let _ = tracing_subscriber::FmtSubscriber::builder()
      .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
      .try_init();
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
  }

  async fn create_notebook_header(
    notebook_number: NotebookNumber,
    balance_tips: Vec<BalanceTip>,
    mock_notary: &MockNotary,
  ) -> NotebookHeader {
    let merkle_leafs = balance_tips.iter().map(|x| x.encode()).collect::<Vec<_>>();
    let changed_accounts_root = merkle_root::<Blake2Hasher, _>(&merkle_leafs.clone());
    let mut notary_state = mock_notary.state.lock().await;
    for (leaf_index, balance_tip) in balance_tips.iter().enumerate() {
      let proof = merkle_proof::<Blake2Hasher, _, _>(merkle_leafs.clone(), leaf_index);

      notary_state.balance_proofs.insert(
        (notebook_number, balance_tip.tip().into()),
        BalanceProof {
          notary_id: 1,
          balance: balance_tip.balance,
          account_origin: balance_tip.account_origin.clone(),
          notebook_number,
          tick: notebook_number,
          notebook_proof: Some(MerkleProof {
            proof: BoundedVec::truncate_from(proof.proof),
            number_of_leaves: proof.number_of_leaves as u32,
            leaf_index: proof.leaf_index as u32,
          }),
        },
      );

      notary_state.balance_tips.insert(
        (
          balance_tip.account_id.clone(),
          balance_tip.account_type.clone(),
        ),
        ulx_notary::apis::localchain::BalanceTipResult {
          tick: notebook_number,
          balance_tip: balance_tip.tip().into(),
          notebook_number,
        },
      );
    }

    let changed_account_origins = BoundedVec::truncate_from(
      balance_tips
        .iter()
        .map(|x| x.account_origin.clone())
        .collect::<Vec<_>>(),
    );

    NotebookHeader {
      version: 1,
      notary_id: 1,
      notebook_number,
      tick: 1,
      tax: 0,
      data_domains: Default::default(),
      finalized_block_number: 1,
      block_votes_count: 0,
      block_voting_power: 0,
      parent_secret: None,
      block_votes_root: H256([0u8; 32]),
      changed_account_origins,
      blocks_with_votes: Default::default(),
      secret_hash: H256::random(),
      chain_transfers: Default::default(),
      changed_accounts_root,
    }
  }
  fn mainchain_transfer(address: &str, amount: u128) -> LocalchainTransfer {
    LocalchainTransfer {
      amount: amount.into(),
      notary_id: 1,
      expiration_block: 1,
      address: address.to_string(),
      account_nonce: 1,
    }
  }
}
