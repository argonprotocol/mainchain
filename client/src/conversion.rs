use crate::{api::runtime_types, BlakeTwo256};
use argon_primitives::BlockNumber;
use polkadot_sdk::*;
use sp_arithmetic::{FixedU128, Permill};
use subxt::config::substrate::{DigestItem, SubstrateHeader};

impl<T, X: sp_core::Get<u32>> From<sp_core::bounded_vec::BoundedVec<T, X>>
	for runtime_types::bounded_collections::bounded_vec::BoundedVec<T>
{
	fn from(value: sp_core::bounded_vec::BoundedVec<T, X>) -> Self {
		runtime_types::bounded_collections::bounded_vec::BoundedVec(value.into())
	}
}

impl From<runtime_types::argon_primitives::bitcoin::BitcoinNetwork>
	for argon_primitives::bitcoin::BitcoinNetwork
{
	fn from(value: runtime_types::argon_primitives::bitcoin::BitcoinNetwork) -> Self {
		match value {
			runtime_types::argon_primitives::bitcoin::BitcoinNetwork::Bitcoin => Self::Bitcoin,
			runtime_types::argon_primitives::bitcoin::BitcoinNetwork::Testnet => Self::Testnet,
			runtime_types::argon_primitives::bitcoin::BitcoinNetwork::Signet => Self::Signet,
			runtime_types::argon_primitives::bitcoin::BitcoinNetwork::Regtest => Self::Regtest,
		}
	}
}
impl From<runtime_types::argon_primitives::bitcoin::BitcoinCosignScriptPubkey>
	for argon_primitives::bitcoin::BitcoinCosignScriptPubkey
{
	fn from(value: runtime_types::argon_primitives::bitcoin::BitcoinCosignScriptPubkey) -> Self {
		match value {
			runtime_types::argon_primitives::bitcoin::BitcoinCosignScriptPubkey::P2WSH {
				wscript_hash,
			} => argon_primitives::bitcoin::BitcoinCosignScriptPubkey::P2WSH {
				wscript_hash: wscript_hash.into(),
			},
		}
	}
}

impl From<argon_primitives::bitcoin::BitcoinNetwork>
	for runtime_types::argon_primitives::bitcoin::BitcoinNetwork
{
	fn from(value: argon_primitives::bitcoin::BitcoinNetwork) -> Self {
		match value {
			argon_primitives::bitcoin::BitcoinNetwork::Bitcoin => Self::Bitcoin,
			argon_primitives::bitcoin::BitcoinNetwork::Testnet => Self::Testnet,
			argon_primitives::bitcoin::BitcoinNetwork::Signet => Self::Signet,
			argon_primitives::bitcoin::BitcoinNetwork::Regtest => Self::Regtest,
		}
	}
}

impl<T> From<Vec<T>> for runtime_types::bounded_collections::bounded_vec::BoundedVec<T> {
	fn from(value: Vec<T>) -> Self {
		runtime_types::bounded_collections::bounded_vec::BoundedVec(value)
	}
}

impl<T, X: sp_core::Get<u32>>
	TryFrom<runtime_types::bounded_collections::bounded_vec::BoundedVec<T>>
	for sp_core::bounded_vec::BoundedVec<T, X>
{
	type Error = Vec<T>;
	fn try_from(
		value: runtime_types::bounded_collections::bounded_vec::BoundedVec<T>,
	) -> Result<Self, Self::Error> {
		sp_core::bounded_vec::BoundedVec::<T, X>::try_from(value.0)
	}
}

impl From<runtime_types::argon_primitives::tick::Ticker> for argon_primitives::tick::Ticker {
	fn from(value: runtime_types::argon_primitives::tick::Ticker) -> Self {
		Self::new(value.tick_duration_millis, value.channel_hold_expiration_ticks)
	}
}

// ----- bitcoin -----
impl From<runtime_types::argon_primitives::bitcoin::H256Le> for argon_primitives::bitcoin::H256Le {
	fn from(value: runtime_types::argon_primitives::bitcoin::H256Le) -> Self {
		Self(value.0)
	}
}

impl From<argon_primitives::bitcoin::H256Le> for runtime_types::argon_primitives::bitcoin::H256Le {
	fn from(value: argon_primitives::bitcoin::H256Le) -> Self {
		Self(value.0)
	}
}

impl From<runtime_types::argon_primitives::bitcoin::UtxoRef>
	for argon_primitives::bitcoin::UtxoRef
{
	fn from(value: runtime_types::argon_primitives::bitcoin::UtxoRef) -> Self {
		Self { txid: value.txid.into(), output_index: value.output_index }
	}
}

