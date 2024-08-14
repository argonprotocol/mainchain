use argon_node_runtime::AccountId;
use clap::{Parser, ValueEnum};
use sc_cli::RunCmd;

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: ArgonRunCmd,

	/// Bitcoin node to verify minted bitcoins using compact filters. Should be a hosted/trusted
	/// full node. Include optional auth inline
	#[arg(long)]
	pub bitcoin_rpc_url: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct ArgonRunCmd {
	#[clap(flatten)]
	pub inner: RunCmd,

	/// Enable an account to author blocks
	///
	/// The account address must be given in SS58 format.
	#[arg(long, value_name = "SS58_ADDRESS", value_parser = parse_ss58_account_id)]
	pub author: Option<AccountId>,

	/// How many permissionless compute mining threads to run
	///
	/// NOTE: once mining slots are active, compute miners are given lower priority for block
	/// acceptance.
	#[arg(long, verbatim_doc_comment)]
	pub compute_miners: Option<u32>,

	/// Flags to control the randomx compute challenge. Can be specified multiple times.
	/// - LargePages: use large memory pages for the randomx dataset (default active)
	/// - Secure: use secure memory for the randomx dataset (default active)
	#[arg(long, verbatim_doc_comment)]
	pub randomx_flags: Vec<RandomxFlag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RandomxFlag {
	LargePages,
	Secure,
}

impl Cli {
	pub fn block_author(&self) -> Option<AccountId> {
		if let Some(block_author) = &self.run.author {
			Some(block_author.clone())
		} else if let Some(account) = self.run.inner.get_keyring() {
			Some(account.to_account_id())
		} else if self.run.inner.shared_params.dev {
			use sp_core::crypto::Pair;
			let block_author = sp_core::sr25519::Pair::from_string("//Alice", None).unwrap();
			Some(AccountId::from(block_author.public()))
		} else {
			None
		}
	}
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

	/// Sub-commands concerned with benchmarking.
	#[command(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),

	/// Db meta columns information.
	ChainInfo(sc_cli::ChainInfoCmd),
}

fn parse_ss58_account_id(data: &str) -> Result<AccountId, String> {
	sp_core::crypto::Ss58Codec::from_ss58check(data).map_err(|err| format!("{:?}", err))
}
