use std::env;

use anyhow::Context;
use argon_client::{api::storage, ReconnectingClient};
use argon_notary::{
	block_watch::spawn_block_sync,
	notebook_closer::{spawn_notebook_closer, NOTARY_KEYID},
	NotaryServer,
};
use argon_primitives::{tick::Ticker, AccountId, CryptoType, KeystoreParams, NotaryId};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use sp_core::{crypto::Ss58Codec, sr25519, Pair};
use sqlx::{migrate, postgres::PgPoolOptions};

#[derive(Parser, Debug)]
#[command(version = env!("IMPL_VERSION"), about, author, arg_required_else_help = true, long_about = None)]
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

		/// The notary operator account id. Required if notary is not registered yet
		#[clap(short, long, env = "ARGON_OPERATOR_ACCOUNT_ID")]
		operator_address: Option<String>,

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
	/// Inserts a Notary compatible key into the keystore. NOTE: you still need to register it in
	/// mainchain
	InsertKey {
		#[allow(missing_docs)]
		#[clap(flatten)]
		keystore_params: KeystoreParams,
		/// The secret key URI.
		/// If the value is a file, the file content is used as URI.
		/// If not given, you will be prompted for the URI.
		#[arg(long, verbatim_doc_comment)]
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
	color_backtrace::install();
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
			mut operator_address,
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
				if operator_address.is_none() {
					operator_address =
						Some(sr25519::Pair::from_string("//Ferdie", None)?.public().to_ss58check());
				}
				keystore_params.open_in_memory(
					"//Ferdie//notary",
					CryptoType::Ed25519,
					NOTARY_KEYID,
				)?
			} else {
				keystore_params.open()?
			};

			let mut mainchain_client = ReconnectingClient::new(vec![trusted_rpc_url.clone()]);
			let ticker: Ticker = mainchain_client.get().await?.lookup_ticker().await?;

			let operator_account_id = if let Some(address) = operator_address {
				AccountId::from_ss58check_with_version(address.as_str())
					.map_err(|_| Error::Input("Invalid account id".to_string()))
					.map(|(a, _version)| a)?
			} else {
				let active_notaries = mainchain_client
					.get()
					.await?
					.fetch_storage(&storage().notaries().active_notaries(), None)
					.await?
					.expect("active notaries storage is not available")
					.0;
				active_notaries
					.iter()
					.find_map(|notary| {
						if notary.notary_id == notary_id {
							Some(notary.operator_account_id.0.into())
						} else {
							None
						}
					})
					.ok_or_else(|| {
						Error::Input("Notary not found in active notaries".to_string())
					})?
			};

			let server = NotaryServer::start(
				notary_id,
				operator_account_id.clone(),
				pool.clone(),
				ticker,
				bind_addr,
			)
			.await?;

			if sync_blocks {
				spawn_block_sync(trusted_rpc_url.clone(), notary_id, pool.clone(), ticker).await?;
			}
			if finalize_notebooks {
				let handles = spawn_notebook_closer(
					pool.clone(),
					notary_id,
					operator_account_id,
					keystore,
					ticker,
					server.completed_notebook_sender.clone(),
				)?;

				let mut subscription = server.audit_failure_stream.subscribe(10);
				tokio::spawn(async move {
					while (subscription.next().await).is_some() {
						handles.0.abort();
						handles.1.abort();
					}
				});
			}

			// print to stdout - ignore log filters
			println!("Listening on ws://{}", server.addr);
			let watching_server = server.clone();
			let _ = tokio::spawn(async move { watching_server.wait_for_close().await }).await;
			tracing::info!("Notary server closed");
		},
		Commands::InsertKey { suri, keystore_params } => {
			keystore_params
				.open_with_account(suri.as_ref(), CryptoType::Ed25519, NOTARY_KEYID, false)
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
