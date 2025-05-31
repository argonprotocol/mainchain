pub use argon_notary_apis::{Client, create_client};

pub mod apis {
	pub use argon_notary_apis::{localchain, notebook};
}
pub use argon_notary_apis::error::Error;
pub use argon_primitives::ensure;
pub use server::NotaryServer;

pub mod stores;

pub mod block_watch;

pub mod notebook_closer;

pub(crate) mod middleware;
pub(crate) mod notary_metrics;
pub(crate) mod rpc_metrics;
pub mod s3_archive;
pub mod server;
