use age::{Decryptor, Encryptor, secrecy::SecretString};
use anyhow::{anyhow, bail};
use argon_primitives::KeystoreParams;
use bitcoin::bip32::Xpriv;
use clap::Args;
use directories::BaseDirs;
use std::{
	fs,
	fs::File,
	io::{BufReader, BufWriter, Read, Write},
	path::{Path, PathBuf},
};

/// A local file that can store an encrypted (or plaintext) xpriv.
#[derive(Debug, Clone, Args)]
pub struct XprivFile {
	/// Specify the path of the master xpriv
	#[arg(global = true, long, value_name = "PATH")]
	pub xpriv_path: Option<PathBuf>,

	/// Allow insecure secret argv flags such as `--xpriv-password`.
	#[arg(global = true, long, hide = true, default_value_t = false)]
	pub allow_insecure_cli_secrets: bool,

	/// Use interactive shell for entering the encryption password used by the xpriv.
	#[arg(global = true, long, conflicts_with_all = & ["xpriv_password", "xpriv_password_filename", "xpriv_password_ledger", "xpriv_password_yubikey"])]
	pub xpriv_password_interactive: bool,

	/// Encryption password used by the xpriv file.
	///
	/// This allows appending an extra user-defined secret to the seed.
	#[arg(
		global = true,
		long,
        value_parser = secret_string_from_str,
        conflicts_with_all = & ["xpriv_password_interactive", "xpriv_password_filename", "xpriv_password_ledger", "xpriv_password_yubikey"]
    )]
	pub xpriv_password: Option<SecretString>,

	/// File that contains the encryption password used by the xpriv file.
	#[arg(
			global = true,
			long = "xpriv-password-file",
			alias = "xpriv-password-filename",
	        value_name = "PATH",
	        conflicts_with_all = & ["xpriv_password_interactive", "xpriv_password", "xpriv_password_ledger", "xpriv_password_yubikey"]
	    )]
	pub xpriv_password_filename: Option<PathBuf>,

	/// Use a ledger to protect your XPriv password (UNDER DEVELOPMENT)
	#[arg(global = true, long, conflicts_with_all = & ["xpriv_password", "xpriv_password_filename", "xpriv_password_interactive", "xpriv_password_yubikey"])]
	pub xpriv_password_ledger: Option<bool>,

	/// Use a yubikey to protect your XPriv password (UNDER DEVELOPMENT)
	#[arg(global = true, long, conflicts_with_all = & ["xpriv_password", "xpriv_password_filename", "xpriv_password_interactive", "xpriv_password_ledger"])]
	pub xpriv_password_yubikey: Option<bool>,

	/// Allow writing an unencrypted xpriv file (unsafe).
	#[arg(global = true, long, default_value_t = false)]
	pub unsafe_plaintext_xpriv: bool,
}

impl XprivFile {
	fn read_password(&self) -> anyhow::Result<Option<SecretString>> {
		if self.xpriv_password.is_some() && !self.allow_insecure_cli_secrets {
			bail!(
				"`--xpriv-password` is disabled by default because command-line args leak via process lists and shell history. Use --xpriv-password-file or --xpriv-password-interactive. To bypass (unsafe), add --allow-insecure-cli-secrets."
			);
		}

		let xpriv_params = KeystoreParams {
			keystore_path: self.xpriv_path.clone(),
			password_interactive: self.xpriv_password_interactive,
			password: self.xpriv_password.clone(),
			password_filename: self.xpriv_password_filename.clone(),
		};

		if self.xpriv_password_ledger.unwrap_or(false) {
			bail!(
				"Ledger password protection is not yet implemented. Use --xpriv-password-file or --xpriv-password-interactive."
			);
		}
		if self.xpriv_password_yubikey.unwrap_or(false) {
			bail!(
				"Yubikey password protection is not yet implemented. Use --xpriv-password-file or --xpriv-password-interactive."
			);
		}
		xpriv_params.read_password()
	}

	pub fn write(&self, xpriv: &Xpriv) -> anyhow::Result<PathBuf> {
		let password = self.read_password()?;
		if password.is_none() && !self.unsafe_plaintext_xpriv {
			bail!(
				"Refusing to write plaintext xpriv. Provide an encryption password (--xpriv-password-file or --xpriv-password-interactive). To bypass (unsafe), pass --unsafe-plaintext-xpriv."
			);
		}

		let path = self.xpriv_path.clone().ok_or(anyhow!("No key path provided"))?;
		let path = expand_path(&path);
		if path.is_file() {
			bail!("File already exists");
		}

		if let Some(parent) = path.parent() {
			if !parent.exists() {
				fs::create_dir_all(parent)?;
			}
		}

		let mut output_file = File::create(path.clone())?;

		#[cfg(unix)]
		{
			use std::{fs::Permissions, os::unix::fs::PermissionsExt};
			output_file
				.set_permissions(Permissions::from_mode(0o600))
				.map_err(|e| anyhow!("Failed to set permissions: {e}"))?;
		}

		if let Some(password) = password {
			let encryptor = Encryptor::with_user_passphrase(password);

			let mut writer = encryptor.wrap_output(BufWriter::new(output_file))?;

			writer.write_all(xpriv.encode().as_ref())?;
			writer.finish()?;
		} else {
			eprintln!("WARNING: writing unencrypted xpriv due to --unsafe-plaintext-xpriv");
			output_file.write_all(xpriv.encode().as_ref())?;
		}

		Ok(path)
	}

