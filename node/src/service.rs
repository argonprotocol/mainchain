//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.
use crate::{
	command::MiningConfig,
	rpc,
	rpc::GrandpaDeps,
	runtime_api::{BaseHostRuntimeApis, opaque::Block},
};
use argon_bitcoin_utxo_tracker::UtxoTracker;
#[cfg(any(not(debug_assertions), test))]
use argon_node_consensus::read_chain_spec_bitcoin_network;
use argon_node_consensus::{
	BlockBuilderParams, NotaryClient, NotebookDownloader, aux_client::ArgonAux,
	create_import_queue, read_chain_spec_ticker, run_block_builder_task, run_notary_sync,
};
use argon_primitives::{AccountId, TickApis, digests::ArgonDigests, tick::Tick};
use polkadot_sdk::*;
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_consensus::BasicQueue;
use sc_consensus_grandpa::{
	BeforeBestBlockBy, FinalityProofProvider as GrandpaFinalityProofProvider, GrandpaBlockImport,
	ThreeQuartersOfTheUnfinalizedChain,
};
use sc_rpc::SubscriptionTaskExecutor;
use sc_service::{
	TaskManager, WarpSyncConfig, config::Configuration, error::Error as ServiceError,
};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::{ConstructRuntimeApi, ProvideRuntimeApi};
use sp_runtime::traits::Header as HeaderT;
use sp_keystore::{Keystore, KeystorePtr};
use std::{sync::Arc, time::Duration};

pub(crate) type FullClient<Runtime> = sc_service::TFullClient<
	Block,
	Runtime,
	sc_executor::WasmExecutor<sp_io::SubstrateHostFunctions>,
>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;
const GRANDPA_KEY_GUARD_INTERVAL: Duration = Duration::from_secs(30);

type ArgonBlockImport<Runtime> = argon_node_consensus::import_queue::ArgonBlockImport<
	Block,
	GrandpaBlockImport<FullBackend, Block, FullClient<Runtime>, FullSelectChain>,
	FullClient<Runtime>,
	AccountId,
>;

pub type Service<Runtime> = sc_service::PartialComponents<
	FullClient<Runtime>,
	FullBackend,
	FullSelectChain,
	BasicQueue<Block>,
	sc_transaction_pool::TransactionPoolHandle<Block, FullClient<Runtime>>,
	(
		ArgonBlockImport<Runtime>,
		Arc<NotaryClient<Block, FullClient<Runtime>, AccountId>>,
		ArgonAux<Block, FullClient<Runtime>>,
		Arc<UtxoTracker>,
		sc_consensus_grandpa::LinkHalf<Block, FullClient<Runtime>, FullSelectChain>,
		Option<Telemetry>,
	),
>;

