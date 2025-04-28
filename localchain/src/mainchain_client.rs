use polkadot_sdk::*;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::anyhow;
use sp_core::crypto::AccountId32;
use sp_core::Decode;
use sp_core::{ByteArray, H256};
use sp_runtime::MultiSignature;
use subxt::runtime_api::Payload as RuntimeApiPayload;
use subxt::storage::Address as StorageAddress;
use subxt::utils::Yes;
use tokio::sync::{Mutex, RwLock};
use tracing::warn;

use argon_client::api::storage;
use argon_client::api::{runtime_types, tx};

use argon_client::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use argon_client::{
  api, ArgonConfig, ArgonExtrinsicParamsBuilder, FetchAt, MainchainClient as InnerMainchainClient,
  TxInBlockWithEvents,
};
use argon_primitives::host::Host;
use argon_primitives::tick::{Tick, Ticker};
use argon_primitives::{
  Balance, ChainIdentity, Domain, DomainTopLevel, NotaryId, NotebookNumber, TransferToLocalchainId,
};

use crate::AccountStore;
use crate::Keystore;
use crate::{bail, Result};

#[cfg_attr(feature = "napi", napi)]
#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct MainchainClient {
  client: Arc<RwLock<Option<InnerMainchainClient>>>,
  pub host: String,
  join_handles: Arc<Mutex<Option<(tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>)>>>,
}

#[cfg(test)]
impl MainchainClient {
  pub fn mock() -> Self {
    Self {
      host: "mock".to_string(),
      client: Arc::new(RwLock::new(None)),
      join_handles: Arc::new(Mutex::new(None)),
    }
  }
}

impl MainchainClient {
  async fn client(&self) -> Result<InnerMainchainClient> {
    self.ensure_connected(10_000).await?;
    let client_lock = self.client.read().await;
    let client_rpc = (*client_lock)
      .as_ref()
      .ok_or_else(|| anyhow!("Could not connect to mainchain client at {}", self.host))?;
    Ok(client_rpc.clone())
  }

  async fn ensure_connected(&self, timeout_millis: i64) -> Result<()> {
    if self.client.read().await.is_some() {
      return Ok(());
    }

    if let Some((handle1, handle2)) = (*self.join_handles.lock().await).take() {
      handle1.abort();
      handle2.abort();
    }

    let mut client =
      InnerMainchainClient::try_until_connected(self.host.as_str(), 5_000, timeout_millis as u64)
        .await?;
    let mut on_error = client.subscribe_client_error();
    let ws_client = client.ws_client.clone();

    self.client.write().await.replace(client);

    let client_lock_1 = self.client.clone();
    let client_lock_2 = self.client.clone();
    let url_1 = self.host.clone();
    let url_2 = self.host.clone();

    let handle1 = tokio::spawn(async move {
      let url = url_1.clone();
      let err = on_error.recv().await.unwrap_or_default();

      warn!("Disconnected from mainchain at {url} with error {err}",);
      client_lock_1.write().await.take();
    });

    let handle2 = tokio::spawn(async move {
      let url = url_2.clone();
      let _ = ws_client.on_disconnect().await;

      warn!("Disconnected from mainchain at {url}",);
      client_lock_2.write().await.take();
    });

    *self.join_handles.lock().await = Some((handle1, handle2));

    Ok(())
  }

  pub async fn close(&self) -> Result<()> {
    if let Some((handle1, handle2)) = (*self.join_handles.lock().await).take() {
      handle1.abort();
      handle2.abort();
    }
    let mut client_lock = self.client.write().await;
    if let Some(client) = (*client_lock).take() {
      drop(client);
    }
    Ok(())
  }

  pub async fn connect(host: String, timeout_millis: i64) -> Result<Self> {
    let instance = Self {
      host,
      client: Arc::new(RwLock::new(None)),
      join_handles: Arc::new(Mutex::new(None)),
    };
    instance.ensure_connected(timeout_millis).await?;
    Ok(instance)
  }

  pub async fn call<Call: RuntimeApiPayload>(&self, payload: Call) -> Result<Call::ReturnType> {
    Ok(self.client().await?.call(payload, None).await?)
  }

