use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use crate::to_js_error;
use crate::AccountStore;
use codec::Decode;
use codec::Encode;
use napi::bindgen_prelude::*;
use sp_core::crypto::AccountId32;
use sp_core::{ByteArray, H256};
use subxt::error::RpcError;
use subxt::runtime_api::RuntimeApiPayload;
use subxt::storage::address::Yes;
use subxt::storage::StorageAddress;
use tokio::sync::Mutex;
use tracing::warn;
use ulixee_client::api;
use ulixee_client::api::runtime_types;
use ulixee_client::api::storage;
use ulixee_client::UlxFullclient;
use ulx_primitives::host::Host;
use ulx_primitives::tick::Ticker;
use ulx_primitives::DataTLD;
use ulx_primitives::MAX_DOMAIN_NAME_LENGTH;

#[napi]
#[derive(Clone)]
pub struct MainchainClient {
  client: Arc<Mutex<Option<UlxFullclient>>>,
  host: String,
}

#[cfg(test)]
impl MainchainClient {
  pub fn mock() -> Self {
    Self {
      host: "mock".to_string(),
      client: Arc::new(Mutex::new(None)),
    }
  }
}

#[napi]
impl MainchainClient {
  async fn client(&self) -> Result<UlxFullclient> {
    self.ensure_connected(10_000).await?;
    let client_lock = self.client.lock().await;
    let client_rpc = (*client_lock).as_ref().ok_or_else(|| {
      to_js_error(format!(
        "Could not connect to mainchain client at {}",
        self.host
      ))
    })?;
    Ok(client_rpc.clone())
  }

  async fn ensure_connected(&self, timeout_millis: i64) -> Result<()> {
    let mut client_lock = self.client.lock().await;
    if (*client_lock).is_some() {
      return Ok(());
    }

    let client = UlxFullclient::try_until_connected(self.host.clone(), timeout_millis as u64)
      .await
      .map_err(to_js_error)?;
    let ws_client = client.ws_client.clone();

    *client_lock = Some(client);
    drop(client_lock);

    let client_lock = self.client.clone();
    let url = self.host.clone();
    tokio::spawn(async move {
      let client_lock = client_lock.clone();
      let _ = ws_client.on_disconnect().await;

      warn!("Disconnected from mainchain at {url}",);
      *client_lock.lock().await = None;
    });

    Ok(())
  }

  #[napi]
  pub async fn close(&self) -> Result<()> {
    let mut client_lock = self.client.lock().await;
    if let Some(client) = (*client_lock).take() {
      drop(client);
    }
    Ok(())
  }

  #[napi(factory)]
  pub async fn connect(host: String, timeout_millis: i64) -> Result<Self> {
    let instance = Self {
      host,
      client: Arc::new(Mutex::new(None)),
    };
    instance.ensure_connected(timeout_millis).await?;
    Ok(instance)
  }

  pub async fn call<Call: RuntimeApiPayload>(&self, payload: Call) -> Result<Call::ReturnType> {
    let client = self.client().await?;

    let api = client
      .live
      .runtime_api()
      .at_latest()
      .await
      .map_err(to_js_error)?;
    match api.call(payload).await {
      Ok(result) => Ok(result),
      Err(e) => {
        match e {
          subxt::Error::Rpc(ref rpc_error) => match rpc_error {
            RpcError::ClientError(_) => {
              *(self.client.lock().await) = None;
            }
            _ => {}
          },
          _ => {}
        }
        Err(to_js_error(e))
      }
    }
  }

  pub async fn fetch_storage<Address>(
    &self,
    address: &Address,
    at: Option<H256>,
  ) -> Result<Option<Address::Target>>
  where
    Address: StorageAddress<IsFetchable = Yes>,
  {
    let client = self.client().await?;
    let client = client.live;
    let storage = match at {
      Some(at) => client.storage().at(at),
      None => client.storage().at_latest().await.map_err(to_js_error)?,
    };

    match storage.fetch(address).await {
      Ok(result) => Ok(result),
      Err(e) => {
        match e {
          subxt::Error::Rpc(ref rpc_error) => match rpc_error {
            RpcError::ClientError(_) => {
              *(self.client.lock().await) = None;
            }
            _ => {}
          },
          _ => {}
        }
        Err(to_js_error(e))
      }
    }
  }

