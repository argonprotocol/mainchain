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
pub enum ReleaseStep {
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
		let policy = Self::create_policy(&cosign_script_args).map_err(Error::from)?;
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

	pub fn unlock_height(&self, release_step: ReleaseStep) -> u32 {
		(match release_step {
			ReleaseStep::OwnerCosign | ReleaseStep::VaultCosign =>
				self.script_args.created_at_height,
			ReleaseStep::VaultClaim => self.script_args.vault_claim_height,
			ReleaseStep::OwnerClaim => self.script_args.open_claim_height,
		}) as u32
	}

	pub fn calculate_fee(
		&self,
		is_cosign: bool,
		input_count: usize,
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
		if input_count == 0 {
			return Err(Error::NoUtxos)
		}
		let weight = predict_weight(
			vec![
				InputWeightPrediction::from_slice(0, witness_element_lengths.as_slice());
				input_count
			],
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
			log::error!("Miniscript error: {e}");
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
			#[cfg(all(debug_assertions, feature = "std"))]
			if let Descriptor::Wsh(ref wsh) = descriptor {
				println!("Miniscript: {wsh}");
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
		BitcoinSignature, BitcoinSyncStatus, CompressedBitcoinPubkey, Satoshis, UtxoAddress,
		UtxoRef,
	};
	use argon_testing::*;
	use serial_test::serial;

	use crate::{CosignReleaser, ReleaseStep, UtxoSpendFilter};

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
	#[serial]
	fn vault_can_claim_the_timelock_script() {
		let (bitcoind, tracker, block_address, network) = start_bitcoind();

		let block_height = bitcoind.client.get_block_count().unwrap();

		let (master_xpriv, _fingerprint) = create_xpriv(network);
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

		println!("{src_tx:#?} #{vout:?}");

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
			.calculate_fee(false, 1, pay_to_script_pubkey.clone(), fee_rate)
			.unwrap();

		// fails locktime if not cleared
		{
			let mut unlocker = CosignReleaser::from_script(
				cosign_script.clone(),
				vec![(UtxoRef { txid: txid.into(), output_index: vout }, Amount::ONE_BTC.to_sat())],
				ReleaseStep::VaultClaim,
				fee,
				pay_to_script_pubkey.clone(),
			)
			.expect("unlocker");

			unlocker.psbt.unsigned_tx.lock_time = LockTime::from_consensus(block_height as u32);
			unlocker.sign_derived(master_xpriv, vault_hd_path.clone()).expect("sign");

			assert!(unlocker.extract_tx().is_err());
		}
		// fails to allow vault_reclaim key too
		{
			let mut unlocker = CosignReleaser::from_script(
				cosign_script.clone(),
				vec![(UtxoRef { txid: txid.into(), output_index: vout }, Amount::ONE_BTC.to_sat())],
				ReleaseStep::VaultClaim,
				fee,
				pay_to_script_pubkey.clone(),
			)
			.expect("unlocker");

			unlocker.psbt.unsigned_tx.lock_time = LockTime::from_consensus(block_height as u32);
			unlocker
				.sign_derived(master_xpriv, vault_reclaim_hd_path.clone())
				.expect("sign");

			assert!(unlocker.extract_tx().is_err());
		}

		let mut unlocker = CosignReleaser::from_script(
			cosign_script.clone(),
			vec![(UtxoRef { txid: txid.into(), output_index: vout }, Amount::ONE_BTC.to_sat())],
			ReleaseStep::VaultClaim,
			fee,
			pay_to_script_pubkey.clone(),
		)
		.expect("unlocker");

		unlocker
			.sign_derived(master_xpriv, vault_reclaim_hd_path.clone())
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
			UtxoAddress {
				lock_id: 1,
				script_pubkey: cosign_script.get_script_pubkey().try_into().unwrap(),
				submitted_at_height: register_height,
			},
			1,
			&block_address,
		);
		drop(bitcoind);
	}

	#[test]
	#[serial]
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
			.calculate_fee(false, 1, pay_to_script_pubkey.clone(), fee_rate)
			.unwrap();

		// cannot accept until the cosign height
		let mut block_height = block_height;
		while block_height < open_claim_height {
			let mut unlocker = CosignReleaser::from_script(
				cosign_script.clone(),
				vec![(UtxoRef { txid: txid.into(), output_index: vout }, amount)],
				ReleaseStep::OwnerClaim,
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
			let mut unlocker = CosignReleaser::from_script(
				cosign_script.clone(),
				vec![(UtxoRef { txid: txid.into(), output_index: vout }, amount)],
				ReleaseStep::OwnerClaim,
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
				UtxoAddress {
					lock_id: 1,
					script_pubkey: cosign_script.get_script_pubkey().try_into().unwrap(),
					submitted_at_height: register_height,
				},
				1,
				&block_address,
			);
		}
		drop(bitcoind);
	}

	#[test]
	#[serial]
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
			.calculate_fee(false, 1, pay_to_script_pubkey.clone(), fee_rate)
			.unwrap();

		// cannot accept until the cosign height
		add_blocks(&bitcoind, open_claim_height.saturating_sub(block_height), &block_address);

		assert_eq!(bitcoind.client.get_block_count().unwrap(), open_claim_height);
		{
			let unlocker = CosignReleaser::from_script(
				cosign_script.clone(),
				vec![(UtxoRef { txid: txid.into(), output_index: vout }, amount)],
				ReleaseStep::OwnerClaim,
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
				println!("Analyzed Psbt: {analyzed:#?}");
			}

			let import = bitcoind
				.client
				.wallet_process_psbt(
					&psbt_text,
					Some(true),
					Some(EcdsaSighashType::AllPlusAnyoneCanPay.into()),
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
				println!("Analyzed Psbt: {analyzed:#?}");
			}
			let tx = bitcoind.client.finalize_psbt(&psbt_text, Some(true)).unwrap();
			print!("Finalized: {tx:?}");
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
				UtxoAddress {
					lock_id: 1,
					script_pubkey: cosign_script.get_script_pubkey().try_into().unwrap(),
					submitted_at_height: register_height,
				},
				1,
				&block_address,
			);
		}
		drop(bitcoind);
	}

	#[test]
	#[serial]
	fn vault_and_owner_can_cosign() {
		let (bitcoind, tracker, block_address, network) = start_bitcoind();
		// 1. Owner creates a new pubkey and submits to blockchain
		let secp = Secp256k1::new();
		let owner_keypair = PrivateKey::generate(network);
		let owner_compressed_pubkey = owner_keypair.public_key(&secp);
		let owner_pubkey: CompressedBitcoinPubkey = owner_compressed_pubkey.into();
		let amount: Satoshis = Amount::ONE_BTC.to_sat() * 5;

		let (vault_master_xpriv, _vault_fingerprint) = create_xpriv(network);
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

		let first_amount = amount / 2;
		let second_amount = amount - first_amount;
		let (first_txid, _first_vout, first_tx) =
			fund_script_address(&bitcoind, &script_address, first_amount, &block_address);
		let (second_txid, _second_vout, second_tx) =
			fund_script_address(&bitcoind, &script_address, second_amount, &block_address);

		let source_txins =
			[first_tx.transaction().unwrap().raw_hex(), second_tx.transaction().unwrap().raw_hex()];
		let block_hash = second_tx.blockhash.unwrap();
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
			.refresh_utxo_status(
				vec![(
					None,
					UtxoAddress {
						lock_id: 1,
						script_pubkey: utxo_script_pubkey,
						submitted_at_height: block_height,
					},
				)],
				1000,
			)
			.unwrap();
		assert_eq!(sync.funded.len(), 2);
		assert!(sync.funded.iter().any(|funding| funding.utxo_ref.txid == first_txid.into()));
		assert!(sync.funded.iter().any(|funding| funding.utxo_ref.txid == second_txid.into()));
		let utxos = sync
			.funded
			.iter()
			.map(|funding| (funding.utxo_ref.clone(), funding.satoshis))
			.collect::<Vec<_>>();

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
			.calculate_fee(true, utxos.len(), out_script_pubkey.clone().into(), feerate)
			.unwrap();

		// 5. vault sees unlock request (outaddress, fee) and creates a transaction
		let vault_signatures = {
			let script_args = CosignScriptArgs {
				vault_pubkey,
				vault_claim_pubkey: vault_claim_pubkey.into(),
				owner_pubkey,
				vault_claim_height,
				open_claim_height,
				created_at_height: register_height,
			};
			let mut unlocker = CosignReleaser::new(
				script_args,
				utxos.clone(),
				ReleaseStep::VaultCosign,
				fee,
				out_script_pubkey.clone().into(),
				network,
			)
			.expect("unlocker");

			let vault_signatures =
				unlocker.sign_derived(vault_master_xpriv, vault_hd_path).expect("sign");
			assert_eq!(vault_signatures.len(), 2);

			// test can verify signature
			let vault_signature_api = vault_signatures
				.iter()
				.map(|(signature, _)| signature.clone().try_into().unwrap())
				.collect::<Vec<BitcoinSignature>>();
			let vault_pubkey_api: CompressedBitcoinPubkey = vault_signatures[0].1.into();
			assert!(unlocker
				.verify_signatures_raw(vault_pubkey_api, &vault_signature_api)
				.unwrap());
			vault_signatures
		};

		// 6. User sees the transaction and cosigns
		let tx = {
			let mut unlocker = CosignReleaser::from_script(
				user_cosign_script.clone(),
				utxos.clone(),
				ReleaseStep::OwnerCosign,
				fee,
				out_script_pubkey.clone().into(),
			)
			.unwrap();
			for (input_index, (vault_signature, vault_pubkey)) in
				vault_signatures.into_iter().enumerate()
			{
				unlocker.add_signature(input_index, vault_pubkey, vault_signature).unwrap();
			}
			unlocker.sign(owner_keypair).expect("sign");
			unlocker.extract_tx().expect("tx")
		};

		println!("{tx:#?}");
		let tx_hex = tx.raw_hex();

		let acceptance = bitcoind.client.test_mempool_accept(&[tx_hex.clone()]).expect("checked");
		let did_accept = acceptance.first().unwrap();
		println!("{did_accept:?}");
		println!("btcdeb --tx={tx_hex:?} --txin={source_txins:?}");
		assert!(did_accept.allowed);

		check_spent(
			tx_hex.as_str(),
			&tracker,
			&bitcoind,
			utxos[0].0.clone(),
			UtxoAddress {
				lock_id: 1,
				script_pubkey: utxo_script_pubkey,
				submitted_at_height: register_height,
			},
			2,
			&block_address,
		);
		drop(bitcoind);
	}

	#[test]
	fn vault_uploaded_xpub_root_can_sign_release_child() {
		let network = Network::Regtest;
		let secp = Secp256k1::new();
		let amount: Satoshis = Amount::ONE_BTC.to_sat();
		let (master_xpriv, _) = create_xpriv(network);
		let uploaded_vault_xpub_path: bitcoin::bip32::DerivationPath = "m/0'".parse().unwrap();
		let uploaded_vault_xpriv = master_xpriv
			.derive_priv(&secp, &uploaded_vault_xpub_path)
			.expect("uploaded vault xpub root");
		let (vault_compressed_pubkey, _) = derive(&master_xpriv, "m/0'/1");
		let vault_claim_pubkey = derive(&master_xpriv, "m/0'/2").0;
		let owner_pubkey: CompressedBitcoinPubkey =
			PrivateKey::generate(network).public_key(&secp).into();
		let vault_pubkey: CompressedBitcoinPubkey = vault_compressed_pubkey.into();
		let out_script_pubkey = vault_compressed_pubkey.p2wpkh_script_code();
		let script_args = CosignScriptArgs {
			vault_pubkey,
			vault_claim_pubkey: vault_claim_pubkey.into(),
			owner_pubkey,
			vault_claim_height: 120,
			open_claim_height: 240,
			created_at_height: 100,
		};
		let cosign_script = CosignScript::new(script_args.clone(), network).unwrap();
		let fee_rate = FeeRate::from_sat_per_vb(15).unwrap();
		let one_input_fee = cosign_script
			.calculate_fee(true, 1, out_script_pubkey.clone(), fee_rate)
			.unwrap();
		let two_input_fee = cosign_script
			.calculate_fee(true, 2, out_script_pubkey.clone(), fee_rate)
			.unwrap();
		assert!(two_input_fee > one_input_fee);
		let fee = Amount::from_sat(500);
		let first_utxo_ref = UtxoRef {
			txid: "0000000000000000000000000000000000000000000000000000000000000001"
				.parse::<bitcoin::Txid>()
				.unwrap()
				.into(),
			output_index: 0,
		};
		let second_utxo_ref = UtxoRef {
			txid: "0000000000000000000000000000000000000000000000000000000000000002"
				.parse::<bitcoin::Txid>()
				.unwrap()
				.into(),
			output_index: 1,
		};
		let mut releaser = CosignReleaser::new(
			script_args,
			vec![(second_utxo_ref.clone(), amount / 2), (first_utxo_ref.clone(), amount / 2)],
			ReleaseStep::VaultCosign,
			fee,
			out_script_pubkey,
			network,
		)
		.expect("unlocker");
		assert_eq!(
			releaser.psbt.unsigned_tx.input[0].previous_output,
			bitcoin::OutPoint {
				txid: first_utxo_ref.txid.into(),
				vout: first_utxo_ref.output_index,
			}
		);
		assert_eq!(
			releaser.psbt.unsigned_tx.input[1].previous_output,
			bitcoin::OutPoint {
				txid: second_utxo_ref.txid.into(),
				vout: second_utxo_ref.output_index,
			}
		);

		let mut signatures = releaser
			.sign_derived(
				uploaded_vault_xpriv,
				alloc::vec![bitcoin::bip32::ChildNumber::from_normal_idx(1).unwrap()].into(),
			)
			.expect("sign");
		assert_eq!(signatures.len(), 2);
		let signed_pubkey = signatures[0].1;

		assert_eq!(signed_pubkey, vault_compressed_pubkey.into());
		let vault_signatures_api = signatures
			.drain(..)
			.map(|(signature, _)| signature.try_into().unwrap())
			.collect::<Vec<BitcoinSignature>>();
		assert!(!releaser
			.verify_signatures_raw(vault_pubkey, &vault_signatures_api[..1])
			.unwrap());
		let mut wrong_sighash_signatures = vault_signatures_api.clone();
		*wrong_sighash_signatures[0].0.last_mut().unwrap() = EcdsaSighashType::All.to_u32() as u8;
		assert!(!releaser.verify_signatures_raw(vault_pubkey, &wrong_sighash_signatures).unwrap());
		assert!(releaser.verify_signatures_raw(vault_pubkey, &vault_signatures_api).unwrap());
	}

	fn check_spent(
		tx_hex: &str,
		tracker: &UtxoSpendFilter,
		bitcoind: &BitcoinD,
		utxo_ref: UtxoRef,
		utxo_value: UtxoAddress,
		expected_spent: usize,
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

		let latest = tracker
			.refresh_utxo_status(vec![(Some(utxo_ref), utxo_value)], 1000)
			.expect("sync 2");
		assert_eq!(latest.spent.len(), expected_spent);
		let spend = &latest.spent[0];
		assert_eq!(spend.lock_id, 1);
		assert_eq!(spend.bitcoin_height, tx_block_height as BitcoinHeight);
	}
}
