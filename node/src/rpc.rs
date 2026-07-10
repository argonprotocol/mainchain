//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]
use polkadot_sdk::*;
use std::sync::Arc;

use crate::runtime_api::opaque::{Block, Hash};
use argon_primitives::{AccountId, Balance, BlockNumber, BlockSealAuthorityId, MiningApis, Nonce};
use jsonrpsee::{types::ErrorObjectOwned, RpcModule};
use sc_client_api::{AuxStore, BlockBackend, ProofProvider};
use sc_consensus_grandpa::FinalityProofProvider;
use sc_network_sync::{SyncState, SyncStatus, SyncingService, WarpSyncPhase};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

/// Full client dependencies.
pub struct FullDeps<C, P, B> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Current network synchronization service.
	pub sync_service: Arc<SyncingService<Block>>,
	/// Best block when the node's synchronization service started.
	pub starting_block: BlockNumber,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
}

/// Dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: sc_consensus_grandpa::SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: sc_consensus_grandpa::SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: sc_consensus_grandpa::GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: sc_rpc::SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, B>(
	deps: FullDeps<C, P, B>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ ProofProvider<Block>
		+ BlockBackend<Block>
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ AuxStore,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BlockBuilder<Block>,
	C::Api: MiningApis<Block, AccountId, BlockSealAuthorityId>,
	P: TransactionPool + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: sc_client_api::StateBackend<sp_runtime::traits::HashingFor<Block>>,
{
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut module = RpcModule::new(());
	let FullDeps { client, pool, sync_service, starting_block, grandpa } = deps;
	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	module.merge(System::new(client.clone(), pool).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

	module.merge(
		Grandpa::new(
			subscription_executor,
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;

	let sync_service = sync_service.clone();
	let client_for_sync_status = client.clone();
	module.register_async_method("system_syncStatus", move |_, _, _| {
		let sync_service = sync_service.clone();
		let client = client_for_sync_status.clone();

		async move {
			let sync_status = sync_service.status().await.map_err(|_| {
				ErrorObjectOwned::owned(-32603, "Syncing service has terminated", None::<()>)
			})?;
			let current_block = client.info().best_number;

			Ok::<_, ErrorObjectOwned>(to_sync_status_response(
				starting_block,
				current_block,
				sync_status,
			))
		}
	})?;

	Ok(module)
}

fn to_sync_status_response(
	starting_block: BlockNumber,
	current_block: BlockNumber,
	sync_status: SyncStatus<Block>,
) -> serde_json::Value {
	let (state, target_block) = match sync_status.state {
		SyncState::Idle => ("idle", None),
		SyncState::Downloading { target } => ("downloading", Some(target)),
		SyncState::Importing { target } => ("importing", Some(target)),
	};

	serde_json::json!({
		"startingBlock": starting_block,
		"currentBlock": current_block,
		"state": state,
		"targetBlock": target_block,
		"bestSeenBlock": sync_status.best_seen_block,
		"numPeers": sync_status.num_peers,
		"queuedBlocks": sync_status.queued_blocks,
		"stateSync": sync_status.state_sync.map(|progress| serde_json::json!({
			"percentage": progress.percentage,
			"size": progress.size,
			"phase": match progress.phase {
				sc_network_sync::strategy::state_sync::StateSyncPhase::DownloadingState => "downloadingState",
				sc_network_sync::strategy::state_sync::StateSyncPhase::ImportingState => "importingState",
			},
		})),
		"warpSync": sync_status.warp_sync.map(|progress| {
			let (phase, block) = match progress.phase {
				WarpSyncPhase::AwaitingPeers { .. } => ("awaitingPeers", None),
				WarpSyncPhase::DownloadingWarpProofs => ("downloadingWarpProofs", None),
				WarpSyncPhase::DownloadingTargetBlock => ("downloadingTargetBlock", None),
				WarpSyncPhase::DownloadingState => ("downloadingState", None),
				WarpSyncPhase::ImportingState => ("importingState", None),
				WarpSyncPhase::DownloadingBlocks(block) => ("downloadingBlocks", Some(block)),
				WarpSyncPhase::Complete => ("complete", None),
			};

			serde_json::json!({
				"phase": phase,
				"block": block,
				"totalBytes": progress.total_bytes,
				"status": progress.status,
			})
		}),
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use sc_network_sync::{
		strategy::state_sync::{StateSyncPhase, StateSyncProgress},
		SyncState, SyncStatus, WarpSyncPhase, WarpSyncProgress,
	};
	use serde_json::json;

	#[test]
	fn converts_sync_status() {
		let status = SyncStatus::<Block> {
			state: SyncState::Downloading { target: 42 },
			best_seen_block: Some(42),
			num_peers: 3,
			queued_blocks: 7,
			state_sync: Some(StateSyncProgress {
				percentage: 65,
				size: 1234,
				phase: StateSyncPhase::DownloadingState,
			}),
			warp_sync: Some(WarpSyncProgress {
				phase: WarpSyncPhase::DownloadingBlocks(40),
				total_bytes: 5678,
				status: Some("downloading history".to_string()),
			}),
		};

		let response = to_sync_status_response(10, 20, status);

		assert_eq!(
			serde_json::to_value(response).unwrap(),
			json!({
				"startingBlock": 10,
				"currentBlock": 20,
				"state": "downloading",
				"targetBlock": 42,
				"bestSeenBlock": 42,
				"numPeers": 3,
				"queuedBlocks": 7,
				"stateSync": {
					"percentage": 65,
					"size": 1234,
					"phase": "downloadingState"
				},
				"warpSync": {
					"phase": "downloadingBlocks",
					"block": 40,
					"totalBytes": 5678,
					"status": "downloading history"
				}
			})
		);
	}
}
