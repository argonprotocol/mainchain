use anyhow::anyhow;
use argon_notary_apis::localchain::{BalanceChangeResult, BalanceTipResult};
use argon_notary_apis::LocalchainRpcClient;
use argon_notary_apis::NotebookRpcClient;
use argon_primitives::{
  AccountId, AccountOrigin, AccountType, BalanceProof, BalanceTip, Notarization, NotebookNumber,
  SignedNotebookHeader,
};
use futures::stream::TryStreamExt;
use futures::StreamExt;
use sp_core::ed25519;
use sp_runtime::traits::Verify;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::mainchain_client::MainchainClient;
use crate::{bail, AccountStore, Error, Result};

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone)]
pub struct NotaryClients {
  clients_by_id: Arc<Mutex<HashMap<u32, NotaryClient>>>,
  mainchain_client: Arc<Mutex<Option<MainchainClient>>>,
}

impl NotaryClients {
  pub fn new(mainchain_client: &MainchainClient) -> Self {
    Self {
      clients_by_id: Arc::new(Mutex::new(HashMap::new())),
      mainchain_client: Arc::new(Mutex::new(Some(mainchain_client.clone()))),
    }
  }

  pub fn from(mainchain_client: Arc<Mutex<Option<MainchainClient>>>) -> Self {
    Self {
      clients_by_id: Arc::new(Mutex::new(HashMap::new())),
      mainchain_client,
    }
  }

  pub async fn close(&self) {
    let mut clients_by_id = self.clients_by_id.lock().await;
    for (_, client) in clients_by_id.drain() {
      drop(client);
    }
  }

  pub async fn attach_mainchain(&self, mainchain_client: Option<MainchainClient>) {
    let mut mainchain_client_ref = self.mainchain_client.lock().await;
    *mainchain_client_ref = mainchain_client;
  }

  pub async fn use_client(&self, client: &NotaryClient) {
    let mut clients_by_id = self.clients_by_id.lock().await;
    clients_by_id.insert(client.notary_id, client.clone());
  }

  pub async fn get(&self, notary_id: u32) -> Result<NotaryClient> {
    let mut clients_by_id = self.clients_by_id.lock().await;
    if let Some(client) = clients_by_id.get(&notary_id) {
      if client.is_connected().await {
        return Ok(client.clone());
      }
    }

    let mainchain_mutex = self.mainchain_client.lock().await;

    let Some(ref mainchain_client) = *mainchain_mutex else {
      bail!("Mainchain client not set");
    };

    let Some(notary_details) = mainchain_client.get_notary_details(notary_id).await? else {
      bail!("Notary {} not found", notary_id);
    };

    let client = NotaryClient::connect(
      notary_id,
      notary_details.public_key,
      notary_details.hosts[0].clone(),
      true,
    )
    .await?;
    clients_by_id.insert(notary_id, client.clone());
    Ok(client)
  }
}

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone)]
pub struct NotaryClient {
  pub notary_id: u32,
  public: ed25519::Public,
  client: Arc<Mutex<argon_notary_apis::Client>>,
  last_metadata: Arc<Mutex<Option<argon_primitives::NotebookMeta>>>,
  pub auto_verify_header_signatures: bool,
}

impl NotaryClient {
  pub async fn is_connected(&self) -> bool {
    let client = self.client.lock().await;
    (*client).is_connected()
  }

  pub async fn connect(
    notary_id: u32,
    public_key: Vec<u8>,
    host: String,
    auto_verify_header_signatures: bool,
  ) -> Result<Self> {
    let public: [u8; 32] = public_key
      .try_into()
      .map_err(|_| anyhow!("Unable to parse the notary public key"))?;
    Ok(Self {
      notary_id,
      public: ed25519::Public::from_raw(public),
      auto_verify_header_signatures,
      last_metadata: Arc::new(Mutex::new(None)),
      client: Arc::new(Mutex::new(argon_notary_apis::create_client(&host).await?)),
    })
  }

  pub async fn get_notarization(
    &self,
    account_id32: AccountId,
    account_type: AccountType,
    notebook_number: NotebookNumber,
    change_number: u32,
  ) -> Result<Notarization> {
    let client = self.client.lock().await;
    let res = (*client)
      .get_notarization(account_id32, account_type, notebook_number, change_number)
      .await?;

    Ok(res)
  }

  pub async fn get_account_origin(
    &self,
    address: String,
    account_type: AccountType,
  ) -> Result<AccountOrigin> {
    let client = self.client.lock().await;
    let account_id = AccountStore::parse_address(&address)?;
    let res = (*client).get_origin(account_id, account_type).await?;

    Ok(res)
  }

  pub async fn get_balance_tip(
    &self,
    address: String,
    account_type: AccountType,
  ) -> Result<BalanceTipResult> {
    let client = self.client.lock().await;
    let account_id = AccountStore::parse_address(&address)?;
    let res = (*client).get_tip(account_id, account_type).await?;

    Ok(res)
  }

