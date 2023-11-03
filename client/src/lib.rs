use subxt::{
	config::{Config, DefaultExtrinsicParams, DefaultExtrinsicParamsBuilder},
	error::Error,
	OnlineClient,
};

pub use spec::api;

pub mod signature_messages;
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

pub async fn try_until_connected(url: String, retry_delay_millis: u64) -> Result<UlxClient, Error> {
	let rpc = loop {
		match UlxClient::from_url(url.clone()).await {
			Ok(client) => break client,
			Err(why) => {
				println!("failed to connect to client due to {:?}, retrying soon..", why);
				tokio::time::sleep(std::time::Duration::from_millis(retry_delay_millis)).await;
			},
		}
	};
	Ok(rpc)
}
