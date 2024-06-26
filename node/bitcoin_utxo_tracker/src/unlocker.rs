use std::collections::BTreeMap;

use anyhow::{anyhow, bail, ensure};
use bitcoin::{
	absolute::LockTime,
	bip32::{KeySource, Xpriv},
	ecdsa::Signature,
	key::Secp256k1,
	psbt::Input,
	sighash::SighashCache,
	transaction::{predict_weight, InputWeightPrediction, Version},
	Address, Amount, EcdsaSighashType, FeeRate, Network, OutPoint, PrivateKey, Psbt, PubkeyHash,
	PublicKey, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use hwi::{types::HWIDevice, HWIClient};

use ulx_primitives::bitcoin::{
	create_timelock_multisig_script, BitcoinHeight, BitcoinPubkeyHash, Satoshis,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CosignScript {
	vault_pubkey_hash: BitcoinPubkeyHash,
	owner_pubkey_hash: BitcoinPubkeyHash,
	vault_claim_height: BitcoinHeight,
	open_claim_height: BitcoinHeight,
	created_at_height: BitcoinHeight,
	script: ScriptBuf,
}

impl CosignScript {
	pub fn new(
		vault_pubkey_hash: BitcoinPubkeyHash,
		owner_pubkey_hash: BitcoinPubkeyHash,
		vault_claim_height: BitcoinHeight,
		open_claim_height: BitcoinHeight,
		created_at_height: BitcoinHeight,
	) -> anyhow::Result<Self> {
		let script = create_timelock_multisig_script(
			vault_pubkey_hash.clone(),
			owner_pubkey_hash.clone(),
			vault_claim_height,
			open_claim_height,
		)
		.map_err(|_| anyhow!("Unable to create the timelock script"))?;
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
		let height = match unlock_step {
			UnlockStep::OwnerCosign | UnlockStep::VaultCosign => self.created_at_height,
			UnlockStep::VaultClaim => self.vault_claim_height,
			UnlockStep::OwnerClaim => self.open_claim_height,
		} as u32;
		height
	}
	pub fn calculate_fee(
		&self,
		is_cosign: bool,
		to_script_pubkey: ScriptBuf,
		fee_rate: FeeRate,
	) -> anyhow::Result<Amount> {
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
		let Some(fee) = fee_rate.fee_wu(weight) else { bail!("Fee rate too low") };
		Ok(fee)
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UtxoUnlocker {
	pub cosign_script: CosignScript,
	pub unlock_step: UnlockStep,
	pub psbt: Psbt,
}

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

impl UtxoUnlocker {
	pub fn from_script(
		cosign_script: CosignScript,
		utxo_satoshis: Satoshis,
		utxo_txid: bitcoin::Txid,
		utxo_vout: u32,
		unlock_step: UnlockStep,
		fee: Amount,
		to_script_pubkey: ScriptBuf,
	) -> anyhow::Result<Self> {
		let lock_time = cosign_script.unlock_height(unlock_step);
		let out_point = OutPoint { txid: utxo_txid, vout: utxo_vout };
		let out_amount = Amount::from_sat(utxo_satoshis);
		let unsigned_tx = Transaction {
			version: Version::TWO, // Post BIP-68.
			lock_time: LockTime::from_height(lock_time)
				.map_err(|_| anyhow!("Invalid lock time"))?,
			input: vec![TxIn {
				previous_output: out_point.clone(),
				sequence: Sequence::ENABLE_LOCKTIME_NO_RBF,
				..TxIn::default()
			}],
			output: vec![TxOut {
				value: out_amount
					.checked_sub(fee)
					.ok_or(anyhow!("Fee caused overflow of amount"))?,
				script_pubkey: to_script_pubkey,
			}],
		};

		let mut psbt = Psbt::from_unsigned_tx(unsigned_tx)?;

		psbt.inputs[0] = Input {
			witness_utxo: Some(TxOut {
				value: out_amount,
				script_pubkey: cosign_script.get_script_pubkey(),
			}),
			witness_script: Some(cosign_script.script.clone()),
			sighash_type: Some(EcdsaSighashType::All.into()),
			..Input::default()
		};

		Ok(Self { cosign_script, unlock_step, psbt })
	}
	pub fn new(
		vault_pubkey_hash: PubkeyHash,
		owner_pubkey_hash: PubkeyHash,
		created_at_height: BitcoinHeight,
		vault_claim_height: BitcoinHeight,
		open_claim_height: BitcoinHeight,
		utxo_satoshis: Satoshis,
		utxo_txid: bitcoin::Txid,
		utxo_vout: u32,
		unlock_step: UnlockStep,
		fee: Amount,
		pay_to_script_pubkey: ScriptBuf,
	) -> anyhow::Result<Self> {
		Self::from_script(
			CosignScript::new(
				vault_pubkey_hash.into(),
				owner_pubkey_hash.into(),
				vault_claim_height,
				open_claim_height,
				created_at_height,
			)?,
			utxo_satoshis,
			utxo_txid,
			utxo_vout,
			unlock_step,
			fee,
			pay_to_script_pubkey,
		)
	}

	pub fn add_signature(&mut self, pubkey: PublicKey, signature: Signature) {
		self.psbt.inputs[0].partial_sigs.insert(pubkey, signature);
	}

	pub fn sign(&mut self, privkey: PrivateKey) -> anyhow::Result<(Signature, PublicKey)> {
		let psbt = &mut self.psbt;
		let mut cache = SighashCache::new(&psbt.unsigned_tx);
		let (msg, ecdsa_type) = psbt.sighash_ecdsa(0, &mut cache)?;
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
		key_source: KeySource,
	) -> anyhow::Result<(Signature, PublicKey)> {
		let psbt = &mut self.psbt;
		let secp = Secp256k1::new();
		let child_xpriv = master_xpriv.derive_priv(&secp, &key_source.1)?;
		let child_priv = child_xpriv.to_priv();
		let pubkey = child_priv.public_key(&secp);

		psbt.inputs[0].bip32_derivation.insert(pubkey.inner, key_source);

		match psbt.sign(&master_xpriv, &secp) {
			Ok(_) => ensure!(psbt.inputs[0].partial_sigs.len() >= 1, "Expected 1 signature"),
			Err((_, errs)) => bail!("Error signing Partially Signed Bitcoin script {:?}", errs),
		};

		let Some((_, signature)) =
			psbt.inputs[0].partial_sigs.iter().find(|(k, _)| k.inner == pubkey.inner)
		else {
			bail!("Could not sign with derived key");
		};

		Ok((signature.clone(), pubkey))
	}

	pub fn sign_hwi(
		&mut self,
		key_source: KeySource,
		device: Option<HWIDevice>,
		network: Network,
	) -> anyhow::Result<(Signature, PublicKey)> {
		let psbt = &mut self.psbt;
		let mut device = device;
		if device.is_none() {
			let devices = HWIClient::enumerate()
				.map_err(|e| anyhow!("Error enumerating devices: {:?}", e))?;
			for d in devices {
				match d {
					Ok(x) => {
						device = Some(x);
						break;
					},
					Err(_) => {},
				}
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

		Ok((signature.clone(), pubkey.into()))
	}

	pub fn extract_tx(&mut self) -> anyhow::Result<Transaction> {
		let mut witness = Witness::new();
		let psbt = &mut self.psbt;
		let mut sigs: Vec<Vec<u8>> = vec![];
		for (pubkey, sig) in psbt.inputs[0].partial_sigs.iter() {
			let pubkey_hash: BitcoinPubkeyHash = pubkey.pubkey_hash().into();
			// vault is verified on stack first
			if pubkey_hash == self.cosign_script.vault_pubkey_hash {
				sigs.push(sig.to_vec());
				sigs.push(pubkey.to_bytes());
			} else if pubkey_hash == self.cosign_script.owner_pubkey_hash {
				sigs.insert(0, sig.to_vec());
				sigs.insert(1, pubkey.to_bytes());
			} else {
				bail!("Unknown pubkey hash in partial sigs");
			}
		}
		for sig in sigs {
			witness.push(sig);
		}
		match self.unlock_step {
			UnlockStep::VaultCosign | UnlockStep::OwnerCosign => witness.push([COSIGN_CODE]),
			UnlockStep::VaultClaim => witness.push([VAULT_CLAIM_CODE]),
			UnlockStep::OwnerClaim => witness.push([OPEN_CLAIM_CODE]),
		}
		witness.push(self.cosign_script.script.clone());

		psbt.inputs[0].final_script_witness = Some(witness);

		// Clear all the data fields as per the spec.
		psbt.inputs[0].partial_sigs = BTreeMap::new();
		psbt.inputs[0].sighash_type = None;
		psbt.inputs[0].redeem_script = None;
		psbt.inputs[0].witness_script = None;
		psbt.inputs[0].bip32_derivation = BTreeMap::new();

		let tx = psbt.clone().extract_tx()?;
		Ok(tx)
	}
}
