use std::sync::Arc;

use anyhow::anyhow;
use futures::FutureExt;
use jsonrpsee::{
	client_transport::ws::{Url, WsTransportClientBuilder},
	core::client::ClientBuilder,
	ws_client::{PingConfig, WsClient},
};
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

pub use spec::api;

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

pub async fn try_until_connected(
	url: String,
	retry_delay_millis: u64,
	timeout_millis: u64,
) -> Result<UlxClient, Error> {
	let start = std::time::Instant::now();
	let rpc = loop {
		match if cfg!(any(debug_assertions, test)) {
			UlxClient::from_insecure_url(url.clone()).await
		} else {
			UlxClient::from_url(url.clone()).await
		} {
			Ok(client) => break client,
			Err(why) => {
				if start.elapsed().as_millis() as u64 > timeout_millis {
					return Err(Error::Other(
						"Failed to connect to client within timeout".to_string(),
					));
				}
				println!(
					"failed to connect client to {:?}, {:?} retrying soon..",
					url.clone(),
					why
				);
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
	pub async fn try_until_connected(
		url: String,
		retry_delay_millis: u64,
		timeout_millis: u64,
	) -> Result<Self, Error> {
		let start = std::time::Instant::now();
		let url = Url::parse(&url)
			.map_err(|e| Error::Other(format!("Invalid Mainchain URL: {} -> {}", url, e)))?;

		let (sender, receiver) = loop {
			let builder = {
				#[allow(clippy::let_and_return)]
				let transport_builder = WsTransportClientBuilder::default();
				#[cfg(any(target_os = "ios", target_os = "android"))]
				let transport_builder = transport_builder.use_webpki_rustls();

				transport_builder
			};
			match builder.build(url.clone()).await {
				Ok(client) => break client,
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
		};
		let client = ClientBuilder::default()
			.enable_ws_ping(PingConfig::default())
			.build_with_tokio(sender, receiver);
		let client = Arc::new(client);

		let rpc = RpcClient::new(client.clone());
		Ok(Self {
			rpc: rpc.clone(),
			live: UlxClient::from_rpc_client(rpc.clone()).await?,
			methods: LegacyRpcMethods::new(rpc.clone()),
			ws_client: client,
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
		let start = std::time::Instant::now();
		let url = Url::parse(&url)
			.map_err(|e| Error::Other(format!("Invalid Notary URL:  {} -> {}", url, e)))?;
		let (sender, receiver) = loop {
			match WsTransportClientBuilder::default().build(url.clone()).await {
				Ok(client) => break client,
				Err(why) => {
					if start.elapsed().as_millis() as u64 > 10_000u64 {
						return Err(anyhow!("Failed to connect to client within timeout",));
					}
					warn!(
						"failed to connect to client due to {} - {:?}, retrying soon..",
						url, why
					);
					tokio::time::sleep(std::time::Duration::from_millis(1_000u64)).await;
				},
			}
		};
		let client = ClientBuilder::default()
			.enable_ws_ping(PingConfig::default())
			.build_with_tokio(sender, receiver);

		let client = Arc::new(client);
		let rpc_client = RpcClient::new(client.clone());
		let ulx_client = UlxClient::from_rpc_client(rpc_client).await?;
		*lock = Some(ulx_client.clone());
		drop(lock);

		let handle_mutex = self.handle.clone();
		let handle = tokio::spawn(async move {
			let url = url.clone();
			let client_lock = client_lock.clone();
			let ws_client = client.clone();
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

#[cfg(test)]
mod test {
	use super::*;
	use std::env;
	use ulx_testing::{test_context, test_context_from_url};

	#[tokio::test]
	async fn test_getting_ticker() -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();
		let use_live = env::var("USE_LIVE").unwrap_or(String::from("false")).parse::<bool>()?;
		let ctx = if use_live {
			test_context_from_url("ws://localhost:9944").await
		} else {
			test_context().await
		};

		let ws_url = ctx.ws_url.clone();

		let mut client = MultiurlClient::new(vec![ws_url.clone()]);
		let ticker = client.lookup_ticker().await;
		assert!(ticker.is_ok());
		Ok(())
	}

	#[ignore]
	#[tokio::test]
	async fn test_redirecting_client() -> anyhow::Result<()> {
		let _ = tracing_subscriber::fmt::try_init();

		let client =
			UlxClient::from_insecure_url("wss://husky-witty-highly.ngrok-free.app").await?;
		let ticker = client
			.runtime_api()
			.at(client.genesis_hash())
			.call(api::runtime_apis::tick_apis::TickApis.ticker())
			.await;
		assert!(ticker.is_ok());
		Ok(())
	}
}
