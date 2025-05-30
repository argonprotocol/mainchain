use crate::{
	cosign_script::{CosignScript, CosignScriptArgs, ReleaseStep},
	errors::Error,
};
use alloc::vec;
use argon_primitives::{
	bitcoin::{BitcoinError, BitcoinSignature, CompressedBitcoinPubkey, Satoshis},
	ensure,
};
use bitcoin::{
	Amount, EcdsaSighashType, Network, OutPoint, PrivateKey, Psbt, PublicKey, ScriptBuf, Sequence,
	Transaction, TxIn, TxOut, Witness,
	absolute::LockTime,
	bip32::{DerivationPath, Xpriv, Xpub},
	ecdsa::Signature,
	key::Secp256k1,
	psbt::Input,
	sighash::SighashCache,
	transaction::Version,
};
use k256::ecdsa::signature::Verifier;
use log::trace;
use miniscript::psbt::PsbtExt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CosignReleaser {
	pub cosign_script: CosignScript,
	pub release_step: ReleaseStep,
	pub psbt: Psbt,
}

impl CosignReleaser {
	pub fn from_script(
		cosign_script: CosignScript,
		utxo_satoshis: Satoshis,
		utxo_txid: bitcoin::Txid,
		utxo_vout: u32,
		release_step: ReleaseStep,
		fee: Amount,
		to_script_pubkey: ScriptBuf,
	) -> Result<Self, Error> {
		let lock_time = cosign_script.unlock_height(release_step);
		let out_point = OutPoint { txid: utxo_txid, vout: utxo_vout };
		let out_amount = Amount::from_sat(utxo_satoshis);
		let unsigned_tx = Transaction {
			version: Version::TWO, // Post BIP-68.
			lock_time: LockTime::from_height(lock_time)
				.map_err(|_| BitcoinError::InvalidLockTime)?,
			input: vec![TxIn {
				previous_output: out_point,
				sequence: Sequence::ENABLE_LOCKTIME_NO_RBF,
				..TxIn::default()
			}],
			output: vec![TxOut {
				value: out_amount.checked_sub(fee).ok_or(Error::FeeOverflow)?,
				script_pubkey: to_script_pubkey,
			}],
		};

		let mut psbt = Psbt::from_unsigned_tx(unsigned_tx).map_err(Error::PsbtError)?;

		psbt.inputs[0] = Input {
			witness_utxo: Some(TxOut {
				value: out_amount,
				script_pubkey: cosign_script.get_script_pubkey(),
			}),
			witness_script: Some(cosign_script.script.clone()),
			sighash_type: Some(EcdsaSighashType::AllPlusAnyoneCanPay.into()),
			..Input::default()
		};
		let descriptor = cosign_script.create_descriptor()?;
		psbt.update_input_with_descriptor(0, &descriptor).map_err(|_| {
			log::error!("Error updating PSBT with descriptor: {:#?}", descriptor);
			Error::PsbtFinalizeError
		})?;

		Ok(Self { cosign_script, release_step, psbt })
	}

	#[allow(clippy::too_many_arguments)]
	pub fn new(
		cosign_script_args: CosignScriptArgs,
		utxo_satoshis: Satoshis,
		utxo_txid: bitcoin::Txid,
		utxo_vout: u32,
		release_step: ReleaseStep,
		fee: Amount,
		pay_to_script_pubkey: ScriptBuf,
		network: Network,
	) -> Result<Self, Error> {
		Self::from_script(
			CosignScript::new(cosign_script_args, network)?,
			utxo_satoshis,
			utxo_txid,
			utxo_vout,
			release_step,
			fee,
			pay_to_script_pubkey,
		)
	}

	pub fn add_signature(&mut self, pubkey: PublicKey, signature: Signature) {
		self.psbt.inputs[0].partial_sigs.insert(pubkey, signature);
	}

