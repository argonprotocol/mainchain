use crate::utils::{create_active_notary, register_miner, register_miner_keys};
use argon_client::{api, conversion::SubxtRuntime};
use argon_primitives::{AccountId, ArgonDigests, BlockSealDigest};
use argon_testing::{test_miner_count, ArgonTestNode};
use serial_test::serial;
use sp_keyring::AccountKeyring;
use std::{collections::HashSet, env};
use tokio::join;

/// Tests default votes submitted by a notebook after nodes register as vote miners
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_end_to_end_default_vote_mining() {
	env::set_var("RUST_LOG", "info");
	let grandpa_miner = ArgonTestNode::start_with_args("alice", 0).await.unwrap();
	let miner_threads = test_miner_count();
	let miner_1 = grandpa_miner.fork_node("bob", miner_threads).await.unwrap();
	let miner_2 = grandpa_miner.fork_node("dave", miner_threads).await.unwrap();

	let test_notary = create_active_notary(&grandpa_miner).await.expect("Notary registered");

	let mut blocks_sub = grandpa_miner.client.live.blocks().subscribe_finalized().await.unwrap();
	let mut authors = HashSet::<AccountId>::new();
	let mut counter = 0;
	let mut rewards_counter = 0;
	loop {
		if let Some(Ok(block)) = blocks_sub.next().await {
			if let Some(author) =
				block.header().runtime_digest().logs.iter().find_map(|a| a.as_author())
			{
				let keyring = AccountKeyring::from_account_id(&author).unwrap();
				println!("Block Author {:?}", keyring);

				if let Ok(ownership) =
					grandpa_miner.client.get_ownership(&author, Some(block.hash())).await
				{
					if ownership.free >= 500_000 && !authors.contains(&author) {
						println!("Block Author is ready {:?}", keyring);
						authors.insert(author);
					}
				}
			}
			if authors.len() == 2 {
				println!("Both authors have produced blocks");
				if rewards_counter == 0 {
					rewards_counter = counter + 5;
				}
				if counter >= rewards_counter {
					println!("Both authors have rewards matured");
					break;
				}
			}
			counter += 1;
			if counter >= 50 {
				panic!("Blocks not produced by both authors after 30 blocks -> {:?}", authors);
			}
		}
	}

	let miner_1_keyring = miner_1.keyring();
	let miner_2_keyring = miner_2.keyring();

	let mut blocks_sub = grandpa_miner.client.live.blocks().subscribe_finalized().await.unwrap();
	let (keys1, keys2) = join!(
		register_miner_keys(&miner_1, miner_1_keyring),
		register_miner_keys(&miner_2, miner_2_keyring)
	);
	let (miner2_res, miner1_res) = join!(
		register_miner(&miner_2, miner_2_keyring, keys2.unwrap()),
		register_miner(&miner_1, miner_1_keyring, keys1.unwrap())
	);
	miner2_res.unwrap();
	miner1_res.unwrap();
	let mut miner_registrations = 0;
	let mut block_loops = 0;
	let mut vote_blocks = 0;
	let mut miner_vote_blocks = (0, 0);
	authors.clear();
	loop {
		if let Some(Ok(block)) = blocks_sub.next().await {
			let events = block.events().await.unwrap();
			for event in events.iter().flatten() {
				if let Ok(Some(api::mining_slot::events::NewMiners { new_miners, start_index })) =
					event.as_event()
				{
					println!("New Miners at index: {:?} {}", new_miners.0, start_index);
					miner_registrations += new_miners.0.len();
					// once we've seen both, reset the counter in case they're in different blocks
					if miner_registrations == 2 {
						block_loops = 0;
					}
				}
			}
			let mut author = None;
			let mut block_seal = None;
			for digest in block.header().runtime_digest().logs.iter() {
				if let Some(seal) = digest.as_block_seal() {
					block_seal = Some(seal.clone());
				}
				if let Some(a) = digest.as_author::<AccountId>() {
					author = Some(a);
				}
			}
			if let (Some(author), Some(BlockSealDigest::Vote { .. })) = (author, block_seal) {
				if author == miner_1_keyring.to_account_id() {
					miner_vote_blocks.0 += 1;
				} else if author == miner_2_keyring.to_account_id() {
					miner_vote_blocks.1 += 1;
				}
				vote_blocks += 1;
				if miner_vote_blocks.0 >= 1 && miner_vote_blocks.1 >= 1 {
					break;
				}
			}
			block_loops += 1;
			println!("Block Loops: {}", block_loops);
			if block_loops >= 20 {
				break;
			}
		}
	}
	assert_eq!(miner_registrations, 2);
	assert!(vote_blocks >= 2);
	assert!(miner_vote_blocks.0 >= 1);
	assert!(miner_vote_blocks.1 >= 1);
	println!(
		"Vote Blocks: {}. Miner 1 ({}), Miner 2 ({})",
		vote_blocks, miner_vote_blocks.0, miner_vote_blocks.1
	);

	drop(miner_1);
	drop(miner_2);
	drop(test_notary);
}
