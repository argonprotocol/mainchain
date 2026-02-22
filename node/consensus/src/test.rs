use crate::{
	NotebookTickChecker,
	mock_importer::{
		create_params, has_state, new_importer, new_importer_from_client, new_importer_with_notary,
		new_importer_with_notary_from_client, pending_import_count,
	},
	pending_import_replay::MAX_PENDING_IMPORTS,
};
use argon_primitives::{
	NotebookAuditResult,
	prelude::{sp_core::U256, sp_runtime::Permill, *},
	tick::Ticker,
};
use argon_runtime::{Block, Header};
use codec::{Decode, Encode};
use polkadot_sdk::{
	frame_support::assert_ok, sc_client_api::KeyValueStates, sc_consensus::ImportedState,
	sp_core::H256,
};
use sc_consensus::{BlockImport, ImportResult, StateAction, StorageChanges};
use sc_consensus_grandpa::{FinalityProof, GrandpaJustification};
use sp_blockchain::{BlockStatus, HeaderBackend};
use sp_consensus::BlockOrigin;
use sp_runtime::RuntimeAppPublic;
use std::{collections::HashSet, time::Duration};
use tokio::time::Instant;

#[test]
fn decode_finality() {
	// First block in mainnet on 105 is 17572. Version deployed on
	// set id 1 at 17574
	// set id 0
	let encoded_17573 = hex::decode("7927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712e9062d560600000000007927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712a54400000c7927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712a54400005c9a28ed3f1a5bb94bd8780e9ad3640edd55652dd516fd303d539c64b25572de7b31a99ad3e0e8d636a5615978b107da853b3d9e2360fcbf2e3b1542e77fce0a45a74d33ead0b5ff58607fc60556cf1b291d4c503254ae07f17b3d54f8c5c27f7927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712a5440000b5c9b05beb4413ed4565492339a1735ff25f419d100d6cea8e5c7947f3ffb9e6079c034c23720f4cbb6151e260b2bd5fedfd3667589c338f5271787b27f08b0c803c5c3c4059380a8603f785a093c227a8a2f4a7437c466f1f7233a6881400e67927b62bef2a417d0affc650f9a3cd2e3ef69a27cbd7ba14691774b0ea2cd712a54400009230a62facc51e07b1210eb52d7257d12257c66106aa723439019070b647efc9d25e7d58dde8cc2e4a8abec11601895c069ad09ad98e77e9af76aa2fb72e560f962abf1be4e94bb80e6488a2af551c529571fdd1d972b5c7e311d7507f0882ec0000").unwrap();

	let finality_proof = FinalityProof::<Header>::decode(&mut &encoded_17573[..]).unwrap();

	let justification =
		GrandpaJustification::<Block>::decode(&mut &finality_proof.justification[..]).unwrap();

	for signed in justification.justification.commit.precommits.iter() {
		let message = finality_grandpa::Message::Precommit(signed.precommit.clone());
		println!("Message: {signed:#?}");

		for i in 0..10u64 {
			let buf = (message.clone(), justification.justification.round, i).encode();
			if signed.id.verify(&buf, &signed.signature) {
				println!("Signature verified at {i}");
				assert_eq!(i, 0);
			}
		}
	}
}

#[test]
fn test_notebook_tick_checker() {
	let mut checker = NotebookTickChecker::new();
	let tick_1 = 1;
	let tick_2 = 2;
	let now = Instant::now();
	checker.add(tick_1, now + Duration::from_secs(10));
	checker.add(tick_2, now - Duration::from_secs(5));

	assert_eq!(checker.get_ready(), [tick_2].into_iter().collect::<HashSet<_>>());
	assert_eq!(checker.get_next_check_delay().unwrap().as_secs(), 9);

	assert_eq!(checker.ticks_to_recheck.len(), 1);
}

