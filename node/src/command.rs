use frame_benchmarking_cli::{BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE};
use sc_cli::SubstrateCli;
use sc_service::PartialComponents;
use sp_keyring::Sr25519Keyring;

use argon_node_runtime::{Block, EXISTENTIAL_DEPOSIT};

use crate::{
	benchmarking::{inherent_benchmark_data, RemarkBuilder, TransferKeepAliveBuilder},
	chain_spec,
	cli::{Cli, RandomxFlag, Subcommand},
	service,
};

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
			"" | "local" => Box::new(chain_spec::local_testnet_config()?),
			// This creates a whole new, incompatible genesis, so label it as such
			"fresh-testnet" => Box::new(chain_spec::testnet_config()?),
			"testnet" => Box::new(chain_spec::ChainSpec::from_json_bytes(
				&include_bytes!("./chain_spec/testnet1.json")[..],
			)?),
			path =>
				Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		})
	}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	color_backtrace::install();
	let cli = Cli::from_args();

	let bitcoin_rpc_url = cli.bitcoin_rpc_url.clone().unwrap_or_default();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				if bitcoin_rpc_url.is_empty() {
					return Err("Bitcoin RPC URL is required for block validation".into());
				}
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config, bitcoin_rpc_url)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } =
					service::new_partial(&config, bitcoin_rpc_url)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } =
					service::new_partial(&config, bitcoin_rpc_url)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				if bitcoin_rpc_url.is_empty() {
					return Err("Bitcoin RPC URL is required for block validation".into());
				}
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config, bitcoin_rpc_url)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, backend, .. } =
					service::new_partial(&config, bitcoin_rpc_url)?;
				let aux_revert = Box::new(|client, _, blocks| {
					sc_consensus_grandpa::revert(client, blocks)?;
					Ok(())
				});
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				// This switch needs to be in the client, since the client decides
				// which sub-commands it wants to support.
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						if !cfg!(feature = "runtime-benchmarks") {
							return Err(
								"Runtime benchmarking wasn't enabled when building the node. \
							You can enable it with `--features runtime-benchmarks`."
									.into(),
							);
						}

						cmd.run_with_spec::<sp_runtime::traits::HashingFor<Block>, ()>(Some(
							config.chain_spec,
						))
					},
					BenchmarkCmd::Block(cmd) => {
						let PartialComponents { client, .. } =
							service::new_partial(&config, bitcoin_rpc_url)?;
						cmd.run(client)
					},
					#[cfg(not(feature = "runtime-benchmarks"))]
					BenchmarkCmd::Storage(_) => Err(
						"Storage benchmarking can be enabled with `--features runtime-benchmarks`."
							.into(),
					),
					#[cfg(feature = "runtime-benchmarks")]
					BenchmarkCmd::Storage(cmd) => {
						let PartialComponents { client, backend, .. } =
							service::new_partial(&config, bitcoin_rpc_url)?;
						let db = backend.expose_db();
						let storage = backend.expose_storage();

						cmd.run(config, client, db, storage)
					},
					BenchmarkCmd::Overhead(cmd) => {
						let PartialComponents { client, .. } =
							service::new_partial(&config, bitcoin_rpc_url)?;
						let ext_builder = RemarkBuilder::new(client.clone());

						cmd.run(
							config,
							client,
							inherent_benchmark_data()?,
							Vec::new(),
							&ext_builder,
						)
					},
					BenchmarkCmd::Extrinsic(cmd) => {
						let PartialComponents { client, .. } =
							service::new_partial(&config, bitcoin_rpc_url)?;
						// Register the *Remark* and *TKA* builders.
						let ext_factory = ExtrinsicFactory(vec![
							Box::new(RemarkBuilder::new(client.clone())),
							Box::new(TransferKeepAliveBuilder::new(
								client.clone(),
								Sr25519Keyring::Alice.to_account_id(),
								EXISTENTIAL_DEPOSIT,
							)),
						]);

						cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
					},
					BenchmarkCmd::Machine(cmd) =>
						cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()),
				}
			})
		},
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		},
		None => {
			let runner = cli.create_runner(&cli.run.inner)?;
			if bitcoin_rpc_url.is_empty() {
				return Err("Bitcoin RPC URL is required to run a node".into());
			}

			let mut randomx_config = argon_randomx::Config::default();
			if cli.run.randomx_flags.contains(&RandomxFlag::LargePages) {
				randomx_config.large_pages = true;
			}
			if cli.run.randomx_flags.contains(&RandomxFlag::Secure) {
				randomx_config.secure = true;
			}
			let _ = argon_randomx::set_global_config(randomx_config);

			runner.run_node_until_exit(|config| async move {
				match config.network.network_backend {
					sc_network::config::NetworkBackendType::Libp2p => service::new_full::<
						sc_network::NetworkWorker<
							argon_node_runtime::opaque::Block,
							<argon_node_runtime::opaque::Block as sp_runtime::traits::Block>::Hash,
						>,
					>(
						config,
						cli.block_author(),
						cli.run.compute_miners,
						bitcoin_rpc_url,
					)
					.map_err(sc_cli::Error::Service),
					sc_network::config::NetworkBackendType::Litep2p =>
						service::new_full::<sc_network::Litep2pNetworkBackend>(
							config,
							cli.block_author(),
							cli.run.compute_miners,
							bitcoin_rpc_url,
						)
						.map_err(sc_cli::Error::Service),
				}
			})
		},
	}
}
