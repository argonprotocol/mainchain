use anyhow::{anyhow, bail, ensure};
use argon_client::signer::KeystoreSigner;
use argon_primitives::{ADDRESS_PREFIX, AccountId, CryptoType, KeystoreParams};
use clap::{Parser, ValueEnum};
use dotenv::dotenv;
use polkadot_sdk::*;
use sp_core::{
	Pair as PairT,
	crypto::{Ss58Codec, key_types::ACCOUNT},
	sr25519,
};
use sp_runtime::traits::IdentifyAccount;
use std::env;
use tracing::{error, info};
use url::Url;

use crate::{bitcoin_tip::bitcoin_loop, price_index::price_index_loop};

mod argon_price;
mod argonot_price;
mod bitcoin_tip;
mod coin_usd_prices;
mod price_index;
mod uniswap_oracle;
mod us_cpi;
mod us_cpi_schedule;
pub(crate) mod utils;

#[derive(Parser, Debug)]
#[command(author, version = env!("IMPL_VERSION"), about, arg_required_else_help = true, long_about = None)]
#[clap(arg_required_else_help = true)]
struct Cli {
	#[command(subcommand)]
	pub subcommand: Subcommand,

	/// Start in dev mode (using default //Dave or //Eve as operator)
	#[clap(global = true, long)]
	dev: bool,

	/// What mainchain RPC websocket url do you want to reach out use to sync blocks?
	#[clap(global = true, short, long, env, default_value = "ws://127.0.0.1:9944")]
	trusted_rpc_url: String,

	#[allow(missing_docs)]
	#[clap(flatten)]
	keystore_params: KeystoreParams,

	/// The signer to use from the keystore (Required if not in dev mode)
	#[clap(global = true, long, env)]
	signer_address: Option<String>,

	/// What type of crypto to use for the signer (Required if not in dev mode)
	#[clap(global = true, long, env)]
	signer_crypto: Option<OracleCryptoType>,
}
#[derive(Debug, Clone, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommand {
	PriceIndex {
		/// Use a price simulator instead of uniswap
		#[cfg(feature = "simulated-prices")]
		#[clap(short, long)]
		simulate_prices: bool,
	},
	Bitcoin {
		/// The Bitcoin full node to follow for longest chain. Should be a hosted/trusted
		/// full node. Include optional auth inline
		#[clap(long, env)]
		bitcoin_rpc_url: String,
	},
	/// Inserts an Oracle compatible key into the keystore.
	InsertKey {
		/// The secret key URI.
		/// If the value is a file, the file content is used as URI.
		/// If not given, you will be prompted for the URI.
		#[clap(long, verbatim_doc_comment)]
		suri: Option<String>,

		/// The crypto type
		#[clap(short, long, env, default_value_t=CryptoType::Sr25519)]
		crypto_type: CryptoType,

		/// Expected address
		#[clap(long)]
		verify_address: Option<String>,
	},
}

impl Default for Subcommand {
	fn default() -> Self {
		Subcommand::PriceIndex {
			#[cfg(feature = "simulated-prices")]
			simulate_prices: false,
		}
	}
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
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or(tracing_subscriber::EnvFilter::from("info")),
		)
		.try_init();
	color_backtrace::install();
	dotenv().ok();
	dotenv::from_filename("oracle/.env").ok();

	let Cli { trusted_rpc_url, keystore_params, dev, signer_address, signer_crypto, subcommand } =
		Cli::parse();

	if let Subcommand::InsertKey { suri, crypto_type, verify_address } = &subcommand {
		let (_keystore, address) = keystore_params.open_with_account(
			suri.as_ref(),
			crypto_type.clone(),
			ACCOUNT,
			false,
		)?;
		if let Some(verify_address) = verify_address {
			if *verify_address != address {
				error!(
					?address,
					?verify_address,
					"The provided key does not match the `verify_address` param"
				);
				bail!("The provided key does not match the `verify_address` param");
			}
		}
		info!(?address,
			keystore_path = ?keystore_params.keystore_path,
			"Inserted key to keystore");
		return Ok(());
	}

	// load dotenv from the oracle directory if running in dev mode
	if dev {
		let binary_path = env::current_exe()?;
		if binary_path.ends_with("debug/argon-oracle") {
			let from_workspace_root = binary_path.join("../../oracle/.env");
			dotenv::from_filename(from_workspace_root).ok();
		}
	}

	tracing::info!(
		"Running {:?} (url={}). {}",
		subcommand,
		trusted_rpc_url,
		if dev { "Dev Mode" } else { "" }
	);

	let mut signer_address = signer_address;
	let mut signer_crypto = signer_crypto;

	let keystore = if dev && signer_address.is_none() {
		let suri = match subcommand {
			Subcommand::PriceIndex { .. } => "//Eve",
			Subcommand::Bitcoin { .. } => "//Dave",
			_ => bail!("Signer address and crypto type must be provided"),
		};
		let pair = sr25519::Pair::from_string(suri, None)?;
		let account_id = pair.public().into_account();

		signer_address = Some(account_id.to_ss58check_with_version(ADDRESS_PREFIX.into()));
		signer_crypto = Some(OracleCryptoType::Sr25519);
		keystore_params.open_in_memory(suri, CryptoType::Sr25519, ACCOUNT)?
	} else {
		keystore_params.open()?
	};

	let (signer_account, signer_crypto) = match (signer_address, signer_crypto) {
		(Some(signer_address), Some(signer_crypto)) => {
			let (signer_account, format) = AccountId::from_ss58check_with_version(&signer_address)?;
			ensure!(format.prefix() == ADDRESS_PREFIX, "Invalid address format");
			Ok::<_, anyhow::Error>((signer_account, signer_crypto))
		},
		_ => bail!("Signer address and crypto type must be provided"),
	}?;

	let signer = KeystoreSigner::new(keystore, signer_account, signer_crypto.into());

	match subcommand {
		Subcommand::PriceIndex {
			#[cfg(feature = "simulated-prices")]
			simulate_prices,
		} => {
			#[cfg(feature = "simulated-prices")]
			{
				price_index_loop(trusted_rpc_url, signer, simulate_prices).await?
			}
			#[cfg(not(feature = "simulated-prices"))]
			{
				price_index_loop(trusted_rpc_url, signer, false).await?
			}
		},
		Subcommand::Bitcoin { bitcoin_rpc_url } => {
			let mut bitcoin_url = Url::parse(&bitcoin_rpc_url).map_err(|e| {
				anyhow!("Unable to parse bitcoin rpc url ({}) {:?}", bitcoin_rpc_url, e)
			})?;
			let (user, password) = (bitcoin_url.username(), bitcoin_url.password());
			let bitcoin_auth = if !user.is_empty() {
				Some((user.to_string(), password.unwrap_or_default().to_string()))
			} else {
				None
			};
			bitcoin_url.set_username("").ok();
			bitcoin_url.set_password(None).ok();

			bitcoin_loop(bitcoin_url.to_string(), bitcoin_auth, trusted_rpc_url, signer).await?
		},
		_ => bail!("Handled above, qed, not possible"),
	};
	Ok(())
}
