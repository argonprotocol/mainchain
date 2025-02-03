use crate::{
	aux_client::ArgonAux,
	block_creator::BlockCreator,
	compute_worker::{run_compute_solver_threads, ComputeHandle},
	notary_client::VotingPowerInfo,
	notebook_sealer::NotebookSealer,
};
use argon_bitcoin_utxo_tracker::UtxoTracker;
use argon_primitives::{
	inherents::BlockSealInherentNodeSide, Balance, BitcoinApis, BlockCreatorApis, BlockSealApis,
	BlockSealAuthorityId, MiningApis, NotaryApis, NotebookApis, TickApis, VotingSchedule,
	BLOCK_SEAL_KEY_TYPE,
};
use argon_runtime::{NotaryRecordT, NotebookVerifyError};
use codec::Codec;
use futures::prelude::*;
use sc_client_api::{AuxStore, BlockchainEvents};
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
use tokio::{sync::Mutex, time};
use tracing::{trace, warn};

#[cfg(test)]
pub(crate) mod mock_notary;

pub mod aux_client;
mod aux_data;
pub(crate) mod block_creator;
pub(crate) mod compute_worker;
pub mod error;
pub mod import_queue;
pub(crate) mod metrics;
pub(crate) mod notary_client;
pub(crate) mod notebook_sealer;

pub use notary_client::{run_notary_sync, NotaryClient, NotebookDownloader};

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
	A: Codec + Clone + PartialEq + Send + Sync + 'static,
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
		let mut import_stream = client.every_import_notification_stream();
		let mut finalized_stream = client.finality_notification_stream();
		let idle_delay = if ticker.tick_duration_millis <= 10_000 { 100 } else { 1000 };
		let idle_delay = Duration::from_millis(idle_delay);
		let mut notebook_tick_rx = notary_client.tick_voting_power_receiver.lock().await;
		let mut stale_branches = LruMap::new(ByLength::new(500));

		let compute_state = ComputeState::new(compute_handle.clone(), client.clone(), ticker);
		loop {
			let mut check_for_better_blocks: Option<VotingPowerInfo> = None;
			let mut next_notebooks_at_tick: Option<VotingPowerInfo> = None;
			tokio::select! {
				notebook = notebook_tick_rx.next() => {
					check_for_better_blocks = notebook;
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
						if stale_branches.get(&block.hash).is_some() {
							continue;
						}
						// If this block can still be finalized, see if we can beat it. This could be the best block
						// or could be a new branch
						let voting_schedule = VotingSchedule::when_creating_block(tick);
						if let Ok(info) = aux_client.get_tick_voting_power(voting_schedule.notebook_tick()) {
							check_for_better_blocks = info
						}
					}
				},
				finalized = finalized_stream.next() => {
					if let Some(finalized) = finalized {
						for hash in finalized.stale_heads.iter() {
							stale_branches.insert(*hash, ());
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
				_on_delay = time::sleep(idle_delay) => {},
			}

			// don't try to check for blocks during a sync
			if sync_oracle.is_major_syncing() {
				continue;
			}

			if let Some((notebook_tick, voting_power, notebooks)) = check_for_better_blocks {
				if let Err(err) = notebook_sealer
					.check_for_new_blocks(notebook_tick, voting_power, notebooks)
					.await
				{
					warn!("Error while checking for new blocks: {:?}", err);
				}
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

#[cfg(test)]
mod test {
	use argon_runtime::{Block, Header};
	use codec::{Decode, Encode};
	use sc_consensus_grandpa::{FinalityProof, GrandpaJustification};
	use sp_runtime::RuntimeAppPublic;
	#[test]
	fn decode_finality() {
		// First block in mainnet on 105 is 17572. Version deployed on
		// set id 1 at 17574
		// set id 0
		let encoded_17573 = hex::decode("7927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712e9062d560600000000007927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712a54400000c7927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712a54400005c9a28ed3f1a5bb94bd8780e9ad3640edd55652dd516fd303d539c64b25572de7b31a99ad3e0e8d636a5615978b107da853b3d9e2360fcbf2e3b1542e77fce0a45a74d33ead0b5ff58607fc60556cf1b291d4c503254ae07f17b3d54f8c5c27f7927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712a5440000b5c9b05beb4413ed4565492339a1735ff25f419d100d6cea8e5c7947f3ffb9e6079c034c23720f4cbb6151e260b2bd5fedfd3667589c338f5271787b27f08b0c803c5c3c4059380a8603f785a093c227a8a2f4a7437c466f1f7233a6881400e67927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712a54400009230a62facc51e07b1210eb52d7257d12257c66106aa723439019070b647efc9d25e7d58dde8cc2e4a8abec11601895c069ad09ad98e77e9af76aa2fb72e560f962abf1be4e94bb80e6488a2af551c529571fdd1d972b5c7e311d7507f0882ec0000").unwrap();

		let finality_proof = FinalityProof::<Header>::decode(&mut &encoded_17573[..]).unwrap();

		let justification =
			GrandpaJustification::<Block>::decode(&mut &finality_proof.justification[..]).unwrap();

		for signed in justification.justification.commit.precommits.iter() {
			let message = finality_grandpa::Message::Precommit(signed.precommit.clone());

			for i in 0..10u64 {
				let buf = (message.clone(), justification.justification.round, i).encode();
				if signed.id.verify(&buf, &signed.signature) {
					println!("Signature verified at {}", i);
					assert_eq!(i, 0);
				}
			}
		}
	}
}
