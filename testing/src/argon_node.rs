#![allow(clippy::await_holding_lock)]

use std::{
	env,
	io::{BufRead, BufReader},
	process,
	process::Command,
	sync::mpsc,
};

use bitcoind::{bitcoincore_rpc::Auth, BitcoinD};
use lazy_static::lazy_static;
use tokio::task::spawn_blocking;
use url::Url;

use argon_client::MainchainClient;

use crate::{bitcoind::read_rpc_url, get_target_dir, start_bitcoind};

pub struct ArgonTestNode {
	// Keep a handle to the node; once it's dropped the node is killed.
	proc: Option<process::Child>,
	pub client: MainchainClient,
	pub bitcoind: Option<BitcoinD>,
	pub bitcoin_rpc_url: Option<Url>,
}

impl Drop for ArgonTestNode {
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
impl ArgonTestNode {
	pub async fn start(authority: String, compute_miners: u16) -> anyhow::Result<Self> {
		#[allow(clippy::await_holding_lock)]
		let _lock = CONTEXT_LOCK.lock().unwrap();

		let (bitcoin, rpc_url, _) = start_bitcoind().map_err(|e| {
			eprintln!("ERROR starting bitcoind {:#?}", e);
			e
		})?;
		let overall_log = env::var("RUST_LOG").unwrap_or("warn".to_string());
		let argon_log = match overall_log.as_str() {
			"trace" => "trace",
			"debug" => "debug",
			_ => "info",
		};

		// set cumulus_relay_chain to info to ensure we get the PARACHAIN prefix
		let rust_log = format!(
			"{},argon={},cumulus_relay_chain=info,sc_rpc_server=info",
			overall_log, argon_log
		);

		let target_dir = get_target_dir();

		println!(
			"Starting argon-node with bitcoin rpc url: {} at {}",
			rpc_url,
			target_dir.display()
		);

		let mut proc = Command::new("./argon-node")
			.current_dir(target_dir)
			.env("RUST_LOG", rust_log)
			.stderr(process::Stdio::piped())
			.arg("--dev")
			.arg("--detailed-log-output")
			.arg(format!("--{}", authority.to_lowercase()))
			.arg("--port=0")
			.arg("--rpc-port=0")
			.arg(format!("--compute-miners={}", compute_miners))
			.arg(format!("--bitcoin-rpc-url={}", rpc_url))
			.spawn()?;

		// Wait for RPC port to be logged (it's logged to stderr).
		let stderr = proc.stderr.take().unwrap();
		let stderr_reader = BufReader::new(stderr);
		let (tx, rx) = mpsc::channel();

		let tx_clone = tx.clone();

		spawn_blocking(move || {
			for line in stderr_reader.lines() {
				let line = line.expect("failed to obtain next line from stdout");

				let line_port = line
					.rsplit_once("Running JSON-RPC server: addr=127.0.0.1:")
					.map(|(_, port)| port);

				if let Some(mut line_port) = line_port {
					if line_port.contains(",") {
						line_port = line_port.split(',').next().unwrap();
					}
					// trim non-numeric chars from the end of the port part of the line.
					let port_str = line_port.trim_end_matches(|b: char| !b.is_ascii_digit());

					// expect to have a number here (the chars after '127.0.0.1:') and parse them
					// into a u16.
					let port_num: u16 = port_str.parse().unwrap_or_else(|_| {
						panic!("valid port expected for log line, got '{port_str}'")
					});
					let ws_url = format!("ws://127.0.0.1:{}", port_num);
					tx_clone.send(ws_url).unwrap();
					break;
				}
			}
		});

		let ws_url = rx.recv().expect("Failed to start node");

		let client = MainchainClient::from_url(ws_url.as_str()).await?;

		Ok(ArgonTestNode {
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
		ArgonTestNode { proc: None, client, bitcoind, bitcoin_rpc_url }
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
