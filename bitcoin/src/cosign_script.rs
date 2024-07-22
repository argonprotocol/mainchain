use alloc::vec;
use bitcoin::{
	transaction::{predict_weight, InputWeightPrediction},
	Address, FeeRate, Network, ScriptBuf,
};

use ulx_primitives::bitcoin::{BitcoinError, BitcoinHeight, BitcoinPubkeyHash};

use crate::errors::Error;
pub use bitcoin::Amount;

pub const COSIGN_CODE: u8 = 1;
pub const VAULT_CLAIM_CODE: u8 = 2;
pub const OPEN_CLAIM_CODE: u8 = 3;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum UnlockStep {
	VaultCosign,
	OwnerCosign,
	VaultClaim,
	OwnerClaim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CosignScript {
	pub vault_pubkey_hash: BitcoinPubkeyHash,
	pub owner_pubkey_hash: BitcoinPubkeyHash,
	pub vault_claim_height: BitcoinHeight,
	pub open_claim_height: BitcoinHeight,
	pub created_at_height: BitcoinHeight,
	pub script: ScriptBuf,
}

impl CosignScript {
	pub fn new(
		vault_pubkey_hash: BitcoinPubkeyHash,
		owner_pubkey_hash: BitcoinPubkeyHash,
		vault_claim_height: BitcoinHeight,
		open_claim_height: BitcoinHeight,
		created_at_height: BitcoinHeight,
	) -> Result<Self, Error> {
		let script = Self::create_script(
			vault_pubkey_hash,
			owner_pubkey_hash,
			vault_claim_height,
			open_claim_height,
		)
		.map_err(|e| Error::TimelockScriptError(e))?;
		Ok(Self {
			vault_pubkey_hash,
			owner_pubkey_hash,
			vault_claim_height,
			open_claim_height,
			created_at_height,
			script,
		})
	}

	pub fn get_script_address(&self, network: Network) -> Address {
		Address::p2wsh(&self.script, network)
	}

	pub fn get_script_pubkey(&self) -> ScriptBuf {
		self.script.clone().to_p2wsh()
	}

	pub fn set_registered_height(&mut self, height: BitcoinHeight) {
		self.created_at_height = height;
	}

	pub fn unlock_height(&self, unlock_step: UnlockStep) -> u32 {
		(match unlock_step {
			UnlockStep::OwnerCosign | UnlockStep::VaultCosign => self.created_at_height,
			UnlockStep::VaultClaim => self.vault_claim_height,
			UnlockStep::OwnerClaim => self.open_claim_height,
		}) as u32
	}
	pub fn calculate_fee(
		&self,
		is_cosign: bool,
		to_script_pubkey: ScriptBuf,
		fee_rate: FeeRate,
	) -> Result<Amount, Error> {
		const MAX_SIGNATURE_SIZE: usize = 73;
		const COMPRESSED_PUBKEY_SIZE: usize = 33;
		const UNLOCK_CODE_SIZE: usize = 1;

		let mut witness_element_lengths =
			vec![MAX_SIGNATURE_SIZE, COMPRESSED_PUBKEY_SIZE, UNLOCK_CODE_SIZE, self.script.len()];

		if is_cosign {
			witness_element_lengths.push(MAX_SIGNATURE_SIZE);
			witness_element_lengths.push(COMPRESSED_PUBKEY_SIZE);
		}
		let weight = predict_weight(
			vec![InputWeightPrediction::from_slice(0, witness_element_lengths.as_slice())],
			vec![to_script_pubkey.len()],
		);
		let Some(fee) = fee_rate.fee_wu(weight) else { return Err(Error::FeeTooLow) };
		Ok(fee)
	}

	/// Creates a bitcoin script the does the following:
	/// - Until `vault_claim_height`, multisig requires both public keys and signatures to be
	///   revealed
	/// - Between `vault_claim_height` and `open_claim_height`, only the vault can claim the funds
	/// - After `open_claim_hei
	#[rustfmt::skip]
	pub fn create_script(
		vault_pubkey_hash: BitcoinPubkeyHash,
		owner_pubkey_hash: BitcoinPubkeyHash,
		vault_claim_height: BitcoinHeight,
		open_claim_height: BitcoinHeight,
	) -> Result<bitcoin::ScriptBuf, BitcoinError> {
		use bitcoin::blockdata::{opcodes::all::*, script::Builder};
		use bitcoin::absolute::LockTime;

		let script = Builder::new()
			// code 1 is unlock
			.push_opcode(OP_DUP)
			.push_int(COSIGN_CODE as i64)
			.push_opcode(OP_EQUAL)
			.push_opcode(OP_IF)
			.push_opcode(OP_DROP)
			.push_opcode(OP_DUP)
			.push_opcode(OP_HASH160)
			.push_slice(vault_pubkey_hash.0)
			// set 1 to stack if this is the vault
			.push_opcode(OP_EQUALVERIFY)
			.push_opcode(OP_CHECKSIGVERIFY)

			// now consume user key
			.push_opcode(OP_DUP)
			.push_opcode(OP_HASH160)
			.push_slice(owner_pubkey_hash.0)
			//  OP_EQUALVERIFY OP_CHECKSIG at end

			.push_opcode(OP_ELSE)
			.push_int(VAULT_CLAIM_CODE as i64)
			.push_opcode(OP_EQUAL)
			// code 2 is vault claim
			.push_opcode(OP_IF)
			.push_lock_time(LockTime::from_height(vault_claim_height as u32).map_err(|_| BitcoinError::InvalidLockTime)?)
			.push_opcode(OP_CLTV)
			.push_opcode(OP_DROP)

			.push_opcode(OP_DUP)
			.push_opcode(OP_HASH160)
			.push_slice(vault_pubkey_hash.0)
			//  OP_EQUALVERIFY OP_CHECKSIG at end

			// code 3 is owner claim
			.push_opcode(OP_ELSE)
			.push_lock_time(LockTime::from_height(open_claim_height as u32).map_err(|_| BitcoinError::InvalidLockTime)?)
			.push_opcode(OP_CLTV)
			.push_opcode(OP_DROP)

			.push_opcode(OP_DUP)
			.push_opcode(OP_HASH160)
			.push_slice(owner_pubkey_hash.0)
			//  OP_EQUALVERIFY OP_CHECKSIG at end
			.push_opcode(OP_ENDIF)
			.push_opcode(OP_ENDIF)


			.push_opcode(OP_EQUALVERIFY)
			.push_opcode(OP_CHECKSIG)


			.into_script();
		Ok(script)
	}
}

#[cfg(test)]
mod test {
	use bitcoin::{
		absolute::LockTime, blockdata::script::Script, secp256k1::Secp256k1, Address, Amount,
		CompressedPublicKey, FeeRate, Network, PrivateKey,
	};
	use bitcoincore_rpc::{RawTx, RpcApi};
	use bitcoind::BitcoinD;

	use ulx_primitives::bitcoin::{
		BitcoinBlock, BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinPubkeyHash,
		BitcoinScriptPubkey, BitcoinSignature, BitcoinSyncStatus, CompressedBitcoinPubkey,
		Satoshis, UtxoRef, UtxoValue,
	};
	use ulx_testing::*;

	use crate::{UnlockStep, UtxoSpendFilter, UtxoUnlocker};

	use super::*;

	const NUM_BLOCKS: u32 = 101;

	fn start_bitcoind() -> (BitcoinD, UtxoSpendFilter, Address) {
		let (bitcoind, rpc_url) = ulx_testing::start_bitcoind().expect("start_bitcoin");

		let block_address = add_wallet_address(&bitcoind);
		add_blocks(&bitcoind, NUM_BLOCKS as u64, &block_address);

		let auth = if !rpc_url.username().is_empty() {
			Some((
				rpc_url.username().to_string(),
				rpc_url.password().unwrap_or_default().to_string(),
			))
		} else {
			None
		};

		let tracker = UtxoSpendFilter::new(rpc_url.origin().unicode_serialization(), auth).unwrap();
		(bitcoind, tracker, block_address)
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

		let script = CosignScript::create_script(
			vault_compressed_pubkey.pubkey_hash().into(),
			owner_compressed_pubkey.pubkey_hash().into(),
			vault_claim_height,
			open_claim_height,
		)
		.expect("script")
		.to_bytes();
		println!("{:#?}", Script::from_bytes(script.as_slice()).to_asm_string());

		let script_address = Address::p2wsh(Script::from_bytes(&script.clone()), Network::Regtest);

		let (txid, vout, src_tx) = fund_script_address(
			&bitcoind,
			&script_address,
			Amount::ONE_BTC.to_sat(),
			&block_address,
		);

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
				vout,
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
			assert!(!did_accept.allowed);
			let reject = did_accept.reject_reason.as_ref().unwrap();
			assert!(reject.contains("Locktime requirement not satisfied"));
		}

		let mut unlocker = UtxoUnlocker::from_script(
			cosign_script.clone(),
			Amount::ONE_BTC.to_sat(),
			txid,
			vout,
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
			let acceptance =
				bitcoind.client.test_mempool_accept(&[tx_hex.clone()]).expect("checked");

			println!("{:?}", acceptance[0]);
			assert!(!acceptance[0].allowed);
			let reject = acceptance[0].reject_reason.as_ref().unwrap();
			assert!(reject.contains("non-final"));
		}

		// cannot accept until the cosign height
		let mut block_height = block_height;
		while block_height < vault_claim_height {
			let acceptance =
				bitcoind.client.test_mempool_accept(&[tx_hex.clone()]).expect("checked");
			assert!(!acceptance[0].allowed);
			add_blocks(&bitcoind, 1, &block_address);
			block_height = bitcoind.client.get_block_count().unwrap();
		}

		assert_eq!(bitcoind.client.get_block_count().unwrap(), vault_claim_height);
		{
			let acceptance =
				bitcoind.client.test_mempool_accept(&[tx_hex.clone()]).expect("checked");

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
			owner_pubkey_hash,
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
			assert!(!did_accept.allowed);
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
				&&bitcoind,
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
		let (vault_compressed_pubkey, vault_hd_path) =
			derive(&vault_master_xpriv, "m/48'/0'/0'/0/1");
		let vault_pubkey_hash: BitcoinPubkeyHash = vault_compressed_pubkey.pubkey_hash().into();

		let block_height = bitcoind.client.get_block_count().unwrap();

		let open_claim_height = block_height + 20;
		let vault_claim_height = block_height + 10;

		// 2. Vault publishes details script_pubkey and vault pubkey hash

		// 3. Owner recreates the script from the details and submits to blockchain
		let script_address = {
			let cosign_script = CosignScript::new(
				vault_pubkey_hash,
				owner_pubkey_hash,
				vault_claim_height,
				open_claim_height,
				block_height,
			)
			.expect("script address");
			cosign_script.get_script_address(Network::Regtest)
		};

		let utxo_script_pubkey: BitcoinCosignScriptPubkey =
			script_address.clone().try_into().expect("can convert address to script");

		let (txid, _vout, tx) =
			fund_script_address(&bitcoind, &script_address, amount, &block_address);

		let source_txin = tx.transaction().unwrap().raw_hex();
		let block_hash = tx.blockhash.unwrap();
		let block_height = bitcoind.client.get_block_count().unwrap();
		let register_height = block_height;

		tracker
			.sync_to_block(&BitcoinSyncStatus {
				confirmed_block: BitcoinBlock { block_hash: block_hash.into(), block_height },
				synched_block: None,
				oldest_allowed_block_height: block_height - 10,
			})
			.expect("sync");

		let sync = tracker
			.refresh_utxo_status(vec![(
				None,
				UtxoValue {
					utxo_id: 1,
					satoshis: amount,
					script_pubkey: utxo_script_pubkey,
					submitted_at_height: block_height,
					watch_for_spent_until_height: open_claim_height,
				},
			)])
			.unwrap();
		assert_eq!(sync.verified.len(), 1);
		let utxo = sync.verified.get(&1).unwrap();
		assert_eq!(utxo.txid, txid.into());

		// 4. User submits the out address
		let out_script_pubkey: BitcoinScriptPubkey =
			owner_compressed_pubkey.p2wpkh_script_code().unwrap().try_into().unwrap();
		let feerate = FeeRate::from_sat_per_vb(15).expect("cant translate fee");
		let user_cosign_script = CosignScript::new(
			vault_pubkey_hash,
			owner_pubkey_hash,
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
				vault_pubkey_hash.into(),
				owner_pubkey_hash.into(),
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

			let (vault_signature, vault_pubkey) = unlocker
				.sign_derived(vault_master_xpriv, (vault_fingerprint, vault_hd_path))
				.expect("sign");

			// test can verify signature
			let vault_signature_api: BitcoinSignature = vault_signature.clone().try_into().unwrap();
			let vault_pubkey_api: CompressedBitcoinPubkey = vault_pubkey.clone().into();
			assert!(unlocker.verify_signature_raw(vault_pubkey_api, &vault_signature_api).is_ok());
			(vault_signature, vault_pubkey)
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
			unlocker.add_signature(vault_pubkey, vault_signature);
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
				script_pubkey: utxo_script_pubkey,
				submitted_at_height: register_height,
				watch_for_spent_until_height: open_claim_height,
			},
			&block_address,
		);
	}

	fn check_spent(
		tx_hex: &str,
		tracker: &UtxoSpendFilter,
		bitcoind: &BitcoinD,
		utxo_ref: UtxoRef,
		utxo_value: UtxoValue,
		block_address: &Address,
	) {
		let final_txid = bitcoind.client.send_raw_transaction(tx_hex).expect("sent");
		let tx_result = wait_for_txid(bitcoind, &final_txid, block_address);
		let tx_block_height = bitcoind
			.client
			.get_block_header_info(&tx_result.blockhash.unwrap())
			.unwrap()
			.height;
		let block_height = bitcoind.client.get_block_count().unwrap();
		let block_hash = bitcoind.client.get_best_block_hash().unwrap();

		tracker
			.sync_to_block(&BitcoinSyncStatus {
				confirmed_block: BitcoinBlock { block_hash: block_hash.into(), block_height },
				synched_block: None,
				oldest_allowed_block_height: block_height - 10,
			})
			.expect("sync");

		let latest =
			tracker.refresh_utxo_status(vec![(Some(utxo_ref), utxo_value)]).expect("sync 2");
		assert_eq!(latest.spent.len(), 1);

		assert_eq!(latest.spent.get(&1), Some(&(tx_block_height as BitcoinHeight)));
	}
}
