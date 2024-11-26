use std::{
	env,
	io::{BufRead, BufReader},
	process,
	process::Command,
	sync::{mpsc, Arc},
	thread,
};

use anyhow::Context;
use argon_client::{
	api::{
		runtime_types::argon_primitives::{
			host::Host,
			notary::{NotaryMeta, NotaryName},
		},
		tx,
	},
	signer::Ed25519Signer,
	MainchainClient,
};
use rand::seq::SliceRandom;
use sp_core::{crypto::Pair, ed25519};
use sqlx::{postgres::PgPoolOptions, PgPool};
use strip_ansi_escapes::strip;
use tokio::{runtime::Runtime, task::spawn_blocking};

use crate::{get_target_dir, ArgonTestNode};

pub struct ArgonTestNotary {
	// Keep a handle to the node; once it's dropped the node is killed.
	pub proc: Option<process::Child>,
	pub client: Arc<argon_notary_apis::Client>,
	pub ws_url: String,
	pub operator: ed25519::Pair,
	pub db_name: String,
	pub db_url: String,
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
	pub async fn start(
		node: &ArgonTestNode,
		operator: Option<ed25519::Pair>,
	) -> anyhow::Result<Self> {
		let rust_log = env::var("RUST_LOG").unwrap_or("warn".to_string());

		let target_dir = get_target_dir();

		let operator = operator
			.unwrap_or_else(|| ed25519::Pair::from_string("//Ferdie//notary", None).unwrap());
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

		let mut proc = Command::new("./argon-notary")
			.current_dir(&target_dir)
			.env("RUST_LOG", rust_log)
			.stdout(process::Stdio::piped())
			.args(vec!["run", "--db-url", &db_url, "--dev", "-t", &node.client.url])
			.spawn()?;

		// Wait for RPC port to be logged (it's logged to stdout).
		let stdout = proc.stdout.take().unwrap();
		let stdout_reader = BufReader::new(stdout);
		let (tx, rx) = mpsc::channel();

		let tx_clone = tx.clone();

		spawn_blocking(move || {
			for line in stdout_reader.lines() {
				let line = line.expect("failed to obtain next line from stdout");
				let cleaned_log = strip(&line);
				println!("NOTARY>> {}", String::from_utf8_lossy(&cleaned_log));

				let line_port = line.rsplit_once("Listening on ws://").map(|(_, port)| port);

				if let Some(line_port) = line_port {
					let line_port = line_port.trim_end_matches(|b: char| !b.is_ascii_digit());
					let ws_url = format!("ws://{}", line_port);
					tx_clone.send(ws_url).unwrap();
				}
			}
		});

		let ws_url = rx.recv().expect("Failed to start notary");
		let client = argon_notary_apis::create_client(&ws_url).await?;
		Ok(Self {
			proc: Some(proc),
			client: Arc::new(client),
			ws_url,
			operator,
			db_name,
			db_url: db_base_url,
		})
	}

	pub async fn register_operator(&self, argon_test_node: &ArgonTestNode) -> anyhow::Result<()> {
		let operator = self.operator;

		let host: argon_primitives::host::Host = self.ws_url.clone().into();
		let notary_proposal = tx().notaries().propose(NotaryMeta {
			name: NotaryName("test".as_bytes().to_vec().into()),
			hosts: vec![Host(host.0.into())].into(),
			public: operator.public().0,
		});
		println!("notary proposal {:?}", notary_proposal.call_data());
		let signer = Ed25519Signer::new(operator);
		let ext = argon_test_node
			.client
			.live
			.tx()
			.sign_and_submit_then_watch_default(&notary_proposal, &signer)
			.await?;

		MainchainClient::wait_for_ext_in_block(ext, false).await?;

		Ok(())
	}
}

fn generate_random_db_name() -> String {
	const DB_NAME_LENGTH: usize = 12;
	const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";
	let mut rng = rand::thread_rng();

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
