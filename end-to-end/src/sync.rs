use crate::utils::{
	activate_notary, activate_vote_mining, create_active_notary_with_archive_bucket,
	wait_for_finalized_catchup,
};
use argon_client::{FetchAt, api::storage, conversion::SubxtRuntime};
use argon_notary_audit::VerifyError;
use argon_primitives::{ArgonDigests, BlockSealDigest};
use argon_testing::{ArgonNodeStartArgs, ArgonTestNode, ArgonTestNotary, test_miner_count};
use polkadot_sdk::sp_core::{DeriveJunction, H256, Pair};
use serial_test::serial;
use std::{
	env,
	fs::{copy, create_dir_all, read_dir, read_to_string, remove_dir_all, write},
	future::Future,
	io,
	path::{Path, PathBuf},
	process::Command,
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicUsize, Ordering},
	},
	time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SYNC_TEST_TIMEOUT_SECS: u64 = 12 * 60;
const SYNC_SOAK_TEST_TIMEOUT_SECS: u64 = 45 * 60;

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_normal_fast_sync_smoke_catches_up_to_notebook_history() {
	run_sync_test_with_timeout(
		"test_normal_fast_sync_smoke_catches_up_to_notebook_history",
		SYNC_TEST_TIMEOUT_SECS,
		async {
			let mut source_args = ArgonNodeStartArgs::new("alice", 0, "").unwrap();
			let archive_bucket = ArgonTestNotary::create_archive_bucket();
			let archive_host =
				format!("{}/{}", ArgonTestNotary::get_minio_url(), archive_bucket.clone());
			source_args.notebook_archive_urls.push(archive_host);

			let source = ArgonTestNode::start(source_args).await.unwrap();
			let miner_threads = test_miner_count();
			let _miner = source.fork_node("bob", miner_threads).await.unwrap();
			let _test_notary = create_active_notary_with_archive_bucket(&source, archive_bucket)
				.await
				.expect("Notary registered");

			let target = wait_for_finalized_notebook_snapshot(&source, 4, 40).await;
			let mut sync_args = source.get_fork_args("charlie", 0);
			sync_args.is_validator = false;
			sync_args.is_archive_node = false;
			sync_args.extra_flags.push("--sync=fast".to_string());
			let sync_node = source.fork_node_with(sync_args).await.unwrap();

			wait_for_finalized_catchup(&source, &sync_node).await.unwrap();
			assert_node_matches_snapshot(
				&sync_node,
				&source,
				target,
				"fast sync node should catch up to recent notebook history",
			)
			.await;
		},
	)
	.await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "sync recovery scenario runs in the sync action"]
async fn test_normal_fast_sync_catches_up_to_mixed_history() {
	run_sync_test_with_timeout(
		"test_normal_fast_sync_catches_up_to_mixed_history",
		SYNC_TEST_TIMEOUT_SECS,
		assert_basic_sync_mode_catches_up("fast"),
	)
	.await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "sync recovery scenario runs in the sync action"]
async fn test_normal_warp_sync_catches_up_to_mixed_history() {
	run_sync_test_with_timeout(
		"test_normal_warp_sync_catches_up_to_mixed_history",
		SYNC_TEST_TIMEOUT_SECS,
		assert_basic_sync_mode_catches_up("warp"),
	)
	.await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "sync recovery scenario runs in the sync action"]
async fn test_normal_fast_sync_recovers_from_notebook_access_backlog_pressure() {
	run_sync_test_with_timeout(
		"test_normal_fast_sync_recovers_from_notebook_access_backlog_pressure",
		SYNC_TEST_TIMEOUT_SECS,
		assert_sync_mode_recovers_from_notebook_access_backlog_pressure("fast"),
	)
	.await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "sync recovery scenario runs in the sync action"]
async fn test_normal_warp_sync_recovers_from_notebook_access_backlog_pressure() {
	run_sync_test_with_timeout(
		"test_normal_warp_sync_recovers_from_notebook_access_backlog_pressure",
		SYNC_TEST_TIMEOUT_SECS,
		assert_sync_mode_recovers_from_notebook_access_backlog_pressure("warp"),
	)
	.await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "restart during warp state sync is not a default CI contract"]
