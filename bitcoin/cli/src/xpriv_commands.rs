use crate::xpriv_file::XprivFile;
use anyhow::bail;
use argon_bitcoin::{derive_pubkey, derive_xpub, xpriv_from_mnemonic, xpriv_from_seed};
use argon_primitives::{bitcoin::BitcoinNetwork, read_secret_text_input};
use clap::Subcommand;
use rand::Rng;
use std::path::PathBuf;

/// Create, secure, and manage your Bitcoin Master XPriv Key
#[derive(Subcommand, Debug)]
pub(crate) enum XPrivCommands {
	/// Create a Bitcoin Master XPriv Key in a Local File
	Master {
		#[clap(flatten)]
		xpriv_file: XprivFile,

		/// Read mnemonic from a file (preferred over passing mnemonic in argv).
		#[clap(long, conflicts_with_all = & ["mnemonic_interactive", "mnemonic"])]
		mnemonic_file: Option<PathBuf>,

		/// Enter mnemonic interactively with hidden terminal input.
		#[clap(long, conflicts_with_all = & ["mnemonic_file", "mnemonic"])]
		mnemonic_interactive: bool,

		/// Legacy and insecure: puts mnemonic in process arguments.
		#[clap(long, hide = true, conflicts_with_all = & ["mnemonic_file", "mnemonic_interactive"])]
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

impl XPrivCommands {
	pub async fn process(self) -> anyhow::Result<()> {
		match self {
			XPrivCommands::Master {
				xpriv_file,
				mnemonic_file,
				mnemonic_interactive,
				mnemonic,
				bitcoin_network,
			} => {
				let network = bitcoin_network.unwrap_or(BitcoinNetwork::Bitcoin);
				let mnemonic = read_mnemonic(
					mnemonic,
					mnemonic_file,
					mnemonic_interactive,
					xpriv_file.allow_insecure_cli_secrets,
				)?;

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
				println!("{child_xpub}");
			},
			XPrivCommands::DerivePubkey { xpriv_file, hd_path } => {
				let xpriv = xpriv_file.read()?;
				let pubkey = derive_pubkey(&xpriv, &hd_path)?;
				println!("{pubkey}");
			},
		}
		Ok(())
	}
}

fn read_mnemonic(
	mnemonic_from_argv: Option<String>,
	mnemonic_file: Option<PathBuf>,
	mnemonic_interactive: bool,
	allow_insecure_cli_secrets: bool,
) -> anyhow::Result<Option<String>> {
	if mnemonic_from_argv.is_some() && !allow_insecure_cli_secrets {
		bail!(
			"`--mnemonic` is disabled by default because command-line args leak via process lists and shell history. Use --mnemonic-file or --mnemonic-interactive. To bypass (unsafe), add --allow-insecure-cli-secrets."
		);
	}
	let mnemonic = read_secret_text_input(
		"Mnemonic: ",
		mnemonic_interactive,
		mnemonic_from_argv,
		mnemonic_file,
	)?
	.map(|m| m.trim().to_string());
	if matches!(mnemonic.as_deref(), Some("")) {
		bail!("Mnemonic cannot be empty");
	}
	Ok(mnemonic)
}

#[cfg(test)]
mod tests {
	use super::read_mnemonic;

	#[test]
	fn read_mnemonic_blocks_argv_secret_without_override() {
		let err = read_mnemonic(
			Some(
				"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
					.to_string(),
			),
			None,
			false,
			false,
		)
		.unwrap_err()
		.to_string();
		assert!(err.contains("`--mnemonic` is disabled by default"));
	}

	#[test]
	fn read_mnemonic_allows_argv_secret_with_override() {
		let mnemonic = read_mnemonic(
			Some(
				"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
					.to_string(),
			),
			None,
			false,
			true,
		)
		.unwrap()
		.unwrap();
		assert_eq!(
			mnemonic,
			"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
		);
	}
}
