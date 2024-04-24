pub use ulx_notary_apis::{create_client, Client};

pub mod apis {
	pub use ulx_notary_apis::{localchain, notebook};
}
pub use error::Error;
pub use server::NotaryServer;
pub use ulx_primitives::ensure;

pub mod error;
pub mod stores;

pub mod block_watch;

pub mod notebook_closer;

pub mod server;
