//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{cmp::max, sync::Arc, time::Duration};

use futures::FutureExt;
use sc_client_api::{Backend, BlockBackend};
use sc_consensus_grandpa::{FinalityProofProvider, GrandpaBlockImport, SharedVoterState};
use sc_service::{
	config::Configuration, error::Error as ServiceError, TaskManager, WarpSyncParams,
};
use sc_telemetry::{log, Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;

use ulx_node_consensus::{
	aux::UlxAux,
	basic_queue::BasicQueue,
	compute_worker::run_compute_solver_threads,
	import_queue::{UlxImportQueue, UlxVerifier},
};
use ulx_node_runtime::{self, opaque::Block, AccountId, RuntimeApi};

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

type UlxBlockImport = ulx_node_consensus::import_queue::UlxBlockImport<
	Block,
	GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>,
	FullClient,
	FullSelectChain,
	AccountId,
>;

#[allow(clippy::type_complexity)]
pub fn new_partial(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		BasicQueue<Block>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(
			UlxBlockImport,
			UlxAux<Block, FullClient>,
			sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
			Option<Telemetry>,
		),
	>,
	ServiceError,
> {
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

	let executor = sc_service::new_wasm_executor::<sp_io::SubstrateHostFunctions>(config);

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

	let aux_client = UlxAux::<Block, _>::new(client.clone());
	let ulx_block_import = UlxBlockImport::new(
		grandpa_block_import.clone(),
		client.clone(),
		aux_client.clone(),
		select_chain.clone(),
	);

	let import_queue = UlxImportQueue::<Block>::new(
		UlxVerifier::new(),
		Box::new(ulx_block_import.clone()),
		client.clone(),
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
		other: (ulx_block_import, aux_client, grandpa_link, telemetry),
	})
}

/// Builds a new service for a full client.
pub fn new_full(
	config: Configuration,
	mining_account_id: Option<AccountId>,
	mining_threads: Option<u32>,
) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		transaction_pool,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		other: (ulx_block_import, aux_client, grandpa_link, mut telemetry),
	} = new_partial(&config)?;

	let mut net_config = sc_network::config::FullNetworkConfiguration::new(&config.network);
	let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
		&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
		&config.chain_spec,
	);
	let (grandpa_protocol_config, grandpa_notification_service) =
		sc_consensus_grandpa::grandpa_peers_set_config(grandpa_protocol_name.clone());
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
			warp_sync_params: Some(WarpSyncParams::WithProvider(warp_sync)),
			block_relay: None,
		})?;

	if config.offchain_worker.enabled {
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-worker",
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				is_validator: config.role.is_authority(),
				keystore: Some(keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: network.clone(),
				enable_http_requests: true,
				custom_extensions: |_| vec![],
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	let role = config.role.clone();
	let name = config.network.node_name.clone();
	let prometheus_registry = config.prometheus_registry().cloned();
	let keystore = keystore_container.keystore();

	let (rpc_extensions_builder, shared_voter_state) = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let rpc_backend = backend.clone();
		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let shared_voter_state = SharedVoterState::empty();
		let shared_voter_state2 = shared_voter_state.clone();
		let finality_proof_provider = FinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);

		let rpc_extensions_builder = Box::new(move |deny_unsafe, subscription_executor| {
			let deps = rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				grandpa: rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor,
					finality_provider: finality_proof_provider.clone(),
				},
				backend: rpc_backend.clone(),
			};
			rpc::create_full(deps).map_err(Into::into)
		});
		(rpc_extensions_builder, shared_voter_state2)
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
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
				ulx_node_consensus::compute_worker::create_compute_miner(
					Box::new(ulx_block_import.clone()),
					client.clone(),
					aux_client.clone(),
					select_chain.clone(),
					proposer_factory_compute,
					sync_service.clone(),
					block_author.clone(),
					sync_service.clone(),
					block_seconds.clone(),
				);

			task_manager.spawn_essential_handle().spawn_blocking(
				"ulx-compute-miner",
				Some("block-authoring"),
				compute_task,
			);

			let (vote_watch_task, create_block_stream) = ulx_node_consensus::notary_client_task(
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
			let block_create_task = ulx_node_consensus::tax_block_creator(
				Box::new(ulx_block_import),
				client.clone(),
				aux_client.clone(),
				proposer_factory_tax,
				sync_service.clone(),
				block_seconds,
				create_block_stream,
			);

			task_manager.spawn_essential_handle().spawn_blocking(
				"ulx-vote-blocks-watch",
				Some("block-authoring"),
				vote_watch_task,
			);
			task_manager.spawn_essential_handle().spawn_blocking(
				"ulx-blocks",
				Some("block-authoring"),
				block_create_task,
			);

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
				shared_voter_state,
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

			let mining_threads = if let Some(mining_threads) = mining_threads {
				mining_threads as usize
			} else {
				max(num_cpus::get() - 1, 1)
			};
			log::info!("Mining is enabled, {} threads", mining_threads);
			run_compute_solver_threads(&mut task_manager, compute_miner, mining_threads);
		} else {
			log::info!("Mining is disabled");
		}
	}

	network_starter.start_network();
	Ok(task_manager)
}