pub fn new_partial<Runtime>(
	config: &Configuration,
	mining_config: &MiningConfig,
) -> Result<Service<Runtime>, ServiceError>
where
	Runtime: ConstructRuntimeApi<Block, FullClient<Runtime>> + Send + Sync + 'static,
	Runtime::RuntimeApi: BaseHostRuntimeApis,
{
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = sc_service::new_wasm_executor::<sp_io::SubstrateHostFunctions>(&config.executor);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, Runtime, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	// let runtime_overrides =
	// 	GrandpaStateOverrider::for_chain(Chain::from_str(config.chain_spec.name())?);
	// client.set_state_overrider(Box::new(runtime_overrides));

	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});
	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = Arc::from(
		sc_transaction_pool::Builder::new(
			task_manager.spawn_essential_handle(),
			client.clone(),
			config.role.is_authority().into(),
		)
		.with_options(config.transaction_pool.clone())
		.with_prometheus(config.prometheus_registry())
		.build(),
	);
	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let (bitcoin_url, bitcoin_auth) = mining_config
		.bitcoin_rpc_url_with_auth()
		.map_err(|e| ServiceError::Other(format!("Failed to parse bitcoin rpc url {e:?}")))?;
	let utxo_tracker = UtxoTracker::new(
		bitcoin_url.origin().unicode_serialization(),
		bitcoin_auth,
		config.prometheus_registry(),
	)
	.map_err(|e| ServiceError::Other(format!("Failed to initialize bitcoin monitoring {e:?}")))?;

	let utxo_tracker = Arc::new(utxo_tracker);

	let aux_client = ArgonAux::<Block, _>::new(client.clone());
	let ticker = read_chain_spec_ticker(config.chain_spec.as_ref())
		.map_err(|e| ServiceError::Other(e.to_string()))?;
	let ticker = {
		let latest_tick = resolve_startup_tick(&client)?;
		aux_client
			.migrate(latest_tick)
			.map_err(|e| ServiceError::Other(format!("Failed to migrate aux data: {e:?}")))?;
		ticker
	};
	let idle_delay = if ticker.tick_duration_millis <= 10_000 { 100 } else { 1000 };
	let notebook_downloader = NotebookDownloader::new(
		mining_config.notebook_archive_hosts.clone(),
		mining_config.notebook_download_trust_mode,
		mining_config.notebook_header_max_bytes,
		mining_config.notebook_body_max_bytes,
	)
	.map_err(|e| ServiceError::Other(format!("Failed to initialize notebook downloader {e:?}")))?;
	let notary_client = run_notary_sync(
		&task_manager,
		client.clone(),
		aux_client.clone(),
		idle_delay,
		notebook_downloader,
		config.prometheus_registry(),
		ticker,
		config.role.is_authority(),
	);

	let (import_queue, argon_block_import) = create_import_queue(
		client.clone(),
		aux_client.clone(),
		notary_client.clone(),
		Some(Box::new(grandpa_block_import.clone())),
		grandpa_block_import,
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
		telemetry.as_ref().map(|telemetry| telemetry.handle()),
		utxo_tracker.clone(),
	);

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (
			argon_block_import,
			notary_client,
			aux_client,
			utxo_tracker,
			grandpa_link,
			telemetry,
		),
	})
}

