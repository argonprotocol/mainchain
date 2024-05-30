use crate::api::runtime_types;

impl<T, X: sp_core::Get<u32>> From<sp_core::bounded_vec::BoundedVec<T, X>>
	for runtime_types::bounded_collections::bounded_vec::BoundedVec<T>
{
	fn from(value: sp_core::bounded_vec::BoundedVec<T, X>) -> Self {
		runtime_types::bounded_collections::bounded_vec::BoundedVec(value.into())
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

impl From<runtime_types::ulx_primitives::tick::Ticker> for ulx_primitives::tick::Ticker {
	fn from(value: runtime_types::ulx_primitives::tick::Ticker) -> Self {
		Self::new(value.tick_duration_millis, value.genesis_utc_time)
	}
}

// ----- bitcoin -----
impl From<runtime_types::ulx_primitives::bitcoin::H256Le> for ulx_primitives::bitcoin::H256Le {
	fn from(value: runtime_types::ulx_primitives::bitcoin::H256Le) -> Self {
		Self(value.0)
	}
}

impl Into<runtime_types::ulx_primitives::bitcoin::H256Le> for ulx_primitives::bitcoin::H256Le {
	fn into(self) -> runtime_types::ulx_primitives::bitcoin::H256Le {
		runtime_types::ulx_primitives::bitcoin::H256Le(self.0)
	}
}

impl From<runtime_types::ulx_primitives::bitcoin::UtxoRef> for ulx_primitives::bitcoin::UtxoRef {
	fn from(value: runtime_types::ulx_primitives::bitcoin::UtxoRef) -> Self {
		Self { txid: value.txid.into(), output_index: value.output_index }
	}
}

impl Into<runtime_types::ulx_primitives::bitcoin::UtxoRef> for ulx_primitives::bitcoin::UtxoRef {
	fn into(self) -> runtime_types::ulx_primitives::bitcoin::UtxoRef {
		runtime_types::ulx_primitives::bitcoin::UtxoRef {
			txid: self.txid.into(),
			output_index: self.output_index,
		}
	}
}

impl From<runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash>
	for ulx_primitives::bitcoin::BitcoinPubkeyHash
{
	fn from(value: runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash) -> Self {
		Self(value.0)
	}
}

impl Into<runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash>
	for ulx_primitives::bitcoin::BitcoinPubkeyHash
{
	fn into(self) -> runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash {
		runtime_types::ulx_primitives::bitcoin::BitcoinPubkeyHash(self.0)
	}
}

impl TryFrom<runtime_types::ulx_primitives::bitcoin::BitcoinSignature>
	for ulx_primitives::bitcoin::BitcoinSignature
{
	type Error = Vec<u8>;
	fn try_from(
		value: runtime_types::ulx_primitives::bitcoin::BitcoinSignature,
	) -> Result<Self, Self::Error> {
		value.0 .0.try_into()
	}
}

impl Into<runtime_types::ulx_primitives::bitcoin::BitcoinSignature>
	for ulx_primitives::bitcoin::BitcoinSignature
{
	fn into(self) -> runtime_types::ulx_primitives::bitcoin::BitcoinSignature {
		runtime_types::ulx_primitives::bitcoin::BitcoinSignature(self.0.into())
	}
}

impl TryFrom<runtime_types::ulx_primitives::bitcoin::BitcoinScriptPubkey>
	for ulx_primitives::bitcoin::BitcoinScriptPubkey
{
	type Error = Vec<u8>;
	fn try_from(
		value: runtime_types::ulx_primitives::bitcoin::BitcoinScriptPubkey,
	) -> Result<Self, Self::Error> {
		value.0 .0.try_into()
	}
}

impl Into<runtime_types::ulx_primitives::bitcoin::BitcoinScriptPubkey>
	for ulx_primitives::bitcoin::BitcoinScriptPubkey
{
	fn into(self) -> runtime_types::ulx_primitives::bitcoin::BitcoinScriptPubkey {
		runtime_types::ulx_primitives::bitcoin::BitcoinScriptPubkey(self.0.into())
	}
}

impl Into<runtime_types::ulx_primitives::bitcoin::CompressedBitcoinPubkey>
	for ulx_primitives::bitcoin::CompressedBitcoinPubkey
{
	fn into(self) -> runtime_types::ulx_primitives::bitcoin::CompressedBitcoinPubkey {
		runtime_types::ulx_primitives::bitcoin::CompressedBitcoinPubkey(self.0)
	}
}

impl From<runtime_types::ulx_primitives::bitcoin::CompressedBitcoinPubkey>
	for ulx_primitives::bitcoin::CompressedBitcoinPubkey
{
	fn from(value: runtime_types::ulx_primitives::bitcoin::CompressedBitcoinPubkey) -> Self {
		Self(value.0.into())
	}
}
