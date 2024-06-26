#![allow(dead_code)]

use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use bitcoin::{
	absolute::LockTime,
	bip32::{DerivationPath, Fingerprint, Xpriv, Xpub},
	hashes::Hash,
	secp256k1::Secp256k1,
	Address, Amount, CompressedPublicKey, FeeRate, Network, PrivateKey, Script, Txid,
};
use bitcoincore_rpc::{json::GetRawTransactionResult, RawTx, RpcApi};
use bitcoind::BitcoinD;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use rand::{rngs::OsRng, RngCore};
use sc_client_api::backend::AuxStore;

use ulx_primitives::bitcoin::{
	create_timelock_multisig_script, BitcoinBlock, BitcoinCosignScriptPubkey, BitcoinHeight,
	BitcoinPubkeyHash, BitcoinRejectedReason, BitcoinScriptPubkey, BitcoinSyncStatus, H256Le,
	Satoshis, UtxoRef, UtxoValue,
};

use crate::unlocker::{CosignScript, UnlockStep, UtxoUnlocker};

use super::*;

const NUM_BLOCKS: u32 = 101;

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

		UtxoTracker::prune_filters(101, &mut filters);
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
		UtxoTracker::prune_filters(100, &mut filters);
		assert_eq!(filters.len(), 1);
		assert_eq!(filters[0].block_height, 105);
		assert_eq!(filters[0].block_hash, H256Le([111; 32]));
	}
}

#[test]
fn can_track_blocks_and_verify_utxos() {
	let (bitcoind, tracker, block_address) = start_bitcoind();

	let block_height = bitcoind.client.get_block_count().unwrap();

	let key1 = "033bc8c83c52df5712229a2f72206d90192366c36428cb0c12b6af98324d97bfbc"
		.parse::<CompressedPublicKey>()
		.unwrap();
	let key2 = "026c468be64d22761c30cd2f12cbc7de255d592d7904b1bab07236897cc4c2e766"
		.parse::<CompressedPublicKey>()
		.unwrap();

	let script = create_timelock_multisig_script(
		key1.pubkey_hash().into(),
		key2.pubkey_hash().into(),
		block_height + 100,
		block_height + 200,
	)
	.expect("script");
	let script_address = Address::p2wsh(script.as_script(), Network::Regtest);

	let submitted_at_height = block_height + 1;

	let (txid, vout, _tx) =
		fund_script_address(&bitcoind, &script_address, Amount::ONE_BTC.to_sat(), &block_address);

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
	let updated_filters = tracker.update_filters(&sync_status, &aux).unwrap();
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
}