/// Builds a new service for a full client.
pub fn new_full<Runtime, N>(
	config: Configuration,
	mining_config: MiningConfig,
) -> sc_service::error::Result<TaskManager>
where
	Runtime: ConstructRuntimeApi<Block, FullClient<Runtime>> + Send + Sync + 'static,
	Runtime::RuntimeApi: BaseHostRuntimeApis,
	N: sc_network::NetworkBackend<Block, <Block as sp_runtime::traits::Block>::Hash>,
{
	let params = new_partial::<Runtime>(&config, &mining_config)?;
	let Service {
		select_chain,
		client,
		backend,
		mut task_manager,
		import_queue,
		transaction_pool,
		keystore_container,
		other,
	} = params;
	let (argon_block_import, notary_client, aux_client, utxo_tracker, grandpa_link, mut telemetry) =
		other;

	let metrics = N::register_notification_metrics(config.prometheus_registry());
	let mut net_config = sc_network::config::FullNetworkConfiguration::<
		Block,
		<Block as sp_runtime::traits::Block>::Hash,
		N,
	>::new(&config.network, config.prometheus_registry().cloned());
	let peer_store_handle = net_config.peer_store_handle();

	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);

	let (grandpa_protocol_config, grandpa_notification_service) =
		sc_consensus_grandpa::grandpa_peers_set_config::<_, N>(
			grandpa_protocol_name.clone(),
			metrics.clone(),
			Arc::clone(&peer_store_handle),
		);
	net_config.add_notification_protocol(grandpa_protocol_config);

	let warp_sync = Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		grandpa_link.shared_authority_set().clone(),
		Vec::default(),
	));
	let (network, system_rpc_tx, tx_handler_controller, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config: Some(WarpSyncConfig::WithProvider(warp_sync)),
			block_relay: None,
			metrics,
		})?;

	let role = config.role;
	let name = config.network.node_name.clone();
	let disable_grandpa = config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();

	#[cfg(not(debug_assertions))]
	{
		let bitcoin_network = read_chain_spec_bitcoin_network(config.chain_spec.as_ref())
			.map_err(|e| ServiceError::Other(e.to_string()))?;
		utxo_tracker.ensure_correct_network(bitcoin_network).map_err(|e| {
			ServiceError::Other(format!("Failed to get bitcoin network validated {:?}", e))
		})?;
	}
	let shared_voter_state = sc_consensus_grandpa::SharedVoterState::empty();

	let rpc_builder = {
		let client = client.clone();
		let transaction_pool = transaction_pool.clone();
		let backend = backend.clone();
		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let finality_proof_provider = GrandpaFinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);
		let shared_voter_state = shared_voter_state.clone();
		Box::new(move |subscription_executor: SubscriptionTaskExecutor| {
			let deps = rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				backend: backend.clone(),

				grandpa: GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor: subscription_executor.clone(),
					finality_provider: finality_proof_provider.clone(),
				},
			};

			rpc::create_full(deps).map_err(Into::into)
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config,
		keystore: keystore_container.keystore(),
		backend: backend.clone(),
		network: network.clone(),
		sync_service: sync_service.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
		tracing_execute_block: None,
	})?;

	if role.is_authority() {
		let compute_threads = mining_config.compute_threads() as u32;
		let compute_author = mining_config.compute_author;
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		run_block_builder_task(
			BlockBuilderParams {
				block_import: argon_block_import,
				client: client.clone(),
				notary_client,
				backend,
				keystore: keystore_container.keystore(),
				sync_oracle: sync_service.clone(),
				select_chain: select_chain.clone(),
				proposer: proposer_factory,
				authoring_duration: Duration::from_secs(10),
				utxo_tracker,
				aux_client: aux_client.clone(),
				justification_sync_link: sync_service.clone(),
				compute_author,
				compute_threads,
			},
			&task_manager,
		);
	}
	// grandpa voter task
	if !disable_grandpa {
		let grandpa_authority_keystore = if role.is_authority() {
			let keystore = keystore_container.keystore();
			ensure_single_local_grandpa_key(
				&keystore,
				"startup",
				"Refusing to start authority with multiple local GRANDPA keys",
			)?;

			let runtime_guard_keystore = keystore.clone();
			task_manager.spawn_essential_handle().spawn_blocking(
				"grandpa-key-guard",
				None,
				async move {
					loop {
						if let Err(err) = ensure_single_local_grandpa_key(
							&runtime_guard_keystore,
							"runtime",
							"Detected multiple local GRANDPA keys while running",
						) {
							log::error!("{err:?}");
							return;
						}

						std::thread::sleep(GRANDPA_KEY_GUARD_INTERVAL);
					}
				},
			);

			Some(keystore)
		} else {
			None
		};

		// TODO: we need to create a keystore for each grandpa voter we want to run. Probably a
		// service 	 that can dynamically allocate an deallocate voters with restricted/filtered
		// keystore access start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_voter = sc_consensus_grandpa::GrandpaParams {
			config: sc_consensus_grandpa::Config {
				// FIXME #1578 make this available through chainspec
				gossip_duration: Duration::from_millis(333),
				justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
				name: Some(name),
				observer_enabled: false,
				keystore: grandpa_authority_keystore,
				local_role: role,
				telemetry: telemetry.as_ref().map(|x| x.handle()),
				protocol_name: grandpa_protocol_name,
			},
			link: grandpa_link,
			network,
			sync: Arc::new(sync_service),
			notification_service: grandpa_notification_service,
			voting_rule: sc_consensus_grandpa::VotingRulesBuilder::new()
				// - Best Block (tick 5)
				// - Notebooks for tick (4)
				// - Eligible Votes (tick 3)
				// - Votes for blocks at tick 2
				.add(BeforeBestBlockBy(4u32))
				.add(ThreeQuartersOfTheUnfinalizedChain)
				.build(),
			prometheus_registry,
			shared_voter_state,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_consensus_grandpa::run_grandpa_voter(grandpa_voter)?,
		);
	}

	Ok(task_manager)
}

