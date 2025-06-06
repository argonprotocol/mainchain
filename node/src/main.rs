//! Argon node implementation.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;

mod cli;
mod command;
// mod grandpa_set_id_patch;
mod rpc;
pub(crate) mod runtime_api;

fn main() -> polkadot_sdk::sc_cli::Result<()> {
	command::run()
}
