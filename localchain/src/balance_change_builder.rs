use argon_primitives::{
  AccountId, AccountType, Balance, BalanceChange, Domain, MultiSignatureBytes, Note, NoteType,
  DOMAIN_LEASE_COST, MINIMUM_CHANNEL_HOLD_SETTLEMENT,
};
use lazy_static::lazy_static;
use sp_core::bounded_vec::BoundedVec;
use sp_core::{ed25519::Signature as EdSignature, ByteArray};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::bail;
use crate::Result;
use crate::{AccountStore, BalanceChangeStatus, LocalchainTransfer};

lazy_static! {
  static ref EMPTY_SIGNATURE: MultiSignatureBytes =
    MultiSignatureBytes::from(EdSignature::from_raw([0; 64]));
}

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone)]
pub struct BalanceChangeBuilder {
  balance_change: Arc<Mutex<BalanceChange>>,
  pub account_type: AccountType,
  pub address: String,
  pub local_account_id: i64,
  pub change_number: u32,
  pub sync_status: Option<BalanceChangeStatus>,
}

impl BalanceChangeBuilder {
  pub(crate) fn new(
    balance_change: BalanceChange,
    local_account_id: i64,
    status: Option<BalanceChangeStatus>,
  ) -> Self {
    let account_type = balance_change.account_type;
    let change_number = balance_change.change_number;
    let address = AccountStore::to_address(&balance_change.account_id);
    let mut change = balance_change;
    change.change_number += 1;
    Self {
      balance_change: Arc::new(Mutex::new(change)),
      account_type,
      local_account_id,
      address,
      change_number,
      sync_status: status,
    }
  }

  pub fn new_account(
    address: String,
    local_account_id: i64,
    account_type: AccountType,
  ) -> Result<Self> {
    Ok(Self::new(
      BalanceChange {
        account_id: AccountStore::parse_address(&address)?,
        account_type,
        change_number: 0,
        previous_balance_proof: None,
        balance: 0,
        channel_hold_note: None,
        notes: Default::default(),
        signature: EMPTY_SIGNATURE.clone(),
      },
      local_account_id,
      None,
    ))
  }

  pub async fn is_empty_signature(&self) -> bool {
    let balance_change = self.balance_change.lock().await;
    balance_change.signature == *EMPTY_SIGNATURE
  }

  pub async fn inner(&self) -> BalanceChange {
    self.balance_change.lock().await.clone()
  }

  pub fn balance_change_lock(&self) -> Arc<Mutex<BalanceChange>> {
    self.balance_change.clone()
  }

  pub async fn balance(&self) -> Balance {
    let balance_change = self.balance_change.lock().await;
    balance_change.balance
  }

  pub async fn account_id32(&self) -> Vec<u8> {
    let balance_change = self.balance_change.lock().await;
    balance_change.account_id.to_raw_vec()
  }

  pub async fn is_pending_claim(&self) -> bool {
    matches!(
      self.sync_status,
      Some(BalanceChangeStatus::WaitingForSendClaim)
    )
  }

  pub async fn send(
    &self,
    amount: Balance,
    restrict_to_addresses: Option<Vec<String>>,
  ) -> Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.balance < amount {
      bail!(
        "Insufficient balance {} to send {}",
        balance_change.balance,
        amount
      );
    }

    let mut to = None;

    if let Some(restrict_to_addresses) = restrict_to_addresses {
      let list: Result<Vec<AccountId>> = restrict_to_addresses
        .iter()
        .map(|a| AccountStore::parse_address(a))
        .collect::<_>();
      let list = list?;
      to = Some(BoundedVec::truncate_from(list));
    }

