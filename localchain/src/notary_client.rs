use crate::mainchain_client::MainchainClient;
use crate::to_js_error;
use futures::stream::TryStreamExt;
use futures::StreamExt;
use napi::bindgen_prelude::*;
use sp_core::crypto::Ss58Codec;
use sp_core::ed25519;
use sp_runtime::traits::Verify;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use ulx_notary::apis::localchain::BalanceChangeResult;
use ulx_notary::apis::LocalchainRpcClient;
use ulx_notary::apis::NotebookRpcClient;
use ulx_primitives::{
  AccountId, AccountType, BalanceProof, BalanceTip, Notarization, NotebookNumber,
  SignedNotebookHeader,
};

#[napi]
#[derive(Clone)]
pub struct NotaryClients {
  clients_by_id: Arc<Mutex<HashMap<u32, NotaryClient>>>,
  mainchain_client: MainchainClient,
}

#[napi]
impl NotaryClients {
  #[napi(factory)]
  pub fn new(mainchain_client: &MainchainClient) -> Self {
    Self {
      clients_by_id: Arc::new(Mutex::new(HashMap::new())),
      mainchain_client: mainchain_client.clone(),
    }
  }

  pub async fn close(&self) {
    let mut clients_by_id = self.clients_by_id.lock().await;
    for (_, client) in clients_by_id.drain() {
      drop(client);
    }
  }

  #[napi]
  pub async fn use_client(&self, client: &NotaryClient) {
    let mut clients_by_id = self.clients_by_id.lock().await;
    clients_by_id.insert(client.notary_id, client.clone());
  }

  #[napi]
  pub async fn get(&self, notary_id: u32) -> Result<NotaryClient> {
    let mut clients_by_id = self.clients_by_id.lock().await;
    if let Some(client) = clients_by_id.get(&notary_id) {
      if client.is_connected().await {
        return Ok(client.clone());
      }
    }

    let Some(notary_details) = self
      .mainchain_client
      .get_notary_details(notary_id)
      .await
      .map_err(to_js_error)?
    else {
      return Err(to_js_error(format!("Notary {} not found", notary_id)));
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

#[napi]
#[derive(Clone)]
pub struct NotaryClient {
  pub notary_id: u32,
  public: ed25519::Public,
  client: Arc<Mutex<ulx_notary::Client>>,
  last_metadata: Arc<Mutex<Option<ulx_primitives::NotebookMeta>>>,
  pub auto_verify_header_signatures: bool,
}

#[napi]
impl NotaryClient {
  #[napi]
  pub async fn is_connected(&self) -> bool {
    let client = self.client.lock().await;
    (*client).is_connected()
  }

  #[napi(factory)]
  pub async fn connect(
    notary_id: u32,
    public_key: Uint8Array,
    host: String,
    auto_verify_header_signatures: bool,
  ) -> Result<Self> {
    let public: [u8; 32] = public_key.as_ref().try_into().map_err(to_js_error)?;
    Ok(Self {
      notary_id,
      public: ed25519::Public::from_raw(public),
      auto_verify_header_signatures,
      last_metadata: Arc::new(Mutex::new(None)),
      client: Arc::new(Mutex::new(
        ulx_notary::create_client(&host)
          .await
          .map_err(to_js_error)?,
      )),
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
      .await
      .map_err(to_js_error)?;

    Ok(res)
  }

  #[napi]
  pub async fn get_balance_tip(
    &self,
    address: String,
    account_type: AccountType,
  ) -> Result<BalanceTipResult> {
    let client = self.client.lock().await;
    let account_id = AccountId::from_ss58check(&address).map_err(to_js_error)?;
    let res = (*client)
      .get_tip(account_id, account_type)
      .await
      .map_err(to_js_error)?;

    Ok(BalanceTipResult {
      balance_tip: Uint8Array::from(res.balance_tip.as_ref()),
      notebook_number: res.notebook_number,
      tick: res.tick,
    })
  }

  pub async fn notarize(&self, notarization: Notarization) -> anyhow::Result<BalanceChangeResult> {
    let client = self.client.lock().await;
    let res = (*client)
      .notarize(
        notarization.balance_changes,
        notarization.block_votes,
        notarization.data_domains,
      )
      .await
      .map_err(|e| {
        tracing::error!("Error sending notarization: {:?}", e);
        e
      })?;
    Ok(res)
  }

  #[napi(getter)]
  pub async fn metadata(&self) -> Result<NotebookMeta> {
    let client = self.client.lock().await;
    let meta = (*client).metadata().await.map_err(to_js_error)?;

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
  ) -> anyhow::Result<BalanceProof> {
    let client = self.client.lock().await;

    let proof = (*client).get_balance_proof(notebook_number, tip).await?;
    Ok(proof)
  }

  pub fn verify_header(&self, header: &SignedNotebookHeader) -> anyhow::Result<()> {
    if !header
      .signature
      .verify(header.header.hash().as_bytes(), &self.public)
    {
      return Err(anyhow::anyhow!("This header has an invalid signature"));
    }
    Ok(())
  }

  pub async fn wait_for_notebook(
    &self,
    notebook_number: u32,
  ) -> anyhow::Result<SignedNotebookHeader> {
    let mut has_seen_notebook = {
      let last_metadata = self.last_metadata.lock().await;
      if let Some(meta) = &*last_metadata {
        meta.finalized_notebook_number >= notebook_number
      } else {
        false
      }
    };
    if has_seen_notebook == false {
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
      *last_metadata = Some(ulx_primitives::NotebookMeta {
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
    anyhow::bail!("No header found")
  }
}
#[napi]
#[derive(Clone)]
pub struct NotebookMeta {
  pub finalized_notebook_number: u32,
  pub finalized_tick: u32,
}

#[napi]
#[derive(Clone)]
pub struct BalanceTipResult {
  pub balance_tip: Uint8Array,
  pub notebook_number: u32,
  pub tick: u32,
}