  pub async fn get_ticker(&self) -> Result<ulx_primitives::tick::Ticker> {
    let ticker = self
      .call(api::runtime_apis::tick_apis::TickApis.ticker())
      .await?;

    Ok(Ticker::new(
      ticker.tick_duration_millis,
      ticker.genesis_utc_time,
    ))
  }

  #[napi]
  pub async fn get_best_block_hash(&self) -> Result<Uint8Array> {
    let best_block_hash = &self
      .client()
      .await?
      .methods
      .chain_get_block_hash(None)
      .await
      .map_err(to_js_error)?
      .ok_or_else(|| to_js_error(format!("No best block found")))?;
    Ok(Uint8Array::from(best_block_hash.as_bytes()))
  }

  #[napi]
  pub async fn get_vote_block_hash(&self, current_tick: u32) -> Result<Option<BestBlockForVote>> {
    let best_hash = &self.get_best_block_hash().await?;
    let best_hash_bytes = best_hash.as_ref();
    let grandparent_tick = current_tick - 2;
    let best_votes = self
      .fetch_storage(
        &api::ticks::storage::StorageApi.recent_blocks_at_ticks(grandparent_tick),
        Some(H256::from_slice(&best_hash_bytes)),
      )
      .await?
      .ok_or_else(|| to_js_error(format!("No best block found")))?
      .0;

    let Some(best_vote_block) = best_votes.last() else {
      return Ok(None);
    };

    let minimum = self
      .fetch_storage(
        &api::block_seal_spec::storage::StorageApi.current_vote_minimum(),
        Some(H256::from_slice(&best_hash_bytes)),
      )
      .await?
      .ok_or_else(|| to_js_error(format!("No minimum vote requirement found")))?;

    Ok(Some(BestBlockForVote {
      block_hash: Uint8Array::from(best_vote_block.0.to_vec()),
      vote_minimum: BigInt::from(minimum),
    }))
  }

  #[napi]
  pub async fn get_data_domain_registration(
    &self,
    domain_name: String,
    tld: DataTLD,
  ) -> Result<Option<DataDomainRegistration>> {
    let bytes = domain_name.as_bytes().to_vec();
    if bytes.len() > MAX_DOMAIN_NAME_LENGTH as usize {
      return Err(to_js_error(format!(
        "Domain name {} is too long",
        domain_name
      )));
    }

    let domain = runtime_types::ulx_primitives::data_domain::DataDomain {
      domain_name: runtime_types::bounded_collections::bounded_vec::BoundedVec(bytes),
      top_level_domain: runtime_types::ulx_primitives::data_tld::DataTLD::decode(
        &mut tld.encode().as_slice(),
      )
      .map_err(to_js_error)?,
    };

    let best_block_hash = &self.get_best_block_hash().await?.to_vec();
    if let Some(x) = self
      .fetch_storage(
        &storage().data_domain().registered_data_domains(domain),
        Some(H256::from_slice(&best_block_hash)),
      )
      .await?
    {
      let registered_to_address = match AccountId32::from_slice(&x.account_id.0) {
        Ok(s) => AccountStore::to_address(&s),
        Err(_) => {
          return Err(to_js_error(format!(
            "Could not parse the data domain registration address {}",
            &x.account_id
          )))
        }
      };
      Ok(Some(DataDomainRegistration {
        registered_to_address,
        registered_at_tick: x.registered_at_tick,
      }))
    } else {
      Ok(None)
    }
  }

