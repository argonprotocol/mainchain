#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
extern crate alloc;
extern crate core;

use alloc::{
	format,
	string::{String, ToString},
	vec::Vec,
};
use anyhow::Result;
use argon_bitcoin::{
	Amount, CosignReleaser, CosignScript, CosignScriptArgs, ReleaseStep, primitives::*, psbt_utils,
};
use argon_primitives::bitcoin::{
	BitcoinHeight, BitcoinNetwork, BitcoinScriptPubkey, H256Le, Satoshis,
};
use core::str::FromStr;
use wasm_bindgen::prelude::*;

fn create_cosign(
	vault_pubkey_hex: &str,
	vault_claim_pubkey_hex: &str,
	owner_pubkey_hex: &str,
	vault_claim_height: BitcoinHeight,
	open_claim_height: BitcoinHeight,
	created_at_height: BitcoinHeight,
	bitcoin_network: BitcoinNetwork,
) -> Result<CosignScript, String> {
	console_error_panic_hook::set_once();
	let cosign_script = CosignScript::new(
		CosignScriptArgs {
			vault_pubkey: from_hex::<[u8; 33]>(vault_pubkey_hex)?.into(),
			vault_claim_pubkey: from_hex::<[u8; 33]>(vault_claim_pubkey_hex)?.into(),
			owner_pubkey: from_hex::<[u8; 33]>(owner_pubkey_hex)?.into(),
			vault_claim_height,
			open_claim_height,
			created_at_height,
		},
		bitcoin_network.into(),
	)
	.map_err(|err| err.to_string())?;
	Ok(cosign_script)
}

#[wasm_bindgen(js_name = "createCosignPubkey")]
pub fn create_cosign_pubkey(
	vault_pubkey_hex: &str,
	vault_claim_pubkey_hex: &str,
	owner_pubkey_hex: &str,
	vault_claim_height: BitcoinHeight,
	open_claim_height: BitcoinHeight,
	created_at_height: BitcoinHeight,
	bitcoin_network: BitcoinNetwork,
) -> Result<String, String> {
	console_error_panic_hook::set_once();
	let cosign_script = create_cosign(
		vault_pubkey_hex,
		vault_claim_pubkey_hex,
		owner_pubkey_hex,
		vault_claim_height,
		open_claim_height,
		created_at_height,
		bitcoin_network,
	)?;
	Ok(format!("0x{}", cosign_script.get_script_pubkey().to_hex_string()))
}

#[wasm_bindgen(js_name = "calculateFee")]
#[allow(clippy::too_many_arguments)]
pub fn calculate_fee(
	vault_pubkey_hex: &str,
	vault_claim_pubkey_hex: &str,
	owner_pubkey_hex: &str,
	vault_claim_height: BitcoinHeight,
	open_claim_height: BitcoinHeight,
	created_at_height: BitcoinHeight,
	bitcoin_network: BitcoinNetwork,
	fee_rate_sats_per_vb: Satoshis,
	to_script_pubkey: &str,
) -> Result<u64, String> {
	console_error_panic_hook::set_once();
	let cosign_script = create_cosign(
		vault_pubkey_hex,
		vault_claim_pubkey_hex,
		owner_pubkey_hex,
		vault_claim_height,
		open_claim_height,
		created_at_height,
		bitcoin_network,
	)?;
	let to_scriptpub: BitcoinScriptPubkey = from_hex(to_script_pubkey)?;
	let fee_rate_sats_per_vb =
		FeeRate::from_sat_per_vb(fee_rate_sats_per_vb).ok_or("Invalid fee rate")?;
	Ok(cosign_script
		.calculate_fee(true, to_scriptpub.into(), fee_rate_sats_per_vb)
		.map_err(|err| err.to_string())?
		.to_sat())
}