	/// No std friendly version of verifying a signature
	pub fn verify_signature_raw(
		&self,
		pubkey: CompressedBitcoinPubkey,
		signature_der_bytes: &BitcoinSignature,
	) -> Result<bool, Error> {
		let psbt = &self.psbt;
		let mut cache = SighashCache::new(&psbt.unsigned_tx);

		// Get the sighash message
		let (msg, _) = match psbt.sighash_ecdsa(0, &mut cache) {
			Ok(result) => result,
			Err(_) => return Ok(false),
		};

		let (_sighash_type, sigdata) =
			signature_der_bytes.0.split_last().ok_or(Error::InvalidSignatureBytes)?;

		let signature =
			k256::ecdsa::Signature::from_der(sigdata).map_err(|_| Error::InvalidSignatureBytes)?;

		let pubkey = k256::ecdsa::VerifyingKey::from_sec1_bytes(&pubkey.0)
			.map_err(|_| Error::InvalidCompressPubkeyBytes)?;

		Ok(pubkey.verify(msg.as_ref(), &signature).is_ok())
	}

	pub fn sign(&mut self, privkey: PrivateKey) -> Result<(Signature, PublicKey), Error> {
		let psbt = &mut self.psbt;
		let mut cache = SighashCache::new(&psbt.unsigned_tx);
		let (msg, ecdsa_type) = psbt.sighash_ecdsa(0, &mut cache).map_err(Error::SignError)?;
		let secp = Secp256k1::new();
		let sig = secp.sign_ecdsa(&msg, &privkey.inner);
		let signature = Signature { signature: sig, sighash_type: ecdsa_type };
		let pubkey = privkey.public_key(&secp);
		psbt.inputs[0].partial_sigs.insert(pubkey, signature);
		Ok((signature, pubkey))
	}

	pub fn sign_derived(
		&mut self,
		master_xpriv: Xpriv,
		hd_path: DerivationPath,
	) -> Result<(Signature, PublicKey), Error> {
		let psbt = &mut self.psbt;
		let secp = Secp256k1::new();
		let child_xpriv = master_xpriv.derive_priv(&secp, &hd_path).map_err(Error::Bip32Error)?;
		let master_xpub = Xpub::from_priv(&Secp256k1::new(), &master_xpriv);

		let child_priv = child_xpriv.to_priv();
		let pubkey = child_priv.public_key(&secp);

		psbt.inputs[0]
			.bip32_derivation
			.insert(pubkey.inner, (master_xpub.fingerprint(), hd_path.clone()));

		trace!("Signing with derived key: {}", pubkey);

		match psbt.sign(&master_xpriv, &secp) {
			Ok(_) => ensure!(!psbt.inputs[0].partial_sigs.is_empty(), Error::SignatureExpected),
			Err((_, errs)) => return Err(Error::SigningErrors(errs)),
		};

		trace!("Signed: {:?}", psbt.inputs[0].partial_sigs.len());
		let Some((_, signature)) =
			psbt.inputs[0].partial_sigs.iter().find(|(k, _)| k.inner == pubkey.inner)
		else {
			return Err(Error::DerivedKeySignError);
		};

		Ok((*signature, pubkey))
	}

	pub fn create_witness(&mut self) -> Result<(), Error> {
		let mut witness = Witness::new();
		let psbt = &mut self.psbt;
		let partial_sigs = &psbt.inputs[0].partial_sigs;
		let owner_pubkey = self.cosign_script.script_args.bitcoin_owner_pubkey()?;

		let vault_pubkey = self.cosign_script.script_args.bitcoin_vault_pubkey()?;

		let vault_claim_pubkey = self.cosign_script.script_args.bitcoin_vault_claim_pubkey()?;

		if let Some(sig) = partial_sigs.get(&vault_pubkey) {
			witness.push(sig.to_vec());
		}
		if let Some(sig) = partial_sigs.get(&vault_claim_pubkey) {
			witness.push(sig.to_vec());
		}

		if let Some(sig) = partial_sigs.get(&owner_pubkey) {
			witness.push(sig.to_vec());
		}
		witness.push(self.cosign_script.script.clone());

		psbt.inputs[0].final_script_witness = Some(witness);
		Ok(())
	}

