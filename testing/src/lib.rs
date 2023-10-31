use std::path::PathBuf;

use anyhow::anyhow;
use substrate_runner::SubstrateNode;
use subxt::backend::rpc;

use ulixee_client::UlxClient;

pub struct TestContext {
	// Keep a handle to the node; once it's dropped the node is killed.
	_proc: Option<SubstrateNode>,
	pub rpc_client: rpc::RpcClient,
	pub client: UlxClient,
	pub ws_url: String,
}

impl TestContext {
	pub async fn test_context_with(authority: String) -> anyhow::Result<Self> {
		let mut node_builder = SubstrateNode::builder();

		let mut relative_dir = PathBuf::from(module_path!());
		relative_dir.push("../../target/release/ulx-node");

		node_builder.binary_paths(vec![
			PathBuf::from("./target/release/ulx-node").into_os_string(),
			PathBuf::from("../target/release/ulx-node").into_os_string(),
			relative_dir.into_os_string(),
		]);
		node_builder.arg(authority.to_lowercase());
		node_builder.arg_val("miners", "4");

		// Spawn the node and retrieve a URL to it:
		let proc = node_builder.spawn().map_err(|e| {
			panic!("Unable to launch a ulx-node binary - {e}. Make sure you build a release for tests.\n\ncargo build --release --features=fast-runtime");
		}).expect("Unable to launch a ulx-node binary");

		let ws_url = format!("ws://127.0.0.1:{}", proc.ws_port());

		let client = UlxClient::from_url(&ws_url)
			.await
			.map_err(|e| anyhow!("Failed to connect to node at {ws_url}: {e}"))?;

		let rpc_client = rpc::RpcClient::from_url(ws_url.as_str())
			.await
			.expect("Unable to connect RPC client to test node");

		Ok(Self { _proc: Some(proc), client, rpc_client, ws_url })
	}
}

pub async fn test_context_from_url(url: &str) -> TestContext {
	TestContext {
		_proc: None,
		client: UlxClient::from_url(url).await.unwrap(),
		rpc_client: rpc::RpcClient::from_url(url).await.unwrap(),
		ws_url: url.to_string(),
	}
}

pub async fn test_context_with(authority: String) -> TestContext {
	TestContext::test_context_with(authority).await.unwrap()
}

pub async fn test_context() -> TestContext {
	test_context_with("alice".to_string()).await
}
