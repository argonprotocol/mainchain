use crate::{
	aux_client::ArgonAux,
	block_creator::BlockCreator,
	compute_worker::{ComputeHandle, run_compute_solver_threads},
	notary_client::VotingPowerInfo,
	notebook_sealer::NotebookSealer,
};
use argon_bitcoin_utxo_tracker::UtxoTracker;
use argon_primitives::{
	BLOCK_SEAL_KEY_TYPE, Balance, BitcoinApis, BlockCreatorApis, BlockSealApis,
	BlockSealAuthorityId, MiningApis, NotaryApis, NotebookApis, TickApis, VotingSchedule,
	inherents::BlockSealInherentNodeSide,
	prelude::{sp_arithmetic::Permill, sp_core::U256},
	tick::{Tick, Ticker},
};
use argon_runtime::{NotaryRecordT, NotebookVerifyError};
use codec::{Codec, MaxEncodedLen};
use futures::prelude::*;
use polkadot_sdk::*;
use sc_client_api::{AuxStore, BlockBackend, BlockchainEvents};
use sc_consensus::BlockImport;
use sc_service::TaskManager;
use sc_utils::mpsc::tracing_unbounded;
use schnellru::{ByLength, LruMap};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Environment, SelectChain, SyncOracle};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::traits::{Block as BlockT, Header};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time, time::Instant};
use tracing::{debug, info, trace, warn};

#[cfg(test)]
pub(crate) mod mock_importer;
#[cfg(test)]
pub(crate) mod mock_notary;
#[cfg(test)]
mod test;

pub mod aux_client;
mod aux_data;
pub(crate) mod block_creator;
pub(crate) mod compute_worker;
pub mod error;
pub mod import_queue;
pub(crate) mod metrics;
pub(crate) mod notary_client;
pub(crate) mod notebook_sealer;
pub(crate) mod pending_import_replay;
pub mod state_anchor;

pub use notary_client::{NotaryClient, NotebookDownloader, run_notary_sync};
pub use state_anchor::{
	DEFAULT_STATE_LOOKBACK_DEPTH, GenesisStorageReadError, ResolveBestOrFinalizedStateHashError,
	read_chain_spec_bitcoin_network, read_chain_spec_ticker, read_genesis_storage_value,
	resolve_best_or_finalized_state_hash, resolve_stateful_hash,
};

use crate::{compute_worker::ComputeState, notebook_sealer::create_vote_seal};
pub use import_queue::create_import_queue;

pub struct BlockBuilderParams<
	Block: BlockT,
	BI: Clone,
	Client: AuxStore,
	Proposer,
	A: Clone,
	SC: Clone,
	SO: Clone,
	JS: Clone,
	B,
> {
	/// The account id to use for authoring compute blocks
	pub compute_author: Option<A>,
	/// Used to actually import blocks.
	pub block_import: BI,
	/// The underlying para client.
	pub client: Arc<Client>,
	/// The underlying keystore, which should contain Aura consensus keys.
	pub keystore: KeystorePtr,
	/// The underlying block proposer this should call into.
	pub proposer: Proposer,
	/// The amount of time to spend authoring each block.
	pub authoring_duration: Duration,
	/// The aux client used to interact with the local auxillary storage
	pub aux_client: ArgonAux<Block, Client>,
	/// The Bitcoin UTXO tracker
	pub utxo_tracker: Arc<UtxoTracker>,

	pub justification_sync_link: JS,
	pub sync_oracle: SO,

	pub select_chain: SC,
	/// How many mining threads to activate
	pub compute_threads: u32,

	/// A notary client to verify notebooks
	pub notary_client: Arc<NotaryClient<Block, Client, A>>,

	pub backend: Arc<B>,
}

