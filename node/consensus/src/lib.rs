use sp_runtime::traits::{Block as BlockT, Header as HeaderT, UniqueSaturatedInto};

pub use block_creator::{notary_client_task, tax_block_creator};

#[cfg(test)]
mod tests;

pub mod aux;
pub mod basic_queue;
mod basic_queue_import;
mod block_creator;
mod compute_solver;
pub mod compute_worker;
mod digests;
pub mod error;
pub mod import_queue;
mod metrics;
mod notary_client;
pub mod notebook_watch;

const LOG_TARGET: &str = "node::consensus";

pub(crate) fn convert_u32<Block: BlockT>(number: &<Block::Header as HeaderT>::Number) -> u32 {
	UniqueSaturatedInto::<u32>::unique_saturated_into(number.clone())
}