  #[napi]
  pub async fn get_data_domain_zone_record(
    &self,
    domain_name: String,
    tld: DataTLD,
  ) -> Result<Option<ZoneRecord>> {
    let domain = runtime_types::ulx_primitives::data_domain::DataDomain {
      domain_name: runtime_types::bounded_collections::bounded_vec::BoundedVec(
        domain_name.into_bytes(),
      ),
      top_level_domain: runtime_types::ulx_primitives::data_tld::DataTLD::decode(
        &mut tld.encode().as_slice(),
      )
      .map_err(to_js_error)?,
    };
    let Some(zone_record) = self
      .fetch_storage(
        &storage().data_domain().zone_records_by_domain(domain),
        None,
      )
      .await?
    else {
      return Ok(None);
    };

    let payment_address = match AccountId32::from_slice(&zone_record.payment_account.0) {
      Ok(s) => s,
      Err(_) => {
        return Err(to_js_error(format!(
          "Could not parse the data domain zone record payment address {}",
          &zone_record.payment_account
        )))
      }
    };
    let mut versions = HashMap::new();
    for (version, host) in zone_record.versions {
      let host_url = Host {
        is_secure: host.host.is_secure,
        ip: host.host.ip,
        port: host.host.port,
      }
      .get_url();

      let datastore_id = match String::from_utf8(host.datastore_id.0) {
        Ok(s) => s,
        Err(_) => {
          return Err(to_js_error(format!(
            "Could not parse datastore_id bytes into string"
          )))
        }
      };

      versions.insert(
        format!("{}.{}.{}", version.major, version.minor, version.patch),
        VersionHost {
          datastore_id,
          host: host_url,
        },
      );
    }

    Ok(Some(ZoneRecord {
      payment_address: AccountStore::to_address(&payment_address),
      notary_id: zone_record.notary_id,
      versions,
    }))
  }

  #[napi]
  pub async fn get_notary_details(&self, notary_id: u32) -> Result<Option<NotaryDetails>> {
    let notaries = self
      .fetch_storage(&storage().notaries().active_notaries(), None)
      .await?
      .ok_or_else(|| to_js_error("No notaries found"))?;
    let notary = notaries.0.into_iter().find_map(|n| {
      if n.notary_id == notary_id {
        return Some(NotaryDetails {
          id: n.notary_id,
          hosts: n
            .meta
            .hosts
            .0
            .into_iter()
            .map(|h| Host::format_url(h.is_secure, h.ip, h.port))
            .collect::<Vec<_>>(),
          public_key: Uint8Array::from(n.meta.public.0.to_vec()),
        });
      }
      None
    });

    Ok(notary)
  }

  #[napi]
  pub async fn get_account(&self, address: String) -> Result<AccountInfo> {
    let account_id32 = subxt::utils::AccountId32::from_str(&address).map_err(to_js_error)?;
    let info = self
      .fetch_storage(&storage().system().account(account_id32), None)
      .await?
      .ok_or_else(|| to_js_error(format!("No account found for address {}", address)))?;
    Ok(AccountInfo {
      nonce: info.nonce,
      consumers: info.consumers,
      providers: info.providers,
      sufficients: info.sufficients,
      data: ArgonBalancesAccountData {
        free: BigInt::from(info.data.free),
        reserved: BigInt::from(info.data.reserved),
        frozen: BigInt::from(info.data.frozen),
        flags: BigInt::from(info.data.flags.0),
      },
    })
  }

  #[napi]
  pub async fn get_account_nonce(&self, address: String) -> Result<u32> {
    let account_id32 = subxt::utils::AccountId32::from_str(&address).map_err(to_js_error)?;
    let nonce = self
      .fetch_storage(&storage().system().account(account_id32), None)
      .await?
      .ok_or_else(|| to_js_error(format!("No account found for address {}", address)))?
      .nonce;
    Ok(nonce)
  }

