use argon_primitives::{bitcoin::BitcoinNetwork, tick::Ticker};
use codec::Decode;
use polkadot_sdk::{
	sc_client_api::BlockBackend,
	sc_service::{ChainSpec, TFullClient},
	sp_blockchain::{self, HeaderBackend},
	sp_consensus::BlockStatus,
	sp_io,
	sp_runtime::traits::{Block as BlockT, Header},
};
use std::fmt;

pub const DEFAULT_STATE_LOOKBACK_DEPTH: usize = 2048;

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveBestOrFinalizedStateHashError<E> {
	Client(E),
	NoAvailableStateHash,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GenesisStorageReadError {
	BuildStorage(String),
	MissingKey { pallet: String, storage_item: String },
	Decode { pallet: String, storage_item: String, error: String },
}

impl fmt::Display for GenesisStorageReadError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::BuildStorage(error) => {
				write!(f, "Failed to build chain-spec genesis storage: {error}")
			},
			Self::MissingKey { pallet, storage_item } => {
				write!(f, "Missing genesis storage key {pallet}.{storage_item} in chain spec")
			},
			Self::Decode { pallet, storage_item, error } => {
				write!(f, "Failed to decode genesis storage {pallet}.{storage_item}: {error}")
			},
		}
	}
}

impl std::error::Error for GenesisStorageReadError {}

pub trait StateAnchorClient<Hash> {
	type Error;

	fn has_block_state(&self, hash: Hash) -> bool;
	fn parent_hash(&self, hash: &Hash) -> Result<Option<Hash>, Self::Error>;
}

impl<B, RuntimeApi, Exec> StateAnchorClient<B::Hash> for TFullClient<B, RuntimeApi, Exec>
where
	B: BlockT,
	TFullClient<B, RuntimeApi, Exec>: HeaderBackend<B> + BlockBackend<B>,
{
	type Error = sp_blockchain::Error;

	fn has_block_state(&self, hash: B::Hash) -> bool {
		self.block_status(hash).unwrap_or(BlockStatus::Unknown) == BlockStatus::InChainWithState
	}

	fn parent_hash(&self, hash: &B::Hash) -> Result<Option<B::Hash>, Self::Error> {
		Ok(self.header(*hash)?.map(|header| *header.parent_hash()))
	}
}

pub fn resolve_stateful_hash<Client, Hash>(
	client: &Client,
	start_hash: Hash,
	max_depth: usize,
) -> Result<Option<Hash>, Client::Error>
where
	Client: StateAnchorClient<Hash>,
	Hash: Copy + PartialEq,
{
	let mut cursor = start_hash;
	for _ in 0..max_depth {
		if client.has_block_state(cursor) {
			return Ok(Some(cursor));
		}

		let Some(parent) = client.parent_hash(&cursor)? else {
			return Ok(None);
		};
		if parent == cursor {
			return Ok(None);
		}
		cursor = parent;
	}
	Ok(None)
}

pub fn resolve_best_or_finalized_state_hash<Client, Hash>(
	client: &Client,
	best_hash: Hash,
	finalized_hash: Hash,
	max_depth: usize,
) -> Result<Hash, ResolveBestOrFinalizedStateHashError<Client::Error>>
where
	Client: StateAnchorClient<Hash>,
	Hash: Copy + PartialEq,
{
	if client.has_block_state(best_hash) {
		return Ok(best_hash);
	}

	resolve_stateful_hash(client, finalized_hash, max_depth)
		.map_err(ResolveBestOrFinalizedStateHashError::Client)?
		.ok_or(ResolveBestOrFinalizedStateHashError::NoAvailableStateHash)
}

pub fn read_chain_spec_ticker(
	chain_spec: &dyn ChainSpec,
) -> Result<Ticker, GenesisStorageReadError> {
	read_genesis_storage_value(chain_spec, b"Ticks", b"GenesisTicker")
}

pub fn read_chain_spec_bitcoin_network(
	chain_spec: &dyn ChainSpec,
) -> Result<BitcoinNetwork, GenesisStorageReadError> {
	read_genesis_storage_value(chain_spec, b"BitcoinUtxos", b"BitcoinNetwork")
}

pub fn read_genesis_storage_value<T: Decode>(
	chain_spec: &dyn ChainSpec,
	pallet: &[u8],
	storage_item: &[u8],
) -> Result<T, GenesisStorageReadError> {
	let storage = chain_spec
		.as_storage_builder()
		.build_storage()
		.map_err(|e| GenesisStorageReadError::BuildStorage(e.to_string()))?;
	let key = storage_value_key(pallet, storage_item);
	let bytes = storage.top.get(&key).ok_or_else(|| GenesisStorageReadError::MissingKey {
		pallet: String::from_utf8_lossy(pallet).to_string(),
		storage_item: String::from_utf8_lossy(storage_item).to_string(),
	})?;
	T::decode(&mut &bytes[..]).map_err(|e| GenesisStorageReadError::Decode {
		pallet: String::from_utf8_lossy(pallet).to_string(),
		storage_item: String::from_utf8_lossy(storage_item).to_string(),
		error: e.to_string(),
	})
}

fn storage_value_key(pallet: &[u8], storage_item: &[u8]) -> Vec<u8> {
	let mut key = Vec::with_capacity(32);
	key.extend_from_slice(&sp_io::hashing::twox_128(pallet));
	key.extend_from_slice(&sp_io::hashing::twox_128(storage_item));
	key
}

#[cfg(test)]
mod test {
	use super::{
		ResolveBestOrFinalizedStateHashError, StateAnchorClient,
		resolve_best_or_finalized_state_hash, resolve_stateful_hash,
	};

	struct TestClient;

	impl StateAnchorClient<u32> for TestClient {
		type Error = ();

		fn has_block_state(&self, hash: u32) -> bool {
			hash == 7 || hash == 11
		}

		fn parent_hash(&self, hash: &u32) -> Result<Option<u32>, Self::Error> {
			Ok(hash.checked_sub(1))
		}
	}

	#[test]
	fn resolves_from_ancestor_chain() {
		let client = TestClient;
		let resolved = resolve_stateful_hash(&client, 10, 20).expect("lookup should not fail");
		assert_eq!(resolved, Some(7));
	}

	#[test]
	fn prefers_best_when_available() {
		let client = TestClient;
		let resolved = resolve_best_or_finalized_state_hash(&client, 11, 9, 20)
			.expect("lookup should not fail");
		assert_eq!(resolved, 11);
	}

	#[test]
	fn errors_when_no_state_available() {
		let client = TestClient;
		let resolved = resolve_best_or_finalized_state_hash(&client, 3, 2, 1);
		assert_eq!(resolved, Err(ResolveBestOrFinalizedStateHashError::NoAvailableStateHash));
	}
}