  pub async fn fetch_storage<Address>(
    &self,
    address: &Address,
    at: Option<H256>,
  ) -> Result<Option<Address::Target>>
  where
    Address: StorageAddress<IsFetchable = Yes>,
  {
    Ok(
      self
        .client()
        .await?
        .fetch_storage(address, at.map(Into::into).unwrap_or_default())
        .await?,
    )
  }

  pub async fn get_ticker(&self) -> Result<Ticker> {
    let client = self.client().await?;
    Ok(client.lookup_ticker().await?)
  }

  pub async fn get_chain_identity(&self) -> Result<ChainIdentity> {
    let client = self.client().await?;
    Ok(client.get_chain_identity().await?)
  }

  pub async fn get_best_block_hash(&self) -> Result<H256> {
    Ok(self.client().await?.best_block_hash().await?)
  }

  pub async fn get_vote_block_hash(&self, current_tick: Tick) -> Result<Option<BestBlockForVote>> {
    let (block_hash, vote_minimum) = self
      .client()
      .await?
      .get_vote_block_hash(current_tick)
      .await?;
    Ok(Some(BestBlockForVote {
      block_hash,
      vote_minimum,
    }))
  }

  pub async fn get_domain_registration(
    &self,
    domain_name: String,
    top_level: DomainTopLevel,
  ) -> Result<Option<DomainRegistration>> {
    let domain_hash: argon_client::types::H256 =
      Domain::from_string(domain_name, top_level).hash().into();

    let best_block_hash = self.get_best_block_hash().await?;
    if let Some(x) = self
      .fetch_storage(
        &storage().domains().registered_domains(domain_hash),
        Some(best_block_hash),
      )
      .await?
    {
      let registered_to_address = match AccountId32::from_slice(&x.account_id.0) {
        Ok(s) => AccountStore::to_address(&s),
        Err(_) => {
          bail!(
            "Could not parse the domain registration address {}",
            &x.account_id
          );
        }
      };
      Ok(Some(DomainRegistration {
        registered_to_address,
        registered_at_tick: x.registered_at_tick as i64,
      }))
    } else {
      Ok(None)
    }
  }

  pub async fn get_domain_zone_record(
    &self,
    domain_name: String,
    top_level: DomainTopLevel,
  ) -> Result<Option<ZoneRecord>> {
    let domain: argon_client::types::H256 =
      Domain::from_string(domain_name, top_level).hash().into();
    let Some(zone_record) = self
      .fetch_storage(&storage().domains().zone_records_by_domain(domain), None)
      .await?
    else {
      return Ok(None);
    };

    let payment_address = match AccountId32::from_slice(&zone_record.payment_account.0) {
      Ok(s) => s,
      Err(_) => {
        bail!(
          "Could not parse the domain zone record payment address {}",
          &zone_record.payment_account
        );
      }
    };
    let mut versions = HashMap::new();
    for (version, host) in zone_record.versions {
      let datastore_id = match String::from_utf8(host.datastore_id.0) {
        Ok(s) => s,
        Err(_) => {
          bail!("Could not parse datastore_id bytes into string");
        }
      };

      let prim_host: Host = host.host.0 .0.into();
      let host_string: String = prim_host.try_into()?;
      versions.insert(
        format!("{}.{}.{}", version.major, version.minor, version.patch),
        VersionHost {
          datastore_id,
          host: host_string,
        },
      );
    }

    Ok(Some(ZoneRecord {
      payment_address: AccountStore::to_address(&payment_address),
      notary_id: zone_record.notary_id,
      versions,
    }))
  }

  pub async fn get_notary_details(&self, notary_id: u32) -> Result<Option<NotaryDetails>> {
    let notaries = self
      .fetch_storage(&storage().notaries().active_notaries(), None)
      .await?
      .ok_or_else(|| anyhow!("No notaries found"))?;
    let notary = notaries.0.into_iter().find_map(|n| {
      if n.notary_id == notary_id {
        return Some(n);
      }
      None
    });
    let Some(notary) = notary else {
      return Ok(None);
    };

    let hosts: anyhow::Result<Vec<String>, _> = notary
      .meta
      .hosts
      .0
      .into_iter()
      .map(|h| Host::from(h.0 .0).try_into())
      .collect();
    let notary = NotaryDetails {
      id: notary.notary_id,
      hosts: hosts?,
      public_key: notary.meta.public.into(),
    };

    Ok(Some(notary))
  }