fn resolve_startup_tick<Runtime>(client: &Arc<FullClient<Runtime>>) -> Result<Tick, ServiceError>
where
	Runtime: ConstructRuntimeApi<Block, FullClient<Runtime>> + Send + Sync + 'static,
	Runtime::RuntimeApi: TickApis<Block>,
{
	let info = client.info();
	let best_header = client
		.header(info.best_hash)
		.map_err(|e| {
			ServiceError::Other(format!(
				"Failed to read best header {} at startup: {e:?}",
				info.best_hash
			))
		})?
		.ok_or_else(|| {
			ServiceError::Other(format!("Best header {} missing at startup", info.best_hash))
		})?;

	if *best_header.number() == 0 {
		// Genesis-only fallback: use runtime state while no tick digest exists yet.
		return client.runtime_api().current_tick(info.best_hash).map_err(|e| {
			ServiceError::Other(format!(
				"Failed to read startup tick from genesis runtime state: {e:?}"
			))
		});
	}

	best_header
		.digest()
		.convert_first(|a| a.as_tick())
		.map(|digest| digest.0)
		.ok_or_else(|| {
			ServiceError::Other(format!(
				"Missing tick digest in best header {} (#{}) at startup",
				info.best_hash,
				best_header.number()
			))
			})
}

fn ensure_single_local_grandpa_key(
	keystore: &KeystorePtr,
	stage: &'static str,
	err_prefix: &'static str,
) -> Result<(), ServiceError> {
	let grandpa_keys = keystore.keys(sp_consensus_grandpa::KEY_TYPE).map_err(|e| {
		ServiceError::Other(format!("Failed to list local GRANDPA keys ({stage}): {e:?}"))
	})?;
	let key_count = grandpa_keys.len();
	if key_count <= 1 {
		return Ok(());
	}

	let local_keys = grandpa_keys
		.into_iter()
		.map(|key| format!("0x{}", hex::encode(key)))
		.collect::<Vec<_>>()
		.join(",");

	Err(ServiceError::Other(format!(
		"{err_prefix} ({stage}): found {} keys [{}]. Configure exactly one local GRANDPA key.",
		key_count, local_keys
	)))
}

#[cfg(test)]
mod tests {
	use super::{read_chain_spec_bitcoin_network, read_chain_spec_ticker};
	use crate::chain_spec::{development_config, mainnet_config};
	use argon_primitives::{bitcoin::BitcoinNetwork, tick::Ticker};

	#[test]
	fn reads_dev_chain_genesis_values_from_state_anchor() {
		let chain_spec = development_config().expect("Development chain spec should build");
		let ticker = read_chain_spec_ticker(&chain_spec).expect("Ticker should decode");
		let bitcoin_network =
			read_chain_spec_bitcoin_network(&chain_spec).expect("Bitcoin network should decode");

		assert_eq!(ticker, Ticker::new(2_000, 2));
		assert_eq!(bitcoin_network, BitcoinNetwork::Regtest);
	}

	#[test]
	fn reads_mainnet_chain_genesis_values_from_state_anchor() {
		let chain_spec = mainnet_config().expect("Mainnet chain spec should build");
		let ticker = read_chain_spec_ticker(&chain_spec).expect("Ticker should decode");
		let bitcoin_network =
			read_chain_spec_bitcoin_network(&chain_spec).expect("Bitcoin network should decode");

		assert_eq!(ticker, Ticker::new(60_000, 60));
		assert_eq!(bitcoin_network, BitcoinNetwork::Bitcoin);
	}
}
