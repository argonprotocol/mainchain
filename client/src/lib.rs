use std::sync::Arc;

use anyhow::anyhow;
use jsonrpsee::{
	client_transport::ws::{Url, WsTransportClientBuilder},
	core::client::ClientBuilder,
	ws_client::{PingConfig, WsClient},
};
use sp_core::{crypto::AccountId32, H256};
use subxt::{
	backend::{legacy::LegacyRpcMethods, rpc::RpcClient, BlockRef},
	config::{Config, DefaultExtrinsicParams, DefaultExtrinsicParamsBuilder},
	error::{Error, RpcError},
	runtime_api::RuntimeApiPayload,
	storage::{address::Yes, StorageAddress},
	tx::{TxInBlock, TxProgress, TxStatus},
	OnlineClient,
};
use tokio::{
	sync::{mpsc, Mutex, RwLock},
	task::JoinHandle,
};
use tracing::warn;

pub use spec::api;
use ulx_primitives::{AccountId, Nonce};

use crate::api::{storage, system, ulixee_balances};

mod conversion;
pub mod signer;
mod spec;

pub enum UlxConfig {}

pub type UlxOnlineClient = OnlineClient<UlxConfig>;

impl Config for UlxConfig {
	type Hash = subxt::utils::H256;
	type AccountId = subxt::utils::AccountId32;
	type Address = subxt::utils::MultiAddress<Self::AccountId, ()>;
	type Signature = subxt::utils::MultiSignature;
	type Hasher = subxt::config::substrate::BlakeTwo256;
	type Header = subxt::config::substrate::SubstrateHeader<u32, Self::Hasher>;
	type ExtrinsicParams = UlxExtrinsicParams<Self>;
	type AssetId = ();
}

/// A struct representing the signed extra and additional parameters required
/// to construct a transaction for a Ulx node.
pub type UlxExtrinsicParams<T> = DefaultExtrinsicParams<T>;

/// A builder which leads to [`UlxExtrinsicParams`] being constructed.
/// This is what you provide to methods like `sign_and_submit()`.
pub type UlxExtrinsicParamsBuilder<T> = DefaultExtrinsicParamsBuilder<T>;

pub fn account_id_to_subxt(account_id: &AccountId) -> subxt::utils::AccountId32 {
	let bytes: [u8; 32] = *account_id.as_ref();
	subxt::utils::AccountId32::from(bytes)
}

#[derive(Clone)]
pub struct MainchainClient {
	pub live: UlxOnlineClient,
	pub rpc: RpcClient,
	pub ws_client: Arc<WsClient>,
	pub methods: LegacyRpcMethods<UlxConfig>,
	pub url: String,
	on_client_error: Option<mpsc::Sender<String>>,
}

impl MainchainClient {
	pub fn ext_params_builder() -> UlxExtrinsicParamsBuilder<UlxConfig> {
		UlxExtrinsicParamsBuilder::<UlxConfig>::new()
	}

	pub async fn params_with_best_nonce(
		&self,
		account_id: AccountId32,
	) -> anyhow::Result<UlxExtrinsicParamsBuilder<UlxConfig>> {
		let nonce = self.get_account_nonce(account_id).await?;
		Ok(Self::ext_params_builder().nonce(nonce.into()))
	}

	pub fn subscribe_client_error(&mut self) -> mpsc::Receiver<String> {
		let (tx, rx) = mpsc::channel(1);
		self.on_client_error = Some(tx);
		rx
	}

	pub async fn new(ws_client: WsClient, url: String) -> Result<Self, Error> {
		let ws_client = Arc::new(ws_client);
		let rpc = RpcClient::new(ws_client.clone());
		let live = UlxOnlineClient::from_rpc_client(rpc.clone()).await?;
		let methods = LegacyRpcMethods::new(rpc.clone());
		Ok(Self { rpc, live, methods, ws_client, url, on_client_error: Default::default() })
	}

	pub async fn from_url(url: &str) -> Result<Self, Error> {
		let ws_client = Self::create_ws_client(url).await?;
		Self::new(ws_client, url.to_string()).await
	}

