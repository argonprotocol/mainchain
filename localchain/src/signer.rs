use crate::keystore::CryptoScheme;
use crate::LocalKeystore;
use crate::Localchain;
use crate::{to_js_error, with_crypto_scheme, AccountStore};
use codec::Encode;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ErrorStrategy;
use napi::threadsafe_function::ThreadsafeFunction;
use sc_cli::KeystoreParams;
use sc_service::config::KeystoreConfig;
use sp_core::crypto::SecretString;
use sp_core::crypto::Ss58Codec;
use sp_core::{ByteArray, Pair};
use sp_runtime::traits::IdentifyAccount;
use sp_runtime::MultiSigner;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[napi]
pub struct Signer {
  sign_from_js: Option<ThreadsafeFunction<(String, Uint8Array), ErrorStrategy::Fatal>>,
  keystore: Option<Arc<LocalKeystore>>,
  address_to_public_keys: Arc<Mutex<HashMap<String, MultiSigner>>>,
}

#[napi]
impl Signer {
  #[napi(
    constructor,
    ts_args_type = "signer?: (address: string, signatureMessage: Uint8Array) => Promise<Uint8Array>"
  )]
  pub fn new(
    signer: Option<ThreadsafeFunction<(String, Uint8Array), ErrorStrategy::Fatal>>,
  ) -> Self {
    Signer {
      sign_from_js: signer,
      keystore: None,
      address_to_public_keys: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  pub fn with_keystore(keystore: LocalKeystore) -> Self {
    Signer {
      sign_from_js: None,
      keystore: Some(Arc::new(keystore)),
      address_to_public_keys: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  #[napi]
  pub fn create_account_id(&self, scheme: CryptoScheme) -> Result<String> {
    let Some(ref keystore) = self.keystore else {
      return Err(Error::from_reason("No keystore loaded"));
    };
    with_crypto_scheme!(scheme, generate_key(None, keystore))
  }

  #[napi]
  pub async unsafe fn attach_keystore(
    &mut self,
    path: String,
    password: KeystorePasswordOption,
  ) -> Result<()> {
    let mut keystore_params = KeystoreParams {
      keystore_path: Some(PathBuf::from(path.clone())),
      password_interactive: false,
      password: None,
      password_filename: None,
    };
    if let Some(password) = password.password {
      let temp_string = String::from_utf8(password.to_vec())
        .map_err(|_| Error::from_reason("Cannot read provided password buffer"))?;
      keystore_params.password = Some(SecretString::new(temp_string));
    } else if let Some(password_file) = password.password_file {
      keystore_params.password_filename = Some(PathBuf::from(password_file));
    } else if let Some(interactive_cli) = password.interactive_cli {
      keystore_params.password_interactive = interactive_cli;
    }
    let base_path = PathBuf::from(Localchain::get_default_path());
    let keystore = match keystore_params
      .keystore_config(base_path.as_path())
      .map_err(to_js_error)?
    {
      KeystoreConfig::Path { path, password } => LocalKeystore::open(path, password)
        .map_err(to_js_error)?
        .into(),
      _ => unreachable!("keystore_config always returns path and password; qed"),
    };
    self.keystore = Some(keystore);
    Ok(())
  }

  #[napi]
  pub fn can_sign(&self, address: String) -> bool {
    if let Some(keystore) = self.keystore.as_ref() {
      for key in keystore.multi_public_keys() {
        let account_id = key.into_account();
        let key_address = AccountStore::to_address(&account_id);
        if key_address == address {
          return true;
        }
      }
    }
    return false;
  }

  #[napi]
  pub async fn sign(&self, address: String, message: Uint8Array) -> Result<Uint8Array> {
    if self.keystore.is_some() && self.get_public_key(address.clone()).await.is_ok() {
      return self.sign_with_keystore(address, message).await;
    }

    if let Some(sign_from_js) = &self.sign_from_js {
      let promise_result = sign_from_js
        .call_async::<Promise<Uint8Array>>((address, message))
        .await?;
      let signature = promise_result.await?;
      return Ok(signature);
    }

    Err(Error::from_reason(format!("Unable to sign for address {}", address)))
  }

  #[napi]
  pub async fn sign_with_keystore(
    &self,
    address: String,
    message: Uint8Array,
  ) -> Result<Uint8Array> {
    let keystore = self
      .keystore
      .clone()
      .ok_or(Error::from_reason("No keystore loaded"))?;
    let public = self.get_public_key(address.clone()).await?;
    let signature = keystore
      .multi_sign(&public, &message)
      .map_err(to_js_error)?
        .ok_or(Error::from_reason("No signature"))?;

    return Ok(Uint8Array::from(signature.encode()));
  }

  async fn get_public_key(&self, address: String) -> Result<MultiSigner> {
    let mut address_to_public_keys = self.address_to_public_keys.lock().await;
    if let Some(result) = (*address_to_public_keys).get(&address) {
      return Ok(result.clone());
    }
    let keystore = &self
      .keystore
      .clone()
      .ok_or(Error::from_reason("No keystore".to_string()))?;

    for key in keystore.multi_public_keys() {
      let account_id = key.clone().into_account();
      if AccountStore::to_address(&account_id) == address {
        address_to_public_keys.insert(address, key.clone());
        return Ok(key);
      }
    }

    Err(Error::from_reason(format!(
      "Could not find key for address {}",
      address
    )))
  }
}

/// Options to provide the password for a keystore. NOTE that this library cannot clear out memory in javascript.
/// Only a single option should be picked.
#[napi(object)]
pub struct KeystorePasswordOption {
  /// Provides a password directly for the keystore. Converted to a SecretString inside Rust, but not cleared out in javascript or napi.
  pub password: Option<Buffer>,
  /// Initiate a prompt from the cli to load the password.
  pub interactive_cli: Option<bool>,
  /// Load the password from a file.
  pub password_file: Option<String>,
}

pub fn get_address<P: Pair + 'static>(public: &Vec<u8>) -> Result<String> {
  Ok(
    P::Public::from_slice(public.as_slice())
      .map_err(|_| to_js_error("Could not translate slice to public key"))?
      .to_ss58check_with_version(AccountStore::address_format()),
  )
}

pub fn generate_key<P: Pair + 'static>(seed: Option<&str>, keystore: &LocalKeystore) -> Result<String> {
  let public = keystore.generate_new::<P>(seed).map_err(to_js_error)?;
  let address = public.to_ss58check_with_version(AccountStore::address_format());
  Ok(address)
}
