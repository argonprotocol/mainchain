use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::convert::{TryFrom, TryInto};
use frame_support::pallet_prelude::TypeInfo;
use polkadot_sdk::*;
use serde::{Deserialize, Serialize};
use sp_core::{ConstU32, H256};
use sp_debug_derive::RuntimeDebug;
use sp_runtime::BoundedVec;

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, TypeInfo, RuntimeDebug)]
pub struct BitcoinSyncStatus {
	pub confirmed_block: BitcoinBlock,
	pub synched_block: Option<BitcoinBlock>,
	pub oldest_allowed_block_height: BitcoinHeight,
}

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, TypeInfo, RuntimeDebug)]
pub struct BitcoinBlock {
	#[codec(compact)]
	pub block_height: BitcoinHeight,
	pub block_hash: BitcoinBlockHash,
}

impl BitcoinBlock {
	pub fn new(block_height: BitcoinHeight, block_hash: BitcoinBlockHash) -> Self {
		Self { block_height, block_hash }
	}
}

#[derive(RuntimeDebug, Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, TypeInfo)]
pub enum BitcoinError {
	InvalidLockTime,
	InvalidByteLength,
	InvalidPolicy,
	UnsafePolicy,
	InvalidPubkey,
}

#[derive(
	Clone,
	Copy,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	RuntimeDebug,
	MaxEncodedLen,
)]
#[repr(transparent)]
pub struct CompressedBitcoinPubkey(pub [u8; 33]);
impl From<[u8; 33]> for CompressedBitcoinPubkey {
	fn from(value: [u8; 33]) -> Self {
		Self(value)
	}
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	RuntimeDebug,
	MaxEncodedLen,
)]
#[repr(transparent)]
pub struct BitcoinSignature(pub BoundedVec<u8, ConstU32<73>>);

impl TryFrom<Vec<u8>> for BitcoinSignature {
	type Error = Vec<u8>;
	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(Self(value.try_into()?))
	}
}

/// A Script Pubkey for a Bitcoin UTXO. Supported types are:
/// - P2WSH (Pay to Witness Script Hash)
#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebug, Copy,
)]
#[repr(transparent)]
pub enum BitcoinCosignScriptPubkey {
	/// Pay to Witness Script Hash
	P2WSH { wscript_hash: H256 },
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebug)]
#[repr(transparent)]
pub struct BitcoinScriptPubkey(pub BoundedVec<u8, ConstU32<34>>); // allow p2wsh, p2tr max
impl TryFrom<Vec<u8>> for BitcoinScriptPubkey {
	type Error = Vec<u8>;
	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(Self(value.try_into()?))
	}
}
#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	RuntimeDebug,
	Copy,
	MaxEncodedLen,
)]
#[repr(transparent)]
pub struct OpaqueBitcoinXpub(pub [u8; 78]);
impl TryFrom<Vec<u8>> for OpaqueBitcoinXpub {
	type Error = Vec<u8>;
	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(Self(value.try_into()?))
	}
}

/// A Bitcoin sighash. This is a 32-byte hash that is used to sign a Bitcoin transaction.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebug)]
#[repr(transparent)]
pub struct BitcoinSighash(pub [u8; 32]);

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	TypeInfo,
	RuntimeDebug,
	Ord,
	PartialOrd,
)]
pub struct UtxoRef {
	pub txid: H256Le,
	#[codec(compact)]
	pub output_index: u32,
}

pub type UtxoId = u64;
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub struct UtxoValue {
	pub utxo_id: UtxoId,
	pub script_pubkey: BitcoinCosignScriptPubkey,
	#[codec(compact)]
	pub satoshis: Satoshis,
	#[codec(compact)]
	pub submitted_at_height: BitcoinHeight,
	#[codec(compact)]
	pub watch_for_spent_until_height: BitcoinHeight,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub enum BitcoinRejectedReason {
	/// The UTXO has a different number of satoshis than what is in the blockchain
	SatoshisMismatch,
	/// The UTXO has been spent
	Spent,
	/// We failed to confirm the utxo before the expiration lookup time
	LookupExpired,
	/// This UTXO is already tracked
	DuplicateUtxo,
}

#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub enum XpubErrors {
	InvalidXpubkey,
	InvalidXpubkeyChild,
	BitcoinConversionFailed,
	WrongExtendedKeyLength(usize),
	UnknownVersion([u8; 4]),
	WrongNetwork,
	DecodeFingerprintError,
	DecodeChildNumberError,
	DecodeChainCodeError,
	DecodePubkeyError,
}