  pub async fn notarize(&self, notarization: Notarization) -> Result<BalanceChangeResult> {
    let json = serde_json::to_string_pretty(&notarization).unwrap();
    for i in 0..5 {
      let client = self.client.lock().await;

      let res = (*client)
        .notarize(
          notarization.balance_changes.clone(),
          notarization.block_votes.clone(),
          notarization.domains.clone(),
        )
        .await;

      return match res {
        Ok(x) => Ok(x),
        Err(e) => {
          let e: Error = e.into();
          // this error only occurs mid-processing of the notebook, but we don't want to try forever
          if matches!(
            e,
            Error::NotaryApiError(argon_notary_apis::Error::NotebookNotFinalized)
          ) {
            tokio::time::sleep(std::time::Duration::from_secs(
              1 + (1.5f32.powf(i as f32)) as u64,
            ))
            .await;
            continue;
          }
          tracing::error!("Error sending notarization: {:?} {}", e, json);
          Err(e)
        }
      };
    }
    bail!("Failed to send notarization")
  }

  pub async fn metadata(&self) -> Result<NotebookMeta> {
    let client = self.client.lock().await;
    let meta = (*client).metadata().await?;

    *self.last_metadata.lock().await = Some(meta.clone());
    Ok(NotebookMeta {
      finalized_tick: meta.finalized_tick,
      finalized_notebook_number: meta.finalized_notebook_number,
    })
  }

  pub async fn get_balance_proof(
    &self,
    notebook_number: NotebookNumber,
    tip: BalanceTip,
  ) -> Result<BalanceProof> {
    let client = self.client.lock().await;

    let proof = (*client).get_balance_proof(notebook_number, tip).await?;
    Ok(proof)
  }

  pub fn verify_header(&self, header: &SignedNotebookHeader) -> Result<()> {
    if !header
      .signature
      .verify(header.header.hash().as_bytes(), &self.public)
    {
      bail!("This header has an invalid signature");
    }
    Ok(())
  }

  pub async fn wait_for_notebook(&self, notebook_number: u32) -> Result<SignedNotebookHeader> {
    let mut has_seen_notebook = {
      let last_metadata = self.last_metadata.lock().await;
      if let Some(meta) = &*last_metadata {
        meta.finalized_notebook_number >= notebook_number
      } else {
        false
      }
    };
    if !has_seen_notebook {
      let meta = self.metadata().await?;
      has_seen_notebook = meta.finalized_notebook_number >= notebook_number;
    }

    if has_seen_notebook {
      let client = self.client.lock().await;
      let header = (*client).get_header(notebook_number).await?;
      if self.auto_verify_header_signatures {
        self.verify_header(&header)?;
      }
      return Ok(header);
    }

    let mut subscription_stream = {
      let client = self.client.lock().await;
      let subscription = (*client).subscribe_headers().await?;
      subscription.into_stream()
    };
    while let Some(header) = subscription_stream.next().await {
      let header = header?;
      let mut last_metadata = self.last_metadata.lock().await;
      *last_metadata = Some(argon_primitives::NotebookMeta {
        finalized_tick: header.header.tick,
        finalized_notebook_number: header.header.notebook_number,
      });
      if header.header.notebook_number == notebook_number {
        if self.auto_verify_header_signatures {
          self.verify_header(&header)?;
        }
        return Ok(header);
      }
    }
    bail!("No header found")
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use crate::error::NapiOk;
  use crate::{MainchainClient, NotebookMeta};
  use argon_primitives::AccountType;
  use napi::bindgen_prelude::*;

  use super::NotaryClient;
  use super::NotaryClients;

  #[napi]
  pub struct BalanceTipResult {
    pub balance_tip: Uint8Array,
    pub notebook_number: u32,
    pub tick: u32,
  }

  #[napi]
  impl NotaryClients {
    #[napi(factory, js_name = "new")]
    pub fn new_napi(mainchain_client: &MainchainClient) -> NotaryClients {
      NotaryClients::new(mainchain_client)
    }

    #[napi(js_name = "close")]
    pub async fn close_napi(&self) {
      self.close().await;
    }

    #[napi(js_name = "useClient")]
    pub async fn use_client_napi(&self, client: &NotaryClient) {
      self.use_client(client).await;
    }

    #[napi(js_name = "get")]
    pub async fn get_napi(&self, notary_id: u32) -> napi::Result<NotaryClient> {
      self.get(notary_id).await.napi_ok()
    }
  }
  #[napi]
  impl NotaryClient {
    #[napi(js_name = "isConnected")]
    pub async fn is_connected_napi(&self) -> bool {
      self.is_connected().await
    }

    #[napi(factory, js_name = "connect")]
    pub async fn connect_napi(
      notary_id: u32,
      public_key: Uint8Array,
      host: String,
      auto_verify_header_signatures: bool,
    ) -> napi::Result<NotaryClient> {
      NotaryClient::connect(
        notary_id,
        public_key.as_ref().to_vec(),
        host,
        auto_verify_header_signatures,
      )
      .await
      .napi_ok()
    }

    #[napi(js_name = "getBalanceTip")]
    pub async fn get_balance_tip_napi(
      &self,
      address: String,
      account_type: AccountType,
    ) -> napi::Result<BalanceTipResult> {
      let result = self
        .get_balance_tip(address, account_type)
        .await
        .napi_ok()?;
      Ok(BalanceTipResult {
        balance_tip: result.balance_tip.as_ref().to_vec().into(),
        notebook_number: result.notebook_number,
        tick: result.tick,
      })
    }

    #[napi(getter, js_name = "metadata")]
    pub async fn get_metadata_napi(&self) -> napi::Result<NotebookMeta> {
      self.metadata().await.napi_ok()
    }
  }
}

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Clone)]
pub struct NotebookMeta {
  pub finalized_notebook_number: u32,
  pub finalized_tick: u32,
}
