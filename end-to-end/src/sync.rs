use crate::utils::{
	activate_vote_mining, create_active_notary_with_archive_bucket, wait_for_finalized_catchup,
};
use argon_client::{FetchAt, api::storage, conversion::SubxtRuntime};
use argon_notary_audit::VerifyError;
use argon_primitives::{ArgonDigests, BlockSealDigest};
use argon_testing::{ArgonNodeStartArgs, ArgonTestNode, ArgonTestNotary, test_miner_count};
use polkadot_sdk::sp_core::{DeriveJunction, Pair};
use serial_test::serial;
use std::{
	env,
	fs::{copy, create_dir_all, read_dir, read_to_string, remove_dir_all, write},
	io,
	path::{Path, PathBuf},
	process::Command,
	time::Duration,
};

#[tokio::test(flavor = "multi_thread")]
#[serial]
async fn test_fast_sync_catches_up_to_mixed_history() {
	assert_basic_sync_mode_catches_up("fast").await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow warp sync scenario"]
async fn test_warp_sync_catches_up_to_mixed_history() {
	assert_basic_sync_mode_catches_up("warp").await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow sync recovery scenario"]
async fn test_fast_sync_recovers_after_notebook_archive_delay() {
	assert_sync_mode_recovers_after_notebook_archive_delay("fast").await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow sync recovery scenario"]
async fn test_warp_sync_recovers_after_notebook_archive_delay() {
	assert_sync_mode_recovers_after_notebook_archive_delay("warp").await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow sync recovery scenario"]
async fn test_fast_sync_recovers_after_peer_stall() {
	assert_sync_mode_recovers_after_peer_stall("fast").await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow sync recovery scenario"]
async fn test_warp_sync_recovers_after_peer_stall() {
	assert_sync_mode_recovers_after_peer_stall("warp").await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow warp sync recovery scenario"]
async fn test_warp_sync_recovers_after_state_sync_restart() {
	assert_warp_sync_recovers_after_state_sync_restart().await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow live sync soak"]
async fn test_long_running_network_late_node_catches_up() {
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
}

#[tokio::test(flavor = "multi_thread")]
#[serial]
#[ignore = "slow live sync soak"]
async fn test_long_running_network_recovers_after_notary_outage() {
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
}

#[derive(Clone, Copy)]
struct FinalizedSnapshot {
	number: u32,
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

#[derive(Debug)]
struct FinalizedHistoryWindow {
	notebook_blocks_seen: u32,
	compute_blocks_seen: u32,
	vote_blocks_seen: u32,
}

async fn finalized_snapshot(node: &ArgonTestNode) -> FinalizedSnapshot {
	let finalized_hash = node.client.latest_finalized_block_hash().await.unwrap();
	let finalized_number = node.client.block_number(finalized_hash.hash()).await.unwrap();
	FinalizedSnapshot { number: finalized_number }
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
	let latest_finalized_number = node.client.block_number(latest_finalized.hash()).await.unwrap();
	assert!(
		latest_finalized_number >= snapshot.number,
		"{context}: expected finalized number >= {}, got {}",
		snapshot.number,
		latest_finalized_number,
	);
	assert_eq!(
		source.client.block_at_height(latest_finalized_number).await.unwrap(),
		Some(latest_finalized.hash()),
		"{context}",
	);

	let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
	let mut last_state_error = None;
	loop {
		let best_hash = node.client.best_block_hash().await.unwrap();
		let best_number = node.client.block_number(best_hash).await.unwrap();
		let source_best_at_height = source.client.block_at_height(best_number).await.unwrap();

		if best_number >= snapshot.number && source_best_at_height == Some(best_hash) {
			match node
				.client
				.fetch_storage(&storage().system().number(), FetchAt::Block(best_hash))
				.await
			{
				Ok(Some(state_number)) => {
					assert_eq!(
						state_number, best_number,
						"{context}: best state should match best block"
					);
					break;
				},
				Ok(None) => last_state_error = Some("storage returned None".to_string()),
				Err(err) => last_state_error = Some(err.to_string()),
			}
		}

		assert!(
			tokio::time::Instant::now() < deadline,
			"{context}: synced node did not recover state for its best block. best={best_number}, source_at_height={source_best_at_height:?}, last_state_error={last_state_error:?}",
		);

		println!(
			"Waiting for synced node to recover state at best block {best_number}. Last state error: {last_state_error:?}"
		);
		tokio::time::sleep(Duration::from_millis(250)).await;
	}
}

async fn assert_basic_sync_mode_catches_up(sync_mode: &str) {
	let mut harness = SyncHarness::start().await;
	harness.assert_warmup_history_window(40).await;

	let target = finalized_snapshot(&harness.source).await;
	let sync_node = harness.start_sync_node("ferdie", Some(sync_mode)).await;
	wait_for_finalized_catchup(&harness.source, &sync_node).await.unwrap();
	assert_node_matches_snapshot(
		&sync_node,
		&harness.source,
		target,
		&format!("{sync_mode} sync node should catch up to mixed finalized history"),
	)
	.await;
	if sync_mode == "warp" {
		assert_block_history_gap_fill_completes(
			&sync_node,
			"warp sync should complete historical block gap fill after state sync",
		)
		.await;
	}
	drop(sync_node);
	drop(harness);
}

async fn assert_sync_mode_recovers_after_notebook_archive_delay(sync_mode: &str) {
	let mut harness = SyncHarness::start().await;
	harness.assert_warmup_history_window(40).await;

	let target = finalized_snapshot(&harness.source).await;
	let missing_archive_host = "http://127.0.0.1:9/missing-notebook-archive".to_string();
	let mut sync_node = harness
		.start_sync_node_with("eve", Some(sync_mode), |args| {
			args.notebook_archive_urls = vec![missing_archive_host];
		})
		.await;

	tokio::time::sleep(Duration::from_secs(5)).await;
	sync_node.start_args.notebook_archive_urls = vec![harness.archive_host.clone()];
	sync_node.restart(Duration::from_secs(1)).await.unwrap();

	wait_for_finalized_catchup(&harness.source, &sync_node).await.unwrap();
	assert_node_matches_snapshot(
		&sync_node,
		&harness.source,
		target,
		&format!("{sync_mode} sync should recover after notebook archive delay"),
	)
	.await;
	if sync_mode == "warp" {
		assert_block_history_gap_fill_completes(
			&sync_node,
			"warp sync should complete block history after notebook archive delay",
		)
		.await;
	}
}

async fn assert_sync_mode_recovers_after_peer_stall(sync_mode: &str) {
	let mut harness = SyncHarness::start().await;
	harness.assert_warmup_history_window(40).await;

	let target = finalized_snapshot(&harness.source).await;
	let mut sync_node = harness
		.start_sync_node_with("ferdie", Some(sync_mode), |args| {
			args.bootnodes.clear();
		})
		.await;

	tokio::time::sleep(Duration::from_secs(5)).await;
	let stalled_finalized = sync_node.client.latest_finalized_block().await.unwrap();
	assert!(
		stalled_finalized < target.number,
		"{sync_mode} sync unexpectedly reached finalized target without peers: finalized={stalled_finalized}, target={}",
		target.number,
	);

	sync_node.start_args.bootnodes = harness.source.boot_url.clone();
	sync_node.restart(Duration::from_secs(1)).await.unwrap();

	wait_for_finalized_catchup(&harness.source, &sync_node).await.unwrap();
	assert_node_matches_snapshot(
		&sync_node,
		&harness.source,
		target,
		&format!("{sync_mode} sync should recover after peer stall"),
	)
	.await;
	if sync_mode == "warp" {
		assert_block_history_gap_fill_completes(
			&sync_node,
			"warp sync should complete block history after peer stall",
		)
		.await;
	}
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
		"warp sync should complete block history after state-sync restart",
	)
	.await;
}

async fn assert_block_history_gap_fill_completes(node: &ArgonTestNode, context: &str) {
	node.log_watcher
		.wait_for_log_for_secs("Block history download is complete", 1, 60)
		.await
		.unwrap_or_else(|err| panic!("{context}: {err:?}"));
}

impl SyncHarness {
	async fn start() -> Self {
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
		let archive_host =
			format!("{}/{}", ArgonTestNotary::get_minio_url(), archive_bucket.clone());
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
			create_active_notary_with_archive_bucket(&source, archive_bucket.clone())
				.await
				.expect("Notary registered")
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
