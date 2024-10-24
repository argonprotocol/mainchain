//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{cmp::max, sync::Arc, time::Duration};

use sc_client_api::BlockBackend;
use sc_consensus_grandpa::{GrandpaBlockImport, SharedVoterState};
use sc_service::{
	config::Configuration, error::Error as ServiceError, TaskManager, WarpSyncConfig,
};
use sc_telemetry::{log, Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use url::Url;

use argon_bitcoin_utxo_tracker::UtxoTracker;
use argon_node_consensus::{
	aux_client::ArgonAux,
	compute_worker::run_compute_solver_threads,
	import_queue::{ArgonImportQueue, ArgonVerifier},
};
use argon_node_runtime::{self, opaque::Block, AccountId, RuntimeApi};

use crate::rpc;

pub(crate) type FullClient = sc_service::TFullClient<
	Block,
	RuntimeApi,
	sc_executor::WasmExecutor<sp_io::SubstrateHostFunctions>,
>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

type ArgonBlockImport = argon_node_consensus::import_queue::ArgonBlockImport<
	Block,
	GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>,
	FullClient,
	FullSelectChain,
	AccountId,
>;

pub type Service = sc_service::PartialComponents<
	FullClient,
	FullBackend,
	FullSelectChain,
	ArgonImportQueue<Block>,
	sc_transaction_pool::FullPool<Block, FullClient>,
	(
		ArgonBlockImport,
		ArgonAux<Block, FullClient>,
		Arc<UtxoTracker>,
		sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
		Option<Telemetry>,
	),
>;

pub fn new_partial(
	config: &Configuration,
	bitcoin_rpc_url: String,
) -> Result<Service, ServiceError> {
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
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);
	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let bitcoin_url = Url::parse(&bitcoin_rpc_url).map_err(|e| {
		ServiceError::Other(format!(
			"Unable to parse bitcoin rpc url ({}) {:?}",
			bitcoin_rpc_url, e
		))
	})?;
	let (user, password) = (bitcoin_url.username(), bitcoin_url.password());

	let bitcoin_auth = if !user.is_empty() {
		Some((user.to_string(), password.unwrap_or_default().to_string()))
	} else {
		None
	};

	let utxo_tracker = UtxoTracker::new(bitcoin_url.origin().unicode_serialization(), bitcoin_auth)
		.map_err(|e| {
			ServiceError::Other(format!("Failed to initialize bitcoin monitoring {:?}", e))
		})?;

	let utxo_tracker = Arc::new(utxo_tracker);

	let aux_client = ArgonAux::<Block, _>::new(client.clone());
	let argon_block_import = ArgonBlockImport::new(
		grandpa_block_import.clone(),
		client.clone(),
		aux_client.clone(),
		select_chain.clone(),
		utxo_tracker.clone(),
	);

	let import_queue = ArgonImportQueue::<Block>::new(
		ArgonVerifier::new(),
		Box::new(argon_block_import.clone()),
		Some(Box::new(grandpa_block_import.clone())),
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
	);

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (argon_block_import, aux_client, utxo_tracker, grandpa_link, telemetry),
	})
}

/// Builds a new service for a full client.
pub fn new_full<
	N: sc_network::NetworkBackend<Block, <Block as sp_runtime::traits::Block>::Hash>,
