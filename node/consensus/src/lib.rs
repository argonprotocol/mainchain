use sp_runtime::traits::{Block as BlockT, Header as HeaderT, UniqueSaturatedInto};

pub use block_creator::{notary_client_task, tax_block_creator};

#[cfg(test)]
mod tests;

pub mod aux_client;
mod aux_data;
mod block_creator;
mod compute_solver;
pub mod compute_worker;
mod digests;
pub mod error;
pub mod import_queue;
mod notary_client;
pub mod notebook_watch;

const LOG_TARGET: &str = "node::consensus";
