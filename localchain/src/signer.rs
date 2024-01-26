use crate::{to_js_error, AccountStore, Localchain};
use codec::Encode;
use key_types::ACCOUNT;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ErrorStrategy;
use napi::threadsafe_function::ThreadsafeFunction;
use sc_cli::KeystoreParams;
use sc_keystore::LocalKeystore;
use sc_service::config::KeystoreConfig;
use sc_service::BasePath;
use sp_core::crypto::SecretString;
use sp_core::crypto::{key_types, CryptoTypeId, Ss58Codec};
use sp_core::ecdsa::CRYPTO_ID as ECDSA_CRYPTO_ID;
use sp_core::ed25519::CRYPTO_ID as ED_CRYPTO_ID;
use sp_core::sr25519::CRYPTO_ID as SR_CRYPTO_ID;
use sp_core::ByteArray;
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::MultiSignature;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[napi]
pub struct Signer {
  sign_from_js: Option<ThreadsafeFunction<(String, Uint8Array), ErrorStrategy::Fatal>>,
  keystore: Option<KeystorePtr>,
  address_to_public_keys: Arc<Mutex<HashMap<String, (Vec<u8>, CryptoTypeId)>>>,
}

#[napi]
impl Signer {
  #[napi(
    constructor,
    ts_args_type = "signer: (address: string, signatureMessage: Uint8Array) => Promise<Uint8Array>"
  )]
  pub fn new(signer: ThreadsafeFunction<(String, Uint8Array), ErrorStrategy::Fatal>) -> Self {
    Signer {
      sign_from_js: Some(signer),
      keystore: None,
      address_to_public_keys: Arc::new(Mutex::new(HashMap::new())),
    }
  }

  pub fn with_keystore(keystore: KeystorePtr) -> Self {
    Signer {
      sign_from_js: None,
      keystore: Some(keystore),
      address_to_public_keys: Arc::new(Mutex::new(HashMap::new())),
    }
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
    let base_path = BasePath::new(Localchain::get_default_path());
    let keystore: KeystorePtr = match keystore_params
      .keystore_config(base_path.path())
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
  pub async fn sign(&self, address: String, message: Uint8Array) -> Result<Uint8Array> {
    if self.keystore.is_some() && self.get_key_type(address.clone()).await.is_ok() {
      return self.sign_with_keystore(address, message).await;
    }

    if let Some(sign_from_js) = &self.sign_from_js {
      let promise_result = sign_from_js
        .call_async::<Promise<Uint8Array>>((address, message))
        .await?;
      let signature = promise_result.await?;
      return Ok(signature);
    }

    Err(Error::from_reason("Unable to sign"))
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
    let (public, crypto_type_id) = self.get_key_type(address.clone()).await?;
    let multisignature: MultiSignature = match crypto_type_id {
      SR_CRYPTO_ID => {
        let public = &public[..]
          .try_into()
          .map_err(|e| Error::from_reason(format!("Error converting public key {:?}", e)))?;
        let result = keystore
          .sr25519_sign(ACCOUNT, &public, &message.to_vec())
          .map_err(|e| Error::from_reason(format!("Error signing {}", e.to_string())))?
          .ok_or(Error::from_reason("Could not sign"))?;
        result.into()
      }
      ED_CRYPTO_ID => {
        let public = &public[..]
          .try_into()
          .map_err(|e| Error::from_reason(format!("Error converting public key {:?}", e)))?;
        let result = keystore
          .ed25519_sign(ACCOUNT, &public, &message.to_vec())
          .map_err(|e| Error::from_reason(format!("Error signing {}", e.to_string())))?
          .ok_or(Error::from_reason("Could not sign"))?;
        result.into()
      }
      ECDSA_CRYPTO_ID => {
        let public = &public[..]
          .try_into()
          .map_err(|e| Error::from_reason(format!("Error converting public key {:?}", e)))?;
        let result = keystore
          .ecdsa_sign(ACCOUNT, &public, &message.to_vec())
          .map_err(|e| Error::from_reason(format!("Error signing {}", e.to_string())))?
          .ok_or(Error::from_reason("Could not sign"))?;
        result.into()
      }
      _ => {
        return Err(Error::from_reason(format!(
          "Unsupported crypto type id {:?}",
          crypto_type_id
        )))
      }
    };
    return Ok(Uint8Array::from(multisignature.encode()));
  }

  async fn get_key_type(&self, address: String) -> Result<(Vec<u8>, CryptoTypeId)> {
    let mut address_to_public_keys = self.address_to_public_keys.lock().await;
    if let Some(result) = (*address_to_public_keys).get(&address) {
      return Ok((result.0.clone(), result.1));
    }
    let keystore = &self
      .keystore
      .clone()
      .ok_or(Error::from_reason("No keystore".to_string()))?;

    let address_format = AccountStore::address_format();

    if let Some(public) = &keystore
      .sr25519_public_keys(ACCOUNT)
      .iter()
      .find(|x| x.to_ss58check_with_version(address_format) == address)
    {
      let result = (public.to_raw_vec(), SR_CRYPTO_ID);
      address_to_public_keys.insert(address, result.clone());
      return Ok(result);
    }

    if let Some(public) = &keystore
      .ed25519_public_keys(ACCOUNT)
      .iter()
      .find(|x| x.to_ss58check_with_version(address_format) == address)
    {
      let result = (public.to_raw_vec(), ED_CRYPTO_ID);
      address_to_public_keys.insert(address, result.clone());
      return Ok(result);
    }

    if let Some(public) = &keystore
      .ecdsa_public_keys(ACCOUNT)
      .iter()
      .find(|x| x.to_ss58check_with_version(address_format) == address)
    {
      let result = (public.to_raw_vec(), ECDSA_CRYPTO_ID);
      address_to_public_keys.insert(address, result.clone());
      return Ok(result);
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
