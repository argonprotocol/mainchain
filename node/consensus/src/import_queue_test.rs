use crate::mock_importer::{create_params, has_state, new_importer};
use argon_primitives::prelude::*;
use hex_literal::hex;
use polkadot_sdk::{
	frame_support::assert_ok,
	sc_client_api::{BlockBackend, KeyValueStates},
	sc_consensus::ImportedState,
	sp_core::H256,
	sp_runtime::Justifications,
};
use sc_consensus::{BlockImport, ImportResult, StateAction, StorageChanges};
use sp_blockchain::{BlockGap, BlockGapType, HeaderBackend};
use sp_consensus::BlockOrigin;

#[tokio::test]
async fn test_gap_header_not_best() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	let params =
		create_params(1, parent, 1, None, BlockOrigin::NetworkInitialSync, StateAction::Skip, None);

	let res = importer.import_block(params).await.unwrap();
	assert!(matches!(res, ImportResult::Imported(_)));
	assert_eq!(client.info().best_number, 0u32);
}

#[tokio::test]
async fn test_higher_fork_power_sets_best() {
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
async fn test_header_plus_state_can_be_best() {
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
async fn test_state_upgrade() {
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
async fn test_known_initial_sync_authority_change_uses_empty_justification_marker() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	let authority_change_hash =
		H256::from(hex!("f3f9fb2a75a34d87a78984decf2a0432dfab8e08f75cd42cdc7f1c4fbb8a568d"));

	client.force_finalized(17_572, authority_change_hash);

	let mut params = create_params(
		17_572,
		parent,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Skip,
		None,
	);
	params.post_hash = Some(authority_change_hash);

	let result = importer.import_block(params).await.unwrap();
	assert!(matches!(result, ImportResult::Imported(_)));
	assert_eq!(client.last_import_had_empty_justifications(), Some(true));
}

#[tokio::test]
async fn test_finalized_upgrade_reimports() {
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
async fn test_duplicate_header_short_circuits() {
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
async fn test_known_header_with_justification_marker_reimports() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	let params =
		create_params(1, parent, 1, None, BlockOrigin::NetworkBroadcast, StateAction::Skip, None);

	let _ = importer.import_block(params).await.unwrap();

	let mut params2 =
		create_params(1, parent, 1, None, BlockOrigin::NetworkBroadcast, StateAction::Skip, None);
	params2.justifications = Some(Justifications::default());

	let res2 = importer.import_block(params2).await.unwrap();
	assert!(matches!(res2, ImportResult::Imported(_)));
	assert_eq!(client.info().best_number, 0u32);
}

#[tokio::test]
async fn test_block_gap_reimport_does_not_short_circuit_known_header() {
	let (importer, client) = new_importer();
	let parent = client.info().best_hash;
	let params =
		create_params(1, parent, 1, None, BlockOrigin::NetworkInitialSync, StateAction::Skip, None);
	let block_hash = params.header.hash();

	let _ = importer.import_block(params).await.unwrap();
	assert_eq!(client.block_status(block_hash).unwrap(), sp_consensus::BlockStatus::InChainPruned);

	client.set_block_gap(Some(BlockGap { start: 1, end: 1, gap_type: BlockGapType::MissingBody }));

	let mut params = create_params(
		1,
		parent,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::ExecuteIfPossible,
		None,
	);
	params.body = Some(Vec::new());
	let result = importer.import_block(params).await.unwrap();
	assert!(matches!(result, ImportResult::Imported(_)));
}

#[tokio::test]
async fn test_execute_if_possible_allows_genesis_parent_when_pruned() {
	let (importer, client) = new_importer();
	let genesis_hash = client.info().genesis_hash;
	client.set_state(genesis_hash, sp_consensus::BlockStatus::InChainPruned);

	let mut params = create_params(
		1,
		genesis_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::ExecuteIfPossible,
		None,
	);
	params.body = Some(Vec::new());
	let result = importer.import_block(params).await.unwrap();
	assert!(!matches!(result, ImportResult::MissingState));
}

#[tokio::test]
async fn test_execute_if_possible_does_not_missing_state_in_network_initial_sync() {
	let (importer, client) = new_importer();
	let genesis_hash = client.info().genesis_hash;

	let block_1 = create_params(
		1,
		genesis_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Skip,
		None,
	);
	let block_1_hash = block_1.header.hash();
	let _ = importer.import_block(block_1).await.unwrap();

	let mut block_2 = create_params(
		2,
		block_1_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::ExecuteIfPossible,
		None,
	);
	block_2.body = Some(Vec::new());
	let result = importer.import_block(block_2).await.unwrap();
	assert!(!matches!(result, ImportResult::MissingState));
}

#[tokio::test]
async fn test_execute_if_possible_initial_sync_block_with_pruned_parent_is_not_best() {
	let (importer, client) = new_importer();
	let original_best = client.info().best_hash;
	let genesis_hash = client.info().genesis_hash;

	let block_1 = create_params(
		1,
		genesis_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::Skip,
		None,
	);
	let block_1_hash = block_1.header.hash();
	let _ = importer.import_block(block_1).await.unwrap();

	let mut block_2 = create_params(
		2,
		block_1_hash,
		1,
		None,
		BlockOrigin::NetworkInitialSync,
		StateAction::ExecuteIfPossible,
		None,
	);
	block_2.body = Some(Vec::new());
	let block_2_hash = block_2.header.hash();
	let result = importer.import_block(block_2).await.unwrap();

	assert!(matches!(result, ImportResult::Imported(_)));
	assert_eq!(client.info().best_hash, original_best);
	assert_ne!(client.info().best_hash, block_2_hash);
}

#[tokio::test]
async fn test_tie_loser() {
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
async fn test_duplicate_vote_block_same_tick_fails() {
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
async fn test_duplicate_compute_loser_same_power_fails() {
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
async fn test_reorg_to_lower_power_then_recover() {
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