#[test]
fn vault_can_claim_the_timelock_script() {
	let (bitcoind, tracker, block_address) = start_bitcoind();

	let block_height = bitcoind.client.get_block_count().unwrap();

	let (master_xpriv, fingerprint) = create_xpriv();
	let (vault_compressed_pubkey, vault_hd_path) = derive(&master_xpriv, "m/0'/0/1");

	let owner_compressed_pubkey =
		"026c468be64d22761c30cd2f12cbc7de255d592d7904b1bab07236897cc4c2e766"
			.parse::<CompressedPublicKey>()
			.unwrap();

	let open_claim_height = block_height + 8;
	let vault_claim_height = block_height + 4;

	let script = create_timelock_multisig_script(
		vault_compressed_pubkey.pubkey_hash().into(),
		owner_compressed_pubkey.pubkey_hash().into(),
		vault_claim_height,
		open_claim_height,
	)
	.expect("script")
	.to_bytes();
	println!("{:#?}", Script::from_bytes(script.as_slice()).to_asm_string());

	let script_address = Address::p2wsh(Script::from_bytes(&script.clone()), Network::Regtest);

	let (txid, vout, src_tx) =
		fund_script_address(&bitcoind, &script_address, Amount::ONE_BTC.to_sat(), &block_address);

	println!("{:#?} #{:?}", src_tx, vout);

	let block_height = bitcoind.client.get_block_count().unwrap();
	let register_height = block_height;
	assert!(block_height < open_claim_height);

	let fee_rate = FeeRate::from_sat_per_vb(15).expect("cant translate fee");
	let cosign_script = CosignScript::new(
		vault_compressed_pubkey.pubkey_hash().into(),
		owner_compressed_pubkey.pubkey_hash().into(),
		vault_claim_height,
		open_claim_height,
		register_height,
	)
	.unwrap();

	let pay_to_script_pubkey = vault_compressed_pubkey.p2wpkh_script_code();
	let fee = cosign_script
		.calculate_fee(false, pay_to_script_pubkey.clone(), fee_rate)
		.unwrap();

	// fails locktime if not cleared
	{
		let mut unlocker = UtxoUnlocker::from_script(
			cosign_script.clone(),
			Amount::ONE_BTC.to_sat(),
			txid,
			vout as u32,
			UnlockStep::VaultClaim,
			fee,
			pay_to_script_pubkey.clone(),
		)
		.expect("unlocker");

		unlocker.psbt.unsigned_tx.lock_time = LockTime::from_consensus(block_height as u32);
		unlocker
			.sign_derived(master_xpriv, (fingerprint, vault_hd_path.clone()))
			.expect("sign");

		let tx = unlocker.extract_tx().expect("tx");
		let acceptance = bitcoind.client.test_mempool_accept(&[tx.raw_hex()]).expect("checked");
		let did_accept = acceptance.first().unwrap();
		println!("{:?}", did_accept);
		assert_eq!(did_accept.allowed, false);
		let reject = did_accept.reject_reason.as_ref().unwrap();
		assert!(reject.contains("Locktime requirement not satisfied"));
	}

	let mut unlocker = UtxoUnlocker::from_script(
		cosign_script.clone(),
		Amount::ONE_BTC.to_sat(),
		txid,
		vout as u32,
		UnlockStep::VaultClaim,
		fee,
		pay_to_script_pubkey.clone(),
	)
	.expect("unlocker");

	unlocker
		.sign_derived(master_xpriv, (fingerprint, vault_hd_path.clone()))
		.expect("sign");

	let tx = unlocker.extract_tx().expect("tx");
	let tx_hex = tx.raw_hex();

	// returns non-final if tx locktime not reached yet
	{
		let acceptance = bitcoind.client.test_mempool_accept(&[tx_hex.clone()]).expect("checked");

		println!("{:?}", acceptance[0]);
		assert_eq!(acceptance[0].allowed, false);
		let reject = acceptance[0].reject_reason.as_ref().unwrap();
		assert!(reject.contains("non-final"));
	}

	// cannot accept until the cosign height
	let mut block_height = block_height;
	while block_height < vault_claim_height {
		let acceptance = bitcoind.client.test_mempool_accept(&[tx_hex.clone()]).expect("checked");
		assert_eq!(acceptance[0].allowed, false);
		add_blocks(&bitcoind, 1, &block_address);
		block_height = bitcoind.client.get_block_count().unwrap();
	}

	assert_eq!(bitcoind.client.get_block_count().unwrap(), vault_claim_height);
	{
		let acceptance = bitcoind.client.test_mempool_accept(&[tx_hex.clone()]).expect("checked");

		println!("{:?}", acceptance[0]);
		println!(
			"btcdeb --tx={:?} --txin={:?}",
			tx.raw_hex(),
			src_tx.transaction().unwrap().raw_hex()
		);
		assert!(acceptance[0].allowed);
	}

	check_spent(
		tx_hex.as_str(),
		&tracker,
		&bitcoind,
		UtxoRef { txid: txid.into(), output_index: vout as u32 },
		UtxoValue {
			utxo_id: 1,
			satoshis: Amount::ONE_BTC.to_sat(),
			script_pubkey: cosign_script.get_script_pubkey().try_into().unwrap(),
			submitted_at_height: register_height,
			watch_for_spent_until_height: open_claim_height,
		},
		&block_address,
	);
}

