use crate::{
	aux_client::ArgonAux,
	block_creator::BlockCreator,
	compute_worker::{run_compute_solver_threads, ComputeHandle},
	notary_client::{run_notary_sync, VotingPowerInfo},
	notebook_sealer::NotebookSealer,
};
use argon_bitcoin_utxo_tracker::UtxoTracker;
use argon_node_runtime::{NotaryRecordT, NotebookVerifyError};
use argon_primitives::{
	inherents::BlockSealInherentNodeSide, Balance, BitcoinApis, BlockCreatorApis, BlockSealApis,
	BlockSealAuthorityId, NotaryApis, NotebookApis, TickApis, VotingSchedule,
};
use codec::Codec;
use futures::prelude::*;
use futures_timer::Delay;
use sc_client_api::{AuxStore, BlockchainEvents};
use sc_consensus::BlockImport;
use sc_service::TaskManager;
use sc_utils::mpsc::tracing_unbounded;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::{BlockOrigin, Environment, SelectChain, SyncOracle};
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block as BlockT, Header};
use sp_timestamp::Timestamp;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{trace, warn};

#[cfg(test)]
pub(crate) mod mock_notary;

pub mod aux_client;
mod aux_data;
pub(crate) mod block_creator;
pub(crate) mod compute_worker;
pub mod error;
pub mod import_queue;
pub(crate) mod notary_client;
pub(crate) mod notebook_sealer;

use crate::{compute_worker::MiningMetadata, notebook_sealer::create_vote_seal};
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
> {
	pub author: A,
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
	pub mining_threads: u32,
}