	pub async fn try_until_connected(
		url: &str,
		retry_delay_millis: u64,
		timeout_millis: u64,
	) -> Result<Self, Error> {
		let start = std::time::Instant::now();

		loop {
			match Self::from_url(url).await {
				Ok(client) => return Ok(client),
				Err(why) => {
					if start.elapsed().as_millis() as u64 > timeout_millis {
						return Err(Error::Other(
							"Failed to connect to client within timeout".to_string(),
						));
					}
					warn!(
						"UlxFullClient: failed to connect client to {} - {:?}, retrying soon..",
						url, why
					);
					tokio::time::sleep(std::time::Duration::from_millis(retry_delay_millis)).await;
				},
			}
		}
	}

	pub async fn get_account(
		&self,
		account_id: AccountId32,
	) -> anyhow::Result<system::storage::types::account::Account> {
		let account_id32 = account_id_to_subxt(&account_id);
		let info = self
			.fetch_storage(&storage().system().account(account_id32), None)
			.await?
			.ok_or_else(|| anyhow!("No record found for account {:?}", &account_id))?;
		Ok(info)
	}

	pub async fn get_argons(
		&self,
		account_id: AccountId32,
	) -> anyhow::Result<ulixee_balances::storage::types::account::Account> {
		let account = self.get_account(account_id).await?;
		Ok(account.data)
	}

	pub async fn get_ulixees(
		&self,
		account_id: AccountId32,
	) -> anyhow::Result<ulixee_balances::storage::types::account::Account> {
		let account_id32 = account_id_to_subxt(&account_id);
		let balance = self
			.fetch_storage(&storage().ulixee_balances().account(account_id32), None)
			.await?
			.ok_or_else(|| anyhow!("No record found for account {:?}", &account_id))?;
		Ok(balance)
	}

	pub async fn get_account_nonce(&self, account_id: AccountId32) -> anyhow::Result<Nonce> {
		let account_id32 = account_id_to_subxt(&account_id);
		let nonce = self.methods.system_account_next_index(&account_id32).await?;
		Ok(nonce as Nonce)
	}

	pub async fn best_block_hash(&self) -> anyhow::Result<H256> {
		let best_block_hash = self
			.methods
			.chain_get_block_hash(None)
			.await?
			.ok_or_else(|| anyhow!("No best block found"))?;
		Ok(best_block_hash)
	}

	pub async fn latest_finalized_block_hash(&self) -> anyhow::Result<BlockRef<H256>> {
		Ok(self.live.backend().latest_finalized_block_ref().await?)
	}
	pub async fn latest_finalized_block(&self) -> anyhow::Result<u32> {
		let block_number = self
			.fetch_storage(&storage().system().number(), None)
			.await?
			.unwrap_or_default();
		Ok(block_number)
	}

	pub async fn wait_for_ext_in_block(
		mut tx_progress: TxProgress<UlxConfig, OnlineClient<UlxConfig>>,
	) -> anyhow::Result<TxInBlock<UlxConfig, OnlineClient<UlxConfig>>, Error> {
		while let Some(status) = tx_progress.next().await {
			match status? {
				TxStatus::InBestBlock(tx_in_block) | TxStatus::InFinalizedBlock(tx_in_block) => {
					// now, we can attempt to work with the block, eg:
					tx_in_block.wait_for_success().await?;
					return Ok(tx_in_block);
				},
				TxStatus::Error { message } |
				TxStatus::Invalid { message } |
				TxStatus::Dropped { message } => {
					// Handle any errors:
					return Err(Error::from(format!(
						"Error submitting transaction to block: {message}"
					)));
				},
				// Continue otherwise:
				_ => continue,
			}
		}
		Err(Error::from("No valid status encountered for transaction".to_string()))
	}
	pub async fn lookup_ticker(&self) -> anyhow::Result<ulx_primitives::tick::Ticker> {
		let ticker_data = self
			.call(api::runtime_apis::tick_apis::TickApis.ticker(), Some(self.live.genesis_hash()))
			.await?;

		Ok(ticker_data.into())
	}

	pub async fn call<Call: RuntimeApiPayload>(
		&self,
		payload: Call,
		at: Option<H256>,
	) -> Result<Call::ReturnType, Error> {
		let api = match at {
			Some(at) => self.live.runtime_api().at(at),
			None => self.live.runtime_api().at_latest().await?,
		};
		match api.call(payload).await {
			Ok(x) => Ok(x),
			Err(e) => {
				if matches!(e, Error::Rpc(RpcError::ClientError(_))) {
					if let Some(on_client_error) = self.on_client_error.as_ref() {
						let _ = on_client_error.send(e.to_string()).await;
					}
				}
				Err(e)
			},
		}
	}

