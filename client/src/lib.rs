use std::{io::Write, sync::Arc};

use anyhow::anyhow;
use jsonrpsee::{
	client_transport::ws::{Url, WsTransportClientBuilder},
	core::client::ClientBuilder,
	ws_client::{PingConfig, WsClient},
};
use sp_core::{crypto::AccountId32, H256};
use subxt::{
	backend::{legacy::LegacyRpcMethods, rpc::RpcClient, BlockRef},
	config::{Config, DefaultExtrinsicParams, DefaultExtrinsicParamsBuilder, ExtrinsicParams},
	error::{Error, RpcError},
	ext::subxt_core::tx::payload::ValidationDetails,
	runtime_api::Payload as RuntimeApiPayload,
	storage::Address as StorageAddress,
	tx::{Payload, Signer, TxInBlock, TxProgress, TxStatus},
	utils::Yes,
	Metadata, OnlineClient,
};
use tokio::{
	sync::{mpsc, Mutex, RwLock},
	task::JoinHandle,
};
use tracing::{log::debug, warn};

use argon_primitives::{
	tick::Tick, AccountId, BlockNumber, Chain, ChainIdentity, Nonce, VotingSchedule,
};
pub use spec::api;

use crate::api::{
	block_seal_spec::storage::types::current_vote_minimum::CurrentVoteMinimum, ownership, storage,
	system,
};

pub mod conversion;
pub mod signer;
mod spec;

pub enum ArgonConfig {}

pub type ArgonOnlineClient = OnlineClient<ArgonConfig>;

impl Config for ArgonConfig {
	type Hash = subxt::utils::H256;
	type AccountId = subxt::utils::AccountId32;
	type Address = subxt::utils::MultiAddress<Self::AccountId, ()>;
	type Signature = subxt::utils::MultiSignature;
	type Hasher = subxt::config::substrate::BlakeTwo256;
	type Header = subxt::config::substrate::SubstrateHeader<u32, Self::Hasher>;
	type ExtrinsicParams = ArgonExtrinsicParams<Self>;
	type AssetId = ();
}

/// A struct representing the signed extra and additional parameters required
/// to construct a transaction for a argon node.
pub type ArgonExtrinsicParams<T> = DefaultExtrinsicParams<T>;

pub type ArgonTxProgress = TxProgress<ArgonConfig, ArgonOnlineClient>;

/// A builder which leads to [`ArgonExtrinsicParams`] being constructed.
/// This is what you provide to methods like `sign_and_submit()`.
pub type ArgonExtrinsicParamsBuilder<T> = DefaultExtrinsicParamsBuilder<T>;

pub fn account_id_to_subxt(account_id: &AccountId) -> subxt::utils::AccountId32 {
	let bytes: [u8; 32] = *account_id.as_ref();
	subxt::utils::AccountId32::from(bytes)
}

#[derive(Clone)]
pub struct MainchainClient {
	pub live: ArgonOnlineClient,
	pub rpc: RpcClient,
	pub ws_client: Arc<WsClient>,
	pub methods: LegacyRpcMethods<ArgonConfig>,
	pub url: String,
	on_client_error: Option<mpsc::Sender<String>>,
}

impl MainchainClient {
	pub fn ext_params_builder() -> ArgonExtrinsicParamsBuilder<ArgonConfig> {
		ArgonExtrinsicParamsBuilder::<ArgonConfig>::new()
	}

	pub async fn params_with_best_nonce(
		&self,
		account_id: AccountId32,
	) -> anyhow::Result<ArgonExtrinsicParamsBuilder<ArgonConfig>> {
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
		let live = ArgonOnlineClient::from_rpc_client(rpc.clone()).await?;
		let methods = LegacyRpcMethods::new(rpc.clone());
		Ok(Self { rpc, live, methods, ws_client, url, on_client_error: Default::default() })
	}

	pub async fn from_url(url: &str) -> Result<Self, Error> {
		let ws_client = Self::create_ws_client(url).await?;
		Self::new(ws_client, url.to_string()).await
	}

