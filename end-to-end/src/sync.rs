use argon_testing::ArgonTestNode;
use polkadot_sdk::*;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use subxt::ext::subxt_rpcs::rpc_params;

#[derive(Clone, Serialize, Deserialize)]
pub struct EncodedFinalityProof(pub sp_core::Bytes);

/// Tests that finality can be proven
#[tokio::test]
#[serial]
async fn test_can_prove_finality() {
	let grandpa_miner = ArgonTestNode::start_with_args("alice", 0).await.unwrap();
	let miner_threads = 1;
	let miner_1 = grandpa_miner.fork_node("bob", miner_threads).await.unwrap();
	let miner_2 = grandpa_miner.fork_node("ferdie", miner_threads).await.unwrap();

	let mut blocks_sub = grandpa_miner.client.live.blocks().subscribe_finalized().await.unwrap();

	loop {
		if let Some(Ok(block)) = blocks_sub.next().await {
			let block_number = block.header().number;
			if block_number == 0 {
				continue;
			}
			let events = block.events().await.unwrap().iter().flatten().collect::<Vec<_>>();
			println!(
				"Events: {:?}",
				events
					.iter()
					.map(|a| format!("{}.{}", a.pallet_name(), a.variant_name()))
					.collect::<Vec<_>>()
			);
			// api won't work until grandpa rotates
			if !events.iter().any(|a| a.pallet_name() == "Grandpa") {
				continue;
			}
			let proof = grandpa_miner
				.client
				.rpc
				.request::<Option<EncodedFinalityProof>>(
					"grandpa_proveFinality",
					rpc_params![block_number],
				)
				.await
				.unwrap();
			assert!(proof.is_some());
			break;
		}
	}

	drop(miner_1);
	drop(miner_2);
	drop(grandpa_miner);
}
