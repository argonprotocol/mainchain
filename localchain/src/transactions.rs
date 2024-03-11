use futures::TryFutureExt;
use napi::bindgen_prelude::*;
use sqlx::SqlitePool;

use ulx_primitives::{AccountType, Note};

use crate::file_transfer::ArgonFileType;
use crate::notarization_builder::NotarizationBuilder;
use crate::notary_client::NotaryClients;
use crate::signer::Signer;
use crate::{to_js_error, OpenEscrow, OpenEscrowsStore, TickerRef};

#[derive(Debug, PartialOrd, PartialEq)]
#[napi]
pub enum TransactionType {
  Send,
  Request,
  OpenEscrow,
}

impl From<i64> for TransactionType {
  fn from(i: i64) -> Self {
    match i {
      0 => TransactionType::Send,
      1 => TransactionType::Request,
      2 => TransactionType::OpenEscrow,
      _ => panic!("Unknown transaction type {}", i),
    }
  }
}

#[napi(object)]
#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub struct LocalchainTransaction {
  pub id: u32,
  pub transaction_type: TransactionType,
}

#[napi]
pub struct Transactions {
  db: SqlitePool,
  ticker: TickerRef,
  notary_clients: NotaryClients,
  signer: Signer,
}
#[napi]
impl Transactions {
  pub fn new(
    db: SqlitePool,
    ticker: TickerRef,
    notary_clients: &NotaryClients,
    signer: &Signer,
  ) -> Self {
    Self {
      db,
      ticker,
      notary_clients: notary_clients.clone(),
      signer: signer.clone(),
    }
  }

  fn new_notarization(&self) -> NotarizationBuilder {
    NotarizationBuilder::new(
      self.db.clone(),
      self.notary_clients.clone(),
      self.signer.clone(),
    )
  }

  pub async fn create(&self, transaction_type: TransactionType) -> Result<LocalchainTransaction> {
    let transaction_id = sqlx::query_scalar!(
      "INSERT INTO transactions (transaction_type) VALUES (?) RETURNING ID",
      TransactionType::Request as i64
    )
    .fetch_one(&self.db)
    .map_err(to_js_error)
    .await?;

    Ok(LocalchainTransaction {
      id: transaction_id as u32,
      transaction_type,
    })
  }

  #[napi]
  pub async fn request(&self, milligons: BigInt) -> Result<String> {
    let transaction = self.create(TransactionType::Request).await?;

    let jump_notarization = self.new_notarization();
    jump_notarization.set_transaction(transaction).await;
    let milligons_plus_tax = jump_notarization.get_total_for_after_tax_balance(milligons.clone());
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

  #[napi]
  pub async fn create_escrow(
    &self,
    escrow_milligons: BigInt,
    data_domain: String,
    data_domain_address: String,
  ) -> Result<OpenEscrow> {
    let jump_notarization = self.new_notarization();
    let transaction = self.create(TransactionType::OpenEscrow).await?;
    jump_notarization.set_transaction(transaction.clone()).await;

    let amount_plus_tax =
      jump_notarization.get_total_for_after_tax_balance(escrow_milligons.clone());
    let jump_account = jump_notarization.fund_jump_account(amount_plus_tax).await?;
    let _ = jump_notarization.notarize().await?;

    let escrow_notarization = self.new_notarization();
    escrow_notarization.set_transaction(transaction).await;
    escrow_notarization
      .add_account_by_id(jump_account.local_account_id)
      .await?
      .create_escrow_hold(escrow_milligons, data_domain, data_domain_address)
      .await?;
    escrow_notarization.notarize().await?;

    let escrow = OpenEscrowsStore::new(
      self.db.clone(),
      self.ticker.clone(),
      &self.notary_clients,
      &self.signer,
    );
    let open_escrow = escrow
      .open_client_escrow(jump_account.local_account_id)
      .await?;
    Ok(open_escrow)
  }

  #[napi]
  pub async fn send(&self, milligons: BigInt, to: Option<Vec<String>>) -> Result<String> {
    let jump_notarization = self.new_notarization();
    let transaction = self.create(TransactionType::Send).await?;
    jump_notarization.set_transaction(transaction.clone()).await;
    let jump_account = jump_notarization
      .fund_jump_account(milligons.clone())
      .await?;
    let _ = jump_notarization.notarize().await?;

    let amount = milligons.get_u128().1;
    let tax = Note::calculate_transfer_tax(amount);

    let fund_notarization = self.new_notarization();
    fund_notarization.set_transaction(transaction).await;
    fund_notarization
      .add_account_by_id(jump_account.local_account_id)
      .await?
      .send(BigInt::from(amount - tax), to)
      .await?;

    let json = fund_notarization
      .export_as_file(ArgonFileType::Send)
      .await?;

    Ok(json)
  }
}

#[cfg(test)]
mod tests {
  use sp_keyring::AccountKeyring::{Alice, Bob};
  use sp_keyring::Ed25519Keyring;
  use sp_keyring::Ed25519Keyring::Ferdie;
  use ulx_primitives::AccountType;

  use crate::test_utils::{create_mock_notary, create_pool, mock_notary_clients};
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
      .send(
        3500_u128.into(),
        Some(vec![bob_localchain.address().await?]),
      )
      .await?;

    let bob_builder = bob_localchain.begin_change();
    bob_builder.import_argon_file(alice_json).await?;
    let _ = bob_builder.notarize().await?;

    let alice_accounts = alice_localchain.accounts().list_js(Some(true)).await?;
    assert_eq!(alice_accounts.len(), 4);

