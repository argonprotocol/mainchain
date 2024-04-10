use anyhow::anyhow;
use codec::Encode;
use sp_core::crypto::SecretString;
use sp_core::crypto::Ss58Codec;
use sp_core::{ByteArray, Pair};
use sqlx::SqlitePool;
use std::fs;
use std::path::PathBuf;
#[cfg(feature = "napi")]
use std::sync::Arc;
#[cfg(feature = "napi")]
use tokio::sync::Mutex;

use crate::cli::EmbeddedKeyPassword;
use crate::embedded_keystore::{CryptoScheme, EmbeddedKeystore};
use crate::AccountStore;
use crate::{bail, Result};

#[cfg_attr(feature = "napi", napi)]
#[derive(Clone)]
pub struct Keystore {
  #[cfg(feature = "napi")]
  js_callbacks: Arc<Mutex<Option<(napi_ext::JsCallbacks, String)>>>,
  embedded_keystore: EmbeddedKeystore,
  db: SqlitePool,
}

impl Keystore {
  pub(crate) fn new(db: SqlitePool) -> Self {
    Keystore {
      #[cfg(feature = "napi")]
      js_callbacks: Default::default(),
      embedded_keystore: EmbeddedKeystore::new(db.clone()),
      db,
    }
  }

  /// Bootstrap this localchain with a new key. Must be empty or will throw an error! Defaults to SR25519 if no scheme is provided.
  pub async fn bootstrap(
    &self,
    // The crypto scheme. Defaults to SR25519 if not provided.
    scheme: Option<CryptoScheme>,
    password_option: Option<EmbeddedKeyPassword>,
  ) -> Result<String> {
    let mut password = None;
    if let Some(password_option) = password_option {
      password = password_option.get_password()?;
    }
    let scheme = scheme.unwrap_or(CryptoScheme::Sr25519);
    let address = self.embedded_keystore.create(scheme, password).await?;

    AccountStore::bootstrap(self.db.clone(), address.clone(), None).await?;
    Ok(address)
  }

  /// Import a known keypair into the embedded keystore.
  pub async fn import_suri(
    &self,
    // The secret uri path to import.
    suri: String,
    // The crypto scheme.
    scheme: CryptoScheme,
    password_option: Option<EmbeddedKeyPassword>,
  ) -> Result<String> {
    let mut password = None;
    if let Some(password_option) = password_option {
      password = password_option.get_password()?;
    }

    let address = self
      .embedded_keystore
      .import(&suri, scheme, password)
      .await?;
    AccountStore::bootstrap(self.db.clone(), address.clone(), None).await?;
    Ok(address)
  }

  pub async fn unlock(&self, password_option: Option<EmbeddedKeyPassword>) -> Result<()> {
    let mut password = None;
    if let Some(password_option) = password_option {
      password = password_option.get_password()?;
    }
    self.embedded_keystore.unlock(password).await?;
    Ok(())
  }

  pub async fn lock(&self) {
    self.embedded_keystore.lock().await;
  }

  pub async fn is_unlocked(&self) -> bool {
    self.embedded_keystore.is_unlocked().await
  }

  pub async fn derive_account_id(&self, hd_path: String) -> Result<String> {
    if self.is_unlocked().await {
      return self.embedded_keystore.derive(&hd_path).await;
    };

    #[cfg(feature = "napi")]
    {
      if let Some(js_callbacks) = self.js_callbacks.lock().await.as_ref() {
        let promise_result = js_callbacks
          .0
          .derive
          .call_async(Ok(hd_path))
          .await?;
        return Ok(promise_result.await?);
      }
    }
    bail!("No keystore loaded");
  }

  pub async fn can_sign(&self, address: String) -> bool {
    self.embedded_keystore.can_sign(address).await
  }

  pub async fn sign(&self, address: String, message: Vec<u8>) -> Result<Vec<u8>> {
    if self.embedded_keystore.is_unlocked().await {
      if let Some(signature) = self
        .embedded_keystore
        .sign(address.clone(), message.as_ref())
        .await?
      {
        return Ok(signature.encode().into());
      }
    }

    #[cfg(feature = "napi")]
    {
      if let Some(js_callbacks) = self.js_callbacks.lock().await.as_ref() {
        let promise_result = js_callbacks
          .0
          .sign
          .call_async(Ok((address, message.into())))
          .await?;
        let buffer = promise_result.await?;
        return Ok(buffer.as_ref().to_vec());
      }
    }

    bail!("Unable to sign for address {}", address)
  }
}

#[cfg(feature = "napi")]
pub mod napi_ext {
  use napi::bindgen_prelude::{Buffer, Promise, Uint8Array};
  use napi::threadsafe_function::ThreadsafeFunction;
  use napi_derive::napi;

