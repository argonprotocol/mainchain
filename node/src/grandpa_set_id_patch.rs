use argon_primitives::Chain;
use codec::Encode;
use polkadot_sdk::*;
use sc_cli::RuntimeVersion;
use sc_client_api::StorageKey;
use sp_consensus_grandpa::SetId;
use sp_runtime::traits::{Block as BlockT, NumberFor};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct GrandpaStateOverrider<B: BlockT> {
	pub grandpa_set_id_by_block_number: BTreeMap<NumberFor<B>, Vec<u8>>,
	_phantom: std::marker::PhantomData<B>,
}

impl<B> GrandpaStateOverrider<B>
where
	B: BlockT + Encode,
{
	pub fn new() -> Self {
		Self {
			_phantom: std::marker::PhantomData,
			grandpa_set_id_by_block_number: Default::default(),
		}
	}

	pub fn for_chain(chain: Chain) -> Self {
		let mut overrider = Self::new();
		if chain == Chain::Mainnet {
			overrider
				.grandpa_set_id_by_block_number
				.insert(NumberFor::<B>::from(17574u32), (1 as SetId).encode());
		}
		if chain == Chain::Testnet {
			overrider
				.grandpa_set_id_by_block_number
				.insert(NumberFor::<B>::from(30271u32), (1 as SetId).encode());
			overrider
				.grandpa_set_id_by_block_number
				.insert(NumberFor::<B>::from(34562u32), (2 as SetId).encode());

			// now starts rotating at intervals of 1440 blocks
			let start_of_rotation = 38881u32;
			let set_id: SetId = 3;
			for i in 0..=10u32 {
				overrider.grandpa_set_id_by_block_number.insert(
					NumberFor::<B>::from(start_of_rotation + (i * 1440u32)),
					(set_id + i as SetId).encode(),
				);
			}
		}
		overrider
	}

	pub fn get_override(&self, spec_version: u32, at_number: NumberFor<B>) -> Option<Vec<u8>> {
		if !(104..=109).contains(&spec_version) {
			return None
		}
		self.grandpa_set_id_by_block_number.iter().find_map(|(key, value)| {
			if at_number >= *key {
				return Some(value.clone())
			}
			None
		})
	}
}

const GRANDPA_CURRENT_SET_ID: &str = "GrandpaApi_current_set_id";
const GRANDPA_CURRENT_SET_ID_STORAGE_KEY: [u8; 32] =
	hex_literal::hex!("5f9cc45b7a00c5899361e1c6099678dc8a2d09463effcc78a22d75b9cb87dffc");

impl<B> sc_service::StateOverrider<B> for GrandpaStateOverrider<B>
where
	B: BlockT,
{
	fn should_lookup_call_version(&self, function: &str) -> bool {
		function == GRANDPA_CURRENT_SET_ID
	}

	fn on_call(
		&self,
		_hash: &B::Hash,
		at_number: NumberFor<B>,
		runtime_version: &RuntimeVersion,
		function: &str,
	) -> Option<Vec<u8>> {
		if function == GRANDPA_CURRENT_SET_ID {
			return self.get_override(runtime_version.spec_version, at_number)
		}
		None
	}

	fn should_lookup_storage_version(&self, key: &StorageKey) -> bool {
		key.as_ref() == &GRANDPA_CURRENT_SET_ID_STORAGE_KEY[..]
	}

	fn on_storage_read(
		&self,
		_hash: &B::Hash,
		at_number: NumberFor<B>,
		runtime_version: &RuntimeVersion,
		key: &StorageKey,
	) -> Option<Vec<u8>> {
		if key.as_ref() == &GRANDPA_CURRENT_SET_ID_STORAGE_KEY[..] {
			return self.get_override(runtime_version.spec_version, at_number)
		}
		None
	}
}