pub type BitcoinBlockHash = H256Le;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
)]
pub enum NetworkKind {
	/// The Bitcoin mainnet network.
	Main,
	/// Some kind of testnet network.
	Test,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	Serialize,
	Deserialize,
	Clone,
	Copy,
	PartialEq,
	Eq,
	RuntimeDebug,
	Default,
)]
#[cfg_attr(all(feature = "std", not(feature = "uniffi")), derive(clap::ValueEnum))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub enum BitcoinNetwork {
	/// Mainnet Bitcoin.
	Bitcoin,
	/// Bitcoin's testnet network.
	Testnet,
	/// Bitcoin's signet network
	Signet,
	/// Bitcoin's regtest network.
	#[default]
	Regtest,
}

pub type XPubFingerprint = [u8; 4];
pub type XPubChildNumber = u32;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	TypeInfo,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
)]
pub struct BitcoinXPub {
	pub public_key: CompressedBitcoinPubkey,
	/// Depth in the key derivation hierarchy.
	#[codec(compact)]
	pub depth: u8,

	/// Parent fingerprint.
	pub parent_fingerprint: XPubFingerprint,

	/// Child number.
	#[codec(compact)]
	pub child_number: XPubChildNumber,

	/// Chain code.
	pub chain_code: [u8; 32],
	pub network: NetworkKind,
}

impl BitcoinXPub {
	pub fn matches_network(&self, network: BitcoinNetwork) -> bool {
		match network {
			BitcoinNetwork::Bitcoin => self.network == NetworkKind::Main,
			BitcoinNetwork::Testnet => self.network == NetworkKind::Test,
			BitcoinNetwork::Signet => self.network == NetworkKind::Test,
			BitcoinNetwork::Regtest => self.network == NetworkKind::Test,
		}
	}
}

#[cfg(feature = "bitcoin")]
mod bitcoin_compat {
	use alloc::vec::Vec;
	use polkadot_sdk::*;

	use crate::bitcoin::{
		BitcoinCosignScriptPubkey, BitcoinNetwork, BitcoinScriptPubkey, BitcoinSignature,
		BitcoinXPub, CompressedBitcoinPubkey, H256Le, NetworkKind, OpaqueBitcoinXpub, UtxoRef,
		XpubErrors,
	};
	use ::bip32::secp256k1::ecdsa::VerifyingKey;
	use bip32::{ChildNumber, ExtendedKeyAttrs, XPub};
	use bitcoin::{
		Network,
		hashes::{FromSliceError, Hash},
	};
	use sp_core::H256;
	use sp_runtime::BoundedVec;

	/// Version bytes for extended public keys on the Bitcoin network.
	const VERSION_BYTES_MAINNET_PUBLIC: [u8; 4] = [0x04, 0x88, 0xB2, 0x1E];
	/// Version bytes for extended public keys on any of the testnet networks.
	const VERSION_BYTES_TESTNETS_PUBLIC: [u8; 4] = [0x04, 0x35, 0x87, 0xCF];

	const VERSION_BYTES_MAINNET_ZPUB: [u8; 4] = [0x04, 0xB2, 0x47, 0x46];
	const VERSION_BYTES_TESTNETS_ZPUB: [u8; 4] = [0x04, 0x5F, 0x1C, 0xF6];

	impl BitcoinXPub {
		pub fn get_xpub(&self) -> Result<XPub, XpubErrors> {
			let attrs = ExtendedKeyAttrs {
				depth: self.depth,
				parent_fingerprint: self.parent_fingerprint,
				child_number: ChildNumber::from(self.child_number),
				chain_code: self.chain_code,
			};
			let pubkey = self.public_key;
			let verifying_key = VerifyingKey::from_sec1_bytes(&pubkey.0)
				.map_err(|_| XpubErrors::BitcoinConversionFailed)?;
			Ok(XPub::new(verifying_key, attrs))
		}

		pub fn is_hardened(&self) -> bool {
			let child_number = ChildNumber::from(self.child_number);
			child_number.is_hardened()
		}

		pub fn derive_pubkey(&self, index: u32) -> Result<BitcoinXPub, XpubErrors> {
			let child_number =
				ChildNumber::new(index, false).map_err(|_| XpubErrors::InvalidXpubkeyChild)?;
			let xpub_child = self
				.get_xpub()?
				.derive_child(child_number)
				.map_err(|_| XpubErrors::InvalidXpubkeyChild)?;

			Ok(BitcoinXPub {
				public_key: CompressedBitcoinPubkey(xpub_child.to_bytes()),
				depth: xpub_child.attrs().depth,
				parent_fingerprint: xpub_child.attrs().parent_fingerprint,
				child_number: u32::from(xpub_child.attrs().child_number),
				chain_code: xpub_child.attrs().chain_code,
				network: self.network,
			})
		}
	}

