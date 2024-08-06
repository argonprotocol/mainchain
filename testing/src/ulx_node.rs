#![allow(clippy::await_holding_lock)]

use crate::{bitcoind::read_rpc_url, start_bitcoind};
use bitcoind::{bitcoincore_rpc::Auth, BitcoinD};
use lazy_static::lazy_static;
use std::{
	env,
	io::{BufRead, BufReader},
	path::PathBuf,
	process,
	process::Command,
};
use ulixee_client::MainchainClient;
use url::Url;

pub struct UlxTestNode {
	// Keep a handle to the node; once it's dropped the node is killed.
	proc: Option<process::Child>,
	pub client: MainchainClient,
	pub bitcoind: Option<BitcoinD>,
	pub bitcoin_rpc_url: Option<Url>,
}

impl Drop for UlxTestNode {
	fn drop(&mut self) {
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
		}
		if let Some(mut bitcoind) = self.bitcoind.take() {
			let _ = bitcoind.stop();
		}
	}
}

lazy_static! {
	static ref CONTEXT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
impl UlxTestNode {
	pub async fn start(authority: String) -> anyhow::Result<Self> {
		#[allow(clippy::await_holding_lock)]
		let _lock = CONTEXT_LOCK.lock().unwrap();
		let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

		let (bitcoin, rpc_url, _) = start_bitcoind().map_err(|e| {
			eprintln!("ERROR starting bitcoind {:#?}", e);
			e
		})?;
		let rust_log =
			format!("{},sc_rpc_server=info", env::var("RUST_LOG").unwrap_or("warn".to_string()));

		let workspace_cargo_path = project_dir.join("..");
		let workspace_cargo_path =
			workspace_cargo_path.canonicalize().expect("Failed to canonicalize path");
		let workspace_cargo_path = workspace_cargo_path.as_path().join("target/debug");
		let root = workspace_cargo_path.as_os_str();

		println!("Starting ulx-node with bitcoin rpc url: {}", rpc_url);

		let mut proc = Command::new("./ulx-node")
			.current_dir(root)
			.env("RUST_LOG", rust_log)
			.stderr(process::Stdio::piped())
			.arg("--dev")
			.arg(format!("--{}", authority.to_lowercase()))
			.arg("--compute-miners=4")
			.arg("--port=0")
			.arg("--rpc-port=0")
			.arg(format!("--bitcoin-rpc-url={}", rpc_url))
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

		let client =
			MainchainClient::from_url(format!("ws://127.0.0.1:{}", ws_port).as_str()).await?;

		Ok(UlxTestNode {
			proc: Some(proc),
			client,
			bitcoind: Some(bitcoin),
			bitcoin_rpc_url: Some(rpc_url),
		})
	}

	pub async fn from_url(url: String, bitcoind: Option<BitcoinD>) -> Self {
		let client = MainchainClient::from_url(url.as_str())
			.await
			.expect("Failed to connect to node at {url}: {e}");

		let bitcoin_rpc_url = bitcoind.as_ref().map(|b| read_rpc_url(b).unwrap());
		UlxTestNode { proc: None, client, bitcoind, bitcoin_rpc_url }
	}

	pub fn get_bitcoin_url(&self) -> (String, Auth) {
		let rpc_url = self.bitcoin_rpc_url.clone().unwrap();

		println!("rpc_url: {:?}", rpc_url);
		let auth = if !rpc_url.username().is_empty() {
			Auth::UserPass(
				rpc_url.username().to_string(),
				rpc_url.password().unwrap_or_default().to_string(),
			)
		} else {
			Auth::None
		};
		(rpc_url.to_string(), auth)
	}
}