#[test]
fn owner_can_reclaim_the_timelock_script() {
	let (bitcoind, tracker, block_address) = start_bitcoind();

	let block_height = bitcoind.client.get_block_count().unwrap();

	let (master_xpriv, _) = create_xpriv();
	let (vault_compressed_pubkey, _) = derive(&master_xpriv, "m/0'/0/1");

	let secp = Secp256k1::new();
	let owner_keypair = PrivateKey::generate(Network::Regtest);
	let owner_compressed_pubkey = owner_keypair.public_key(&secp);
	let owner_pubkey_hash: BitcoinPubkeyHash = owner_compressed_pubkey.pubkey_hash().into();
	let amount: Satoshis = Amount::ONE_BTC.to_sat() * 5;

	let open_claim_height = block_height + 10;
	let vault_claim_height = block_height + 5;
	let register_height = block_height;
	let mut cosign_script = CosignScript::new(
		vault_compressed_pubkey.pubkey_hash().into(),
		owner_pubkey_hash.clone(),
		vault_claim_height,
		open_claim_height,
		register_height,
	)
	.unwrap();

	let script_address = cosign_script.get_script_address(Network::Regtest);

	let (txid, vout, src_tx) =
		fund_script_address(&bitcoind, &script_address, amount, &block_address);

	let block_height = bitcoind.client.get_block_count().unwrap();

	assert!(block_height < open_claim_height);
	cosign_script.set_registered_height(block_height);

	let fee_rate = FeeRate::from_sat_per_vb(15).expect("cant translate fee");

	let pay_to_script_pubkey = owner_compressed_pubkey.p2wpkh_script_code().unwrap();
	let fee = cosign_script
		.calculate_fee(false, pay_to_script_pubkey.clone(), fee_rate)
		.unwrap();

	// cannot accept until the cosign height
	let mut block_height = block_height;
	while block_height < open_claim_height {
		let mut unlocker = UtxoUnlocker::from_script(
			cosign_script.clone(),
			amount,
			txid,
			vout,
			UnlockStep::OwnerClaim,
			fee,
			pay_to_script_pubkey.clone(),
		)
		.expect("unlocker");

		unlocker.psbt.unsigned_tx.lock_time = LockTime::from_consensus(block_height as u32);
		unlocker.sign(owner_keypair).expect("sign");

		let tx = unlocker.extract_tx().expect("tx");
		let acceptance = bitcoind.client.test_mempool_accept(&[tx.raw_hex()]).expect("checked");
		let did_accept = acceptance.first().unwrap();
		println!("{} {:?}", block_height, did_accept);
		assert_eq!(did_accept.allowed, false);
		let reject = did_accept.reject_reason.as_ref().unwrap();
		if reject.contains("Script failed") {
			println!(
				"btcdeb --tx={:?} --txin={:?}",
				tx.raw_hex(),
				src_tx.transaction().unwrap().raw_hex()
			);
		}
		assert!(reject.contains("Locktime requirement not satisfied"));

		add_blocks(&bitcoind, 1, &block_address);
		block_height = bitcoind.client.get_block_count().unwrap();
	}

	assert_eq!(bitcoind.client.get_block_count().unwrap(), open_claim_height);
	{
		let mut unlocker = UtxoUnlocker::from_script(
			cosign_script.clone(),
			amount,
			txid,
			vout,
			UnlockStep::OwnerClaim,
			fee,
			pay_to_script_pubkey.clone(),
		)
		.expect("unlocker");

		unlocker.sign(owner_keypair).expect("sign");
		let tx = unlocker.extract_tx().expect("tx");

		println!(
			"btcdeb --tx={:?} --txin={:?}",
			tx.raw_hex(),
			src_tx.transaction().unwrap().raw_hex()
		);
		let acceptance = bitcoind.client.test_mempool_accept(&[tx.raw_hex()]).expect("checked");

		println!("{:?}", acceptance[0]);
		assert!(acceptance[0].allowed);

		check_spent(
			&tx.raw_hex(),
			&tracker,
			&bitcoind,
			UtxoRef { txid: txid.into(), output_index: vout },
			UtxoValue {
				utxo_id: 1,
				satoshis: Amount::ONE_BTC.to_sat(),
				script_pubkey: cosign_script.get_script_pubkey().try_into().unwrap(),
				submitted_at_height: register_height,
				watch_for_spent_until_height: open_claim_height,
			},
			&block_address,
		);
	}
}