	pub fn extract_tx(&mut self) -> Result<Transaction, Error> {
		let tx = {
			let mut psbt = self.psbt.clone();
			psbt = psbt.finalize(&Secp256k1::new()).map_err(|(_, e)| {
				log::error!("Error finalizing PSBT: {:#?}", e);
				Error::PsbtFinalizeError
			})?;
			psbt.extract_tx().map_err(Error::ExtractTxError)?
		};

		// Clear all the data fields as per the spec.
		{
			let psbt = &mut self.psbt;
			psbt.inputs[0].partial_sigs.clear();
			psbt.inputs[0].sighash_type = None;
			psbt.inputs[0].redeem_script = None;
			psbt.inputs[0].witness_script = None;
			psbt.inputs[0].bip32_derivation.clear();
		}

		Ok(tx)
	}

	/// Broadcasts the transaction to a Bitcoin node. NOTE: You must return `true` from the
	/// `on_status` callback to break the loop and return from this function.
	///
	/// # Arguments
	/// * `url` - The URL of the Bitcoin node to broadcast the transaction to.
	/// * `status_check_delay` - The delay between status checks for the transaction.
	/// * `on_status` - A callback function that is called with the transaction status. If it
	///   returns `true`, the function will return successfully.
	#[cfg(feature = "std")]
	pub async fn broadcast<F>(
		&mut self,
		url: &str,
		status_check_delay: std::time::Duration,
		on_status: F,
	) -> Result<(), Error>
	where
		F: Fn(bitcoincore_rpc::json::GetRawTransactionResult) -> bool + Send + Sync + 'static,
	{
		use bitcoincore_rpc::RpcApi;

		let tx = self.extract_tx()?;
		let url = url.parse::<url::Url>().map_err(|e| Error::BroadcastError(e.to_string()))?;
		let auth = if url.username().is_empty() && url.password().is_none() {
			bitcoincore_rpc::Auth::None
		} else {
			bitcoincore_rpc::Auth::UserPass(
				url.username().to_string(),
				url.password().unwrap_or("").to_string(),
			)
		};
		let client = bitcoincore_rpc::Client::new(url.as_str(), auth)
			.map_err(|e| Error::BroadcastError(e.to_string()))?;
		let txid = client
			.send_raw_transaction(&tx)
			.map_err(|e| Error::BroadcastError(e.to_string()))?;
		log::info!("Transaction broadcast with txid: {}", txid);
		// wait for the tx to be confirmed
		loop {
			let info = client
				.get_raw_transaction_info(&txid, None)
				.map_err(|e| Error::BroadcastError(e.to_string()))?;
			if on_status(info) {
				return Ok(());
			}
			tokio::time::sleep(status_check_delay).await;
		}
	}
}

#[cfg(feature = "hwi")]
mod hwi_ext {
	use super::*;
	use anyhow::{Result, anyhow, bail};
	use hwi::{HWIClient, types::HWIDevice};

	impl CosignReleaser {
		pub fn sign_hwi(
			&mut self,
			key_source: KeySource,
			device: Option<HWIDevice>,
			network: Network,
		) -> Result<(Signature, PublicKey)> {
			let psbt = &mut self.psbt;
			let mut device = device;
			if device.is_none() {
				let devices = HWIClient::enumerate()
					.map_err(|e| anyhow!("Error enumerating devices: {:?}", e))?;

				for d in devices.into_iter().flatten() {
					device = Some(d);
				}
			};
			let device = device.ok_or(anyhow!("No device found"))?;

			let client = HWIClient::get_client(&device, false, network.into())?;
			let x_pubkey = client.get_xpub(&key_source.1, false)?;
			let pubkey = x_pubkey.public_key;

			psbt.inputs[0].bip32_derivation.insert(pubkey, key_source);

			psbt.combine(client.sign_tx(psbt)?.psbt)?;
			let Some((_, signature)) =
				psbt.inputs[0].partial_sigs.iter().find(|(k, _)| k.inner == pubkey)
			else {
				bail!("Could not sign with hardware wallet");
			};

			Ok((*signature, pubkey.into()))
		}
	}
}
