use std::collections::HashMap;
use std::str::FromStr;
use std::string::String;
use std::sync::Arc;

use crate::{bail, Result};
use anyhow::anyhow;
use clap::ValueEnum;
use sp_core::crypto::{
  ExposeSecret, Pair as CorePair, SecretString, SecretUri, Ss58Codec, Zeroize,
};
use sp_core::{ecdsa, ed25519, sr25519};
use sp_runtime::MultiSignature;
use sqlx::{FromRow, SqlitePool};
use tokio::sync::Mutex;

use crate::AccountStore;

/// A local based keystore that has a primary key in the local key table
#[derive(Clone)]
pub struct EmbeddedKeystore {
  db: SqlitePool,
  derived_pairs_by_address: Arc<Mutex<HashMap<String, PairWrapper>>>,
  unlocked_account: Arc<Mutex<Option<UnlockedAccount>>>,
}

impl EmbeddedKeystore {
  pub fn new(db: SqlitePool) -> Self {
    Self {
      db,
      derived_pairs_by_address: Arc::new(Mutex::new(HashMap::new())),
      unlocked_account: Arc::new(Mutex::new(None)),
    }
  }

  pub async fn is_unlocked(&self) -> bool {
    self.unlocked_account.lock().await.as_ref().is_some()
  }

  pub async fn lock(&self) {
    self.unlocked_account.lock().await.take();
  }

  pub async fn unlock(&self, password: Option<SecretString>) -> Result<()> {
    let key = sqlx::query_as!(KeyRow, "SELECT * FROM key LIMIT 1",)
      .fetch_one(&self.db)
      .await
      .map_err(|_| anyhow!("No embedded key found"))?;

    let expected_address = key.address.clone();
    let pair = self.unlock_key(key, password.clone()).await?;
    if pair.address() != expected_address {
      bail!("Could not unlock the embedded key");
    }

    *self.unlocked_account.lock().await = Some(UnlockedAccount {
      address: pair.address(),
      password: password.clone(),
    });

    Ok(())
  }

  pub async fn import(
    &self,
    suri: &str,
    crypto_scheme: CryptoScheme,
    password: Option<SecretString>,
  ) -> Result<String> {
    let mut pass_str = password.clone().map(|x| x.expose_secret().clone());

    let pair = PairWrapper::from_string(suri, pass_str.as_deref(), crypto_scheme)
      .map_err(|_| anyhow!("Could not generate pair from secret uri"))?;

    let address = pair.address();
    self
      .insert_pair(address.clone(), suri.to_string(), crypto_scheme)
      .await?;

    *self.unlocked_account.lock().await = Some(UnlockedAccount {
      address: address.clone(),
      password,
    });

    pass_str.zeroize();

    Ok(address)
  }

  /// Bootstrap this localchain with a new key. Must be empty or will throw an error!
  pub async fn create(
    &self,
    crypto_scheme: CryptoScheme,
    password: Option<SecretString>,
  ) -> Result<String> {
    let mut pass_str = password.clone().map(|x| x.expose_secret().clone());
    let (pair, phrase) = PairWrapper::generate_with_phrase(pass_str.as_deref(), crypto_scheme);

    let address = pair.address();
    self
      .insert_pair(address.clone(), phrase, crypto_scheme)
      .await?;
    *self.unlocked_account.lock().await = Some(UnlockedAccount {
      address: address.clone(),
      password,
    });

    pass_str.zeroize();

    Ok(address)
  }

  async fn insert_pair(
    &self,
    address: String,
    phrase: String,
    crypto_scheme: CryptoScheme,
  ) -> Result<()> {
    // Only one row allowed!!
    let existing = sqlx::query!("SELECT address as true FROM key LIMIT 1",)
      .fetch_optional(&self.db)
      .await?;
    if existing.is_some() {
      return Err(anyhow!("This keystore already has an embedded key"))?;
    }

    let phrase_bytes = phrase.as_bytes();
    let crypto_scheme = crypto_scheme as i64;
    let res = sqlx::query!(
      r#"
        INSERT INTO key (address, crypto_type, data)
        VALUES (?, ?, ?)
      "#,
      address,
      crypto_scheme,
      phrase_bytes,
    )
    .execute(&self.db)
    .await?;
    if res.rows_affected() != 1 {
      bail!("Unable to insert key");
    }

    Ok(())
  }