async fn test_warp_sync_recovers_after_state_sync_restart() {
	run_sync_test_with_timeout(
		"test_warp_sync_recovers_after_state_sync_restart",
		SYNC_TEST_TIMEOUT_SECS,
		assert_warp_sync_recovers_after_state_sync_restart(),
	)
	.await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow live sync soak"]
async fn test_soak_late_node_catches_up() {
	run_sync_test_with_timeout(
		"test_soak_late_node_catches_up",
		SYNC_SOAK_TEST_TIMEOUT_SECS,
		async {
			let settings = SyncSoakSettings::from_env();
			let mut harness = SyncHarness::start().await;
			harness.assert_warmup_history_window(settings.warmup_finalized_blocks).await;

			let late_sync_target = finalized_snapshot(&harness.source).await;
			let late_node = harness.start_sync_node("ferdie", None).await;
			wait_for_finalized_catchup(&harness.source, &late_node).await.unwrap();
			assert_node_matches_snapshot(
				&late_node,
				&harness.source,
				late_sync_target,
				"late node should catch up to the long-running network",
			)
			.await;
		},
	)
	.await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow live sync soak"]
async fn test_soak_recovers_after_notary_outage() {
	run_sync_test_with_timeout(
		"test_soak_recovers_after_notary_outage",
		SYNC_SOAK_TEST_TIMEOUT_SECS,
		async {
			let settings = SyncSoakSettings::from_env();
			let mut harness = SyncHarness::start().await;
			harness.assert_warmup_history_window(settings.warmup_finalized_blocks).await;

			let mut recovering_node = harness.start_sync_node("eve", None).await;
			tokio::time::sleep(Duration::from_secs(3)).await;
			harness.test_notary.stop();
			tokio::time::sleep(Duration::from_secs(settings.outage_seconds)).await;
			recovering_node.restart(Duration::from_secs(1)).await.unwrap();
			tokio::time::sleep(Duration::from_secs(2)).await;
			harness.restart_test_notary().await.unwrap();
			harness.ensure_vote_mining_active().await;
			harness
				.assert_mixed_history_window(
					settings.recovery_finalized_blocks,
					"recovery window should resume notebook, vote, and compute history after notary restart",
				)
				.await;

			let recovery_target = finalized_snapshot(&harness.source).await;
			wait_for_finalized_catchup(&harness.source, &recovering_node).await.unwrap();
			assert_node_matches_snapshot(
				&recovering_node,
				&harness.source,
				recovery_target,
				"restarted syncing node should recover after notary outage",
			)
			.await;
		},
	)
	.await;
}

async fn run_sync_test_with_timeout(
	test_name: &str,
	timeout_secs: u64,
	test: impl Future<Output = ()>,
) {
	tokio::time::timeout(Duration::from_secs(timeout_secs), test)
		.await
		.unwrap_or_else(|_| panic!("{test_name} timed out after {timeout_secs}s"));
}

#[derive(Clone, Copy)]
struct FinalizedSnapshot {
	number: u32,
	hash: H256,
}

struct SyncSoakSettings {
	warmup_finalized_blocks: u32,
	recovery_finalized_blocks: u32,
	outage_seconds: u64,
}

struct SyncHarness {
	source: ArgonTestNode,
	vote_miner_1: ArgonTestNode,
	vote_miner_2: ArgonTestNode,
	test_notary: ArgonTestNotary,
	state_cache: Option<SyncStateCache>,
	archive_host: String,
	reused_state: bool,
}

struct DelayedArchiveProxy {
	url: String,
	ready: Arc<AtomicBool>,
	blocked_requests: Arc<AtomicUsize>,
	handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug)]
struct FinalizedHistoryWindow {
	notebook_blocks_seen: u32,
	compute_blocks_seen: u32,
	vote_blocks_seen: u32,
}

async fn finalized_snapshot(node: &ArgonTestNode) -> FinalizedSnapshot {
	let finalized_hash = node.client.latest_finalized_block_hash().await.unwrap();
	let finalized_number = node.client.block_number(finalized_hash.hash()).await.unwrap();
	FinalizedSnapshot { number: finalized_number, hash: finalized_hash.hash() }
}

async fn wait_for_finalized_notebook_snapshot(
	node: &ArgonTestNode,
	min_notebook_blocks: u32,
	max_finalized_blocks: u32,
) -> FinalizedSnapshot {
	let start_finalized = node.client.latest_finalized_block().await.unwrap();
	let mut blocks_sub = node.client.live.blocks().subscribe_finalized().await.unwrap();
	let mut notebook_blocks_seen = 0;
	let mut finalized_seen = 0;

	while let Some(Ok(block)) = blocks_sub.next().await {
		if block.number() <= start_finalized {
			continue;
		}
		finalized_seen += 1;

		let has_notebooks = block.header().runtime_digest().logs.iter().any(|digest| {
			digest
				.as_notebooks::<VerifyError>()
				.is_some_and(|notebooks| !notebooks.notebooks.is_empty())
		});
		if has_notebooks {
			notebook_blocks_seen += 1;
			if notebook_blocks_seen >= min_notebook_blocks {
				return FinalizedSnapshot { number: block.number(), hash: block.hash() };
			}
		}

		assert!(
			finalized_seen < max_finalized_blocks,
			"only saw {notebook_blocks_seen} notebook block(s) after {finalized_seen} finalized blocks",
		);
	}

	panic!("finalized block subscription ended before enough notebook blocks were observed");
}

async fn header_hash_at_height(node: &ArgonTestNode, height: u32) -> Option<H256> {
	node.client.methods.chain_get_block_hash(Some(height.into())).await.unwrap()
}

async fn observe_finalized_history_window(
	node: &ArgonTestNode,
	additional_finalized_blocks: u32,
) -> FinalizedHistoryWindow {
	let start_finalized = node.client.latest_finalized_block().await.unwrap();
	let target_number = start_finalized + additional_finalized_blocks;
	let mut blocks_sub = node.client.live.blocks().subscribe_finalized().await.unwrap();
	let mut notebook_blocks_seen = 0;
	let mut compute_blocks_seen = 0;
	let mut vote_blocks_seen = 0;

	loop {
		let Some(Ok(block)) = blocks_sub.next().await else {
			panic!("finalized block subscription ended before target");
		};
		if block.number() <= start_finalized {
			continue;
		}

		let mut saw_notebooks = false;
		let mut saw_vote_seal = false;
		for digest in block.header().runtime_digest().logs.iter() {
			if let Some(notebooks) = digest.as_notebooks::<VerifyError>() {
				if !notebooks.notebooks.is_empty() {
					saw_notebooks = true;
				}
			}
			if matches!(digest.as_block_seal(), Some(BlockSealDigest::Vote { .. })) {
				saw_vote_seal = true;
			}
		}

		if saw_notebooks {
			notebook_blocks_seen += 1;
		}
		if saw_vote_seal {
			vote_blocks_seen += 1;
		} else {
			compute_blocks_seen += 1;
		}

		if block.number() >= target_number {
			return FinalizedHistoryWindow {
				notebook_blocks_seen,
				compute_blocks_seen,
				vote_blocks_seen,
			};
		}
	}
}

async fn assert_node_matches_snapshot(
	node: &ArgonTestNode,
	source: &ArgonTestNode,
	snapshot: FinalizedSnapshot,
	context: &str,
) {
	let latest_finalized = node.client.latest_finalized_block_hash().await.unwrap();
	let latest_finalized_hash = latest_finalized.hash();
	let latest_finalized_number = node.client.block_number(latest_finalized_hash).await.unwrap();
	assert!(
		latest_finalized_number >= snapshot.number,
		"{context}: expected finalized number >= {}, got {}",
		snapshot.number,
		latest_finalized_number,
	);

	let source_finalized_hash = header_hash_at_height(source, latest_finalized_number).await;
	assert_eq!(source_finalized_hash, Some(latest_finalized_hash), "{context}",);

	let synced_target_hash = header_hash_at_height(node, snapshot.number).await;
	assert_eq!(
		synced_target_hash,
		Some(snapshot.hash),
		"{context}: synced node should retain the target finalized block hash",
	);

	let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
	loop {
		let best_hash = node.client.best_block_hash().await.unwrap();
		let best_number = node.client.block_number(best_hash).await.unwrap();
		let source_best_at_height = header_hash_at_height(source, best_number).await;
		let mut last_state_error = None;

		if best_number >= snapshot.number && source_best_at_height == Some(best_hash) {
			match node
				.client
				.fetch_storage(&storage().system().number(), FetchAt::Block(best_hash))
				.await
			{
				Ok(Some(state_number)) => {
					assert_eq!(
						state_number, best_number,
						"{context}: state should match block number at {best_hash:?}"
					);
					return;
				},
				Ok(None) => last_state_error = Some("storage returned None".to_string()),
				Err(err) => last_state_error = Some(err.to_string()),
			}
		}

		assert!(
			tokio::time::Instant::now() < deadline,
			"{context}: synced node did not recover a source-matching best block with state. best={best_number}, source_at_height={source_best_at_height:?}, last_state_error={last_state_error:?}",
		);

		println!(
			"Waiting for synced node to recover a source-matching best block with state. best={best_number}, last_state_error={last_state_error:?}"
		);
		tokio::time::sleep(Duration::from_millis(250)).await;
	}
}

async fn assert_node_finalized_lags_behind(
	node: &ArgonTestNode,
	source: &ArgonTestNode,
	min_lag: u32,
	context: &str,
) {
	let source_finalized = source.client.latest_finalized_block().await.unwrap();
	let node_finalized = node.client.latest_finalized_block().await.unwrap();
	assert!(
		node_finalized.saturating_add(min_lag) <= source_finalized,
		"{context}: expected node finalized block {node_finalized} to lag source finalized block {source_finalized} by at least {min_lag} blocks",
	);
}

async fn assert_basic_sync_mode_catches_up(sync_mode: &str) {
	let mut harness = SyncHarness::start().await;
	harness.assert_warmup_history_window(40).await;

	let target = finalized_snapshot(&harness.source).await;
	let sync_node = harness.start_sync_node("ferdie", Some(sync_mode)).await;
	wait_for_finalized_catchup(&harness.source, &sync_node).await.unwrap();
	if sync_mode == "warp" {
		assert_block_history_gap_fill_completes(
			&sync_node,
			&harness.source,
			target,
			"warp sync should complete historical block gap fill after state sync",
		)
		.await;
	}
	assert_node_matches_snapshot(
		&sync_node,
		&harness.source,
		target,
		&format!("{sync_mode} sync node should catch up to mixed finalized history"),
	)
	.await;
	drop(sync_node);
	drop(harness);
}

async fn assert_sync_mode_recovers_from_notebook_access_backlog_pressure(sync_mode: &str) {
	let delayed_archive =
		DelayedArchiveProxy::start(&ArgonTestNotary::get_minio_url()).await.unwrap();
	let mut harness = SyncHarness::start().await;
	let delayed_archive_host = format!(
		"{}/{}",
		delayed_archive.url.trim_end_matches('/'),
		harness.archive_host.rsplit('/').next().unwrap()
	);
	harness.assert_warmup_history_window(40).await;

	let sync_node = harness
		.start_sync_node_with("eve", Some(sync_mode), |args| {
			args.notebook_archive_urls = vec![delayed_archive_host.clone()];
		})
		.await;

	harness
		.assert_vote_history_window(
			50,
			"source should build notebook-backed vote history while sync node lacks notebook archive access",
		)
		.await;
	wait_for_blocked_archive_requests(
		&delayed_archive,
		60,
		&format!(
			"{sync_mode} sync should attempt blocked notebook archive reads before archive access is restored"
		),
	)
	.await;

	assert_node_finalized_lags_behind(
		&sync_node,
		&harness.source,
		10,
		&format!("{sync_mode} sync should accumulate backlog without notebook archive access"),
	)
	.await;
	delayed_archive.release();

	harness
		.assert_vote_history_window(
			20,
			"source should keep producing notebook-backed vote history while notebook archive access recovers",
		)
		.await;

	let target = finalized_snapshot(&harness.source).await;
	wait_for_finalized_catchup(&harness.source, &sync_node).await.unwrap();
	if sync_mode == "warp" {
		assert_block_history_gap_fill_completes(
			&sync_node,
			&harness.source,
			target,
			"warp sync should complete historical block gap fill after notebook access backlog pressure",
		)
		.await;
	}
	assert_node_matches_snapshot(
		&sync_node,
		&harness.source,
		target,
		&format!("{sync_mode} sync should recover from notebook access backlog pressure"),
	)
	.await;
}

async fn assert_warp_sync_recovers_after_state_sync_restart() {
	let mut harness = SyncHarness::start().await;
	harness.assert_warmup_history_window(40).await;

	let target = finalized_snapshot(&harness.source).await;
	let mut sync_node = harness.start_sync_node("ferdie", Some("warp")).await;
	sync_node
		.log_watcher
		.wait_for_log_for_secs("Warp sync is complete, continuing with state sync", 1, 30)
		.await
		.unwrap();
	sync_node.restart(Duration::from_secs(1)).await.unwrap();

	wait_for_finalized_catchup(&harness.source, &sync_node).await.unwrap();
	assert_node_matches_snapshot(
		&sync_node,
		&harness.source,
		target,
		"warp sync should recover after restart during state sync",
	)
	.await;
	assert_block_history_gap_fill_completes(
		&sync_node,
		&harness.source,
		target,
		"warp sync should complete block history after state-sync restart",
	)
	.await;
}

async fn assert_block_history_gap_fill_completes(
	node: &ArgonTestNode,
	source: &ArgonTestNode,
	snapshot: FinalizedSnapshot,
	context: &str,
) {
	node.log_watcher
		.wait_for_log_for_secs("Block history download is complete", 1, 60)
		.await
		.unwrap_or_else(|err| panic!("{context}: {err:?}"));

	let mut checked_heights = Vec::new();
	for height in [1, snapshot.number / 2, snapshot.number.saturating_sub(1)] {
		if height == 0 || checked_heights.contains(&height) {
			continue;
		}
		checked_heights.push(height);

		let source_hash = header_hash_at_height(source, height).await;
		let node_hash = header_hash_at_height(node, height).await;
		assert_eq!(
			node_hash, source_hash,
			"{context}: historical block mismatch at height {height}"
		);
	}
}

async fn wait_for_blocked_archive_requests(
	delayed_archive: &DelayedArchiveProxy,
	timeout_secs: u64,
	context: &str,
) {
	let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

	loop {
		if delayed_archive.blocked_request_count() > 0 {
			return;
		}

		assert!(tokio::time::Instant::now() < deadline, "{context}");
		tokio::time::sleep(Duration::from_millis(250)).await;
	}
}

impl SyncHarness {
	async fn start() -> Self {
		Self::start_with_archive_endpoint(None).await
	}

	async fn start_with_archive_endpoint(archive_endpoint: Option<String>) -> Self {
		let compute_threads = test_miner_count();
		let state_cache = SyncStateCache::from_env();
		if let Some(cache) = &state_cache {
			cache.prepare_run_dirs().unwrap();
		}
		let mut source_args = ArgonNodeStartArgs::new("alice", compute_threads, "").unwrap();
		let archive_bucket = state_cache
			.as_ref()
			.map(|cache| cache.archive_bucket.clone())
			.unwrap_or_else(ArgonTestNotary::create_archive_bucket);
		let archive_base = archive_endpoint.clone().unwrap_or_else(ArgonTestNotary::get_minio_url);
		let archive_host = format!("{}/{}", archive_base.trim_end_matches('/'), archive_bucket);
		source_args.notebook_archive_urls.push(archive_host.clone());
		if let Some(cache) = &state_cache {
			source_args.base_data_path = cache.run_node_path("alice");
			source_args.cleanup_base_data_path = false;
			create_dir_all(&source_args.base_data_path).unwrap();
		}

		let source = ArgonTestNode::start(source_args).await.unwrap();
		let reused_state = state_cache.as_ref().is_some_and(|cache| cache.reused_state);

		let test_notary = if reused_state {
			let checkpoint_db_name = state_cache
				.as_ref()
				.and_then(|cache| cache.checkpoint_db_name.as_deref())
				.expect("reused sync state should include a checkpoint notary db name");
			let run_db_name = ArgonTestNotary::clone_database(checkpoint_db_name)
				.await
				.expect("checkpoint notary db should clone");
			ArgonTestNotary::start_with_archive(
				&source,
				archive_bucket.clone(),
				state_cache.as_ref().and_then(|cache| cache.notary_port),
				Some(run_db_name),
				true,
			)
			.await
			.expect("Notary restarted from preserved sync state")
		} else {
			match archive_endpoint {
				Some(archive_endpoint) => {
					let test_notary = ArgonTestNotary::start_with_archive_endpoint(
						&source,
						archive_bucket.clone(),
						Some(archive_endpoint),
						None,
						None,
						true,
					)
					.await
					.expect("Notary started with archive endpoint");
					activate_notary(&source, &test_notary).await.expect("Notary registered");
					test_notary
				},
				None => create_active_notary_with_archive_bucket(&source, archive_bucket.clone())
					.await
					.expect("Notary registered"),
			}
		};
		if let Some(cache) = &state_cache {
			let notary_port = test_notary
				.ws_url
				.parse::<url::Url>()
				.unwrap()
				.port()
				.expect("test notary ws url should include a port");
			write(cache.root.join("notary_port.txt"), notary_port.to_string()).unwrap();
		}

		let mut vote_miner_1_args = source.get_fork_args("bob", 0);
		if let Some(cache) = &state_cache {
			vote_miner_1_args.base_data_path = cache.run_node_path("bob");
			vote_miner_1_args.cleanup_base_data_path = false;
			create_dir_all(&vote_miner_1_args.base_data_path).unwrap();
		}
		let vote_miner_1 = source.fork_node_with(vote_miner_1_args).await.unwrap();

		let mut vote_miner_2_args = source.get_fork_args("dave", 0);
		if let Some(cache) = &state_cache {
			vote_miner_2_args.base_data_path = cache.run_node_path("dave");
			vote_miner_2_args.cleanup_base_data_path = false;
			create_dir_all(&vote_miner_2_args.base_data_path).unwrap();
		}
		let vote_miner_2 = source.fork_node_with(vote_miner_2_args).await.unwrap();
		if !reused_state {
			activate_vote_mining(&source, &vote_miner_1, &vote_miner_2).await.unwrap();
		}

		Self {
			source,
			vote_miner_1,
			vote_miner_2,
			test_notary,
			state_cache,
			archive_host,
			reused_state,
		}
	}

	async fn assert_mixed_history_window(&self, additional_finalized_blocks: u32, context: &str) {
		let history =
			observe_finalized_history_window(&self.source, additional_finalized_blocks).await;
		assert!(history.notebook_blocks_seen > 0, "{context}: {history:?}");
		assert!(history.vote_blocks_seen > 0, "{context}: {history:?}");
		assert!(history.compute_blocks_seen > 0, "{context}: {history:?}");
	}

	async fn assert_vote_history_window(&self, additional_finalized_blocks: u32, context: &str) {
		let history =
			observe_finalized_history_window(&self.source, additional_finalized_blocks).await;
		assert!(history.notebook_blocks_seen > 0, "{context}: {history:?}");
		assert!(history.vote_blocks_seen > 0, "{context}: {history:?}");
	}

	async fn assert_warmup_history_window(&mut self, additional_finalized_blocks: u32) {
		self.ensure_vote_mining_active().await;
		let history_window = if self.reused_state {
			additional_finalized_blocks.min(30)
		} else {
			additional_finalized_blocks
		};
		self.ensure_vote_mining_active().await;
		self.assert_vote_history_window(
			history_window.min(30),
			"warmup window should resume notebook and vote history before sync scenarios start",
		)
		.await;
		self.capture_checkpoint().await;
	}

	async fn start_sync_node(&self, authority: &str, sync_mode: Option<&str>) -> ArgonTestNode {
		self.start_sync_node_with(authority, sync_mode, |_| {}).await
	}

	async fn start_sync_node_with(
		&self,
		authority: &str,
		sync_mode: Option<&str>,
		configure: impl FnOnce(&mut ArgonNodeStartArgs),
	) -> ArgonTestNode {
		let mut sync_args = self.source.get_fork_args(authority, 0);
		sync_args.is_validator = false;
		sync_args.is_archive_node = false;
		if let Some(sync_mode) = sync_mode {
			sync_args.extra_flags.push(format!("--sync={sync_mode}"));
		}
		configure(&mut sync_args);
		self.source.fork_node_with(sync_args).await.unwrap()
	}

	async fn capture_checkpoint(&mut self) {
		let Some(cache) = &self.state_cache else {
			return;
		};
		if cache.reused_state {
			return;
		}

		self.test_notary.stop();
		self.vote_miner_1.stop().unwrap();
		self.vote_miner_2.stop().unwrap();
		self.source.stop().unwrap();

		cache
			.capture_checkpoint_dirs(
				&self.source.start_args.base_data_path,
				&self.vote_miner_1.start_args.base_data_path,
				&self.vote_miner_2.start_args.base_data_path,
			)
			.unwrap();

		let checkpoint_db_name = ArgonTestNotary::clone_database(&self.test_notary.db_name)
			.await
			.expect("warmup checkpoint should clone notary db");
		cache.store_checkpoint_notary_db_name(&checkpoint_db_name).unwrap();

		self.source.restart(Duration::from_secs(1)).await.unwrap();
		self.vote_miner_1.restart(Duration::from_secs(1)).await.unwrap();
		self.vote_miner_2.restart(Duration::from_secs(1)).await.unwrap();
		self.restart_test_notary().await.unwrap();
	}

	async fn restart_test_notary(&mut self) -> anyhow::Result<()> {
		if !self.reused_state {
			return self.test_notary.restart().await;
		}

		let cache =
			self.state_cache.as_ref().expect("reused sync state should include state cache");
		let checkpoint_db_name = cache
			.checkpoint_db_name
			.as_deref()
			.expect("reused sync state should include a checkpoint notary db name");
		let notary_port = self
			.test_notary
			.ws_url
			.parse::<url::Url>()
			.unwrap()
			.port()
			.or(cache.notary_port)
			.expect("reused sync state should include a fixed notary port");

		self.test_notary.stop();
		let run_db_name = ArgonTestNotary::clone_database(checkpoint_db_name).await?;
		self.test_notary = ArgonTestNotary::start_with_archive(
			&self.source,
			cache.archive_bucket.clone(),
			Some(notary_port),
			Some(run_db_name),
			true,
		)
		.await?;
		Ok(())
	}

	async fn ensure_vote_mining_active(&self) {
		let bob_account = self.source.client.api_account(&self.vote_miner_1.account_id);
		let dave_account = self.source.client.api_account(&self.vote_miner_2.account_id);
		let bob_second_account = self
			.vote_miner_1
			.keyring()
			.pair()
			.clone()
			.derive(vec![DeriveJunction::hard(1)].into_iter(), None)
			.unwrap()
			.0
			.public()
			.into();
		let bob_second_account = self.source.client.api_account(&bob_second_account);
		let fetch_at = FetchAt::Best;

		let bob_registration = self
			.source
			.client
			.fetch_storage(&storage().mining_slot().account_index_lookup(bob_account), fetch_at)
			.await
			.unwrap();
		let bob_second_registration = self
			.source
			.client
			.fetch_storage(
				&storage().mining_slot().account_index_lookup(bob_second_account),
				fetch_at,
			)
			.await
			.unwrap();
		let dave_registration = self
			.source
			.client
			.fetch_storage(&storage().mining_slot().account_index_lookup(dave_account), fetch_at)
			.await
			.unwrap();

		if bob_registration.is_some() &&
			bob_second_registration.is_some() &&
			dave_registration.is_some()
		{
			return;
		}

		let active_miners = self
			.source
			.client
			.fetch_storage(&storage().mining_slot().active_miners_count(), fetch_at)
			.await
			.unwrap()
			.unwrap_or_default();
		let next_frame_id = self
			.source
			.client
			.fetch_storage(&storage().mining_slot().next_frame_id(), fetch_at)
			.await
			.unwrap()
			.unwrap_or_default();
		println!(
			"Refreshing vote mining before sync scenario start. bob={bob_registration:?} bob_second={bob_second_registration:?} dave={dave_registration:?} active_miners={active_miners} next_frame_id={next_frame_id}"
		);

		activate_vote_mining(&self.source, &self.vote_miner_1, &self.vote_miner_2)
			.await
			.unwrap();
	}
}

impl SyncSoakSettings {
	fn from_env() -> Self {
		Self {
			warmup_finalized_blocks: env::var("ARGON_LONG_SYNC_FINALIZED_BLOCKS")
				.ok()
				.and_then(|value| value.parse::<u32>().ok())
				.unwrap_or(400),
			recovery_finalized_blocks: env::var("ARGON_LONG_SYNC_RECOVERY_BLOCKS")
				.ok()
				.and_then(|value| value.parse::<u32>().ok())
				.unwrap_or(80),
			outage_seconds: env::var("ARGON_LONG_SYNC_OUTAGE_SECONDS")
				.ok()
				.and_then(|value| value.parse::<u64>().ok())
				.unwrap_or(12),
		}
	}
}

struct SyncStateCache {
	root: PathBuf,
	checkpoint_root: PathBuf,
	run_root: PathBuf,
	archive_bucket: String,
	notary_port: Option<u16>,
	checkpoint_db_name: Option<String>,
	reused_state: bool,
}

impl SyncStateCache {
	fn from_env() -> Option<Self> {
		let root = PathBuf::from(env::var("ARGON_LONG_SYNC_STATE_DIR").ok()?);
		create_dir_all(&root).unwrap();
		let checkpoint_root = root.join("checkpoint");
		let run_root = root.join("run");
		create_dir_all(&checkpoint_root).unwrap();
		create_dir_all(&run_root).unwrap();

		let checkpoint_nodes = ["alice", "bob", "dave"]
			.into_iter()
			.filter(|authority| checkpoint_root.join(authority).join("chains").exists())
			.count();
		assert!(
			checkpoint_nodes == 0 || checkpoint_nodes == 3,
			"incomplete sync state checkpoint at {}: expected alice/bob/dave chain data together",
			checkpoint_root.display(),
		);

		let archive_bucket_path = root.join("archive_bucket.txt");
		let notary_port_path = root.join("notary_port.txt");
		let checkpoint_db_name_path = root.join("notary_db_name.txt");
		let reused_state = checkpoint_nodes == 3;
		let archive_bucket = if archive_bucket_path.exists() {
			read_to_string(&archive_bucket_path).unwrap().trim().to_string()
		} else {
			let archive_bucket = ArgonTestNotary::create_archive_bucket();
			write(&archive_bucket_path, &archive_bucket).unwrap();
			archive_bucket
		};
		assert!(
			!reused_state || !archive_bucket.is_empty(),
			"missing archive bucket for preserved sync state at {}",
			root.display(),
		);
		let notary_port = if notary_port_path.exists() {
			Some(
				read_to_string(&notary_port_path)
					.unwrap()
					.trim()
					.parse::<u16>()
					.expect("sync state notary port should parse"),
			)
		} else {
			None
		};
		assert!(
			!reused_state || notary_port.is_some(),
			"missing notary port for preserved sync state at {}",
			root.display(),
		);
		let checkpoint_db_name = if checkpoint_db_name_path.exists() {
			Some(read_to_string(&checkpoint_db_name_path).unwrap().trim().to_string())
		} else {
			None
		};
		assert!(
			!reused_state || checkpoint_db_name.is_some(),
			"missing notary db name for preserved sync state at {}",
			root.display(),
		);

		Some(Self {
			root,
			checkpoint_root,
			run_root,
			archive_bucket,
			notary_port,
			checkpoint_db_name,
			reused_state,
		})
	}

	fn prepare_run_dirs(&self) -> io::Result<()> {
		for authority in ["alice", "bob", "dave"] {
			let run_path = self.run_node_path(authority);
			stop_stale_sync_node(&run_path);
			if run_path.exists() {
				remove_dir_all(&run_path)?;
			}
			if self.reused_state {
				copy_dir_all(&self.checkpoint_node_path(authority), &run_path)?;
			} else {
				create_dir_all(&run_path)?;
			}
		}
		Ok(())
	}

	fn capture_checkpoint_dirs(
		&self,
		source_path: &PathBuf,
		vote_miner_1_path: &PathBuf,
		vote_miner_2_path: &PathBuf,
	) -> io::Result<()> {
		for (authority, source) in
			[("alice", source_path), ("bob", vote_miner_1_path), ("dave", vote_miner_2_path)]
		{
			let checkpoint_path = self.checkpoint_node_path(authority);
			if checkpoint_path.exists() {
				remove_dir_all(&checkpoint_path)?;
			}
			copy_dir_all(source, &checkpoint_path)?;
		}
		Ok(())
	}

	fn store_checkpoint_notary_db_name(&self, db_name: &str) -> io::Result<()> {
		write(self.root.join("notary_db_name.txt"), db_name)
	}

	fn checkpoint_node_path(&self, authority: &str) -> PathBuf {
		self.checkpoint_root.join(authority)
	}

	fn run_node_path(&self, authority: &str) -> PathBuf {
		self.run_root.join(authority)
	}
}

impl DelayedArchiveProxy {
	async fn start(target_archive_endpoint: &str) -> anyhow::Result<Self> {
		let target_url = url::Url::parse(target_archive_endpoint)?;
		let target_host = target_url
			.host_str()
			.ok_or_else(|| anyhow::anyhow!("archive endpoint should include a host"))?
			.to_string();
		let target_port = target_url
			.port_or_known_default()
			.ok_or_else(|| anyhow::anyhow!("archive endpoint should include a port"))?;
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
		let local_addr = listener.local_addr()?;
		let ready = Arc::new(AtomicBool::new(false));
		let blocked_requests = Arc::new(AtomicUsize::new(0));
		let ready_for_task = ready.clone();
		let blocked_requests_for_task = blocked_requests.clone();

		let handle = tokio::spawn(async move {
			loop {
				let Ok((socket, _)) = listener.accept().await else {
					return;
				};
				tokio::spawn(handle_archive_proxy_connection(
					socket,
					target_host.clone(),
					target_port,
					ready_for_task.clone(),
					blocked_requests_for_task.clone(),
				));
			}
		});

		Ok(Self { url: format!("http://{local_addr}"), ready, blocked_requests, handle })
	}

	fn release(&self) {
		self.ready.store(true, Ordering::SeqCst);
	}

	fn blocked_request_count(&self) -> usize {
		self.blocked_requests.load(Ordering::SeqCst)
	}
}

impl Drop for DelayedArchiveProxy {
	fn drop(&mut self) {
		self.handle.abort();
	}
}

async fn handle_archive_proxy_connection(
	mut socket: tokio::net::TcpStream,
	target_host: String,
	target_port: u16,
	ready: Arc<AtomicBool>,
	blocked_requests: Arc<AtomicUsize>,
) -> io::Result<()> {
	let request = read_http_request(&mut socket).await?;
	let Some((method, path)) = request_method_and_path(&request) else {
		return write_http_response(&mut socket, "400 Bad Request").await;
	};
	if !ready.load(Ordering::SeqCst) && method == "GET" && path.contains("/notary/") {
		blocked_requests.fetch_add(1, Ordering::SeqCst);
		return write_http_response(&mut socket, "503 Service Unavailable").await;
	}

	let mut upstream = tokio::net::TcpStream::connect((target_host.as_str(), target_port)).await?;
	upstream.write_all(&request).await?;
	let _ = tokio::io::copy_bidirectional(&mut socket, &mut upstream).await?;
	Ok(())
}

async fn read_http_request(socket: &mut tokio::net::TcpStream) -> io::Result<Vec<u8>> {
	let mut request = Vec::new();
	let mut buffer = [0; 1024];
	while request.len() < 64 * 1024 {
		let bytes_read = socket.read(&mut buffer).await?;
		if bytes_read == 0 {
			break;
		}
		request.extend_from_slice(&buffer[..bytes_read]);
		if request.windows(4).any(|window| window == b"\r\n\r\n") {
			break;
		}
	}
	Ok(request)
}

fn request_method_and_path(request: &[u8]) -> Option<(String, String)> {
	let request = String::from_utf8_lossy(request);
	let request_line = request.lines().next()?;
	let mut parts = request_line.split_whitespace();
	match (parts.next(), parts.next(), parts.next()) {
		(Some(method), Some(path), Some(_)) => Some((method.to_string(), path.to_string())),
		_ => None,
	}
}

async fn write_http_response(socket: &mut tokio::net::TcpStream, status: &str) -> io::Result<()> {
	let response = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
	socket.write_all(response.as_bytes()).await
}

fn copy_dir_all(source: &PathBuf, destination: &PathBuf) -> io::Result<()> {
	create_dir_all(destination)?;
	for entry in read_dir(source)? {
		let entry = entry?;
		let source_path = entry.path();
		let destination_path = destination.join(entry.file_name());
		let metadata = entry.metadata()?;
		if metadata.is_dir() {
			copy_dir_all(&source_path, &destination_path)?;
		} else {
			copy(&source_path, &destination_path)?;
		}
	}
	Ok(())
}

fn stop_stale_sync_node(base_data_path: &Path) {
	let _ = Command::new("pkill")
		.args(["-INT", "-f", &format!("argon-node.*--base-path={}", base_data_path.display())])
		.status();
	std::thread::sleep(Duration::from_secs(1));
}