pub fn run_block_builder_task<Block, BI, C, PF, A, SC, SO, JS>(
	params: BlockBuilderParams<Block, BI, C, PF, A, SC, SO, JS>,
	task_manager: &TaskManager,
) where
	Block: BlockT + 'static,
	Block::Hash: Send + 'static,
	BI: BlockImport<Block> + Clone + Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>
		+ BlockchainEvents<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ 'static,
	C::Api: NotebookApis<Block, NotebookVerifyError>
		+ BlockSealApis<Block, A, BlockSealAuthorityId>
		+ BlockCreatorApis<Block, A, NotebookVerifyError>
		+ NotaryApis<Block, NotaryRecordT>
		+ TickApis<Block>
		+ BitcoinApis<Block, Balance>,
	PF: Environment<Block> + Send + Sync + 'static,
	PF::Proposer: sp_consensus::Proposer<Block>,
	A: Codec + Clone + Send + Sync + 'static,
	SC: SelectChain<Block> + Clone + Send + Sync + 'static,
	SO: SyncOracle + Clone + Send + Sync + 'static,
	JS: sc_consensus::JustificationSyncLink<Block> + Clone + Send + Sync + 'static,
{
	let (notebook_tick_tx, mut notebook_tick_rx) =
		tracing_unbounded("node::consensus::notebook_tick_stream", 100);
	let (compute_block_tx, mut compute_block_rx) =
		tracing_unbounded("node::consensus::compute_block_stream", 10);
	let (tax_vote_sender, mut tax_vote_rx) = tracing_unbounded("node::consensus::tax_votes", 1000);

	let BlockBuilderParams {
		author,
		block_import,
		client,
		proposer,
		authoring_duration,
		keystore,
		aux_client,
		utxo_tracker,
		sync_oracle,
		select_chain,
		justification_sync_link,
		mining_threads,
	} = params;

	let block_creator = BlockCreator {
		author,
		block_import,
		client: client.clone(),
		proposer: Arc::new(Mutex::new(proposer)),
		authoring_duration,
		aux_client: aux_client.clone(),
		justification_sync_link,
		utxo_tracker,
	};
	let best_hash = client.info().best_hash;
	let ticker = client.runtime_api().ticker(best_hash).expect("Ticker not available");
	let idle_delay = if ticker.tick_duration_millis <= 10_000 { 100 } else { 1000 };
	run_notary_sync(task_manager, client.clone(), aux_client.clone(), notebook_tick_tx, idle_delay);

	let compute_handle = ComputeHandle::new(compute_block_tx);

	if mining_threads > 0 {
		run_compute_solver_threads(task_manager, compute_handle.clone(), mining_threads)
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
	// loop looking for next blocks to create
	let block_creator_task = async move {
		loop {
			tokio::select! {biased;
				// tax blocks are built all at once with the winning seal
				tax_vote = tax_vote_rx.next() => {
					if let Some(command) = tax_vote {
						trace!("Got tax vote command at tick {:?}", command.current_tick);
						let vote = command.vote;
						let seal_strength = vote.seal_strength;
						let authority = vote.closest_miner.1.clone();

						let Some(proposal) =  creator
							.propose(
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
							&keystore,
							&pre_hash,
							&authority,
							seal_strength,
						) {
							Ok(x) => x,
							Err(err) => {
								warn!("Unable to create vote seal: {:?}", err);
								continue;
							},
						};
						creator.submit_block(proposal, digest).await;
					}
				},
				// compute blocks are created with a hash on top of the pre-built block
				compute_block = compute_block_rx.next() => {
					if let Some((block, digest)) = compute_block {
						creator.submit_block(block.proposal, digest).await;
					}
				},

			}
		}
	};

	let block_finder_task = async move {
		let ticker = ticker;
		let mut import_stream = client.every_import_notification_stream();
		let compute_delay = Duration::from_millis(idle_delay);

		loop {
			let mut tick = ticker.current();
			let mut next: Option<VotingPowerInfo> = None;
			tokio::select! {biased;
				notebook = notebook_tick_rx.next() => {
					next = notebook;
				},
				block_next = import_stream.next() => {
					if let Some(block) = block_next {
						if block.origin == BlockOrigin::Own || sync_oracle.is_major_syncing() {
							continue;
						}
						let Ok(tick) = client.runtime_api().current_tick(block.hash) else {
							continue;
						};
						let voting_schedule = VotingSchedule::when_creating_block(tick);
						if let Ok(info) = aux_client.get_tick_voting_power(voting_schedule.notebook_tick()) {
							next = info
						}
					}
				},
				_on_delay = Delay::new(compute_delay) => {
					next = None;
				},
			}

			// don't try to check for blocks during a sync
			if sync_oracle.is_major_syncing() {
				continue;
			}

			let next_tick = ticker.current();
			if next.is_none() && tick != next_tick {
				tick = next_tick;
				let voting_schedule = VotingSchedule::when_creating_block(tick);
				if let Ok(info) = aux_client.get_tick_voting_power(voting_schedule.notebook_tick())
				{
					next = info
				}
			}

			if let Some((notebook_tick, voting_power, notebooks)) = next {
				if let Err(err) = notebook_sealer
					.check_for_new_blocks(notebook_tick, voting_power, notebooks)
					.await
				{
					warn!("Error while checking for new blocks: {:?}", err);
				}
			}

			// TODO: this whole section needs to loop within a tick to create blocks, and as more
			//   notebooks come in,  it needs to create more blocks (cause they're stronger than
			//   the previous ones)

			// clear out anything that should no longer be running compute
			let best_hash = client.info().best_hash;
			if compute_handle.best_hash() != Some(best_hash) {
				compute_handle.stop_solving_current();

				let block_tick = client.runtime_api().current_tick(best_hash).unwrap_or_default();
				let has_tax_votes =
					client.runtime_api().has_eligible_votes(best_hash).unwrap_or_default();
				let compute_puzzle = match client.runtime_api().compute_puzzle(best_hash) {
					Ok(x) => x,
					Err(err) => {
						warn!(
							"Unable to pull new block for compute miner. No difficulty found!! {}",
							err
						);
						continue;
					},
				};
				let next_tick = ticker.time_for_tick(block_tick + 1);
				// allow a little bit of time for the block to be built, but wait for notebooks to
				// come in
				let delay = ticker.tick_duration_millis * 2 / 3;

				let genesis_hash = client.info().genesis_hash;
				compute_handle.new_best_block(MiningMetadata {
					best_hash,
					has_tax_votes,
					activate_mining_time: Timestamp::from(next_tick + delay),
					key_block_hash: compute_puzzle.get_key_block(genesis_hash),
					emergency_tick: block_tick + 3,
					difficulty: compute_puzzle.difficulty,
				});
			}

			// don't do anything if we are syncing or not ready to solve
			let time = Timestamp::current();
			let tick = ticker.tick_for_time(time.as_millis());
			if sync_oracle.is_major_syncing() || !compute_handle.ready_to_solve(tick, time) {
				continue;
			}

			// check for any new mining that should be activated
			let best_hash = client.info().best_hash;
			if compute_handle.proposal_parent_hash() != Some(best_hash) {
				trace!(?best_hash, ?tick, "Fallback mining activated");
				let Some(proposal) = block_creator
					.propose(tick, time.as_millis(), best_hash, BlockSealInherentNodeSide::Compute)
					.await
				else {
					continue;
				};

				let pre_hash = proposal.proposal.block.header().hash();

				compute_handle.start_solving(best_hash, pre_hash, proposal);
			}
		}
	};
	let handle = task_manager.spawn_essential_handle();
	handle.spawn("main-block-building-loop", Some("block-authoring"), block_finder_task);
	handle.spawn("block-creator", Some("block-authoring"), block_creator_task);
}
