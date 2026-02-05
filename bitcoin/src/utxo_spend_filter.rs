#![allow(dead_code)]

use crate::client::Client;
use anyhow::bail;
use argon_primitives::{
	bitcoin::{
		BitcoinBlock, BitcoinHeight, BitcoinNetwork, BitcoinSyncStatus, H256Le, Satoshis, UtxoRef,
		UtxoValue,
	},
	inherents::{BitcoinUtxoFunding, BitcoinUtxoSpend, BitcoinUtxoSync},
};
use bitcoin::{bip158, hashes::Hash};
use bitcoincore_rpc::{Auth, RpcApi};
use codec::{Decode, Encode};
use parking_lot::Mutex;
use polkadot_sdk::*;
use sp_runtime::RuntimeDebug;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Decode, Encode, PartialEq, Eq, RuntimeDebug)]
pub struct BlockFilter {
	pub block_hash: H256Le,
	pub previous_block_hash: Option<H256Le>,
	pub block_height: u64,
	pub filter: Vec<u8>,
}

impl BlockFilter {
	pub fn to_filter(&self) -> bip158::BlockFilter {
		bip158::BlockFilter::new(&self.filter)
	}

	pub fn to_block(&self) -> BitcoinBlock {
		BitcoinBlock { block_hash: self.block_hash.clone(), block_height: self.block_height }
	}
}

pub struct UtxoSpendFilter {
	client: Client,
	synched_filters: Arc<Mutex<Vec<BlockFilter>>>,
}

impl UtxoSpendFilter {
	pub fn new(rpc_url: String, auth: Option<(String, String)>) -> anyhow::Result<Self> {
		let auth = if let Some((username, password)) = auth {
			Auth::UserPass(username, password)
		} else {
			Auth::None
		};
		let client = Client::new(&rpc_url, auth)?;

		Ok(Self { client, synched_filters: Default::default() })
	}

	pub fn get_stored_filters(&self) -> Vec<BlockFilter> {
		self.synched_filters.lock().clone()
	}

	pub fn get_network(&self) -> anyhow::Result<BitcoinNetwork> {
		Ok(self.client.get_blockchain_info()?.chain.into())
	}

	pub fn load_filters(&self, filters: Vec<BlockFilter>) {
		*self.synched_filters.lock() = filters;
	}

	fn get_header_and_filter(&self, block_hash: &H256Le) -> anyhow::Result<BlockFilter> {
		let hash = bitcoin::BlockHash::from_slice(&block_hash.0)?;
		let header = self.client.get_block_header_info(&hash)?;
		let filter = self.client.get_block_filter(&hash)?;
		Ok(BlockFilter {
			block_height: header.height as u64,
			block_hash: block_hash.clone(),
			previous_block_hash: header.previous_block_hash.map(Into::into),
			filter: filter.filter,
		})
	}

	pub fn sync_to_block(&self, sync_status: &BitcoinSyncStatus) -> anyhow::Result<()> {
		let mut stored_filters = self.synched_filters.lock();
		let latest_block_hash = &sync_status.confirmed_block.block_hash;
		if stored_filters.last().map(|a| a.block_hash.clone()) != Some(latest_block_hash.clone()) {
			let entry = self.get_header_and_filter(latest_block_hash)?;
			stored_filters.push(entry);
		}

		let mut keep_sync_back_to = sync_status.oldest_allowed_block_height;
		// make sure we don't have a gap in the blocks
		if let Some(synched_block) = &sync_status.synched_block {
			if synched_block.block_height < keep_sync_back_to {
				keep_sync_back_to = synched_block.block_height;
			}
		}

		Self::prune_filters(keep_sync_back_to, &mut stored_filters);
		while stored_filters.first().map(|x| x.block_height) > Some(keep_sync_back_to) {
			let Some(first) = stored_filters.first() else {
				break;
			};
			if let Some(prev_hash) = &first.previous_block_hash {
				let entry = self.get_header_and_filter(prev_hash)?;
				stored_filters.insert(0, entry);
			} else {
				break;
			}
		}
		Ok(())
	}

