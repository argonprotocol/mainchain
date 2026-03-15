use crate::utils::{create_active_notary_with_archive_bucket, wait_for_finalized_catchup};
use argon_client::conversion::SubxtRuntime;
use argon_notary_audit::VerifyError;
use argon_primitives::ArgonDigests;
use argon_testing::{ArgonNodeStartArgs, ArgonTestNode, ArgonTestNotary, test_miner_count};
use serial_test::serial;
use std::time::Duration;

/// Tests importing a block with a notary down
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_import_of_block_with_notary_down() {
	let mut grandpa_miner = ArgonNodeStartArgs::new("alice", 0, "").unwrap();
	let archive_bucket = ArgonTestNotary::create_archive_bucket();
	let archive_host = format!("{}/{}", ArgonTestNotary::get_minio_url(), archive_bucket.clone());
	grandpa_miner.notebook_archive_urls.push(archive_host.clone());

	let grandpa_miner = ArgonTestNode::start(grandpa_miner).await.unwrap();
	let miner_threads = test_miner_count();
	let _miner_1 = grandpa_miner.fork_node("bob", miner_threads).await.unwrap();

	let mut test_notary = create_active_notary_with_archive_bucket(&grandpa_miner, archive_bucket)
		.await
		.expect("Notary registered");

	let mut blocks_before_notebooks = 0;
	let mut block_hash = None;
	{
		let mut blocks_sub =
			grandpa_miner.client.live.blocks().subscribe_finalized().await.unwrap();

		while let Some(Ok(block)) = blocks_sub.next().await {
			if let Some(notebooks) = block
				.header()
				.runtime_digest()
				.logs
				.iter()
				.find_map(|a| a.as_notebooks::<VerifyError>())
			{
				if !notebooks.notebooks.is_empty() {
					println!("Got block with notebooks - ({}) {}", block.number(), block.hash());
					block_hash = Some(block.hash());
					break;
				}
			}
			blocks_before_notebooks += 1;
			if blocks_before_notebooks > 20 {
				panic!("No block with notebooks found");
			}
		}
	}

	let block_hash = block_hash.expect("No block with notebooks found");

	test_notary.stop();

	let miner_2 = grandpa_miner.fork_node("dave", miner_threads).await.unwrap();

	println!("Restarting notary");
	test_notary.restart().await.unwrap();
	println!("Notary restarted");

	let mut counter = 0;
	{
		let mut blocks_sub = miner_2.client.live.blocks().subscribe_finalized().await.unwrap();
		while let Some(Ok(block)) = blocks_sub.next().await {
			if let Some(notebooks) = block
				.header()
				.runtime_digest()
				.logs
				.iter()
				.find_map(|a| a.as_notebooks::<VerifyError>())
			{
				if !notebooks.notebooks.is_empty() {
					println!("Miner 2 got a block with notebooks");
				}
				if block.hash() == block_hash {
					println!("Miner 2 got the block initially rejected with notebooks");
					break;
				}
				counter += 1;
				if counter > 20 {
					panic!("Didn't get back on same chain");
				}
			}
		}
	}
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_late_node_catches_up_to_existing_notebook_history() {
	let mut grandpa_miner = ArgonNodeStartArgs::new("alice", 0, "").unwrap();
	let archive_bucket = ArgonTestNotary::create_archive_bucket();
	let archive_host = format!("{}/{}", ArgonTestNotary::get_minio_url(), archive_bucket.clone());
	grandpa_miner.notebook_archive_urls.push(archive_host);

	let grandpa_miner = ArgonTestNode::start(grandpa_miner).await.unwrap();
	let miner_threads = test_miner_count();
	let _miner_1 = grandpa_miner.fork_node("bob", miner_threads).await.unwrap();
	let _test_notary = create_active_notary_with_archive_bucket(&grandpa_miner, archive_bucket)
		.await
		.expect("Notary registered");

	let notebook_blocks = wait_for_finalized_notebook_blocks(&grandpa_miner, 12, 80).await;
	let (target_number, target_hash) = *notebook_blocks.last().expect("notebook block exists");

	let late_node = grandpa_miner.fork_node("dave", 0).await.unwrap();
	wait_for_finalized_catchup(&grandpa_miner, &late_node).await.unwrap();

	assert_eq!(
		late_node.client.block_at_height(target_number).await.unwrap(),
		Some(target_hash),
		"late node should finalize the same notebook-heavy history",
	);
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_late_node_recovers_after_restart_during_notary_outage() {
	let mut grandpa_miner = ArgonNodeStartArgs::new("alice", 0, "").unwrap();
	let archive_bucket = ArgonTestNotary::create_archive_bucket();
	let archive_host = format!("{}/{}", ArgonTestNotary::get_minio_url(), archive_bucket.clone());
	grandpa_miner.notebook_archive_urls.push(archive_host);

	let grandpa_miner = ArgonTestNode::start(grandpa_miner).await.unwrap();
	let miner_threads = test_miner_count();
	let _miner_1 = grandpa_miner.fork_node("bob", miner_threads).await.unwrap();
	let mut test_notary = create_active_notary_with_archive_bucket(&grandpa_miner, archive_bucket)
		.await
		.expect("Notary registered");

	let notebook_blocks = wait_for_finalized_notebook_blocks(&grandpa_miner, 8, 60).await;
	let (target_number, target_hash) = *notebook_blocks.last().expect("notebook block exists");

	test_notary.stop();

	let mut late_node = grandpa_miner.fork_node("dave", 0).await.unwrap();
	tokio::time::sleep(Duration::from_secs(3)).await;
	late_node.restart(Duration::from_secs(1)).await.unwrap();

	test_notary.restart().await.unwrap();
	wait_for_finalized_catchup(&grandpa_miner, &late_node).await.unwrap();

	assert_eq!(
		late_node.client.block_at_height(target_number).await.unwrap(),
		Some(target_hash),
		"late node should recover the stalled notebook history after restart",
	);
}

async fn wait_for_finalized_notebook_blocks(
	node: &ArgonTestNode,
	min_notebook_blocks: usize,
	max_finalized_blocks: usize,
) -> Vec<(u32, subxt::utils::H256)> {
	let start_finalized = node.client.latest_finalized_block().await.unwrap();
	let mut blocks_sub = node.client.live.blocks().subscribe_finalized().await.unwrap();
	let mut notebook_blocks = Vec::new();
	let mut finalized_seen = 0usize;

	while let Some(Ok(block)) = blocks_sub.next().await {
		if block.number() <= start_finalized {
			continue;
		}
		finalized_seen += 1;

		if let Some(notebooks) = block
			.header()
			.runtime_digest()
			.logs
			.iter()
			.find_map(|digest| digest.as_notebooks::<VerifyError>())
		{
			if !notebooks.notebooks.is_empty() {
				notebook_blocks.push((block.number(), block.hash()));
				if notebook_blocks.len() >= min_notebook_blocks {
					return notebook_blocks;
				}
			}
		}

		assert!(
			finalized_seen < max_finalized_blocks,
			"only saw {} notebook block(s) after {} finalized blocks",
			notebook_blocks.len(),
			finalized_seen,
		);
	}

	panic!("finalized block subscription ended before enough notebook blocks were observed");
}