    let bob_accounts = bob_localchain.accounts().list_js(Some(true)).await?;
    assert_eq!(bob_accounts.len(), 2);

    let mut tips = vec![];
    for account in alice_accounts.clone() {
      if let Some(latest) = alice_localchain
        .balance_changes()
        .get_latest_for_account_js(account.id)
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
        } else {
          if account.hd_path.is_some() {
            assert_eq!(latest.balance, "0");
            assert_eq!(latest.status, BalanceChangeStatus::WaitingForSendClaim);
            assert!(latest.transaction_id.is_some());
            assert_eq!(latest.change_number, 2);
          } else {
            assert_eq!(latest.balance, "1500");
            assert_eq!(latest.status, BalanceChangeStatus::SubmittedToNotary);
            assert!(latest.transaction_id.is_some());
            assert_eq!(latest.change_number, 2);
          }
        }
        tips.push(latest.get_balance_tip(&account)?);
      }
    }

    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account_js(AccountType::Deposit, 1)
      .await?
      .is_none());
    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account_js(AccountType::Tax, 1)
      .await?
      .is_none());

    for account in bob_accounts.clone() {
      let latest = bob_localchain
        .balance_changes()
        .get_latest_for_account_js(account.id)
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
        .get_latest_for_account_js(account.id)
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
            assert_eq!(latest.status, BalanceChangeStatus::SubmittedToNotary);
            assert_eq!(latest.change_number, 2);
          } else {
            assert_eq!(latest.balance, "200");
            assert_eq!(latest.status, BalanceChangeStatus::SubmittedToNotary);
            assert_eq!(latest.change_number, 1);
          }
        } else {
          if account.hd_path.is_some() {
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
    }

    for account in bob_accounts {
      let latest = bob_localchain
        .balance_changes()
        .get_latest_for_account_js(account.id)
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
      .find_idle_jump_account_js(AccountType::Deposit, 1)
      .await?
      .is_some());
    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account_js(AccountType::Tax, 1)
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
    let bob_request_json = bob_localchain
      .transactions()
      .request(3500_u128.into())
      .await?;

    let alice_builder = alice_localchain.begin_change();
    alice_builder
      .accept_argon_file_request(bob_request_json)
      .await?;
    let _ = alice_builder.notarize().await?;
    println!("Alice accepted");
    let alice_accounts = alice_localchain.accounts().list_js(Some(true)).await?;
    assert_eq!(alice_accounts.len(), 2);

    let bob_accounts = bob_localchain.accounts().list_js(Some(true)).await?;
    assert_eq!(bob_accounts.len(), 4);

    for account in alice_accounts.clone() {
      if let Some(latest) = alice_localchain
        .balance_changes()
        .get_latest_for_account_js(account.id)
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
          assert_eq!(latest.status, BalanceChangeStatus::SubmittedToNotary);
          assert_eq!(latest.change_number, 2);
        }
      }
    }

    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account_js(AccountType::Deposit, 1)
      .await?
      .is_none());
    assert!(alice_localchain
      .accounts()
      .find_idle_jump_account_js(AccountType::Tax, 1)
      .await?
      .is_none());

    for account in bob_accounts.clone() {
      let Some(latest) = bob_localchain
        .balance_changes()
        .get_latest_for_account_js(account.id)
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
      } else {
        if account.hd_path.is_some() {
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
        .get_latest_for_account_js(account.id)
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
            assert_eq!(latest.status, BalanceChangeStatus::SubmittedToNotary);
            assert_eq!(latest.change_number, 2);
          } else {
            assert_eq!(latest.balance, "200");
            assert_eq!(latest.status, BalanceChangeStatus::SubmittedToNotary);
            assert_eq!(latest.change_number, 1);
          }
        } else {
          if account.hd_path.is_some() {
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
    }

    for account in bob_accounts {
      let latest = bob_localchain
        .balance_changes()
        .get_latest_for_account_js(account.id)
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
      } else {
        if account.hd_path.is_some() {
          assert_eq!(latest.balance, "0");
          assert_eq!(latest.status, BalanceChangeStatus::SubmittedToNotary);
          assert!(latest.transaction_id.is_none());
          assert_eq!(latest.change_number, 2);
        } else {
          assert_eq!(latest.balance, "3500");
          assert_eq!(latest.status, BalanceChangeStatus::SubmittedToNotary);
          assert_eq!(latest.change_number, 2);
        }
      }
    }
    assert!(bob_localchain
      .accounts()
      .find_idle_jump_account_js(AccountType::Deposit, 1)
      .await?
      .is_some());
    assert!(bob_localchain
      .accounts()
      .find_idle_jump_account_js(AccountType::Tax, 1)
      .await?
      .is_some());

    Ok(())
  }

  async fn mock_localchain(
    pool: &SqlitePool,
    suri: &str,
    crypto_scheme: CryptoScheme,
    notary_clients: &NotaryClients,
  ) -> Localchain {
    let ticker = TickerRef {
      ticker: Ticker::start(Duration::from_secs(60)),
    };
    let signer = Signer::new(pool.clone());
    let _ = signer
      .import_suri_to_embedded(suri.to_string(), crypto_scheme, None)
      .await
      .expect("should import");
    Localchain {
      db: pool.clone(),
      signer: signer.clone(),
      ticker: ticker.clone(),
      notary_clients: notary_clients.clone(),
      path: ":memory:".to_string(),
      mainchain_client: Default::default(),
    }
  }
}