	pub fn read(&self) -> anyhow::Result<Xpriv> {
		let password = self.read_password()?;
		let input_path = self.xpriv_path.clone().ok_or(anyhow!("No key path provided"))?;
		let input_path = expand_path(&input_path);
		let mut input_file = File::open(input_path)?;

		let mut bytes = Vec::new();
		if let Some(password) = password {
			let decryptor = Decryptor::new(BufReader::new(input_file))?;
			if let Decryptor::Passphrase(decryptor) = decryptor {
				decryptor.decrypt(&password, None)?.read_to_end(&mut bytes)?;
			} else {
				bail!("Decryption failed");
			}
		} else {
			input_file.read_to_end(&mut bytes)?;
		}

		Xpriv::decode(bytes.as_ref()).map_err(|_| anyhow!("Invalid Xpriv"))
	}
}

pub fn secret_string_from_str(s: &str) -> Result<SecretString, String> {
	std::str::FromStr::from_str(s).map_err(|_| "Could not get SecretString".to_string())
}

pub fn expand_path(path: &Path) -> PathBuf {
	if let Some(str_path) = path.to_str() {
		if str_path.starts_with('~') {
			if let Some(dirs) = BaseDirs::new() {
				return PathBuf::from(str_path.replacen('~', dirs.home_dir().to_str().unwrap(), 1));
			}
		}
	}
	path.to_path_buf()
}

#[cfg(test)]
mod tests {
	use super::XprivFile;
	use age::secrecy::SecretString;
	use bitcoin::{Network, bip32::Xpriv};
	use std::{path::PathBuf, time::SystemTime};

	fn temp_file(name: &str) -> PathBuf {
		let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
		std::env::temp_dir().join(format!("argon-bitcoin-cli-{name}-{nanos}.key"))
	}

	fn sample_xpriv() -> Xpriv {
		Xpriv::new_master(Network::Regtest, &[7u8; 32]).unwrap()
	}

	#[test]
	fn write_requires_password_by_default() {
		let path = temp_file("unencrypted-default-rejected");
		let file = XprivFile {
			xpriv_path: Some(path.clone()),
			allow_insecure_cli_secrets: false,
			xpriv_password_interactive: false,
			xpriv_password: None,
			xpriv_password_filename: None,
			xpriv_password_ledger: None,
			xpriv_password_yubikey: None,
			unsafe_plaintext_xpriv: false,
		};
		let err = file.write(&sample_xpriv()).unwrap_err().to_string();
		assert!(err.contains("Refusing to write plaintext xpriv"));
	}

	#[test]
	fn write_allows_unsafe_plaintext_when_explicitly_enabled() {
		let path = temp_file("unsafe-plaintext");
		let file = XprivFile {
			xpriv_path: Some(path.clone()),
			allow_insecure_cli_secrets: false,
			xpriv_password_interactive: false,
			xpriv_password: None,
			xpriv_password_filename: None,
			xpriv_password_ledger: None,
			xpriv_password_yubikey: None,
			unsafe_plaintext_xpriv: true,
		};
		file.write(&sample_xpriv()).unwrap();
		assert!(path.exists());
		let _ = std::fs::remove_file(path);
	}

	#[test]
	fn write_encrypts_when_password_is_provided() {
		let path = temp_file("encrypted");
		let xpriv = sample_xpriv();
		let file = XprivFile {
			xpriv_path: Some(path.clone()),
			allow_insecure_cli_secrets: true,
			xpriv_password_interactive: false,
			xpriv_password: Some(SecretString::new("test-password".to_string())),
			xpriv_password_filename: None,
			xpriv_password_ledger: None,
			xpriv_password_yubikey: None,
			unsafe_plaintext_xpriv: false,
		};
		file.write(&xpriv).unwrap();
		let bytes = std::fs::read(&path).unwrap();
		assert_ne!(bytes, xpriv.encode().to_vec());
		let _ = std::fs::remove_file(path);
	}

	#[test]
	fn read_password_blocks_argv_secret_without_override() {
		let file = XprivFile {
			xpriv_path: None,
			allow_insecure_cli_secrets: false,
			xpriv_password_interactive: false,
			xpriv_password: Some(SecretString::new("test-password".to_string())),
			xpriv_password_filename: None,
			xpriv_password_ledger: None,
			xpriv_password_yubikey: None,
			unsafe_plaintext_xpriv: false,
		};
		let err = file.read_password().unwrap_err().to_string();
		assert!(err.contains("`--xpriv-password` is disabled by default"));
	}

	#[test]
	fn read_password_allows_argv_secret_with_override() {
		let file = XprivFile {
			xpriv_path: None,
			allow_insecure_cli_secrets: true,
			xpriv_password_interactive: false,
			xpriv_password: Some(SecretString::new("test-password".to_string())),
			xpriv_password_filename: None,
			xpriv_password_ledger: None,
			xpriv_password_yubikey: None,
			unsafe_plaintext_xpriv: false,
		};
		assert!(file.read_password().unwrap().is_some());
	}
}
