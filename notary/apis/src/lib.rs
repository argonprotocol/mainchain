use jsonrpsee::ws_client::WsClientBuilder;

pub use crate::{localchain::LocalchainRpcClient, notebook::NotebookRpcClient};

pub mod localchain;
pub mod notebook;

pub type Client = jsonrpsee::core::client::Client;

pub async fn create_client(url: &str) -> anyhow::Result<Client> {
	let client = WsClientBuilder::default().build(&url).await?;
	Ok(client)
}
