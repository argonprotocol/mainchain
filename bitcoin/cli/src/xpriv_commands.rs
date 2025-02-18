use crate::{helpers::get_bitcoin_network, xpriv_file::XprivFile};
use anyhow::anyhow;
use argon_client::MainchainClient;
use bip39::Mnemonic;
use bitcoin::{
	bip32,
	bip32::{Xpriv, Xpub},
	key::Secp256k1,
};
use clap::{Parser, Subcommand};
use rand::Rng;
use std::str::FromStr;

/// Create, secure, and manage your Bitcoin Master XPriv Key
#[derive(Subcommand, Debug)]
pub(crate) enum XPrivCommands {
	/// Create a Bitcoin Master XPriv Key in a Local File
	Master {
		#[clap(flatten)]
		xpriv_file: XprivFile,

		/// Optionally import your own mnemonic to generate the Master XPriv
		#[clap(long)]
		mnemonic: Option<String>,
	},
	#[clap(name = "derive-xpub")]
	/// Derive an XPub from your Master XPriv
	DeriveXPub {
		#[clap(flatten)]
		xpriv_file: XprivFile,

		/// HD Path to derive the XPub from (NOTE: must be hardened to submit to Argon)
		#[clap(long)]
		hd_path: String,
	},
}

#[derive(Parser, Debug)]
struct OneArg {
	arg: String,
}
impl XPrivCommands {
	pub async fn process(self, rpc_url: String) -> anyhow::Result<()> {
		match self {
			XPrivCommands::Master { xpriv_file, mnemonic } => {
				let client = MainchainClient::from_url(&rpc_url).await?;
				let network = get_bitcoin_network(&client, None).await?;
				let mnemonic = if let Some(x) = mnemonic {
					Mnemonic::from_str(&x).map_err(|e| anyhow!(e))?
				} else {
					let mut rng = rand::thread_rng();
					let mut bytes = [0u8; 32];
					rng.fill(&mut bytes);
					Mnemonic::from_entropy(&bytes[..]).map_err(|e| anyhow!(e))?
				};
				let seed = mnemonic.to_seed("");

				let xpriv = Xpriv::new_master(network, &seed).map_err(|e| anyhow!(e))?;

				let path = xpriv_file.write(&xpriv)?;

				println!("Your Master XPriv has been saved to {}", path.display());
			},
			XPrivCommands::DeriveXPub { xpriv_file, hd_path } => {
				let xpriv = xpriv_file.read()?;
				let hd_path = bip32::DerivationPath::from_str(&hd_path).map_err(|e| anyhow!(e))?;

				let child = xpriv.derive_priv(&Secp256k1::new(), &hd_path)?;

				let child_xpub = Xpub::from_priv(&Secp256k1::new(), &child);
				println!("{}", child_xpub);
			},
		}
		Ok(())
	}
}
