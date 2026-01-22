pub use crate::{
	localchain::LocalchainRpcClient, notebook::NotebookRpcClient, system::SystemRpcClient,
};
use anyhow::anyhow;
use argon_primitives::{
	NotaryId, Notebook, NotebookNumber, SignedNotebookHeader,
	notary::{NotebookBytes, SignedHeaderBytes},
};
use codec::Decode;
use jsonrpsee::{
	async_client::ClientBuilder,
	client_transport::ws::{Url, WsTransportClientBuilder},
};
use std::{fmt::Debug, sync::OnceLock, time::Duration};
use tracing::trace;

pub mod error;
pub mod localchain;
pub mod notebook;
pub mod system;

pub use error::Error;

pub type Client = jsonrpsee::core::client::Client;

fn shared_http_client() -> &'static reqwest::Client {
	static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
	HTTP_CLIENT.get_or_init(|| {
		// One shared pool avoids creating a new TCP/TLS connection for every download.
		// Keep timeouts here conservative; per-request timeouts are still applied in `download()`.
		reqwest::Client::builder()
			.connect_timeout(Duration::from_secs(5))
			.pool_idle_timeout(Duration::from_secs(90))
			.pool_max_idle_per_host(20)
			.build()
			.expect("failed to build shared reqwest client")
	})
}

#[derive(Debug, Clone)]
pub struct ArchiveHost {
	pub url: Url,
	client: reqwest::Client,
}

pub fn get_notebook_bucket(notary_id: NotaryId) -> String {
	format!("notary/{}/notebook", notary_id)
}

pub fn get_header_bucket(notary_id: NotaryId) -> String {
	format!("notary/{}/header", notary_id)
}

pub fn get_notebook_url(url: &str, notary_id: NotaryId, notebook_number: NotebookNumber) -> String {
	let url = url.trim_end_matches('/');
	format!("{}/notary/{}/notebook/{}.scale", url, notary_id, notebook_number)
}

pub fn get_header_url(url: &str, notary_id: NotaryId, notebook_number: NotebookNumber) -> String {
	let url = url.trim_end_matches('/');
	format!("{}/notary/{}/header/{}.scale", url, notary_id, notebook_number)
}

impl ArchiveHost {
	pub fn new(url: String) -> anyhow::Result<Self> {
		Ok(Self { url: Url::parse(&url)?, client: shared_http_client().clone() })
	}

	pub fn get_header_url(&self, notary_id: NotaryId, notebook_number: NotebookNumber) -> String {
		get_header_url(self.url.as_str(), notary_id, notebook_number)
	}

	pub fn get_notebook_url(&self, notary_id: NotaryId, notebook_number: NotebookNumber) -> String {
		get_notebook_url(self.url.as_str(), notary_id, notebook_number)
	}

	async fn download(url: String, timeout: Duration) -> anyhow::Result<Vec<u8>> {
		download(shared_http_client(), url, timeout).await
	}

	pub async fn download_header_bytes(
		url: String,
		timeout: Duration,
	) -> anyhow::Result<SignedHeaderBytes> {
		let bytes = Self::download(url, timeout).await?;
		Ok(SignedHeaderBytes(bytes))
	}

	pub async fn download_notebook_bytes(
		url: String,
		timeout: Duration,
	) -> anyhow::Result<NotebookBytes> {
		let bytes = Self::download(url, timeout).await?;
		Ok(NotebookBytes(bytes))
	}

	pub async fn get_header(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		timeout: Duration,
	) -> anyhow::Result<SignedHeaderBytes> {
		let url = self.get_header_url(notary_id, notebook_number);
		let bytes = download(&self.client, url, timeout).await?;
		Ok(SignedHeaderBytes(bytes))
	}

	pub async fn get_notebook(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		timeout: Duration,
	) -> anyhow::Result<NotebookBytes> {
		let url = self.get_notebook_url(notary_id, notebook_number);
		let bytes = download(&self.client, url, timeout).await?;
		Ok(NotebookBytes(bytes))
	}
}

async fn download(
	client: &reqwest::Client,
	url: String,
	timeout: Duration,
) -> anyhow::Result<Vec<u8>> {
	let result = client.get(url.clone()).timeout(timeout).send().await?;
	let status = result.status();
	if !status.is_success() {
		return Err(anyhow!("Failed to download: {:?}", &result));
	}

	let content_length = result.content_length().unwrap_or(0);
	let headers = result.headers().clone();
	let bytes = result.bytes().await.map(|b| b.to_vec())?;

	trace!(?url, ?headers, ?content_length, "Notary/notebook download complete",);

	if let Some(content_length_header) = headers.get("content-length") {
		if content_length.to_string() != content_length_header.to_str()? {
			return Err(anyhow!("Content length mismatch"));
		}
	}

	Ok(bytes)
}

pub async fn download_notebook_header(
	notary_client: &Client,
	notebook_number: NotebookNumber,
	timeout: Duration,
) -> anyhow::Result<SignedNotebookHeader> {
	let url = notary_client.get_header_download_url(notebook_number).await?;
	let bytes = ArchiveHost::download(url, timeout).await?;
	Ok(SignedNotebookHeader::decode(&mut &bytes[..])?)
}

pub async fn download_notebook(
	notary_client: &Client,
	notebook_number: NotebookNumber,
	timeout: Duration,
) -> anyhow::Result<Notebook> {
	let url = notary_client.get_notebook_download_url(notebook_number).await?;
	let bytes = ArchiveHost::download(url, timeout).await?;
	Ok(Notebook::decode(&mut &bytes[..])?)
}

pub async fn create_client(url: &str) -> anyhow::Result<Client> {
	let transport_builder = WsTransportClientBuilder::default();
	let url = Url::parse(url).map_err(|e| anyhow!("Invalid URL: {:?} -> {}", url, e))?;

	let (sender, receiver) = transport_builder.build(url).await?;
	let client = ClientBuilder::default().build_with_tokio(sender, receiver);
	Ok(client)
}
