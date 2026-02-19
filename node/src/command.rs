use crate::{
	chain_spec,
	cli::{Cli, RandomxFlag, Subcommand},
	runtime_api::{BaseHostRuntimeApis, opaque::Block},
	service,
	service::{FullClient, new_partial},
};
use argon_notary_apis::DownloadTrustMode;
use argon_primitives::prelude::sp_api::ConstructRuntimeApi;
use polkadot_sdk::{sc_service::TaskManager, *};
use sc_chain_spec::ChainSpec;
use sc_cli::{Error, Result as CliResult, SubstrateCli};
use sc_network::{Litep2pNetworkBackend, NetworkWorker, config::NetworkBackendType};
use sp_core::crypto::AccountId32;
use sp_keyring::Sr25519Keyring::Alice;
use std::cmp::max;
use url::Url;

type CanaryRuntimeApi = argon_canary_runtime::RuntimeApi;
type ArgonRuntimeApi = argon_runtime::RuntimeApi;
type BlockHashT = <Block as sp_runtime::traits::Block>::Hash;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Argon Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/argonprotocol/mainchain/issues".into()
	}

	fn copyright_start_year() -> i32 {
		2023
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(chain_spec::development_config()?),
			"meta" => Box::new(chain_spec::metadata_config()?),
			"dev-docker" => Box::new(chain_spec::docker_dev_config()?),
			"" | "local" => Box::new(chain_spec::local_testnet_config()?),
			// This creates a whole new, incompatible genesis, so label it as such
			"gen-testnet" => Box::new(chain_spec::testnet_config()?),
			"gen-main" => Box::new(chain_spec::mainnet_config()?),
			"testnet" => Box::new(chain_spec::ChainSpec::from_json_bytes(
				&include_bytes!("./chain_spec/testnet1.json")[..],
			)?),
			"mainnet" => Box::new(chain_spec::ChainSpec::from_json_bytes(
				&include_bytes!("./chain_spec/argon_foundation.json")[..],
			)?),
			path =>
				Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		})
	}
}
/// Can be called for a `Configuration` to check if it is a configuration for the `Crab Parachain`
/// network.
pub trait IdentifyVariant {
	fn is_canary(&self) -> bool;
}
impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_canary(&self) -> bool {
		matches!(self.id(), "argon-dev" | "argon-local" | "argon-testnet")
	}
}

macro_rules! construct_async_run {
	(|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
		let runner = $cli.create_runner($cmd)?;
		let mining_config = MiningConfig::new(&$cli);
		if runner.config().chain_spec.is_canary() {
			runner.async_run(|$config| {
				let $components = new_partial::<CanaryRuntimeApi>(&$config, &mining_config)?;
				Ok::<_, sc_cli::Error>(( { $( $code )* }, $components.task_manager))
			})
		} else {
			runner.async_run(|$config| {
				let $components = new_partial::<ArgonRuntimeApi>(&$config, &mining_config)?;
				Ok::<_, sc_cli::Error>(( { $( $code )* }, $components.task_manager))
			})
		}
	}}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	color_backtrace::install();
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				cmd.run(components.client, components.import_queue)
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				cmd.run(components.client, config.database)
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				cmd.run(components.client, config.chain_spec)
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				cmd.run(components.client, components.import_queue)
			})
		},
		Some(Subcommand::Revert(cmd)) => {
			construct_async_run!(|components, cli, cmd, config| {
				cmd.run(components.client, components.backend, None)
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		},
		None => {
			let mut cli = cli;
			// this is required for hyperbridge
			cli.run.base.offchain_worker_params.indexing_enabled = true;

			let runner = cli.create_runner(&cli.run.base)?;

			let mut randomx_config = argon_randomx::Config::default();
			if cli.run.compute_flags.contains(&RandomxFlag::LargePages) {
				randomx_config.large_pages = true;
			}
			if cli.run.compute_flags.contains(&RandomxFlag::Secure) {
				randomx_config.secure = true;
			}
			let _ = argon_randomx::full_vm::set_global_config(randomx_config);

			runner.run_node_until_exit(|config| async move {
				let mining_config = MiningConfig::new(&cli);

				let is_lipb2p =
					matches!(config.network.network_backend, NetworkBackendType::Libp2p);
				if config.chain_spec.is_canary() {
					run_node_with_runtime::<CanaryRuntimeApi>(config, mining_config, is_lipb2p)
				} else {
					run_node_with_runtime::<ArgonRuntimeApi>(config, mining_config, is_lipb2p)
				}
			})
		},
	}
}

fn run_node_with_runtime<R>(
	config: sc_service::Configuration,
	mining_config: MiningConfig,
	is_lipb2p: bool,
) -> Result<TaskManager, Error>
where
	R: ConstructRuntimeApi<Block, FullClient<R>> + Send + Sync + 'static,
	R::RuntimeApi: BaseHostRuntimeApis,
{
	if is_lipb2p {
		service::new_full::<R, NetworkWorker<Block, BlockHashT>>(config, mining_config)
	} else {
		service::new_full::<R, Litep2pNetworkBackend>(config, mining_config)
	}
	.map_err(Error::Service)
}

pub struct MiningConfig {
	compute_threads: Option<u32>,
	pub compute_author: Option<AccountId32>,
	bitcoin_rpc_url: Option<String>,
	pub notebook_archive_hosts: Vec<String>,
	pub notebook_download_trust_mode: DownloadTrustMode,
	pub notebook_header_max_bytes: Option<u64>,
	pub notebook_body_max_bytes: Option<u64>,
}

impl From<Cli> for MiningConfig {
	fn from(cli: Cli) -> Self {
		Self::new(&cli)
	}
}

impl MiningConfig {
	const MB: u64 = 1024 * 1024;

