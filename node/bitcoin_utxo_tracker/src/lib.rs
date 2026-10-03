#![allow(dead_code)]

mod metrics;

use anyhow::ensure;
use argon_bitcoin::{BlockFilter, UtxoSpendFilter};
use argon_primitives::{
	bitcoin::{BitcoinSyncStatus, Satoshis, UtxoAddress, UtxoRef},
	inherents::{BitcoinUtxoSync, BitcoinUtxoSyncVersion, BITCOIN_SPENDING_TXID_SPEC_VERSION},
	prelude::sp_api::{ApiExt, Core},
	Balance, BitcoinApis,
};
use codec::{Decode, Encode};
use log::info;
pub use metrics::BitcoinMetrics;
use parking_lot::Mutex;
use polkadot_sdk::*;
use sc_client_api::backend::AuxStore;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block as BlockT;
use std::{sync::Arc, time::Instant};
use substrate_prometheus_endpoint::Registry;

pub fn get_bitcoin_inherent<C, B>(
	tracker: &Arc<UtxoTracker>,
	client: &Arc<C>,
	block_hash: &B::Hash,
) -> anyhow::Result<Option<BitcoinUtxoSyncVersion>>
where
	B: BlockT,
	C: ProvideRuntimeApi<B> + AuxStore + 'static,
	C::Api: BitcoinApis<B, Balance> + sp_api::Core<B>,
{
	let api = client.runtime_api();
	let mut minimum_satoshis: Satoshis = 1000;
	let api_version = api.api_version::<dyn BitcoinApis<B, Balance>>(*block_hash).ok().flatten();
	// If we have version 2 of the API, use it to get minimum satoshis.
	if api_version.is_some_and(|version| version >= 2) {
		minimum_satoshis = api.get_minimum_satoshis(*block_hash)?;
	}

	let Some(sync_status) = api.get_sync_status(*block_hash)? else {
		return Ok(None);
	};

	let start_time = Instant::now();
	let utxos = if api_version.is_some_and(|version| version >= 4) {
		api.active_utxo_addresses(*block_hash)?
	} else {
		api.active_utxos(*block_hash)?
			.into_iter()
			.map(|(utxo_ref, address)| {
				(
					utxo_ref,
					argon_primitives::bitcoin::UtxoAddress {
						lock_id: address.lock_id,
						script_pubkey: address.script_pubkey,
						submitted_at_height: address.submitted_at_height,
					},
				)
			})
			.collect()
	};
	let utxo_count = utxos.len() as u64;
	let result = tracker.sync(sync_status, utxos, minimum_satoshis, client)?;
	if let Some(ref metrics) = tracker.metrics {
		metrics.track(&result, utxo_count, start_time);
	}
	let runtime_version = api.version(*block_hash)?;
	if runtime_version.spec_version >= BITCOIN_SPENDING_TXID_SPEC_VERSION {
		Ok(Some(BitcoinUtxoSyncVersion::Current(result)))
	} else {
		Ok(Some(BitcoinUtxoSyncVersion::V2(result.into())))
	}
}

pub struct UtxoTracker {
	pub(crate) filter: Arc<Mutex<UtxoSpendFilter>>,
	metrics: Option<BitcoinMetrics>,
}

impl UtxoTracker {
	pub fn new(
		rpc_url: String,
		auth: Option<(String, String)>,
		registry: Option<&Registry>,
	) -> anyhow::Result<Self> {
		let filter = UtxoSpendFilter::new(rpc_url, auth)?;
		let metrics = registry.and_then(|a| BitcoinMetrics::new(a).ok());
		Ok(Self { filter: Arc::new(Mutex::new(filter)), metrics })
	}

	pub fn ensure_correct_network(
		&self,
		network: argon_primitives::bitcoin::BitcoinNetwork,
	) -> anyhow::Result<()> {
		let filter = self.filter.lock();
		let connected_network = filter.get_network()?;
		ensure!(
			connected_network == network,
			"Incorrect bitcoin network connected to. Should be {:?}, but is {:?}",
			&network,
			&connected_network,
		);
		info!(target: "node::bitcoin_utxo_tracker", "Connected to correct bitcoin network: {connected_network:?}");
		Ok(())
	}

