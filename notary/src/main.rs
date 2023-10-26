use std::{env, path::PathBuf};

use anyhow::Context;
use clap::{crate_version, Parser, Subcommand};
use sc_cli::{utils, with_crypto_scheme, CryptoScheme, KeystoreParams};
use sc_keystore::LocalKeystore;
use sc_service::config::KeystoreConfig;
use sp_core::{bytes::from_hex, crypto::SecretString, H256};
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
use sqlx::postgres::PgPoolOptions;

use notary::{
	notary::{Notary, NOTARY_KEYID},
	run_server,
};
use ulx_notary_primitives::NotaryId;

#[derive(Parser, Debug)]
#[clap(version = crate_version!())]
#[command(author, version, about, arg_required_else_help = true, long_about = None)]
struct Cli {
	#[clap(long)]
	dev: bool,

	#[command(subcommand)]
	command: Commands,

	#[allow(missing_docs)]
	#[clap(flatten)]
	keystore_params: KeystoreParams,

	#[clap(long, env = "ULX_NOTARY_BASE_PATH")]
	base_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
	/// Starts a notary server
	Run {
		/// Bind to a specific address on this machine
		#[clap(short, long, env, default_value = "127.0.0.1:0")]
		bind_addr: String,

		/// What mainchain RPC websocket url do you want to reach out use to sync blocks and submit
		/// notebooks?
		#[clap(short, long, env, default_value = "ws://127.0.0.1:9944")]
		trusted_rpc_url: String,

		/// Required notary id you are running
		#[clap(short, long, env = "ULX_NOTARY_ID", default_value = "1")]
		notary_id: NotaryId,

		#[clap(short, long, env = "DATABASE_URL")]
		db_url: String,

		/// Should this node sync blocks? Multiple nodes should not sync blocks.
		#[clap(short, long, env, default_value = "false")]
		sync_blocks: bool,

		/// Should this node finalize notebooks?
		#[clap(short, long, env, default_value = "false")]
		finalize_notebooks: bool,

		#[clap(short, long, env)]
		genesis_block: String,
	},
	/// Inserts a Notary compatible key into the keystore. NOTE: you still need to register it
	InsertKey {
		/// The secret key URI.
		/// If the value is a file, the file content is used as URI.
		/// If not given, you will be prompted for the URI.
		#[arg(long)]
		suri: Option<String>,
	},
}

/// Error type for the CLI.
#[derive(Debug, thiserror::Error)]
enum Error {
	#[error("Invalid input: {0}")]
	Input(String),

	#[error("Key store operation failed")]
	KeystoreOperation,

	#[error("Key storage issue encountered")]
	KeyStorage(#[from] sc_keystore::Error),
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

	let keystore: KeystorePtr = match &cli.base_path {
		Some(r) => {
			let base_path = r.clone();
			match cli.keystore_params.keystore_config(&base_path)? {
				KeystoreConfig::Path { path, password } =>
					Ok(LocalKeystore::open(path, password)?.into()),
				_ => unreachable!("keystore_config always returns path and password; qed"),
			}
		},
		None if cli.dev => Ok(MemoryKeystore::new().into()),
		None => Err("No base path provided"),
	}
	.map_err(|_| Error::KeystoreOperation)?;

	match cli.command {
		Commands::Run {
			db_url,
			bind_addr,
			trusted_rpc_url,
			notary_id,
			sync_blocks,
			finalize_notebooks,
			genesis_block,
		} => {
			MemoryKeystore::new();
			let pool = PgPoolOptions::new()
				.max_connections(100)
				.connect(&db_url)
				.await
				.context("failed to connect to db")?;

			let genesis: H256 = H256::from_slice(from_hex(genesis_block.as_str())?.as_slice());

			let notary = Notary::start(
				trusted_rpc_url.as_str(),
				notary_id,
				genesis,
				keystore.into(),
				pool,
				finalize_notebooks,
				sync_blocks,
			)
			.await?;
			let server_addr = run_server(&notary, bind_addr).await?;
			let url = format!("ws://{}", server_addr);

			println!("Listening on {}", url);
		},
		Commands::InsertKey { suri } => {
			let suri = utils::read_uri(suri.as_ref())?;
			let password = cli.keystore_params.read_password()?;
			let public =
				with_crypto_scheme!(CryptoScheme::Ed25519, to_vec(&suri, password.clone()))?;

			keystore
				.insert(NOTARY_KEYID, &suri, &public[..])
				.map_err(|_| Error::KeystoreOperation)?;
		},
	};

	Ok(())
}
fn to_vec<P: sp_core::Pair>(
	uri: &str,
	pass: Option<SecretString>,
) -> anyhow::Result<Vec<u8>, Error> {
	let p = utils::pair_from_suri::<P>(uri, pass).map_err(|e| Error::Input(e.to_string()))?;
	Ok(p.public().as_ref().to_vec())
}
