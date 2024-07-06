use std::env;

use anyhow::{anyhow, bail, ensure};
use clap::{crate_version, Parser, ValueEnum};
use dotenv::dotenv;
use sp_core::{
	crypto::{key_types::ACCOUNT, Ss58Codec},
	sr25519, Pair as PairT,
};
use sp_runtime::traits::IdentifyAccount;
use tracing::warn;
use ulixee_client::signer::KeystoreSigner;
use ulx_primitives::{AccountId, CryptoType, KeystoreParams, ADDRESS_PREFIX};
use url::Url;

use crate::{bitcoin_tip::bitcoin_loop, price_index::price_index_loop};

mod argon_price;
mod bitcoin_tip;
mod btc_price;
mod price_index;
mod us_cpi;
mod us_cpi_schedule;
pub(crate) mod utils;

#[derive(Parser, Debug)]
#[clap(version = crate_version!())]
#[command(author, version, about, arg_required_else_help = true, long_about = None)]
struct Cli {
	/// Start in dev mode (using default //Alice as operator)
	#[clap(long)]
	dev: bool,

	/// What mainchain RPC websocket url do you want to reach out use to sync blocks?
	#[clap(short, long, env, default_value = "ws://127.0.0.1:9944")]
	trusted_rpc_url: String,

	#[allow(missing_docs)]
	#[clap(flatten)]
	keystore_params: KeystoreParams,

	/// The signer to use from the keystore (Required if not in dev mode)
	#[clap(long, env)]
	signer_address: Option<String>,

	/// What type of crypto to use for the signer (Required if not in dev mode)
	#[clap(long, env)]
	signer_crypto: Option<OracleCryptoType>,

	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,
}
#[derive(Debug, Clone, clap::Subcommand, Default)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommand {
	#[default]
	PriceIndex,
	Bitcoin {
		/// The Bitcoin full node to follow for longest chain. Should be a hosted/trusted
		/// full node. Include optional auth inline
		#[clap(long, env)]
		bitcoin_rpc_url: String,
	},
}

#[derive(ValueEnum, Debug, Clone)]
enum OracleCryptoType {
	Sr25519,
	Ed25519,
}
impl From<OracleCryptoType> for CryptoType {
	fn from(crypto: OracleCryptoType) -> CryptoType {
		match crypto {
			OracleCryptoType::Sr25519 => CryptoType::Sr25519,
			OracleCryptoType::Ed25519 => CryptoType::Ed25519,
		}
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let _ = tracing_subscriber::FmtSubscriber::builder()
		.with_env_filter(tracing_subscriber::EnvFilter::from("debug"))
		.try_init();
	env::set_var("RUST_BACKTRACE", "1");
	dotenv().ok();

	let Cli { trusted_rpc_url, keystore_params, dev, signer_address, signer_crypto, subcommand } =
		Cli::parse();

	tracing::info!(
		"Running {:?} (url={}). {}",
		subcommand,
		trusted_rpc_url,
		if dev { "Dev Mode" } else { "" }
	);
	if subcommand.is_none() {
		warn!("No subcommand provided, defaulting to price index. NOTE: the bitcoin oracle must be activated somewhere else.");
	}

	let keystore = if dev {
		keystore_params.open_dev("//Alice", CryptoType::Sr25519, ACCOUNT)?
	} else {
		keystore_params.open()?
	};

	let (signer_account, signer_crypto) = match (signer_address, signer_crypto) {
		(Some(signer_address), Some(signer_crypto)) => {
			let (signer_account, format) = AccountId::from_ss58check_with_version(&signer_address)?;
			ensure!(format.prefix() == ADDRESS_PREFIX, "Invalid address format");
			Ok::<_, anyhow::Error>((signer_account, signer_crypto))
		},
		(None, None) => {
			if !dev {
				bail!("Signer address and crypto type must be provided")
			}

			let pair = sr25519::Pair::from_string("//Alice", None)?;
			let account_id = pair.public().into_account();
			Ok::<_, anyhow::Error>((account_id.into(), OracleCryptoType::Sr25519))
		},
		_ => bail!("Signer address and crypto type must be provided"),
	}?;

	let signer = KeystoreSigner::new(keystore, signer_account, signer_crypto.into());
	match subcommand {
		Some(Subcommand::PriceIndex) | None =>
			price_index_loop(trusted_rpc_url, signer, dev).await?,
		Some(Subcommand::Bitcoin { bitcoin_rpc_url }) => {
			let bitcoin_url = Url::parse(&bitcoin_rpc_url).map_err(|e| {
				anyhow!("Unable to parse bitcoin rpc url ({}) {:?}", bitcoin_rpc_url, e)
			})?;
			let (user, password) = (bitcoin_url.username(), bitcoin_url.password());

			let bitcoin_auth = if !user.is_empty() {
				Some((user.to_string(), password.unwrap_or_default().to_string()))
			} else {
				None
			};
			bitcoin_loop(bitcoin_rpc_url, bitcoin_auth, trusted_rpc_url, signer).await?
		},
	};
	Ok(())
}
