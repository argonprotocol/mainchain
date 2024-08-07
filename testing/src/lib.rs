pub use bitcoind::start_bitcoind;
use std::env;

mod argon_bitcoin;
mod argon_node;
mod argon_notary;
mod argon_oracle;
mod bitcoind;

pub use argon_bitcoin::run_bitcoin_cli;
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