  /// Create a new derived account from the main key. You must have unlocked the main key to use this function.
  pub async fn derive(&self, path: &str) -> Result<String> {
    let parent_pair = self.load_key().await?;
    let SecretUri { junctions, .. } = SecretUri::from_str(path)?;

    let derived_pair = match parent_pair {
      PairWrapper::Ed25519(parent) => {
        let (pair, _seed) = parent.derive(junctions.into_iter(), None)?;
        PairWrapper::Ed25519(Box::new(pair))
      }
      PairWrapper::Sr25519(parent) => {
        let (pair, _seed) = parent.derive(junctions.into_iter(), None)?;
        PairWrapper::Sr25519(Box::new(pair))
      }
      PairWrapper::Ecdsa(parent) => {
        let (pair, _seed) = parent.derive(junctions.into_iter(), None)?;
        PairWrapper::Ecdsa(Box::new(pair))
      }
    };
    let address = derived_pair.address();
    self
      .derived_pairs_by_address
      .lock()
      .await
      .insert(address.clone(), derived_pair);
    Ok(address)
  }

  pub async fn can_sign(&self, address: String) -> bool {
    self
      .unlocked_account
      .lock()
      .await
      .as_ref()
      .map(|a| a.address == address)
      .unwrap_or(false)
      || self
        .derived_pairs_by_address
        .lock()
        .await
        .contains_key(&address)
  }

  pub async fn sign(&self, address: String, msg: &[u8]) -> Result<Option<MultiSignature>> {
    if let Some(pair) = self.derived_pairs_by_address.lock().await.get(&address) {
      return Ok(Some(pair.sign(msg)));
    }

    let pair = self.load_key().await?;
    if pair.address() == address {
      return Ok(Some(pair.sign(msg)));
    }

    bail!("Unable to sign for address {}", address)
  }

  async fn load_key(&self) -> Result<PairWrapper> {
    let Some(ref unlocked_account) = *self.unlocked_account.lock().await else {
      bail!("This keystore is not unlocked");
    };

    let address = unlocked_account.address.clone();
    let key = sqlx::query_as!(
      KeyRow,
      "SELECT * FROM key WHERE address = ? LIMIT 1",
      address,
    )
    .fetch_one(&self.db)
    .await
    .map_err(|_| anyhow!("No embedded key found"))?;

    let key_address = key.address.clone();
    let pair = self
      .unlock_key(key, unlocked_account.password.clone())
      .await?;

    if pair.address() != key_address {
      bail!("Address mismatch");
    }

    Ok(pair)
  }

  async fn unlock_key(&self, key: KeyRow, password: Option<SecretString>) -> Result<PairWrapper> {
    let suri =
      String::from_utf8(key.data).map_err(|_| anyhow!("Unable to read key data from keystore"))?;

    let password = password
      .as_ref()
      .map(|p| p.expose_secret())
      .map(|p| p.as_str());

    let pair = PairWrapper::from_string(&suri, password, key.crypto_type)
      .map_err(|_| anyhow!("Unable to unlock your embedded key"))?;

    Ok(pair)
  }
}

#[cfg_attr(feature = "napi", napi)]
#[cfg_attr(not(feature = "napi"), derive(Clone, Copy))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum CryptoScheme {
  Ed25519,
  Sr25519,
  Ecdsa,
}

impl From<i64> for CryptoScheme {
  fn from(value: i64) -> Self {
    match value {
      0 => CryptoScheme::Ed25519,
      1 => CryptoScheme::Sr25519,
      2 => CryptoScheme::Ecdsa,
      _ => panic!("Invalid crypto scheme"),
    }
  }
}

#[derive(Clone)]
pub enum PairWrapper {
  Ed25519(Box<ed25519::Pair>),
  Sr25519(Box<sr25519::Pair>),
  Ecdsa(Box<ecdsa::Pair>),
}

#[derive(Clone)]
struct UnlockedAccount {
  address: String,
  password: Option<SecretString>,
}

#[derive(FromRow)]
struct KeyRow {
  address: String,
  crypto_type: CryptoScheme,
  data: Vec<u8>,
}

impl PairWrapper {
  fn sign(&self, msg: &[u8]) -> MultiSignature {
    match self {
      PairWrapper::Ed25519(pair) => pair.sign(msg).into(),
      PairWrapper::Sr25519(pair) => pair.sign(msg).into(),
      PairWrapper::Ecdsa(pair) => pair.sign(msg).into(),
    }
  }

