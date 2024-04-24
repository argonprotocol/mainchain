use std::{
	env,
	io::{BufRead, BufReader},
	path::PathBuf,
	process,
	process::Command,
};

use anyhow::anyhow;
use subxt::backend::{legacy::LegacyRpcMethods, rpc};

use ulixee_client::{UlxClient, UlxConfig};

pub struct TestContext {
	// Keep a handle to the node; once it's dropped the node is killed.
	proc: Option<process::Child>,
	pub rpc_client: rpc::RpcClient,
	pub rpc_methods: LegacyRpcMethods<UlxConfig>,
	pub client: UlxClient,
	pub ws_url: String,
}

impl Drop for TestContext {
	fn drop(&mut self) {
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
		}
	}
}

impl TestContext {
	pub async fn test_context_with(authority: String) -> anyhow::Result<Self> {
		let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

		let rust_log =
			format!("{},sc_rpc_server=info", env::var("RUST_LOG").unwrap_or("warn".to_string()));

		let workspace_cargo_path = project_dir.join("..");
		let workspace_cargo_path =
			workspace_cargo_path.canonicalize().expect("Failed to canonicalize path");
		let workspace_cargo_path = workspace_cargo_path.as_path().join("target/debug");
		let root = workspace_cargo_path.as_os_str();
		println!("run from {}", root.to_str().unwrap_or(""));

		let mut proc = Command::new("./ulx-node")
			.current_dir(root)
			.env("RUST_LOG", rust_log)
			.stderr(process::Stdio::piped())
			.arg("--dev")
			.arg(format!("--{}", authority.to_lowercase()))
			.arg("--miners=4")
			.arg("--port=0")
			.arg("--rpc-port=0")
			.spawn()?;

		// Wait for RPC port to be logged (it's logged to stderr).
		let stderr = proc.stderr.take().unwrap();

		let mut ws_port = None;
		for line in BufReader::new(stderr).lines().take(500) {
			let line = line.expect("failed to obtain next line from stdout for port discovery");

			let line_port = line
				.rsplit_once("Running JSON-RPC server: addr=127.0.0.1:")
				.map(|(_, port)| port);

			if let Some(line_port) = line_port {
				// trim non-numeric chars from the end of the port part of the line.
				let port_str = line_port.trim_end_matches(|b: char| !b.is_ascii_digit());

				// expect to have a number here (the chars after '127.0.0.1:') and parse them into a
				// u16.
				let port_num: u16 = port_str.parse().unwrap_or_else(|_| {
					panic!("valid port expected for log line, got '{port_str}'")
				});
				ws_port = Some(port_num);
				break;
			}
		}

		let ws_port = ws_port.expect("Failed to find ws port");

		let ws_url = format!("ws://127.0.0.1:{}", ws_port);

		let rpc_client = rpc::RpcClient::from_insecure_url(ws_url.as_str())
			.await
			.expect("Unable to connect RPC client to test node");

		let client = UlxClient::from_rpc_client(rpc_client.clone())
			.await
			.map_err(|e| anyhow!("Failed to connect to node at {ws_url}: {e}"))?;

		let methods = LegacyRpcMethods::new(rpc_client.clone());

		Ok(Self { proc: Some(proc), client, rpc_client, rpc_methods: methods, ws_url })
	}
}

pub async fn test_context_from_url(url: &str) -> TestContext {
	let rpc_client = rpc::RpcClient::from_insecure_url(url)
		.await
		.expect("Unable to connect RPC client to test node");

	let client = UlxClient::from_rpc_client(rpc_client.clone())
		.await
		.expect("Failed to connect to node at {url}: {e}");

	let rpc_methods = LegacyRpcMethods::new(rpc_client.clone());
	TestContext { proc: None, client, rpc_client, rpc_methods, ws_url: url.to_string() }
}

pub async fn test_context_with(authority: String) -> TestContext {
	TestContext::test_context_with(authority)
		.await
		.expect("Unable to create test context - ensure debug ulx-node build is available")
}

pub async fn test_context() -> TestContext {
	test_context_with("alice".to_string()).await
}
