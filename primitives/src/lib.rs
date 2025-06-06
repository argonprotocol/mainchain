#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

use core::fmt::Display;
use polkadot_sdk::*;
use serde::{Deserialize, Serialize};
use sp_core::{H256, crypto::Ss58AddressFormatRegistry};
use sp_runtime::{
	MultiSignature,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
};

pub use account::{AccountType, LocalchainAccountId};
pub use balance_change::*;
pub use block_seal::{BLOCK_SEAL_KEY_TYPE, BlockSealAuthorityId, BlockSealAuthoritySignature};
pub use block_vote::*;
#[cfg(feature = "std")]
use core::str::FromStr;
pub use digests::{AUTHOR_DIGEST_ID, BLOCK_SEAL_DIGEST_ID, BlockSealDigest, *};
pub use domain::*;
pub use domain_top_level::DomainTopLevel;
#[cfg(feature = "std")]
pub use keystore_helper::*;

pub use crate::{apis::*, notary::NotaryId, note::*, notebook::*, providers::*};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub mod apis;
pub mod balance_change;
pub mod block_seal;
pub mod block_vote;
pub mod digests;
mod domain;
pub mod domain_top_level;
pub mod host;
pub mod inherents;
pub mod macros;
pub mod notary;
pub mod vault;

pub mod account;
pub mod bitcoin;
pub mod note;
pub mod notebook;
pub mod providers;
pub mod tick;

pub mod argon_utils;

#[cfg(feature = "std")]
pub mod keystore_helper;

pub mod fork_power;
#[cfg(feature = "std")]
pub mod git_version;

/// An index to a block.
pub type BlockNumber = u32;

pub use block_seal::ComputeDifficulty;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

#[cfg_attr(feature = "napi", napi_derive::napi)]
pub const ADDRESS_PREFIX: u16 = Ss58AddressFormatRegistry::SubstrateAccount as u16;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A timestamp: milliseconds since the unix epoch.
pub type Moment = u64;

pub type VaultId = u32;

/// A hash of some data used by the chain.
pub type HashOutput = H256;
pub type BlockHash = BlakeTwo256;

pub const TOKEN_DECIMALS: u8 = 6;
pub const MICROGONS_PER_ARGON: u128 = 1_000_000;
pub const ARGON_TOKEN_SYMBOL: &str = "ARGN";
pub const OWNERSHIP_TOKEN_SYMBOL: &str = "ARGNOT";

pub mod prelude {
	pub use super::{
		AccountId, Balance, BlockNumber, Chain, ChainIdentity, Moment, Nonce, NotaryId,
		NotebookNumber, VaultId,
		block_seal::{BlockSealAuthorityId, FrameId},
		localchain::*,
		tick::Tick,
	};
	pub use polkadot_sdk::*;
}
pub mod localchain {
	pub use crate::{
		AccountType, BalanceChange, BestBlockVoteSeal, BlockVote, BlockVoteT, Note, NoteType,
		VoteMinimum,
	};
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "std", derive(clap::ValueEnum))]
#[cfg_attr(not(feature = "napi"), derive(Clone, Copy))]
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum = "kebab-case"))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum Chain {
	Mainnet,
	Testnet,
	LocalTestnet,
	Devnet,
}

#[cfg(feature = "std")]
impl FromStr for Chain {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"Argon" => Ok(Chain::Mainnet),
			"Argon Testnet" => Ok(Chain::Testnet),
			"Argon Local Testnet" => Ok(Chain::LocalTestnet),
			"Argon Development" => Ok(Chain::Devnet),
			_ => Err("Unknown chain".to_string()),
		}
	}
}

#[cfg(feature = "std")]
impl TryFrom<String> for Chain {
	type Error = String;

	fn try_from(s: String) -> Result<Self, Self::Error> {
		Chain::from_str(&s)
	}
}

impl Display for Chain {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		match self {
			Chain::Mainnet => write!(f, "Argon"),
			Chain::Testnet => write!(f, "Argon Testnet"),
			Chain::LocalTestnet => write!(f, "Argon Local Testnet"),
			Chain::Devnet => write!(f, "Argon Development"),
		}
	}
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ChainIdentity {
	pub chain: Chain,
	pub genesis_hash: H256,
}
