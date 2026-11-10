use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::convert::{TryFrom, TryInto};
use frame_support::pallet_prelude::TypeInfo;
use polkadot_sdk::*;
use serde::{Deserialize, Serialize};
use sp_arithmetic::{helpers_128bit::multiply_by_rational_with_rounding, Rounding};
use sp_core::{ConstU32, H256};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Saturating, Zero},
	BoundedVec, FixedPointNumber, FixedU128,
};

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, TypeInfo, Debug)]
pub struct BitcoinSyncStatus {
	pub confirmed_block: BitcoinBlock,
	pub synched_block: Option<BitcoinBlock>,
	pub oldest_allowed_block_height: BitcoinHeight,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, TypeInfo, Debug, MaxEncodedLen,
)]
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

#[derive(Debug, Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, TypeInfo)]
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
	Debug,
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
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, MaxEncodedLen,
)]
#[repr(transparent)]
pub struct BitcoinSignature(pub BoundedVec<u8, ConstU32<74>>); // max DER sig size is 73 bytes + 1 byte sighash type

impl TryFrom<Vec<u8>> for BitcoinSignature {
	type Error = Vec<u8>;
	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(Self(value.try_into()?))
	}
}

/// A Script Pubkey for a Bitcoin UTXO. Supported types are:
/// - P2WSH (Pay to Witness Script Hash)
#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	Debug,
	Copy,
	MaxEncodedLen,
)]
#[repr(transparent)]
pub enum BitcoinCosignScriptPubkey {
	/// Pay to Witness Script Hash
	P2WSH { wscript_hash: H256 },
}

#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, MaxEncodedLen,
)]
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
	Debug,
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug)]
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
	Debug,
	Ord,
	PartialOrd,
	MaxEncodedLen,
)]
pub struct UtxoRef {
	/// The transaction id in bitcoin little-endian format
	pub txid: H256Le,
	/// The output index in the transaction
	#[codec(compact)]
	pub output_index: u32,
}

pub type BitcoinLockId = u64;
pub type FissionId = u64;
pub type LiquidId = u64;

/// Existing redemption-curve ARGON cost per target-valued microgon of Bitcoin.
/// `r` is ARGON's market USD price divided by its target USD price. Multiply the target-valued
/// Bitcoin basis by this rate to obtain the ARGON burn. A zero ratio has no finite quote.
pub fn redemption_rate(r: FixedU128) -> Option<FixedU128> {
	// At or above target (r >= 1): no penalty; unlock cost equals the Bitcoin basis.
	if r >= FixedU128::one() {
		return Some(FixedU128::one());
	}

	// Mild deviation (0.90 <= r < 1): the quadratic curve provides the near-target discount.
	// Rate: 20r² - 38r + 19.
	if r >= FixedU128::from_rational(9, 10) {
		const FX_20: FixedU128 = FixedU128::from_u32(20);
		const FX_38: FixedU128 = FixedU128::from_u32(38);
		const FX_19: FixedU128 = FixedU128::from_u32(19);
		return ((FX_20 * r.saturating_pow(2)) + FX_19).checked_sub(&(FX_38 * r));
	}

	// Moderate deviation (0.01 <= r < 0.90): require more ARGON per unit of Bitcoin value.
	// Rate: (0.5618r + 0.3944) / r.
	if r >= FixedU128::from_rational(1, 100) {
		const FX_0_5618: FixedU128 = FixedU128::from_rational(5618, 10000);
		const FX_0_3944: FixedU128 = FixedU128::from_rational(3944, 10000);
		return ((FX_0_5618 * r) + FX_0_3944).checked_div(&r);
	}

	// Extreme deviation (r < 0.01): use the steeper burn schedule.
	// Rate: (1 / r) * (0.576r + 0.4). Preserve this evaluation order and its fixed-point rounding.
	const FX_0_576: FixedU128 = FixedU128::from_rational(576, 1000);
	const FX_0_4: FixedU128 = FixedU128::from_rational(4, 10);
	Some(FixedU128::one().checked_div(&r)?.saturating_mul((FX_0_576 * r) + FX_0_4))
}

