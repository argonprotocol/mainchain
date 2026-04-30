use crate::{ArgonTestNode, get_target_dir, log_watcher::LogWatcher};
use anyhow::{Context, bail};
use argon_client::{
	MainchainClient,
	api::{
		runtime_types::argon_primitives::{
			host::Host,
			notary::{NotaryMeta, NotaryName},
		},
		tx,
	},
	signer::Sr25519Signer,
};
use polkadot_sdk::*;
use rand::prelude::IndexedRandom;
use sp_core::{
	crypto::{Pair, Ss58Codec},
	ed25519, sr25519,
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
	env, net::TcpListener, path::PathBuf, process, process::Command, sync::Arc, thread,
	time::Duration,
};
use tokio::runtime::Runtime;
use url::Url;
use uuid::Uuid;

const MAX_DB_NAME_ATTEMPTS: usize = 32;

pub struct ArgonTestNotary {
	// Keep a handle to the node; once it's dropped the node is killed.
	pub proc: Option<process::Child>,
	pub client: Arc<argon_notary_apis::Client>,
	pub ws_url: String,
	pub operator: sr25519::Pair,
	pub db_name: String,
	pub db_url: String,
	log_watcher: LogWatcher,
	start_args: Args,
	cleanup_db: bool,
}

struct Args {
	port: u16,
	prometheus_port: u16,
	operator_address: String,
	db_url: String,
	mainchain_url: String,
	archive_bucket: String,
	archive_endpoint: Option<String>,
}

impl Drop for ArgonTestNotary {
	fn drop(&mut self) {
		self.log_watcher.close();
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
			let _ = proc.wait();
		}
		if !self.cleanup_db {
			return;
		}
		let db_url = self.db_url.clone();
		let db_name = self.db_name.clone();
		thread::spawn(move || {
			let handle = Runtime::new().unwrap();
			handle.block_on(async {
				if !is_valid_db_identifier(&db_name) {
					eprintln!("Skipping drop of invalid notary db name {db_name:?}");
					return;
				}
				let pool =
					PgPool::connect(&db_url).await.context("failed to connect to db").unwrap();
				sqlx::query(&format!("DROP DATABASE \"{db_name}\" WITH(FORCE)"))
					.execute(&pool)
					.await
					.map_err(|e| eprintln!("Failed to drop db: {e:?}"))
					.ok();
			});
		})
		.join()
		.expect("Failed to drop db");
	}
}

