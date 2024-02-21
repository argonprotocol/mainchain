// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use clap::ValueEnum;
use std::any::TypeId;
use std::hash::Hash;
use std::io;
use std::{
  collections::HashMap,
  fs::{self, File},
  io::Write,
  path::PathBuf,
};

use parking_lot::RwLock;
use sp_core::crypto::{ByteArray, ExposeSecret, Pair as CorePair, SecretString, Zeroize};
use sp_core::ecdsa::CRYPTO_ID as ECDSA_CRYPTO_ID;
use sp_core::ed25519::CRYPTO_ID as ED_CRYPTO_ID;
use sp_core::sr25519::CRYPTO_ID as SR_CRYPTO_ID;
use sp_core::{ecdsa, ed25519, sr25519, Pair};
use sp_runtime::{MultiSignature, MultiSigner};

type Result<T> = std::result::Result<T, Error>;

/// A local based keystore that is either memory-based or filesystem-based.
pub struct LocalKeystore(RwLock<KeystoreInner>);

impl LocalKeystore {
  /// Create a local keystore from filesystem.
  pub fn open<T: Into<PathBuf>>(path: T, password: Option<SecretString>) -> Result<Self> {
    let inner = KeystoreInner::open(path, password)?;
    Ok(Self(RwLock::new(inner)))
  }

  /// Create a local keystore in memory.
  pub fn in_memory() -> Self {
    let inner = KeystoreInner::new_in_memory();
    Self(RwLock::new(inner))
  }

  pub fn has_key<T: CorePair + 'static>(&self, public: &T::Public) -> bool {
    self
      .0
      .read()
      .key_pair_by_type::<T>(public)
      .map(|pair| pair.is_some())
      .unwrap_or(false)
  }

  pub fn multi_public_keys(&self) -> Vec<MultiSigner> {
    let keys = self.0.read().raw_public_keys().unwrap_or_default();
    keys
      .into_iter()
      .map(|(k, c)| {
        let multi = match c {
          CryptoScheme::Ed25519 => {
            ed25519::Public::from_slice(k.as_slice()).map(|p| MultiSigner::Ed25519(p.into()))
          }
          CryptoScheme::Sr25519 => {
            sr25519::Public::from_slice(k.as_slice()).map(|p| MultiSigner::Sr25519(p.into()))
          }
          CryptoScheme::Ecdsa => {
            ecdsa::Public::from_slice(k.as_slice()).map(|p| MultiSigner::Ecdsa(p.into()))
          }
        };
        multi.unwrap()
      })
      .collect()
  }

  pub fn public_keys<T: CorePair + 'static>(&self) -> Vec<T::Public> {
    let scheme = get_crypto_scheme::<T>();
    self
      .0
      .read()
      .raw_public_keys()
      .map(|v| {
        v.into_iter()
          .filter_map(|(k, c)| {
            if c == scheme {
              T::Public::from_slice(k.as_slice()).ok()
            } else {
              None
            }
          })
          .collect()
      })
      .unwrap_or_default()
  }

  pub fn generate_new<T: CorePair + 'static>(&self, seed: Option<&str>) -> Result<T::Public> {
    let pair = match seed {
      Some(seed) => self.0.write().insert_ephemeral_from_seed_by_type::<T>(seed),
      None => self.0.write().generate_by_type::<T>(),
    }?;
    Ok(pair.public())
  }

  pub fn multi_sign(&self, public: &MultiSigner, msg: &[u8]) -> Result<Option<MultiSignature>> {
    match public {
      MultiSigner::Ed25519(public) => self.sign::<ed25519::Pair>(public, msg),
      MultiSigner::Sr25519(public) => self.sign::<sr25519::Pair>(public, msg),
      MultiSigner::Ecdsa(public) => self.sign::<ecdsa::Pair>(public, msg),
    }
  }

  pub fn sign<T: CorePair + 'static>(
    &self,
    public: &T::Public,
    msg: &[u8],
  ) -> Result<Option<MultiSignature>> {
    let Some(pair) = self.0.read().key_pair_by_type::<T>(public)? else {
      return Ok(None);
    };
    let pair: T = pair;
    let crypto_scheme = get_crypto_scheme::<T>();

    let signature = pair.sign(msg);
    let signature = signature.as_ref();
    let multi_signature: MultiSignature = match crypto_scheme {
      CryptoScheme::Ed25519 => ed25519::Signature::try_from(signature)
        .map_err(|_| Error::SignatureConversionError)?
        .into(),
      CryptoScheme::Sr25519 => sr25519::Signature::try_from(signature)
        .map_err(|_| Error::SignatureConversionError)?
        .into(),
      CryptoScheme::Ecdsa => ecdsa::Signature::try_from(signature)
        .map_err(|_| Error::SignatureConversionError)?
        .into(),
    };

    Ok(Some(multi_signature))
  }

  pub fn insert(
    &self,
    suri: &str,
    crypto_scheme: CryptoScheme,
    password: Option<SecretString>,
  ) -> std::result::Result<MultiSigner, ()> {
    let mut pass_str = password.map(|x| x.expose_secret().clone());

    let borrowed_pass = pass_str.as_ref().map(|x| x.as_str());
    let public = match crypto_scheme {
      CryptoScheme::Ed25519 => {
        let pair = ed25519::Pair::from_string(suri, borrowed_pass).map_err(|_| ())?;
        let public = pair.public();
        self
          .0
          .write()
          .insert::<ed25519::Pair>(suri, &public)
          .map_err(|_| ())?;
        MultiSigner::Ed25519(public)
      }
      CryptoScheme::Sr25519 => {
        let pair = sr25519::Pair::from_string(suri, borrowed_pass).map_err(|_| ())?;
        let public = pair.public();
        self
          .0
          .write()
          .insert::<sr25519::Pair>(suri, &public)
          .map_err(|_| ())?;
        MultiSigner::Sr25519(public)
      }
      CryptoScheme::Ecdsa => {
        let pair = ecdsa::Pair::from_string(suri, borrowed_pass).map_err(|_| ())?;
        let public = pair.public();
        self
          .0
          .write()
          .insert::<ecdsa::Pair>(suri, &public)
          .map_err(|_| ())?;
        MultiSigner::Ecdsa(public)
      }
    };
    pass_str.zeroize();
    Ok(public)
  }
}