  fn address(&self) -> String {
    match self {
      PairWrapper::Ed25519(pair) => pair
        .public()
        .to_ss58check_with_version(AccountStore::address_format()),
      PairWrapper::Sr25519(pair) => pair
        .public()
        .to_ss58check_with_version(AccountStore::address_format()),
      PairWrapper::Ecdsa(pair) => pair
        .public()
        .to_ss58check_with_version(AccountStore::address_format()),
    }
  }

  fn generate_with_phrase(
    password: Option<&str>,
    crypto_scheme: CryptoScheme,
  ) -> (PairWrapper, String) {
    match crypto_scheme {
      CryptoScheme::Ed25519 => {
        let (pair, phrase, _seed) = ed25519::Pair::generate_with_phrase(password);
        (PairWrapper::Ed25519(Box::new(pair)), phrase)
      }
      CryptoScheme::Sr25519 => {
        let (pair, phrase, _seed) = sr25519::Pair::generate_with_phrase(password);
        (PairWrapper::Sr25519(Box::new(pair)), phrase)
      }
      CryptoScheme::Ecdsa => {
        let (pair, phrase, _seed) = ecdsa::Pair::generate_with_phrase(password);
        (PairWrapper::Ecdsa(Box::new(pair)), phrase)
      }
    }
  }

  fn from_string(suri: &str, password: Option<&str>, crypto_scheme: CryptoScheme) -> Result<Self> {
    match crypto_scheme {
      CryptoScheme::Ed25519 => {
        let pair = ed25519::Pair::from_string(suri, password)?;
        Ok(PairWrapper::Ed25519(Box::new(pair)))
      }
      CryptoScheme::Sr25519 => {
        let pair = sr25519::Pair::from_string(suri, password)?;
        Ok(PairWrapper::Sr25519(Box::new(pair)))
      }
      CryptoScheme::Ecdsa => {
        let pair = ecdsa::Pair::from_string(suri, password)?;
        Ok(PairWrapper::Ecdsa(Box::new(pair)))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use sp_core::{sr25519, Pair};
  use sp_keyring::Sr25519Keyring;

  use super::*;

  #[sqlx::test]
  async fn imports_and_reloads(pool: SqlitePool) -> anyhow::Result<()> {
    let keystore = EmbeddedKeystore::new(pool.clone());
    let address = keystore
      .import("//Alice", CryptoScheme::Sr25519, None)
      .await?;
    let keyring = Sr25519Keyring::Alice
      .to_account_id()
      .to_ss58check_with_version(AccountStore::address_format());
    assert_eq!(address, keyring);
    assert!(keystore.can_sign(address.clone()).await);
    keystore.lock().await;
    assert!(!(keystore.can_sign(address.clone()).await));

    let keystore2 = EmbeddedKeystore::new(pool.clone());
    assert!(!(keystore2.can_sign(address.clone()).await));
    assert!(keystore2.unlock(None).await.is_ok());
    assert!(keystore2.can_sign(address.clone()).await);

    Ok(())
  }
  #[sqlx::test]
  async fn imports_and_derives(pool: SqlitePool) -> anyhow::Result<()> {
    let keystore = EmbeddedKeystore::new(pool.clone());
    let address = keystore
      .import("//Alice", CryptoScheme::Sr25519, None)
      .await?;
    let keyring = Sr25519Keyring::Alice
      .to_account_id()
      .to_ss58check_with_version(AccountStore::address_format());
    assert_eq!(address, keyring);

    let derived_1 = keystore.derive("//1").await?;
    assert_eq!(
      derived_1,
      sr25519::Pair::from_string("//Alice//1", None)?
        .public()
        .to_ss58check_with_version(AccountStore::address_format())
    );

    Ok(())
  }

  #[sqlx::test]
  async fn password_being_used(pool: SqlitePool) -> anyhow::Result<()> {
    let password = String::from("password");
    let keystore = EmbeddedKeystore::new(pool.clone());
    let address = keystore
      .import(
        "//Alice",
        CryptoScheme::Sr25519,
        Some(password.clone().into()),
      )
      .await?;

    let keystore2 = EmbeddedKeystore::new(pool.clone());
    assert!(keystore2.unlock(None).await.is_err());
    assert!(!(keystore2.can_sign(address.clone()).await));

    assert!(keystore2
      .unlock(Some(password.clone().into()))
      .await
      .is_ok());
    assert!(keystore2.can_sign(address).await);

    Ok(())
  }
}