#[test]
fn vault_and_owner_can_cosign() {
	let (bitcoind, tracker, block_address) = start_bitcoind();

	// 1. Owner creates a new pubkey and submits to blockchain
	let secp = Secp256k1::new();
	let owner_keypair = PrivateKey::generate(Network::Regtest);
	let owner_compressed_pubkey = owner_keypair.public_key(&secp);
	let owner_pubkey_hash: BitcoinPubkeyHash = owner_compressed_pubkey.pubkey_hash().into();
	let amount: Satoshis = Amount::ONE_BTC.to_sat() * 5;

	let (vault_master_xpriv, vault_fingerprint) = create_xpriv();
	let (vault_compressed_pubkey, vault_hd_path) = derive(&vault_master_xpriv, "m/48'/0'/0'/0/1");
	let vault_pubkey_hash: BitcoinPubkeyHash = vault_compressed_pubkey.pubkey_hash().into();

	let block_height = bitcoind.client.get_block_count().unwrap();

	let open_claim_height = block_height + 20;
	let vault_claim_height = block_height + 10;

	// 2. Vault publishes details script_pubkey and vault pubkey hash

	// 3. Owner recreates the script from the details and submits to blockchain
	let script_address = {
		let cosign_script = CosignScript::new(
			vault_pubkey_hash.clone(),
			owner_pubkey_hash.clone(),
			vault_claim_height,
			open_claim_height,
			block_height,
		)
		.expect("script address");
		cosign_script.get_script_address(Network::Regtest)
	};

	let utxo_script_pubkey: BitcoinCosignScriptPubkey =
		script_address.clone().try_into().expect("can convert address to script");

	let (txid, _vout, tx) = fund_script_address(&bitcoind, &script_address, amount, &block_address);

	let source_txin = tx.transaction().unwrap().raw_hex();
	let block_hash = tx.blockhash.unwrap();
	let block_height = bitcoind.client.get_block_count().unwrap();
	let register_height = block_height;
	let sync = tracker
		.sync(
			BitcoinSyncStatus {
				confirmed_block: BitcoinBlock { block_hash: block_hash.into(), block_height },
				synched_block: None,
				oldest_allowed_block_height: block_height - 10,
			},
			vec![(
				None,
				UtxoValue {
					utxo_id: 1,
					satoshis: amount,
					script_pubkey: utxo_script_pubkey.clone(),
					submitted_at_height: block_height,
					watch_for_spent_until_height: open_claim_height,
				},
			)],
			&Arc::new(TestAuxStore::new()),
		)
		.unwrap();
	assert_eq!(sync.verified.len(), 1);
	let utxo = sync.verified.get(&1).unwrap();
	assert_eq!(utxo.txid, txid.into());

	// 4. User submits the out address
	let out_script_pubkey: BitcoinScriptPubkey =
		owner_compressed_pubkey.p2wpkh_script_code().unwrap().try_into().unwrap();
	let feerate = FeeRate::from_sat_per_vb(15).expect("cant translate fee");
	let user_cosign_script = CosignScript::new(
		vault_pubkey_hash.clone(),
		owner_pubkey_hash.clone(),
		vault_claim_height,
		open_claim_height,
		register_height,
	)
	.unwrap();
	let fee = user_cosign_script
		.calculate_fee(true, out_script_pubkey.clone().into(), feerate)
		.unwrap();

	// 5. vault sees unlock request (outaddress, fee) and creates a transaction
	let (vault_signature, vault_pubkey) = {
		let mut unlocker = UtxoUnlocker::new(
			vault_pubkey_hash.clone().into(),
			owner_pubkey_hash.clone().into(),
			register_height,
			vault_claim_height,
			open_claim_height,
			amount,
			utxo.txid.clone().into(),
			utxo.output_index,
			UnlockStep::VaultCosign,
			fee,
			out_script_pubkey.clone().into(),
		)
		.expect("unlocker");

		unlocker
			.sign_derived(vault_master_xpriv, (vault_fingerprint, vault_hd_path))
			.expect("sign")
	};

	// 6. User sees the transaction and cosigns
	let tx = {
		let mut unlocker = UtxoUnlocker::from_script(
			user_cosign_script.clone(),
			amount,
			utxo.txid.clone().into(),
			utxo.output_index,
			UnlockStep::OwnerCosign,
			fee,
			out_script_pubkey.clone().into(),
		)
		.unwrap();
		unlocker.add_signature(vault_pubkey, vault_signature.into());
		unlocker.sign(owner_keypair).expect("sign");
		unlocker.extract_tx().expect("tx")
	};

	println!("{:#?}", tx);
	let tx_hex = tx.raw_hex();

	let acceptance = bitcoind.client.test_mempool_accept(&[tx_hex.clone()]).expect("checked");
	let did_accept = acceptance.first().unwrap();
	println!("{:?}", did_accept);
	println!("btcdeb --tx={:?} --txin={:?}", tx_hex, source_txin);
	assert!(did_accept.allowed);

	check_spent(
		tx_hex.as_str(),
		&tracker,
		&bitcoind,
		utxo.clone(),
		UtxoValue {
			utxo_id: 1,
			satoshis: amount,
			script_pubkey: utxo_script_pubkey.clone(),
			submitted_at_height: register_height,
			watch_for_spent_until_height: open_claim_height,
		},
		&block_address,
	);
}

