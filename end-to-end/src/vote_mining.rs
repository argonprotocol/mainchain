use crate::utils::{
	create_active_notary, mining_slot_ownership_needed, register_miner_keys, register_miners,
};
use argon_client::{
	conversion::SubxtRuntime,
	signer::{Signer, Sr25519Signer},
	FetchAt,
};
use argon_primitives::{AccountId, ArgonDigests, BlockSealDigest};
use argon_testing::{test_miner_count, ArgonTestNode};
use polkadot_sdk::*;
use serial_test::serial;
use sp_core::{DeriveJunction, Pair};
use sp_keyring::Sr25519Keyring;
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
	let ownership_needed = mining_slot_ownership_needed(&grandpa_miner).await.unwrap();
	let mut authors = HashSet::<AccountId>::new();
	let mut counter = 0;
	let mut rewards_counter = 0;
	loop {
		if let Some(Ok(block)) = blocks_sub.next().await {
			if let Some(author) =
				block.header().runtime_digest().logs.iter().find_map(|a| a.as_author())
			{
				let keyring = Sr25519Keyring::from_account_id(&author).unwrap();
				println!("Block Author {:?} ({})", keyring, block.number());

				if let Ok(ownership) =
					grandpa_miner.client.get_ownership(&author, FetchAt::Block(block.hash())).await
				{
					if ownership.free >= (ownership_needed * 2) && !authors.contains(&author) {
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
				panic!("Blocks not produced by both authors after 50 blocks -> {:?}", authors);
			}
		}
	}

	let miner_1_keyring = miner_1.keyring();
	let miner_1_second_account = miner_1
		.keyring()
		.pair()
		.clone()
		.derive(vec![DeriveJunction::hard(1)].into_iter(), None)
		.unwrap()
		.0;
	let miner_1_second_signer = Sr25519Signer::new(miner_1_second_account);
	let miner_1_second_account = miner_1_second_signer.account_id();

	let miner_2_keyring = miner_2.keyring();

	let (keys1, keys_1_2, keys2) = join!(
		register_miner_keys(&miner_1, miner_1_keyring, 1),
		register_miner_keys(&miner_1, miner_1_keyring, 2),
		register_miner_keys(&miner_2, miner_2_keyring, 1)
	);
	let mut blocks_sub = grandpa_miner.client.live.blocks().subscribe_finalized().await.unwrap();
	let (miner2_res, miner1_res) = join!(
		register_miners(
			&miner_2,
			miner_2_keyring.pair().into(),
			vec![(miner_2.account_id.clone(), keys2.unwrap())]
		),
		register_miners(
			&miner_1,
			miner_1_keyring.pair().into(),
			vec![
				(miner_1.account_id.clone(), keys1.unwrap()),
				(miner_1_second_account.clone(), keys_1_2.unwrap())
			],
		),
	);
	miner2_res.unwrap();
	miner1_res.unwrap();

	let mut vote_blocks = 0;
	let mut miner_vote_blocks = (0, 0, 0);
	authors.clear();

	let mut block_loops = 0;
	while let Some(Ok(block)) = blocks_sub.next().await {
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
			} else if author == miner_1_second_account {
				miner_vote_blocks.2 += 1;
			}
			vote_blocks += 1;
			if miner_vote_blocks.0 >= 1 && miner_vote_blocks.1 >= 1 && miner_vote_blocks.2 >= 1 {
				break;
			}
		}
		block_loops += 1;
		println!("Block Loops: {}", block_loops);
		if block_loops >= 40 {
			break;
		}
	}

	println!(
		"Vote Blocks: {}. Miner 1 ({}), Miner 1, key 2 ({}) Miner 2 ({})",
		vote_blocks, miner_vote_blocks.0, miner_vote_blocks.2, miner_vote_blocks.1
	);
	assert!(vote_blocks >= 2);
	assert!(miner_vote_blocks.0 >= 1);
	assert!(miner_vote_blocks.1 >= 1);
	assert!(miner_vote_blocks.2 >= 1);

	drop(miner_1);
	drop(miner_2);
	drop(test_notary);
}
