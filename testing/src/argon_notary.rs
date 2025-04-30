use crate::{get_target_dir, log_watcher::LogWatcher, ArgonTestNode};
use anyhow::Context;
use argon_client::{
	api::{
		runtime_types::argon_primitives::{
			host::Host,
			notary::{NotaryMeta, NotaryName},
		},
		tx,
	},
	signer::Sr25519Signer,
	MainchainClient,
};
use polkadot_sdk::*;
use rand::prelude::IndexedRandom;
use sp_core::{
	crypto::{Pair, Ss58Codec},
	ed25519, sr25519,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, path::PathBuf, process, process::Command, sync::Arc, thread};
use tokio::runtime::Runtime;
use url::Url;
use uuid::Uuid;

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
}

struct Args {
	port: u16,
	operator_address: String,
	db_url: String,
	mainchain_url: String,
	archive_bucket: String,
}

impl Drop for ArgonTestNotary {
	fn drop(&mut self) {
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
		}
		let db_url = self.db_url.clone();
		let db_name = self.db_name.clone();
		thread::spawn(move || {
			let handle = Runtime::new().unwrap();
			handle.block_on(async {
				let pool =
					PgPool::connect(&db_url).await.context("failed to connect to db").unwrap();
				sqlx::query(&format!("DROP DATABASE \"{}\" WITH(FORCE)", db_name))
					.execute(&pool)
					.await
					.map_err(|e| eprintln!("Failed to drop db: {:?}", e))
					.ok();
			});
		})
		.join()
		.expect("Failed to drop db");
	}
}

impl ArgonTestNotary {
	pub fn stop(&mut self) {
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
		}
	}

	pub fn get_minio_url() -> String {
		env::var("MINIO_URL").unwrap_or("http://localhost:9000".to_string())
	}

	pub async fn restart(&mut self) -> anyhow::Result<()> {
		self.stop();
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
		let proc = Command::new("./argon-notary")
			.current_dir(&target_dir)
			.env("RUST_LOG", rust_log)
			.stdout(process::Stdio::piped())
			.args(vec![
				"run",
				"--db-url",
				&args.db_url,
				"--dev",
				"--operator-address",
				&args.operator_address,
				"-t",
				&args.mainchain_url,
				"--bind-addr",
				&format!("127.0.0.1:{}", args.port),
				"--archive-bucket",
				&args.archive_bucket,
			])
			.spawn()?;
		Ok(proc)
	}

	pub fn create_archive_bucket() -> String {
		let archive_uuid = Uuid::new_v4();
		let bucket_name = format!("notary-archives-{}", archive_uuid);
		bucket_name
	}

	pub async fn start_with_archive(
		node: &ArgonTestNode,
		archive_bucket: String,
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

		let mut uid: String = "".to_string();
		for _ in 0..10 {
			uid = generate_random_db_name();
			let db_name = format!("notary_{}", uid);
			let result = sqlx::query("SELECT 1 FROM pg_database WHERE datname = $1")
				.bind(&db_name)
				.fetch_one(&pool)
				.await;
			if result.is_err() {
				break;
			}
		}

		let db_name = format!("notary_{}", uid);
		let db_url = format!("{}/{}", db_base_url, db_name);
		sqlx::query(&format!("CREATE DATABASE \"{}\"", db_name)).execute(&pool).await?;
		// migrate from notary project path
		let output = Command::new("./argon-notary")
			.current_dir(&target_dir)
			.args(vec!["migrate", "--db-url", &db_url])
			.output()?;
		println!("Migrated notary db {}: {:?}", db_name, output.stdout);

		let mut args = Args {
			port: 0,
			operator_address: operator.public().to_ss58check(),
			db_url: db_url.clone(),
			mainchain_url: node.client.url.clone(),
			archive_bucket,
		};
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
		})
	}

	pub async fn start(node: &ArgonTestNode) -> anyhow::Result<Self> {
		let archive_bucket = Self::create_archive_bucket();
		Self::start_with_archive(node, archive_bucket).await
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
