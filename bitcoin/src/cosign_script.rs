use alloc::{format, string::ToString, vec};
use core::str::FromStr;

pub use bitcoin::Amount;
use bitcoin::{
	absolute::LockTime,
	transaction::{predict_weight, InputWeightPrediction},
	Address, FeeRate, Network, PublicKey, ScriptBuf,
};
use miniscript::{
	policy::{
		concrete::{DescriptorCtx, Policy},
		Concrete,
	},
	Descriptor, FromStrKey, MiniscriptKey, Segwitv0,
};

use argon_primitives::bitcoin::{BitcoinError, BitcoinHeight, CompressedBitcoinPubkey};

use crate::errors::Error;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum UnlockStep {
	VaultCosign,
	OwnerCosign,
	VaultClaim,
	OwnerClaim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CosignScriptArgs {
	pub vault_pubkey: CompressedBitcoinPubkey,
	pub vault_claim_pubkey: CompressedBitcoinPubkey,
	pub owner_pubkey: CompressedBitcoinPubkey,
	pub vault_claim_height: BitcoinHeight,
	pub open_claim_height: BitcoinHeight,
	pub created_at_height: BitcoinHeight,
}

impl CosignScriptArgs {
	pub fn bitcoin_vault_pubkey(&self) -> Result<bitcoin::PublicKey, BitcoinError> {
		self.vault_pubkey.try_into().map_err(|_| BitcoinError::InvalidPubkey)
	}

	pub fn bitcoin_owner_pubkey(&self) -> Result<bitcoin::PublicKey, BitcoinError> {
		self.owner_pubkey.try_into().map_err(|_| BitcoinError::InvalidPubkey)
	}

	pub fn bitcoin_vault_claim_pubkey(&self) -> Result<bitcoin::PublicKey, BitcoinError> {
		self.vault_claim_pubkey.try_into().map_err(|_| BitcoinError::InvalidPubkey)
	}

	pub fn vault_claim_locktime(&self) -> Result<LockTime, BitcoinError> {
		LockTime::from_height(self.vault_claim_height as u32)
			.map_err(|_| BitcoinError::InvalidLockTime)
	}

	pub fn open_claim_locktime(&self) -> Result<LockTime, BitcoinError> {
		LockTime::from_height(self.open_claim_height as u32)
			.map_err(|_| BitcoinError::InvalidLockTime)
	}
}

#[derive(Clone, Eq, Debug, PartialEq)]
pub struct CosignScript {
	pub script_args: CosignScriptArgs,
	pub policy: Policy<PublicKey>,
	pub script: ScriptBuf,
	pub address: Address,
	pub descriptor: Descriptor<PublicKey>,
}

impl CosignScript {
	pub fn new(cosign_script_args: CosignScriptArgs, network: Network) -> Result<Self, Error> {
		let policy =
			Self::create_policy(&cosign_script_args).map_err(Error::TimelockScriptError)?;
		let descriptor = Self::build_descriptor(&cosign_script_args, &policy)?;
		let script = descriptor.script_code().map_err(|_| BitcoinError::InvalidPolicy)?;
		let address = descriptor.address(network).map_err(|_| Error::AddressError)?;
		Ok(Self { script_args: cosign_script_args, policy, script, address, descriptor })
	}

	pub fn get_script_address(&self) -> Address {
		self.address.clone()
	}

	pub fn get_script_pubkey(&self) -> ScriptBuf {
		self.script.clone().to_p2wsh()
	}

	pub fn set_registered_height(&mut self, height: BitcoinHeight) {
		self.script_args.created_at_height = height;
	}

	pub fn unlock_height(&self, unlock_step: UnlockStep) -> u32 {
		(match unlock_step {
			UnlockStep::OwnerCosign | UnlockStep::VaultCosign => self.script_args.created_at_height,
			UnlockStep::VaultClaim => self.script_args.vault_claim_height,
			UnlockStep::OwnerClaim => self.script_args.open_claim_height,
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

		let mut witness_element_lengths =
			vec![MAX_SIGNATURE_SIZE, COMPRESSED_PUBKEY_SIZE, self.script.len()];

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

	#[rustfmt::skip]
	pub fn create_policy(
		cosign_script_args: &CosignScriptArgs) -> Result<Policy<PublicKey>, BitcoinError> {
		let vault_pubkey = cosign_script_args.bitcoin_vault_pubkey()?;
		let owner_pubkey: PublicKey = cosign_script_args.bitcoin_owner_pubkey()?;
		let vault_claim_pubkey: PublicKey = cosign_script_args.bitcoin_vault_claim_pubkey()?;
		let open_claim_height = cosign_script_args.open_claim_locktime()?;
		let vault_claim_height = cosign_script_args.vault_claim_locktime()?;
		let policy_str = format!(r#"or(
			thresh(2, pk({vault_pubkey}), pk({owner_pubkey}), after({open_claim_height})),
			and(pk({vault_claim_pubkey}), after({vault_claim_height}))
		)"#);
		// strip whitespace
		let policy_str = policy_str.split_whitespace().collect::<alloc::string::String>();
		Concrete::from_str(&policy_str).map_err(|e| {
			log::error!("Miniscript error: {}", e);
			BitcoinError::InvalidPolicy
		})
	}

	pub fn create_descriptor<Pk: MiniscriptKey + FromStrKey>(
		&self,
	) -> Result<Descriptor<Pk>, BitcoinError> {
		Self::get_descriptor::<Pk>(&self.script_args)
	}

	/// Creates a miniscript policy that does the following:
	/// - Until `vault_claim_height`, multisig requires `vault_pubkey` + `owner_pubkey` signatures
	/// - Between `vault_claim_height` and `open_claim_height`, only the `vault_claim_pubkey` can
	///   claim the funds
	/// - After `open_claim_height`, the `owner_pubkey` can claim the funds
	pub fn get_descriptor<Pk: MiniscriptKey + FromStrKey>(
		cosign_script_args: &CosignScriptArgs,
	) -> Result<Descriptor<Pk>, BitcoinError> {
		const COMPILED_DESCRIPTOR: &str = "wsh(andor(pk({vault_claim_pubkey}),after({vault_claim_height}),thresh(2,pk({vault_pubkey}),s:pk({owner_pubkey}),snl:after({open_claim_height}))))";
		let vault_pubkey = cosign_script_args.bitcoin_vault_pubkey()?;
		let owner_pubkey = cosign_script_args.bitcoin_owner_pubkey()?;
		let vault_claim_pubkey = cosign_script_args.bitcoin_vault_claim_pubkey()?;
		let vault_claim_height = cosign_script_args.vault_claim_locktime()?;
		let open_claim_height = cosign_script_args.open_claim_locktime()?;

		let descriptor_str = COMPILED_DESCRIPTOR
			.replace("{vault_pubkey}", &vault_pubkey.to_string())
			.replace("{owner_pubkey}", &owner_pubkey.to_string())
			.replace("{vault_claim_pubkey}", &vault_claim_pubkey.to_string())
			.replace("{vault_claim_height}", &vault_claim_height.to_string())
			.replace("{open_claim_height}", &open_claim_height.to_string());

		let descriptor = miniscript::Descriptor::<Pk>::from_str(&descriptor_str)
			.map_err(|_| BitcoinError::InvalidPolicy)?;
		Ok(descriptor)
	}
	pub fn build_descriptor<Pk: MiniscriptKey + FromStrKey>(
		cosign_script_args: &CosignScriptArgs,
		policy: &Policy<Pk>,
	) -> Result<Descriptor<Pk>, BitcoinError> {
		if option_env!("BUILD_MINISCRIPT_POLICY").is_some() {
			let descriptor = policy
				.compile_to_descriptor::<Segwitv0>(DescriptorCtx::Wsh)
				.map_err(|_| BitcoinError::InvalidPolicy)?;
			#[cfg(debug_assertions)]
			if let Descriptor::Wsh(ref wsh) = descriptor {
				#[cfg(debug_assertions)]
				println!("Miniscript: {}", wsh);
			}

			descriptor.sanity_check().map_err(|_| BitcoinError::UnsafePolicy)?;
			Ok(descriptor)
		} else {
			Self::get_descriptor(cosign_script_args)
		}
	}
}

#[cfg(test)]
mod test {
	use bitcoin::{
		absolute::LockTime, blockdata::script::Script, secp256k1::Secp256k1, Address, Amount,
		CompressedPublicKey, EcdsaSighashType, FeeRate, Network, PrivateKey,
	};
	use bitcoincore_rpc::{jsonrpc::base64, RawTx, RpcApi};
	use bitcoind::BitcoinD;

	use argon_primitives::bitcoin::{
		BitcoinBlock, BitcoinCosignScriptPubkey, BitcoinHeight, BitcoinScriptPubkey,
		BitcoinSignature, BitcoinSyncStatus, CompressedBitcoinPubkey, Satoshis, UtxoRef, UtxoValue,
	};
	use argon_testing::*;

	use crate::{UnlockStep, UtxoSpendFilter, UtxoUnlocker};

	use super::*;

	const NUM_BLOCKS: u32 = 101;

	fn start_bitcoind() -> (BitcoinD, UtxoSpendFilter, Address, Network) {
		let (bitcoind, rpc_url, network) = argon_testing::start_bitcoind().expect("start_bitcoin");

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
		(bitcoind, tracker, block_address, network)
	}

	#[test]
	fn vault_can_claim_the_timelock_script() {
		let (bitcoind, tracker, block_address, network) = start_bitcoind();

		let block_height = bitcoind.client.get_block_count().unwrap();

		let (master_xpriv, fingerprint) = create_xpriv(network);
		let (vault_compressed_pubkey, vault_hd_path) = derive(&master_xpriv, "m/0'/0/1");
		let (vault_claim_pubkey, vault_reclaim_hd_path) = derive(&master_xpriv, "m/0'/1/0");

		let owner_compressed_pubkey =
			"026c468be64d22761c30cd2f12cbc7de255d592d7904b1bab07236897cc4c2e766"
				.parse::<CompressedPublicKey>()
				.unwrap();

		let open_claim_height = block_height + 8;
		let vault_claim_height = block_height + 4;

		let script_args = CosignScriptArgs {
			vault_pubkey: vault_compressed_pubkey.into(),
			vault_claim_pubkey: vault_claim_pubkey.into(),
			owner_pubkey: owner_compressed_pubkey.into(),
			vault_claim_height,
			open_claim_height,
			created_at_height: block_height,
		};
		let cosign_script = CosignScript::new(script_args, network).expect("script");
		let script = cosign_script.script.to_bytes();
		println!("{:#?}", Script::from_bytes(script.as_slice()).to_asm_string());

		let script_address = Address::p2wsh(Script::from_bytes(&script.clone()), network);

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
		let script_args = CosignScriptArgs {
			vault_pubkey: vault_compressed_pubkey.into(),
			vault_claim_pubkey: vault_claim_pubkey.into(),
			owner_pubkey: owner_compressed_pubkey.into(),
			vault_claim_height,
			open_claim_height,
			created_at_height: register_height,
		};
		let cosign_script = CosignScript::new(script_args, network).unwrap();

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

			assert!(unlocker.extract_tx().is_err());
		}
		// fails to allow vault_reclaim key too
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
				.sign_derived(master_xpriv, (fingerprint, vault_reclaim_hd_path.clone()))
				.expect("sign");

			assert!(unlocker.extract_tx().is_err());
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
			.sign_derived(master_xpriv, (fingerprint, vault_reclaim_hd_path.clone()))
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
		drop(bitcoind);
	}

	#[test]
	fn owner_can_reclaim_the_timelock_script() {
		let (bitcoind, tracker, block_address, network) = start_bitcoind();

		let block_height = bitcoind.client.get_block_count().unwrap();

		let (master_xpriv, _) = create_xpriv(network);
		let (vault_compressed_pubkey, _) = derive(&master_xpriv, "m/0'/0/1");
		let vault_claim_pubkey = derive(&master_xpriv, "m/0'/1/0").0;

		let secp = Secp256k1::new();
		let owner_keypair = PrivateKey::generate(network);
		let owner_compressed_pubkey = owner_keypair.public_key(&secp);
		let owner_pubkey: CompressedBitcoinPubkey = owner_compressed_pubkey.into();
		let amount: Satoshis = Amount::ONE_BTC.to_sat() * 5;

		let open_claim_height = block_height + 10;
		let vault_claim_height = block_height + 5;
		let register_height = block_height;
		let script_args = CosignScriptArgs {
			vault_pubkey: vault_compressed_pubkey.into(),
			vault_claim_pubkey: vault_claim_pubkey.into(),
			owner_pubkey,
			vault_claim_height,
			open_claim_height,
			created_at_height: register_height,
		};
		let mut cosign_script = CosignScript::new(script_args, network).unwrap();

		let script_address = cosign_script.get_script_address();

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

			// cannot satisfy
			assert!(unlocker.extract_tx().is_err());

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
		drop(bitcoind);
	}

	#[test]
	fn owner_can_reclaim_the_timelock_script_with_bitcoin_core() {
		let (bitcoind, tracker, block_address, network) = start_bitcoind();

		let block_height = bitcoind.client.get_block_count().unwrap();

		let (master_xpriv, _) = create_xpriv(network);
		let (vault_compressed_pubkey, _) = derive(&master_xpriv, "m/0'/0/1");
		let vault_claim_pubkey = derive(&master_xpriv, "m/0'/1/0").0;

		let owner_address = bitcoind
			.client
			.get_new_address(Some("owner"), None)
			.unwrap()
			.require_network(network)
			.unwrap();

		let owner_address_info = bitcoind.client.get_address_info(&owner_address).unwrap();
		let owner_pubkey = owner_address_info.pubkey.unwrap();
		let amount: Satoshis = Amount::ONE_BTC.to_sat() * 5;

		let open_claim_height = block_height + 10;
		let vault_claim_height = block_height + 5;
		let register_height = block_height;
		let script_args = CosignScriptArgs {
			vault_pubkey: vault_compressed_pubkey.into(),
			vault_claim_pubkey: vault_claim_pubkey.into(),
			owner_pubkey: owner_pubkey.into(),
			vault_claim_height,
			open_claim_height,
			created_at_height: register_height,
		};
		let mut cosign_script = CosignScript::new(script_args, network).unwrap();

		let script_address = cosign_script.get_script_address();

		let (txid, vout, _src_tx) =
			fund_script_address(&bitcoind, &script_address, amount, &block_address);

		let block_height = bitcoind.client.get_block_count().unwrap();

		assert!(block_height < open_claim_height);
		cosign_script.set_registered_height(block_height);

		let fee_rate = FeeRate::from_sat_per_vb(15).expect("cant translate fee");

		let pay_to_script_pubkey = owner_pubkey.p2wpkh_script_code().unwrap();
		let fee = cosign_script
			.calculate_fee(false, pay_to_script_pubkey.clone(), fee_rate)
			.unwrap();

		// cannot accept until the cosign height
		add_blocks(&bitcoind, open_claim_height.saturating_sub(block_height), &block_address);

		assert_eq!(bitcoind.client.get_block_count().unwrap(), open_claim_height);
		{
			let unlocker = UtxoUnlocker::from_script(
				cosign_script.clone(),
				amount,
				txid,
				vout,
				UnlockStep::OwnerClaim,
				fee,
				pay_to_script_pubkey.clone(),
			)
			.expect("unlocker");

			let psbt_text = base64::encode(unlocker.psbt.serialize());
			{
				let analyzed = bitcoind
					.client
					.call::<serde_json::Value>(
						"analyzepsbt",
						&[serde_json::to_value(psbt_text.clone()).unwrap()],
					)
					.unwrap();
				println!("Analyzed Psbt: {:#?}", analyzed);
			}

			let import = bitcoind
				.client
				.wallet_process_psbt(
					&psbt_text,
					Some(true),
					Some(EcdsaSighashType::All.into()),
					None,
				)
				.unwrap();
			let psbt_text = import.psbt.clone();
			{
				let analyzed = bitcoind
					.client
					.call::<serde_json::Value>(
						"analyzepsbt",
						&[serde_json::to_value(&psbt_text).unwrap()],
					)
					.unwrap();
				println!("Analyzed Psbt: {:#?}", analyzed);
			}
			let tx = bitcoind.client.finalize_psbt(&psbt_text, Some(true)).unwrap();
			print!("Finalized: {:?}", tx);
			let tx_hex = tx.transaction().unwrap().unwrap().raw_hex();

			let acceptance =
				bitcoind.client.test_mempool_accept(&[&tx.hex.unwrap()]).expect("checked");

			println!("{:?}", acceptance[0]);
			assert!(acceptance[0].allowed);

			check_spent(
				&tx_hex,
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
		drop(bitcoind);
	}

	#[test]
	fn vault_and_owner_can_cosign() {
		let (bitcoind, tracker, block_address, network) = start_bitcoind();
		// 1. Owner creates a new pubkey and submits to blockchain
		let secp = Secp256k1::new();
		let owner_keypair = PrivateKey::generate(network);
		let owner_compressed_pubkey = owner_keypair.public_key(&secp);
		let owner_pubkey: CompressedBitcoinPubkey = owner_compressed_pubkey.into();
		let amount: Satoshis = Amount::ONE_BTC.to_sat() * 5;

		let (vault_master_xpriv, vault_fingerprint) = create_xpriv(network);
		let (vault_compressed_pubkey, vault_hd_path) =
			derive(&vault_master_xpriv, "m/48'/0'/0'/0/1");
		let vault_claim_pubkey = derive(&vault_master_xpriv, "m/48'/0'/0'/1/0").0;
		let vault_pubkey: CompressedBitcoinPubkey = vault_compressed_pubkey.into();

		let block_height = bitcoind.client.get_block_count().unwrap();

		let open_claim_height = block_height + 20;
		let vault_claim_height = block_height + 10;

		// 2. Vault publishes details script_pubkey and vault pubkey hash

		// 3. Owner recreates the script from the details and submits to blockchain
		let script_address = {
			let script_args = CosignScriptArgs {
				vault_pubkey,
				vault_claim_pubkey: vault_claim_pubkey.into(),
				owner_pubkey,
				vault_claim_height,
				open_claim_height,
				created_at_height: block_height,
			};
			let cosign_script = CosignScript::new(script_args, network).expect("script address");
			cosign_script.get_script_address()
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
			owner_compressed_pubkey.p2wpkh_script_code().unwrap().into();
		let feerate = FeeRate::from_sat_per_vb(15).expect("cant translate fee");
		let script_args = CosignScriptArgs {
			vault_pubkey,
			vault_claim_pubkey: vault_claim_pubkey.into(),
			owner_pubkey,
			vault_claim_height,
			open_claim_height,
			created_at_height: register_height,
		};
		let user_cosign_script = CosignScript::new(script_args, network).unwrap();
		let fee = user_cosign_script
			.calculate_fee(true, out_script_pubkey.clone().into(), feerate)
			.unwrap();

		// 5. vault sees unlock request (outaddress, fee) and creates a transaction
		let (vault_signature, vault_pubkey) = {
			let script_args = CosignScriptArgs {
				vault_pubkey,
				vault_claim_pubkey: vault_claim_pubkey.into(),
				owner_pubkey,
				vault_claim_height,
				open_claim_height,
				created_at_height: register_height,
			};
			let mut unlocker = UtxoUnlocker::new(
				script_args,
				amount,
				utxo.txid.clone().into(),
				utxo.output_index,
				UnlockStep::VaultCosign,
				fee,
				out_script_pubkey.clone().into(),
				network,
			)
			.expect("unlocker");

			let (vault_signature, vault_pubkey) = unlocker
				.sign_derived(vault_master_xpriv, (vault_fingerprint, vault_hd_path))
				.expect("sign");

			// test can verify signature
			let vault_signature_api: BitcoinSignature = vault_signature.try_into().unwrap();
			let vault_pubkey_api: CompressedBitcoinPubkey = vault_pubkey.into();
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
		drop(bitcoind);
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