#[test]
fn test_notebook_tick_checker_should_delay_block_attempt() {
	let ticker = Ticker::start(Duration::from_secs(2), 2);
	let miner_nonce_score = Some((U256::from(100), Permill::from_percent(50)));
	let now = Instant::now();
	// we can't guarantee when this will run, so we just check it if it does
	if let Some(delay) = NotebookTickChecker::should_delay_block_attempt(
		ticker.current(),
		&ticker,
		miner_nonce_score,
	) {
		assert_eq!(delay.duration_since(now).as_secs(), 1);
	}
}

#[tokio::test]
async fn gap_header_not_best() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	let params =
		create_params(1, parent, 1, None, BlockOrigin::NetworkInitialSync, StateAction::Skip, None);

	let res = importer.import_block(params).await.unwrap();
	assert!(matches!(res, ImportResult::Imported(_)));
	assert_eq!(client.info().best_number, 0u32);
}

#[tokio::test]
async fn missing_parent_state_returns_missing_state_for_execute_if_possible() {
	let (importer, _client) = new_importer();
	let unknown_parent = H256::repeat_byte(1);
	let mut params = create_params(
		1,
		unknown_parent,
		1,
		None,
		BlockOrigin::NetworkBroadcast,
		StateAction::ExecuteIfPossible,
		None,
	);
	params.body = Some(Vec::new());

	let result = importer.import_block(params).await.unwrap();
	assert!(matches!(result, ImportResult::MissingState));
}

#[tokio::test]
async fn execute_if_possible_sync_block_with_pruned_parent_is_deferred_header_only() {
	let (importer, client) = new_importer();
	let genesis_hash = client.info().best_hash;

	let parent = create_params(
		1,
		genesis_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Skip,
		None,
	);
	let parent_hash = parent.post_hash();
	let _ = importer.import_block(parent).await.unwrap();
	assert!(!has_state(&client, parent_hash));

	let mut child = create_params(
		2,
		parent_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::ExecuteIfPossible,
		None,
	);
	child.body = Some(Vec::new());
	let child_hash = child.post_hash();

	let result = importer.import_block(child).await.unwrap();
	assert!(matches!(result, ImportResult::Imported(_)));
	assert_eq!(pending_import_count(&importer).await, 1);
	assert_eq!(client.status(child_hash).unwrap(), BlockStatus::InChain);
	assert!(!has_state(&client, child_hash));
}

#[tokio::test]
async fn deferred_execute_if_possible_recovers_after_importer_restart() {
	let (importer, client) = new_importer();
	let genesis_hash = client.info().best_hash;

	let parent = create_params(
		1,
		genesis_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Skip,
		None,
	);
	let parent_hash = parent.post_hash();
	let _ = importer.import_block(parent).await.unwrap();
	assert!(!has_state(&client, parent_hash));

	let mut child = create_params(
		2,
		parent_hash,
		1,
		None,
		BlockOrigin::NetworkBroadcast,
		StateAction::ExecuteIfPossible,
		None,
	);
	child.body = Some(Vec::new());
	let child_hash = child.post_hash();
	let result = importer.import_block(child).await.unwrap();
	assert!(matches!(result, ImportResult::Imported(_)));
	assert_eq!(pending_import_count(&importer).await, 1);
	drop(importer);

	let importer = new_importer_from_client(client.clone());
	assert_eq!(
		pending_import_count(&importer).await,
		1,
		"deferred queue should survive importer restart",
	);

	client.set_state(parent_hash, sp_consensus::BlockStatus::InChainWithState);
	importer.replay_pending_full_imports().await;

	assert!(has_state(&client, child_hash), "replayed import should recover full block state");
	assert_eq!(pending_import_count(&importer).await, 0);
}

