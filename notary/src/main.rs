use std::env;

use anyhow::Context;
use clap::{crate_version, Parser, Subcommand};
use sqlx::{migrate, postgres::PgPoolOptions};

use argon_client::ReconnectingClient;
use argon_notary::{
	block_watch::spawn_block_sync,
	notebook_closer::{spawn_notebook_closer, NOTARY_KEYID},
	NotaryServer,
};
use argon_primitives::{tick::Ticker, CryptoType, KeystoreParams, NotaryId};

#[derive(Parser, Debug)]
#[clap(version = crate_version!())]
#[command(author, version, about, arg_required_else_help = true, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
	/// Starts a notary server
	Run {
		/// Start in dev mode (in memory keystore with a default key (//Ferdie//notary)
		#[clap(long)]
		dev: bool,

		/// Bind to a specific address on this machine
		#[clap(short, long, env, default_value = "127.0.0.1:0")]
		bind_addr: String,

		/// What mainchain RPC websocket url do you want to reach out use to sync blocks?
		#[clap(short, long, env, default_value = "ws://127.0.0.1:9944")]
		trusted_rpc_url: String,

		/// Required notary id you are running
		#[clap(short, long, env = "ARGON_NOTARY_ID", default_value = "1")]
		notary_id: NotaryId,

		#[allow(missing_docs)]
		#[clap(flatten)]
		keystore_params: KeystoreParams,

		#[clap(short, long, env = "DATABASE_URL")]
		db_url: String,

		/// Should this node sync blocks? Multiple nodes should NOT sync blocks.
		#[clap(short, long, env, default_value = "true")]
		sync_blocks: bool,

		/// Should this node finalize notebook?
		#[clap(short, long, env, default_value = "true")]
		finalize_notebooks: bool,
	},
	/// Inserts a Notary compatible key into the keystore. NOTE: you still need to register it
	InsertKey {
		#[allow(missing_docs)]
		#[clap(flatten)]
		keystore_params: KeystoreParams,
		/// The secret key URI.
		/// If the value is a file, the file content is used as URI.
		/// If not given, you will be prompted for the URI.
		#[arg(long)]
		suri: Option<String>,
	},
	/// Migrate a notary database
	Migrate {
		/// The database url
		#[clap(short, long, env = "DATABASE_URL")]
		db_url: String,
	},
}

/// Error type for the CLI.
#[derive(Debug, thiserror::Error)]
enum Error {
	#[error("Invalid input: {0}")]
	Input(String),

	#[error("Key store operation failed")]
	KeystoreOperation,
}

impl From<&str> for Error {
	fn from(s: &str) -> Error {
		Error::Input(s.to_string())
	}
}

impl From<String> for Error {
	fn from(s: String) -> Error {
		Error::Input(s)
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let cli = Cli::parse();
	tracing_subscriber::FmtSubscriber::builder()
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.try_init()
		.expect("setting default subscriber failed");
	env::set_var("RUST_BACKTRACE", "1");

	match cli.command {
		Commands::Run {
			db_url,
			bind_addr,
			trusted_rpc_url,
			notary_id,
			sync_blocks,
			finalize_notebooks,
			dev,
			keystore_params,
		} => {
			tracing::info!("Running notary. DB={}, Mainchain={}", db_url, trusted_rpc_url);
			let pool = PgPoolOptions::new()
				.max_connections(100)
				.connect(&db_url)
				.await
				.context("failed to connect to db")?;
			let keystore = if dev {
				keystore_params.open_dev("//Ferdie//notary", CryptoType::Ed25519, NOTARY_KEYID)?
			} else {
				keystore_params.open()?
			};

			let mut mainchain_client = ReconnectingClient::new(vec![trusted_rpc_url.clone()]);
			let ticker: Ticker = mainchain_client.get().await?.lookup_ticker().await?.into();

			let server =
				NotaryServer::start(notary_id, pool.clone(), ticker.clone(), bind_addr).await?;

			if sync_blocks {
				spawn_block_sync(trusted_rpc_url.clone(), notary_id, pool.clone(), ticker).await?;
			}
			if finalize_notebooks {
				let _ = spawn_notebook_closer(
					pool.clone(),
					notary_id,
					keystore,
					ticker,
					server.completed_notebook_sender.clone(),
				);
			}

			// print to stdout - ignore log filters
			println!("Listening on ws://{}", server.addr);
			let watching_server = server.clone();
			let _ = tokio::spawn(async move { watching_server.wait_for_close().await }).await;
			tracing::info!("Notary server closed");
		},
		Commands::InsertKey { suri, keystore_params } => {
			keystore_params
				.open_with_account(suri.as_ref(), CryptoType::Ed25519, NOTARY_KEYID)
				.map_err(|_| Error::KeystoreOperation)?;
		},
		Commands::Migrate { db_url } => {
			let pool = PgPoolOptions::new()
				.max_connections(1)
				.connect(&db_url)
				.await
				.context("failed to connect to db")?;

			migrate!().run(&pool).await?;
		},
	};

	Ok(())
}
