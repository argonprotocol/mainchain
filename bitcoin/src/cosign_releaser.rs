use crate::{
	cosign_script::{CosignScript, CosignScriptArgs, ReleaseStep},
	errors::Error,
	psbt_utils::*,
};
use alloc::{vec, vec::Vec};
use argon_primitives::bitcoin::{
	BitcoinError, BitcoinSignature, CompressedBitcoinPubkey, Satoshis, UtxoRef,
};
use bitcoin::{
	absolute::LockTime,
	bip32::{DerivationPath, Xpriv},
	ecdsa::Signature,
	psbt::Input,
	transaction::Version,
	Amount, EcdsaSighashType, Network, OutPoint, PrivateKey, Psbt, PublicKey, ScriptBuf, Sequence,
	Transaction, TxIn, TxOut, Witness,
};
use miniscript::psbt::PsbtExt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CosignReleaser {
	pub cosign_script: CosignScript,
	pub release_step: ReleaseStep,
	pub psbt: Psbt,
}

impl CosignReleaser {
	/// Builds a release with inputs ordered by `UtxoRef` (`txid`, then `output_index`).
	pub fn from_script(
		cosign_script: CosignScript,
		utxos: Vec<(UtxoRef, Satoshis)>,
		release_step: ReleaseStep,
		fee: Amount,
		to_script_pubkey: ScriptBuf,
	) -> Result<Self, Error> {
		let total_satoshis = utxos.iter().try_fold(0u64, |total, (_, satoshis)| {
			total.checked_add(*satoshis).ok_or(Error::FeeOverflow)
		})?;
		let outputs = vec![TxOut {
			value: Amount::from_sat(total_satoshis).checked_sub(fee).ok_or(Error::FeeOverflow)?,
			script_pubkey: to_script_pubkey,
		}];
		Self::from_script_outputs(cosign_script, utxos, release_step, outputs)
	}

	/// Builds a release with inputs ordered by `UtxoRef` and exact caller-specified outputs.
	pub fn from_script_outputs(
		cosign_script: CosignScript,
		mut utxos: Vec<(UtxoRef, Satoshis)>,
		release_step: ReleaseStep,
		outputs: Vec<TxOut>,
	) -> Result<Self, Error> {
		if utxos.is_empty() {
			return Err(Error::NoUtxos)
		}
		utxos.sort_by(|(left, _), (right, _)| left.cmp(right));
		let lock_time = cosign_script.unlock_height(release_step);
		let input_satoshis = utxos.iter().try_fold(0u64, |total, (_, satoshis)| {
			total.checked_add(*satoshis).ok_or(Error::FeeOverflow)
		})?;
		let output_satoshis = outputs.iter().try_fold(0u64, |total, output| {
			total.checked_add(output.value.to_sat()).ok_or(Error::FeeOverflow)
		})?;
		if output_satoshis > input_satoshis {
			return Err(Error::FeeOverflow)
		}
		let unsigned_tx = Transaction {
			version: Version::TWO, // Post BIP-68.
			lock_time: LockTime::from_height(lock_time)
				.map_err(|_| BitcoinError::InvalidLockTime)?,
			input: utxos
				.iter()
				.map(|(utxo_ref, _)| TxIn {
					previous_output: OutPoint {
						txid: utxo_ref.txid.clone().into(),
						vout: utxo_ref.output_index,
					},
					sequence: Sequence::ENABLE_LOCKTIME_NO_RBF,
					..TxIn::default()
				})
				.collect(),
			output: outputs,
		};

		let mut psbt = Psbt::from_unsigned_tx(unsigned_tx).map_err(Error::from)?;
		let descriptor = cosign_script.create_descriptor()?;
		for (index, (_, satoshis)) in utxos.iter().enumerate() {
			psbt.inputs[index] = Input {
				witness_utxo: Some(TxOut {
					value: Amount::from_sat(*satoshis),
					script_pubkey: cosign_script.get_script_pubkey(),
				}),
				witness_script: Some(cosign_script.script.clone()),
				sighash_type: Some(EcdsaSighashType::All.into()),
				..Input::default()
			};
			psbt.update_input_with_descriptor(index, &descriptor).map_err(|_| {
				log::error!("Error updating PSBT input {index} with descriptor: {descriptor:#?}");
				Error::PsbtFinalizeError
			})?;
		}

		Ok(Self { cosign_script, release_step, psbt })
	}

	#[allow(clippy::too_many_arguments)]
	pub fn new(
		cosign_script_args: CosignScriptArgs,
		utxos: Vec<(UtxoRef, Satoshis)>,
		release_step: ReleaseStep,
		fee: Amount,
		pay_to_script_pubkey: ScriptBuf,
		network: Network,
	) -> Result<Self, Error> {
		Self::from_script(
			CosignScript::new(cosign_script_args, network)?,
			utxos,
			release_step,
			fee,
			pay_to_script_pubkey,
		)
	}