    balance_change.balance -= amount;
    balance_change.push_note(amount, NoteType::Send { to });
    Ok(())
  }

  pub async fn claim(&self, amount: Balance) -> Result<ClaimResult> {
    let mut balance_change = self.balance_change.lock().await;

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

  pub async fn claim_channel_hold(&self, amount: Balance) -> Result<ClaimResult> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Deposit {
      bail!(
        "Account {} is not a deposit account",
        balance_change.account_id
      );
    }

    let tax_amount = Note::calculate_channel_hold_tax(amount);
    balance_change.balance += amount - tax_amount;
    balance_change.push_note(amount, NoteType::ChannelHoldClaim);
    balance_change.push_note(tax_amount, NoteType::Tax);
    Ok(ClaimResult::new(amount, tax_amount))
  }

  pub async fn claim_from_mainchain(&self, transfer: LocalchainTransfer) -> Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    let account_id = AccountStore::parse_address(&transfer.address)?;
    if balance_change.account_id != account_id {
      bail!(
        "Transfer address {:?} does not match account address {}",
        transfer.address,
        balance_change.account_id
      );
    }
    if balance_change.account_type != AccountType::Deposit {
      bail!(
        "Transfer address {:?} is not a deposit account",
        transfer.address
      );
    }

    let amount = transfer.amount;
    balance_change.balance += amount;
    balance_change.push_note(
      amount,
      NoteType::ClaimFromMainchain {
        transfer_id: transfer.transfer_id,
      },
    );
    Ok(())
  }

  pub async fn send_to_mainchain(&self, amount: Balance) -> Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Deposit {
      bail!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      );
    }

    if balance_change.balance < amount {
      bail!(
        "Insufficient balance {} to send {}",
        balance_change.balance,
        amount
      );
    }

    balance_change.balance -= amount;
    balance_change.push_note(amount, NoteType::SendToMainchain);
    Ok(())
  }

  pub async fn create_channel_hold(
    &self,
    amount: Balance,
    payment_address: String,
    domain: Option<String>,
    delegated_signer_address: Option<String>,
  ) -> Result<()> {
    let domain_hash = if let Some(domain) = domain {
      Some(Domain::parse(domain)?.hash())
    } else {
      None
    };
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Deposit {
      bail!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      );
    }

    if balance_change.balance < amount {
      bail!(
        "Insufficient balance to create a channel_hold (address={}, balance={}, amount={})",
        self.address,
        balance_change.balance,
        amount
      );
    }
    if amount < MINIMUM_CHANNEL_HOLD_SETTLEMENT {
      bail!(
        "ChannelHold amount {} is less than minimum {}",
        amount,
        MINIMUM_CHANNEL_HOLD_SETTLEMENT
      );
    }

    // NOTE: channel hold doesn't manipulate balance
    balance_change.push_note(
      amount,
      NoteType::ChannelHold {
        recipient: AccountStore::parse_address(&payment_address)?,
        delegated_signer: match delegated_signer_address {
          Some(address) => Some(AccountStore::parse_address(&address)?),
          None => None,
        },
        domain_hash,
      },
    );
    Ok(())
  }

  pub async fn send_to_vote(&self, amount: Balance) -> Result<()> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Tax {
      bail!(
        "Votes must come from tax accounts. Account {:?} is not a tax account",
        balance_change.account_id
      );
    }

    if balance_change.balance < amount {
      bail!(
        "Insufficient balance {} to send {} to votes",
        balance_change.balance,
        amount
      );
    }

    balance_change.balance -= amount;
    balance_change.push_note(amount, NoteType::SendToVote);
    Ok(())
  }

  /// Lease a Domain. Domain leases are converted in full to tax.
  pub async fn lease_domain(&self) -> Result<Balance> {
    let mut balance_change = self.balance_change.lock().await;
    if balance_change.account_type != AccountType::Deposit {
      bail!(
        "Account {:?} is not a deposit account",
        balance_change.account_id
      );
    }
    let amount = DOMAIN_LEASE_COST;

    if balance_change.balance < amount {
      bail!(
        "Insufficient balance {} to lease a Domain for {}",
        balance_change.balance,
        amount
      );
    }

    balance_change.balance -= amount;
    balance_change.push_note(amount, NoteType::LeaseDomain);
    Ok(amount)
  }
}
#[cfg(feature = "napi")]
pub mod napi_ext {
  use crate::balance_change_builder::BalanceChangeBuilder;
  use crate::error::NapiOk;
  use crate::mainchain_client::napi_ext::LocalchainTransfer;
  use argon_primitives::{AccountType, BalanceChange};
  use napi::bindgen_prelude::*;