	pub fn create_polkadotjs_deeplink<Call: Payload>(&self, call: &Call) -> anyhow::Result<String> {
		let ext_data = self.live.tx().call_data(call)?;
		let mut url = Url::parse(&format!(
			"https://polkadot.js.org/apps/#/extrinsics/decode/0x{}",
			hex::encode(ext_data)
		))?;
		url.set_query(Some(&format!("rpc={}", self.url)));
		Ok(url.to_string())
	}

	pub fn extract_call_data(&self, cli_text: &str) -> anyhow::Result<Vec<u8>> {
		let Some(ext_str) = cli_text.split("/extrinsics/decode/0x").last() else {
			return Err(anyhow!("Invalid cli text"));
		};

		let ext_data = hex::decode(ext_str.trim())?;
		Ok(ext_data)
	}

	pub async fn submit_from_polkadot_url(
		&self,
		message: &str,
		signer: &impl Signer<ArgonConfig>,
		params: Option<<ArgonExtrinsicParams<ArgonConfig> as ExtrinsicParams<ArgonConfig>>::Params>,
	) -> anyhow::Result<ArgonTxProgress> {
		let ext_data = self.extract_call_data(message)?;
		self.submit_raw(ext_data, signer, params).await
	}

	pub async fn submit_raw(
		&self,
		payload: Vec<u8>,
		signer: &impl Signer<ArgonConfig>,
		params: Option<<ArgonExtrinsicParams<ArgonConfig> as ExtrinsicParams<ArgonConfig>>::Params>,
	) -> anyhow::Result<ArgonTxProgress> {
		let payload = RawPayload(payload);
		let tx_progress = self
			.live
			.tx()
			.sign_and_submit_then_watch(&payload, signer, params.unwrap_or_default())
			.await?;
		Ok(tx_progress)
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
						"ArgonFullClient: failed to connect client to {} - {:?}, retrying soon..",
						url, why
					);
					tokio::time::sleep(std::time::Duration::from_millis(retry_delay_millis)).await;
				},
			}
		}
	}

	pub async fn get_account(
		&self,
		account_id: &AccountId32,
	) -> anyhow::Result<system::storage::types::account::Account> {
		let account_id32 = account_id_to_subxt(account_id);
		let info = self
			.fetch_storage(&storage().system().account(account_id32), None)
			.await?
			.ok_or_else(|| anyhow!("No record found for account {:?}", &account_id))?;
		Ok(info)
	}

	pub async fn get_argons(
		&self,
		account_id: &AccountId32,
	) -> anyhow::Result<ownership::storage::types::account::Account> {
		let account = self.get_account(account_id).await?;
		Ok(account.data)
	}

	pub async fn get_ownership(
		&self,
		account_id: &AccountId32,
	) -> anyhow::Result<ownership::storage::types::account::Account> {
		let account_id32 = account_id_to_subxt(account_id);
		let balance = self
			.fetch_storage(&storage().ownership().account(account_id32), None)
			.await?
			.ok_or_else(|| anyhow!("No record found for account {:?}", &account_id))?;
		Ok(balance)
	}

	pub async fn get_account_nonce(&self, account_id: AccountId32) -> anyhow::Result<Nonce> {
		let account_id32 = account_id_to_subxt(&account_id);
		self.get_account_nonce_subxt(&account_id32).await
	}
	pub async fn get_account_nonce_subxt(
		&self,
		account_id32: &subxt::utils::AccountId32,
	) -> anyhow::Result<Nonce> {
		let nonce = self.methods.system_account_next_index(account_id32).await?;
		Ok(nonce as Nonce)
	}

	pub async fn block_at_height(&self, height: BlockNumber) -> anyhow::Result<Option<H256>> {
		let best_block = self.best_block_hash().await?;
		let block_hash = self
			.fetch_storage(&storage().system().block_hash(height), Some(best_block))
			.await?;
		Ok(block_hash)
	}

	pub async fn block_number(
		&self,
		hash: <ArgonConfig as Config>::Hash,
	) -> anyhow::Result<BlockNumber> {
		self.live
			.backend()
			.block_header(hash)
			.await?
			.map(|a| a.number)
			.ok_or_else(|| anyhow!("Block header not found for block hash"))
	}

	pub async fn get_vote_block_hash(
		&self,
		current_tick: Tick,
	) -> anyhow::Result<(H256, CurrentVoteMinimum)> {
		let best_hash = self.best_block_hash().await?;
		let voting_schedule = VotingSchedule::when_creating_votes(current_tick);
		let grandparent_tick = voting_schedule.grandparent_votes_tick();
		let votable_blocks = self
			.fetch_storage(
				&api::ticks::storage::StorageApi.recent_blocks_at_ticks(),
				Some(best_hash),
			)
			.await?
			.ok_or_else(|| anyhow!("No vote blocks found"))?
			.0;

		let best_vote_block = votable_blocks
			.into_iter()
			.find_map(|(tick, hash)| if tick == grandparent_tick { Some(hash) } else { None })
			.ok_or_else(|| {
				anyhow!("No vote block found at grandparent tick ({grandparent_tick})")
			})?;
		debug!("Block to vote on at grandparent tick {}: {:?}", grandparent_tick, best_vote_block);

		let minimum = self
			.fetch_storage(
				&api::block_seal_spec::storage::StorageApi.current_vote_minimum(),
				Some(best_hash),
			)
			.await?
			.ok_or_else(|| anyhow!("No minimum vote requirement found"))?;

		Ok((H256(best_vote_block.0), minimum))
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
		mut tx_progress: ArgonTxProgress,
	) -> anyhow::Result<TxInBlock<ArgonConfig, OnlineClient<ArgonConfig>>, Error> {
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
	pub async fn lookup_ticker(&self) -> anyhow::Result<argon_primitives::tick::Ticker> {
		let ticker_data = self.call(api::runtime_apis::tick_apis::TickApis.ticker(), None).await?;

		Ok(ticker_data.into())
	}

	/// Get the system chain and genesis hash.
	pub async fn get_chain_identity(&self) -> Result<ChainIdentity, Error> {
		let chain: Chain = self.methods.system_chain().await?.try_into()?;
		let genesis_hash = self.methods.genesis_hash().await?;
		Ok(ChainIdentity { chain, genesis_hash })
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
		let builder = WsTransportClientBuilder::default();
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

struct RawPayload(Vec<u8>);
impl Payload for RawPayload {
	fn encode_call_data_to(
		&self,
		_metadata: &Metadata,
		out: &mut Vec<u8>,
	) -> Result<(), subxt::ext::subxt_core::Error> {
		out.write(&self.0).map_err(|_| codec::Error::from("Failed to write"))?;
		Ok(())
	}
	fn validation_details(&self) -> Option<ValidationDetails<'_>> {
		None
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

		let argon_client =
			MainchainClient::try_until_connected(url.as_str(), 1_000u64, 10_000u64).await?;
		*lock = Some(argon_client.clone());
		drop(lock);
		let ws_client = argon_client.ws_client.clone();

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

		Ok(argon_client)
	}
}

#[cfg(test)]
mod test {
	use argon_testing::start_argon_test_node;

	use super::*;

	#[tokio::test]
	async fn test_getting_ticker() {
		let _ = tracing_subscriber::fmt::try_init();
		let ctx = start_argon_test_node().await;

		let ws_url = ctx.client.url.clone();

		let mut client = ReconnectingClient::new(vec![ws_url.clone()]);
		let ticker = client.get().await.unwrap().lookup_ticker().await;
		assert!(ticker.is_ok());
	}

	#[ignore]
	#[tokio::test]
	async fn test_redirecting_client() {
		let _ = tracing_subscriber::fmt::try_init();

		let client =
			ArgonOnlineClient::from_insecure_url("wss://husky-witty-highly.ngrok-free.app")
				.await
				.unwrap();
		let ticker = client
			.runtime_api()
			.at(client.genesis_hash())
			.call(api::runtime_apis::tick_apis::TickApis.ticker())
			.await;
		assert!(ticker.is_ok());
	}
}
