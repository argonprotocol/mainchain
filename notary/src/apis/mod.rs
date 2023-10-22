use jsonrpsee::{async_client::Client, ws_client::WsClientBuilder};

use crate::apis::{localchain::LocalchainRpcClient, notebook::NotebookRpcClient};

pub mod localchain;
pub mod notebook;

pub async fn create_client<T: LocalchainRpcClient + NotebookRpcClient>(
	url: &str,
) -> anyhow::Result<Client> {
	let client = WsClientBuilder::default().build(&url).await?;
	Ok(client)
}
