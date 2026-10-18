use crate::utils::{activate_vote_mining, create_active_notary_with_archive_bucket, sudo};
use argon_client::{
	api::{runtime_types::argon_runtime::RuntimeCall, storage},
	conversion::SubxtRuntime,
	signer::{Signer, Sr25519Signer},
	FetchAt,
};
use argon_primitives::{AccountId, ArgonDigests, BlockSealDigest};
use argon_testing::{test_miner_count, ArgonNodeStartArgs, ArgonTestNode, ArgonTestNotary};
use polkadot_sdk::*;
use serial_test::serial;
use sp_core::{DeriveJunction, Pair};
use std::{fs, process::Command, time::Duration};

/// Tests default votes submitted by a notebook after nodes register as vote miners
#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_end_to_end_default_vote_mining() {
	let is_ci = std::env::var("CI").is_ok();
	let compute_threads = test_miner_count();
	let mut grandpa_miner = ArgonNodeStartArgs::new("alice", compute_threads, "").unwrap();

	if is_ci {
		// Voting needs recent tick history. Leave time for cache-only compute and notebook
		// propagation without flooding the nodes with compute blocks.
		let node_binary = grandpa_miner.target_dir.join("argon-node");
		let spec_output =
			Command::new(&node_binary).args(["build-spec", "--chain=dev"]).output().unwrap();
		assert!(
			spec_output.status.success(),
			"build-spec failed: {}",
			String::from_utf8_lossy(&spec_output.stderr)
		);
		let mut spec: serde_json::Value = serde_json::from_slice(&spec_output.stdout).unwrap();
		spec["genesis"]["runtimeGenesis"]["patch"]["ticks"]["ticker"]["tickDurationMillis"] =
			6_000.into();

		let spec_path = grandpa_miner.base_data_path.join("voting-chain.json");
		fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).unwrap();

		let raw_spec_output = Command::new(&node_binary)
			.args(["build-spec", "--raw", "--chain"])
			.arg(&spec_path)
			.output()
			.unwrap();
		assert!(
			raw_spec_output.status.success(),
			"raw build-spec failed: {}",
			String::from_utf8_lossy(&raw_spec_output.stderr)
		);
		fs::write(&spec_path, raw_spec_output.stdout).unwrap();
		grandpa_miner.chain = spec_path.to_string_lossy().into_owned();
	}

	let archive_bucket = ArgonTestNotary::create_archive_bucket();
	let archive_host = format!("{}/{}", ArgonTestNotary::get_minio_url(), archive_bucket.clone());
	grandpa_miner.notebook_archive_urls.push(archive_host);

	let grandpa_miner = ArgonTestNode::start(grandpa_miner).await.unwrap();
	let miner_1 = grandpa_miner.fork_node("bob", 0).await.unwrap();
	let miner_2 = grandpa_miner.fork_node("dave", 0).await.unwrap();

	let test_notary = create_active_notary_with_archive_bucket(&grandpa_miner, archive_bucket)
		.await
		.expect("Notary registered");

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

	activate_vote_mining(&grandpa_miner, &miner_1, &miner_2).await.unwrap();

	if is_ci {
		// Difficulty stops adapting once miners register. Use the normal CI minimum for
		// fallback mining so a high bootstrap difficulty cannot starve voting of tick history.
		sudo(
			&grandpa_miner,
			RuntimeCall::BlockSealSpec(
				argon_client::api::runtime_types::pallet_block_seal_spec::pallet::Call::configure {
					vote_minimum: None,
					compute_difficulty: Some(25),
				},
			),
			true,
		)
		.await
		.unwrap();
	}

	// Ensure registrations are visible in finalized state before counting vote blocks.
	let mut finalized_wait =
		grandpa_miner.client.live.blocks().subscribe_finalized().await.unwrap();
	let lookup_1 = storage()
		.mining_slot()
		.account_index_lookup(grandpa_miner.client.api_account(&miner_1.account_id.clone()));
	let lookup_1b = storage()
		.mining_slot()
		.account_index_lookup(grandpa_miner.client.api_account(&miner_1_second_account.clone()));
	let lookup_2 = storage()
		.mining_slot()
		.account_index_lookup(grandpa_miner.client.api_account(&miner_2.account_id.clone()));
	let mut finalized_blocks = 0;
	loop {
		if let Some(Ok(block)) = finalized_wait.next().await {
			let fetch_at = FetchAt::Block(block.hash());
			let one = grandpa_miner
				.client
				.fetch_storage(&lookup_1, fetch_at)
				.await
				.expect("Fetch miner 1 registration");
			let one_b = grandpa_miner
				.client
				.fetch_storage(&lookup_1b, fetch_at)
				.await
				.expect("Fetch miner 1 secondary registration");
			let two = grandpa_miner
				.client
				.fetch_storage(&lookup_2, fetch_at)
				.await
				.expect("Fetch miner 2 registration");
			if one.is_some() && one_b.is_some() && two.is_some() {
				break;
			}
			finalized_blocks += 1;
			if finalized_blocks >= 120 {
				panic!("Miners not visible in finalized state after 120 blocks");
			}
		}
	}

	// Start a fresh finalized stream after registrations are confirmed.
	let mut blocks_sub = grandpa_miner.client.live.blocks().subscribe_finalized().await.unwrap();
	let start_finalized_block = grandpa_miner.client.latest_finalized_block().await.unwrap();
	let mut vote_blocks = 0;
	let mut miner_vote_blocks = (0, 0, 0);

	let mut block_loops = 0;
	let mut start_tick = None;
	let mut last_tick = None;
	// Preserve the original voting budget despite the longer ticks, including stalled finality.
	let max_wait_seconds = if is_ci { 180 } else { 60 };
	let deadline = tokio::time::Instant::now() + Duration::from_secs(max_wait_seconds);
	while let Ok(Some(Ok(block))) = tokio::time::timeout_at(deadline, blocks_sub.next()).await {
		// Ignore finalized backlog before this phase started.
		if block.number() <= start_finalized_block {
			continue;
		}

		let mut author = None;
		let mut block_seal = None;
		let mut tick = None;
		for digest in block.header().runtime_digest().logs.iter() {
			if let Some(seal) = digest.as_block_seal() {
				block_seal = Some(seal.clone());
			}
			if let Some(a) = digest.as_author::<AccountId>() {
				author = Some(a);
			}
			if let Some(t) = digest.as_tick() {
				tick = Some(t.0);
			}
		}
		if let Some(tick) = tick {
			last_tick = Some(tick);
			if start_tick.is_none() {
				start_tick = Some(tick);
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
		println!("Block Loops: {block_loops}");
		if block_loops >= 300 {
			break;
		}
	}

	println!(
		"Vote Blocks: {}. Miner 1 ({}), Miner 1, key 2 ({}) Miner 2 ({}). Ticks: start {:?}, last {:?}. Finalized start #{}. Wait {} seconds",
		vote_blocks,
		miner_vote_blocks.0,
		miner_vote_blocks.2,
		miner_vote_blocks.1,
		start_tick,
		last_tick,
		start_finalized_block,
		max_wait_seconds
	);
	assert!(vote_blocks >= 2);
	assert!(miner_vote_blocks.0 >= 1);
	assert!(miner_vote_blocks.1 >= 1);
	assert!(miner_vote_blocks.2 >= 1);

	drop(miner_1);
	drop(miner_2);
	drop(test_notary);
}
