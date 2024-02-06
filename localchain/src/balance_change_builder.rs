use crate::{to_js_error, AccountStore, JsDataDomain, LocalchainTransfer};
use napi::bindgen_prelude::*;
use napi::Error;
use sp_core::bounded_vec::BoundedVec;
use sp_core::{ed25519, ByteArray};
use sp_runtime::MultiSignature;
use std::sync::Arc;
use tokio::sync::Mutex;
use ulx_primitives::{
  AccountId, AccountType, BalanceChange, DataDomain, Note, NoteType, DATA_DOMAIN_LEASE_COST,
  MIN_ESCROW_NOTE_MILLIGONS,
};

#[napi]
#[derive(Clone)]
pub struct BalanceChangeBuilder {
  balance_change: Arc<Mutex<BalanceChange>>,
  pub account_type: AccountType,
  pub address: String,
  pub change_number: u32,
}

#[napi]
impl BalanceChangeBuilder {
  pub(crate) fn new(balance_change: BalanceChange) -> Self {
    let account_type = balance_change.account_type;
    let change_number = balance_change.change_number;
    let address = AccountStore::to_address(&balance_change.account_id);
    let mut change = balance_change;
    change.change_number += 1;
    Self {
      balance_change: Arc::new(Mutex::new(change)),
      account_type,
      address,
      change_number,
    }
  }

  #[napi(factory)]
  pub fn new_account(address: String, account_type: AccountType) -> napi::Result<Self> {
    Ok(Self::new(BalanceChange {
      account_id: AccountStore::parse_address(&address)?,
      account_type,
      change_number: 0,
      previous_balance_proof: None,
      balance: 0,
      escrow_hold_note: None,
      notes: Default::default(),
      signature: MultiSignature::from(ed25519::Signature([0; 64])),
    }))
  }

  #[napi]
  pub async fn is_empty_signature(&self) -> bool {
    let balance_change = self.balance_change.lock().await;
    (*balance_change).signature == ed25519::Signature([0; 64]).into()
  }

  pub async fn inner(&self) -> BalanceChange {
    self.balance_change.lock().await.clone()
  }

  pub fn balance_change_lock(&self) -> Arc<Mutex<BalanceChange>> {
    self.balance_change.clone()
  }

  #[napi(getter)]
  pub async fn balance(&self) -> BigInt {
    let balance_change = self.balance_change.lock().await;
    BigInt::from((*balance_change).balance)
  }

  #[napi(getter)]
  pub async fn account_id32(&self) -> Uint8Array {
    let balance_change = self.balance_change.lock().await;
    let bytes = (*balance_change).account_id.to_raw_vec();
    bytes.into()
  }

  #[napi]
  pub async fn send(
    &self,
    amount: BigInt,
    restrict_to_addresses: Option<Vec<String>>,
  ) -> napi::Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    let (_, amount, _) = amount.get_u128();
    if balance_change.balance < amount {
      return Err(Error::from_reason(format!(
        "Insufficient balance {} to send {}",
        balance_change.balance, amount
      )));
    }

    let mut to = None;

    if let Some(restrict_to_addresses) = restrict_to_addresses {
      if restrict_to_addresses.len() > 1 {
        return Err(Error::from_reason("Only one recipient is allowed"));
      }
      let list: napi::Result<Vec<AccountId>> = restrict_to_addresses
        .iter()
        .map(|a| AccountStore::parse_address(&a))
        .collect::<_>();
      let list = list.map_err(to_js_error)?;
      to = Some(BoundedVec::truncate_from(list));
    }