#[tokio::test]
async fn queue_full_imports_header_only_without_peer_retry_signal() {
	let (importer, client) = new_importer();
	let genesis_hash = client.info().best_hash;

	let parent = create_params(
		1,
		genesis_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Skip,
		None,
	);
	let parent_hash = parent.post_hash();
	let _ = importer.import_block(parent).await.unwrap();
	assert!(!has_state(&client, parent_hash));

	for n in 2..=(MAX_PENDING_IMPORTS as u32 + 1) {
		let mut child = create_params(
			n,
			parent_hash,
			1,
			None,
			BlockOrigin::NetworkInitialSync,
			StateAction::ExecuteIfPossible,
			None,
		);
		child.body = Some(Vec::new());
		let result = importer.import_block(child).await.unwrap();
		assert!(matches!(result, ImportResult::Imported(_)));
	}
	assert_eq!(pending_import_count(&importer).await, MAX_PENDING_IMPORTS);

	let mut overflow = create_params(
		MAX_PENDING_IMPORTS as u32 + 2,
		parent_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::ExecuteIfPossible,
		None,
	);
	overflow.body = Some(Vec::new());
	let overflow_hash = overflow.post_hash();
	let overflow_result = importer.import_block(overflow).await.unwrap();
	assert!(matches!(overflow_result, ImportResult::Imported(_)));
	assert_eq!(pending_import_count(&importer).await, MAX_PENDING_IMPORTS);
	assert_eq!(client.status(overflow_hash).unwrap(), BlockStatus::InChain);
	assert!(!has_state(&client, overflow_hash));
}

#[tokio::test]
async fn defers_notebook_verification_and_replays_full_import() {
	let (importer, client) = new_importer_with_notary();
	let parent = client.info().best_hash;
	client.set_runtime_notebooks(
		parent,
		vec![NotebookAuditResult {
			notary_id: 1,
			notebook_number: 1,
			tick: 1,
			audit_first_failure: None,
		}],
	);

	let mut params = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::NetworkBroadcast,
		StateAction::Execute,
		None,
	);
	params.body = Some(Vec::new());
	let block_hash = params.post_hash();

	let start = std::time::Instant::now();
	let first_result = importer.import_block(params).await.unwrap();
	let elapsed = start.elapsed();
	assert!(matches!(first_result, ImportResult::Imported(_)));
	assert!(
		elapsed >= std::time::Duration::from_secs(2),
		"expected notebook defer timeout path to run, got {elapsed:?}",
	);
	assert!(
		elapsed < std::time::Duration::from_secs(5),
		"notebook defer path should fail fast to avoid long import lock stalls, got {elapsed:?}"
	);
	assert!(!has_state(&client, block_hash), "initial import should be header-only");
	assert_eq!(
		client.status(block_hash).unwrap(),
		BlockStatus::InChain,
		"initial import should store a header-only placeholder",
	);
	assert_eq!(pending_import_count(&importer).await, 1);

	client.set_runtime_notebooks(parent, Vec::new());

	importer.replay_pending_full_imports().await;

	assert!(has_state(&client, block_hash), "pending import replay should apply full state");
	assert_eq!(pending_import_count(&importer).await, 0);
}

#[tokio::test]
async fn deferred_notebook_import_recovers_after_importer_restart() {
	let (importer, client) = new_importer_with_notary();
	let parent = client.info().best_hash;
	client.set_runtime_notebooks(
		parent,
		vec![NotebookAuditResult {
			notary_id: 1,
			notebook_number: 1,
			tick: 1,
			audit_first_failure: None,
		}],
	);

	let mut params = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::NetworkBroadcast,
		StateAction::Execute,
		None,
	);
	params.body = Some(Vec::new());
	let block_hash = params.post_hash();

	let first_result = importer.import_block(params).await.unwrap();
	assert!(matches!(first_result, ImportResult::Imported(_)));
	assert!(!has_state(&client, block_hash), "initial import should be header-only");
	assert_eq!(pending_import_count(&importer).await, 1);
	drop(importer);

	let importer = new_importer_with_notary_from_client(client.clone());
	assert_eq!(
		pending_import_count(&importer).await,
		1,
		"deferred queue should survive importer restart",
	);

	client.set_runtime_notebooks(parent, Vec::new());
	importer.replay_pending_full_imports().await;

	assert!(has_state(&client, block_hash), "replayed import should recover full block state");
	assert_eq!(pending_import_count(&importer).await, 0);
}

