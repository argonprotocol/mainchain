#![allow(dead_code)]

use std::{collections::BTreeMap, sync::Arc};

use anyhow::bail;
use bitcoin::{bip158, hashes::Hash};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use codec::{Decode, Encode};
use parking_lot::Mutex;
use sp_runtime::RuntimeDebug;

use argon_primitives::{
	bitcoin::{
		BitcoinBlock, BitcoinHeight, BitcoinRejectedReason, BitcoinSyncStatus, H256Le, UtxoRef,
		UtxoValue,
	},
	inherents::BitcoinUtxoSync,
};

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
	) -> anyhow::Result<BitcoinUtxoSync> {
		let mut scripts: Vec<Vec<u8>> = vec![];
		let mut utxos_by_ref = BTreeMap::new();
		let mut pending_confirmation_by_script = BTreeMap::new();

		for (utxo_ref, lookup) in tracked_utxos {
			scripts.push(lookup.script_pubkey.to_script_bytes());
			if let Some(utxo_ref) = utxo_ref {
				utxos_by_ref.insert(utxo_ref, lookup.clone());
			} else {
				pending_confirmation_by_script
					.insert(lookup.script_pubkey.to_script_bytes(), lookup);
			}
		}
		let scripts = scripts.into_iter();

		let stored_filters = self.synched_filters.lock();
		let Some(latest) = stored_filters.last() else {
			bail!("Could not find latest block filter")
		};
		let mut result = BitcoinUtxoSync {
			sync_to_block: latest.to_block(),
			verified: BTreeMap::new(),
			invalid: BTreeMap::new(),
			spent: BTreeMap::new(),
		};

		for filter in &*stored_filters {
			let block_hash = bitcoin::BlockHash::from_slice(&filter.block_hash.0)?;
			if !filter.to_filter().match_any(&block_hash, scripts.clone())? {
				continue;
			}

			let block = self.client.get_block(&block_hash)?;
			let height = filter.block_height;
			for tx in block.txdata {
				for input in &tx.input {
					let utxo_ref = input.previous_output.into();
					// If we're tracking the UTXO, it has been spent
					if let Some(value) = utxos_by_ref.get(&utxo_ref) {
						// TODO: should we figure out who spent it here?
						result.spent.insert(value.utxo_id, height);
					}
				}

				for (idx, output) in tx.output.iter().enumerate() {
					let Some(pending) =
						pending_confirmation_by_script.remove(output.script_pubkey.as_bytes())
					else {
						continue;
					};

					let utxo_id = pending.utxo_id;

					if output.value.to_sat() != pending.satoshis {
						result.invalid.insert(utxo_id, BitcoinRejectedReason::SatoshisMismatch);
					} else {
						let tx_id = tx.compute_txid().into();
						result
							.verified
							.insert(utxo_id, UtxoRef { txid: tx_id, output_index: idx as u32 });
					};
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