/// A local key store.
///
/// Stores key pairs in a file system store + short lived key pairs in memory.
///
/// Every pair that is being generated by a `seed`, will be placed in memory.
struct KeystoreInner {
  path: Option<PathBuf>,
  /// Map over `( Raw public key, CryptoScheme)` -> `Key phrase/seed`
  additional: HashMap<(Vec<u8>, CryptoScheme), String>,
  password: Option<SecretString>,
}

impl KeystoreInner {
  /// Open the store at the given path.
  ///
  /// Optionally takes a password that will be used to encrypt/decrypt the keys.
  fn open<T: Into<PathBuf>>(path: T, password: Option<SecretString>) -> Result<Self> {
    let path = path.into();
    fs::create_dir_all(&path)?;

    Ok(Self {
      path: Some(path),
      additional: HashMap::new(),
      password,
    })
  }

  /// Get the password for this store.
  fn password(&self) -> Option<&str> {
    self
      .password
      .as_ref()
      .map(|p| p.expose_secret())
      .map(|p| p.as_str())
  }

  /// Create a new in-memory store.
  fn new_in_memory() -> Self {
    Self {
      path: None,
      additional: HashMap::new(),
      password: None,
    }
  }

  /// Get the key phrase for the given public key and key type from the in-memory store.
  fn get_additional_pair(&self, public: &[u8], scheme: CryptoScheme) -> Option<&String> {
    let key = (public.to_vec(), scheme);
    self.additional.get(&key)
  }