#[tokio::test]
async fn deferred_notebook_import_stays_pending_when_notary_is_unavailable() {
	let (importer, client) = new_importer_with_notary();
	let parent = client.info().best_hash;
	client.set_runtime_notebooks(
		parent,
		vec![NotebookAuditResult {
			notary_id: 1,
			notebook_number: 1,
			tick: 1,
			audit_first_failure: None,
		}],
	);

	let mut params = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::NetworkBroadcast,
		StateAction::Execute,
		None,
	);
	params.body = Some(Vec::new());
	let block_hash = params.post_hash();

	let first_result = importer.import_block(params).await.unwrap();
	assert!(matches!(first_result, ImportResult::Imported(_)));
	assert_eq!(pending_import_count(&importer).await, 1);
	importer.replay_pending_full_imports().await;

	assert_eq!(
		pending_import_count(&importer).await,
		1,
		"replay should keep deferred full import queued while notebook audit is unavailable",
	);
	assert!(!has_state(&client, block_hash));
}

#[tokio::test]
async fn replay_scans_past_unready_entry_and_imports_ready_entry() {
	let (importer, client) = new_importer_with_notary();
	let genesis_hash = client.info().best_hash;

	let parent = create_params(
		1,
		genesis_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Skip,
		None,
	);
	let parent_hash = parent.post_hash();
	let _ = importer.import_block(parent).await.unwrap();
	assert!(!has_state(&client, parent_hash));

	client.set_runtime_notebooks(
		parent_hash,
		vec![NotebookAuditResult {
			notary_id: 1,
			notebook_number: 1,
			tick: 1,
			audit_first_failure: None,
		}],
	);

	let mut blocked = create_params(
		2,
		parent_hash,
		1,
		None,
		BlockOrigin::NetworkBroadcast,
		StateAction::ExecuteIfPossible,
		Some(AccountId::from([1u8; 32])),
	);
	blocked.body = Some(Vec::new());
	let blocked_hash = blocked.post_hash();
	let blocked_result = importer.import_block(blocked).await.unwrap();
	assert!(matches!(blocked_result, ImportResult::Imported(_)));

	let mut ready = create_params(
		2,
		parent_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::ExecuteIfPossible,
		Some(AccountId::from([2u8; 32])),
	);
	ready.body = Some(Vec::new());
	let ready_hash = ready.post_hash();
	let ready_result = importer.import_block(ready).await.unwrap();
	assert!(matches!(ready_result, ImportResult::Imported(_)));

	assert_eq!(pending_import_count(&importer).await, 2);
	client.set_state(parent_hash, sp_consensus::BlockStatus::InChainWithState);

	importer.replay_pending_full_imports().await;

	assert!(!has_state(&client, blocked_hash), "blocked replay should remain queued");
	assert!(has_state(&client, ready_hash), "ready replay should still import");
	assert_eq!(
		pending_import_count(&importer).await,
		1,
		"only the blocked replay should remain queued",
	);
}

#[tokio::test]
async fn imports_with_intermediates_do_not_defer_to_replay_queue() {
	let (importer, client) = new_importer();
	let genesis_hash = client.info().best_hash;

	let parent = create_params(
		1,
		genesis_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Skip,
		None,
	);
	let parent_hash = parent.post_hash();
	let _ = importer.import_block(parent).await.unwrap();
	assert!(!has_state(&client, parent_hash));

	let mut child = create_params(
		2,
		parent_hash,
		1,
		None,
		BlockOrigin::NetworkBroadcast,
		StateAction::ExecuteIfPossible,
		None,
	);
	child.body = Some(Vec::new());
	child.insert_intermediate(b"defer-marker", 1u8);
	let child_hash = child.post_hash();

	let result = importer.import_block(child).await.unwrap();
	assert!(matches!(result, ImportResult::MissingState));
	assert_eq!(pending_import_count(&importer).await, 0);
	assert_eq!(client.status(child_hash).unwrap(), BlockStatus::Unknown);
}

