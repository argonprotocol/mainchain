use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use codec::Encode;
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ErrorStrategy;
use napi::threadsafe_function::ThreadsafeFunction;
use sp_core::crypto::SecretString;
use sp_core::crypto::Ss58Codec;
use sp_core::{ByteArray, Pair};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::cli::EmbeddedKeyPassword;
use crate::embedded_keystore::{CryptoScheme, EmbeddedKeystore};
use crate::{to_js_error, AccountStore};

#[napi]
#[derive(Clone)]
pub struct Keystore {
  js_callbacks: Arc<Mutex<Option<JsCallbacks>>>,
  embedded_keystore: EmbeddedKeystore,
  db: SqlitePool,
}

struct JsCallbacks {
  sign: ThreadsafeFunction<(String, Uint8Array), ErrorStrategy::Fatal>,
  derive: ThreadsafeFunction<String, ErrorStrategy::Fatal>,
}

#[napi]
impl Keystore {
  pub(crate) fn new(db: SqlitePool) -> Self {
    Keystore {
      js_callbacks: Default::default(),
      embedded_keystore: EmbeddedKeystore::new(db.clone()),
      db,
    }
  }

  #[napi(
    ts_args_type = "defaultAddress: string, sign: (address: string, signatureMessage: Uint8Array) => Promise<Uint8Array>, derive: (hd_path: string) => Promise<string>"
  )]
  pub async fn use_external(
    &self,
    default_address: String,
    sign: ThreadsafeFunction<(String, Uint8Array), ErrorStrategy::Fatal>,
    derive: ThreadsafeFunction<String, ErrorStrategy::Fatal>,
  ) -> Result<()> {
    // this will check that the address matches
    AccountStore::bootstrap(self.db.clone(), default_address, None)
      .await
      .map_err(to_js_error)?;

    let _ = self
      .js_callbacks
      .lock()
      .await
      .insert(JsCallbacks { derive, sign });
    Ok(())
  }

  /// Bootstrap this localchain with a new key. Must be empty or will throw an error! Defaults to SR25519 if no scheme is provided.
  #[napi]
  pub async fn bootstrap(
    &self,
    // The crypto scheme. Defaults to SR25519 if not provided.
    scheme: Option<CryptoScheme>,
    password_option: Option<KeystorePasswordOption>,
  ) -> Result<String> {
    let mut password = None;
    if let Some(password_option) = password_option {
      password = password_option.get_password().map_err(to_js_error)?;
    }
    let scheme = scheme.unwrap_or(CryptoScheme::Sr25519);
    let address = self
      .embedded_keystore
      .create(scheme, password)
      .await
      .map_err(to_js_error)?;

    AccountStore::bootstrap(self.db.clone(), address.clone(), None)
      .await
      .map_err(to_js_error)?;
    Ok(address)
  }

  /// Import a known keypair into the embedded keystore.
  #[napi]
  pub async fn import_suri(
    &self,
    // The secret uri path to import.
    suri: String,
    // The crypto scheme.
    scheme: CryptoScheme,
    password_option: Option<KeystorePasswordOption>,
  ) -> Result<String> {
    let mut password = None;
    if let Some(password_option) = password_option {
      password = password_option.get_password().map_err(to_js_error)?;
    }

    let address = self
      .embedded_keystore
      .import(&suri, scheme, password)
      .await
      .map_err(to_js_error)?;
    AccountStore::bootstrap(self.db.clone(), address.clone(), None)
      .await
      .map_err(to_js_error)?;
    Ok(address)
  }

  #[napi]
  pub async fn unlock(
    &self,
    password_option: Option<KeystorePasswordOption>,
  ) -> Result<()> {
    let mut password = None;
    if let Some(password_option) = password_option {
      password = password_option.get_password().map_err(to_js_error)?;
    }
    self
      .embedded_keystore
      .unlock(password)
      .await
      .map_err(to_js_error)?;
    Ok(())
  }

  #[napi]
  pub async fn lock(&self) {
    self.embedded_keystore.lock().await;
  }

  #[napi]
  pub async fn is_unlocked(&self) -> bool {
    self.embedded_keystore.is_unlocked().await
  }

  #[napi]
  pub async fn derive_account_id(&self, hd_path: String) -> Result<String> {
    if self.is_unlocked().await {
      return self
        .embedded_keystore
        .derive(&hd_path)
        .await
        .map_err(to_js_error);
    };

    if let Some(js_callbacks) = self.js_callbacks.lock().await.as_ref() {
      let promise_result = js_callbacks
        .derive
        .call_async::<Promise<String>>(hd_path)
        .await?;
      return Ok(promise_result.await?);
    }
    Err(Error::from_reason("No keystore loaded"))
  }

  pub async fn can_sign(&self, address: String) -> bool {
    self.embedded_keystore.can_sign(address).await
  }

  #[napi]
  pub async fn sign(&self, address: String, message: Uint8Array) -> Result<Uint8Array> {
    if self.embedded_keystore.is_unlocked().await {
      if let Some(signature) = self
        .embedded_keystore
        .sign(address.clone(), message.as_ref())
        .await?
      {
        return Ok(signature.encode().into());
      }
    }

    if let Some(js_callbacks) = self.js_callbacks.lock().await.as_ref() {
      let promise_result = js_callbacks
        .sign
        .call_async::<Promise<Uint8Array>>((address, message))
        .await?;
      let signature = promise_result.await?;
      return Ok(signature);
    }

    Err(Error::from_reason(format!(
      "Unable to sign for address {}",
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

impl From<EmbeddedKeyPassword> for KeystorePasswordOption {
  fn from(params: EmbeddedKeyPassword) -> Self {
    KeystorePasswordOption {
      password: params.key_password.map(|p| Buffer::from(p.as_bytes())),
      interactive_cli: Some(params.key_password_interactive),
      password_file: params.key_password_filename,
    }
  }
}

impl KeystorePasswordOption {
  pub fn get_password(&self) -> anyhow::Result<Option<SecretString>> {
    if self.interactive_cli == Some(true) {
      let password = rpassword::prompt_password("Key password: ")?;
      return Ok(Some(SecretString::new(password)));
    } else if let Some(ref file) = self.password_file {
      let password = fs::read_to_string(PathBuf::from(file))?;
      return Ok(Some(SecretString::new(password)));
    } else if let Some(ref password) = self.password {
      let password = password.to_vec();
      let password = String::from_utf8(password)?;
      return Ok(Some(SecretString::new(password)));
    }
    Ok(None)
  }
}

pub fn get_address<P: Pair + 'static>(public: &Vec<u8>) -> Result<String> {
  Ok(
    P::Public::from_slice(public.as_slice())
      .map_err(|_| to_js_error("Could not translate slice to public key"))?
      .to_ss58check_with_version(AccountStore::address_format()),
  )
}
