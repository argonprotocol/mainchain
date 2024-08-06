use std::{fs, path::PathBuf};

use anyhow::{anyhow, bail};
use clap::{Args, ValueEnum};
use sc_keystore::LocalKeystore;
use sp_core::{
	crypto::{ExposeSecret, KeyTypeId, SecretString},
	ed25519, sr25519, Pair,
};
use sp_keystore::{testing::MemoryKeystore, KeystorePtr};

/// Parameters of the keystore
#[derive(Debug, Clone, Args)]
pub struct KeystoreParams {
	/// Specify custom keystore path.
	#[arg(global = true, long, value_name = "PATH")]
	pub keystore_path: Option<PathBuf>,

	/// Use interactive shell for entering the password used by the keystore.
	#[arg(global = true,long, conflicts_with_all = & ["password", "password_filename"])]
	pub password_interactive: bool,

	/// Password used by the keystore.
	///
	/// This allows appending an extra user-defined secret to the seed.
	#[arg(
		global = true,
		long,
        value_parser = secret_string_from_str,
        conflicts_with_all = & ["password_interactive", "password_filename"]
    )]
	pub password: Option<SecretString>,

	/// File that contains the password used by the keystore.
	#[arg(
		global = true,
		long,
        value_name = "PATH",
        conflicts_with_all = & ["password_interactive", "password"]
    )]
	pub password_filename: Option<PathBuf>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CryptoType {
	Sr25519,
	Ed25519,
}

impl KeystoreParams {
	pub fn read_password(&self) -> anyhow::Result<Option<SecretString>> {
		let (password_interactive, password) = (self.password_interactive, self.password.clone());

		let pass = if password_interactive {
			Some(SecretString::new(rpassword::prompt_password("Keystore Password: ")?))
		} else if let Some(ref file) = self.password_filename {
			let password = fs::read_to_string(file)
				.map_err(|e| anyhow!("Unable to read password file: {}", e))?;
			Some(SecretString::new(password))
		} else {
			password
		};

		Ok(pass)
	}

	pub fn open(&self) -> anyhow::Result<KeystorePtr> {
		let password = self.read_password()?;
		let Some(path) = &self.keystore_path else {
			bail!("No keystore path provided");
		};
		let keystore: KeystorePtr = LocalKeystore::open(path, password)?.into();
		Ok(keystore)
	}

	pub fn open_with_account(
		&self,
		suri_or_prompt: Option<&String>,
		crypto_type: CryptoType,
		key_type_id: KeyTypeId,
	) -> anyhow::Result<KeystorePtr> {
		let suri = Self::read_uri(suri_or_prompt)?;
		let password = self.read_password()?;
		let keystore: KeystorePtr = match &self.keystore_path {
			Some(r) => LocalKeystore::open(r, password.clone())?.into(),
			None => MemoryKeystore::new().into(),
		};
		let public_bytes = match crypto_type {
			CryptoType::Sr25519 => {
				let pair = match password {
					Some(pass) => sr25519::Pair::from_string(&suri, Some(&pass.expose_secret()))?,
					None => sr25519::Pair::from_string(&suri, None)?,
				};
				pair.public().0
			},
			CryptoType::Ed25519 => {
				let pair = match password {
					Some(ref pass) =>
						ed25519::Pair::from_string(&suri, Some(&pass.expose_secret()))?,
					None => ed25519::Pair::from_string(&suri, None)?,
				};
				pair.public().0
			},
		};
		keystore
			.insert(key_type_id, &suri, &public_bytes)
			.map_err(|_| anyhow!("Unable to insert dev keypair for alice"))?;
		Ok(keystore)
	}

	pub fn open_dev(
		&self,
		suri: &str,
		crypto_type: CryptoType,
		key_type_id: KeyTypeId,
	) -> anyhow::Result<KeystorePtr> {
		self.open_with_account(Some(&suri.to_string()), crypto_type, key_type_id)
	}

	pub fn read_uri(uri: Option<&String>) -> anyhow::Result<String> {
		let uri = if let Some(uri) = uri {
			let file = PathBuf::from(&uri);
			if file.is_file() {
				fs::read_to_string(uri)?.trim_end().to_owned()
			} else {
				uri.into()
			}
		} else {
			rpassword::prompt_password("URI: ")?
		};

		Ok(uri)
	}
}

/// Parse a secret string, returning a displayable error.
pub fn secret_string_from_str(s: &str) -> Result<SecretString, String> {
	std::str::FromStr::from_str(s).map_err(|_| "Could not get SecretString".to_string())
}
