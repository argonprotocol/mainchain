use sp_runtime::traits::{Block as BlockT, Header as HeaderT, UniqueSaturatedInto};

pub use block_creator::{create_block_watch, tax_block_creator};

#[cfg(test)]
mod tests;

pub mod authority;
mod aux;
pub mod basic_queue;
mod basic_queue_import;
mod block_creator;
pub mod compute_worker;
mod digests;
pub mod error;
pub mod import_queue;
mod metrics;
pub mod notebook_watch;
pub mod rpc_block_votes;

const LOG_TARGET: &str = "node::consensus";

pub(crate) fn convert_u32<Block: BlockT>(number: &<Block::Header as HeaderT>::Number) -> u32 {
	UniqueSaturatedInto::<u32>::unique_saturated_into(number.clone())
}