	impl TryFrom<OpaqueBitcoinXpub> for BitcoinXPub {
		type Error = XpubErrors;
		fn try_from(xpub: OpaqueBitcoinXpub) -> Result<Self, XpubErrors> {
			let data = xpub.0;

			let network = if data.starts_with(&VERSION_BYTES_MAINNET_PUBLIC) ||
				data.starts_with(&VERSION_BYTES_MAINNET_ZPUB)
			{
				NetworkKind::Main
			} else if data.starts_with(&VERSION_BYTES_TESTNETS_PUBLIC) ||
				data.starts_with(&VERSION_BYTES_TESTNETS_ZPUB)
			{
				NetworkKind::Test
			} else {
				let (b0, b1, b2, b3) = (data[0], data[1], data[2], data[3]);
				return Err(XpubErrors::UnknownVersion([b0, b1, b2, b3]));
			};

			let attrs = ExtendedKeyAttrs {
				depth: data[4],
				parent_fingerprint: data[5..9]
					.try_into()
					.map_err(|_| XpubErrors::DecodeFingerprintError)?,
				child_number: u32::from_be_bytes(
					data[9..13].try_into().map_err(|_| XpubErrors::DecodeChildNumberError)?,
				)
				.into(),
				chain_code: data[13..45]
					.try_into()
					.map_err(|_| XpubErrors::DecodeChainCodeError)?,
			};
			let pubkey = data[45..78].try_into().map_err(|_| XpubErrors::DecodePubkeyError)?;

			Ok(Self {
				network,
				depth: attrs.depth,
				parent_fingerprint: attrs.parent_fingerprint,
				child_number: u32::from(attrs.child_number),
				chain_code: attrs.chain_code,
				public_key: CompressedBitcoinPubkey(pubkey),
			})
		}
	}

	impl From<bitcoin::BlockHash> for H256Le {
		fn from(h: bitcoin::BlockHash) -> Self {
			let mut inner = [0u8; 32];
			inner.copy_from_slice(&h[..]);
			H256Le(inner)
		}
	}

	impl TryInto<bitcoin::BlockHash> for H256Le {
		type Error = FromSliceError;
		fn try_into(self) -> Result<bitcoin::BlockHash, Self::Error> {
			bitcoin::BlockHash::from_slice(&self.0)
		}
	}

	impl From<&bitcoin::BlockHash> for H256Le {
		fn from(h: &bitcoin::BlockHash) -> Self {
			let mut inner = [0u8; 32];
			inner.copy_from_slice(&h[..]);
			H256Le(inner)
		}
	}

	impl From<bitcoin::Txid> for H256Le {
		fn from(h: bitcoin::Txid) -> Self {
			let mut inner = [0u8; 32];
			inner.copy_from_slice(&h[..]);
			H256Le(inner)
		}
	}

	impl From<H256Le> for bitcoin::Txid {
		fn from(h: H256Le) -> Self {
			let hash = bitcoin::hashes::sha256d::Hash::from_bytes_ref(&h.0);
			bitcoin::Txid::from_raw_hash(*hash)
		}
	}
	impl From<Network> for BitcoinNetwork {
		fn from(network: Network) -> Self {
			match network {
				Network::Bitcoin => BitcoinNetwork::Bitcoin,
				Network::Testnet => BitcoinNetwork::Testnet,
				Network::Signet => BitcoinNetwork::Signet,
				Network::Regtest => BitcoinNetwork::Regtest,
				_ => unimplemented!(),
			}
		}
	}

	impl From<BitcoinNetwork> for bitcoin::network::NetworkKind {
		fn from(value: BitcoinNetwork) -> Self {
			match value {
				BitcoinNetwork::Bitcoin => bitcoin::network::NetworkKind::Main,
				BitcoinNetwork::Testnet | BitcoinNetwork::Signet | BitcoinNetwork::Regtest =>
					bitcoin::network::NetworkKind::Test,
			}
		}
	}

	impl From<BitcoinNetwork> for Network {
		fn from(network: BitcoinNetwork) -> Self {
			match network {
				BitcoinNetwork::Bitcoin => Network::Bitcoin,
				BitcoinNetwork::Testnet => Network::Testnet,
				BitcoinNetwork::Signet => Network::Signet,
				BitcoinNetwork::Regtest => Network::Regtest,
			}
		}
	}

	impl From<bitcoin::FilterHeader> for H256Le {
		fn from(h: bitcoin::FilterHeader) -> Self {
			let mut inner = [0u8; 32];
			inner.copy_from_slice(&h[..]);
			H256Le(inner)
		}
	}

	impl From<bitcoin::OutPoint> for UtxoRef {
		fn from(outpoint: bitcoin::OutPoint) -> Self {
			Self { txid: outpoint.txid.into(), output_index: outpoint.vout }
		}
	}

	impl From<bitcoin::bip32::Xpub> for OpaqueBitcoinXpub {
		fn from(xpub: bitcoin::bip32::Xpub) -> Self {
			OpaqueBitcoinXpub(xpub.encode())
		}
	}

