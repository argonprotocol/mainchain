use crate::{
	chain_spec,
	cli::{Cli, RandomxFlag, Subcommand},
	runtime_api::opaque::Block,
	service,
	service::new_partial,
};
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use sc_chain_spec::ChainSpec;
use sc_cli::{Error, Result as CliResult, SubstrateCli};
use sc_network::{config::NetworkBackendType, Litep2pNetworkBackend, NetworkWorker};
use sp_core::crypto::AccountId32;
use sp_keyring::AccountKeyring::Alice;
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
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			// Switch on the concrete benchmark sub-command-variant.
			match cmd {
				BenchmarkCmd::Pallet(cmd) =>
					if cfg!(feature = "runtime-benchmarks") {
						runner.sync_run(|config| {
							cmd.run_with_spec::<sp_runtime::traits::HashingFor<Block>, ()>(Some(
								config.chain_spec,
							))
						})
					} else {
						Err("Benchmarking wasn't enabled when building the node. \
					You can enable it with `--features runtime-benchmarks`."
							.into())
					},
				BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
					let mining_config = MiningConfig::new(&cli);
					if config.chain_spec.is_canary() {
						let partials = new_partial::<CanaryRuntimeApi>(&config, &mining_config)?;
						cmd.run(partials.client)
					} else {
						let partials = new_partial::<ArgonRuntimeApi>(&config, &mining_config)?;
						cmd.run(partials.client)
					}
				}),
				#[cfg(not(feature = "runtime-benchmarks"))]
				BenchmarkCmd::Storage(_) => Err(Error::Input(
					"Compile with --features=runtime-benchmarks \
						to enable storage benchmarks."
						.into(),
				)),
				#[cfg(feature = "runtime-benchmarks")]
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|config| {
					let mining_config = cli.into();
					if config.chain_spec.is_canary() {
						let partials = new_partial::<CanaryRuntimeApi>(&config, &mining_config)?;
						let db = partials.backend.expose_db();
						let storage = partials.backend.expose_storage();
						cmd.run(config, partials.client.clone(), db, storage)
					} else {
						let partials = new_partial::<ArgonRuntimeApi>(&config, &mining_config)?;
						let db = partials.backend.expose_db();
						let storage = partials.backend.expose_storage();
						cmd.run(config, partials.client.clone(), db, storage)
					}
				}),
				BenchmarkCmd::Machine(cmd) =>
					runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())),
				// NOTE: this allows the Client to leniently implement
				// new benchmark commands without requiring a companion MR.
				#[allow(unreachable_patterns)]
				_ => Err("Benchmarking sub-command unsupported".into()),
			}
		},
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		},
		None => {
			let mut cli = cli;
			// this is required for hyperbridge
			cli.run.base.offchain_worker_params.indexing_enabled = true;
			// Set max rpc request and response size to 150mb
			cli.run.base.rpc_max_request_size = 150;
			cli.run.base.rpc_max_response_size = 150;
			for x in cli.run.base.experimental_rpc_endpoint.iter_mut() {
				x.max_payload_in_mb = 150;
				x.max_payload_out_mb = 150;
			}

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
				if is_lipb2p {
					if config.chain_spec.is_canary() {
						service::new_full::<CanaryRuntimeApi, NetworkWorker<Block, BlockHashT>>(
							config,
							mining_config,
						)
					} else {
						service::new_full::<ArgonRuntimeApi, NetworkWorker<Block, BlockHashT>>(
							config,
							mining_config,
						)
					}
				} else if config.chain_spec.is_canary() {
					service::new_full::<CanaryRuntimeApi, Litep2pNetworkBackend>(
						config,
						mining_config,
					)
				} else {
					service::new_full::<ArgonRuntimeApi, Litep2pNetworkBackend>(
						config,
						mining_config,
					)
				}
				.map_err(Error::Service)
			})
		},
	}
}

pub struct MiningConfig {
	compute_threads: Option<u32>,
	pub compute_author: Option<AccountId32>,
	bitcoin_rpc_url: Option<String>,
	pub notebook_archive_hosts: Vec<String>,
}

impl From<Cli> for MiningConfig {
	fn from(cli: Cli) -> Self {
		Self::new(&cli)
	}
}

impl MiningConfig {
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

		Self {
			compute_threads,
			compute_author,
			bitcoin_rpc_url,
			notebook_archive_hosts: cli.run.notebook_archive_hosts.clone(),
		}
	}

	pub fn compute_threads(&self) -> usize {
		let compute_threads = if let Some(compute_threads) = self.compute_threads {
			compute_threads as usize
		} else {
			max(num_cpus::get() - 1, 1)
		};
		if compute_threads > 0 {
			if self.compute_author.is_none() {
				panic!("Compute fallback mining is enabled without a compute author. Unable to activate!");
			}
			log::info!("Compute fallback mining is enabled with {} threads", compute_threads);
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

		let bitcoin_url = Url::parse(bitcoin_rpc_url).map_err(|e| {
			Error::Input(format!("Unable to parse bitcoin rpc url ({}) {:?}", bitcoin_rpc_url, e))
		})?;
		let (user, password) = (bitcoin_url.username(), bitcoin_url.password());

		let bitcoin_auth = if !user.is_empty() {
			Some((user.to_string(), password.unwrap_or_default().to_string()))
		} else {
			None
		};
		Ok((bitcoin_url, bitcoin_auth))
	}
}