impl ArgonTestNotary {
	pub fn stop(&mut self) {
		self.log_watcher.close();
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
			let _ = proc.wait();
		}
	}

	pub fn get_minio_url() -> String {
		env::var("MINIO_URL").unwrap_or("http://localhost:9000".to_string())
	}

	pub async fn restart(&mut self) -> anyhow::Result<()> {
		self.stop();
		if self.start_args.port != 0 {
			Self::prepare_fixed_port(self.start_args.port).await?;
		}
		let mut proc = Self::start_process(get_target_dir(), &self.start_args).await?;
		let stdout = proc.stdout.take().unwrap();
		self.log_watcher = LogWatcher::new(stdout, "notary");
		self.proc = Some(proc);
		let _ = self.log_watcher.wait_for_log(r"Listening on (ws://[\d:.]+)", 1).await?;
		self.client = Arc::new(argon_notary_apis::create_client(&self.ws_url).await?);

		Ok(())
	}

	async fn start_process(target_dir: PathBuf, args: &Args) -> anyhow::Result<process::Child> {
		let rust_log = env::var("RUST_LOG").unwrap_or("info".to_string());
		let mut command_args = vec![
			"run".to_string(),
			"--db-url".to_string(),
			args.db_url.clone(),
			"--dev".to_string(),
			"--operator-address".to_string(),
			args.operator_address.clone(),
			"-t".to_string(),
			args.mainchain_url.clone(),
			"--bind-addr".to_string(),
			format!("127.0.0.1:{}", args.port),
			"--prometheus-port".to_string(),
			args.prometheus_port.to_string(),
			"--archive-bucket".to_string(),
			args.archive_bucket.clone(),
		];
		if let Some(archive_endpoint) = args.archive_endpoint.as_ref() {
			command_args.push("--archive-endpoint".to_string());
			command_args.push(archive_endpoint.clone());
		}

		let proc = Command::new("./argon-notary")
			.current_dir(&target_dir)
			.env("RUST_LOG", rust_log)
			.stdout(process::Stdio::piped())
			.args(command_args)
			.spawn()?;
		Ok(proc)
	}

	pub fn create_archive_bucket() -> String {
		let archive_uuid = Uuid::new_v4();
		let bucket_name = format!("notary-archives-{archive_uuid}");
		bucket_name
	}

	pub async fn start_with_archive(
		node: &ArgonTestNode,
		archive_bucket: String,
		fixed_port: Option<u16>,
		existing_db_name: Option<String>,
		cleanup_db: bool,
	) -> anyhow::Result<Self> {
		Self::start_with_archive_endpoint(
			node,
			archive_bucket,
			None,
			fixed_port,
			existing_db_name,
			cleanup_db,
		)
		.await
	}

	pub async fn start_with_archive_endpoint(
		node: &ArgonTestNode,
		archive_bucket: String,
		archive_endpoint: Option<String>,
		fixed_port: Option<u16>,
		existing_db_name: Option<String>,
		cleanup_db: bool,
	) -> anyhow::Result<Self> {
		let target_dir = get_target_dir();

		let operator = sr25519::Pair::from_string("//Ferdie", None).unwrap();
		let db_base_url = env::var("NOTARY_DB_URL")
			.unwrap_or("postgres://postgres:postgres@localhost:5432".to_string());

		let pool = PgPoolOptions::new()
			.max_connections(1)
			.connect(&db_base_url)
			.await
			.context("failed to connect to db")?;

		let db_name = match existing_db_name {
			Some(db_name) => {
				if !is_valid_db_identifier(&db_name) {
					bail!("invalid existing notary db name: {db_name}");
				}
				db_name
			},
			None => Self::find_available_db_name(&pool).await?,
		};
		let db_url = format!("{db_base_url}/{db_name}");
		let database_exists = sqlx::query("SELECT 1 FROM pg_database WHERE datname = $1")
			.bind(&db_name)
			.fetch_optional(&pool)
			.await?
			.is_some();
		let created_database = !database_exists;
		if created_database {
			sqlx::query(&format!("CREATE DATABASE \"{db_name}\"")).execute(&pool).await?;
		}
		// migrate from notary project path
		let output = Command::new("./argon-notary")
			.current_dir(&target_dir)
			.args(vec!["migrate", "--db-url", &db_url])
			.output()?;
		if !output.status.success() {
			if created_database {
				let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS \"{db_name}\""))
					.execute(&pool)
					.await;
			}
			bail!(
				"failed to migrate notary db {db_name}: status={} stdout={} stderr={}",
				output.status,
				String::from_utf8_lossy(&output.stdout),
				String::from_utf8_lossy(&output.stderr),
			);
		}
		println!("Migrated notary db {}: {:?}", db_name, output.stdout);

		let mut args = Args {
			port: fixed_port.unwrap_or_default(),
			prometheus_port: 0,
			operator_address: operator.public().to_ss58check(),
			db_url: db_url.clone(),
			mainchain_url: node.client.url.clone(),
			archive_bucket,
			archive_endpoint,
		};
		if args.port != 0 {
			Self::prepare_fixed_port(args.port).await?;
		}
		let mut proc =
			Self::start_process(target_dir, &args).await.context("failed to start notary")?;

		// Wait for RPC port to be logged (it's logged to stdout).
		let stdout = proc.stdout.take().unwrap();
		let log_watcher = LogWatcher::new(stdout, "notary");
		let ws_url = log_watcher
			.wait_for_log(r"Listening on (ws://[\d:.]+)", 1)
			.await?
			.first()
			.expect("No ws url found")
			.clone();

		args.port = Url::parse(&ws_url)?.port().unwrap();

		let client = argon_notary_apis::create_client(&ws_url).await?;
		Ok(Self {
			proc: Some(proc),
			client: Arc::new(client),
			ws_url,
			log_watcher,
			operator,
			db_name,
			db_url: db_base_url,
			start_args: args,
			cleanup_db,
		})
	}

	async fn prepare_fixed_port(port: u16) -> anyhow::Result<()> {
		if Self::port_available(port) {
			return Ok(());
		}

		eprintln!("Stopping stale notary process on port {port}");
		let _ = Command::new("pkill")
			.args(["-INT", "-f", &format!("argon-notary.*127.0.0.1:{port}")])
			.status();
		tokio::time::sleep(Duration::from_secs(1)).await;
		Self::wait_for_port(port).await
	}

	async fn wait_for_port(port: u16) -> anyhow::Result<()> {
		let deadline = tokio::time::Instant::now() + Duration::from_secs(15);

		loop {
			if Self::port_available(port) {
				return Ok(());
			}
			match TcpListener::bind(("127.0.0.1", port)) {
				Ok(listener) => {
					drop(listener);
					return Ok(());
				},
				Err(error) if tokio::time::Instant::now() < deadline => {
					eprintln!("Waiting for notary port {port} to become available: {error}");
					tokio::time::sleep(Duration::from_millis(250)).await;
				},
				Err(error) => {
					return Err(anyhow::anyhow!(
						"notary port {port} stayed unavailable before restart: {error}"
					));
				},
			}
		}
	}

	fn port_available(port: u16) -> bool {
		TcpListener::bind(("127.0.0.1", port)).map(drop).is_ok()
	}

	pub async fn clone_database(template_db_name: &str) -> anyhow::Result<String> {
		if !is_valid_db_identifier(template_db_name) {
			bail!("invalid template notary db name: {template_db_name}");
		}
		let db_base_url = env::var("NOTARY_DB_URL")
			.unwrap_or("postgres://postgres:postgres@localhost:5432".to_string());
		let pool = PgPoolOptions::new()
			.max_connections(1)
			.connect(&db_base_url)
			.await
			.context("failed to connect to db")?;

		let db_name = Self::find_available_db_name(&pool).await?;

		sqlx::query(&format!("CREATE DATABASE \"{db_name}\" TEMPLATE \"{template_db_name}\""))
			.execute(&pool)
			.await?;

		Ok(db_name)
	}

	pub async fn start(node: &ArgonTestNode) -> anyhow::Result<Self> {
		let archive_bucket = Self::create_archive_bucket();
		Self::start_with_archive(node, archive_bucket, None, None, true).await
	}

	pub async fn register_operator(&self, argon_test_node: &ArgonTestNode) -> anyhow::Result<()> {
		let signer = ed25519::Pair::from_string("//Ferdie//notary", None)?;

		let host: argon_primitives::host::Host = self.ws_url.clone().into();
		let notary_proposal = tx().notaries().propose(NotaryMeta {
			name: NotaryName("test".as_bytes().to_vec().into()),
			hosts: vec![Host(host.0.into())].into(),
			public: signer.public().0,
		});
		println!("notary proposal {:?}", notary_proposal.call_data());
		let operator_account = Sr25519Signer::new(self.operator.clone());
		let ext = argon_test_node
			.client
			.live
			.tx()
			.sign_and_submit_then_watch_default(&notary_proposal, &operator_account)
			.await?;

		MainchainClient::wait_for_ext_in_block(ext, false).await?;

		Ok(())
	}

	async fn find_available_db_name(pool: &PgPool) -> anyhow::Result<String> {
		for _ in 0..MAX_DB_NAME_ATTEMPTS {
			let candidate = format!("notary_{}", generate_random_db_name());
			let exists = sqlx::query("SELECT 1 FROM pg_database WHERE datname = $1")
				.bind(&candidate)
				.fetch_optional(pool)
				.await?
				.is_some();
			if !exists {
				return Ok(candidate);
			}
		}

		bail!("failed to find a free notary db name after {MAX_DB_NAME_ATTEMPTS} attempts")
	}
}

fn generate_random_db_name() -> String {
	const DB_NAME_LENGTH: usize = 12;
	const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";
	let mut rng = rand::rng();

	// Ensure the first character is a letter or underscore
	let first_char = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_"
		.choose(&mut rng)
		.unwrap();

	let mut db_name = String::new();
	db_name.push(*first_char as char);

	for _ in 1..DB_NAME_LENGTH {
		let ch = CHARSET.choose(&mut rng).unwrap();
		db_name.push(*ch as char);
	}

	db_name
}

fn is_valid_db_identifier(name: &str) -> bool {
	let mut chars = name.chars();
	match chars.next() {
		Some(first) if first == '_' || first.is_ascii_alphabetic() => {},
		_ => return false,
	}

	chars.all(|char| char == '_' || char.is_ascii_alphanumeric())
}
