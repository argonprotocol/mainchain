use anyhow::Context;
use argon_client::{
	api::{notaries::storage::types, storage},
	MainchainClient,
};
use argon_notary::{
	block_watch::spawn_block_sync,
	notebook_closer::{spawn_notebook_closer, NOTARY_KEYID},
	s3_archive::S3Archive,
	server::{ArchiveSettings, RpcConfig},
	NotaryServer,
};
use argon_primitives::{tick::Ticker, AccountId, CryptoType, KeystoreParams, NotaryId};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use prometheus::Registry;
use sp_core::{crypto::Ss58Codec, sr25519, ByteArray, Pair};
use sqlx::{migrate, postgres::PgPoolOptions};
use std::{env, time::Duration};
use tracing::warn;

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
		/// Start in dev mode (in memory keystore with a default key and with random minio bucket)
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

		/// An s3 compatible endpoint for the archive (use minIO for self-hosted). Optional if
		/// using a default region or in dev.
		#[clap(long, env = "AWS_S3_ENDPOINT")]
		archive_endpoint: Option<String>,

		/// The s3 compatible bucket-name for writing to the archive. Optional if dev.
		#[clap(long)]
		archive_bucket: Option<String>,

		/// An s3 compatible region for the archive. Optional if dev. NOTE: credentials must be
		/// available in env using default AWS/s3 host or env vars.
		#[clap(long, env = "AWS_S3_REGION")]
		archive_region: Option<String>,

		/// The public read host for the archive (should include a bucket-name if in url). Optional
		/// only if dev.
		#[clap(long)]
		archive_public_host: Option<String>,
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
			archive_bucket,
			mut archive_public_host,
			archive_endpoint,
			archive_region,
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

			let s3_buckets = if dev && archive_public_host.is_none() {
				let (buckets, host) =
					S3Archive::rand_minio_test_bucket(notary_id, archive_bucket).await?;
				archive_public_host.replace(host.archive_host);
				buckets
			} else {
				let archive_region = archive_region.ok_or(Error::Input(
					"Archive region is required when not dev mode".to_string(),
				));
				let archive_bucket = archive_bucket.ok_or(Error::Input(
					"Archive bucket is required when not dev mode".to_string(),
				))?;
				let region = S3Archive::get_region(archive_region.unwrap(), archive_endpoint)?;
				S3Archive::new(notary_id, region, archive_bucket).await?
			};

			let archive_host = archive_public_host
				.ok_or(Error::Input("Archive public host is required".to_string()))?;

			let (operator_account_id, ticker) =
				check_notary(notary_id, trusted_rpc_url.clone(), operator_address.clone()).await?;
			let prom_registry = Registry::new();
			let server = NotaryServer::start(
				notary_id,
				operator_account_id.clone(),
				pool.clone(),
				ArchiveSettings { archive_host },
				// uses prometheus 9116
				RpcConfig::default(),
				ticker,
				bind_addr,
				prom_registry.clone(),
			)
			.await?;

			if sync_blocks {
				let handle = spawn_block_sync(
					trusted_rpc_url.clone(),
					notary_id,
					pool.clone(),
					ticker,
					Duration::from_secs(5),
				)
				.await?;
				tokio::spawn(async move {
					let _ = handle.await.inspect_err(|e| {
						warn!("Block watch exiting {}", e);
					});
				});
			}
			if finalize_notebooks {
				let handles = spawn_notebook_closer(
					pool.clone(),
					notary_id,
					operator_account_id,
					keystore,
					ticker,
					server.completed_notebook_sender.clone(),
					s3_buckets.clone(),
					prom_registry.clone(),
				)?;

				let mut subscription = server.audit_failure_stream.subscribe(10);
				let mut server_handle = server.clone();
				tokio::spawn(async move {
					if let Some(failure) = subscription.next().await {
						warn!("Audit failure detected in {}. Shutting down...", failure);
						// stop notebook close processes immediately
						handles.0.abort();
						handles.1.abort();
						// wait one second to clean up
						tokio::time::sleep(Duration::from_secs(1)).await;
						server_handle.stop().await;
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

/// Check the notary operator account id and ticker
async fn check_notary(
	notary_id: NotaryId,
	mainchain_url: String,
	operator_address: Option<String>,
) -> anyhow::Result<(AccountId, Ticker)> {
	let client = MainchainClient::try_until_connected(&mainchain_url, 2500, 10000).await?;
	let ticker: Ticker = client.lookup_ticker().await?;
	let active_notaries = client
		.fetch_storage(&storage().notaries().active_notaries(), None)
		.await?
		.unwrap_or(types::active_notaries::ActiveNotaries::from(Vec::new()));

	let operator_account_id = if let Some(address) = operator_address {
		Some(
			AccountId::from_ss58check_with_version(address.as_str())
				.map_err(|_| Error::Input("Invalid account id".to_string()))
				.map(|(a, _version)| a)?,
		)
	} else {
		None
	};

	if let Some(notary) = active_notaries.0.iter().find(|n| n.notary_id == notary_id) {
		if let Some(operator_account_id) = operator_account_id {
			if notary.operator_account_id.0 != operator_account_id.as_slice() {
				return Err(
					Error::Input("Notary operator account id does not match".to_string()).into()
				);
			}
		}
		return Ok((notary.operator_account_id.0.into(), ticker));
	}
	if let Some(operator_account_id) = operator_account_id {
		return Ok((operator_account_id, ticker));
	}
	Err(Error::Input("Notary not registered".to_string()).into())
}