	pub fn new(cli: &Cli) -> Self {
		let compute_author = if let Some(compute_author) = &cli.run.compute_author {
			Some(compute_author.clone())
		} else if let Some(account) = &cli.run.base.get_keyring() {
			Some(account.to_account_id())
		} else if cli.run.base.shared_params.dev {
			Some(Alice.to_account_id())
		} else {
			None
		};

		let compute_threads = cli.run.compute_miners;

		let bitcoin_rpc_url = cli.bitcoin_rpc_url.clone();
		let notebook_download_trust_mode = Self::notebook_download_trust_mode(cli);
		let notebook_header_max_bytes = Some(Self::mb_to_bytes(cli.run.notebook_header_max_mb));
		let notebook_body_max_bytes = Some(Self::mb_to_bytes(cli.run.notebook_body_max_mb));

		Self {
			compute_threads,
			compute_author,
			bitcoin_rpc_url,
			notebook_archive_hosts: cli.run.notebook_archive_hosts.clone(),
			notebook_download_trust_mode,
			notebook_header_max_bytes,
			notebook_body_max_bytes,
		}
	}

	fn notebook_download_trust_mode(cli: &Cli) -> DownloadTrustMode {
		let shared_params = &cli.run.base.shared_params;
		let selected_chain_id = shared_params.chain_id(shared_params.is_dev());
		let is_dev_chain = cli
			.load_spec(&selected_chain_id)
			.map(|chain_spec| matches!(chain_spec.id(), "argon-dev" | "argon-local" | "argon-meta"))
			.unwrap_or_else(|_| {
				matches!(selected_chain_id.as_str(), "dev" | "dev-docker" | "local" | "meta")
			});
		if is_dev_chain { DownloadTrustMode::Dev } else { DownloadTrustMode::Strict }
	}

	fn mb_to_bytes(mb: u64) -> u64 {
		mb.saturating_mul(Self::MB)
	}

	pub fn compute_threads(&self) -> usize {
		let compute_threads = if let Some(compute_threads) = self.compute_threads {
			compute_threads as usize
		} else {
			max(num_cpus::get() - 1, 1)
		};
		if compute_threads > 0 {
			if self.compute_author.is_none() {
				panic!(
					"Compute fallback mining is enabled without a compute author. Unable to activate!"
				);
			}
			log::info!("Compute fallback mining is enabled with {compute_threads} threads");
		} else {
			log::info!("Compute fallback mining is disabled");
		}
		compute_threads
	}

	pub fn bitcoin_rpc_url_with_auth(&self) -> CliResult<(Url, Option<(String, String)>)> {
		let Some(bitcoin_rpc_url) = &self.bitcoin_rpc_url else {
			return Err(Error::Input(
				"Bitcoin RPC URL is required for block validation".to_string(),
			));
		};

		let mut bitcoin_url = Url::parse(bitcoin_rpc_url).map_err(|e| {
			Error::Input(format!("Unable to parse bitcoin rpc url ({bitcoin_rpc_url}) {e:?}"))
		})?;
		let (user, password) = (bitcoin_url.username(), bitcoin_url.password());

		let bitcoin_auth = if !user.is_empty() {
			Some((user.to_string(), password.unwrap_or_default().to_string()))
		} else {
			None
		};
		bitcoin_url.set_username("").ok();
		bitcoin_url.set_password(None).ok();
		Ok((bitcoin_url, bitcoin_auth))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use clap::Parser;

	fn parse_cli(args: &[&str]) -> Cli {
		Cli::parse_from(args)
	}

	#[test]
	fn notebook_size_defaults_are_in_mb_and_converted_to_bytes() {
		let cli = parse_cli(&["argon-node", "--chain", "mainnet"]);
		let config = MiningConfig::new(&cli);

		assert_eq!(config.notebook_header_max_bytes, Some(2 * 1024 * 1024));
		assert_eq!(config.notebook_body_max_bytes, Some(16 * 1024 * 1024));
	}

	#[test]
	fn notebook_size_mb_overrides_are_converted_to_bytes() {
		let cli = parse_cli(&[
			"argon-node",
			"--chain",
			"mainnet",
			"--notebook-header-max-mb",
			"2",
			"--notebook-body-max-mb",
			"4",
		]);
		let config = MiningConfig::new(&cli);

		assert_eq!(config.notebook_header_max_bytes, Some(2 * 1024 * 1024));
		assert_eq!(config.notebook_body_max_bytes, Some(4 * 1024 * 1024));
	}

	#[test]
	fn notebook_trust_mode_is_dev_for_dev_chains() {
		for chain in ["dev", "dev-docker", "local", "meta"] {
			let cli = parse_cli(&["argon-node", "--chain", chain]);
			let config = MiningConfig::new(&cli);
			assert_eq!(
				config.notebook_download_trust_mode,
				DownloadTrustMode::Dev,
				"expected dev mode for chain `{chain}`"
			);
		}
	}

	#[test]
	fn notebook_trust_mode_is_strict_for_live_chains() {
		for chain in ["mainnet", "testnet"] {
			let cli = parse_cli(&["argon-node", "--chain", chain]);
			let config = MiningConfig::new(&cli);
			assert_eq!(
				config.notebook_download_trust_mode,
				DownloadTrustMode::Strict,
				"expected strict mode for chain `{chain}`"
			);
		}
	}

	#[test]
	fn notebook_trust_mode_follows_chain_even_with_dev_flag() {
		let cli = parse_cli(&["argon-node", "--chain", "mainnet", "--dev"]);
		let config = MiningConfig::new(&cli);
		assert_eq!(config.notebook_download_trust_mode, DownloadTrustMode::Strict);
	}
}