  #[napi(object, js_name = "ClaimResult")]
  pub struct ClaimResult {
    pub claimed: BigInt,
    pub tax: BigInt,
  }
  impl From<super::ClaimResult> for ClaimResult {
    fn from(result: super::ClaimResult) -> Self {
      Self {
        claimed: result.claimed.into(),
        tax: result.tax.into(),
      }
    }
  }

  #[napi]
  impl BalanceChangeBuilder {
    #[napi(factory, js_name = "newAccount")]
    pub fn new_account_napi(
      address: String,
      local_account_id: i64,
      account_type: AccountType,
    ) -> napi::Result<Self> {
      Self::new_account(address, local_account_id, account_type).napi_ok()
    }

    #[napi(js_name = "isEmptySignature")]
    pub async fn is_empty_signature_napi(&self) -> bool {
      self.is_empty_signature().await
    }

    #[napi(getter, js_name = "balance")]
    pub async fn balance_napi(&self) -> BigInt {
      self.balance().await.into()
    }

    #[napi(getter, js_name = "accountId32")]
    pub async fn account_id32_napi(&self) -> Uint8Array {
      self.account_id32().await.into()
    }

    #[napi(js_name = "isPendingClaim")]
    pub async fn is_pending_claim_napi(&self) -> bool {
      self.is_pending_claim().await
    }

    #[napi(js_name = "send")]
    pub async fn send_napi(
      &self,
      amount: BigInt,
      restrict_to_addresses: Option<Vec<String>>,
    ) -> napi::Result<()> {
      self
        .send(amount.get_u128().1, restrict_to_addresses)
        .await
        .napi_ok()
    }

    #[napi(js_name = "claim")]
    pub async fn claim_napi(&self, amount: BigInt) -> napi::Result<ClaimResult> {
      self
        .claim(amount.get_u128().1)
        .await
        .map(Into::into)
        .napi_ok()
    }

    #[napi(js_name = "claimChannelHold")]
    pub async fn claim_channel_hold_napi(&self, amount: BigInt) -> napi::Result<ClaimResult> {
      self
        .claim_channel_hold(amount.get_u128().1)
        .await
        .map(Into::into)
        .napi_ok()
    }

    #[napi(js_name = "claimFromMainchain")]
    pub async fn claim_from_mainchain_napi(
      &self,
      transfer: LocalchainTransfer,
    ) -> napi::Result<()> {
      self
        .claim_from_mainchain(super::LocalchainTransfer {
          address: transfer.address,
          amount: transfer.amount.get_u128().1,
          notary_id: transfer.notary_id,
          expiration_tick: transfer.expiration_tick as u64,
          transfer_id: transfer.transfer_id,
        })
        .await
        .napi_ok()
    }

    #[napi(js_name = "sendToMainchain")]
    pub async fn send_to_mainchain_napi(&self, amount: BigInt) -> napi::Result<()> {
      self.send_to_mainchain(amount.get_u128().1).await.napi_ok()
    }

    #[napi(js_name = "createChannelHold")]
    pub async fn create_channel_hold_napi(
      &self,
      amount: BigInt,
      payment_address: String,
      domain: Option<String>,
      delegated_signer_address: Option<String>,
    ) -> napi::Result<()> {
      self
        .create_channel_hold(
          amount.get_u128().1,
          payment_address,
          domain,
          delegated_signer_address,
        )
        .await
        .napi_ok()
    }