	/// Synchronize filters and derive UTXO status for the requested Bitcoin range.
	pub fn sync(
		&self,
		sync_status: BitcoinSyncStatus,
		tracked_utxos: Vec<(Option<UtxoRef>, UtxoAddress)>,
		minimum_satoshis: Satoshis,
		aux_store: &Arc<impl AuxStore>,
	) -> anyhow::Result<BitcoinUtxoSync> {
		// Another Argon parent must not replace this range before we derive its UTXO status.
		let filter = self.filter.lock();
		const UTXO_KEY: &[u8; 28] = b"bitcoin_utxo_tracker_filters";

		{
			let synched_filters = filter.get_stored_filters();
			if synched_filters.is_empty() &&
				let Ok(Some(bytes)) = aux_store.get_aux(&UTXO_KEY[..])
			{
				let synched_filters =
					<Vec<BlockFilter>>::decode(&mut &bytes[..]).ok().unwrap_or_default();
				filter.load_filters(synched_filters);
			}
		}
		filter.sync_to_block(&sync_status)?;

		let encoded = filter.get_stored_filters().encode();
		aux_store.insert_aux(&[(&UTXO_KEY[..], encoded.as_slice())], &[])?;
		filter.refresh_utxo_status(tracked_utxos, minimum_satoshis)
	}
}

#[cfg(test)]
mod test {
	use std::{
		collections::BTreeMap,
		sync::{
			atomic::{AtomicBool, Ordering},
			mpsc, Arc,
		},
		thread,
		time::Duration,
	};

	use bitcoin::{
		absolute::LockTime, hashes::Hash, opcodes::OP_TRUE, script::Builder, transaction::Version,
		Address, Amount, CompressedPublicKey, Network, OutPoint, ScriptBuf, Sequence, Transaction,
		TxIn, TxOut, Witness,
	};
	use bitcoincore_rpc::RpcApi;
	use bitcoind::BitcoinD;
	use lazy_static::lazy_static;
	use parking_lot::Mutex;
	use sc_client_api::backend::AuxStore;

	use argon_bitcoin::{CosignScript, CosignScriptArgs};
	use argon_primitives::{
		bitcoin::{BitcoinBlock, BitcoinSyncStatus, H256Le, UtxoAddress, UtxoRef},
		inherents::{BitcoinUtxoFunding, BitcoinUtxoSpend},
	};
	use argon_testing::{add_blocks, add_wallet_address, fund_script_address, get_txid_height};

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

		let tx_height = get_txid_height(&bitcoind, &txid).expect("get tx height");

		let _ = fund_script_address(&bitcoind, &script_address, 999, &block_address);

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
		tracker.sync(sync_status.clone(), vec![], 1000, &aux).unwrap();

		let updated_filters = tracker.filter.lock().get_stored_filters();
		assert_eq!(updated_filters.len(), 11);
		assert_eq!(updated_filters[0].block_height, block_height - 10);
		assert_eq!(updated_filters[10].block_height, block_height);
		assert_eq!(updated_filters[10].block_hash, sync_status.confirmed_block.block_hash);

