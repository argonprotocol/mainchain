#![allow(clippy::await_holding_lock)]

use bitcoin::hex::DisplayHex;
use bitcoind::{bitcoincore_rpc::Auth, BitcoinD};
use lazy_static::lazy_static;
use polkadot_sdk::*;
use sp_core::{crypto::KeyTypeId, ed25519, Pair};
use sp_keyring::Sr25519Keyring;
use std::{
	env,
	fs::{create_dir_all, remove_dir_all},
	path::PathBuf,
	process,
	process::Command,
	str::FromStr,
	thread,
	time::{Duration, SystemTime, UNIX_EPOCH},
};
use subxt::ext::subxt_rpcs::client::RpcParams;
use url::Url;

use crate::{get_target_dir, log_watcher::LogWatcher, start_bitcoind};
use argon_client::MainchainClient;
use argon_primitives::AccountId;

pub struct ArgonTestNode {
	// Keep a handle to the node; once it's dropped the node is killed.
	proc: Option<process::Child>,
	pub client: MainchainClient,
	pub bitcoind: Option<BitcoinD>,
	pub account_id: AccountId,
	pub author_keyring_name: String,
	pub boot_url: String,
	pub log_watcher: LogWatcher,
	pub start_args: ArgonNodeStartArgs,
}

#[derive(Clone, Debug)]
pub struct ArgonNodeStartArgs {
	pub target_dir: PathBuf,
	pub rpc_port: u16,
	pub bootnodes: String,
	pub base_data_path: PathBuf,
	pub rust_log: String,
	pub authority: String,
	pub is_validator: bool,
	pub compute_miners: u16,
	pub bitcoin_rpc: String,
	pub notebook_archive_urls: Vec<String>,
	pub extra_flags: Vec<String>,
	pub is_archive_node: bool,
}

impl ArgonNodeStartArgs {
	pub fn bitcoin_rpc_url(&self) -> anyhow::Result<Url> {
		Url::parse(&self.bitcoin_rpc).map_err(|e| anyhow::anyhow!(e))
	}

	pub fn get_temp_dir() -> anyhow::Result<PathBuf> {
		let thread_id = format!("{:?}", thread::current().id())
			.replace("ThreadId(", "")
			.replace(")", "")
			.replace("\"", "");
		let unique_name = format!(
			"test_dir_{}_{}",
			SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros(),
			thread_id
		);
		let base_dir = env::temp_dir().join("argon_node").join(unique_name);
		create_dir_all(base_dir.clone())?;
		Ok(base_dir)
	}

	pub fn get_account_id(&self) -> AccountId {
		let keyring = Sr25519Keyring::from_str(&self.authority).expect("Invalid authority");
		keyring.to_account_id()
	}

	pub fn new(authority: &str, compute_miners: u16, bootnodes: &str) -> anyhow::Result<Self> {
		let target_dir = get_target_dir();

		let overall_log = env::var("RUST_LOG").unwrap_or("info".to_string());

		let node_log = "info";

		let rust_log = format!(
			"{},node={},pallet=info,argon=trace,sc_rpc_server=info,argon_notary_apis=info",//,grandpa=trace,runtime::grandpa=trace",
			overall_log, node_log,
		);

		Ok(Self {
			target_dir,
			rust_log,
			base_data_path: Self::get_temp_dir()?,
			bootnodes: bootnodes.to_string(),
			compute_miners,
			bitcoin_rpc: "".to_string(),
			authority: authority.to_lowercase().to_string(),
			is_validator: true,
			rpc_port: 0,
			notebook_archive_urls: vec![],
			extra_flags: vec![],
			is_archive_node: true,
		})
	}

	pub fn with_bitcoin_url(
		authority: &str,
		compute_miners: u16,
		bootnodes: &str,
		bitcoin_rpc: Url,
	) -> anyhow::Result<Self> {
		let args = Self::new(authority, compute_miners, bootnodes)?;
		println!(
			"Starting argon-node with bitcoin rpc url: {} at {}",
			bitcoin_rpc,
			args.target_dir.display()
		);
		Ok(args)
	}
}

impl Drop for ArgonTestNode {
	fn drop(&mut self) {
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
		}
		remove_dir_all(self.start_args.base_data_path.clone()).unwrap();
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
	/// Restart the node with the same rpc port. NOTE: does not currently support the p2p address
	/// too
	///
	/// If you want to add that, you need to capture the boot url, and also probably use
	/// pre-generated libp2p nodeIds.
	pub async fn restart(&mut self, wait_duration: Duration) -> anyhow::Result<()> {
		self.log_watcher.close();
		self.proc.take().unwrap().kill()?;
		tokio::time::sleep(wait_duration).await;
		let (proc, log_watcher) = Self::start_process(&mut self.start_args).await?;
		self.proc = Some(proc);
		self.log_watcher = log_watcher;
		self.client = MainchainClient::from_url(self.client.url.as_str()).await?;
		Ok(())
	}

	pub async fn fork_node(&self, authority: &str, miner_threads: u16) -> anyhow::Result<Self> {
		let fork_args = self.get_fork_args(authority, miner_threads);
		self.fork_node_with(fork_args).await
	}

