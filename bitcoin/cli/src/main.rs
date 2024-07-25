mod create_vault;

#[allow(dead_code, unused_imports)]
use std::{env, fmt};

use anyhow::Context;
use clap::{crate_version, Parser, Subcommand};
use codec::Decode;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, ContentArrangement, Table};
use sp_runtime::{FixedPointNumber, FixedU128};
use ulixee_client::{
	api::{apis, bonds::events::bond_created::VaultId, storage, tx},
	conversion::from_api_fixed_u128,
	MainchainClient,
};
use ulx_primitives::bitcoin::SATOSHIS_PER_BITCOIN;

#[derive(Parser, Debug)]
#[clap(version = crate_version!())]
#[command(author, version, about, arg_required_else_help = true, long_about = None)]
struct Cli {
	/// The ulixee rpc url to connect to
	#[clap(short, long, env, default_value = "ws://127.0.0.1:9944")]
	trusted_rpc_url: String,

	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
	Vaults {
		#[clap(subcommand)]
		subcommand: VaultCommands,
	},
}

#[derive(Subcommand, Debug)]
enum VaultCommands {
	/// Show vaults that can support the given amount of btc
	List {
		/// The amount of btc to bond
		#[clap(short, long, default_value = "1.0")]
		btc: f32,
	},
	/// Create a new vault
	Create {
		#[clap(flatten)]
		config: create_vault::VaultConfig,
	},
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let cli = Cli::parse();
	tracing_subscriber::FmtSubscriber::builder()
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.try_init()
		.expect("setting default subscriber failed");
	env::set_var("RUST_BACKTRACE", "1");

	let rpc_url = cli.trusted_rpc_url.clone();

	match cli.command {
		Commands::Vaults { subcommand } => match subcommand {
			VaultCommands::List { btc } => {
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to ulixee node")?;

				let query = storage().vaults().vaults_by_id_iter();

				let best_block = client.best_block_hash().await?;

				let satoshis =
					FixedU128::from_float(btc as f64).saturating_mul_int(SATOSHIS_PER_BITCOIN);
				let Some(argons_needed) = client
					.live
					.runtime_api()
					.at(best_block)
					.call(apis().bitcoin_apis().market_rate(satoshis))
					.await?
				else {
					println!("No price conversion found in blockchain for bitcoin to argon");
					return Ok(());
				};

				println!("Showing for: {:#?} btc", btc);
				println!("Current mint value: {} argons", ArgonFormatter(argons_needed));

				let mut vaults = client.live.storage().at_latest().await?.iter(query).await?;

				let mut table = Table::new();
				table
					.load_preset(UTF8_FULL)
					.apply_modifier(UTF8_ROUND_CORNERS)
					.set_content_arrangement(ContentArrangement::Dynamic)
					.set_header(vec![
						"Id",
						"Available argons",
						"Bonded argons",
						"Securitization",
						"Fee",
					]);

				while let Some(Ok(kv)) = vaults.next().await {
					let vault = kv.value;
					let vault_id: VaultId = Decode::decode(&mut &kv.key_bytes[..])?;

					let bitcoin_argons_available =
						vault.bitcoin_argons.allocated - vault.bitcoin_argons.bonded;
					if bitcoin_argons_available >= argons_needed {
						let fee = vault.bitcoin_argons.base_fee +
							from_api_fixed_u128(vault.bitcoin_argons.annual_percent_rate)
								.saturating_mul_int(argons_needed);

						table.add_row(vec![
							vault_id.to_string(),
							ArgonFormatter(bitcoin_argons_available).to_string(),
							ArgonFormatter(vault.bitcoin_argons.bonded).to_string(),
							ArgonFormatter(vault.securitized_argons).to_string(),
							ArgonFormatter(fee).to_string(),
						]);
					}
				}

				if table.is_empty() {
					println!("No vaults found that can support {} btc", btc);
				} else {
					println!("{table}");
				}
			},
			VaultCommands::Create { config } => {
				let mut config = config;
				if !config.complete_prompt().await {
					return Ok(());
				}
				let client = MainchainClient::from_url(&rpc_url)
					.await
					.context("Failed to connect to ulixee node")?;
				let call = tx().vaults().create(config.as_call_data());
				let ext_params = MainchainClient::ext_params_builder().build();
				let ext_data = client.live.tx().create_partial_signed_offline(&call, ext_params)?;

				println!("Vault creation call\n\n0x{}", hex::encode(ext_data.call_data()));
			},
		},
	};

	Ok(())
}

struct ArgonFormatter(u128);

impl fmt::Display for ArgonFormatter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.0;
		let whole_part = value / 1_000; // Extract the whole part
		let decimal_part = (value % 1_000) / 10; // Extract the decimal part, considering only 2 decimal places
		write!(f, "₳ {}.{:02}", whole_part, decimal_part)
	}
}

struct BTCFormatter(u64, u8);

impl fmt::Display for BTCFormatter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let value = self.0;
		let whole_part = value / SATOSHIS_PER_BITCOIN; // Extract the whole part
		let decimal_part = value % SATOSHIS_PER_BITCOIN; // Extract the decimal part

		// Scale the decimal part according to the requested number of decimals
		let scaled_decimal_part = decimal_part / 10u64.pow(8 - self.1 as u32);

		write!(f, "₿ {}.{:0width$}", whole_part, scaled_decimal_part, width = self.1 as usize)
	}
}