    balance_change.balance -= amount;
    balance_change.push_note(amount, NoteType::Send { to });
    Ok(())
  }

  #[napi]
  pub async fn claim(&self, amount: BigInt) -> napi::Result<ClaimResult> {
    let mut balance_change = self.balance_change.lock().await;

    let (_, amount, _) = amount.get_u128();
    let tax_amount = match balance_change.account_type {
      AccountType::Deposit => Note::calculate_transfer_tax(amount),
      AccountType::Tax => 0,
    };
    balance_change.balance += amount - tax_amount;

    balance_change.push_note(amount, NoteType::Claim);
    if tax_amount > 0 {
      balance_change.push_note(tax_amount, NoteType::Tax);
    }
    Ok(ClaimResult::new(amount, tax_amount))
  }

  #[napi]
  pub async fn claim_escrow(&self, amount: BigInt) -> napi::Result<ClaimResult> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Deposit {
      return Err(to_js_error(format!(
        "Account {} is not a deposit account",
        balance_change.account_id
      )));
    }
    let (_, amount, _) = amount.get_u128();
    let tax_amount = Note::calculate_escrow_tax(amount);
    balance_change.balance += amount - tax_amount;
    balance_change.push_note(amount, NoteType::EscrowClaim);
    balance_change.push_note(tax_amount, NoteType::Tax);
    Ok(ClaimResult::new(amount, tax_amount))
  }

  #[napi]
  pub async fn claim_from_mainchain(&self, transfer: LocalchainTransfer) -> napi::Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    let account_id = AccountStore::parse_address(&transfer.address)?;
    if balance_change.account_id != account_id {
      return Err(Error::from_reason(format!(
        "Transfer address {:?} does not match account address {}",
        transfer.address, balance_change.account_id
      )));
    }
    if balance_change.account_type != AccountType::Deposit {
      return Err(Error::from_reason(format!(
        "Transfer address {:?} is not a deposit account",
        transfer.address
      )));
    }

    let (_, amount, _) = transfer.amount.get_u128();
    balance_change.balance += amount;
    balance_change.push_note(
      amount,
      NoteType::ClaimFromMainchain {
        account_nonce: transfer.account_nonce,
      },
    );
    Ok(())
  }

  #[napi]
  pub async fn send_to_mainchain(&self, amount: BigInt) -> napi::Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Deposit {
      return Err(Error::from_reason(format!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      )));
    }
    let (_, amount, _) = amount.get_u128();

    if balance_change.balance < amount {
      return Err(Error::from_reason(format!(
        "Insufficient balance {} to send {}",
        balance_change.balance, amount
      )));
    }

    balance_change.balance -= amount;
    balance_change.push_note(amount, NoteType::SendToMainchain);
    Ok(())
  }

  #[napi(ts_args_type = "amount: bigint, dataDomain: DataDomain, dataDomainAddress: string")]
  pub async fn create_escrow_hold(
    &self,
    amount: BigInt,
    data_domain: JsDataDomain,
    data_domain_address: String,
  ) -> napi::Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Deposit {
      return Err(Error::from_reason(format!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      )));
    }
    let (_, amount, _) = amount.get_u128();

    if balance_change.balance < amount {
      return Err(Error::from_reason(format!(
        "Insufficient balance {} to create an escrow {}",
        balance_change.balance, amount
      )));
    }
    if amount < MIN_ESCROW_NOTE_MILLIGONS {
      return Err(Error::from_reason(format!(
        "Escrow amount {} is less than minimum {}",
        amount, MIN_ESCROW_NOTE_MILLIGONS
      )));
    }

    let domain: DataDomain = data_domain.into();
    // NOTE: escrow hold doesn't manipulate balance
    balance_change.push_note(
      amount,
      NoteType::EscrowHold {
        data_domain_hash: Some(domain.hash()),
        recipient: AccountStore::parse_address(&data_domain_address)?,
      },
    );
    Ok(())
  }

  #[napi]
  pub async fn send_to_vote(&self, amount: BigInt) -> napi::Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Tax {
      return Err(Error::from_reason(format!(
        "Votes must come from tax accounts. Account {:?} is not a tax account",
        balance_change.account_id
      )));
    }
    let (_, amount, _) = amount.get_u128();

    if balance_change.balance < amount {
      return Err(Error::from_reason(format!(
        "Insufficient balance {} to send {} to votes",
        balance_change.balance, amount
      )));
    }

    balance_change.balance -= amount;
    balance_change.push_note(amount, NoteType::SendToVote);
    Ok(())
  }

  /// Lease a data domain. DataDomain leases are converted in full to tax.
  #[napi]
  pub async fn lease_data_domain(&self) -> napi::Result<BigInt> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Deposit {
      return Err(Error::from_reason(format!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      )));
    }
    let amount = DATA_DOMAIN_LEASE_COST;

    if balance_change.balance < amount {
      return Err(Error::from_reason(format!(
        "Insufficient balance {} to lease a data domain for {}",
        balance_change.balance, amount
      )));
    }

    balance_change.balance -= amount;
    balance_change.push_note(amount, NoteType::LeaseDomain);
    Ok(BigInt::from(amount))
  }

  #[napi]
  pub async fn create_private_server_escrow_hold(
    &self,
    amount: BigInt,
    payment_address: String,
  ) -> napi::Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Deposit {
      return Err(Error::from_reason(format!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      )));
    }
    let (_, amount, _) = amount.get_u128();

    if balance_change.balance < amount {
      return Err(Error::from_reason(format!(
        "Insufficient balance {} to create an escrow {}",
        balance_change.balance, amount
      )));
    }

    balance_change.balance -= amount;
    balance_change.push_note(
      amount,
      NoteType::EscrowHold {
        data_domain_hash: None,
        recipient: AccountStore::parse_address(&payment_address)?,
      },
    );
    Ok(())
  }
}

