use crate::ArgonConfig;
use argon_primitives::{AccountId, CryptoType};
use sp_core::{
	crypto::{key_types::ACCOUNT, AccountId32},
	ed25519, sr25519, Pair,
};
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

	fn address(&self) -> <ArgonConfig as Config>::Address {
		self.account_id().into()
	}

	fn sign(&self, data: &[u8]) -> <ArgonConfig as Config>::Signature {
		<ArgonConfig as Config>::Signature::Ed25519(self.keypair.sign(data).0)
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

	fn address(&self) -> <ArgonConfig as Config>::Address {
		self.account_id().into()
	}

	fn sign(&self, data: &[u8]) -> <ArgonConfig as Config>::Signature {
		<ArgonConfig as Config>::Signature::Sr25519(self.keypair.sign(data).0)
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
		let account_id32: AccountId32 = self.account_id.clone().into();
		let account_id = subxt::utils::AccountId32(account_id32.into());
		account_id
	}

	fn address(&self) -> <ArgonConfig as Config>::Address {
		<ArgonConfig as Config>::Address::Id(self.account_id())
	}

	fn sign(&self, data: &[u8]) -> <ArgonConfig as Config>::Signature {
		let account_id = self.account_id().0;
		match self.crypto_type {
			CryptoType::Sr25519 => {
				let signature = self
					.keystore
					.sr25519_sign(ACCOUNT, &account_id.into(), data)
					.expect("Failed to sign with sr25519")
					.expect("Failed to create signature");

				<ArgonConfig as Config>::Signature::Sr25519(signature.0)
			},
			CryptoType::Ed25519 => {
				let signature = self
					.keystore
					.ed25519_sign(ACCOUNT, &account_id.into(), data)
					.expect("Failed to sign with ed25519")
					.expect("Failed to create signature");

				<ArgonConfig as Config>::Signature::Ed25519(signature.0)
			},
		}
	}
}
