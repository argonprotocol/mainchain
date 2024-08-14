use sqlx::{SqliteConnection, SqlitePool};

use argon_primitives::{AccountType, Balance, Note};

use crate::argon_file::ArgonFileType;
use crate::keystore::Keystore;
use crate::notarization_builder::NotarizationBuilder;
use crate::notarization_tracker::NotarizationTracker;
use crate::notary_client::NotaryClients;
use crate::Result;
use crate::{OpenEscrow, OpenEscrowsStore, TickerRef, ESCROW_MINIMUM_SETTLEMENT};

#[derive(Debug, PartialOrd, PartialEq)]
#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[cfg_attr(not(feature = "napi"), derive(Clone, Copy))]
pub enum TransactionType {
  Send = 0,
  Request = 1,
  OpenEscrow = 2,
  Consolidation = 3,
}

impl From<i64> for TransactionType {
  fn from(i: i64) -> Self {
    match i {
      0 => TransactionType::Send,
      1 => TransactionType::Request,
      2 => TransactionType::OpenEscrow,
      3 => TransactionType::Consolidation,
      _ => panic!("Unknown transaction type {}", i),
    }
  }
}

#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct LocalchainTransaction {
  pub id: u32,
  pub transaction_type: TransactionType,
}

#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct Transactions {
  db: SqlitePool,
  ticker: TickerRef,
  notary_clients: NotaryClients,
  keystore: Keystore,
}

impl Transactions {
  pub fn new(
    db: SqlitePool,
    ticker: TickerRef,
    notary_clients: &NotaryClients,
    keystore: &Keystore,
  ) -> Self {
    Self {
      db,
      ticker,
      notary_clients: notary_clients.clone(),
      keystore: keystore.clone(),
    }
  }

  pub async fn create_static(
    db: &mut SqliteConnection,
    transaction_type: TransactionType,
  ) -> Result<LocalchainTransaction> {
    let type_id = transaction_type as i64;
    let transaction_id = sqlx::query_scalar!(
      "INSERT INTO transactions (transaction_type) VALUES (?) RETURNING ID",
      type_id,
    )
    .fetch_one(db)
    .await?;

    Ok(LocalchainTransaction {
      id: transaction_id as u32,
      transaction_type,
    })
  }

  pub async fn create(&self, transaction_type: TransactionType) -> Result<LocalchainTransaction> {
    let mut db = self.db.acquire().await?;
    Self::create_static(&mut db, transaction_type).await
  }

  pub async fn request(&self, milligons: Balance) -> Result<String> {
    let transaction = self.create(TransactionType::Request).await?;

    let jump_notarization = self.new_notarization();
    jump_notarization.set_transaction(transaction).await;
    let milligons_plus_tax = jump_notarization.get_total_for_after_tax_balance(milligons);
    let jump_account = jump_notarization
      .get_jump_account(AccountType::Deposit)
      .await?;
    let _ = jump_notarization
      .claim_and_pay_tax(
        milligons_plus_tax,
        Some(jump_account.local_account_id),
        false,
      )
      .await?;
    let json_file = jump_notarization
      .export_as_file(ArgonFileType::Request)
      .await?;

    Ok(json_file)
  }

  pub async fn create_escrow(
    &self,
    escrow_milligons: Balance,
    recipient_address: String,
    data_domain: Option<String>,
    notary_id: Option<u32>,
    delegated_signer_address: Option<String>,
  ) -> Result<OpenEscrow> {
    let jump_notarization = self.new_notarization();
    if let Some(notary_id) = notary_id {
      jump_notarization.set_notary_id(notary_id).await;
    }
    let transaction = self.create(TransactionType::OpenEscrow).await?;
    jump_notarization.set_transaction(transaction.clone()).await;

    let escrow_milligons = escrow_milligons.max(ESCROW_MINIMUM_SETTLEMENT);

    let amount_plus_tax = jump_notarization.get_total_for_after_tax_balance(escrow_milligons);
    let jump_account = jump_notarization.fund_jump_account(amount_plus_tax).await?;
    let _ = jump_notarization.notarize().await?;

    let escrow_notarization = self.new_notarization();
    escrow_notarization.set_transaction(transaction).await;
    let balance_change = escrow_notarization
      .add_account_by_id(jump_account.local_account_id)
      .await?;

    if let Some(data_domain) = data_domain {
      balance_change
        .create_escrow_hold(
          escrow_milligons,
          data_domain,
          recipient_address,
          delegated_signer_address,
        )
        .await?;
    } else {
      balance_change
        .create_private_server_escrow_hold(
          escrow_milligons,
          recipient_address,
          delegated_signer_address,
        )
        .await?;
    }
    escrow_notarization.notarize().await?;

    let escrow = OpenEscrowsStore::new(
      self.db.clone(),
      self.ticker.clone(),
      &self.notary_clients,
      &self.keystore,
    );
    let open_escrow = escrow
      .open_client_escrow(jump_account.local_account_id)
      .await?;
    Ok(open_escrow)
  }

