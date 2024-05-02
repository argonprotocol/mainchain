use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::TypeInfo, Deserialize, Serialize};
use sp_core::ConstU32;
use sp_debug_derive::RuntimeDebug;
use sp_runtime::BoundedVec;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, Ord, PartialOrd, Eq, PartialEq)]
pub struct BitcoinUtxo {
	pub txid: H256Le,
	#[codec(compact)]
	pub output_index: u32,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebug)]
pub struct BitcoinSyncStatus {
	pub confirmed_block: BitcoinBlock,
	pub synched_block: Option<BitcoinBlock>,
	pub oldest_allowed_block_height: BitcoinHeight,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebug)]
pub struct BitcoinBlock {
	#[codec(compact)]
	pub block_height: BitcoinHeight,
	pub block_hash: BitcoinBlockHash,
}

/// A Script Pubkey for a Bitcoin UTXO. Supported types are:
/// - P2PKH (Pay to Public Key Hash)
/// - P2SH (Pay to Script Hash)
/// - P2WPKH (Pay to Witness Public Key Hash)
/// - P2WSH (Pay to Witness Script Hash)
pub type BitcoinScriptPubkey = BoundedVec<u8, ConstU32<35>>;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[codec(mel_bound(Balance: MaxEncodedLen, BlockNumber: MaxEncodedLen))]
pub struct LockedUtxo<AccountId, BondId, Balance, BlockNumber> {
	pub account_id: AccountId,
	#[codec(compact)]
	pub bond_id: BondId,
	#[codec(compact)]
	pub lock_price: Balance,
	#[codec(compact)]
	pub satoshis: Satoshis,
	#[codec(compact)]
	pub confirmed_height: BitcoinHeight,
	pub script_pubkey: BitcoinScriptPubkey,
	#[codec(compact)]
	pub expiration_block: BlockNumber,
}

/// Bitcoin peer network.
#[derive(Debug, Copy, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(clap::ValueEnum))]
pub enum BitcoinNetwork {
	/// Bitcoin Mainnet.
	Mainnet,
	/// Bitcoin Testnet.
	Testnet,
	/// Bitcoin regression test net.
	Regtest,
	/// Bitcoin signet.
	Signet,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UtxoLookup {
	pub script_pubkey: BitcoinScriptPubkey,
	pub pending_confirmation: Option<(Satoshis, BitcoinHeight)>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum BitcoinRejectedReason {
	/// The UTXO has a different number of satoshis than what is in the blockchain
	SatoshisMismatch,
	/// The script pubkey of the UTXO does not match the txid and vout
	ScriptPubkeyMismatch,
	/// The UTXO has been spent
	Spent,
	/// The UTXO is too old to be considered valid. This is a security measure to prevent
	/// claiming unowned bitcoins by watching logs
	TooOld,
}

pub type BitcoinBlockHash = H256Le;
pub type BitcoinHeight = u64;
pub type Satoshis = u64;

pub const SATOSHIS_PER_BITCOIN: u64 = 100_000_000;

/// Represents a bitcoin 32 bytes hash digest encoded in little-endian
#[derive(
	Serialize,
	Deserialize,
	Encode,
	Decode,
	Ord,
	PartialOrd,
	Default,
	PartialEq,
	Eq,
	Clone,
	Copy,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
#[repr(transparent)]
pub struct H256Le(pub [u8; 32]);