    #[napi(js_name = "sendToVote")]
    pub async fn send_to_vote_napi(&self, amount: BigInt) -> napi::Result<()> {
      self.send_to_vote(amount.get_u128().1).await.napi_ok()
    }

    /// Lease a Domain. Domain leases are converted in full to tax.
    #[napi(js_name = "leaseDomain")]
    pub async fn lease_domain_napi(&self) -> napi::Result<BigInt> {
      self.lease_domain().await.map(Into::into).napi_ok()
    }

    /// Create scale encoded signature message for the balance change.
    #[napi(js_name = "toSigningMessage")]
    pub fn to_signing_message_napi(balance_change_json: String) -> napi::Result<Uint8Array> {
      let balance_change: BalanceChange = serde_json::from_str(&balance_change_json)?;
      Ok(balance_change.hash().0.into())
    }
  }
}

pub struct ClaimResult {
  pub claimed: Balance,
  pub tax: Balance,
}
impl ClaimResult {
  fn new(claimed: Balance, tax: Balance) -> Self {
    Self { claimed, tax }
  }
}

#[cfg(test)]
mod test {
  use sp_keyring::AccountKeyring::Bob;
  use sp_keyring::Ed25519Keyring::Alice;

  use super::*;

  #[tokio::test]
  async fn test_building_balance_change() {
    let address = AccountStore::to_address(&Bob.to_account_id());
    let builder =
      BalanceChangeBuilder::new_account(address.clone(), 1, AccountType::Deposit).unwrap();
    builder
      .claim_from_mainchain(LocalchainTransfer {
        address,
        amount: 100u128,
        notary_id: 1,
        expiration_tick: 500,
        transfer_id: 1,
      })
      .await
      .unwrap();

    let balance_change = builder.inner().await;

    assert_eq!(balance_change.balance, 100);
    assert_eq!(balance_change.notes.len(), 1);

    builder.send(55u128, None).await.unwrap();
    assert_eq!(builder.inner().await.balance, 45);
  }

  #[tokio::test]
  async fn test_building_balance_change_with_restrict_to_addresses() {
    let address = AccountStore::to_address(&Bob.to_account_id());
    let builder =
      BalanceChangeBuilder::new_account(address.clone(), 1, AccountType::Deposit).unwrap();
    builder
      .claim_from_mainchain(LocalchainTransfer {
        address,
        amount: 100u128,
        notary_id: 1,
        expiration_tick: 500,
        transfer_id: 1,
      })
      .await
      .unwrap();

    let balance_change = builder.inner().await;

    assert_eq!(balance_change.balance, 100);
    assert_eq!(balance_change.notes.len(), 1);

    builder
      .send(
        55u128,
        Some(vec![AccountStore::to_address(&Bob.to_account_id())]),
      )
      .await
      .unwrap();
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
  }

  #[tokio::test]
  async fn test_channel_hold() {
    let address = AccountStore::to_address(&Bob.to_account_id());
    let payment_address = AccountStore::to_address(&Alice.to_account_id());
    let builder =
      BalanceChangeBuilder::new_account(address.clone(), 1, AccountType::Deposit).unwrap();
    builder
      .claim_from_mainchain(LocalchainTransfer {
        address,
        amount: 20_000_000u128,
        notary_id: 1,
        expiration_tick: 500,
        transfer_id: 1,
      })
      .await
      .unwrap();

    builder
      .create_channel_hold(
        1_000_000u128,
        payment_address.clone(),
        Some("test.flights".to_string()),
        None,
      )
      .await
      .unwrap();

    let balance_change = builder.inner().await;
    // no funds move yet
    assert_eq!(balance_change.balance, 20_000_000);
    assert_eq!(balance_change.notes.len(), 2);

    let alice = Alice.to_account_id();

    match &balance_change.notes[1].note_type {
      NoteType::ChannelHold { recipient, .. } => assert_eq!(recipient, &alice),
      _ => unreachable!(),
    };
  }
}