  pub async fn send(&self, milligons: u128, to: Option<Vec<String>>) -> Result<String> {
    let jump_notarization = self.new_notarization();
    let transaction = self.create(TransactionType::Send).await?;
    jump_notarization.set_transaction(transaction.clone()).await;
    let jump_account = jump_notarization.fund_jump_account(milligons).await?;
    let _ = jump_notarization.notarize().await?;

    let amount = milligons;
    let tax = Note::calculate_transfer_tax(amount);

    let fund_notarization = self.new_notarization();
    fund_notarization.set_transaction(transaction).await;
    fund_notarization
      .add_account_by_id(jump_account.local_account_id)
      .await?
      .send(amount - tax, to)
      .await?;

    let json = fund_notarization
      .export_as_file(ArgonFileType::Send)
      .await?;

    Ok(json)
  }

  pub async fn import_argons(&self, argon_file: String) -> Result<NotarizationTracker> {
    let notarization = self.new_notarization();
    notarization.import_argon_file(argon_file).await?;
    notarization.notarize().await
  }

  pub async fn accept_argon_request(&self, argon_file: String) -> Result<NotarizationTracker> {
    let notarization = self.new_notarization();
    notarization.accept_argon_file_request(argon_file).await?;
    notarization.notarize().await
  }

  fn new_notarization(&self) -> NotarizationBuilder {
    NotarizationBuilder::new(
      self.db.clone(),
      self.notary_clients.clone(),
      self.keystore.clone(),
      self.ticker.clone(),
    )
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use crate::error::NapiOk;
  use napi::bindgen_prelude::BigInt;

  use super::Transactions;
  use super::{LocalchainTransaction, TransactionType};
  use crate::notarization_tracker::NotarizationTracker;
  use crate::open_escrows::OpenEscrow;

  #[napi]
  impl Transactions {
    #[napi(js_name = "create")]
    pub async fn create_napi(
      &self,
      transaction_type: TransactionType,
    ) -> napi::Result<LocalchainTransaction> {
      self.create(transaction_type).await.napi_ok()
    }

    #[napi(js_name = "request")]
    pub async fn request_napi(&self, milligons: BigInt) -> napi::Result<String> {
      self.request(milligons.get_u128().1).await.napi_ok()
    }

    #[napi(js_name = "createEscrow")]
    pub async fn create_escrow_napi(
      &self,
      escrow_milligons: BigInt,
      recipient_address: String,
      data_domain: Option<String>,
      notary_id: Option<u32>,
      delegated_signer_address: Option<String>,
    ) -> napi::Result<OpenEscrow> {
      self
        .create_escrow(
          escrow_milligons.get_u128().1,
          recipient_address,
          data_domain,
          notary_id,
          delegated_signer_address,
        )
        .await
        .napi_ok()
    }

    #[napi(js_name = "send")]
    pub async fn send_napi(
      &self,
      milligons: BigInt,
      to: Option<Vec<String>>,
    ) -> napi::Result<String> {
      self.send(milligons.get_u128().1, to).await.napi_ok()
    }

    #[napi(js_name = "importArgons")]
    pub async fn import_argons_napi(
      &self,
      argon_file: String,
    ) -> napi::Result<NotarizationTracker> {
      self.import_argons(argon_file).await.napi_ok()
    }

    #[napi(js_name = "acceptArgonRequest")]
    pub async fn accept_argon_request_napi(
      &self,
      argon_file: String,
    ) -> napi::Result<NotarizationTracker> {
      self.accept_argon_request(argon_file).await.napi_ok()
    }
  }
}

#[cfg(feature = "uniffi")]
pub mod uniffi_ext {
  use crate::error::UniffiResult;
  use anyhow::anyhow;

