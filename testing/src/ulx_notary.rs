use std::{
	env,
	io::{BufRead, BufReader},
	path::PathBuf,
	process,
	process::Command,
	sync::Arc,
};

use anyhow::Context;
use rand::seq::SliceRandom;
use sp_core::{crypto::Pair, ed25519};
use sqlx::postgres::PgPoolOptions;

use ulixee_client::{
	api::{
		runtime_types::ulx_primitives::{
			host::Host,
			notary::{NotaryMeta, NotaryName},
		},
		tx,
	},
	signer::Ed25519Signer,
};

use crate::UlxTestNode;

pub struct UlxTestNotary {
	// Keep a handle to the node; once it's dropped the node is killed.
	proc: Option<process::Child>,
	pub client: Arc<ulx_notary_apis::Client>,
	pub ws_url: String,
	pub operator: ed25519::Pair,
}

impl Drop for UlxTestNotary {
	fn drop(&mut self) {
		if let Some(mut proc) = self.proc.take() {
			let _ = proc.kill();
		}
	}
}

impl UlxTestNotary {
	pub async fn start(
		node: &UlxTestNode,
		operator: Option<ed25519::Pair>,
	) -> anyhow::Result<Self> {
		let rust_log = env::var("RUST_LOG").unwrap_or("warn".to_string());

		let target_dir = {
			let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
			let workspace_cargo_path = project_dir.join("..");
			let workspace_cargo_path =
				workspace_cargo_path.canonicalize().expect("Failed to canonicalize path");
			let workspace_cargo_path = workspace_cargo_path.as_path().join("target/debug");
			workspace_cargo_path
		};
		println!("run from {}", target_dir.to_str().unwrap_or(""));

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
		let output = Command::new("./ulx-notary")
			.current_dir(&target_dir)
			.args(vec!["migrate", "--db-url", &db_url])
			.output()?;
		println!("Migrated db {}: {:?}", db_name, output.stdout);

		let mut proc = Command::new("./ulx-notary")
			.current_dir(&target_dir)
			.env("RUST_LOG", rust_log)
			.stdout(process::Stdio::piped())
			.args(vec!["run", "--db-url", &db_url, "--dev", "-t", &node.client.url])
			.spawn()?;

		// Wait for RPC port to be logged (it's logged to stderr).
		let stdout = proc.stdout.take().unwrap();

		let mut ws_url = None;
		for line in BufReader::new(stdout).lines().take(500) {
			let line = line.expect("failed to obtain next line from stdout for port discovery");

			let line_port = line.rsplit_once("Listening on ws://").map(|(_, port)| port);

			if let Some(line_port) = line_port {
				let line_port = line_port.trim_end_matches(|b: char| !b.is_ascii_digit());
				ws_url = Some(format!("ws://{}", line_port));
				break;
			}
		}

		let ws_url = ws_url.expect("Failed to find ws port");
		let client = ulx_notary_apis::create_client(&ws_url).await?;
		Ok(Self { proc: Some(proc), client: Arc::new(client), ws_url, operator })
	}

	pub async fn register_operator(&self, ulx_test_node: &UlxTestNode) -> anyhow::Result<()> {
		let operator = self.operator;

		let host: ulx_primitives::host::Host = self.ws_url.clone().into();
		let notary_proposal = tx().notaries().propose(NotaryMeta {
			name: NotaryName("test".as_bytes().to_vec().into()),
			hosts: vec![Host(host.0.into())].into(),
			public: operator.public().0,
		});
		println!("notary proposal {:?}", notary_proposal.call_data());
		let signer = Ed25519Signer::new(operator);
		ulx_test_node
			.client
			.live
			.tx()
			.sign_and_submit_then_watch_default(&notary_proposal, &signer)
			.await?
			.wait_for_finalized_success()
			.await?;

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