  pub async fn get_account(&self, address: String) -> Result<AccountInfo> {
    let account_id32 = AccountId32::from_str(&address).map_err(|e| anyhow!(e))?;
    let client = self.client().await?;
    let info = client
      .get_account(&account_id32)
      .await
      .map_err(|_| anyhow!("No account found for address {address}"))?;
    Ok(AccountInfo {
      nonce: info.nonce,
      consumers: info.consumers,
      providers: info.providers,
      sufficients: info.sufficients,
      data: BalancesAccountData {
        free: info.data.free,
        reserved: info.data.reserved,
        frozen: info.data.frozen,
        flags: info.data.flags.0,
      },
    })
  }

  pub async fn get_ownership(&self, address: String) -> Result<BalancesAccountData> {
    let account_id32 = AccountId32::from_str(&address).map_err(|e| anyhow!(e))?;
    let client = self.client().await?;
    let balance = client
      .get_ownership(&account_id32, FetchAt::Finalized)
      .await
      .map_err(|_| anyhow!("No account found for address {address}"))?;
    Ok(BalancesAccountData {
      free: balance.free,
      reserved: balance.reserved,
      frozen: balance.frozen,
      flags: balance.flags.0,
    })
  }

  pub async fn get_account_nonce(&self, address: String) -> Result<u32> {
    let client = self.client().await?;
    let account_id32 = AccountId32::from_str(&address).map_err(|e| anyhow!(e))?;
    let nonce = client.get_account_nonce(&account_id32).await?;
    Ok(nonce as u32)
  }

  pub async fn get_transfer_to_localchain_finalized_block(
    &self,
    transfer_id: TransferToLocalchainId,
  ) -> Result<Option<u32>> {
    let Ok(Some(_)) = self
      .fetch_storage(
        &storage()
          .chain_transfer()
          .pending_transfers_out(transfer_id),
        None,
      )
      .await
    else {
      return Ok(None);
    };

    let finalized = self.latest_finalized_number().await?;
    Ok(Some(finalized))
  }

  pub async fn create_transfer_to_localchain(
    &self,
    address: String,
    amount: u128,
    notary_id: u32,
    keystore: &Keystore,
  ) -> Result<(LocalchainTransfer, TxInBlockWithEvents)> {
    let current_nonce = self.get_account_nonce(address.clone()).await?;
    let mortality = 50; // artibrary number of blocks to keep this tx alive

    let client = self.client().await?;

    let account_id = AccountId32::from_str(&address).map_err(|e| anyhow!(e))?;
    let account_bytes: [u8; 32] = account_id.clone().into();

    let payload = {
      let params = ArgonExtrinsicParamsBuilder::<ArgonConfig>::new()
        .nonce(current_nonce as u64)
        .mortal(mortality)
        .build();
      let tx_tmp = client.live.tx().create_partial_offline(
        &tx().chain_transfer().send_to_localchain(amount, notary_id),
        params,
      )?;
      tx_tmp.signer_payload()
    };

    let signature = keystore.sign(address.clone(), payload).await?;

    let multi_signature = MultiSignature::decode(&mut signature.as_ref())?;

    // have to recreate this because the internal types are not send. inefficient, but small penalty
    let submittable = {
      client
        .live
        .tx()
        .create_partial_offline(
          &tx().chain_transfer().send_to_localchain(amount, notary_id),
          ArgonExtrinsicParamsBuilder::<ArgonConfig>::new()
            .nonce(current_nonce as u64)
            .mortal(mortality)
            .build(),
        )?
        .sign_with_account_and_signature(&account_id, &multi_signature)
    };

    let tx_progress = submittable.submit_and_watch().await?;

    let in_block = InnerMainchainClient::wait_for_ext_in_block(tx_progress, false)
      .await
      .map_err(|e| anyhow!("Error submitting notebook to block: {:?}", e))?;

    let transfer = in_block.events.iter().find_map(|event| {
      if let Some(Ok(transfer)) = event
        .as_event::<api::chain_transfer::events::TransferToLocalchain>()
        .transpose()
      {
        if transfer.account_id.0 == account_bytes {
          return Some(transfer);
        }
      }
      None
    });
    let Some(transfer) = transfer else {
      bail!("No transfer event found for account {}", address);
    };

    Ok((
      LocalchainTransfer {
        address,
        amount,
        notary_id,
        expiration_tick: transfer.expiration_tick,
        transfer_id: transfer.transfer_id,
      },
      in_block,
    ))
  }

