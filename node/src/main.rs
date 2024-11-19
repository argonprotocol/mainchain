//! Argon node implementation.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;

mod cli;
mod command;
mod rpc;
pub(crate) mod runtime_api;

fn main() -> sc_cli::Result<()> {
	command::run()
}
