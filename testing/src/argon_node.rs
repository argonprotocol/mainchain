#![allow(clippy::await_holding_lock)]

use bitcoin::hex::DisplayHex;
use bitcoind::{bitcoincore_rpc::Auth, BitcoinD};
use lazy_static::lazy_static;
use sp_core::{crypto::KeyTypeId, ed25519, Pair};
use sp_keyring::Sr25519Keyring;
use std::{env, process, process::Command, str::FromStr};
use subxt::backend::rpc::RpcParams;
use url::Url;

use crate::{get_target_dir, log_watcher::LogWatcher, start_bitcoind};
use argon_client::MainchainClient;
use argon_primitives::AccountId;

pub struct ArgonTestNode {
	// Keep a handle to the node; once it's dropped the node is killed.
	proc: Option<process::Child>,
	pub client: MainchainClient,
	pub bitcoind: Option<BitcoinD>,
	pub bitcoin_rpc_url: Option<Url>,
	pub account_id: AccountId,
	pub author_keyring_name: String,
	pub boot_url: String,
	pub log_watcher: LogWatcher,
}

impl Drop for ArgonTestNode {
	fn drop(&mut self) {
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
		}
		if let Some(mut bitcoind) = self.bitcoind.take() {
			let _ = bitcoind.stop();
		}
		self.log_watcher.close();
	}
}

lazy_static! {
	static ref CONTEXT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
impl ArgonTestNode {
	pub async fn start_with_bitcoin_rpc(
		authority: &str,
		compute_miners: u16,
		bootnodes: &str,
		bitcoin_rpc: Url,
	) -> anyhow::Result<Self> {
		let target_dir = get_target_dir();

		println!(
			"Starting argon-node with bitcoin rpc url: {} at {}",
			bitcoin_rpc,
			target_dir.display()
		);
		let overall_log = env::var("RUST_LOG").unwrap_or("info".to_string());
		let mut argon_log = match overall_log.as_str() {
			"trace" => "trace",
			"debug" => "debug",
			_ => "warn",
		};
		let node_log = match authority {
			"bob" => {
				argon_log = "trace";
				"trace"
			},
			_ => "info",
		};

		let rust_log = format!(
			"{},node={},runtime=trace,argon={},sc_rpc_server=info",
			overall_log, node_log, argon_log
		);

		let keyring = Sr25519Keyring::from_str(authority).expect("Invalid authority");
		let bootnodes_arg = if bootnodes.is_empty() {
			"--".to_string()
		} else {
			format!("--bootnodes={}", bootnodes)
		};

		let mut proc = Command::new("./argon-node")
			.current_dir(target_dir)
			.env("RUST_LOG", rust_log)
			.stderr(process::Stdio::piped())
			.arg("--dev")
			.arg("--detailed-log-output")
			.arg("--allow-private-ipv4")
			.arg("--state-pruning=archive")
			.arg(format!("--{}", authority.to_lowercase()))
			.arg(format!("--name={}", authority.to_lowercase()))
			.arg("--port=0")
			.arg("--rpc-port=0")
			.arg(format!("--compute-miners={}", compute_miners))
			.arg(format!("--bitcoin-rpc-url={}", bitcoin_rpc))
			.arg(bootnodes_arg)
			.spawn()?;

		// Wait for RPC port to be logged (it's logged to stderr).
		let stderr = proc.stderr.take().unwrap();
		let log_watch = LogWatcher::new(stderr);
		let port_matches = log_watch
			.wait_for_log(r"Running JSON-RPC server: addr=127.0.0.1:(\d+)", 1)
			.await?;
		assert_eq!(port_matches.len(), 1);
		println!("Started argon-node with RPC port {:?}", port_matches);
		let port = port_matches[0].parse::<u16>().expect("Failed to parse port");
		let ws_url = format!("ws://127.0.0.1:{}", port);

		let client = MainchainClient::from_url(ws_url.as_str()).await?;

		let listen_urls = client
			.rpc
			.request::<Vec<String>>("system_localListenAddresses", RpcParams::new())
			.await?;
		Ok(ArgonTestNode {
			proc: Some(proc),
			client,
			bitcoind: None,
			bitcoin_rpc_url: Some(bitcoin_rpc),
			account_id: keyring.to_account_id(),
			author_keyring_name: authority.to_string(),
			boot_url: listen_urls
				.into_iter()
				.find(|a| a.contains("127.0.0.1"))
				.expect("should have a localhost ip")
				.clone(),
			log_watcher: log_watch,
		})
	}

	pub async fn start(
		authority: &str,
		compute_miners: u16,
		bootnodes: &str,
	) -> anyhow::Result<Self> {
		#[allow(clippy::await_holding_lock)]
		let _lock = CONTEXT_LOCK.lock().unwrap();

		let (bitcoin, rpc_url, _) = start_bitcoind().map_err(|e| {
			eprintln!("ERROR starting bitcoind {:#?}", e);
			e
		})?;
		let mut node =
			Self::start_with_bitcoin_rpc(authority, compute_miners, bootnodes, rpc_url).await?;
		node.bitcoind = Some(bitcoin);
		Ok(node)
	}

	pub fn keyring(&self) -> Sr25519Keyring {
		Sr25519Keyring::from_str(self.author_keyring_name.as_str()).expect("Invalid keyring")
	}

	/// Inserts a key into the keystore and returns the public key.
	pub async fn insert_ed25519_keystore_key(
		&self,
		key_type: KeyTypeId,
		mnemonic: String,
	) -> anyhow::Result<[u8; 32]> {
		let key_type = String::from_utf8(key_type.0.to_vec()).expect("Invalid key type");
		let public_key = ed25519::Pair::from_string(mnemonic.as_str(), None)
			.expect("Invalid mnemonic")
			.public()
			.0;

		let mut params = RpcParams::new();
		params.push(key_type.to_string()).expect("should allow inserting key type");
		params.push(mnemonic).expect("should allow inserting mnemonic");
		params
			.push(public_key.as_hex().to_string())
			.expect("should allow inserting public key");

		println!("Inserting key {:?} {:?}", key_type, public_key);
		self.client.rpc.request::<()>("author_insertKey", params).await?;
		Ok(public_key)
	}

	pub fn get_bitcoin_url(&self) -> (String, Auth) {
		let rpc_url = self.bitcoin_rpc_url.clone().unwrap();

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
