use crate::utils::{bankroll_miners, create_active_notary, register_miner};
use argon_client::{api, conversion::SubxtRuntime, signer::Sr25519Signer};
use argon_primitives::{AccountId, ArgonDigests};
use argon_testing::{test_miner_count, ArgonTestNode};
use sp_keyring::AccountKeyring::{Alice, Bob, Eve};
use std::collections::HashSet;
use tokio::join;

/// This test should
/// - Create 2 compute nodes
/// - Register a notary
/// - Create 1 localchain voter
/// - Create bids for both compute miners and add keys to their keystores
#[tokio::test(flavor = "multi_thread")]
async fn test_end_to_end_vote_mining() {
	let miner_threads = test_miner_count();
	let miner_1 = ArgonTestNode::start("alice", miner_threads, "").await.unwrap();
	let miner_2 = ArgonTestNode::start("bob", miner_threads, &miner_1.boot_url).await.unwrap();

	let test_notary = create_active_notary(&miner_1).await.expect("Notary registered");

	let mut blocks_sub = miner_1.client.live.blocks().subscribe_finalized().await.unwrap();
	let mut authors = HashSet::<AccountId>::new();
	let mut counter = 0;
	loop {
		if let Some(Ok(block)) = blocks_sub.next().await {
			if let Some(author) =
				block.header().runtime_digest().logs.iter().find_map(|a| a.as_author())
			{
				println!("Block Author {:?}", author);
				authors.insert(author);
			}
			if authors.len() == 2 {
				println!("Both authors have produced blocks");
				break;
			}
			counter += 1;
			if counter >= 20 {
				panic!("No blocks produced by authors");
			}
		}
	}

	let alice_signer: Sr25519Signer = Alice.pair().into();
	let vote_miner_1 = Bob;
	let vote_miner_2 = Eve;
	bankroll_miners(
		&miner_1,
		&alice_signer,
		vec![vote_miner_2.to_account_id(), vote_miner_1.to_account_id()],
		true,
	)
	.await
	.unwrap();

	let mut blocks_sub = miner_1.client.live.blocks().subscribe_finalized().await.unwrap();
	let res = join!(register_miner(&miner_1, vote_miner_2), register_miner(&miner_2, vote_miner_1));
	assert!(res.0.is_ok());
	assert!(res.1.is_ok());
	let mut miner_registrations = 0;
	let mut block_loops = 0;
	loop {
		if let Some(Ok(block)) = blocks_sub.next().await {
			let events = block.events().await.unwrap();
			for event in events.iter().flatten() {
				if let Ok(Some(api::mining_slot::events::NewMiners { new_miners, .. })) =
					event.as_event()
				{
					miner_registrations += new_miners.0.len();
				}
			}
			if miner_registrations == 2 {
				break;
			}
			block_loops += 1;
			if block_loops >= 20 {
				panic!("No miner registrations");
			}
		}
	}
	assert_eq!(miner_registrations, 2);

	drop(miner_1);
	drop(miner_2);
	drop(test_notary);
}