	/// Synchronize with the latest blocks on the network.
	pub fn refresh_utxo_status(
		&self,
		tracked_utxos: Vec<(Option<UtxoRef>, UtxoValue)>,
		minimum_satoshis: Satoshis,
	) -> anyhow::Result<BitcoinUtxoSync> {
		let mut scripts: Vec<Vec<u8>> = vec![];
		let mut utxo_ref_to_utxo_id = BTreeMap::new();
		let mut funding_address_to_utxo_value = BTreeMap::new();

		for (utxo_ref, lookup) in tracked_utxos {
			let script_bytes = lookup.script_pubkey.to_script_bytes();
			scripts.push(script_bytes.clone());
			if let Some(utxo_ref) = utxo_ref {
				utxo_ref_to_utxo_id.insert(utxo_ref, lookup.utxo_id);
			}
			funding_address_to_utxo_value.insert(script_bytes, lookup);
		}
		let scripts = scripts.into_iter();

		let stored_filters = self.synched_filters.lock();
		let Some(latest) = stored_filters.last() else {
			bail!("Could not find latest block filter")
		};
		let mut result = BitcoinUtxoSync {
			sync_to_block: latest.to_block(),
			funded: Default::default(),
			spent: Default::default(),
		};

		for filter in &*stored_filters {
			let block_hash = bitcoin::BlockHash::from_slice(&filter.block_hash.0)?;
			if !filter.to_filter().match_any(&block_hash, scripts.clone())? {
				continue;
			}

			let block = self.client.get_block(&block_hash)?;
			let height = filter.block_height;
			for tx in block.txdata {
				for (idx, output) in tx.output.iter().enumerate() {
					let Some(utxo_value) =
						funding_address_to_utxo_value.get(output.script_pubkey.as_bytes())
					else {
						continue;
					};

					let sats = output.value.to_sat();
					// Ignore UTXOs below the minimum. This is a potential DoS vector, where an
					// attacker tries to fill blocks with micro-utxos, so we'll ignore before they
					// hit the node
					if sats < minimum_satoshis {
						continue;
					}
					let txid = tx.compute_txid().into();
					let utxo_ref = UtxoRef { txid, output_index: idx as u32 };
					result.funded.push(BitcoinUtxoFunding {
						utxo_id: utxo_value.utxo_id,
						utxo_ref: utxo_ref.clone(),
						satoshis: sats,
						expected_satoshis: utxo_value.satoshis,
						bitcoin_height: height,
					});
					utxo_ref_to_utxo_id.insert(utxo_ref, utxo_value.utxo_id);
				}
				// Check inputs to see if any tracked UTXOs were spent
				for input in &tx.input {
					let utxo_ref = input.previous_output.into();
					// If we're tracking the UTXO, it has been spent
					if let Some(id) = utxo_ref_to_utxo_id.get(&utxo_ref) {
						// TODO: should we figure out who spent it here?
						result.spent.push(BitcoinUtxoSpend {
							utxo_id: *id,
							utxo_ref: Some(utxo_ref),
							bitcoin_height: height,
						});
					}
				}
			}
		}

		Ok(result)
	}

	fn prune_filters(oldest_allowed_block_height: BitcoinHeight, filters: &mut Vec<BlockFilter>) {
		let mut drain_to = 0;
		// make sure the blocks link together with prev_hash
		for (i, header) in filters.iter().enumerate().rev() {
			if i == 0 {
				break;
			}
			let prev_header = &filters[i - 1];
			if let Some(prev_hash) = &header.previous_block_hash {
				if prev_hash != &prev_header.block_hash {
					drain_to = i;
					break;
				}
			}
		}

		if drain_to > 0 {
			filters.drain(..drain_to);
		}
		filters.retain(|f| f.block_height >= oldest_allowed_block_height);
	}
}

#[cfg(test)]
mod test {
	use crate::{BlockFilter, UtxoSpendFilter};
	use argon_primitives::bitcoin::H256Le;

	#[test]
	fn test_prune_filters() {
		// TEST: only keep above the oldest allowed height
		{
			let mut filters: Vec<BlockFilter> = vec![];
			for i in 100..105 {
				filters.push(BlockFilter {
					block_hash: H256Le([i; 32]),
					previous_block_hash: Some(H256Le([i - 1; 32])),
					block_height: i as u64,
					filter: vec![],
				});
			}

			UtxoSpendFilter::prune_filters(101, &mut filters);
			assert_eq!(filters.len(), 4);
		}

		// TEST: should clear history if reorg
		{
			let mut filters: Vec<BlockFilter> = vec![];
			for i in 100..105 {
				filters.push(BlockFilter {
					block_hash: H256Le([i; 32]),
					previous_block_hash: Some(H256Le([i - 1; 32])),
					block_height: i as u64,
					filter: vec![],
				});
			}

			// now simulate us adding a new confirmed block
			filters.push(BlockFilter {
				block_hash: H256Le([111; 32]),
				previous_block_hash: Some(H256Le([1; 32])),
				block_height: 105,
				filter: vec![],
			});
			UtxoSpendFilter::prune_filters(100, &mut filters);
			assert_eq!(filters.len(), 1);
			assert_eq!(filters[0].block_height, 105);
			assert_eq!(filters[0].block_hash, H256Le([111; 32]));
		}
	}
}