  use crate::cli::EmbeddedKeyPassword;
  use crate::error::NapiOk;
  use crate::{AccountStore, CryptoScheme, Keystore};

  pub(crate) struct JsCallbacks {
    pub(crate) sign: ThreadsafeFunction<(String, Uint8Array), Promise<Uint8Array>>,
    pub(crate) derive: ThreadsafeFunction<String, Promise<String>>,
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

  impl Into<EmbeddedKeyPassword> for KeystorePasswordOption {
    fn into(self) -> EmbeddedKeyPassword {
      EmbeddedKeyPassword {
        key_password: self
          .password
          .map(|v| String::from_utf8(v.as_ref().to_vec()).expect("Invalid utf8")),
        key_password_interactive: self.interactive_cli.unwrap_or_default(),
        key_password_filename: self.password_file,
      }
    }
  }
  #[napi]
  impl Keystore {
    #[napi(
      ts_args_type = "defaultAddress: string, sign: (address: string, signatureMessage: Uint8Array) => Promise<Uint8Array>, derive: (hd_path: string) => Promise<string>"
    )]
    pub async fn use_external(
      &self,
      default_address: String,
      sign: ThreadsafeFunction<(String, Uint8Array), Promise<Uint8Array>>,
      derive: ThreadsafeFunction<String, Promise<String>>,
    ) -> napi::Result<()> {
      // this will check that the address matches
      AccountStore::bootstrap(self.db.clone(), default_address.clone(), None)
        .await
        .napi_ok()?;

      let _ = self
        .js_callbacks
        .lock()
        .await
        .insert((JsCallbacks { derive, sign }, default_address));
      Ok(())
    }

    /// Bootstrap this localchain with a new key. Must be empty or will throw an error! Defaults to SR25519 if no scheme is provided.
    #[napi(js_name = "bootstrap")]
    pub async fn bootstrap_napi(
      &self,
      // The crypto scheme. Defaults to SR25519 if not provided.
      scheme: Option<CryptoScheme>,
      password_option: Option<KeystorePasswordOption>,
    ) -> napi::Result<String> {
      self
        .bootstrap(scheme, password_option.map(Into::into))
        .await
        .napi_ok()
    }

    /// Import a known keypair into the embedded keystore.
    #[napi(js_name = "importSuri")]
    pub async fn import_suri_napi(
      &self,
      // The secret uri path to import.
      suri: String,
      // The crypto scheme.
      scheme: CryptoScheme,
      password_option: Option<KeystorePasswordOption>,
    ) -> napi::Result<String> {
      self
        .import_suri(suri, scheme, password_option.map(Into::into))
        .await
        .napi_ok()
    }

    #[napi(js_name = "unlock")]
    pub async fn unlock_napi(
      &self,
      password_option: Option<KeystorePasswordOption>,
    ) -> napi::Result<()> {
      self.unlock(password_option.map(Into::into)).await.napi_ok()
    }

    #[napi(js_name = "lock")]
    pub async fn lock_napi(&self) -> napi::Result<()> {
      self.lock().await;
      Ok(())
    }

    #[napi(js_name = "isUnlocked")]
    pub async fn is_unlocked_napi(&self) -> napi::Result<bool> {
      Ok(self.is_unlocked().await)
    }

    #[napi(js_name = "deriveAccountId")]
    pub async fn derive_account_id_napi(&self, hd_path: String) -> napi::Result<String> {
      self.derive_account_id(hd_path).await.napi_ok()
    }

    #[napi(js_name = "sign")]
    pub async fn sign_napi(
      &self,
      address: String,
      message: Uint8Array,
    ) -> napi::Result<Uint8Array> {
      self
        .sign(address, message.as_ref().to_vec())
        .await
        .map(Into::into)
        .napi_ok()
    }
  }
}

/// Options to provide the password for a keystore. NOTE that this library cannot clear out memory in javascript.
/// Only a single option should be picked.
impl EmbeddedKeyPassword {
  pub fn get_password(&self) -> Result<Option<SecretString>> {
    if self.key_password_interactive == true {
      let password = rpassword::prompt_password("Key password: ")?;
      return Ok(Some(SecretString::new(password)));
    } else if let Some(ref file) = self.key_password_filename {
      let password = fs::read_to_string(PathBuf::from(file))?;
      return Ok(Some(SecretString::new(password)));
    } else if let Some(ref password) = self.key_password {
      return Ok(Some(SecretString::new(password.clone())));
    }
    Ok(None)
  }
}

pub fn get_address<P: Pair + 'static>(public: &Vec<u8>) -> Result<String> {
  Ok(
    P::Public::from_slice(public.as_slice())
      .map_err(|_| anyhow!("Could not translate slice to public key"))?
      .to_ss58check_with_version(AccountStore::address_format()),
  )
}
