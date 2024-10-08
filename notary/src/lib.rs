pub use argon_notary_apis::{create_client, Client};

pub mod apis {
	pub use argon_notary_apis::{localchain, notebook};
}
pub use argon_notary_apis::error::Error;
pub use argon_primitives::ensure;
pub use server::NotaryServer;

pub mod stores;

pub mod block_watch;

pub mod notebook_closer;

pub mod server;
