pub use bitcoind::start_bitcoind;
use current_platform::CURRENT_PLATFORM;
use std::{
	env,
	path::{Path, PathBuf},
};

mod argon_bitcoin;
mod argon_localchain;
mod argon_node;
mod argon_notary;
mod argon_oracle;
mod bitcoind;
mod log_watcher;

pub use crate::argon_node::ArgonNodeStartArgs;
pub use argon_bitcoin::run_bitcoin_cli;
pub use argon_localchain::*;
pub use argon_node::ArgonTestNode;
pub use argon_notary::ArgonTestNotary;
pub use argon_oracle::ArgonTestOracle;
pub use bitcoind::*;

pub async fn start_argon_test_node() -> ArgonTestNode {
	let start_args = ArgonNodeStartArgs::new("alice", test_miner_count(), "").unwrap();
	ArgonTestNode::start(start_args)
		.await
		.expect("Unable to create test context - ensure debug argon-node build is available")
}

pub fn test_miner_count() -> u16 {
	let cpus = num_cpus::get();
	let mut threads = 2;
	if cpus <= 2 {
		threads = 1;
	}
	threads
}

pub(crate) fn get_target_dir() -> PathBuf {
	let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let workspace_cargo_path = project_dir.join("..");
	let workspace_cargo_path =
		workspace_cargo_path.canonicalize().expect("Failed to canonicalize path");
	let triple_target = workspace_cargo_path
		.as_path()
		.join("target")
		.join(CURRENT_PLATFORM)
		.join("debug");
	if Path::is_dir(triple_target.as_path()) &&
		Path::is_file(&triple_target.as_path().join("argon-node"))
	{
		return triple_target;
	}
	workspace_cargo_path.as_path().join("target/debug")
}