	pub fn add_signature(
		&mut self,
		input_index: usize,
		pubkey: PublicKey,
		signature: Signature,
	) -> Result<(), Error> {
		let input = self.psbt.inputs.get_mut(input_index).ok_or(Error::SignatureCountMismatch)?;
		input.partial_sigs.insert(pubkey, signature);
		Ok(())
	}

	/// No std friendly version of verifying a signature
	pub fn verify_signatures_raw(
		&self,
		pubkey: CompressedBitcoinPubkey,
		signatures: &[BitcoinSignature],
	) -> Result<bool, Error> {
		if signatures.len() != self.psbt.inputs.len() {
			return Ok(false)
		}
		for (input_index, signature) in signatures.iter().enumerate() {
			if !verify_signature_raw(&self.psbt, input_index, pubkey, signature)? {
				return Ok(false)
			}
		}
		Ok(true)
	}

	pub fn sign(&mut self, privkey: PrivateKey) -> Result<Vec<(Signature, PublicKey)>, Error> {
		sign(&mut self.psbt, privkey)
	}

	pub fn sign_derived(
		&mut self,
		master_xpriv: Xpriv,
		hd_path: DerivationPath,
	) -> Result<Vec<(Signature, PublicKey)>, Error> {
		sign_derived(&mut self.psbt, master_xpriv, hd_path)
	}

	pub fn create_witness(&mut self) -> Result<(), Error> {
		let owner_pubkey = self.cosign_script.script_args.bitcoin_owner_pubkey()?;
		let vault_pubkey = self.cosign_script.script_args.bitcoin_vault_pubkey()?;
		let vault_claim_pubkey = self.cosign_script.script_args.bitcoin_vault_claim_pubkey()?;
		for input in &mut self.psbt.inputs {
			let mut witness = Witness::new();
			if let Some(sig) = input.partial_sigs.get(&vault_pubkey) {
				witness.push(sig.to_vec());
			}
			if let Some(sig) = input.partial_sigs.get(&vault_claim_pubkey) {
				witness.push(sig.to_vec());
			}
			if let Some(sig) = input.partial_sigs.get(&owner_pubkey) {
				witness.push(sig.to_vec());
			}
			witness.push(self.cosign_script.script.clone());
			input.final_script_witness = Some(witness);
		}
		Ok(())
	}

	pub fn extract_tx(&mut self) -> Result<Transaction, Error> {
		extract_tx(&mut self.psbt)
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
		broadcast(&mut self.psbt, url, status_check_delay, on_status).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use argon_primitives::bitcoin::H256Le;
	use bitcoin::{secp256k1::Secp256k1, Network};

	#[test]
	fn builds_exact_outputs_while_sorting_inputs() {
		let network = Network::Regtest;
		let secp = Secp256k1::new();
		let script = CosignScript::new(
			CosignScriptArgs {
				vault_pubkey: PrivateKey::generate(network).public_key(&secp).into(),
				owner_pubkey: PrivateKey::generate(network).public_key(&secp).into(),
				vault_claim_pubkey: PrivateKey::generate(network).public_key(&secp).into(),
				created_at_height: 100,
				vault_claim_height: 200,
				open_claim_height: 300,
			},
			network,
		)
		.expect("script");
		let first = UtxoRef { txid: H256Le([1; 32]), output_index: 0 };
		let second = UtxoRef { txid: H256Le([2; 32]), output_index: 1 };
		let outputs = vec![
			TxOut { value: Amount::from_sat(400), script_pubkey: ScriptBuf::from_bytes(vec![1]) },
			TxOut { value: Amount::from_sat(590), script_pubkey: script.get_script_pubkey() },
		];

		let releaser = CosignReleaser::from_script_outputs(
			script,
			vec![(second.clone(), 500), (first.clone(), 500)],
			ReleaseStep::VaultCosign,
			outputs.clone(),
		)
		.expect("release");

		assert_eq!(releaser.psbt.unsigned_tx.output, outputs);
		assert!(releaser
			.psbt
			.inputs
			.iter()
			.all(|input| input.sighash_type == Some(EcdsaSighashType::All.into())));
		assert_eq!(
			releaser.psbt.unsigned_tx.input[0].previous_output,
			OutPoint { txid: first.txid.into(), vout: first.output_index }
		);
		assert_eq!(
			releaser.psbt.unsigned_tx.input[1].previous_output,
			OutPoint { txid: second.txid.into(), vout: second.output_index }
		);
	}
}
