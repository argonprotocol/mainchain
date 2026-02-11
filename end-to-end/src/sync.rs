use argon_testing::ArgonTestNode;
use polkadot_sdk::*;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use subxt::ext::subxt_rpcs::rpc_params;
use tokio::select;

#[derive(Clone, Serialize, Deserialize)]
pub struct EncodedFinalityProof(pub sp_core::Bytes);

/// Tests that finality can be proven
#[tokio::test]
#[serial]
async fn test_can_prove_finality() {
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
	let mut miner_3_args = grandpa_miner.get_fork_args("charlie", 0);
	miner_3_args.is_archive_node = false;
	miner_3_args.is_validator = false;
	miner_3_args.extra_flags.push("--sync=warp".to_string());
	miner_3_args.rust_log += ",sync=trace,warp=trace";
	let finalized_count = grandpa_miner.client.latest_finalized_block().await.unwrap();
	let miner_3 = grandpa_miner.fork_node_with(miner_3_args).await.unwrap();
	println!("Charlie started");
	let mut blocks_sub = miner_3.client.live.blocks().subscribe_finalized().await.unwrap();

	// Wait for Charlie to catch up to Alice's finalized height snapshot.
	let timeout_at = tokio::time::Instant::now() + tokio::time::Duration::from_secs(90);
	let mut charlie_finalized = 0u32;
	loop {
		if charlie_finalized >= finalized_count {
			break;
		}
		if tokio::time::Instant::now() >= timeout_at {
			break;
		}

		select! {
			maybe_block = blocks_sub.next() => {
				match maybe_block {
					Some(Ok(block)) => {
						charlie_finalized = block.number();
						println!(
							"Block finalized for charlie. Height {charlie_finalized}, target {finalized_count}"
						);
					}
					Some(Err(error)) => {
						println!("Charlie finalized subscription error: {error:#}");
					}
					None => {
						println!("Charlie finalized subscription ended before catchup");
						break;
					}
				}
			}
			_ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {}
		}
	}
	assert!(
		charlie_finalized >= finalized_count,
		"Charlie finalized {charlie_finalized} but target was {finalized_count}"
	);

	drop(miner_1);
	drop(grandpa_miner);
}
