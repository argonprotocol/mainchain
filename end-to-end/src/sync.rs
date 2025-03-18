use argon_testing::ArgonTestNode;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use std::env;
use subxt::rpc_params;
use tokio::select;

#[derive(Clone, Serialize, Deserialize)]
pub struct EncodedFinalityProof(pub sp_core::Bytes);

/// Tests that finality can be proven
#[tokio::test]
#[serial]
async fn test_can_prove_finality() {
	env::set_var("RUST_LOG", "info");
	let grandpa_miner = ArgonTestNode::start_with_args("alice", 0).await.unwrap();
	let miner_threads = 1;
	let miner_1 = grandpa_miner.fork_node("bob", miner_threads).await.unwrap();
	let _miner_2 = grandpa_miner.fork_node("ferdie", miner_threads).await.unwrap();

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

	// now can we warp sync forward?
	let mut miner_3_args = grandpa_miner.get_fork_args("charlie", miner_threads);
	miner_3_args.is_archive_node = false;
	miner_3_args.extra_flags.push("--sync=warp".to_string());
	// miner_3_args.rust_log = "debug".to_string();

	miner_3_args.rust_log += ",sync=trace,warp=trace";
	let miner_2 = grandpa_miner.fork_node_with(miner_3_args).await.unwrap();
	let mut blocks_sub = miner_2.client.live.blocks().subscribe_finalized().await.unwrap();

	// wait for blocks sub, timeout after 30 seconds
	let mut finalized_count = grandpa_miner.client.latest_finalized_block().await.unwrap();
	loop {
		select! {
			Some(_block) = blocks_sub.next() => {
				finalized_count -= 1;
				if finalized_count == 0 {
					break;
				}
			}
			_ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
				break;
			}
		}
	}
	assert_eq!(finalized_count, 0);

	drop(miner_1);
	drop(grandpa_miner);
}
