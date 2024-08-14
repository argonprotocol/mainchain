#![allow(dead_code)]

use std::sync::Arc;

use codec::{Decode, Encode};
use parking_lot::Mutex;
use sc_client_api::backend::AuxStore;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block as BlockT;

use argon_bitcoin::{BlockFilter, UtxoSpendFilter};
use argon_primitives::{
	bitcoin::{BitcoinSyncStatus, UtxoRef, UtxoValue},
	inherents::BitcoinUtxoSync,
	Balance, BitcoinApis,
};

pub fn get_bitcoin_inherent<C, B>(
	tracker: &Arc<UtxoTracker>,
	client: &Arc<C>,
	block_hash: &B::Hash,
) -> anyhow::Result<Option<BitcoinUtxoSync>>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + AuxStore + 'static,
	C::Api: BitcoinApis<B, Balance>,
{
	let api = client.runtime_api();
	let Some(sync_status) = api.get_sync_status(*block_hash)? else {
		return Ok(None);
	};

	let utxos = api.active_utxos(*block_hash)?;
	Ok(Some(tracker.sync(sync_status, utxos, client)?))
}

pub struct UtxoTracker {
	pub(crate) filter: Arc<Mutex<UtxoSpendFilter>>,
}

impl UtxoTracker {
	pub fn new(rpc_url: String, auth: Option<(String, String)>) -> anyhow::Result<Self> {
		let filter = UtxoSpendFilter::new(rpc_url, auth)?;

		Ok(Self { filter: Arc::new(Mutex::new(filter)) })
	}

	fn update_filters(
		&self,
		sync_status: &BitcoinSyncStatus,
		aux_store: &Arc<impl AuxStore>,
	) -> anyhow::Result<()> {
		let filter = self.filter.lock();
		const UTXO_KEY: &[u8; 28] = b"bitcoin_utxo_tracker_filters";

		{
			let synched_filters = filter.get_stored_filters();
			if synched_filters.is_empty() {
				if let Ok(Some(bytes)) = aux_store.get_aux(&UTXO_KEY[..]) {
					let synched_filters =
						<Vec<BlockFilter>>::decode(&mut &bytes[..]).ok().unwrap_or_default();
					filter.load_filters(synched_filters);
				}
			}
		}
		filter.sync_to_block(sync_status)?;

		let encoded = filter.get_stored_filters().encode();
		aux_store.insert_aux(&[(&UTXO_KEY[..], encoded.as_slice())], &[])?;
		Ok(())
	}

	/// Synchronize with the latest blocks on the network.
	pub fn sync(
		&self,
		sync_status: BitcoinSyncStatus,
		tracked_utxos: Vec<(Option<UtxoRef>, UtxoValue)>,
		aux_store: &Arc<impl AuxStore>,
	) -> anyhow::Result<BitcoinUtxoSync> {
		self.update_filters(&sync_status, aux_store)?;

		self.filter.lock().refresh_utxo_status(tracked_utxos)
	}
}

#[cfg(test)]
mod test {
	use std::{collections::BTreeMap, sync::Arc};

	use bitcoin::{hashes::Hash, Address, Amount, CompressedPublicKey, Network};
	use bitcoincore_rpc::RpcApi;
	use bitcoind::BitcoinD;
	use lazy_static::lazy_static;
	use parking_lot::Mutex;
	use sc_client_api::backend::AuxStore;

	use argon_bitcoin::{CosignScript, CosignScriptArgs};
	use argon_primitives::bitcoin::{
		BitcoinBlock, BitcoinRejectedReason, BitcoinSyncStatus, H256Le, UtxoRef, UtxoValue,
	};
	use argon_testing::{add_blocks, add_wallet_address, fund_script_address};

	use super::*;

