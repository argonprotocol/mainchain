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
use url::Host;

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

fn strict_http_client() -> &'static reqwest::Client {
	static STRICT_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
	STRICT_HTTP_CLIENT.get_or_init(|| {
		reqwest::Client::builder()
			.connect_timeout(Duration::from_secs(5))
			.pool_idle_timeout(Duration::from_secs(90))
			.pool_max_idle_per_host(20)
			// Strict mode allows redirects only when they stay on the same host.
			.redirect(reqwest::redirect::Policy::custom(|attempt| {
				if let Some(previous) = attempt.previous().last() {
					if !is_same_host(attempt.url(), previous) {
						return attempt.error("cross-host redirect is not allowed in strict mode");
					}
				}
				if has_private_or_loopback_target(attempt.url()) {
					return attempt
						.error("private/loopback redirect target is not allowed in strict mode");
				}
				attempt.follow()
			}))
			.build()
			.expect("failed to build strict reqwest client")
	})
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DownloadKind {
	Header,
	Notebook,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DownloadTrustMode {
	Strict,
	Dev,
}

#[derive(Debug, Clone)]
pub struct DownloadPolicy {
	pub trust_mode: DownloadTrustMode,
	pub expected_origin: Option<String>,
	pub expected_path_suffix: Option<String>,
	pub max_bytes: Option<u64>,
}

impl DownloadPolicy {
	pub fn disabled() -> Self {
		Self {
			trust_mode: DownloadTrustMode::Dev,
			expected_origin: None,
			expected_path_suffix: None,
			max_bytes: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct ArchiveHost {
	pub url: Url,
	client: reqwest::Client,
}

pub fn get_notebook_bucket(notary_id: NotaryId) -> String {
	format!("notary/{notary_id}/notebook")
}

pub fn get_header_bucket(notary_id: NotaryId) -> String {
	format!("notary/{notary_id}/header")
}

pub fn get_notebook_url(url: &str, notary_id: NotaryId, notebook_number: NotebookNumber) -> String {
	let url = url.trim_end_matches('/');
	format!("{url}/notary/{notary_id}/notebook/{notebook_number}.scale")
}

pub fn get_header_url(url: &str, notary_id: NotaryId, notebook_number: NotebookNumber) -> String {
	let url = url.trim_end_matches('/');
	format!("{url}/notary/{notary_id}/header/{notebook_number}.scale")
}

pub fn get_download_path_suffix(
	kind: DownloadKind,
	notary_id: NotaryId,
	notebook_number: NotebookNumber,
) -> String {
	match kind {
		DownloadKind::Header => format!("/notary/{notary_id}/header/{notebook_number}.scale"),
		DownloadKind::Notebook => format!("/notary/{notary_id}/notebook/{notebook_number}.scale"),
	}
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
		download(shared_http_client(), url, timeout, None).await
	}

	pub async fn download_header_bytes(
		url: String,
		timeout: Duration,
	) -> anyhow::Result<SignedHeaderBytes> {
		let bytes = Self::download(url, timeout).await?;
		Ok(SignedHeaderBytes(bytes))
	}

	pub async fn download_header_bytes_with_policy(
		url: String,
		timeout: Duration,
		policy: &DownloadPolicy,
	) -> anyhow::Result<SignedHeaderBytes> {
		let bytes = download_with_policy(url, timeout, policy).await?;
		Ok(SignedHeaderBytes(bytes))
	}

	pub async fn download_notebook_bytes(
		url: String,
		timeout: Duration,
	) -> anyhow::Result<NotebookBytes> {
		let bytes = Self::download(url, timeout).await?;
		Ok(NotebookBytes(bytes))
	}

	pub async fn download_notebook_bytes_with_policy(
		url: String,
		timeout: Duration,
		policy: &DownloadPolicy,
	) -> anyhow::Result<NotebookBytes> {
		let bytes = download_with_policy(url, timeout, policy).await?;
		Ok(NotebookBytes(bytes))
	}

	pub async fn get_header(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		timeout: Duration,
	) -> anyhow::Result<SignedHeaderBytes> {
		let url = self.get_header_url(notary_id, notebook_number);
		let bytes = download(&self.client, url, timeout, None).await?;
		Ok(SignedHeaderBytes(bytes))
	}

	pub async fn get_notebook(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
		timeout: Duration,
	) -> anyhow::Result<NotebookBytes> {
		let url = self.get_notebook_url(notary_id, notebook_number);
		let bytes = download(&self.client, url, timeout, None).await?;
		Ok(NotebookBytes(bytes))
	}
}

async fn download_with_policy(
	url: String,
	timeout: Duration,
	policy: &DownloadPolicy,
) -> anyhow::Result<Vec<u8>> {
	let client = match policy.trust_mode {
		DownloadTrustMode::Strict => strict_http_client(),
		DownloadTrustMode::Dev => shared_http_client(),
	};
	download(client, url, timeout, Some(policy)).await
}

async fn download(
	client: &reqwest::Client,
	url: String,
	timeout: Duration,
	policy: Option<&DownloadPolicy>,
) -> anyhow::Result<Vec<u8>> {
	let default_policy;
	let policy = if let Some(policy) = policy {
		policy
	} else {
		default_policy = DownloadPolicy::disabled();
		&default_policy
	};
	let parsed_url = reqwest::Url::parse(&url)
		.map_err(|e| anyhow!("download policy reject: invalid URL `{url}` ({e})"))?;
	validate_download_url(&parsed_url, policy)?;

	let mut result = client.get(url.clone()).timeout(timeout).send().await?;
	let status = result.status();
	if !status.is_success() {
		return Err(anyhow!("Failed to download: {:?}", &result));
	}

	let final_url = result.url().clone();
	if parsed_url != final_url &&
		matches!(policy.trust_mode, DownloadTrustMode::Strict) &&
		!is_same_host(&parsed_url, &final_url)
	{
		return Err(anyhow!(
			"download policy reject: cross-host redirect is not allowed in strict mode (`{parsed_url}` -> `{final_url}`)"
		));
	}
	validate_download_url(&final_url, policy)?;

	let declared_content_length = result.content_length();
	if let Some(content_length) = declared_content_length {
		enforce_download_limits(content_length, policy.max_bytes)?;
	}

	let headers = result.headers().clone();
	let mut bytes = Vec::new();
	let mut total_bytes = 0u64;
	while let Some(chunk) = result.chunk().await? {
		total_bytes = total_bytes.saturating_add(chunk.len() as u64);
		enforce_download_limits(total_bytes, policy.max_bytes)?;
		bytes.extend_from_slice(&chunk);
	}

	trace!(
		?url,
		?headers,
		declared_content_length = ?declared_content_length,
		total_bytes,
		"Notary/notebook download complete",
	);

	if let Some(content_length) = declared_content_length {
		if total_bytes != content_length {
			return Err(anyhow!(
				"download content-length mismatch: declared={content_length}, actual={total_bytes}"
			));
		}
	}

	Ok(bytes)
}

fn validate_download_url(url: &reqwest::Url, policy: &DownloadPolicy) -> anyhow::Result<()> {
	if !matches!(url.scheme(), "http" | "https") {
		return Err(anyhow!(
			"download policy reject: unsupported scheme `{}` for `{}`",
			url.scheme(),
			url
		));
	}

	if matches!(policy.trust_mode, DownloadTrustMode::Strict) {
		if policy.expected_origin.is_none() {
			return Err(anyhow!(
				"download policy reject: strict mode requires expected origin for `{url}`"
			));
		}
		reject_private_or_loopback_target(url)?;
	}

	if let Some(expected_origin) = &policy.expected_origin {
		let expected_origin = reqwest::Url::parse(expected_origin).map_err(|e| {
			anyhow!("download policy reject: invalid expected origin `{expected_origin}` ({e})")
		})?;
		if !is_same_host(url, &expected_origin) {
			let expected_host = expected_origin.host_str().unwrap_or_default();
			let actual_host = url.host_str().unwrap_or_default();
			return Err(anyhow!(
				"download policy reject: host mismatch, expected `{expected_host}` got `{actual_host}`"
			));
		}
	}

	if let Some(expected_path_suffix) = &policy.expected_path_suffix {
		if !url.path().ends_with(expected_path_suffix) {
			return Err(anyhow!(
				"download policy reject: path mismatch, expected suffix `{expected_path_suffix}` got `{}`",
				url.path()
			));
		}
	}

	Ok(())
}

fn is_same_host(left: &reqwest::Url, right: &reqwest::Url) -> bool {
	left.host_str() == right.host_str()
}

fn reject_private_or_loopback_target(url: &reqwest::Url) -> anyhow::Result<()> {
	if !has_private_or_loopback_target(url) {
		return Ok(());
	}
	match url
		.host()
		.ok_or_else(|| anyhow!("download policy reject: missing host in `{url}`"))?
	{
		Host::Domain(host) => Err(anyhow!(
			"download policy reject: localhost target is not allowed in strict mode (`{host}`)"
		)),
		Host::Ipv4(ip) => Err(anyhow!(
			"download policy reject: private/loopback target is not allowed in strict mode (`{ip}`)"
		)),
		Host::Ipv6(ip) => Err(anyhow!(
			"download policy reject: private/loopback target is not allowed in strict mode (`{ip}`)"
		)),
	}
}

fn has_private_or_loopback_target(url: &reqwest::Url) -> bool {
	match url.host() {
		Some(Host::Domain(host)) =>
			host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost"),
		Some(Host::Ipv4(ipv4)) =>
			ipv4.is_private() || ipv4.is_loopback() || ipv4.is_link_local() || ipv4.is_unspecified(),
		Some(Host::Ipv6(ipv6)) =>
			ipv6.is_loopback() ||
				ipv6.is_unspecified() ||
				ipv6.is_unique_local() ||
				ipv6.is_unicast_link_local(),
		None => false,
	}
}

fn enforce_download_limits(total_bytes: u64, max_bytes: Option<u64>) -> anyhow::Result<()> {
	if let Some(max_bytes) = max_bytes {
		if total_bytes > max_bytes {
			return Err(anyhow!("download oversize: {total_bytes} bytes > {max_bytes} bytes"));
		}
	}
	Ok(())
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
	let url = Url::parse(url).map_err(|e| anyhow!("Invalid URL: {url:?} -> {e}"))?;

	let (sender, receiver) = transport_builder.build(url).await?;
	let client = ClientBuilder::default().build_with_tokio(sender, receiver);
	Ok(client)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn strict_policy(origin: &str, suffix: Option<&str>) -> DownloadPolicy {
		DownloadPolicy {
			trust_mode: DownloadTrustMode::Strict,
			expected_origin: Some(origin.to_string()),
			expected_path_suffix: suffix.map(ToString::to_string),
			max_bytes: None,
		}
	}

	#[test]
	fn get_download_path_suffix_builds_expected_paths() {
		assert_eq!(
			get_download_path_suffix(DownloadKind::Header, 3, 42),
			"/notary/3/header/42.scale"
		);
		assert_eq!(
			get_download_path_suffix(DownloadKind::Notebook, 3, 42),
			"/notary/3/notebook/42.scale"
		);
	}

	#[test]
	fn validate_download_url_rejects_strict_without_expected_origin() {
		let url = reqwest::Url::parse("https://archive.argonprotocol.org/notary/1/header/2.scale")
			.expect("valid url");
		let policy = DownloadPolicy {
			trust_mode: DownloadTrustMode::Strict,
			expected_origin: None,
			expected_path_suffix: None,
			max_bytes: None,
		};

		let err = validate_download_url(&url, &policy).expect_err("should reject");
		assert!(err.to_string().contains("strict mode requires expected origin"));
	}

	#[test]
	fn validate_download_url_accepts_same_host_even_if_scheme_or_port_differs() {
		let url =
			reqwest::Url::parse("http://archive.argonprotocol.org:8443/notary/1/header/2.scale")
				.expect("valid url");
		let policy =
			strict_policy("https://archive.argonprotocol.org", Some("/notary/1/header/2.scale"));

		validate_download_url(&url, &policy).expect("should accept same host");
	}

	#[test]
	fn validate_download_url_rejects_host_mismatch_in_strict_mode() {
		let url = reqwest::Url::parse("https://cdn.argonprotocol.org/notary/1/header/2.scale")
			.expect("valid url");
		let policy =
			strict_policy("https://archive.argonprotocol.org", Some("/notary/1/header/2.scale"));

		let err = validate_download_url(&url, &policy).expect_err("should reject");
		assert!(err.to_string().contains("host mismatch"));
	}

	#[test]
	fn validate_download_url_rejects_path_mismatch_in_strict_mode() {
		let url =
			reqwest::Url::parse("https://archive.argonprotocol.org/notary/1/header/999.scale")
				.expect("valid url");
		let policy =
			strict_policy("https://archive.argonprotocol.org", Some("/notary/1/header/2.scale"));

		let err = validate_download_url(&url, &policy).expect_err("should reject");
		assert!(err.to_string().contains("path mismatch"));
	}

	#[test]
	fn validate_download_url_accepts_matching_origin_and_path_in_strict_mode() {
		let url = reqwest::Url::parse("https://archive.argonprotocol.org/notary/1/header/2.scale")
			.expect("valid url");
		let policy =
			strict_policy("https://archive.argonprotocol.org", Some("/notary/1/header/2.scale"));

		validate_download_url(&url, &policy).expect("should be accepted");
	}

	#[test]
	fn validate_download_url_rejects_private_and_loopback_targets_in_strict_mode() {
		let loopback =
			reqwest::Url::parse("http://127.0.0.1/notary/1/header/2.scale").expect("valid url");
		let localhost =
			reqwest::Url::parse("http://localhost/notary/1/header/2.scale").expect("valid url");
		let private =
			reqwest::Url::parse("http://10.0.0.5/notary/1/header/2.scale").expect("valid url");

		let loopback_policy = strict_policy("http://127.0.0.1", None);
		let localhost_policy = strict_policy("http://localhost", None);
		let private_policy = strict_policy("http://10.0.0.5", None);

		assert!(validate_download_url(&loopback, &loopback_policy).is_err());
		assert!(validate_download_url(&localhost, &localhost_policy).is_err());
		assert!(validate_download_url(&private, &private_policy).is_err());
	}

	#[test]
	fn is_same_host_compares_host_only() {
		let a = reqwest::Url::parse("https://archive.argonprotocol.org/foo").expect("valid url");
		let b =
			reqwest::Url::parse("http://archive.argonprotocol.org:8443/bar").expect("valid url");
		let c = reqwest::Url::parse("https://cdn.argonprotocol.org/bar").expect("valid url");

		assert!(is_same_host(&a, &b));
		assert!(!is_same_host(&a, &c));
	}

	#[test]
	fn has_private_or_loopback_target_covers_expected_cases() {
		let urls = [
			("http://localhost/notary/1/header/2.scale", true),
			("http://foo.localhost/notary/1/header/2.scale", true),
			("http://127.0.0.1/notary/1/header/2.scale", true),
			("http://10.1.2.3/notary/1/header/2.scale", true),
			("http://[::1]/notary/1/header/2.scale", true),
			("http://8.8.8.8/notary/1/header/2.scale", false),
			("https://archive.argonprotocol.org/notary/1/header/2.scale", false),
		];

		for (url, expected) in urls {
			let url = reqwest::Url::parse(url).expect("valid url");
			assert_eq!(has_private_or_loopback_target(&url), expected, "{url}");
		}
	}

	#[test]
	fn enforce_download_limits_uses_single_max_threshold() {
		enforce_download_limits(10, Some(10)).expect("at max threshold should pass");
		enforce_download_limits(10, None).expect("no max should allow any size");

		let err = enforce_download_limits(11, Some(10)).expect_err("should reject");
		assert!(err.to_string().contains("download oversize"));
	}
}