  /// Insert the given public/private key pair with the given key type.
  ///
  /// Does not place it into the file system store.
  fn insert_ephemeral_pair<Pair: CorePair + 'static>(&mut self, pair: &Pair, seed: &str) {
    let scheme = get_crypto_scheme::<Pair>();
    let key = (pair.public().to_raw_vec(), scheme);
    self.additional.insert(key, seed.into());
  }

  /// Insert a new key with anonymous crypto.
  ///
  /// Places it into the file system store, if a path is configured.
  fn insert<Pair: CorePair + 'static>(&mut self, suri: &str, public: &Pair::Public) -> Result<()> {
    let scheme = get_crypto_scheme::<Pair>();
    if self.path.is_none() {
      self.additional.insert((public.to_raw_vec(), scheme), suri.into());
      return Ok(());
    }
    if let Some(path) = self.key_file_path(public.as_ref(), scheme) {
      Self::write_to_file(path, suri)?;
    }

    Ok(())
  }

  /// Generate a new key.
  ///
  /// Places it into the file system store, if a path is configured. Otherwise insert
  /// it into the memory cache only.
  fn generate_by_type<Pair: CorePair + 'static>(&mut self) -> Result<Pair> {
    let (pair, phrase, _) = Pair::generate_with_phrase(self.password());
    let public = pair.public();
    let scheme = get_crypto_scheme::<Pair>();

    if let Some(path) = self.key_file_path(public.as_slice(), scheme) {
      Self::write_to_file(path, &phrase)?;
    } else {
      self.insert_ephemeral_pair(&pair, &phrase);
    }

    Ok(pair)
  }

  /// Write the given `data` to `file`.
  fn write_to_file(file: PathBuf, data: &str) -> Result<()> {
    let mut file = File::create(file)?;

    #[cfg(target_family = "unix")]
    {
      use std::os::unix::fs::PermissionsExt;
      file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }

    serde_json::to_writer(&file, data)?;
    file.flush()?;
    Ok(())
  }

  /// Create a new key from seed.
  ///
  /// Does not place it into the file system store.
  fn insert_ephemeral_from_seed_by_type<Pair: CorePair + 'static>(
    &mut self,
    seed: &str,
  ) -> Result<Pair> {
    let pair = Pair::from_string(seed, None).map_err(|_| Error::InvalidSeed)?;

    self.insert_ephemeral_pair(&pair, seed);
    Ok(pair)
  }

  /// Get the key phrase for a given public key and key type.
  fn key_phrase_by_type(&self, public: &[u8], scheme: CryptoScheme) -> Result<Option<String>> {
    if let Some(phrase) = self.get_additional_pair(public, scheme) {
      return Ok(Some(phrase.clone()));
    }

    let path = if let Some(path) = self.key_file_path(public, scheme) {
      path
    } else {
      return Ok(None);
    };

    if path.exists() {
      let file = File::open(path)?;

      serde_json::from_reader(&file).map_err(Into::into).map(Some)
    } else {
      Ok(None)
    }
  }

  /// Get a key pair for the given public key and key type.
  pub fn key_pair_by_type<Pair: CorePair + 'static>(
    &self,
    public: &Pair::Public,
  ) -> Result<Option<Pair>> {
    let scheme = get_crypto_scheme::<Pair>();
    let phrase = if let Some(p) = self.key_phrase_by_type(public.as_slice(), scheme)? {
      p
    } else {
      return Ok(None);
    };

    let pair = Pair::from_string(&phrase, self.password()).map_err(|_| Error::InvalidPhrase)?;

    if &pair.public() == public {
      Ok(Some(pair))
    } else {
      Err(Error::PublicKeyMismatch)
    }
  }

  /// Get the file path for the given public key and key type.
  ///
  /// Returns `None` if the keystore only exists in-memory and there isn't any path to provide.
  fn key_file_path(&self, public: &[u8], scheme: CryptoScheme) -> Option<PathBuf> {
    let mut buf = self.path.as_ref()?.clone();
    let prefix = match scheme {
      CryptoScheme::Ed25519 => array_bytes::bytes2hex("", &ED_CRYPTO_ID.0),
      CryptoScheme::Sr25519 => array_bytes::bytes2hex("", &SR_CRYPTO_ID.0),
      CryptoScheme::Ecdsa => array_bytes::bytes2hex("", &ECDSA_CRYPTO_ID.0),
    };
    let key = array_bytes::bytes2hex("", public);
    buf.push(prefix + key.as_str());
    Some(buf)
  }

  /// Returns a list of raw public keys filtered by `CryptoScheme`
  fn raw_public_keys(&self) -> Result<Vec<(Vec<u8>, CryptoScheme)>> {
    let mut public_keys: Vec<(Vec<u8>, CryptoScheme)> = self
      .additional
      .keys()
      .into_iter()
      .map(|a| a.clone())
      .collect::<Vec<_>>();

    if let Some(path) = &self.path {
      for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let path = entry.path();

        // skip directories and non-unicode file names (hex is unicode)
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
          continue;
        };
        match array_bytes::hex2bytes(name) {
          Ok(ref hex) if hex.len() > 4 => {
            let Some(scheme) = get_prefix_crypto_id(&hex[0..4]) else {
              continue;
            };
            let public = hex[4..].to_vec();
            public_keys.push((public, scheme));
          }
          _ => continue,
        }
      }
    }

    Ok(public_keys)
  }
}

