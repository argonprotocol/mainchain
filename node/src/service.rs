//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{sync::Arc, thread, time::Duration};

use futures::FutureExt;
use sc_client_api::Backend;
pub use sc_executor::NativeElseWasmExecutor;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_core::U256;

use ulx_node_runtime::{self, opaque::Block, RuntimeApi};

use crate::pow::{NonceVerifier, UlixeePowAlgorithm};

// Our native executor instance.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
	/// Only enable the benchmarking host functions when we actually want to benchmark.
	#[cfg(feature = "runtime-benchmarks")]
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;
	/// Otherwise we only use the default Substrate host functions.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ExtendHostFunctions = ();

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		ulx_node_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		ulx_node_runtime::native_version()
	}
}

pub(crate) type FullClient =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
pub type Executor = NativeElseWasmExecutor<ExecutorDispatch>;

type POWBlockImport = sc_consensus_pow::PowBlockImport<
	Block,
	Arc<FullClient>,
	FullClient,
	FullSelectChain,
	UlixeePowAlgorithm<Block, FullClient>,
	Box<
		dyn sp_inherents::CreateInherentDataProviders<
			Block,
			(),
			InherentDataProviders = sp_timestamp::InherentDataProvider,
		>,
	>,
>;

#[allow(clippy::type_complexity)]
pub fn new_partial(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block, FullClient>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(POWBlockImport, Option<Telemetry>),
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

	let executor = sc_service::new_native_or_wasm_executor(config);
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, Executor>(
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

	let algorithm = UlixeePowAlgorithm::new(client.clone());
	let pow_block_import = sc_consensus_pow::PowBlockImport::new(
		client.clone(),
		client.clone(),
		algorithm.clone(),
		0, // check inherents starting at block 0
		select_chain.clone(),
		Box::new(move |_, ()| async move {
			let provider = sp_timestamp::InherentDataProvider::from_system_time();
			Ok(provider)
		})
			as Box<
				dyn sp_inherents::CreateInherentDataProviders<
					Block,
					(),
					InherentDataProviders = sp_timestamp::InherentDataProvider,
				>,
			>,
	);

	let import_queue = sc_consensus_pow::import_queue(
		Box::new(pow_block_import.clone()),
		None,
		algorithm,
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
	)?;

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (pow_block_import, telemetry),
	})
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (pow_block_import, mut telemetry),
	} = new_partial(&config)?;

	let net_config = sc_network::config::FullNetworkConfiguration::new(&config.network);

	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			// TODO: add back with a finalizer
			warp_sync_params: None,
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
	let prometheus_registry = config.prometheus_registry().cloned();

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, _| {
			let deps =
				crate::rpc::FullDeps { client: client.clone(), pool: pool.clone(), deny_unsafe };
			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network: network.clone(),
		client: client.clone(),
		keystore: keystore_container.keystore(),
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
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let algorithm = UlixeePowAlgorithm::new(client.clone());
		// Parameter details:
		//   https://substrate.dev/rustdocs/v3.0.0/sc_consensus_pow/fn.start_mining_worker.html
		// Also refer to kulupu config:
		//   https://github.com/kulupu/kulupu/blob/master/src/service.rs
		let (_worker, worker_task) = sc_consensus_pow::start_mining_worker(
			Box::new(pow_block_import),
			client.clone(),
			select_chain,
			algorithm,
			proposer_factory,
			sync_service.clone(),
			sync_service.clone(),
			// TODO: we probably want to add some randomness here.
			None,
			move |_, ()| async move {
				let provider = sp_timestamp::InherentDataProvider::from_system_time();
				Ok(provider)
			},
			// time to wait for a new block before starting to mine a new one
			Duration::from_secs(10),
			// how long to take to actually build the block (i.e. executing extrinsics)
			Duration::from_secs(10),
		);

		task_manager.spawn_essential_handle().spawn_blocking(
			"pow",
			Some("block-authoring"),
			worker_task,
		);

		// Start Mining
		let mut nonce = U256::from(rand::random::<u128>());
		let mut seal: [u8; 32] = [0; 32];

		thread::spawn(move || loop {
			let worker = _worker.clone();
			let metadata = worker.metadata();
			if let Some(metadata) = metadata {
				let mut verifier = NonceVerifier::new(&metadata.pre_hash, metadata.difficulty);
				nonce.to_big_endian(seal.as_mut_slice());
				if verifier.is_nonce_valid(&seal) {
					nonce = U256::from(rand::random::<u128>());
					let _ = futures::executor::block_on(worker.submit(seal.into()));
				} else {
					nonce = nonce.saturating_add(U256::from(1));
					if nonce == U256::MAX {
						nonce = U256::from(0);
					}
				}
			} else {
				thread::sleep(Duration::new(1, 0));
			}
		});
	}

	network_starter.start_network();
	Ok(task_manager)
}