	#[test]
	fn can_track_blocks_and_verify_utxos() {
		let (bitcoind, tracker, block_address, network) = start_bitcoind();

		let block_height = bitcoind.client.get_block_count().unwrap();
		let vault_claim_pubkey =
			bitcoind.client.get_address_info(&block_address).unwrap().pubkey.unwrap();

		let key1 = "033bc8c83c52df5712229a2f72206d90192366c36428cb0c12b6af98324d97bfbc"
			.parse::<CompressedPublicKey>()
			.unwrap();
		let key2 = "026c468be64d22761c30cd2f12cbc7de255d592d7904b1bab07236897cc4c2e766"
			.parse::<CompressedPublicKey>()
			.unwrap();

		let script = CosignScript::new(
			CosignScriptArgs {
				vault_pubkey: key1.into(),
				owner_pubkey: key2.into(),
				vault_claim_pubkey: vault_claim_pubkey.into(),
				vault_claim_height: block_height + 100,
				open_claim_height: block_height + 200,
				created_at_height: block_height,
			},
			network,
		)
		.expect("script");
		let script_address = script.address;

		let submitted_at_height = block_height + 1;

		let (txid, vout, _tx) = fund_script_address(
			&bitcoind,
			&script_address,
			Amount::ONE_BTC.to_sat(),
			&block_address,
		);

		add_blocks(&bitcoind, 6, &block_address);
		let confirmed = bitcoind.client.get_best_block_hash().unwrap();
		let block_height = bitcoind.client.get_block_count().unwrap();

		let aux = Arc::new(TestAuxStore::new());
		let sync_status = BitcoinSyncStatus {
			confirmed_block: BitcoinBlock {
				block_hash: H256Le(confirmed.to_byte_array()),
				block_height,
			},
			synched_block: None,
			oldest_allowed_block_height: block_height - 10,
		};
		tracker.update_filters(&sync_status, &aux).unwrap();

		let updated_filters = tracker.filter.lock().get_stored_filters();
		assert_eq!(updated_filters.len(), 11);
		assert_eq!(updated_filters[0].block_height, block_height - 10);
		assert_eq!(updated_filters[10].block_height, block_height);
		assert_eq!(updated_filters[10].block_hash, sync_status.confirmed_block.block_hash);

		let tracked = UtxoValue {
			utxo_id: 1,
			satoshis: Amount::ONE_BTC.to_sat(),
			script_pubkey: script_address.try_into().expect("can convert address to script"),
			submitted_at_height,
			watch_for_spent_until_height: 150,
		};
		{
			let result =
				tracker.sync(sync_status.clone(), vec![(None, tracked.clone())], &aux).unwrap();
			assert_eq!(result.verified.len(), 1);
			assert_eq!(
				result.verified.get(&1),
				Some(&UtxoRef { txid: txid.into(), output_index: vout })
			);
		}
		{
			let mut tracked = tracked.clone();
			tracked.satoshis = Amount::from_int_btc(2).to_sat();
			let result =
				tracker.sync(sync_status.clone(), vec![(None, tracked.clone())], &aux).unwrap();
			assert_eq!(result.verified.len(), 0);
			assert_eq!(result.invalid.get(&1), Some(&BitcoinRejectedReason::SatoshisMismatch));
		}
		drop(bitcoind);
	}

	lazy_static! {
		static ref BITCOIND_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
	}
	fn start_bitcoind() -> (BitcoinD, UtxoTracker, Address, Network) {
		// bitcoin will get in a fight with argon for ports, so lock here too
		let _lock = BITCOIND_LOCK.lock().unwrap();
		let (bitcoind, rpc_url, network) = argon_testing::start_bitcoind().expect("start_bitcoin");
		let _ = env_logger::builder().is_test(true).try_init();

		let block_address = add_wallet_address(&bitcoind);
		add_blocks(&bitcoind, 101, &block_address);

		let auth = if !rpc_url.username().is_empty() {
			Some((
				rpc_url.username().to_string(),
				rpc_url.password().unwrap_or_default().to_string(),
			))
		} else {
			None
		};

		let tracker = UtxoTracker::new(rpc_url.origin().unicode_serialization(), auth).unwrap();
		(bitcoind, tracker, block_address, network)
	}

	struct TestAuxStore {
		aux: Mutex<BTreeMap<Vec<u8>, Vec<u8>>>,
	}
	impl TestAuxStore {
		fn new() -> Self {
			Self { aux: Mutex::new(BTreeMap::new()) }
		}
	}

	impl AuxStore for TestAuxStore {
		fn insert_aux<
			'a,
			'b: 'a,
			'c: 'a,
			I: IntoIterator<Item = &'a (&'c [u8], &'c [u8])>,
			D: IntoIterator<Item = &'a &'b [u8]>,
		>(
			&self,
			insert: I,
			delete: D,
		) -> sc_client_api::blockchain::Result<()> {
			let mut aux = self.aux.lock();
			for (k, v) in insert {
				aux.insert(k.to_vec(), v.to_vec());
			}
			for k in delete {
				aux.remove(*k);
			}
			Ok(())
		}

		fn get_aux(&self, key: &[u8]) -> sc_client_api::blockchain::Result<Option<Vec<u8>>> {
			let aux = self.aux.lock();
			Ok(aux.get(key).cloned())
		}
	}
}
