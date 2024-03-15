use anyhow::anyhow;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use codec::{ Encode};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::ErrorStrategy;
use napi::threadsafe_function::ThreadsafeFunction;
use sp_core::crypto::SecretString;
use sp_core::crypto::Ss58Codec;
use sp_core::{ByteArray, Pair};
use sp_runtime::MultiSignature;
use sqlx::SqlitePool;
use subxt::config::Config;
use subxt::tx::Signer;
use subxt::utils::{AccountId32, MultiAddress};
use tokio::sync::Mutex;

use ulixee_client::UlxConfig;

use crate::cli::EmbeddedKeyPassword;
use crate::embedded_keystore::{CryptoScheme, EmbeddedKeystore};
use crate::{to_js_error, AccountStore};

#[napi]
#[derive(Clone)]
pub struct Keystore {
  js_callbacks: Arc<Mutex<Option<(JsCallbacks, String)>>>,
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
    AccountStore::bootstrap(self.db.clone(), default_address.clone(), None)
      .await
      .map_err(to_js_error)?;

    let _ = self
      .js_callbacks
      .lock()
      .await
      .insert((JsCallbacks { derive, sign }, default_address));
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
  pub async fn unlock(&self, password_option: Option<KeystorePasswordOption>) -> Result<()> {
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
        .0
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
        .0
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

  pub async fn get_subxt_signer(&self, address: String) -> anyhow::Result<AccountSubxtSigner> {
    let account = AccountId32::from_str(&address)?;

    if self.embedded_keystore.can_sign(address.clone()).await {
      return Ok(AccountSubxtSigner {
        address,
        account,
        keystore: self.clone(),
      });
    }

    if let Some((_js_callbacks, default_address)) = self.js_callbacks.lock().await.as_ref() {
      if address == *default_address {
        return Ok(AccountSubxtSigner {
          address: default_address.clone(),
          account: account.clone(),
          keystore: self.clone(),
        });
      }
    }

    Err(anyhow!("Unable to sign for address {}", address))
  }
}

pub struct AccountSubxtSigner {
  pub address: String,
  pub account: AccountId32,
  keystore: Keystore,
}

impl Signer<UlxConfig> for AccountSubxtSigner {
  fn account_id(&self) -> <UlxConfig as Config>::AccountId {
    self.account.clone()
  }

  fn address(&self) -> <UlxConfig as Config>::Address {
    MultiAddress::from(self.account_id())
  }

  fn sign(&self, message: &[u8]) -> <UlxConfig as Config>::Signature {
    let keystore = self.keystore.clone();
    let address = self.address.clone();
    let message = message.to_owned();

    let signature = std::thread::spawn(move || {
      // Create a new runtime for the async block
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(async move {
        let signature = keystore
          .embedded_keystore
          .sign(address.clone(), message.as_ref())
          .await
          .unwrap()
          .expect("Unable to sign");
        match signature {
          MultiSignature::Ed25519(a) => <UlxConfig as Config>::Signature::Ed25519(a.0),
          MultiSignature::Sr25519(a) => <UlxConfig as Config>::Signature::Sr25519(a.0),
          MultiSignature::Ecdsa(a) => <UlxConfig as Config>::Signature::Ecdsa(a.0),
        }
      })
    })
    .join()
    .expect("Thread panicked or could not execute");

    signature
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