#[wasm_bindgen(js_name = "signPsbtDerived")]
pub fn sign_psbt_derived(
	psbt_hex: &str,
	xpriv_b58: &str,
	xpriv_hd_path: &str,
	finalize: bool,
) -> Result<String, String> {
	let xpriv = Xpriv::from_str(xpriv_b58).map_err(|e| format!("Invalid xpriv: {e}"))?;
	let psbt_bytes: Vec<u8> = from_hex(psbt_hex).map_err(|e| format!("Invalid PSBT hex: {e}"))?;
	let mut psbt = Psbt::deserialize(&psbt_bytes).map_err(|e| format!("Invalid PSBT: {e}"))?;
	let hd_path =
		DerivationPath::from_str(xpriv_hd_path).map_err(|e| format!("Invalid HD path: {e}"))?;
	let _ = psbt_utils::sign_derived(&mut psbt, xpriv, hd_path)
		.map_err(|e| format!("Error signing PSBT: {e}"))?;
	if finalize {
		psbt = psbt_utils::finalize(&psbt).map_err(|e| format!("Error finalizing PSBT: {e}"))?;
	}
	let signed_psbt_hex = psbt.serialize_hex();
	Ok(format!("0x{signed_psbt_hex}"))
}

#[wasm_bindgen(js_name = "signPsbt")]
pub fn sign_psbt(
	psbt_hex: &str,
	bitcoin_network: BitcoinNetwork,
	private_key_hex: &str,
	finalize: bool,
) -> Result<String, String> {
	let private_key_bytes: Vec<u8> = from_hex(private_key_hex)?;
	let private_key = PrivateKey::from_slice(&private_key_bytes, bitcoin_network)
		.map_err(|e| format!("Invalid private key: {e}"))?;
	let psbt_bytes: Vec<u8> = from_hex(psbt_hex).map_err(|e| format!("Invalid PSBT hex: {e}"))?;

	let mut psbt = Psbt::deserialize(&psbt_bytes).map_err(|e| format!("Invalid PSBT: {e}"))?;
	psbt_utils::sign(&mut psbt, private_key).map_err(|e| format!("Error signing PSBT: {e}"))?;
	if finalize {
		psbt = psbt_utils::finalize(&psbt).map_err(|e| format!("Error finalizing PSBT: {e}"))?;
	}
	let signed_psbt_hex = psbt.serialize_hex();
	Ok(format!("0x{signed_psbt_hex}"))
}

#[wasm_bindgen(js_name = "getCosignPsbt")]
#[allow(clippy::too_many_arguments)]
pub fn get_cosigned_psbt(
	txid: &str,
	vout: u32,
	satoshis: Satoshis,
	vault_pubkey_hex: &str,
	vault_claim_pubkey_hex: &str,
	owner_pubkey_hex: &str,
	vault_claim_height: BitcoinHeight,
	open_claim_height: BitcoinHeight,
	created_at_height: BitcoinHeight,
	bitcoin_network: BitcoinNetwork,
	to_script_pubkey_hex: &str,
	bitcoin_network_fee: Satoshis,
) -> Result<String, String> {
	let txid: [u8; 32] = from_hex(txid)?;
	let pay_scriptpub: BitcoinScriptPubkey = from_hex(to_script_pubkey_hex)?;

	let cosign_script = create_cosign(
		vault_pubkey_hex,
		vault_claim_pubkey_hex,
		owner_pubkey_hex,
		vault_claim_height,
		open_claim_height,
		created_at_height,
		bitcoin_network,
	)?;
	let releaser = CosignReleaser::from_script(
		cosign_script,
		satoshis,
		H256Le(txid).into(),
		vout,
		ReleaseStep::VaultCosign, // this doesn't matter
		Amount::from_sat(bitcoin_network_fee),
		pay_scriptpub.into(),
	)
	.map_err(|err| err.to_string())?;
	Ok(format!("0x{}", releaser.psbt.serialize_hex()))
}

fn from_hex<T: TryFrom<Vec<u8>>>(text: impl AsRef<str>) -> Result<T, String> {
	let mut text = text.as_ref().trim();
	if text.starts_with("0x") {
		text = &text[2..];
	}
	let res: T = hex::decode(text)
		.map_err(|e| format!("Unable to decode the hex text {text} -> {e}"))?
		.try_into()
		.map_err(|_| "Unable to convert the text to a fixed length array".to_string())?;
	Ok(res)
}