  use super::Transactions;
  use super::{LocalchainTransaction, TransactionType};
  use crate::notarization_tracker::uniffi_ext::NotarizationTracker;

  #[uniffi::export(async_runtime = "tokio")]
  impl Transactions {
    #[uniffi::method(name = "create")]
    pub async fn create_uniffi(
      &self,
      transaction_type: TransactionType,
    ) -> UniffiResult<LocalchainTransaction> {
      Ok(self.create(transaction_type).await?)
    }

    #[uniffi::method(name = "request")]
    pub async fn request_uniffi(&self, milligons: String) -> UniffiResult<String> {
      let milligons = milligons
        .parse::<u128>()
        .map_err(|e| anyhow!("Could not parse the milligon value -> {:?}", e))?;
      Ok(self.request(milligons).await?)
    }

    #[uniffi::method(name = "send")]
    pub async fn send_uniffi(
      &self,
      milligons: String,
      to: Option<Vec<String>>,
    ) -> UniffiResult<String> {
      let milligons = milligons
        .parse::<u128>()
        .map_err(|e| anyhow!("Could not parse the milligon value -> {:?}", e))?;

      Ok(self.send(milligons, to).await?)
    }

    #[uniffi::method(name = "importArgons")]
    pub async fn import_argons_uniffi(
      &self,
      argon_file: String,
    ) -> UniffiResult<NotarizationTracker> {
      Ok(self.import_argons(argon_file).await?.into())
    }