  pub async fn wait_for_localchain_transfer(
    &self,
    transfer_id: TransferToLocalchainId,
  ) -> Result<Option<LocalchainTransfer>> {
    if let Some(transfer) = self
      .fetch_storage(
        &storage()
          .chain_transfer()
          .pending_transfers_out(transfer_id),
        None,
      )
      .await?
    {
      return Ok(Some(LocalchainTransfer {
        address: transfer.account_id.to_address(),
        amount: transfer.amount,
        notary_id: transfer.notary_id,
        expiration_tick: transfer.expiration_tick,
        transfer_id,
      }));
    }

    let mut subscription = self
      .client()
      .await?
      .live
      .blocks()
      .subscribe_finalized()
      .await?;
    while let Some(block) = subscription.next().await {
      let Ok(block) = block else {
        continue;
      };

      let events = block.events().await?;
      for event in events.iter() {
        let Ok(event) = event else {
          continue;
        };
        if let Some(Ok(transfer)) = event
          .as_event::<api::chain_transfer::events::TransferToLocalchain>()
          .transpose()
        {
          if transfer.transfer_id == transfer_id {
            return Ok(Some(LocalchainTransfer {
              address: transfer.account_id.to_address(),
              amount: transfer.amount,
              notary_id: transfer.notary_id,
              expiration_tick: transfer.expiration_tick,
              transfer_id,
            }));
          }
        }
      }
    }
    Ok(None)
  }

  pub async fn get_latest_notebook(
    &self,
    notary_id: u32,
  ) -> Result<runtime_types::argon_primitives::notary::NotaryNotebookKeyDetails> {
    let best_block = self.get_best_block_hash().await?;
    if let Some((details, _did_receive_at_tick)) = self
      .fetch_storage(
        &storage()
          .notebook()
          .last_notebook_details_by_notary(notary_id),
        Some(best_block),
      )
      .await?
      .unwrap_or(BoundedVec(Vec::new()))
      .0
      .last()
    {
      return Ok(details.clone());
    }
    bail!("No immortalized notebook found for notary {}", notary_id)
  }

  pub async fn get_account_changes_root(
    &self,
    notary_id: u32,
    notebook_number: u32,
  ) -> Result<Option<H256>> {
    let result = self
      .fetch_storage(
        &storage()
          .notebook()
          .notebook_changed_accounts_root_by_notary(notary_id, notebook_number),
        None,
      )
      .await?
      .map(|a| a.into());

    Ok(result)
  }

  pub async fn latest_finalized_number(&self) -> Result<u32> {
    Ok(self.client().await?.latest_finalized_block().await?)
  }