		let tracked = UtxoAddress {
			lock_id: 1,
			script_pubkey: script_address.try_into().expect("can convert address to script"),
			submitted_at_height,
		};
		// should only find the 1 BTC UTXO
		{
			let result = tracker
				.sync(sync_status.clone(), vec![(None, tracked.clone())], 1000, &aux)
				.unwrap();
			assert_eq!(result.funded.len(), 1);
			assert_eq!(
				result.funded[0],
				BitcoinUtxoFunding {
					lock_id: 1,
					utxo_ref: UtxoRef { txid: txid.into(), output_index: vout },
					satoshis: Amount::ONE_BTC.to_sat(),
					expected_satoshis: 0,
					bitcoin_height: tx_height,
				}
			);
		}
		drop(bitcoind);
	}

	#[test]
	fn concurrent_sync_keeps_each_callers_bitcoin_tip_and_utxos() {
		let (bitcoind, tracker, block_address, network) = start_bitcoind();
		let witness_script = Builder::new().push_opcode(OP_TRUE).into_script();
		let script_address = Address::p2wsh(&witness_script, network);
		let submitted_at_height = bitcoind.client.get_block_count().unwrap() + 1;
		let (txid, vout, _) =
			fund_script_address(&bitcoind, &script_address, 20_000, &block_address);
		let funding_height = get_txid_height(&bitcoind, &txid).unwrap();
		let earlier = BitcoinSyncStatus {
			confirmed_block: BitcoinBlock {
				block_hash: bitcoind.client.get_best_block_hash().unwrap().into(),
				block_height: bitcoind.client.get_block_count().unwrap(),
			},
			synched_block: None,
			oldest_allowed_block_height: submitted_at_height,
		};
		let utxo_ref = UtxoRef { txid: txid.into(), output_index: vout };
		let tracked = vec![(
			Some(utxo_ref.clone()),
			UtxoAddress {
				lock_id: 1,
				script_pubkey: script_address.try_into().unwrap(),
				submitted_at_height,
			},
		)];
		let spending_tx = Transaction {
			version: Version::TWO,
			lock_time: LockTime::ZERO,
			input: vec![TxIn {
				previous_output: OutPoint { txid, vout },
				script_sig: ScriptBuf::new(),
				sequence: Sequence::MAX,
				witness: Witness::from_slice(&[witness_script.as_bytes()]),
			}],
			output: vec![TxOut {
				value: Amount::from_sat(19_000),
				script_pubkey: block_address.script_pubkey(),
			}],
		};
		let spending_txid = bitcoind.client.send_raw_transaction(&spending_tx).unwrap();
		add_blocks(&bitcoind, 8, &block_address);
		let spending_height = earlier.confirmed_block.block_height + 1;
		assert_eq!(get_txid_height(&bitcoind, &spending_txid).unwrap(), spending_height);
		let expected_funding = vec![BitcoinUtxoFunding {
			lock_id: 1,
			utxo_ref: utxo_ref.clone(),
			satoshis: 20_000,
			expected_satoshis: 0,
			bitcoin_height: funding_height,
		}];
		let expected_spend = vec![BitcoinUtxoSpend {
			lock_id: 1,
			utxo_ref: Some(utxo_ref),
			bitcoin_height: spending_height,
			spending_txid: spending_txid.into(),
		}];

		for gap in [1, 8] {
			let later_height = earlier.confirmed_block.block_height + gap;
			let later = BitcoinSyncStatus {
				confirmed_block: BitcoinBlock {
					block_hash: bitcoind.client.get_block_hash(later_height).unwrap().into(),
					block_height: later_height,
				},
				..earlier.clone()
			};
			let aux = Arc::new(TestAuxStore::new());
			let earlier_result =
				tracker.sync(earlier.clone(), tracked.clone(), 1000, &aux).unwrap();
			assert_eq!(earlier_result.sync_to_block, earlier.confirmed_block);
			assert_eq!(earlier_result.funded, expected_funding);
			assert!(earlier_result.spent.is_empty());
			let later_result = tracker.sync(later.clone(), tracked.clone(), 1000, &aux).unwrap();
			assert_eq!(later_result.sync_to_block, later.confirmed_block);
			assert_eq!(later_result.funded, expected_funding);
			assert_eq!(later_result.spent, expected_spend);

			for (first_status, first_expected, second_status, second_expected) in [
				(earlier.clone(), &earlier_result, later.clone(), &later_result),
				(later.clone(), &later_result, earlier.clone(), &earlier_result),
			] {
				let (persisted_tx, persisted_rx) = mpsc::channel();
				let (release_tx, release_rx) = mpsc::channel();
				*aux.pause_first_write.lock() = Some((persisted_tx, release_rx));
				let (started_tx, started_rx) = mpsc::channel();
				let (finished_tx, finished_rx) = mpsc::channel();
				let (first, second) = thread::scope(|scope| {
					let first =
						scope.spawn(|| tracker.sync(first_status, tracked.clone(), 1000, &aux));
					persisted_rx.recv_timeout(Duration::from_secs(10)).unwrap();
					let second = scope.spawn(|| {
						started_tx.send(()).unwrap();
						let result = tracker.sync(second_status, tracked.clone(), 1000, &aux);
						finished_tx.send(()).unwrap();
						result
					});
					started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
					// Queue a competing public sync while the first holds the tracker lock in
					// AuxStore. Holding it >1ms gives parking_lot a fair handoff at unlock.
					let blocked = finished_rx.recv_timeout(Duration::from_millis(50));
					release_tx.send(()).unwrap();
					assert_eq!(blocked, Err(mpsc::RecvTimeoutError::Timeout));
					(first.join().unwrap().unwrap(), second.join().unwrap().unwrap())
				});
				eprintln!(
					"gap={gap}: first tip={} spends={}, second tip={} spends={}",
					first.sync_to_block.block_height,
					first.spent.len(),
					second.sync_to_block.block_height,
					second.spent.len()
				);
				assert_eq!(
					first.sync_to_block, first_expected.sync_to_block,
					"first caller's tip was replaced"
				);
				assert_eq!(first.funded, first_expected.funded);
				assert_eq!(
					first.spent, first_expected.spent,
					"first caller used another parent's spends"
				);
				assert_eq!(&second, second_expected);
			}
			// Rebuild the tracker from a persisted later range and query both parents again.
			tracker.sync(later.clone(), tracked.clone(), 1000, &aux).unwrap();
			let rpc_url = argon_testing::read_rpc_url(&bitcoind).unwrap();
			let auth =
				Some((rpc_url.username().to_string(), rpc_url.password().unwrap().to_string()));
			let restarted =
				UtxoTracker::new(rpc_url.origin().unicode_serialization(), auth, None).unwrap();
			assert_eq!(
				restarted.sync(earlier.clone(), tracked.clone(), 1000, &aux).unwrap(),
				earlier_result
			);
			assert_eq!(restarted.sync(later, tracked.clone(), 1000, &aux).unwrap(), later_result);
		}
	}

	#[test]
	fn sync_recovers_from_errors_and_reanchors_persisted_filters_after_restart() {
		let (bitcoind, tracker, _, _) = start_bitcoind();
		let height = bitcoind.client.get_block_count().unwrap();
		let earlier = BitcoinSyncStatus {
			confirmed_block: BitcoinBlock {
				block_hash: bitcoind.client.get_block_hash(height - 1).unwrap().into(),
				block_height: height - 1,
			},
			synched_block: None,
			oldest_allowed_block_height: height - 2,
		};
		let later = BitcoinSyncStatus {
			confirmed_block: BitcoinBlock {
				block_hash: bitcoind.client.get_best_block_hash().unwrap().into(),
				block_height: height,
			},
			..earlier.clone()
		};
		let aux = Arc::new(TestAuxStore::new());
		let earlier_result = tracker.sync(earlier.clone(), vec![], 1000, &aux).unwrap();
		assert_eq!(earlier_result.sync_to_block, earlier.confirmed_block);
		let persisted = aux.aux.lock().clone();

		aux.fail_next_write.store(true, Ordering::SeqCst);
		let error = tracker.sync(later.clone(), vec![], 1000, &aux).unwrap_err();
		assert!(error.to_string().contains("test auxiliary write failure"));
		assert_eq!(*aux.aux.lock(), persisted);
		assert_eq!(
			tracker.filter.lock().get_stored_filters().last().unwrap().to_block(),
			later.confirmed_block
		);
		assert_eq!(
			tracker.sync(later.clone(), vec![], 1000, &aux).unwrap().sync_to_block,
			later.confirmed_block
		);

		let persisted = aux.aux.lock().clone();
		let invalid = BitcoinSyncStatus {
			confirmed_block: BitcoinBlock { block_hash: H256Le([0; 32]), block_height: height + 1 },
			..later.clone()
		};
		assert!(tracker.sync(invalid, vec![], 1000, &aux).is_err());
		assert_eq!(*aux.aux.lock(), persisted);
		assert_eq!(tracker.sync(earlier.clone(), vec![], 1000, &aux).unwrap(), earlier_result);
		tracker.sync(later.clone(), vec![], 1000, &aux).unwrap();

		let rpc_url = argon_testing::read_rpc_url(&bitcoind).unwrap();
		let auth = Some((rpc_url.username().to_string(), rpc_url.password().unwrap().to_string()));
		let restarted =
			UtxoTracker::new(rpc_url.origin().unicode_serialization(), auth, None).unwrap();
		assert!(restarted.filter.lock().get_stored_filters().is_empty());
		// Persisted filters are ahead of this parent's Bitcoin status.
		assert_eq!(restarted.sync(earlier.clone(), vec![], 1000, &aux).unwrap(), earlier_result);
		restarted.sync(later.clone(), vec![], 1000, &aux).unwrap();

		// A same-height Bitcoin fork must replace the cached hash, including after restart.
		bitcoind
			.client
			.invalidate_block(&bitcoind.client.get_best_block_hash().unwrap())
			.unwrap();
		let fork_address = add_wallet_address(&bitcoind);
		add_blocks(&bitcoind, 1, &fork_address);
		let fork = BitcoinSyncStatus {
			confirmed_block: BitcoinBlock {
				block_hash: bitcoind.client.get_best_block_hash().unwrap().into(),
				block_height: height,
			},
			..later.clone()
		};
		assert_ne!(fork.confirmed_block.block_hash, later.confirmed_block.block_hash);
		let rpc_url = argon_testing::read_rpc_url(&bitcoind).unwrap();
		let auth = Some((rpc_url.username().to_string(), rpc_url.password().unwrap().to_string()));
		let restarted =
			UtxoTracker::new(rpc_url.origin().unicode_serialization(), auth, None).unwrap();
		assert_eq!(
			restarted.sync(fork.clone(), vec![], 1000, &aux).unwrap().sync_to_block,
			fork.confirmed_block
		);
		let filters = restarted.filter.lock().get_stored_filters();
		assert_eq!(filters.len(), 3);
		assert_eq!(filters.last().unwrap().to_block(), fork.confirmed_block);
		for pair in filters.windows(2) {
			assert_eq!(pair[1].previous_block_hash, Some(pair[0].block_hash.clone()));
		}
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

		let tracker =
			UtxoTracker::new(rpc_url.origin().unicode_serialization(), auth, None).unwrap();
		(bitcoind, tracker, block_address, network)
	}

	struct TestAuxStore {
		aux: Mutex<BTreeMap<Vec<u8>, Vec<u8>>>,
		pause_first_write: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
		fail_next_write: AtomicBool,
	}
	impl TestAuxStore {
		fn new() -> Self {
			Self {
				aux: Mutex::new(BTreeMap::new()),
				pause_first_write: Mutex::new(None),
				fail_next_write: AtomicBool::new(false),
			}
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
			if self.fail_next_write.swap(false, Ordering::SeqCst) {
				return Err(sc_client_api::blockchain::Error::Backend(
					"test auxiliary write failure".into(),
				));
			}
			let mut aux = self.aux.lock();
			for (k, v) in insert {
				aux.insert(k.to_vec(), v.to_vec());
			}
			for k in delete {
				aux.remove(*k);
			}
			drop(aux);
			let pause = self.pause_first_write.lock().take();
			if let Some((persisted, release)) = pause {
				persisted.send(()).unwrap();
				release.recv_timeout(Duration::from_secs(10)).unwrap();
			}
			Ok(())
		}

		fn get_aux(&self, key: &[u8]) -> sc_client_api::blockchain::Result<Option<Vec<u8>>> {
			let aux = self.aux.lock();
			Ok(aux.get(key).cloned())
		}
	}
}