    #[uniffi::method(name = "acceptArgonRequest")]
    pub async fn accept_argon_request_uniffi(
      &self,
      argon_file: String,
    ) -> UniffiResult<NotarizationTracker> {
      Ok(self.accept_argon_request(argon_file).await?.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use sp_keyring::AccountKeyring::{Alice, Bob};
  use sp_keyring::Ed25519Keyring;
  use sp_keyring::Ed25519Keyring::Ferdie;

  use argon_primitives::AccountType;

  use crate::test_utils::{create_mock_notary, create_pool, mock_localchain, mock_notary_clients};
  use crate::CryptoScheme::{Ed25519, Sr25519};
  use crate::*;

  #[sqlx::test]
  async fn test_send_transaction(bob_pool: SqlitePool) -> anyhow::Result<()> {
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

    let alice_json = alice_localchain
      .transactions()
      .send(3500_u128, Some(vec![bob_localchain.address().await?]))
      .await?;

    let bob_builder = bob_localchain.begin_change();
    bob_builder.import_argon_file(alice_json).await?;
    let _ = bob_builder.notarize().await?;

    let alice_accounts = alice_localchain.accounts().list(Some(true)).await?;
    assert_eq!(alice_accounts.len(), 4);

    let bob_accounts = bob_localchain.accounts().list(Some(true)).await?;
    assert_eq!(bob_accounts.len(), 2);

    let mut tips = vec![];
    for account in alice_accounts.clone() {
      if let Some(latest) = alice_localchain
        .balance_changes()
        .get_latest_for_account(account.id)
        .await?
      {
        println!(
          "Latest Alice for {:?} {:?} {:#?}",
          account.hd_path, account.account_type, latest
        );
        if account.account_type == AccountType::Tax {
          if account.hd_path.is_some() {
            assert_eq!(latest.balance, "200");
            assert!(latest.transaction_id.is_some());
            assert_eq!(latest.change_number, 1);
          }
        } else if account.hd_path.is_some() {
          assert_eq!(latest.balance, "0");
          assert_eq!(latest.status, BalanceChangeStatus::WaitingForSendClaim);
          assert!(latest.transaction_id.is_some());
          assert_eq!(latest.change_number, 2);
        } else {
          assert_eq!(latest.balance, "1500");
          assert_eq!(latest.status, BalanceChangeStatus::Notarized);
          assert!(latest.transaction_id.is_some());
          assert_eq!(latest.change_number, 2);
        }
        tips.push(latest.get_balance_tip(&account)?);
      }
    }

    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account(AccountType::Deposit, 1)
      .await?
      .is_none());
    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account(AccountType::Tax, 1)
      .await?
      .is_none());

    for account in bob_accounts.clone() {
      let latest = bob_localchain
        .balance_changes()
        .get_latest_for_account(account.id)
        .await?
        .expect("Bob accounts should have balance");
      if account.account_type == AccountType::Tax {
        assert_eq!(latest.balance, "200");
      } else {
        assert_eq!(latest.balance, "3100");
      }
      tips.push(latest.get_balance_tip(&account)?);
    }
    let _ = mock_notary.create_notebook_header(tips).await;

    alice_localchain.balance_sync().sync(None).await?;
    bob_localchain.balance_sync().sync(None).await?;
    for account in alice_accounts {
      if let Some(latest) = alice_localchain
        .balance_changes()
        .get_latest_for_account(account.id)
        .await?
      {
        println!(
          "Latest Alice after sync for {:?} {:?} {:#?}",
          account.hd_path, account.account_type, latest
        );
        if account.account_type == AccountType::Tax {
          if account.hd_path.is_some() {
            // should get moved to tax
            assert_eq!(latest.balance, "0");
            assert_eq!(latest.status, BalanceChangeStatus::Notarized);
            assert_eq!(latest.change_number, 2);
          } else {
            assert_eq!(latest.balance, "200");
            assert_eq!(latest.status, BalanceChangeStatus::Notarized);
            assert_eq!(latest.change_number, 1);
          }
        } else if account.hd_path.is_some() {
          assert_eq!(latest.balance, "0");
          assert_eq!(latest.status, BalanceChangeStatus::NotebookPublished);
          assert!(latest.transaction_id.is_some());
          assert_eq!(latest.change_number, 2);
        } else {
          assert_eq!(latest.balance, "1500");
          assert_eq!(latest.status, BalanceChangeStatus::NotebookPublished);
          assert!(latest.transaction_id.is_some());
          assert_eq!(latest.change_number, 2);
        }
      }
    }

    for account in bob_accounts {
      let latest = bob_localchain
        .balance_changes()
        .get_latest_for_account(account.id)
        .await?
        .expect("Bob accounts should have balance");
      println!(
        "Latest Bob after sync for {:?} {:?} {:#?}",
        account.hd_path, account.account_type, latest
      );
      if account.account_type == AccountType::Tax {
        assert_eq!(latest.balance, "200");
        assert_eq!(latest.status, BalanceChangeStatus::NotebookPublished);
      } else {
        assert_eq!(latest.balance, "3100");
        assert_eq!(latest.status, BalanceChangeStatus::NotebookPublished);
      }
    }
    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account(AccountType::Deposit, 1)
      .await?
      .is_some());
    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account(AccountType::Tax, 1)
      .await?
      .is_some());

    Ok(())
  }

