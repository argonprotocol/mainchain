use argon_primitives::AccountId;
use clap::{Parser, ValueEnum};
use polkadot_sdk::*;
use sc_cli::RunCmd;

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: ArgonRunCmd,

	/// Bitcoin node to verify minted bitcoins using compact filters. Should be a hosted/trusted
	/// full node. Include optional auth inline
	#[arg(long, global = true)]
	pub bitcoin_rpc_url: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct ArgonRunCmd {
	#[clap(flatten)]
	pub base: RunCmd,

	/// The rewards author for compute mining (if activated).
	///
	/// The account address must be given in SS58 format.
	#[arg(long, value_name = "SS58_ADDRESS", value_parser = parse_ss58_account_id)]
	pub compute_author: Option<AccountId>,

	/// How many permissionless compute mining threads to run
	///
	/// NOTE: once mining slots are active, compute miners are given lower priority for block
	/// acceptance.
	#[arg(long, verbatim_doc_comment)]
	pub compute_miners: Option<u32>,

	/// Flags to control the randomx compute challenge. Can be specified multiple times.
	/// - LargePages: use large memory pages for the randomx dataset (default inactive)
	/// - Secure: use secure memory for the randomx dataset (default inactive)
	#[arg(long, verbatim_doc_comment)]
	pub compute_flags: Vec<RandomxFlag>,

	/// The archive hosts to download notary notebooks from if a notary is unavailable. You are
	/// free to use a hosted service, or download a registry to your local machine.
	///
	/// Should be a list of base URL hosts (eg, https://archives.argonprotocol.org).
	#[arg(long, verbatim_doc_comment)]
	pub notebook_archive_hosts: Vec<String>,

	/// Maximum header download size (in MB). Downloads above this limit are rejected.
	#[arg(long, env = "ARGON_NOTEBOOK_HEADER_MAX_MB", default_value_t = 2)]
	pub notebook_header_max_mb: u64,

	/// Maximum notebook body download size (in MB). Downloads above this limit are rejected.
	#[arg(long, env = "ARGON_NOTEBOOK_BODY_MAX_MB", default_value_t = 16)]
	pub notebook_body_max_mb: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RandomxFlag {
	LargePages,
	Secure,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommand {
	/// Key management cli utilities
	#[command(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Db meta columns information.
	ChainInfo(sc_cli::ChainInfoCmd),
}

fn parse_ss58_account_id(data: &str) -> Result<AccountId, String> {
	sp_core::crypto::Ss58Codec::from_ss58check(data).map_err(|err| format!("{err:?}"))
}