fn check_spent(
	tx_hex: &str,
	tracker: &UtxoTracker,
	bitcoind: &BitcoinD,
	utxo_ref: UtxoRef,
	utxo_value: UtxoValue,
	block_address: &Address,
) {
	let final_txid = bitcoind.client.send_raw_transaction(tx_hex).expect("sent");
	let tx_result = wait_for_txid(&bitcoind, &final_txid, block_address);
	let tx_block_height = bitcoind
		.client
		.get_block_header_info(&tx_result.blockhash.unwrap())
		.unwrap()
		.height;
	let block_height = bitcoind.client.get_block_count().unwrap();
	let block_hash = bitcoind.client.get_best_block_hash().unwrap();

	let latest = tracker
		.sync(
			BitcoinSyncStatus {
				confirmed_block: BitcoinBlock { block_hash: block_hash.into(), block_height },
				synched_block: None,
				oldest_allowed_block_height: block_height - 10,
			},
			vec![(Some(utxo_ref), utxo_value)],
			&Arc::new(TestAuxStore::new()),
		)
		.expect("sync 2");
	assert_eq!(latest.spent.len(), 1);

	assert_eq!(latest.spent.get(&1), Some(&(tx_block_height as BitcoinHeight)));
}

fn fund_script_address(
	bitcoind: &BitcoinD,
	script_address: &Address,
	amount: Satoshis,
	block_address: &Address,
) -> (Txid, u32, GetRawTransactionResult) {
	let txid = bitcoind
		.client
		.send_to_address(
			&script_address,
			Amount::from_sat(amount),
			None,
			None,
			None,
			None,
			None,
			None,
		)
		.unwrap();
	let tx = wait_for_txid(&bitcoind, &txid, &block_address);
	let vout = tx
		.vout
		.iter()
		.position(|o| o.script_pub_key.script().unwrap() == script_address.script_pubkey())
		.unwrap() as u32;
	(txid, vout, tx)
}

lazy_static! {
	static ref BITCOIND_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
fn start_bitcoind() -> (BitcoinD, UtxoTracker, Address) {
	// bitcoin will get in a fight with ulixee for ports, so lock here too
	let _lock = BITCOIND_LOCK.lock().unwrap();
	let (bitcoind, rpc_url) = ulx_testing::start_bitcoind().expect("start_bitcoin");
	let _ = env_logger::builder().is_test(true).try_init();

	let block_address = add_wallet_address(&bitcoind);
	add_blocks(&bitcoind, NUM_BLOCKS as u64, &block_address);

	let auth = if rpc_url.username().len() > 0 {
		Some((rpc_url.username().to_string(), rpc_url.password().unwrap_or_default().to_string()))
	} else {
		None
	};

	let tracker = UtxoTracker::new(rpc_url.origin().unicode_serialization(), auth).unwrap();
	(bitcoind, tracker, block_address.into())
}

fn wait_for_txid(
	bitcoind: &BitcoinD,
	txid: &Txid,
	block_address: &Address,
) -> GetRawTransactionResult {
	loop {
		// Attempt to get the raw transaction with verbose output
		let result = bitcoind.client.call::<GetRawTransactionResult>(
			"getrawtransaction",
			&[txid.to_string().into(), 1.into()],
		);

		if let Ok(tx) = result {
			if tx.confirmations.unwrap_or_default() > 1 {
				return tx;
			}
		}
		std::thread::sleep(std::time::Duration::from_secs(1));
		add_blocks(&bitcoind, 1, block_address);
	}
}

fn create_xpriv() -> (Xpriv, Fingerprint) {
	let secp = Secp256k1::new();
	let mut seed = [0u8; 32];
	OsRng.fill_bytes(&mut seed);
	let master_xpriv = Xpriv::new_master(Network::Regtest, &seed).unwrap();
	let master_xpub = Xpub::from_priv(&secp, &master_xpriv);
	let fingerprint = master_xpub.fingerprint();
	(master_xpriv, fingerprint)
}

fn derive(master_xpriv: &Xpriv, path: &str) -> (CompressedPublicKey, DerivationPath) {
	let secp = Secp256k1::new();
	let path = DerivationPath::from_str(path).expect("Invalid derivation path");
	let child_xpriv = master_xpriv.derive_priv(&secp, &path).expect("Unable to derive child key");

	let vault_privkey = child_xpriv.to_priv();
	let vault_compressed_pubkey = CompressedPublicKey::from_private_key(&secp, &vault_privkey)
		.expect("Unable to derive pubkey");
	(vault_compressed_pubkey, path)
}

fn add_wallet_address(bitcoind: &BitcoinD) -> Address {
	let address = bitcoind.client.get_new_address(None, None).unwrap();
	address.require_network(Network::Regtest).unwrap()
}

fn add_blocks(bitcoind: &BitcoinD, count: u64, grant_to_address: &Address) {
	bitcoind.client.generate_to_address(count, grant_to_address).unwrap();
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