  #[sqlx::test]
  async fn test_request_transaction(bob_pool: SqlitePool) -> anyhow::Result<()> {
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

    // TODO: until jump accounts are free, this is needed
    mock_notary
      .create_claim_from_mainchain(
        bob_localchain.begin_change(),
        200u128,
        Ed25519Keyring::Bob.to_account_id(),
      )
      .await?;

    println!("Bob requesting");
    let bob_request_json = bob_localchain.transactions().request(3500_u128).await?;

    let alice_builder = alice_localchain.begin_change();
    alice_builder
      .accept_argon_file_request(bob_request_json)
      .await?;
    let _ = alice_builder.notarize().await?;
    println!("Alice accepted");
    let alice_accounts = alice_localchain.accounts().list(Some(true)).await?;
    assert_eq!(alice_accounts.len(), 2);

    let bob_accounts = bob_localchain.accounts().list(Some(true)).await?;
    assert_eq!(bob_accounts.len(), 4);

    for account in alice_accounts.clone() {
      if let Some(latest) = alice_localchain
        .balance_changes()
        .get_latest_for_account(account.id)
        .await?
      {
        println!(
          "Latest for Alice {:?} {:?} {:#?}",
          account.hd_path, account.account_type, latest
        );
        if account.account_type == AccountType::Tax {
          assert_eq!(latest.balance, "0");
        } else {
          assert_eq!(latest.balance, "1300");
          assert_eq!(latest.status, BalanceChangeStatus::Notarized);
          assert_eq!(latest.change_number, 2);
        }
      }
    }

    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account(AccountType::Deposit, 1)
      .await?
      .is_none());
    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account(AccountType::Tax, 1)
      .await?
      .is_none());

    for account in bob_accounts.clone() {
      let Some(latest) = bob_localchain
        .balance_changes()
        .get_latest_for_account(account.id)
        .await?
      else {
        continue;
      };
      println!(
        "Latest for Bob {:?} {:?} {:#?}",
        account.hd_path, account.account_type, latest
      );
      if account.account_type == AccountType::Tax {
        if account.hd_path.is_none() {
          assert_eq!(latest.balance, "0");
          assert_eq!(latest.change_number, 1);
        } else {
          assert_eq!(latest.balance, "200");
          assert!(latest.transaction_id.is_some());
          assert_eq!(latest.change_number, 1);
        }
      } else if account.hd_path.is_some() {
        assert_eq!(latest.balance, "3500");
        assert_eq!(latest.status, BalanceChangeStatus::WaitingForSendClaim);
        assert!(latest.transaction_id.is_some());
        assert_eq!(latest.change_number, 1);
      } else {
        // this is still the claim from mainchain
        assert_eq!(latest.balance, "200");
        assert_eq!(latest.status, BalanceChangeStatus::NotebookPublished);
        assert_eq!(latest.change_number, 1);
      }
    }
    let tips = mock_notary.get_pending_tips().await;
    let _ = mock_notary.create_notebook_header(tips).await;

    alice_localchain.balance_sync().sync(None).await?;
    println!("Alice synched");
    bob_localchain.balance_sync().sync(None).await?;
    println!("Bob synched");
    for account in alice_accounts {
      if let Some(latest) = alice_localchain
        .balance_changes()
        .get_latest_for_account(account.id)
        .await?
      {
        println!(
          "Latest Alice after sync for {} {:?} {:#?}",
          account.address, account.account_type, latest
        );
        if account.account_type == AccountType::Tax {
          if account.hd_path.is_some() {
            // should get moved to default tax
            assert_eq!(latest.balance, "0");
            assert_eq!(latest.status, BalanceChangeStatus::Notarized);
            assert_eq!(latest.change_number, 2);
          } else {
            assert_eq!(latest.balance, "200");
            assert_eq!(latest.status, BalanceChangeStatus::Notarized);
            assert_eq!(latest.change_number, 1);
          }
        } else if account.hd_path.is_some() {
          assert_eq!(latest.balance, "0");
          assert_eq!(latest.status, BalanceChangeStatus::NotebookPublished);
          assert!(latest.transaction_id.is_some());
          assert_eq!(latest.change_number, 2);
        } else {
          assert_eq!(latest.balance, "1300");
          assert_eq!(latest.status, BalanceChangeStatus::NotebookPublished);
          assert_eq!(latest.change_number, 2);
        }
      }
    }

    for account in bob_accounts {
      let latest = bob_localchain
        .balance_changes()
        .get_latest_for_account(account.id)
        .await?
        .expect("Bob accounts should have balance");
      println!(
        "Latest Bob after sync for {} {:?} {:#?}",
        account.address, account.account_type, latest
      );
      if account.account_type == AccountType::Tax {
        if account.hd_path.is_some() {
          assert_eq!(latest.balance, "0");
          assert_eq!(latest.change_number, 2);
        } else {
          assert_eq!(latest.balance, "400");
          assert_eq!(latest.change_number, 1);
        }
      } else if account.hd_path.is_some() {
        // should be getting consolidated
        assert_eq!(latest.balance, "0");
        assert_eq!(latest.status, BalanceChangeStatus::Notarized);
        assert!(latest.transaction_id.is_some());
        assert_eq!(latest.net_balance_change, "-3500");
        assert_eq!(latest.change_number, 2);
      } else {
        assert_eq!(latest.balance, "3500");
        assert_eq!(latest.status, BalanceChangeStatus::Notarized);
        assert_eq!(latest.change_number, 2);
      }
    }
    assert!(bob_localchain
      .accounts()
      .find_idle_jump_account(AccountType::Deposit, 1)
      .await?
      .is_some());
    assert!(bob_localchain
      .accounts()
      .find_idle_jump_account(AccountType::Tax, 1)
      .await?
      .is_some());

    Ok(())
  }
}
