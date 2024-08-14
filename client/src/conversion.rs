use crate::api::runtime_types;
use sp_arithmetic::FixedU128;

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
			} => argon_primitives::bitcoin::BitcoinCosignScriptPubkey::P2WSH { wscript_hash },
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
		Self::new(value.tick_duration_millis, value.genesis_utc_time, value.escrow_expiration_ticks)
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

impl From<[u8; 78]> for runtime_types::argon_primitives::bitcoin::OpaqueBitcoinXpub {
	fn from(value: [u8; 78]) -> Self {
		Self(value)
	}
}

pub fn to_api_fixed_u128(value: FixedU128) -> runtime_types::sp_arithmetic::fixed_point::FixedU128 {
	runtime_types::sp_arithmetic::fixed_point::FixedU128(value.into_inner())
}

pub fn from_api_fixed_u128(
	value: runtime_types::sp_arithmetic::fixed_point::FixedU128,
) -> FixedU128 {
	FixedU128::from_inner(value.0)
}