>(
	config: Configuration,
	mining_account_id: Option<AccountId>,
	mining_threads: Option<u32>,
	bitcoin_rpc_url: String,
) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		transaction_pool,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		other: (argon_block_import, aux_client, utxo_tracker, grandpa_link, mut telemetry),
	} = new_partial(&config, bitcoin_rpc_url)?;

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
	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
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
	let prometheus_registry = config.prometheus_registry().cloned();
	let keystore = keystore_container.keystore();

	#[cfg(not(debug_assertions))]
	{
		utxo_tracker.ensure_correct_network(&client).map_err(|e| {
			ServiceError::Other(format!("Failed to get bitcoin network validated {:?}", e))
		})?;
	}

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();

		Box::new(move |_| {
			rpc::create_full(rpc::FullDeps { client: client.clone(), pool: pool.clone() })
				.map_err(Into::into)
		})
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: Arc::new(network.clone()),
		client: client.clone(),
		keystore,
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: rpc_extensions_builder,
		backend,
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		config,
		telemetry: telemetry.as_mut(),
	})?;

	if role.is_authority() {
		if let Some(block_author) = mining_account_id {
			let proposer_factory_compute = sc_basic_authorship::ProposerFactory::new(
				task_manager.spawn_handle(),
				client.clone(),
				transaction_pool.clone(),
				prometheus_registry.as_ref(),
				telemetry.as_ref().map(|x| x.handle()),
			);

			// how long to take to actually build the block (i.e. executing extrinsics)
			let block_seconds = Duration::from_secs(10);

			let (compute_miner, compute_task) =
				argon_node_consensus::compute_worker::create_compute_miner(
					Box::new(argon_block_import.clone()),
					client.clone(),
					aux_client.clone(),
					proposer_factory_compute,
					sync_service.clone(),
					block_author.clone(),
					sync_service.clone(),
					utxo_tracker.clone(),
					block_seconds,
				);

			task_manager.spawn_essential_handle().spawn_blocking(
				"argon-compute-miner",
				Some("block-authoring"),
				compute_task,
			);

			let (
				seal_watch_task,
				notary_sync_task,
				notary_notebook_queue_task,
				create_block_stream,
			) = argon_node_consensus::block_creation_task(
				client.clone(),
				select_chain,
				aux_client.clone(),
				keystore_container.keystore(),
			);
			let proposer_factory_tax = sc_basic_authorship::ProposerFactory::new(
				task_manager.spawn_handle(),
				client.clone(),
				transaction_pool.clone(),
				prometheus_registry.as_ref(),
				telemetry.as_ref().map(|x| x.handle()),
			);
			let block_create_task = argon_node_consensus::tax_block_creator(
				Box::new(argon_block_import),
				client.clone(),
				aux_client.clone(),
				proposer_factory_tax,
				sync_service.clone(),
				block_seconds,
				create_block_stream,
				utxo_tracker.clone(),
			);

			task_manager.spawn_essential_handle().spawn_blocking(
				"argon-vote-block-seal-watch",
				Some("block-authoring"),
				seal_watch_task,
			);
			task_manager.spawn_essential_handle().spawn_blocking(
				"argon-vote-notary-sync",
				Some("block-authoring"),
				notary_sync_task,
			);
			task_manager.spawn_essential_handle().spawn_blocking(
				"argon-vote-notebook-queue",
				Some("block-authoring"),
				notary_notebook_queue_task,
			);
			task_manager.spawn_essential_handle().spawn_blocking(
				"argon-blocks",
				Some("block-authoring"),
				block_create_task,
			);

			let mining_threads = if let Some(mining_threads) = mining_threads {
				mining_threads as usize
			} else {
				max(num_cpus::get() - 1, 1)
			};
			if mining_threads > 0 {
				log::info!("Mining is enabled as {:?}, {} threads", block_author, mining_threads);
				run_compute_solver_threads(&task_manager, compute_miner, mining_threads)
			} else {
				log::info!("Mining is disabled");
			}
		} else {
			log::info!("Mining is disabled");
		}

		let grandpa_config = sc_consensus_grandpa::Config {
			// FIXME #1578 make this available through chainspec
			gossip_duration: Duration::from_millis(333),
			justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
			name: Some(name),
			observer_enabled: false,
			keystore: Some(keystore_container.keystore()),
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: grandpa_protocol_name,
		};

		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = sc_consensus_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			sync: Arc::new(sync_service),
			notification_service: grandpa_notification_service,
			voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			sc_consensus_grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	network_starter.start_network();
	Ok(task_manager)
}
