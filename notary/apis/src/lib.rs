use anyhow::anyhow;
use jsonrpsee::{
	async_client::ClientBuilder,
	client_transport::ws::{Url, WsTransportClientBuilder},
};

pub use crate::{localchain::LocalchainRpcClient, notebook::NotebookRpcClient};

pub mod localchain;
pub mod notebook;

pub type Client = jsonrpsee::core::client::Client;

pub async fn create_client(url: &str) -> anyhow::Result<Client> {
	let transport_builder = WsTransportClientBuilder::default();
	#[cfg(any(target_os = "ios", target_os = "android"))]
	let transport_builder = transport_builder.use_webpki_rustls();
	let url = Url::parse(url).map_err(|e| anyhow!("Invalid URL: {:?} -> {}", url, e))?;

	let (sender, receiver) = transport_builder.build(url).await?;
	let client = ClientBuilder::default().build_with_tokio(sender, receiver);
	Ok(client)
}