/// Frozen purchase valuation and curve rates for one deferred Liquid repayment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiquidRepayPrices {
	/// Target-valued microgons per whole BTC at purchase, retaining fractional microgons.
	microgons_at_target_per_btc: FixedU128,
	/// Purchased microgons committed per target-valued BTC microgon; selects the Bitcoin amount.
	commitment_rate: FixedU128,
	/// Microgons actually burned per target-valued BTC microgon after recovery; never exceeds
	/// the commitment rate. The remainder returns to the buyer.
	burn_rate: FixedU128,
}

impl LiquidRepayPrices {
	pub fn new(
		microgons_at_target_per_btc: FixedU128,
		purchase_r_value: FixedU128,
		r_value_high_water: FixedU128,
	) -> Option<Self> {
		if microgons_at_target_per_btc.is_zero() {
			return None;
		}

		// Capture the redemption rates at purchase and at the recovery high-water mark.
		let purchase_redemption_rate = redemption_rate(purchase_r_value)?;
		let high_water_redemption_rate = redemption_rate(r_value_high_water.max(purchase_r_value))?;
		let at_target_rate = FixedU128::one();

		// Preserve the captured curve discount's market value, expressed in ARGON at target.
		let discount_at_target = at_target_rate
			.saturating_sub(purchase_redemption_rate)
			.checked_mul(&purchase_r_value)?;
		let nonrefundable_burn_rate = at_target_rate.checked_sub(&discount_at_target)?;

		// Defer any curve cost above the at-target burn; recovery can reduce that portion.
		let deferred_burn_rate = purchase_redemption_rate.saturating_sub(at_target_rate);
		let remaining_deferred_rate = high_water_redemption_rate
			.saturating_sub(at_target_rate)
			.min(deferred_burn_rate);

		// Keep the full commitment fixed while reducing the burn by the recovered premium.
		let commitment_rate = nonrefundable_burn_rate.checked_add(&deferred_burn_rate)?;
		let burn_rate = nonrefundable_burn_rate.checked_add(&remaining_deferred_rate)?;
		Some(Self { microgons_at_target_per_btc, commitment_rate, burn_rate })
	}

	/// Whether recovery can still remove an above-target portion of this burn.
	pub fn has_refundable_premium(&self) -> bool {
		self.burn_rate > FixedU128::one()
	}

	/// Return committed and burned microgons for the same Bitcoin amount. Match ordinary
	/// redemption's original Fission price cap and round the BTC basis before the curve rates.
	pub fn quote(
		&self,
		satoshis: Satoshis,
		original_microgons_per_btc: u128,
	) -> Option<(u128, u128)> {
		// Value the repaid Bitcoin at both the purchase price and the original Fission price.
		let purchase_btc_value = multiply_by_rational_with_rounding(
			self.microgons_at_target_per_btc.into_inner(),
			u128::from(satoshis),
			FixedU128::DIV.checked_mul(u128::from(SATOSHIS_PER_BITCOIN))?,
			Rounding::Down,
		)?;
		let original_btc_value = multiply_by_rational_with_rounding(
			original_microgons_per_btc,
			u128::from(satoshis),
			u128::from(SATOSHIS_PER_BITCOIN),
			Rounding::Down,
		)?;

		// Apply both rates to the same capped Bitcoin value.
		let redemption_basis = purchase_btc_value.min(original_btc_value);
		Some((
			self.commitment_rate.checked_mul_int(redemption_basis)?,
			self.burn_rate.checked_mul_int(redemption_basis)?,
		))
	}
}

#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct UtxoAddress {
	/// Tracking id for the BitcoinLock used across Bitcoin pallets
	pub lock_id: BitcoinLockId,
	/// The script pubkey of the Lock UTXOs
	pub script_pubkey: BitcoinCosignScriptPubkey,
	/// The Bitcoin height from which the node must scan for outputs to this script.
	#[codec(compact)]
	pub submitted_at_height: BitcoinHeight,
}

/// Legacy runtime-API representation retained for `BitcoinApis::active_utxos` compatibility.
#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct UtxoValue {
	pub lock_id: BitcoinLockId,
	pub script_pubkey: BitcoinCosignScriptPubkey,
	#[codec(compact)]
	pub satoshis: Satoshis,
	#[codec(compact)]
	pub submitted_at_height: BitcoinHeight,
	#[codec(compact)]
	pub watch_for_spent_until_height: BitcoinHeight,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub enum BitcoinRejectedReason {
	/// The UTXO has satoshis is outside the accepted range
	SatoshisOutsideAcceptedRange,
	/// The UTXO has been spent
	Spent,
	/// We failed to verify the utxo before the expiration lookup time
	VerificationExpired,
	/// This UTXO is already funded/verified under a different utxo
	AlreadyVerified,
}

