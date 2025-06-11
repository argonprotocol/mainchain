use anyhow::anyhow;
use argon_primitives::bitcoin::BitcoinNetwork;
use bip39::Mnemonic;
use bitcoin::{
	bip32::{DerivationPath, Xpriv, Xpub},
	key::Secp256k1,
	secp256k1::PublicKey,
};
use core::str::FromStr;

#[cfg(feature = "std")]
pub fn xpriv_from_mnemonic(mnemonic: &str, network: BitcoinNetwork) -> anyhow::Result<Xpriv> {
	let mnemonic = Mnemonic::from_str(mnemonic).map_err(|e| anyhow!(e))?;
	xpriv_from_seed(&mnemonic.to_seed(""), network)
}

pub fn xpriv_from_seed(seed: &[u8], network: BitcoinNetwork) -> anyhow::Result<Xpriv> {
	let xpriv = Xpriv::new_master(network, seed).map_err(|e| anyhow!(e))?;
	Ok(xpriv)
}

pub fn derive_xpub(xpriv: &Xpriv, hd_path: &str) -> anyhow::Result<Xpub> {
	let hd_path = DerivationPath::from_str(hd_path).map_err(|e| anyhow!(e))?;

	let child = xpriv.derive_priv(&Secp256k1::new(), &hd_path).map_err(|e| anyhow!(e))?;

	let child_xpub = Xpub::from_priv(&Secp256k1::new(), &child);
	Ok(child_xpub)
}

pub fn derive_pubkey(xpriv: &Xpriv, hd_path: &str) -> anyhow::Result<PublicKey> {
	let hd_path = DerivationPath::from_str(hd_path).map_err(|e| anyhow!(e))?;

	let child = xpriv.derive_priv(&Secp256k1::new(), &hd_path).map_err(|e| anyhow!(e))?;

	let pubkey = child.to_keypair(&Secp256k1::new()).public_key();

	Ok(pubkey)
}
