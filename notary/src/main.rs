use std::path::PathBuf;
use std::{env, fs};

use anyhow::{anyhow, Context};
use clap::{crate_version, Args, Parser, Subcommand};
use sc_keystore::LocalKeystore;
use sp_core::crypto::{ExposeSecret, Zeroize};
use sp_core::{crypto::SecretString, ed25519, ByteArray, Pair as PairT};
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
use sqlx::{migrate, postgres::PgPoolOptions};

use ulixee_client::MultiurlClient;
use ulx_notary::{
	block_watch::spawn_block_sync,
	notebook_closer::{spawn_notebook_closer, NOTARY_KEYID},
	NotaryServer,
};
use ulx_primitives::{tick::Ticker, NotaryId};

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
		/// Start in dev mode (in memory keystore with a default key (//Ferdie)
		#[clap(long)]
		dev: bool,

		/// Bind to a specific address on this machine
		#[clap(short, long, env, default_value = "127.0.0.1:0")]
		bind_addr: String,

		/// What mainchain RPC websocket url do you want to reach out use to sync blocks?
		#[clap(short, long, env, default_value = "ws://127.0.0.1:9944")]
		trusted_rpc_url: String,

		/// Required notary id you are running
		#[clap(short, long, env = "ULX_NOTARY_ID", default_value = "1")]
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
			let password = read_password(keystore_params.clone())?;
			let keystore = read_keystore(keystore_params.keystore_path, password, dev)?;
			if dev {
				let suri = "//Ferdie//notary";
				let pair = ed25519::Pair::from_string(&suri, None)?;
				keystore
					.insert(NOTARY_KEYID, &suri, pair.public().as_slice())
					.map_err(|_| Error::KeystoreOperation)?;
			}

			let mut mainchain_client = MultiurlClient::new(vec![trusted_rpc_url.clone()]);
			let ticker = mainchain_client.lookup_ticker().await?;
			let ticker = Ticker::new(ticker.tick_duration_millis, ticker.genesis_utc_time);

			let server = NotaryServer::start(notary_id, pool.clone(), bind_addr).await?;

			if sync_blocks {
				spawn_block_sync(trusted_rpc_url.clone(), notary_id, &pool, ticker.clone()).await?;
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
			let suri = read_uri(suri.as_ref())?;

			let password = read_password(keystore_params.clone())?;
			let keystore = read_keystore(keystore_params.keystore_path, password.clone(), false)?;

			let p: ed25519::Pair = if let Some(pass) = password {
				let mut pass_str = pass.expose_secret().clone();
				let pair = ed25519::Pair::from_string(&suri, Some(&pass_str));
				pass_str.zeroize();
				pair
			} else {
				ed25519::Pair::from_string(&suri, None)
			}
			.map_err(|e| Error::Input(e.to_string()))?;
			let public = p.public().0;
			keystore
				.insert(NOTARY_KEYID, &suri, &public[..])
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

/// Parameters of the keystore
#[derive(Debug, Clone, Args)]
pub struct KeystoreParams {
	/// Specify custom keystore path.
	#[arg(long, value_name = "PATH")]
	pub keystore_path: Option<PathBuf>,

	/// Use interactive shell for entering the password used by the keystore.
	#[arg(long, conflicts_with_all = &["password", "password_filename"])]
	pub password_interactive: bool,

	/// Password used by the keystore.
	///
	/// This allows appending an extra user-defined secret to the seed.
	#[arg(
	long,
	value_parser = secret_string_from_str,
	conflicts_with_all = &["password_interactive", "password_filename"]
	)]
	pub password: Option<SecretString>,

	/// File that contains the password used by the keystore.
	#[arg(
	long,
	value_name = "PATH",
	conflicts_with_all = &["password_interactive", "password"]
	)]
	pub password_filename: Option<PathBuf>,
}

fn read_password(keystore_params: KeystoreParams) -> anyhow::Result<Option<SecretString>> {
	let (password_interactive, password) =
		(keystore_params.password_interactive, keystore_params.password.clone());

	let pass = if password_interactive {
		Some(SecretString::new(rpassword::prompt_password("Keystore Password: ")?))
	} else if let Some(ref file) = keystore_params.password_filename {
		let password = fs::read_to_string(file).map_err(|e| anyhow!("Unable to read password file: {}", e))?;
		Some(SecretString::new(password))
	} else {
		password
	};

	Ok(pass)
}

fn read_keystore(
	keystore_path: Option<PathBuf>,
	password: Option<SecretString>,
	dev: bool,
) -> anyhow::Result<KeystorePtr> {
	let keystore: KeystorePtr = match &keystore_path {
		Some(r) => Ok(LocalKeystore::open(r, password)?.into()),
		None if dev => Ok(MemoryKeystore::new().into()),
		None => Err("No keystore path provided"),
	}
	.map_err(|_| Error::KeystoreOperation)?;
	Ok(keystore)
}

pub fn read_uri(uri: Option<&String>) -> anyhow::Result<String> {
	let uri = if let Some(uri) = uri {
		let file = PathBuf::from(&uri);
		if file.is_file() {
			std::fs::read_to_string(uri)?.trim_end().to_owned()
		} else {
			uri.into()
		}
	} else {
		rpassword::prompt_password("URI: ")?
	};

	Ok(uri)
}
/// Parse a secret string, returning a displayable error.
pub fn secret_string_from_str(s: &str) -> std::result::Result<SecretString, String> {
	std::str::FromStr::from_str(s).map_err(|_| "Could not get SecretString".to_string())
}