	pub fn get_fork_args(&self, authority: &str, miner_threads: u16) -> ArgonNodeStartArgs {
		let mut new_args = self.start_args.clone();
		let base_data_path = ArgonNodeStartArgs::get_temp_dir().unwrap();
		new_args.authority = authority.to_string();
		new_args.rpc_port = 0;
		new_args.base_data_path = base_data_path;
		new_args.bootnodes = self.boot_url.clone();
		new_args.compute_miners = miner_threads;
		new_args
	}

	pub async fn fork_node_with(&self, fork_args: ArgonNodeStartArgs) -> anyhow::Result<Self> {
		Self::start(fork_args).await
	}

	async fn start_process(
		args: &mut ArgonNodeStartArgs,
	) -> anyhow::Result<(process::Child, LogWatcher)> {
		let mut more_args = args.extra_flags.clone();
		if !args.bootnodes.is_empty() {
			more_args.push(format!("--bootnodes={}", &args.bootnodes))
		};
		if !args.notebook_archive_urls.is_empty() {
			more_args
				.push(format!("--notebook-archive-hosts={}", args.notebook_archive_urls.join(",")))
		};

		if args.is_validator {
			more_args.push("--validator".to_string());
		}

		if args.is_archive_node {
			more_args.push("--state-pruning=archive".to_string());
			more_args.push("--blocks-pruning=archive".to_string());
		}

		println!("Starting argon-node with args: {:?}. {:?}", args, more_args);

		let mut proc = Command::new("./argon-node")
			.current_dir(args.target_dir.clone())
			.env("RUST_LOG", &args.rust_log)
			.stderr(process::Stdio::piped())
			.stdout(process::Stdio::null()) // Redirect stdout to /dev/null
			.arg("--no-mdns")
			.arg(format!("--base-path={}", args.base_data_path.display()))
			.arg("--detailed-log-output")
			.arg("--unsafe-force-node-key-generation")
			.arg("--allow-private-ipv4")
			.arg("--no-telemetry")
			.arg("--no-prometheus")
			.arg("--chain=dev")
			.arg(format!("--{}", &args.authority.to_lowercase()))
			.arg(format!("--name={}", &args.authority.to_lowercase()))
			.arg("--port=0")
			.arg(format!("--rpc-port={}", args.rpc_port))
			.arg(format!("--compute-miners={}", args.compute_miners))
			.arg(format!("--bitcoin-rpc-url={}", &args.bitcoin_rpc))
			.args(more_args.into_iter())
			.spawn()?;

		// Wait for RPC port to be logged (it's logged to stderr).
		let stderr = proc.stderr.take().unwrap();
		let log_watch = LogWatcher::new(stderr, &args.authority);
		let port_matches = log_watch
			.wait_for_log(r"Running JSON-RPC server: addr=127.0.0.1:(\d+)", 1)
			.await?;
		assert_eq!(port_matches.len(), 1);
		println!("Started argon-node with RPC port {:?}", port_matches);
		args.rpc_port = port_matches[0].parse::<u16>().expect("Failed to parse port");
		Ok((proc, log_watch))
	}

	pub async fn start_with_args(
		authority: &str,
		compute_miners: u16,
	) -> anyhow::Result<ArgonTestNode> {
		let args = ArgonNodeStartArgs::new(authority, compute_miners, "")?;
		Self::start(args).await
	}

	pub async fn start(mut args: ArgonNodeStartArgs) -> anyhow::Result<Self> {
		#[allow(clippy::await_holding_lock)]
		let _lock = CONTEXT_LOCK.lock().unwrap();

		let mut bitcoind = None;
		if args.bitcoin_rpc.is_empty() {
			let (bitcoin, rpc_url, _) = start_bitcoind().map_err(|e| {
				eprintln!("ERROR starting bitcoind {:#?}", e);
				e
			})?;
			bitcoind = Some(bitcoin);
			args.bitcoin_rpc = rpc_url.to_string();
		}
		let (proc, log_watch) = Self::start_process(&mut args).await?;
		let ws_url = format!("ws://127.0.0.1:{}", args.rpc_port);

		let client = MainchainClient::from_url(ws_url.as_str()).await?;

		let listen_urls = client
			.rpc
			.request::<Vec<String>>("system_localListenAddresses", RpcParams::new())
			.await?;
		Ok(ArgonTestNode {
			proc: Some(proc),
			client,
			bitcoind,
			account_id: args.get_account_id(),
			author_keyring_name: args.authority.to_string(),
			boot_url: listen_urls
				.into_iter()
				.find(|a| a.contains("127.0.0.1"))
				.expect("should have a localhost ip")
				.clone(),
			log_watcher: log_watch,
			start_args: args.clone(),
		})
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
		let mut rpc_url = self.start_args.bitcoin_rpc_url().unwrap();

		let auth = if !rpc_url.username().is_empty() {
			Auth::UserPass(
				rpc_url.username().to_string(),
				rpc_url.password().unwrap_or_default().to_string(),
			)
		} else {
			Auth::None
		};
		rpc_url.set_password(None).unwrap();
		rpc_url.set_username("").unwrap();
		(rpc_url.to_string(), auth)
	}
}