/// Keystore error
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// IO error.
  #[error(transparent)]
  Io(#[from] io::Error),
  /// JSON error.
  #[error(transparent)]
  Json(#[from] serde_json::Error),
  /// Invalid password.
  #[error(
  "Requested public key and public key of the loaded private key do not match. \n
			This means either that the keystore password is incorrect or that the private key was stored under a wrong public key."
  )]
  PublicKeyMismatch,
  /// Invalid BIP39 phrase
  #[error("Invalid recovery phrase (BIP39) data")]
  InvalidPhrase,
  /// Invalid seed
  #[error("Invalid seed")]
  InvalidSeed,
  /// Public key type is not supported
  #[error("Crypto type is not supported")]
  CryptoSchemeNotSupported,
  /// Keystore unavailable
  #[error("Keystore unavailable")]
  Unavailable,
  /// Signature Conversion Error
  #[error("Failed to convert signature from bytes")]
  SignatureConversionError,
}

fn get_crypto_scheme<P: Pair + 'static>() -> CryptoScheme {
  let type_id = TypeId::of::<P::Public>();

  if type_id == TypeId::of::<ed25519::Public>() {
    CryptoScheme::Ed25519
  } else if type_id == TypeId::of::<sr25519::Public>() {
    CryptoScheme::Sr25519
  } else if type_id == TypeId::of::<ecdsa::Public>() {
    CryptoScheme::Ecdsa
  } else {
    unreachable!()
  }
}

fn get_prefix_crypto_id(prefix: &[u8]) -> Option<CryptoScheme> {
  if prefix == ED_CRYPTO_ID.0 {
    Some(CryptoScheme::Ed25519)
  } else if prefix == SR_CRYPTO_ID.0 {
    Some(CryptoScheme::Sr25519)
  } else if prefix == ECDSA_CRYPTO_ID.0 {
    Some(CryptoScheme::Ecdsa)
  } else {
    None
  }
}

#[napi]
#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum CryptoScheme {
  Ed25519,
  Sr25519,
  Ecdsa,
}

impl Hash for CryptoScheme {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    format!("{:?}", self).hash(state);
  }
}

#[cfg(test)]
mod tests {
  use std::{fs, str::FromStr};

  use sp_core::{ed25519, sr25519, Pair};
  use sp_core::crypto::Ss58Codec;
  use sp_core::testing::SR25519;
  use tempfile::TempDir;

  use super::*;

  #[test]
  fn basic_store() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = KeystoreInner::open(temp_dir.path(), None).unwrap();

    assert!(store.raw_public_keys().unwrap().is_empty());

    let key: ed25519::Pair = store.generate_by_type::<ed25519::Pair>().unwrap();
    let key2: ed25519::Pair = store.key_pair_by_type(&key.public()).unwrap().unwrap();

    assert_eq!(key.public(), key2.public());

