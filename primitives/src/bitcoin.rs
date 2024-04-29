use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::TypeInfo, Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, Ord, PartialOrd, Eq, PartialEq)]
pub struct BitcoinUtxoId {
	pub txid: H256Le,
	pub output_index: u32,
}

pub type Satoshis = u64;

pub const SATOSHIS_PER_BITCOIN: u64 = 100_000_000;

/// Compressed ECDSA (secp256k1 curve) Public Key
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
#[repr(transparent)]
pub struct CompressedPublicKey(pub [u8; 33]);

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