#[tokio::test]
async fn higher_fork_power_sets_best() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;

	// weaker block (power 1)
	let p1 = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Execute,
		None,
	);

	let _ = importer.import_block(p1).await.unwrap();

	// stronger block (power 2)
	let p2 = create_params(
		1,
		parent,
		2,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Execute,
		None,
	);
	let h2 = p2.header.hash();
	let _ = importer.import_block(p2).await.unwrap();

	assert_eq!(client.info().best_hash, h2);
}

#[tokio::test]
async fn header_plus_state_can_be_best() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	let params = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::ApplyChanges(StorageChanges::Import(ImportedState {
			block: H256::zero(),
			state: KeyValueStates(Vec::new()),
		})),
		None,
	);

	let res = importer.import_block(params).await.unwrap();
	// We just care that full import ran; NoopImport returns Imported(...)
	assert!(matches!(res, ImportResult::Imported(_)));
}

#[tokio::test]
async fn state_upgrade_test() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	// header-only import
	let gap =
		create_params(1, parent, 1, None, BlockOrigin::NetworkInitialSync, StateAction::Skip, None);
	let hash = gap.header.hash();
	importer.import_block(gap).await.unwrap();
	assert!(!has_state(&client, hash));

	// now with state
	let state = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::ApplyChanges(StorageChanges::Import(ImportedState {
			block: H256::zero(),
			state: KeyValueStates(Vec::new()),
		})),
		None,
	);
	importer.import_block(state).await.unwrap();
	assert!(has_state(&client, hash));
}

#[tokio::test]
async fn finalized_upgrade_reimports() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	let params1 =
		create_params(1, parent, 1, None, BlockOrigin::NetworkBroadcast, StateAction::Skip, None);
	let _ = importer.import_block(params1).await.unwrap();

	// second import – now marked finalized
	let mut params2 =
		create_params(1, parent, 1, None, BlockOrigin::NetworkBroadcast, StateAction::Skip, None);
	params2.finalized = true;

	let res2 = importer.import_block(params2).await.unwrap();
	assert!(matches!(res2, ImportResult::Imported(_)));
}

#[tokio::test]
async fn justification_upgrade_reimports() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	let params1 =
		create_params(1, parent, 1, None, BlockOrigin::NetworkBroadcast, StateAction::Skip, None);
	let _ = importer.import_block(params1).await.unwrap();

	// second import - now with a justification payload
	let mut params2 =
		create_params(1, parent, 1, None, BlockOrigin::NetworkBroadcast, StateAction::Skip, None);
	params2.justifications = Some(sp_runtime::Justifications::from(([1, 2, 3, 4], vec![1u8])));

	let res2 = importer.import_block(params2).await.unwrap();
	assert!(matches!(res2, ImportResult::Imported(_)));
}

#[tokio::test]
async fn duplicate_header_short_circuits() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	let params =
		create_params(1, parent, 1, None, BlockOrigin::NetworkBroadcast, StateAction::Skip, None);

	// first import
	let _ = importer.import_block(params).await.unwrap();

	// build identical params again (BlockImportParams isn't Clone)
	let params2 =
		create_params(1, parent, 1, None, BlockOrigin::NetworkBroadcast, StateAction::Skip, None);

	let res2 = importer.import_block(params2).await.unwrap();
	assert!(matches!(res2, ImportResult::AlreadyInChain));
}

#[tokio::test]
async fn tie_loser_test() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;

	// loser (hash2 > hash1)
	let loser = create_params(1, parent, 1, None, BlockOrigin::Own, StateAction::Execute, None);
	assert_ok!(importer.import_block(loser).await); // Imported

	// winner (smaller hash)
	let winner = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::Own,
		StateAction::Execute,
		Some(AccountId::from([2u8; 32])),
	);
	let h_win = winner.header.hash();
	assert_ok!(importer.import_block(winner).await); // Imported(best)

	assert_eq!(client.info().best_hash, h_win);

	// replay loser
	let loser2 = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::Own,
		StateAction::Skip,
		Some(AccountId::from([0u8; 32])),
	);
	let res = importer.import_block(loser2).await.unwrap();
	assert!(matches!(res, ImportResult::AlreadyInChain));
	assert_eq!(client.info().best_hash, h_win);
}

