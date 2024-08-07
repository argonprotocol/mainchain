use crate::{
	bond_commands::BondCommands,
	formatters::{parse_number, Pct64},
	helpers::{read_bitcoin_xpub, read_percent_to_fixed_128},
	vault_commands::VaultCommands,
};
use anyhow::anyhow;
use clap::{crate_version, Parser, Subcommand};
use sp_runtime::FixedU128;
use std::{env, str::FromStr};

mod bond_commands;
mod formatters;
mod helpers;
mod vault_commands;
mod vault_create;

#[derive(Parser, Debug)]
#[clap(version = crate_version!())]
#[command(author, version, about, arg_required_else_help = true, long_about = None)]
struct Cli {
	/// The argon rpc url to connect to
	#[clap(short, long, env, global = true, default_value = "ws://127.0.0.1:9944")]
	trusted_rpc_url: String,

	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
	/// List, create and manage vaults
	Vault {
		#[clap(subcommand)]
		subcommand: VaultCommands,
	},
	/// Create, unlock and monitor bonds
	Bond {
		#[clap(subcommand)]
		subcommand: BondCommands,
	},
	/// Utilities for working with Bitcoin and Argon primitives
	Utils {
		#[clap(subcommand)]
		subcommand: UtilCommands,
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
		Commands::Vault { subcommand } => subcommand.process(rpc_url).await?,
		Commands::Bond { subcommand } => subcommand.process(rpc_url).await?,
		Commands::Utils { subcommand } => subcommand.process().await?,
	};

	Ok(())
}

#[derive(Subcommand, Debug)]
enum UtilCommands {
	/// Translate a percent into a FixedU128 for the polkadot.js apps
	ToFixed(OneArg),
	/// Convert a FixedU128 to a readable percent
	FromFixed(OneArg),
	/// XPub to bytes
	#[clap(name = "encode-xpub")]
	EncodeXPub(OneArg),
}

#[derive(Parser, Debug)]
struct OneArg {
	arg: String,
}
impl UtilCommands {
	pub async fn process(self) -> anyhow::Result<()> {
		match self {
			UtilCommands::ToFixed(OneArg { arg }) => {
				let fixed = parse_number(&arg).map_err(|e| anyhow!(e))?;
				let fixed = read_percent_to_fixed_128(fixed);
				println!("{}", fixed.into_inner());
			},
			UtilCommands::FromFixed(fixed) => {
				let fixed = FixedU128::from_str(&fixed.arg).map_err(|e| anyhow!(e))?;
				let percent = fixed.mul(FixedU128::from_u32(100)).to_float();

				println!("{}", Pct64(percent));
			},
			UtilCommands::EncodeXPub(xpub) => {
				let xpub = read_bitcoin_xpub(&xpub.arg).map_err(|e| anyhow!(e))?;
				println!("0x{}", hex::encode(xpub.0));
			},
		}
		Ok(())
	}
}