pub fn run_block_builder_task<Block, BI, C, PF, A, SC, SO, JS, B>(
	params: BlockBuilderParams<Block, BI, C, PF, A, SC, SO, JS, B>,
	task_manager: &TaskManager,
) where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	BI: BlockImport<Block> + Clone + Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>
		+ BlockchainEvents<Block>
		+ HeaderBackend<Block>
		+ BlockBackend<Block>
		+ AuxStore
		+ 'static,
	C::Api: NotebookApis<Block, NotebookVerifyError>
		+ BlockSealApis<Block, A, BlockSealAuthorityId>
		+ BlockCreatorApis<Block, A, NotebookVerifyError>
		+ NotaryApis<Block, NotaryRecordT>
		+ MiningApis<Block, A, BlockSealAuthorityId>
		+ TickApis<Block>
		+ BitcoinApis<Block, Balance>,
	PF: Environment<Block> + Send + Sync + 'static,
	PF::Proposer: sp_consensus::Proposer<Block>,
	A: Codec + Clone + MaxEncodedLen + PartialEq + Send + Sync + 'static,
	SC: SelectChain<Block> + Clone + Send + Sync + 'static,
	SO: SyncOracle + Clone + Send + Sync + 'static,
	JS: sc_consensus::JustificationSyncLink<Block> + Clone + Send + Sync + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
{
	let (compute_block_tx, mut compute_block_rx) =
		tracing_unbounded("node::consensus::compute_block_stream", 10);
	let (tax_vote_sender, mut tax_vote_rx) = tracing_unbounded("node::consensus::tax_votes", 1000);

	let BlockBuilderParams {
		compute_author,
		block_import,
		client,
		proposer,
		notary_client,
		authoring_duration,
		keystore,
		backend,
		aux_client,
		utxo_tracker,
		sync_oracle,
		select_chain,
		justification_sync_link,
		compute_threads,
	} = params;

	let consensus_metrics = notary_client.metrics.clone();
	let block_creator = BlockCreator {
		block_import,
		client: client.clone(),
		backend,
		proposer: Arc::new(Mutex::new(proposer)),
		authoring_duration,
		aux_client: aux_client.clone(),
		justification_sync_link,
		utxo_tracker,
		metrics: consensus_metrics.clone(),
		_phantom: Default::default(),
	};

	let ticker = {
		let best_hash = client.info().best_hash;
		client.runtime_api().ticker(best_hash).expect("Ticker not available")
	};

	let compute_handle = ComputeHandle::new(compute_block_tx);

	if compute_threads > 0 {
		run_compute_solver_threads(
			task_manager,
			compute_handle.clone(),
			compute_threads,
			consensus_metrics.clone(),
		)
	}

	let notebook_sealer = NotebookSealer::new(
		client.clone(),
		ticker,
		select_chain.clone(),
		keystore.clone(),
		aux_client.clone(),
		tax_vote_sender.clone(),
	);

	let creator = block_creator.clone();
	let seal_keystore = keystore.clone();
	// loop looking for next blocks to create
	let block_creator_task = async move {
		loop {
			tokio::select! {
				// tax blocks are built all at once with the winning seal
				tax_vote = tax_vote_rx.next() => {
					if let Some(command) = tax_vote {
						trace!("Got tax vote command at tick {:?}", command.current_tick);
						let vote = command.vote;
						let seal_strength = vote.seal_strength;
						let author = vote.closest_miner.0.clone();
						let miner_nonce_score = vote.miner_nonce_score.map(|(distance, _)| distance);
						let authority = vote.closest_miner.1.clone();

						let Some(proposal) =  creator
							.propose(
								author,
								command.current_tick,
								command.timestamp_millis,
								command.parent_hash,
								BlockSealInherentNodeSide::from_vote(vote),
							)
							.await else {
							continue;
						};
						let pre_hash = proposal.proposal.block.header().hash();
						let digest = match create_vote_seal(
							&seal_keystore,
							&pre_hash,
							&authority,
							seal_strength,
							miner_nonce_score,
						) {
							Ok(x) => x,
							Err(err) => {
								warn!("Unable to create vote seal: {:?}", err);
								continue;
							},
						};
						creator.submit_block(proposal, digest, &ticker).await;
					}
				},
				// compute blocks are created with a hash on top of the pre-built block
				compute_block = compute_block_rx.next() => {
					if let Some((block, digest)) = compute_block {
						creator.submit_block(block.proposal, digest, &ticker).await;
					}
				},

			}
		}
	};

	let is_compute_enabled = compute_threads > 0;
	let consensus_metrics_finder = consensus_metrics.clone();

	let block_finder_task = async move {
		*notary_client.pause_queue_processing.write().await = true;
		let mut import_stream = client.every_import_notification_stream();
		let mut finalized_stream = client.finality_notification_stream();
		let idle_delay = if ticker.tick_duration_millis <= 10_000 { 100 } else { 1000 };
		let idle_delay = Duration::from_millis(idle_delay);
		let mut notebook_tick_rx = notary_client.tick_voting_power_receiver.lock().await;
		let mut stale_branches = LruMap::new(ByLength::new(500));

		let compute_state = ComputeState::new(compute_handle.clone(), client.clone(), ticker);
		let mut notebook_ticks_recheck = NotebookTickChecker::new();
		loop {
			let mut check_notebook_tick: Option<Tick> = None;
			let mut next_notebooks_at_tick: Option<VotingPowerInfo> = None;
			tokio::select! {
				notebook = notebook_tick_rx.next() => {
					check_notebook_tick = notebook.map(|(t,_,_)| t);
					next_notebooks_at_tick = notebook;
				},
				block_next = import_stream.next() => {
					if let Some(block) = block_next {
						if block.origin == BlockOrigin::Own || sync_oracle.is_major_syncing() {
							continue;
						}
						let Ok(tick) = client.runtime_api().current_tick(block.hash) else {
							continue;
						};
						let block_number = *block.header.number();
						if block_number < client.info().finalized_number {
							continue;
						}
						if stale_branches.get(&block.hash).is_some() || stale_branches.get(block.header.parent_hash()).is_some() {
							continue;
						}
						// If this block can still be finalized, see if we can beat it. This could be the best block
						// or could be a new branch. NOTE: we only want to do this if we have notebooks, otherwise we might kick the
						// chain back to compute. We will try to solve again once the notebook arrives anyway and look for beatable blocks
						let voting_schedule = VotingSchedule::when_creating_block(tick);
						if let Ok( Some((tick, _, notebooks))) = aux_client.get_tick_voting_power(voting_schedule.notebook_tick()) {
							if notebooks > 0 {
								check_notebook_tick = Some(tick);
							}
						}
					}
				},
				finalized = finalized_stream.next() => {
					if let Some(finalized) = finalized {
						for stale in finalized.stale_blocks.iter() {
							if stale.is_head {
								stale_branches.insert(stale.hash, ());
							}
						}
						if let Some(metrics) = consensus_metrics_finder.as_ref() {
							let authority_keys = keystore.ed25519_public_keys(BLOCK_SEAL_KEY_TYPE).into_iter().map(BlockSealAuthorityId::from).collect::<HashSet<_>>();

							for hash in &[&*finalized.tree_route, &[finalized.hash]].concat() {
								let minted = client.runtime_api().get_block_payouts(*hash).unwrap_or_default();
								let mut is_my_block = false;
								let mut ownership_tokens = 0;
								let mut argons = 0;
								for payout in minted {
									if Some(payout.account_id) == compute_author {
										is_my_block = true;
										ownership_tokens += payout.ownership;
										argons += payout.argons;
									} else if let Some(authority) = payout.block_seal_authority {
										if authority_keys.contains(&authority) {
											is_my_block = true;
											argons += payout.argons;
											ownership_tokens += payout.ownership;
										}
									}
								}
								if is_my_block {
									tracing::info!(?hash, ownership_tokens, argons, "Your block got finalized!");
									metrics.record_finalized_block(ownership_tokens, argons);
								}
							}
						}
					}
				},
				_on_delay = time::sleep(notebook_ticks_recheck.get_next_check_delay().unwrap_or(idle_delay)) => {},
			}

			// don't try to check for blocks during a sync
			if sync_oracle.is_major_syncing() {
				*notary_client.pause_queue_processing.write().await = true;
				continue;
			}

			// make sure best hash is synched (there's a delay in some sync modes between the block
			// being imported and state synched)
			let best_hash = client.info().best_hash;
			let best_number = client.info().best_number;
			let state_status =
				client.block_status(best_hash).unwrap_or(sp_consensus::BlockStatus::Unknown);
			if state_status != sp_consensus::BlockStatus::InChainWithState {
				*notary_client.pause_queue_processing.write().await = true;
				debug!(
					?best_hash,
					?state_status,
					"Best block state not available (yet?). Not starting mining."
				);
				continue;
			}

			if *notary_client.pause_queue_processing.read().await {
				info!(
					?best_hash,
					?best_number,
					"🏁 Node state is synched. Activating notary sync."
				);
				*notary_client.pause_queue_processing.write().await = false;
			}

			let mut notebooks_to_check = notebook_ticks_recheck.get_ready();
			if let Some(notebook_tick) = check_notebook_tick {
				notebooks_to_check.insert(notebook_tick);
			}
			for notebook_tick in notebooks_to_check {
				match notebook_sealer.check_for_new_blocks(notebook_tick).await {
					Ok(result) =>
						if let Some(at_time) = result.recheck_notebook_tick_time {
							notebook_ticks_recheck.add(notebook_tick, at_time);
						},
					Err(err) => {
						tracing::warn!(notebook_tick, ?err, "Error while checking for new blocks",)
					},
				};
			}

			if !is_compute_enabled {
				continue;
			}

			// don't deal with compute blocks if we don't have a compute author
			let Some(ref compute_author) = compute_author else {
				continue;
			};

			let time = ticker.now_adjusted_to_ntp();
			let tick = ticker.tick_for_time(time);
			let Some(best_hash) = compute_state.on_new_notebook_tick(
				best_hash,
				next_notebooks_at_tick,
				&consensus_metrics_finder,
				tick,
			) else {
				continue;
			};
			if stale_branches.get(&best_hash).is_some() {
				trace!(?best_hash, "Best hash branch is stale, trying again.");
				continue;
			}

			// don't do anything if we are syncing or not ready to solve
			if !sync_oracle.is_major_syncing() &&
				compute_handle.ready_to_solve(tick, time) &&
				!compute_handle.is_solving()
			{
				if let Some(proposal) = block_creator
					.propose(
						compute_author.clone(),
						tick,
						time,
						best_hash,
						BlockSealInherentNodeSide::Compute,
					)
					.await
				{
					trace!(?best_hash, ?tick, "Fallback mining activated");
					compute_handle.start_solving(proposal);
				}
			}
		}
	};
	let handle = task_manager.spawn_essential_handle();
	handle.spawn("main-block-building-loop", Some("block-authoring"), block_finder_task);
	handle.spawn("block-creator", Some("block-authoring"), block_creator_task);
}

