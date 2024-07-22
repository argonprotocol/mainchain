pub use bitcoind::start_bitcoind;
use std::env;

mod bitcoind;
mod ulx_node;
mod ulx_notary;
mod ulx_oracle;

pub use ulx_node::UlxTestNode;
pub use ulx_notary::UlxTestNotary;
pub use ulx_oracle::UlxTestOracle;

pub async fn start_ulx_test_node() -> UlxTestNode {
	let use_live = env::var("USE_LIVE")
		.unwrap_or(String::from("false"))
		.parse::<bool>()
		.unwrap_or_default();

	if use_live {
		UlxTestNode::from_url("ws://localhost:9944".to_string(), None).await
	} else {
		UlxTestNode::start("alice".to_string())
			.await
			.expect("Unable to create test context - ensure debug ulx-node build is available")
	}
}