    assert_eq!(
      store.raw_public_keys().unwrap()[0].0,
      key.public().to_vec()
    );
  }

  #[test]
  fn has_keys_works() {
    let temp_dir = TempDir::new().unwrap();
    let store = LocalKeystore::open(temp_dir.path(), None).unwrap();

    let key: ed25519::Pair = store.0.write().generate_by_type().unwrap();
    let key2 = ed25519::Pair::generate().0;

    assert!(store.has_key::<ed25519::Pair>(&key.public()));
    assert!(!store.has_key::<ed25519::Pair>(&key2.public()));
  }

  #[test]
  fn test_insert_ephemeral_from_seed() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = KeystoreInner::open(temp_dir.path(), None).unwrap();

    let pair: ed25519::Pair = store
      .insert_ephemeral_from_seed_by_type(
        "0x3d97c819d68f9bafa7d6e79cb991eebcd77d966c5334c0b94d9e1fa7ad0869dc",
      )
      .unwrap();
    assert_eq!(
      "5DKUrgFqCPV8iAXx9sjy1nyBygQCeiUYRFWurZGhnrn3HJCA",
      pair.public().to_ss58check()
    );

    drop(store);
    let store = KeystoreInner::open(temp_dir.path(), None).unwrap();
    // Keys generated from seed should not be persisted!
    assert!(store
      .key_pair_by_type::<ed25519::Pair>(&pair.public())
      .unwrap()
      .is_none());
  }

  #[test]
  fn password_being_used() {
    let password = String::from("password");
    let temp_dir = TempDir::new().unwrap();
    let mut store = KeystoreInner::open(
      temp_dir.path(),
      Some(FromStr::from_str(password.as_str()).unwrap()),
    )
    .unwrap();

    let pair: ed25519::Pair = store.generate_by_type().unwrap();
    assert_eq!(
      pair.public(),
      store
        .key_pair_by_type::<ed25519::Pair>(&pair.public())
        .unwrap()
        .unwrap()
        .public(),
    );

    // Without the password the key should not be retrievable
    let store = KeystoreInner::open(temp_dir.path(), None).unwrap();
    assert!(store.key_pair_by_type::<ed25519::Pair>(&pair.public()).is_err());

    let store = KeystoreInner::open(
      temp_dir.path(),
      Some(FromStr::from_str(password.as_str()).unwrap()),
    )
    .unwrap();
    assert_eq!(
      pair.public(),
      store
        .key_pair_by_type::<ed25519::Pair>(&pair.public())
        .unwrap()
        .unwrap()
        .public(),
    );
  }

  #[test]
  fn store_unknown_and_extract_it() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = KeystoreInner::open(temp_dir.path(), None).unwrap();

    let secret_uri = "//Alice";
    let key_pair = sr25519::Pair::from_string(secret_uri, None).expect("Generates key pair");

    store
      .insert::<sr25519::Pair>(secret_uri, &key_pair.public())
      .expect("Inserts unknown key");

    let store_key_pair = store
      .key_pair_by_type::<sr25519::Pair>(&key_pair.public())
      .expect("Gets key pair from keystore")
      .unwrap();

    assert_eq!(key_pair.public(), store_key_pair.public());
  }

  #[test]
  fn store_ignores_files_with_invalid_name() {
    let temp_dir = TempDir::new().unwrap();
    let store = LocalKeystore::open(temp_dir.path(), None).unwrap();

    let file_name = temp_dir
      .path()
      .join(array_bytes::bytes2hex("", &SR25519.0[..2]));
    fs::write(file_name, "test").expect("Invalid file is written");

    assert!(store.public_keys::<sr25519::Pair>().is_empty());
  }

  #[test]
  fn generate_with_seed_is_not_stored() {
    let temp_dir = TempDir::new().unwrap();
    let store = LocalKeystore::open(temp_dir.path(), None).unwrap();
    let _alice_tmp_key = store
      .generate_new::<sr25519::Pair>(Some("//Alice"))
      .unwrap();

    assert_eq!(store.public_keys::<sr25519::Pair>().len(), 1);

    drop(store);
    let store = LocalKeystore::open(temp_dir.path(), None).unwrap();
    assert_eq!(store.public_keys::<sr25519::Pair>().len(), 0);
  }

  #[test]
  fn generate_can_be_fetched_in_memory() {
    let store = LocalKeystore::in_memory();
    store
      .generate_new::<sr25519::Pair>(Some("//Alice"))
      .unwrap();

    assert_eq!(store.public_keys::<sr25519::Pair>().len(), 1);
    store.generate_new::<sr25519::Pair>(None).unwrap();
    assert_eq!(store.public_keys::<sr25519::Pair>().len(), 2);
  }

  #[test]
  #[cfg(target_family = "unix")]
  fn uses_correct_file_permissions_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let store = LocalKeystore::open(temp_dir.path(), None).unwrap();

    let public = store.generate_new::<sr25519::Pair>(None).unwrap();

    let path = store
      .0
      .read()
      .key_file_path(public.as_ref(), CryptoScheme::Sr25519)
      .unwrap();
    let permissions = File::open(path).unwrap().metadata().unwrap().permissions();

    assert_eq!(0o100600, permissions.mode());
  }
}