	impl TryInto<bitcoin::ecdsa::Signature> for BitcoinSignature {
		type Error = bitcoin::ecdsa::Error;
		fn try_into(self) -> Result<bitcoin::ecdsa::Signature, Self::Error> {
			bitcoin::ecdsa::Signature::from_slice(self.0.as_slice())
		}
	}

	impl TryFrom<bitcoin::ecdsa::Signature> for BitcoinSignature {
		type Error = Vec<u8>;
		fn try_from(sig: bitcoin::ecdsa::Signature) -> Result<Self, Self::Error> {
			Ok(Self(sig.serialize().to_vec().try_into()?))
		}
	}

	impl TryInto<bitcoin::CompressedPublicKey> for CompressedBitcoinPubkey {
		type Error = bitcoin::secp256k1::Error;
		fn try_into(self) -> Result<bitcoin::CompressedPublicKey, Self::Error> {
			bitcoin::CompressedPublicKey::from_slice(&self.0)
		}
	}

	impl From<bitcoin::CompressedPublicKey> for CompressedBitcoinPubkey {
		fn from(pubkey: bitcoin::CompressedPublicKey) -> Self {
			Self(pubkey.to_bytes())
		}
	}

	impl From<bitcoin::PublicKey> for CompressedBitcoinPubkey {
		fn from(pubkey: bitcoin::PublicKey) -> Self {
			pubkey.inner.serialize().into()
		}
	}
	impl TryInto<bitcoin::PublicKey> for CompressedBitcoinPubkey {
		type Error = bitcoin::key::FromSliceError;
		fn try_into(self) -> Result<bitcoin::PublicKey, Self::Error> {
			bitcoin::PublicKey::from_slice(&self.0)
		}
	}

	impl BitcoinCosignScriptPubkey {
		pub fn to_script_bytes(&self) -> Vec<u8> {
			let script_buf: bitcoin::ScriptBuf = (*self).into();
			script_buf.to_bytes()
		}
	}

	#[derive(Debug)]
	pub enum BitcoinScriptPubkeyError {
		UnsupportedScript,
	}
	impl TryFrom<bitcoin::ScriptBuf> for BitcoinCosignScriptPubkey {
		type Error = BitcoinScriptPubkeyError;
		fn try_from(script: bitcoin::ScriptBuf) -> Result<Self, Self::Error> {
			if script.is_p2wsh() {
				let mut inner = [0u8; 32];
				inner.copy_from_slice(&script.as_bytes()[2..]);
				return Ok(BitcoinCosignScriptPubkey::P2WSH { wscript_hash: H256(inner) });
			}
			Err(BitcoinScriptPubkeyError::UnsupportedScript)
		}
	}

	impl From<BitcoinScriptPubkey> for bitcoin::ScriptBuf {
		fn from(val: BitcoinScriptPubkey) -> Self {
			bitcoin::ScriptBuf::from_bytes(val.0.into_inner())
		}
	}

	impl From<bitcoin::ScriptBuf> for BitcoinScriptPubkey {
		fn from(val: bitcoin::ScriptBuf) -> Self {
			Self(BoundedVec::truncate_from(val.to_bytes()))
		}
	}

	impl TryInto<BitcoinCosignScriptPubkey> for bitcoin::Address {
		type Error = BitcoinScriptPubkeyError;
		fn try_into(self) -> Result<BitcoinCosignScriptPubkey, Self::Error> {
			self.script_pubkey().try_into()
		}
	}

	impl From<BitcoinCosignScriptPubkey> for bitcoin::ScriptBuf {
		fn from(val: BitcoinCosignScriptPubkey) -> Self {
			match val {
				BitcoinCosignScriptPubkey::P2WSH { wscript_hash } => {
					let bytes = wscript_hash.to_fixed_bytes();
					let raw_hash = bitcoin::hashes::sha256::Hash::from_bytes_ref(&bytes);
					let script_hash = bitcoin::WScriptHash::from_raw_hash(*raw_hash);
					bitcoin::ScriptBuf::new_p2wsh(&script_hash)
				},
			}
		}
	}
}

/// Returns the block height of the next Bitcoin day (eg, next iteration of 144 blocks)
pub(crate) fn get_rounded_up_bitcoin_day_height(block_height: BitcoinHeight) -> BitcoinHeight {
	if block_height % 144 == 0 {
		return block_height;
	}
	block_height - (block_height % 144) + 144
}

pub type BitcoinHeight = u64;
pub type Satoshis = u64;

pub const SATOSHIS_PER_BITCOIN: u64 = 100_000_000;

/// Represents a bitcoin 32 bytes hash digest encoded in little-endian

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	Ord,
	PartialOrd,
)]
#[repr(transparent)]
pub struct H256Le(pub [u8; 32]);
