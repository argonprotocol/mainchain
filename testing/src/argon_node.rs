use anyhow::{bail, Context};
use bitcoin::hex::DisplayHex;
use bitcoind::{bitcoincore_rpc::Auth, BitcoinD};
use polkadot_sdk::*;
use sp_core::{crypto::KeyTypeId, ed25519, Pair};
use sp_keyring::Sr25519Keyring;
use std::{
	env,
	fs::{create_dir, create_dir_all, remove_dir_all},
	path::PathBuf,
	process,
	process::Command,
	str::FromStr,
	time::Duration,
};
use subxt::ext::subxt_rpcs::client::RpcParams;
use url::Url;
use uuid::Uuid;

use crate::{get_target_dir, log_watcher::LogWatcher, start_bitcoind};
use argon_client::MainchainClient;
use argon_primitives::AccountId;

const NODE_STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
static NODE_START_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub struct ArgonTestNode {
	// Keep a handle to the node; once it's dropped the node is killed.
	proc: Option<NodeProcess>,
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
	pub chain: String,
	pub rpc_port: u16,
	pub bootnodes: String,
	pub base_data_path: PathBuf,
	pub cleanup_base_data_path: bool,
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
		let test_root = env::temp_dir().join("argon_node");
		create_dir_all(&test_root)?;
		let base_dir = test_root.join(format!("test_dir_{}", Uuid::new_v4()));
		create_dir(&base_dir)?;
		Ok(base_dir)
	}

	pub fn get_account_id(&self) -> AccountId {
		let keyring = Sr25519Keyring::from_str(&self.authority).expect("Invalid authority");
		keyring.to_account_id()
	}

	pub fn command(&self) -> Command {
		let mut more_args = self.extra_flags.clone();
		if !self.bootnodes.is_empty() {
			more_args.push(format!("--bootnodes={}", &self.bootnodes));
		}
		if !self.notebook_archive_urls.is_empty() {
			more_args
				.push(format!("--notebook-archive-hosts={}", self.notebook_archive_urls.join(",")));
		}
		if self.is_validator {
			more_args.push("--validator".to_string());
		}
		if self.is_archive_node {
			more_args.push("--state-pruning=archive".to_string());
			more_args.push("--blocks-pruning=archive".to_string());
		}

		let mut command = Command::new("./argon-node");
		command
			.current_dir(self.target_dir.clone())
			.env("RUST_LOG", &self.rust_log)
			.stderr(process::Stdio::piped())
			.stdout(process::Stdio::null())
			.arg("--no-mdns")
			.arg(format!("--base-path={}", self.base_data_path.display()))
			.arg("--detailed-log-output")
			.arg("--unsafe-force-node-key-generation")
			.arg("--allow-private-ipv4")
			.arg("--no-telemetry")
			.arg("--no-prometheus")
			.arg(format!("--chain={}", self.chain))
			.arg(format!("--{}", self.authority.to_lowercase()))
			.arg(format!("--name={}", self.authority.to_lowercase()))
			.arg("--port=0")
			.arg(format!("--rpc-port={}", self.rpc_port))
			.arg(format!("--compute-miners={}", self.compute_miners))
			.arg(format!("--bitcoin-rpc-url={}", &self.bitcoin_rpc))
			.args(more_args);
		command
	}

	pub fn new(authority: &str, compute_miners: u16, bootnodes: &str) -> anyhow::Result<Self> {
		let target_dir = get_target_dir();

		let overall_log = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
		let rust_log = format!(
			"{overall_log},node=info,pallet=info,argon=trace,sc_rpc_server=info,argon_notary_apis=info",
		);

		// CI uses lower chain difficulty, so cache-only mining can avoid dataset setup.
		let mut extra_flags = vec![];
		if env::var("CI").is_ok() {
			extra_flags.push("--compute-flags=light".to_string());
			if cfg!(target_os = "macos") {
				// Hosted macOS can temporarily lose its resolver configuration. Libp2p can
				// still connect these local IP-only networks through its TCP fallback.
				extra_flags.push("--network-backend=libp2p".to_string());
			}
		}

		Ok(Self {
			target_dir,
			chain: env::var("ARGON_TEST_CHAIN").unwrap_or_else(|_| "dev".to_string()),
			rust_log,
			base_data_path: Self::get_temp_dir()?,
			cleanup_base_data_path: true,
			bootnodes: bootnodes.to_string(),
			compute_miners,
			bitcoin_rpc: "".to_string(),
			authority: authority.to_lowercase(),
			is_validator: true,
			rpc_port: 0,
			notebook_archive_urls: vec![],
			extra_flags,
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
		drop(self.proc.take());
		self.log_watcher.close();
		if self.start_args.cleanup_base_data_path {
			let base_data_path = self.start_args.base_data_path.clone();
			if let Err(error) = remove_dir_all(&base_data_path) &&
				error.kind() != std::io::ErrorKind::NotFound
			{
				eprintln!("Failed to remove node base path {}: {error}", base_data_path.display());
			}
		}
		if let Some(mut bitcoind) = self.bitcoind.take() {
			let _ = bitcoind.stop();
		}
	}
}

impl ArgonTestNode {
	pub fn stop(&mut self) -> anyhow::Result<()> {
		if let Some(mut proc) = self.proc.take() {
			proc.stop()?;
		}
		self.log_watcher.close();
		Ok(())
	}

	/// Restart the node with the same RPC port and refresh the local bootnode address.
	pub async fn restart(&mut self, wait_duration: Duration) -> anyhow::Result<()> {
		self.stop()?;
		tokio::time::sleep(wait_duration).await;
		let mut start_args = self.start_args.clone();
		let (proc, log_watcher, client, boot_url) =
			Self::start_process(&mut start_args, NODE_STARTUP_TIMEOUT).await?;

		self.proc = Some(proc);
		self.log_watcher = log_watcher;
		self.client = client;
		self.boot_url = boot_url;
		self.start_args = start_args;
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

	pub async fn start_with_args(
		authority: &str,
		compute_miners: u16,
	) -> anyhow::Result<ArgonTestNode> {
		let args = ArgonNodeStartArgs::new(authority, compute_miners, "")?;
		Self::start(args).await
	}

	pub async fn start(mut args: ArgonNodeStartArgs) -> anyhow::Result<Self> {
		let _lock = NODE_START_LOCK.lock().await;

		let mut bitcoind = None;
		if args.bitcoin_rpc.is_empty() {
			let (bitcoin, rpc_url, _) = start_bitcoind().map_err(|e| {
				eprintln!("ERROR starting bitcoind {e:#?}");
				e
			})?;
			bitcoind = Some(bitcoin);
			args.bitcoin_rpc = rpc_url.to_string();
		}
		let (proc, log_watcher, client, boot_url) =
			Self::start_process(&mut args, NODE_STARTUP_TIMEOUT).await?;
		Ok(ArgonTestNode {
			proc: Some(proc),
			client,
			bitcoind,
			account_id: args.get_account_id(),
			author_keyring_name: args.authority.to_string(),
			boot_url,
			log_watcher,
			start_args: args,
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

		println!("Inserting key {key_type:?} {public_key:?}");
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

	async fn start_process(
		args: &mut ArgonNodeStartArgs,
		startup_timeout: Duration,
	) -> anyhow::Result<(NodeProcess, LogWatcher, MainchainClient, String)> {
		println!("Starting argon-node with args: {args:?}");
		let mut proc = NodeProcess { child: args.command().spawn()? };
		let stderr = proc.child.stderr.take().context("node stderr should be piped")?;
		let log_watcher = LogWatcher::new(stderr, &args.authority);
		let mut phase = "RPC startup";

		let ready: anyhow::Result<_> = match tokio::time::timeout(startup_timeout, async {
			let port_matches = log_watcher
				.wait_for_log(r"Running JSON-RPC server: addr=127.0.0.1:(\d+)", 1)
				.await?;
			let rpc_port = port_matches
				.first()
				.context("node RPC port was not logged")?
				.parse::<u16>()
				.context("invalid node RPC port")?;
			let ws_url = format!("ws://127.0.0.1:{rpc_port}");

			phase = "RPC connection";
			// Fast sync can expose RPC before the finalized block's state is available.
			let client = MainchainClient::try_until_connected(
				&ws_url,
				100,
				startup_timeout.as_millis() as u64,
			)
			.await?;

			phase = "P2P listener";
			loop {
				if let Some(status) = proc.child.try_wait()? {
					bail!("node exited before startup completed: {status}");
				}
				let listen_urls = client
					.rpc
					.request::<Vec<String>>("system_localListenAddresses", RpcParams::new())
					.await?;
				if let Some(address) =
					listen_urls.into_iter().find(|address| address.contains("127.0.0.1"))
				{
					return Ok((rpc_port, client, normalize_bootnode_address(address)));
				}
				tokio::time::sleep(Duration::from_millis(100)).await;
			}
		})
		.await
		{
			Ok(result) => result,
			Err(_) => Err(anyhow::anyhow!("timed out after {startup_timeout:?}")),
		};
		let (rpc_port, client, boot_url) = ready.with_context(|| {
			format!(
				"Node {} startup failed during {phase}. Recent logs:\n{}",
				args.authority,
				log_watcher.tail(40),
			)
		})?;

		args.rpc_port = rpc_port;
		println!("Started argon-node with RPC port {rpc_port} and bootnode {boot_url}");
		Ok((proc, log_watcher, client, boot_url))
	}
}

struct NodeProcess {
	child: process::Child,
}

impl NodeProcess {
	fn stop(&mut self) -> std::io::Result<()> {
		if self.child.try_wait()?.is_none() {
			self.child.kill()?;
		}
		self.child.wait()?;
		Ok(())
	}
}

impl Drop for NodeProcess {
	fn drop(&mut self) {
		let _ = self.stop();
	}
}

fn normalize_bootnode_address(address: String) -> String {
	let Some((base, first_peer_segment)) = address.split_once("/p2p/") else {
		return address;
	};
	let Some((peer_id, repeated_peer_segments)) = first_peer_segment.split_once("/p2p/") else {
		return address;
	};

	if repeated_peer_segments.split("/p2p/").all(|segment| segment == peer_id) {
		format!("{base}/p2p/{peer_id}")
	} else {
		address
	}
}

#[cfg(all(test, unix))]
mod tests {
	use super::*;
	use bitcoin::hex::FromHex;
	use sp_core::H256;
	use std::{
		fs,
		os::unix::fs::PermissionsExt,
		sync::{
			atomic::{AtomicUsize, Ordering},
			Arc,
		},
	};
	use subxt::ext::{
		codec::Encode,
		frame_metadata::OpaqueMetadata,
		jsonrpsee::{core::JsonValue, server::ServerBuilder, types::ErrorObjectOwned, RpcModule},
		subxt_rpcs::methods::legacy::RuntimeVersion,
	};

	#[tokio::test]
	async fn startup_recovers_when_runtime_state_becomes_available() {
		let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
		let rpc_port = server.local_addr().unwrap().port();
		let runtime_requests = Arc::new(AtomicUsize::new(0));
		let mut rpc = RpcModule::new(runtime_requests.clone());
		rpc.register_method("state_getRuntimeVersion", |_, requests, _| {
			if requests.fetch_add(1, Ordering::SeqCst) == 0 {
				return Err(ErrorObjectOwned::owned(
					4003,
					"Client error: Api called for an unknown Block: State already discarded",
					None::<()>,
				));
			}
			Ok(RuntimeVersion {
				spec_version: 160,
				transaction_version: 1,
				other: Default::default(),
			})
		})
		.unwrap();
		rpc.register_method("chain_getBlockHash", |_, _, _| {
			Ok::<_, ErrorObjectOwned>(H256::repeat_byte(1))
		})
		.unwrap();
		rpc.register_method("chain_getFinalizedHead", |_, _, _| {
			Ok::<_, ErrorObjectOwned>(H256::repeat_byte(1))
		})
		.unwrap();
		let metadata: JsonValue =
			include_str!("../../client/nodejs/metadata.json").parse().unwrap();
		let metadata = OpaqueMetadata(
			Vec::from_hex(metadata["result"].as_str().unwrap().trim_start_matches("0x")).unwrap(),
		)
		.encode();
		let metadata = format!("0x{}", metadata.as_hex());
		rpc.register_method("state_call", move |params, _, _| {
			let (method, _, _): (String, String, H256) = params.parse()?;
			if method == "Metadata_metadata" {
				Ok(metadata.clone())
			} else if method == "Metadata_metadata_at_version" {
				Ok("0x00".to_string())
			} else {
				Err(ErrorObjectOwned::owned(-32601, "Method not found", None::<()>))
			}
		})
		.unwrap();
		rpc.register_method("state_getStorage", |_, _, _| None::<String>).unwrap();
		rpc.register_method("system_localListenAddresses", |_, _, _| {
			vec![
				"/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWD8WrhiunjNThrRGB2VKWHd2exaeJ4Cb5vAysWfZZdMrs",
			]
		})
		.unwrap();
		rpc.register_subscription(
			"state_subscribeRuntimeVersion",
			"state_runtimeVersion",
			"state_unsubscribeRuntimeVersion",
			|_, pending, _, _| async move {
				if let Ok(sink) = pending.accept().await {
					sink.closed().await;
				}
			},
		)
		.unwrap();
		let server_handle = server.start(rpc);

		let (mut args, _) = startup_stub(Some(rpc_port));
		let data_path = args.base_data_path.clone();
		let (proc, _, client, boot_url) =
			ArgonTestNode::start_process(&mut args, Duration::from_secs(5))
				.await
				.expect("startup should recover after the missing-state error");

		assert!(runtime_requests.load(Ordering::SeqCst) >= 2);
		assert_eq!(client.live.runtime_version().spec_version, 160);
		assert!(boot_url.starts_with("/ip4/127.0.0.1/tcp/30333/p2p/"));
		assert_eq!(args.rpc_port, rpc_port);

		drop(client);
		drop(proc);
		remove_dir_all(data_path).unwrap();
		server_handle.stop().unwrap();
		server_handle.stopped().await;
	}

	#[tokio::test]
	async fn rpc_connection_retries_until_timeout_and_kills_the_node_process() {
		let (mut args, pid_path) = startup_stub(Some(0));
		let data_path = args.base_data_path.clone();
		let startup_timeout = Duration::from_secs(2);
		let started_at = std::time::Instant::now();
		let error = ArgonTestNode::start_process(&mut args, startup_timeout)
			.await
			.err()
			.expect("RPC connection should time out");

		let pid = fs::read_to_string(&pid_path).unwrap();
		let alive = Command::new("/bin/kill").args(["-0", pid.trim()]).output().unwrap();
		if alive.status.success() {
			Command::new("/bin/kill").args(["-KILL", pid.trim()]).output().unwrap();
		}
		remove_dir_all(data_path).unwrap();
		assert!(!alive.status.success(), "startup error left the node process alive");
		assert!(started_at.elapsed() >= startup_timeout, "startup failed before retrying");
		let message = format!("{error:#}");
		assert!(message.contains("RPC connection"), "{message}");
		assert!(message.contains("timed out"), "{message}");
	}

	#[tokio::test]
	async fn cancelling_startup_kills_the_node_process() {
		let (args, pid_path) = startup_stub(None);
		let data_path = args.base_data_path.clone();
		{
			let startup = ArgonTestNode::start(args);
			tokio::pin!(startup);
			tokio::select! {
				_ = &mut startup => panic!("node unexpectedly finished startup"),
				ready = tokio::time::timeout(Duration::from_secs(5), async {
					while !pid_path.exists() {
						tokio::time::sleep(Duration::from_millis(10)).await;
					}
				}) => ready.expect("node process did not start"),
			}
		}

		let pid = fs::read_to_string(&pid_path).unwrap();
		let alive = Command::new("/bin/kill").args(["-0", pid.trim()]).output().unwrap();
		if alive.status.success() {
			Command::new("/bin/kill").args(["-KILL", pid.trim()]).output().unwrap();
		}
		remove_dir_all(data_path).unwrap();
		assert!(!alive.status.success(), "cancelled startup left the node process alive");
	}

	#[tokio::test]
	async fn startup_timeout_reports_the_phase_and_recent_logs() {
		let (mut args, pid_path) = startup_stub(None);
		let data_path = args.base_data_path.clone();
		let error = ArgonTestNode::start_process(&mut args, Duration::from_secs(2))
			.await
			.err()
			.expect("RPC startup should time out");

		let pid = fs::read_to_string(&pid_path).unwrap();
		let alive = Command::new("/bin/kill").args(["-0", pid.trim()]).output().unwrap();
		if alive.status.success() {
			Command::new("/bin/kill").args(["-KILL", pid.trim()]).output().unwrap();
		}
		remove_dir_all(data_path).unwrap();
		assert!(!alive.status.success(), "timed-out startup left the node process alive");
		let message = format!("{error:#}");
		assert!(message.contains("RPC startup"), "{message}");
		assert!(message.contains("timed out"), "{message}");
		assert!(message.contains("startup stub running"), "{message}");
	}

	fn startup_stub(rpc_port: Option<u16>) -> (ArgonNodeStartArgs, PathBuf) {
		let mut args = ArgonNodeStartArgs::new("alice", 0, "").unwrap();
		args.target_dir = args.base_data_path.clone();
		args.bitcoin_rpc = "http://127.0.0.1:1".to_string();
		let executable = args.target_dir.join("argon-node");
		let mut script =
			"#!/bin/sh\nprintf '%s\\n' \"$$\" > \"$0.pid\"\nprintf 'startup stub running\\n' >&2\n"
				.to_string();
		if let Some(rpc_port) = rpc_port {
			script.push_str(&format!(
				"printf 'Running JSON-RPC server: addr=127.0.0.1:{rpc_port}\\n' >&2\n"
			));
		}
		script.push_str("exec /bin/sleep 60\n");
		fs::write(&executable, script).unwrap();
		fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
		let pid_path = args.target_dir.join("argon-node.pid");
		(args, pid_path)
	}
}