#[tokio::test]
async fn duplicate_vote_block_same_tick_fails() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;

	let author = AccountId::from([9u8; 32]);
	let vote_key = H256::random();

	// First vote → ok
	let p1 = create_params(
		1,
		parent,
		0,
		Some(vote_key),
		BlockOrigin::NetworkBroadcast,
		StateAction::Execute,
		Some(author.clone()),
	);
	let p1_hash = p1.post_hash();
	assert!(matches!(importer.import_block(p1).await.unwrap(), ImportResult::Imported(_)));

	// Second vote by same author + same voting_key at same tick ⇒ Err
	let mut p2 = create_params(
		1,
		parent,
		0,
		Some(vote_key),
		BlockOrigin::NetworkBroadcast,
		StateAction::Execute,
		Some(author),
	);
	p2.header.extrinsics_root = H256::random();
	p2.header.state_root = H256::random();
	p2.post_hash = Some(p2.header.hash()); // refresh the cached value
	let p2_hash = p2.post_hash();
	let err = importer.import_block(p2).await;

	assert_ne!(p1_hash, p2_hash, "post hashes should differ");

	assert!(err.is_err(), "duplicate vote block should fail");
}

#[tokio::test]
async fn duplicate_compute_loser_same_power_fails() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;

	// Winner first (smaller hash) so later blocks at same power are losers
	let winner = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::Own,
		StateAction::Execute,
		Some(AccountId::from([1u8; 32])),
	);
	let h_winner = winner.post_hash();
	importer.import_block(winner).await.unwrap();

	// First loser from author X
	let author = AccountId::from([2u8; 32]);
	let mut loser1 = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::Own,
		StateAction::Execute,
		Some(author.clone()),
	);
	loser1.header.extrinsics_root = H256::random();
	loser1.header.state_root = H256::random();
	loser1.post_hash = Some(loser1.header.hash()); // refresh the cached
	assert_ne!(loser1.post_hash(), h_winner, "loser1 should differ from winner");
	importer.import_block(loser1).await.unwrap(); // ok

	// Second loser, same author, same fork-power, same tick ⇒ Err
	let loser2 =
		create_params(1, parent, 1, None, BlockOrigin::Own, StateAction::Execute, Some(author));
	let err = importer.import_block(loser2).await;
	assert!(err.is_err(), "duplicate compute loser should fail");
}

#[tokio::test]
async fn reorg_to_lower_power_then_recover() {
	let (importer, client) = new_importer();
	let genesis = client.info().best_hash;

	// Node-2 fork: height 1..100, power 200.
	let mut parent = genesis;
	let mut hash50 = H256::zero();
	for n in 1..=100 {
		let p = create_params(
			n,
			parent,
			200 + n as u128,
			None,
			BlockOrigin::Own,
			StateAction::Execute,
			None,
		);
		if n == 50 {
			hash50 = p.header.hash();
		}
		parent = p.header.hash();
		importer.import_block(p).await.unwrap();
	}
	assert_eq!(client.info().best_number, 100);

	// Archive node finalises *lower-power* fork at height 51, power 151.
	let p = create_params(51, hash50, 151, None, BlockOrigin::Own, StateAction::Execute, None);
	let back_best = p.header.hash();
	assert_ok!(importer.import_block(p).await);
	client.force_best(51, back_best);

	// Fresh block from node-2 with power 152 must become best.
	let p = create_params(
		52,
		client.info().best_hash,
		152,
		None,
		BlockOrigin::Own,
		StateAction::Execute,
		None,
	);
	let hash130 = p.header.hash();
	importer.import_block(p).await.unwrap();

	assert_eq!(client.info().best_hash, hash130);
}