	pub async fn fetch_storage<Address>(
		&self,
		address: &Address,
		at: Option<H256>,
	) -> Result<Option<Address::Target>, Error>
	where
		Address: StorageAddress<IsFetchable = Yes>,
	{
		let storage = match at {
			Some(at) => self.live.storage().at(at),
			None => self.live.storage().at_latest().await?,
		};

		match storage.fetch(address).await {
			Ok(x) => Ok(x),
			Err(e) => {
				if matches!(e, Error::Rpc(RpcError::ClientError(_))) {
					if let Some(on_client_error) = self.on_client_error.as_ref() {
						let _ = on_client_error.send(e.to_string()).await;
					}
				}
				Err(e)
			},
		}
	}

	async fn create_ws_client(url: &str) -> Result<WsClient, Error> {
		let url = Url::parse(url)
			.map_err(|e| Error::Other(format!("Invalid Mainchain URL: {} -> {}", url, e)))?;
		let builder = {
			#[allow(clippy::let_and_return)]
			let transport_builder = WsTransportClientBuilder::default();
			#[cfg(any(target_os = "ios", target_os = "android"))]
			let transport_builder = transport_builder.use_webpki_rustls();

			transport_builder
		};
		let (sender, receiver) = builder
			.build(url)
			.await
			.map_err(|e| Error::Other(format!("Websocket handshake error {:?}", e)))?;
		let client = ClientBuilder::default()
			.enable_ws_ping(PingConfig::default())
			.build_with_tokio(sender, receiver);
		Ok(client)
	}
}

#[derive(Clone)]
pub struct ReconnectingClient {
	urls: Vec<String>,
	client: Arc<RwLock<Option<MainchainClient>>>,
	current_index: usize,
	handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ReconnectingClient {
	pub fn new(urls: Vec<String>) -> Self {
		Self {
			urls,
			client: Arc::new(RwLock::new(None)),
			handle: Arc::new(Mutex::new(None)),
			current_index: 0,
		}
	}

	pub async fn close(&mut self) {
		let mut handle = self.handle.lock().await;
		if let Some(handle) = handle.take() {
			handle.abort();
		}
	}

	pub async fn get(&mut self) -> anyhow::Result<MainchainClient> {
		{
			let lock = self.client.read().await;
			if let Some(client) = &*lock {
				return Ok(client.clone());
			}
		}

		self.current_index += 1;
		if self.current_index >= self.urls.len() {
			self.current_index = 0;
		}
		let url = self.urls[self.current_index].clone();

		let mut lock = self.client.write().await;
		let client_lock = self.client.clone();

		let ulx_client =
			MainchainClient::try_until_connected(url.as_str(), 1_000u64, 10_000u64).await?;
		*lock = Some(ulx_client.clone());
		drop(lock);
		let ws_client = ulx_client.ws_client.clone();

		let handle_mutex = self.handle.clone();
		let handle = tokio::spawn(async move {
			let url = url.clone();
			let client_lock = client_lock.clone();
			let ws_client = ws_client.clone();
			let _ = ws_client.on_disconnect().await;

			warn!("Disconnected from mainchain at {url} client",);
			*client_lock.write().await = None;
			*handle_mutex.lock().await = None;
		});
		*(self.handle.lock().await) = Some(handle);

		Ok(ulx_client)
	}
}

#[cfg(test)]
mod test {
	use ulx_testing::start_ulx_test_node;

	use super::*;

	#[tokio::test]
	async fn test_getting_ticker() -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let ctx = start_ulx_test_node().await;

		let ws_url = ctx.client.url.clone();

		let mut client = ReconnectingClient::new(vec![ws_url.clone()]);
		let ticker = client.get().await?.lookup_ticker().await;
		assert!(ticker.is_ok());
		Ok(())
	}

	#[ignore]
	#[tokio::test]
	async fn test_redirecting_client() -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();

		let client =
			UlxOnlineClient::from_insecure_url("wss://husky-witty-highly.ngrok-free.app").await?;
		let ticker = client
			.runtime_api()
			.at(client.genesis_hash())
			.call(api::runtime_apis::tick_apis::TickApis.ticker())
			.await;
		assert!(ticker.is_ok());
		Ok(())
	}
}
