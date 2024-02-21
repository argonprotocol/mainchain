use anyhow::anyhow;
use futures::FutureExt;
use jsonrpsee::ws_client::{WsClient, WsClientBuilder};
pub use spec::api;
use std::sync::Arc;
use subxt::{
	backend::{legacy::LegacyRpcMethods, rpc::RpcClient},
	config::{Config, DefaultExtrinsicParams, DefaultExtrinsicParamsBuilder},
	error::Error,
	OnlineClient,
};
use tokio::{
	sync::{Mutex, RwLock},
	task::JoinHandle,
};
use tracing::warn;

use crate::api::runtime_types::ulx_primitives::tick::Ticker;

mod spec;

pub enum UlxConfig {}

pub type UlxClient = OnlineClient<UlxConfig>;

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

pub async fn local_client() -> Result<UlxClient, Error> {
	OnlineClient::<UlxConfig>::new().await
}

pub async fn try_until_connected(url: String, retry_delay_millis: u64, timeout_millis: u64) -> Result<UlxClient, Error> {
	let start = std::time::Instant::now();
	let rpc = loop {
		match UlxClient::from_url(url.clone()).await {
			Ok(client) => break client,
			Err(why) => {
				if start.elapsed().as_millis() as u64 > timeout_millis {
					return Err(Error::Other("Failed to connect to client within timeout".to_string()));
				}
				println!("failed to connect to client due to {:?}, retrying soon..", why);
				tokio::time::sleep(std::time::Duration::from_millis(retry_delay_millis)).await;
			},
		}
	};
	Ok(rpc)
}

#[derive(Clone)]
pub struct UlxFullclient {
	pub live: UlxClient,
	pub rpc: RpcClient,
	pub ws_client: Arc<WsClient>,
	pub methods: LegacyRpcMethods<UlxConfig>,
}
impl UlxFullclient {
	pub async fn try_until_connected(url: String, retry_delay_millis: u64, timeout_millis: u64) -> Result<Self, Error> {
		let start = std::time::Instant::now();
		let ws_client = loop {
			match WsClientBuilder::default().build(&url).await {
				Ok(client) => break client,
				Err(why) => {
					if start.elapsed().as_millis() as u64 > timeout_millis {
						return Err(Error::Other("Failed to connect to client within timeout".to_string()));
					}
					warn!(
						"failed to connect to client due to {} - {:?}, retrying soon..",
						url, why
					);
					tokio::time::sleep(std::time::Duration::from_millis(retry_delay_millis)).await;
				},
			}
		};
		let ws_client = Arc::new(ws_client);
		let rpc = RpcClient::new(ws_client.clone());
		Ok(Self {
			rpc: rpc.clone(),
			live: UlxClient::from_rpc_client(rpc.clone()).await?,
			methods: LegacyRpcMethods::new(rpc.clone()),
			ws_client,
		})
	}
}

#[derive(Clone)]
pub struct MultiurlClient {
	urls: Vec<String>,
	client: Arc<RwLock<Option<UlxClient>>>,
	current_index: usize,
	handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl MultiurlClient {
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

	pub async fn lookup_ticker(&mut self) -> anyhow::Result<Ticker> {
		let client = self.get().await?;
		let ticker_data = client
			.runtime_api()
			.at(client.genesis_hash())
			.call(api::runtime_apis::tick_apis::TickApis.ticker())
			.await?;
		Ok(ticker_data)
	}
	pub async fn get(&mut self) -> anyhow::Result<UlxClient> {
		{
			let lock = self.client.read().await;
			if let Some(client) = &*lock {
				return Ok(client.clone())
			}
		}

		self.current_index += 1;
		if self.current_index >= self.urls.len() {
			self.current_index = 0;
		}
		let url = self.urls[self.current_index].clone();

		let mut lock = self.client.write().await;
		let client_lock = self.client.clone();
		let ws_client = WsClientBuilder::default()
			.build(&url)
			.await
			.map_err(|e| anyhow!("Could not connect to mainchain node at {} - {:?}", url, e))?;
		let ws_client = Arc::new(ws_client);
		let rpc_client = RpcClient::new(ws_client.clone());
		let ulx_client = UlxClient::from_rpc_client(rpc_client).await?;
		*lock = Some(ulx_client.clone());
		drop(lock);

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

	pub fn on_rpc_error(&mut self) {
		let mut lock = self.client.write().now_or_never().unwrap();
		*lock = None;
	}
}