pub(crate) struct NotebookTickChecker {
	ticks_to_recheck: Vec<(Tick, Instant)>,
}

impl NotebookTickChecker {
	pub fn new() -> Self {
		Self { ticks_to_recheck: vec![] }
	}

	pub fn add(&mut self, tick: Tick, at_time: Instant) {
		if !self.ticks_to_recheck.iter().any(|(t, _)| *t == tick) {
			self.ticks_to_recheck.push((tick, at_time));
		}
	}

	pub fn get_next_check_delay(&self) -> Option<Duration> {
		if let Some(check_at) = self.ticks_to_recheck.iter().map(|(_, at_time)| *at_time).min() {
			let now = Instant::now();
			if check_at > now {
				return Some(check_at.saturating_duration_since(now));
			}
		}
		None
	}

	pub(crate) fn should_delay_block_attempt(
		block_tick: Tick,
		ticker: &Ticker,
		miner_nonce_score: Option<(U256, Permill)>,
	) -> Option<Instant> {
		let (_, percentile) = miner_nonce_score?;
		if block_tick == ticker.current() {
			// offset the block creation by the miner's percentile of nonce score
			// it must account for the current delay into the tick duration
			let duration_to_next_tick = ticker.duration_to_next_tick();
			let duration_per_tick = Duration::from_millis(ticker.tick_duration_millis);
			let elapsed = duration_per_tick.saturating_sub(duration_to_next_tick);
			let millis_offset = percentile.mul_floor(duration_per_tick.as_millis() as u64);
			let start_delay = Duration::from_millis(millis_offset);
			if start_delay > elapsed {
				let start_time = Some(Instant::now() + start_delay);
				tracing::trace!(
					start_delay = ?start_delay,
					miner_percentile = ?percentile,
					duration_to_next_tick = ?duration_to_next_tick,
					"Delay vote block creation due to miner percentile vs tick elapsed"
				);
				return start_time;
			}
		}
		None
	}

	pub fn get_ready(&mut self) -> HashSet<Tick> {
		let mut notebooks_to_check = HashSet::new();
		self.ticks_to_recheck.retain(|(tick, at_time)| {
			if at_time <= &Instant::now() {
				info!(notebook_tick = tick, "Re-checking beatable blocks at notebook tick");
				notebooks_to_check.insert(*tick);
				return false;
			}
			true
		});
		notebooks_to_check
	}
}
