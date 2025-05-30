use crate::xpriv_file::XprivFile;
use argon_bitcoin::{derive_pubkey, derive_xpub, xpriv_from_mnemonic, xpriv_from_seed};
use argon_primitives::bitcoin::BitcoinNetwork;
use clap::{Parser, Subcommand};
use rand::Rng;

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

		/// If running tests, configure the bitcoin network to use
		#[clap(long)]
		bitcoin_network: Option<BitcoinNetwork>,
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

	/// Derive an Bitcoin Pubkey from your Master XPriv
	DerivePubkey {
		#[clap(flatten)]
		xpriv_file: XprivFile,

		/// HD Path to derive the Pubkey from
		#[clap(long)]
		hd_path: String,
	},
}

#[derive(Parser, Debug)]
struct OneArg {
	arg: String,
}
impl XPrivCommands {
	pub async fn process(self) -> anyhow::Result<()> {
		match self {
			XPrivCommands::Master { xpriv_file, mnemonic, bitcoin_network } => {
				let network = bitcoin_network.unwrap_or(BitcoinNetwork::Bitcoin);

				let xpriv = if let Some(x) = mnemonic {
					xpriv_from_mnemonic(&x, network)?
				} else {
					let mut rng = rand::rng();
					let mut bytes = [0u8; 32];
					rng.fill(&mut bytes);
					xpriv_from_seed(&bytes, network)?
				};

				let path = xpriv_file.write(&xpriv)?;

				println!("Your Master XPriv has been saved to {}", path.display());
			},
			XPrivCommands::DeriveXPub { xpriv_file, hd_path } => {
				let xpriv = xpriv_file.read()?;

				let child_xpub = derive_xpub(&xpriv, &hd_path)?;
				println!("{}", child_xpub);
			},
			XPrivCommands::DerivePubkey { xpriv_file, hd_path } => {
				let xpriv = xpriv_file.read()?;
				let pubkey = derive_pubkey(&xpriv, &hd_path)?;
				println!("{}", pubkey);
			},
		}
		Ok(())
	}
}
