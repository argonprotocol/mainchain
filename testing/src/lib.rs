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

pub use argon_bitcoin::run_bitcoin_cli;
pub use argon_localchain::*;
pub use argon_node::ArgonTestNode;
pub use argon_notary::ArgonTestNotary;
pub use argon_oracle::ArgonTestOracle;
pub use bitcoind::*;

pub async fn start_argon_test_node() -> ArgonTestNode {
	let use_live = env::var("USE_LIVE")
		.unwrap_or(String::from("false"))
		.parse::<bool>()
		.unwrap_or_default();

	if use_live {
		ArgonTestNode::from_url("ws://localhost:9944".to_string(), None).await
	} else {
		ArgonTestNode::start("alice".to_string())
			.await
			.expect("Unable to create test context - ensure debug argon-node build is available")
	}
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
	if Path::is_dir(triple_target.as_path()) {
		return triple_target;
	}
	workspace_cargo_path.as_path().join("target/debug")
}
