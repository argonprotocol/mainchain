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
		long,
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
}

impl XprivFile {
	fn read_password(&self) -> anyhow::Result<Option<SecretString>> {
		let xpriv_params = KeystoreParams {
			keystore_path: self.xpriv_path.clone(),
			password_interactive: self.xpriv_password_interactive,
			password: self.xpriv_password.clone(),
			password_filename: self.xpriv_password_filename.clone(),
		};

		if self.xpriv_password_ledger.is_some() {
			unimplemented!("Ledger password protection is not yet implemented");
		}
		if self.xpriv_password_yubikey.is_some() {
			unimplemented!("Yubikey password protection is not yet implemented");
		}
		xpriv_params.read_password()
	}

	pub fn write(&self, xpriv: &Xpriv) -> anyhow::Result<PathBuf> {
		let password = self.read_password()?;

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
				.map_err(|e| anyhow!("Failed to set permissions: {}", e))?;
		}

		if let Some(password) = password {
			let encryptor = Encryptor::with_user_passphrase(password);

			let mut writer = encryptor.wrap_output(BufWriter::new(output_file))?;

			writer.write_all(xpriv.encode().as_ref())?;
			writer.finish()?;
		} else {
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