#[napi(object)]
pub struct ClaimResult {
  pub claimed: BigInt,
  pub tax: BigInt,
}
impl ClaimResult {
  fn new(claimed: u128, tax: u128) -> Self {
    Self {
      claimed: BigInt::from(claimed),
      tax: BigInt::from(tax),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::JsDataDomain;
  use sp_keyring::AccountKeyring::Bob;
  use sp_keyring::Ed25519Keyring::Alice;
  use ulx_primitives::DataTLD;

  #[tokio::test]
  async fn test_building_balance_change() -> anyhow::Result<()> {
    let address = AccountStore::to_address(&Bob.to_account_id());
    let builder = BalanceChangeBuilder::new_account(address.clone(), AccountType::Deposit)?;
    builder
      .claim_from_mainchain(LocalchainTransfer {
        address,
        amount: BigInt::from(100u128),
        notary_id: 1,
        expiration_block: 500,
        account_nonce: 1,
      })
      .await?;

    let balance_change = builder.inner().await;

    assert_eq!(balance_change.balance, 100);
    assert_eq!(balance_change.notes.len(), 1);

    builder.send(BigInt::from(55u128), None).await?;
    assert_eq!(builder.inner().await.balance, 45);

    Ok(())
  }

  #[tokio::test]
  async fn test_building_balance_change_with_restrict_to_addresses() -> anyhow::Result<()> {
    let address = AccountStore::to_address(&Bob.to_account_id());
    let builder = BalanceChangeBuilder::new_account(address.clone(), AccountType::Deposit)?;
    builder
      .claim_from_mainchain(LocalchainTransfer {
        address,
        amount: BigInt::from(100u128),
        notary_id: 1,
        expiration_block: 500,
        account_nonce: 1,
      })
      .await?;

    let balance_change = builder.inner().await;

    assert_eq!(balance_change.balance, 100);
    assert_eq!(balance_change.notes.len(), 1);

    builder
      .send(
        BigInt::from(55u128),
        Some(vec![AccountStore::to_address(&Bob.to_account_id())]),
      )
      .await?;
    let balance_change = builder.inner().await;
    assert_eq!(balance_change.balance, 45);
    assert_eq!(balance_change.notes.len(), 2);
    assert!(matches!(
      balance_change.notes[1].note_type,
      NoteType::Send { to: Some(_) }
    ),);
    let to = match &balance_change.notes[1].note_type {
      NoteType::Send { to } => to.clone().unwrap().to_vec(),
      _ => unreachable!(),
    };
    assert_eq!(to.len(), 1);
    assert_eq!(to[0], Bob.to_account_id());

    Ok(())
  }

  #[tokio::test]
  async fn test_escrow_hold() -> anyhow::Result<()> {
    let address = AccountStore::to_address(&Bob.to_account_id());
    let data_domain_author = AccountStore::to_address(&Alice.to_account_id());
    let builder = BalanceChangeBuilder::new_account(address.clone(), AccountType::Deposit)?;
    builder
      .claim_from_mainchain(LocalchainTransfer {
        address,
        amount: BigInt::from(20_000u128),
        notary_id: 1,
        expiration_block: 500,
        account_nonce: 1,
      })
      .await?;

    builder
      .create_escrow_hold(
        BigInt::from(1_000u128),
        JsDataDomain {
          domain_name: "test".to_string(),
          top_level_domain: DataTLD::Flights,
        },
        data_domain_author.clone(),
      )
      .await?;

    let balance_change = builder.inner().await;
    // no funds move yet
    assert_eq!(balance_change.balance, 20_000);
    assert_eq!(balance_change.notes.len(), 2);

    let alice = Alice.to_account_id();

    match &balance_change.notes[1].note_type {
      NoteType::EscrowHold {
        data_domain_hash: _,
        recipient,
      } => assert_eq!(recipient, &alice),
      _ => unreachable!(),
    };

    Ok(())
  }
}
