use crate::UlxConfig;
use sp_core::{ed25519, sr25519, Pair};
use subxt::{tx::Signer, Config};

pub struct Ed25519Signer {
	pub keypair: ed25519::Pair,
}

impl Ed25519Signer {
	pub fn new(keypair: ed25519::Pair) -> Self {
		Self { keypair }
	}
}

impl Signer<UlxConfig> for Ed25519Signer {
	fn account_id(&self) -> <UlxConfig as Config>::AccountId {
		<UlxConfig as Config>::AccountId::from(self.keypair.public().0)
	}

	fn address(&self) -> <UlxConfig as Config>::Address {
		self.account_id().into()
	}

	fn sign(&self, data: &[u8]) -> <UlxConfig as Config>::Signature {
		<UlxConfig as Config>::Signature::Ed25519(self.keypair.sign(data).0)
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
impl Signer<UlxConfig> for Sr25519Signer {
	fn account_id(&self) -> <UlxConfig as Config>::AccountId {
		<UlxConfig as Config>::AccountId::from(self.keypair.public().0)
	}

	fn address(&self) -> <UlxConfig as Config>::Address {
		self.account_id().into()
	}

	fn sign(&self, data: &[u8]) -> <UlxConfig as Config>::Signature {
		<UlxConfig as Config>::Signature::Sr25519(self.keypair.sign(data).0)
	}
}
