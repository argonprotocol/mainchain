use crate::ArgonConfig;
use argon_primitives::{AccountId, CryptoType};
use sp_core::{crypto::key_types::ACCOUNT, ed25519, sr25519, Pair};
use sp_keystore::Keystore;
pub use subxt::tx::Signer;
use subxt::Config;

pub struct Ed25519Signer {
	pub keypair: ed25519::Pair,
}

impl Ed25519Signer {
	pub fn new(keypair: ed25519::Pair) -> Self {
		Self { keypair }
	}
}

impl Signer<ArgonConfig> for Ed25519Signer {
	fn account_id(&self) -> <ArgonConfig as Config>::AccountId {
		<ArgonConfig as Config>::AccountId::from(self.keypair.public().0)
	}

	fn sign(&self, data: &[u8]) -> <ArgonConfig as Config>::Signature {
		self.keypair.sign(data).into()
	}
}
impl From<ed25519::Pair> for Ed25519Signer {
	fn from(keypair: ed25519::Pair) -> Self {
		Self::new(keypair)
	}
}

pub struct Sr25519Signer {
	pub keypair: sr25519::Pair,
}
impl Sr25519Signer {
	pub fn new(keypair: sr25519::Pair) -> Self {
		Self { keypair }
	}
}
impl Signer<ArgonConfig> for Sr25519Signer {
	fn account_id(&self) -> <ArgonConfig as Config>::AccountId {
		<ArgonConfig as Config>::AccountId::from(self.keypair.public().0)
	}

	fn sign(&self, data: &[u8]) -> <ArgonConfig as Config>::Signature {
		self.keypair.sign(data).into()
	}
}
impl From<sr25519::Pair> for Sr25519Signer {
	fn from(keypair: sr25519::Pair) -> Self {
		Self::new(keypair)
	}
}

pub struct KeystoreSigner {
	pub keystore: sp_keystore::KeystorePtr,
	pub account_id: AccountId,
	pub crypto_type: CryptoType,
}

impl KeystoreSigner {
	pub fn new(
		keystore: sp_keystore::KeystorePtr,
		account_id: AccountId,
		crypto_type: CryptoType,
	) -> Self {
		Self { keystore, account_id, crypto_type }
	}
}
impl Signer<ArgonConfig> for KeystoreSigner {
	fn account_id(&self) -> <ArgonConfig as Config>::AccountId {
		self.account_id.clone()
	}

	fn sign(&self, data: &[u8]) -> <ArgonConfig as Config>::Signature {
		let account_id: [u8; 32] = self.account_id.clone().into();
		match self.crypto_type {
			CryptoType::Sr25519 => self
				.keystore
				.sr25519_sign(ACCOUNT, &account_id.into(), data)
				.expect("Failed to sign with sr25519")
				.expect("Failed to create signature")
				.into(),
			CryptoType::Ed25519 => self
				.keystore
				.ed25519_sign(ACCOUNT, &account_id.into(), data)
				.expect("Failed to sign with ed25519")
				.expect("Failed to create signature")
				.into(),
		}
	}
}
