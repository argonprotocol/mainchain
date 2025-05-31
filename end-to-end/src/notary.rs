use crate::utils::create_active_notary_with_archive_bucket;
use argon_client::{
	api::runtime_types::argon_notary_audit::error::VerifyError, conversion::SubxtRuntime,
};
use argon_primitives::ArgonDigests;
use argon_testing::{ArgonNodeStartArgs, ArgonTestNode, ArgonTestNotary, test_miner_count};
use serial_test::serial;

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