  #[napi]
  pub async fn wait_for_localchain_transfer(
    &self,
    address: String,
    nonce: u32,
  ) -> Result<Option<LocalchainTransfer>> {
    let account_id32 = subxt::utils::AccountId32::from_str(&address).map_err(to_js_error)?;
    if let Some(transfer) = self
      .fetch_storage(
        &storage()
          .chain_transfer()
          .pending_transfers_out(account_id32.clone(), nonce),
        None,
      )
      .await
      .map_err(to_js_error)?
    {
      return Ok(Some(LocalchainTransfer {
        address,
        amount: BigInt::from(transfer.amount),
        notary_id: transfer.notary_id,
        expiration_block: transfer.expiration_block,
        account_nonce: nonce,
      }));
    }

    if self.get_account_nonce(address.clone()).await? >= nonce {
      return Ok(None);
    }

    let mut subscription = self
      .client()
      .await?
      .live
      .blocks()
      .subscribe_finalized()
      .await
      .map_err(to_js_error)?;
    while let Some(block) = subscription.next().await {
      let Ok(block) = block else {
        continue;
      };

      let events = block.events().await.map_err(to_js_error)?;
      for event in events.iter() {
        let Ok(event) = event else {
          continue;
        };
        if let Some(Ok(transfer)) = event
          .as_event::<api::chain_transfer::events::TransferToLocalchain>()
          .transpose()
        {
          if transfer.account_id == account_id32 && transfer.account_nonce == nonce {
            return Ok(Some(LocalchainTransfer {
              address,
              amount: BigInt::from(transfer.amount),
              notary_id: transfer.notary_id,
              expiration_block: transfer.expiration_block,
              account_nonce: nonce,
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
  ) -> Result<runtime_types::ulx_primitives::notary::NotaryNotebookKeyDetails> {
    if let Some((details, _did_receive_at_tick)) = self
      .fetch_storage(
        &storage()
          .notebook()
          .last_notebook_details_by_notary(&notary_id),
        None,
      )
      .await?
      .ok_or_else(|| to_js_error(format!("No notebook found for notary {}", notary_id)))?
      .0
      .last()
    {
      return Ok(details.clone());
    }
    Err(to_js_error(format!(
      "No notebook found for notary {}",
      notary_id
    )))
  }

  #[napi]
  pub async fn get_account_changes_root(
    &self,
    notary_id: u32,
    notebook_number: u32,
  ) -> Result<Uint8Array> {
    let result = self
      .fetch_storage(
        &storage()
          .notebook()
          .notebook_changed_accounts_root_by_notary(&notary_id, &notebook_number),
        None,
      )
      .await?
      .map(|a| Uint8Array::from(a.as_bytes()));

    result.ok_or_else(|| {
      to_js_error(format!(
        "No submitted notebook found for notary {} with notebook {}",
        notary_id, notebook_number
      ))
    })
  }

  #[napi]
  pub async fn latest_finalized_number(&self) -> Result<u32> {
    let block_number = self
      .fetch_storage(&storage().system().number(), None)
      .await?
      .unwrap_or_default();
    Ok(block_number)
  }

  #[napi]
  pub async fn wait_for_notebook_finalized(
    &self,
    notary_id: u32,
    notebook_number: u32,
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
      .await
      .map_err(to_js_error)?;
    while let Some(block) = subscription.next().await {
      let Ok(block) = block else {
        continue;
      };
      let block_height = block.number();

      let events = block.events().await.map_err(to_js_error)?;
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

    Err(to_js_error(format!(
      "No notebook submissions found for notary {} with notebook {}",
      notary_id, notebook_number
    )))
  }
}

#[napi(object)]
pub struct LocalchainTransfer {
  pub address: String,
  pub amount: BigInt,
  pub notary_id: u32,
  pub expiration_block: u32,
  pub account_nonce: u32,
}
#[napi(object)]
pub struct AccountInfo {
  pub nonce: u32,
  pub consumers: u32,
  pub providers: u32,
  pub sufficients: u32,
  pub data: ArgonBalancesAccountData,
}

#[napi(object)]
pub struct ArgonBalancesAccountData {
  pub free: BigInt,
  pub reserved: BigInt,
  pub frozen: BigInt,
  pub flags: BigInt,
}
#[napi(object)]
pub struct ZoneRecord {
  pub payment_address: String,
  pub notary_id: u32,
  /// A mapping of versions to host addresses.
  pub versions: HashMap<String, VersionHost>,
}

#[napi(object)]
pub struct VersionHost {
  /// Datastore id is a 2-50 char string that uniquely identifies a data domain.
  pub datastore_id: String,
  /// The host address where the data domain can be accessed.
  pub host: String,
}
#[napi(object)]
pub struct NotaryDetails {
  pub id: u32,
  pub hosts: Vec<String>,
  pub public_key: Uint8Array,
}

#[napi(object)]
pub struct DataDomainRegistration {
  pub registered_to_address: String,
  pub registered_at_tick: u32,
}

#[napi(object)]
pub struct BestBlockForVote {
  pub block_hash: Uint8Array,
  pub vote_minimum: BigInt,
}