#[derive(Clone, PartialEq, Eq, Debug)]
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
	Debug,
	Default,
	MaxEncodedLen,
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
	Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, Debug,
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
		hashes::{FromSliceError, Hash},
		Network,
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

		/// Derives a child public key from the current xpub at the given index.
		/// NOTE: this uses unhardened derivation because it's in the public blockchain and hardened
		/// derivation requires a private key. This does mean that if a single private key for a
		/// vault is compromised, all the other xprivs derived from it can be compromised as well.
		/// TODO: when we switch to taproot, we can use hardened derivation for the vault xpub
		/// equivalent.
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
pub fn get_rounded_up_bitcoin_day_height(block_height: BitcoinHeight) -> BitcoinHeight {
	if block_height.is_multiple_of(144) {
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
	Debug,
	TypeInfo,
	Ord,
	PartialOrd,
	MaxEncodedLen,
)]
#[repr(transparent)]
pub struct H256Le(pub [u8; 32]);

#[cfg(test)]
mod tests {
	use super::*;
	use crate::MICROGONS_PER_ARGON;

	#[test]
	fn repayment_refunds_curve_premium_using_recovery_without_spending_it_on_more_bitcoin() {
		let basis = 100_000 * MICROGONS_PER_ARGON;
		for (purchase_r, high_r, committed, burned) in [
			(50, 50, 135_060, 135_060),
			(50, 80, 135_060, 105_480),
			(50, 100, 135_060, 100_000),
			(50, 95, 135_060, 100_000),
			(80, 85, 105_480, 102_580),
			(80, 70, 105_480, 105_480),
			(100, 100, 100_000, 100_000),
		] {
			let prices = LiquidRepayPrices::new(
				FixedU128::saturating_from_integer(basis),
				FixedU128::from_rational(purchase_r, 100),
				FixedU128::from_rational(high_r, 100),
			)
			.unwrap();
			assert_eq!(
				prices.quote(SATOSHIS_PER_BITCOIN, basis),
				Some((committed * MICROGONS_PER_ARGON, burned * MICROGONS_PER_ARGON))
			);
			assert_eq!(prices.has_refundable_premium(), burned > 100_000);
		}
	}

	#[test]
	fn captured_curve_discount_keeps_its_market_value_in_target_argons() {
		let basis = 100_000 * MICROGONS_PER_ARGON;
		for (purchase_r, ordinary, guaranteed) in [(95, 95_000, 95_250), (98, 96_800, 96_864)] {
			let r = FixedU128::from_rational(purchase_r, 100);
			assert_eq!(
				redemption_rate(r).unwrap().checked_mul_int(basis),
				Some(ordinary * MICROGONS_PER_ARGON)
			);
			for high in [r, FixedU128::one()] {
				let prices =
					LiquidRepayPrices::new(FixedU128::saturating_from_integer(basis), r, high)
						.unwrap();
				assert_eq!(
					prices.quote(SATOSHIS_PER_BITCOIN, basis),
					Some((guaranteed * MICROGONS_PER_ARGON, guaranteed * MICROGONS_PER_ARGON))
				);
			}
		}
	}

	#[test]
	fn repayment_quotes_use_the_captured_btc_value_and_original_fission_cap() {
		let basis = 100_000 * MICROGONS_PER_ARGON;
		let prices = LiquidRepayPrices::new(
			FixedU128::saturating_from_integer(basis),
			FixedU128::from_rational(1, 2),
			FixedU128::one(),
		)
		.unwrap();
		assert_eq!(
			prices.quote(SATOSHIS_PER_BITCOIN, basis * 2),
			Some((135_060 * MICROGONS_PER_ARGON, basis))
		);
		assert_eq!(
			prices.quote(SATOSHIS_PER_BITCOIN, basis / 2),
			Some((67_530 * MICROGONS_PER_ARGON, basis / 2))
		);
		assert_eq!(prices.quote(1, basis), Some((1_350, 1_000)));
		assert!(
			LiquidRepayPrices::new(FixedU128::one(), FixedU128::zero(), FixedU128::one()).is_none()
		);
	}
}