impl From<argon_primitives::bitcoin::UtxoRef>
	for runtime_types::argon_primitives::bitcoin::UtxoRef
{
	fn from(value: argon_primitives::bitcoin::UtxoRef) -> Self {
		Self { txid: value.txid.into(), output_index: value.output_index }
	}
}

impl From<runtime_types::argon_primitives::bitcoin::CompressedBitcoinPubkey>
	for argon_primitives::bitcoin::CompressedBitcoinPubkey
{
	fn from(value: runtime_types::argon_primitives::bitcoin::CompressedBitcoinPubkey) -> Self {
		Self(value.0)
	}
}

impl From<argon_primitives::bitcoin::CompressedBitcoinPubkey>
	for runtime_types::argon_primitives::bitcoin::CompressedBitcoinPubkey
{
	fn from(value: argon_primitives::bitcoin::CompressedBitcoinPubkey) -> Self {
		Self(value.0)
	}
}

impl TryFrom<runtime_types::argon_primitives::bitcoin::BitcoinSignature>
	for argon_primitives::bitcoin::BitcoinSignature
{
	type Error = Vec<u8>;
	fn try_from(
		value: runtime_types::argon_primitives::bitcoin::BitcoinSignature,
	) -> Result<Self, Self::Error> {
		value.0 .0.try_into()
	}
}

impl From<argon_primitives::bitcoin::BitcoinSignature>
	for runtime_types::argon_primitives::bitcoin::BitcoinSignature
{
	fn from(value: argon_primitives::bitcoin::BitcoinSignature) -> Self {
		Self(value.0.into())
	}
}

impl TryFrom<runtime_types::argon_primitives::bitcoin::BitcoinScriptPubkey>
	for argon_primitives::bitcoin::BitcoinScriptPubkey
{
	type Error = Vec<u8>;
	fn try_from(
		value: runtime_types::argon_primitives::bitcoin::BitcoinScriptPubkey,
	) -> Result<Self, Self::Error> {
		value.0 .0.try_into()
	}
}

impl From<argon_primitives::bitcoin::BitcoinScriptPubkey>
	for runtime_types::argon_primitives::bitcoin::BitcoinScriptPubkey
{
	fn from(value: argon_primitives::bitcoin::BitcoinScriptPubkey) -> Self {
		Self(value.0.into())
	}
}

pub trait SubxtRuntime {
	fn runtime_digest(&self) -> sp_runtime::Digest;
}

impl SubxtRuntime for SubstrateHeader<BlockNumber, BlakeTwo256> {
	fn runtime_digest(&self) -> sp_runtime::Digest {
		let logs = self
			.digest
			.logs
			.iter()
			.map(|digest_item| match digest_item {
				DigestItem::PreRuntime(a, b) => sp_runtime::DigestItem::PreRuntime(*a, b.clone()),
				DigestItem::Consensus(a, b) => sp_runtime::DigestItem::Consensus(*a, b.clone()),
				DigestItem::Seal(a, b) => sp_runtime::DigestItem::Seal(*a, b.clone()),
				DigestItem::Other(a) => sp_runtime::DigestItem::Other(a.clone()),
				DigestItem::RuntimeEnvironmentUpdated =>
					sp_runtime::DigestItem::RuntimeEnvironmentUpdated,
			})
			.collect::<Vec<_>>();
		sp_runtime::Digest { logs }
	}
}

impl From<[u8; 78]> for runtime_types::argon_primitives::bitcoin::OpaqueBitcoinXpub {
	fn from(value: [u8; 78]) -> Self {
		Self(value)
	}
}

pub fn to_api_fixed_u128(value: FixedU128) -> runtime_types::sp_arithmetic::fixed_point::FixedU128 {
	runtime_types::sp_arithmetic::fixed_point::FixedU128(value.into_inner())
}

pub fn to_api_per_mill(value: Permill) -> runtime_types::sp_arithmetic::per_things::Permill {
	runtime_types::sp_arithmetic::per_things::Permill(value.deconstruct())
}

pub fn from_api_fixed_u128(
	value: runtime_types::sp_arithmetic::fixed_point::FixedU128,
) -> FixedU128 {
	FixedU128::from_inner(value.0)
}

pub fn from_api_per_mill(value: runtime_types::sp_arithmetic::per_things::Permill) -> Permill {
	Permill::from_parts(value.0)
}