  pub async fn wait_for_notebook_immortalized(
    &self,
    notary_id: NotaryId,
    notebook_number: NotebookNumber,
  ) -> Result<u32> {
    if let Ok(notebook_details) = self.get_latest_notebook(notary_id).await {
      if notebook_details.notebook_number >= notebook_number {
        return self.latest_finalized_number().await;
      }
    }

    let mut subscription = self
      .client()
      .await?
      .live
      .blocks()
      .subscribe_finalized()
      .await?;
    while let Some(block) = subscription.next().await {
      let Ok(block) = block else {
        continue;
      };
      let block_height = block.number();

      let events = block.events().await?;
      for event in events.iter() {
        let Ok(event) = event else {
          continue;
        };
        if let Some(Ok(notebook)) = event
          .as_event::<api::notebook::events::NotebookSubmitted>()
          .transpose()
        {
          if notebook.notary_id == notary_id && notebook.notebook_number >= notebook_number {
            return Ok(block_height);
          }
        }
      }
    }

    bail!(
      "No notebook submissions found for notary {} with notebook {}",
      notary_id,
      notebook_number
    )
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use napi::bindgen_prelude::*;

  use crate::error::NapiOk;
  use crate::{DomainRegistration, MainchainClient, ZoneRecord};
  use argon_primitives::tick::Tick;
  use argon_primitives::{Chain, DomainTopLevel};

  #[napi(object)]
  pub struct LocalchainTransfer {
    pub address: String,
    pub amount: BigInt,
    pub notary_id: u32,
    pub expiration_tick: i64,
    pub transfer_id: u32,
  }

  #[napi(object)]
  pub struct BalancesAccountData {
    pub free: BigInt,
    pub reserved: BigInt,
    pub frozen: BigInt,
    pub flags: BigInt,
  }

  #[napi(object)]
  pub struct ChainIdentity {
    pub chain: Chain,
    pub genesis: Buffer,
  }

  #[napi(object)]
  pub struct AccountInfo {
    pub nonce: u32,
    pub consumers: u32,
    pub providers: u32,
    pub sufficients: u32,
    pub data: BalancesAccountData,
  }

  #[napi(object)]
  pub struct NotaryDetails {
    pub id: u32,
    pub hosts: Vec<String>,
    pub public_key: Uint8Array,
  }

  #[napi(object)]
  pub struct BestBlockForVote {
    pub block_hash: Uint8Array,
    pub vote_minimum: BigInt,
  }

  #[napi(object)]
  pub struct Ticker {
    pub tick_duration_millis: i64,
    pub channel_hold_expiration_ticks: i64,
  }

  #[napi]
  impl MainchainClient {
    #[napi(js_name = "close")]
    pub async fn close_napi(&self) -> napi::Result<()> {
      self.close().await.napi_ok()
    }

    #[napi(factory, js_name = "connect")]
    pub async fn connect_napi(host: String, timeout_millis: i64) -> napi::Result<Self> {
      MainchainClient::connect(host, timeout_millis)
        .await
        .napi_ok()
    }

    #[napi(js_name = "getTicker")]
    pub async fn get_ticker_napi(&self) -> napi::Result<Ticker> {
      let ticker = self.get_ticker().await.napi_ok()?;
      Ok(Ticker {
        tick_duration_millis: ticker.tick_duration_millis as i64,
        channel_hold_expiration_ticks: ticker.channel_hold_expiration_ticks as i64,
      })
    }

    #[napi(js_name = "getChainIdentity")]
    pub async fn get_chain_identity_napi(&self) -> napi::Result<ChainIdentity> {
      let chain = self.get_chain_identity().await.napi_ok()?;
      Ok(ChainIdentity {
        chain: chain.chain,
        genesis: chain.genesis_hash.as_ref().into(),
      })
    }

    #[napi(js_name = "getBestBlockHash")]
    pub async fn get_best_block_hash_napi(&self) -> napi::Result<Uint8Array> {
      let hash = self.get_best_block_hash().await.napi_ok()?;
      Ok(hash.as_ref().into())
    }

    #[napi(js_name = "getVoteBlockHash")]
    pub async fn get_vote_block_hash_napi(
      &self,
      current_tick: i64,
    ) -> napi::Result<Option<BestBlockForVote>> {
      let best_block = self
        .get_vote_block_hash(current_tick as Tick)
        .await
        .napi_ok()?;
      let Some(best_block) = best_block else {
        return Ok(None);
      };
      Ok(Some(BestBlockForVote {
        block_hash: best_block.block_hash.as_ref().to_vec().into(),
        vote_minimum: best_block.vote_minimum.into(),
      }))
    }

    #[napi(js_name = "getDomainRegistration")]
    pub async fn get_domain_registration_napi(
      &self,
      domain_name: String,
      top_level: DomainTopLevel,
    ) -> napi::Result<Option<DomainRegistration>> {
      self
        .get_domain_registration(domain_name, top_level)
        .await
        .napi_ok()
    }

    #[napi(js_name = "getDomainZoneRecord")]
    pub async fn get_domain_zone_record_napi(
      &self,
      domain_name: String,
      top_level: DomainTopLevel,
    ) -> napi::Result<Option<ZoneRecord>> {
      self
        .get_domain_zone_record(domain_name, top_level)
        .await
        .napi_ok()
    }

    #[napi(js_name = "getNotaryDetails")]
    pub async fn get_notary_details_napi(
      &self,
      notary_id: u32,
    ) -> napi::Result<Option<NotaryDetails>> {
      let result = self
        .get_notary_details(notary_id)
        .await
        .napi_ok()?
        .map(|a| NotaryDetails {
          id: a.id,
          hosts: a.hosts,
          public_key: a.public_key.into(),
        });
      Ok(result)
    }

    #[napi(js_name = "getAccount")]
    pub async fn get_account_napi(&self, address: String) -> napi::Result<AccountInfo> {
      let account = self.get_account(address).await.napi_ok()?;
      Ok(AccountInfo {
        nonce: account.nonce,
        consumers: account.consumers,
        providers: account.providers,
        sufficients: account.sufficients,
        data: BalancesAccountData {
          free: account.data.free.into(),
          reserved: account.data.reserved.into(),
          frozen: account.data.frozen.into(),
          flags: account.data.flags.into(),
        },
      })
    }

    #[napi(js_name = "getOwnership")]
    pub async fn get_ownership_napi(&self, address: String) -> napi::Result<BalancesAccountData> {
      let account = self.get_ownership(address).await.napi_ok()?;
      Ok(BalancesAccountData {
        free: account.free.into(),
        reserved: account.reserved.into(),
        frozen: account.frozen.into(),
        flags: account.flags.into(),
      })
    }

    #[napi(js_name = "getTransferToLocalchainFinalizedBlock")]
    pub async fn get_transfer_to_localchain_finalized_block_napi(
      &self,
      transfer_id: u32,
    ) -> napi::Result<Option<u32>> {
      self
        .get_transfer_to_localchain_finalized_block(transfer_id)
        .await
        .napi_ok()
    }

    #[napi(js_name = "waitForLocalchainTransfer")]
    pub async fn wait_for_localchain_transfer_napi(
      &self,
      transfer_id: u32,
    ) -> napi::Result<Option<LocalchainTransfer>> {
      let result = self
        .wait_for_localchain_transfer(transfer_id)
        .await
        .napi_ok()?;
      let Some(result) = result else {
        return Ok(None);
      };

      Ok(Some(LocalchainTransfer {
        address: result.address,
        amount: result.amount.into(),
        notary_id: result.notary_id,
        expiration_tick: result.expiration_tick as i64,
        transfer_id,
      }))
    }

    #[napi(js_name = "getAccountChangesRoot")]
    pub async fn get_account_changes_root_napi(
      &self,
      notary_id: u32,
      notebook_number: u32,
    ) -> napi::Result<Option<Uint8Array>> {
      self
        .get_account_changes_root(notary_id, notebook_number)
        .await
        .map(|a| a.map(|x| x.as_ref().into()))
        .napi_ok()
    }

    #[napi(js_name = "latestFinalizedNumber")]
    pub async fn latest_finalized_number_napi(&self) -> napi::Result<u32> {
      self.latest_finalized_number().await.napi_ok()
    }

    #[napi(js_name = "waitForNotebookImmortalized")]
    pub async fn wait_for_notebook_immortalized_napi(
      &self,
      notary_id: u32,
      notebook_number: u32,
    ) -> napi::Result<u32> {
      self
        .wait_for_notebook_immortalized(notary_id, notebook_number)
        .await
        .napi_ok()
    }
  }
}

pub struct LocalchainTransfer {
  pub address: String,
  pub amount: Balance,
  pub notary_id: NotaryId,
  pub expiration_tick: Tick,
  pub transfer_id: TransferToLocalchainId,
}

pub struct BalancesAccountData {
  pub free: Balance,
  pub reserved: Balance,
  pub frozen: Balance,
  pub flags: Balance,
}

pub struct AccountInfo {
  pub nonce: u32,
  pub consumers: u32,
  pub providers: u32,
  pub sufficients: u32,
  pub data: BalancesAccountData,
}

#[cfg_attr(feature = "napi", napi(object))]
pub struct ZoneRecord {
  pub payment_address: String,
  pub notary_id: u32,
  /// A mapping of versions to host addresses.
  pub versions: HashMap<String, VersionHost>,
}

#[cfg_attr(feature = "napi", napi(object))]
pub struct VersionHost {
  /// Datastore id is a 2-50 char string that uniquely identifies a domain.
  pub datastore_id: String,
  /// The host address where the domain can be accessed.
  pub host: String,
}

pub struct NotaryDetails {
  pub id: u32,
  pub hosts: Vec<String>,
  pub public_key: Vec<u8>,
}

#[cfg_attr(feature = "napi", napi(object))]
pub struct DomainRegistration {
  pub registered_to_address: String,
  pub registered_at_tick: i64,
}

pub struct BestBlockForVote {
  pub block_hash: H256,
  pub vote_minimum: Balance,
}
