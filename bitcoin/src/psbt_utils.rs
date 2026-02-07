use crate::Error;
use alloc::vec;
use argon_primitives::{
	bitcoin::{BitcoinSignature, CompressedBitcoinPubkey},
	ensure,
};
use bitcoin::{
	PrivateKey, Psbt, PublicKey, Transaction,
	bip32::{DerivationPath, Xpriv, Xpub},
	ecdsa::Signature,
	key::Secp256k1,
	sighash::SighashCache,
};
use k256::ecdsa::signature::hazmat::PrehashVerifier;
use log::trace;
use miniscript::psbt::PsbtExt;

pub fn finalize(psbt: &Psbt) -> Result<Psbt, Error> {
	let secp = Secp256k1::new();
	let psbt = psbt.clone().finalize(&secp).map_err(|(_, e)| {
		log::error!("Error finalizing PSBT: {e:#?}");
		Error::PsbtFinalizeError
	})?;
	Ok(psbt)
}

pub fn extract_tx(psbt: &mut Psbt) -> Result<Transaction, Error> {
	let tx = {
		let finalized_psbt = finalize(psbt)?;
		finalized_psbt.extract_tx().map_err(Error::from)?
	};

	// Clear all the data fields as per the spec.
	{
		psbt.inputs[0].partial_sigs.clear();
		psbt.inputs[0].sighash_type = None;
		psbt.inputs[0].redeem_script = None;
		psbt.inputs[0].witness_script = None;
		psbt.inputs[0].bip32_derivation.clear();
	}

	Ok(tx)
}

#[cfg(feature = "std")]
pub fn get_tx_hex(tx: &Transaction) -> Result<String, Error> {
	use bitcoincore_rpc::RawTx;
	Ok(tx.raw_hex())
}

/// Broadcasts the transaction to a Bitcoin node. NOTE: You must return `true` from the
/// `on_status` callback to break the loop and return from this function.
///
/// # Arguments
/// * `url` - The URL of the Bitcoin node to broadcast the transaction to.
/// * `status_check_delay` - The delay between status checks for the transaction.
/// * `on_status` - A callback function that is called with the transaction status. If it returns
///   `true`, the function will return successfully.
#[cfg(feature = "std")]
pub async fn broadcast<F>(
	psbt: &mut Psbt,
	url: &str,
	status_check_delay: std::time::Duration,
	on_status: F,
) -> Result<(), Error>
where
	F: Fn(bitcoincore_rpc::json::GetRawTransactionResult) -> bool + Send + Sync + 'static,
{
	use bitcoincore_rpc::RpcApi;

	let tx = extract_tx(psbt)?;
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
	log::info!("Transaction broadcast with txid: {txid}");
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

/// No std friendly version of verifying a signature
pub fn verify_signature_raw(
	psbt: &Psbt,
	pubkey: CompressedBitcoinPubkey,
	signature_der_bytes: &BitcoinSignature,
) -> Result<bool, Error> {
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

	Ok(pubkey.verify_prehash(msg.as_ref(), &signature).is_ok())
}

pub fn sign(psbt: &mut Psbt, privkey: PrivateKey) -> Result<(Signature, PublicKey), Error> {
	let mut cache = SighashCache::new(&psbt.unsigned_tx);
	let mut signatures = vec![];
	let secp = Secp256k1::new();
	let pubkey = privkey.public_key(&secp);
	for i in 0..psbt.inputs.len() {
		let (msg, ecdsa_type) = psbt.sighash_ecdsa(i, &mut cache).map_err(Error::from)?;
		let sig = secp.sign_ecdsa(&msg, &privkey.inner);
		let signature = Signature { signature: sig, sighash_type: ecdsa_type };
		signatures.push((pubkey, signature));
	}
	let mut result = None;
	for (i, (pubkey, signature)) in signatures.into_iter().enumerate() {
		psbt.inputs[i].partial_sigs.insert(pubkey, signature);
		if i == 0 {
			result = Some((signature, pubkey));
		}
	}
	Ok(result.expect("At least one signature should be added"))
}

pub fn sign_derived(
	psbt: &mut Psbt,
	master_xpriv: Xpriv,
	hd_path: DerivationPath,
) -> Result<(Signature, PublicKey), Error> {
	let secp = Secp256k1::new();
	let child_xpriv = master_xpriv.derive_priv(&secp, &hd_path).map_err(Error::from)?;
	let master_xpub = Xpub::from_priv(&Secp256k1::new(), &master_xpriv);
	let mut signatures = vec![];

	let child_priv = child_xpriv.to_priv();
	let pubkey = child_priv.public_key(&secp);
	for i in 0..psbt.inputs.len() {
		psbt.inputs[i]
			.bip32_derivation
			.insert(pubkey.inner, (master_xpub.fingerprint(), hd_path.clone()));

		trace!("Signing with derived key: {pubkey}");

		let _ = psbt.sign(&master_xpriv, &secp).map_err(|(_, errs)| Error::from(errs))?;

		ensure!(!psbt.inputs[i].partial_sigs.is_empty(), Error::SignatureExpected);

		trace!("Signed {i}: {:?} sigs", psbt.inputs[i].partial_sigs.len());
		let Some((_, signature)) =
			psbt.inputs[i].partial_sigs.iter().find(|(k, _)| k.inner == pubkey.inner)
		else {
			return Err(Error::DerivedKeySignError);
		};
		signatures.push((*signature, pubkey));
	}

	Ok(signatures.remove(0))
}
