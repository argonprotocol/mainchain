//! Detached AccountSecurity approval signatures, independent of transaction account schemes.

use ::ecdsa::RecoveryId;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use p256::ecdsa::{Error as EcdsaError, Signature as EcdsaSignature, VerifyingKey};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ecdsa, ed25519, hashing::blake2_256, sr25519};
use sp_runtime::{
	traits::{Lazy, Verify},
	AccountId32, MultiSigner,
};

use crate::Signature;

/// Compressed SEC1 P-256 key. Its hash identifies an approval signer, not a transaction account.
#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Hash,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	TypeInfo,
	Serialize,
	Deserialize,
)]
pub struct P256Public(
	/// Canonical 33-byte compressed key; validate external keys with `from_sec1_bytes`.
	#[serde(with = "fixed_bytes")]
	pub [u8; 33],
);

/// P-256 ECDSA approval over SHA-256 of the canonical signing payload.
#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Hash,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	TypeInfo,
	Serialize,
	Deserialize,
)]
pub struct P256Signature(
	/// Fixed-width big-endian `r || s`, followed by a recovery ID in `0..=3`.
	#[serde(with = "fixed_bytes")]
	pub [u8; 65],
);

/// Proof schemes accepted only by AccountSecurity. Existing approval SCALE tags are preserved.
#[derive(
	Clone,
	Debug,
	Eq,
	PartialEq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	TypeInfo,
	Serialize,
	Deserialize,
)]
pub enum ApprovalSignature {
	/// Ordinary Ed25519 account proof over the signing payload.
	#[codec(index = 0)]
	Ed25519(ed25519::Signature),
	/// Ordinary Sr25519 account proof using Substrate's signing context.
	#[codec(index = 1)]
	Sr25519(sr25519::Signature),
	/// Ordinary secp256k1 account proof over Blake2-256 of the signing payload.
	#[codec(index = 2)]
	Ecdsa(ecdsa::Signature),
	/// Ordinary Ethereum-derived account proof over Keccak-256 of the signing payload.
	#[codec(index = 3)]
	Eth(ecdsa::KeccakSignature),
	/// Device proof; this variant is not accepted as an outer transaction signature.
	#[codec(index = 4)]
	P256(P256Signature),
}

impl Verify for ApprovalSignature {
	type Signer = MultiSigner;

	fn verify<L: Lazy<[u8]>>(&self, mut message: L, signer: &AccountId32) -> bool {
		match self {
			Self::Ed25519(signature) => Signature::Ed25519(*signature).verify(message, signer),
			Self::Sr25519(signature) => Signature::Sr25519(*signature).verify(message, signer),
			Self::Ecdsa(signature) => Signature::Ecdsa(*signature).verify(message, signer),
			Self::Eth(signature) => Signature::Eth(*signature).verify(message, signer),
			Self::P256(signature) => signature
				.recover(message.get())
				.is_ok_and(|public| &public.signer_id() == signer),
		}
	}
}

impl P256Public {
	/// Validate a compressed or uncompressed key and retain its canonical compressed encoding.
	pub fn from_sec1_bytes(bytes: &[u8]) -> Result<Self, EcdsaError> {
		let public = VerifyingKey::from_sec1_bytes(bytes)?;
		let compressed = public.to_encoded_point(true);
		let mut bytes = [0; 33];
		bytes.copy_from_slice(compressed.as_bytes());
		Ok(Self(bytes))
	}

	/// Identity stored in an AccountSecurity policy and matched against detached proofs.
	pub fn signer_id(&self) -> AccountId32 {
		blake2_256(self.as_ref()).into()
	}
}

impl P256Signature {
	/// Convert a device's fixed-width signature into the recoverable approval encoding.
	/// The device signs SHA-256 of `message`; decode DER to `r || s` before calling this.
	pub fn from_device_signature(
		message: &[u8],
		signature: &[u8; 64],
		public: &P256Public,
	) -> Result<Self, EcdsaError> {
		let signature = EcdsaSignature::from_slice(signature)?;
		let public = VerifyingKey::from_sec1_bytes(public.as_ref())?;
		let recovery_id = RecoveryId::trial_recovery_from_msg(&public, message, &signature)?;

		let mut bytes = [0; 65];
		bytes[..64].copy_from_slice(&signature.to_bytes());
		bytes[64] = recovery_id.to_byte();
		Ok(Self(bytes))
	}

	/// Recover the compressed public key after verifying ECDSA with SHA-256 of `message`.
	pub fn recover(&self, message: &[u8]) -> Result<P256Public, EcdsaError> {
		let signature = EcdsaSignature::from_slice(&self.0[..64])?;
		let recovery_id = RecoveryId::try_from(self.0[64])?;
		let public = VerifyingKey::recover_from_msg(message, &signature, recovery_id)?;
		let compressed = public.to_encoded_point(true);
		let mut bytes = [0; 33];
		bytes.copy_from_slice(compressed.as_bytes());
		Ok(P256Public(bytes))
	}
}

impl AsRef<[u8]> for P256Public {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl AsRef<[u8]> for P256Signature {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl From<Signature> for ApprovalSignature {
	fn from(signature: Signature) -> Self {
		match signature {
			Signature::Ed25519(signature) => Self::Ed25519(signature),
			Signature::Sr25519(signature) => Self::Sr25519(signature),
			Signature::Ecdsa(signature) => Self::Ecdsa(signature),
			Signature::Eth(signature) => Self::Eth(signature),
		}
	}
}

macro_rules! impl_from_signature {
	($variant:ident, $inner:ty) => {
		impl From<$inner> for ApprovalSignature {
			fn from(signature: $inner) -> Self {
				Self::$variant(signature)
			}
		}
	};
}

impl_from_signature!(Ed25519, ed25519::Signature);
impl_from_signature!(Sr25519, sr25519::Signature);
impl_from_signature!(Ecdsa, ecdsa::Signature);
impl_from_signature!(Eth, ecdsa::KeccakSignature);
impl_from_signature!(P256, P256Signature);

mod fixed_bytes {
	use polkadot_sdk::sp_core::bytes::{
		deserialize_check_len, serialize as serialize_bytes, ExpectedLen,
	};
	use serde::{Deserializer, Serializer};

	pub fn serialize<S: Serializer, const N: usize>(
		bytes: &[u8; N],
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		serialize_bytes(bytes, serializer)
	}

	pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
		deserializer: D,
	) -> Result<[u8; N], D::Error> {
		let mut bytes = [0; N];
		deserialize_check_len(deserializer, ExpectedLen::Exact(&mut bytes))?;
		Ok(bytes)
	}
}

#[cfg(test)]
mod tests {
	use super::{ApprovalSignature, P256Public, P256Signature};
	use crate::Signature;
	use codec::{Decode, Encode};
	use p256::ecdsa::{signature::Signer, SigningKey};
	use polkadot_sdk::{
		sp_core::{ecdsa, ed25519, sr25519, Pair},
		sp_runtime::{
			traits::{IdentifyAccount, Verify},
			MultiSigner,
		},
	};

	#[test]
	fn ordinary_account_proofs_keep_their_encoding_and_verification() {
		let message = b"account security approval";
		let ed25519 = ed25519::Pair::from_seed(&[1; 32]);
		let sr25519 = sr25519::Pair::from_seed(&[2; 32]);
		let ecdsa = ecdsa::Pair::from_seed(&[3; 32]);
		let ethereum = ecdsa::KeccakPair::from_seed(&[4; 32]);
		let proofs = [
			(Signature::from(ed25519.sign(message)), MultiSigner::from(ed25519.public())),
			(Signature::from(sr25519.sign(message)), MultiSigner::from(sr25519.public())),
			(Signature::from(ecdsa.sign(message)), MultiSigner::from(ecdsa.public())),
			(Signature::Eth(ethereum.sign(message)), MultiSigner::Eth(ethereum.public())),
		];
		for (signature, signer) in proofs {
			let proof = ApprovalSignature::from(signature.clone());
			assert_eq!(proof.encode(), signature.encode());
			assert_eq!(
				serde_json::to_value(&proof).unwrap(),
				serde_json::to_value(&signature).unwrap()
			);
			let account = signer.into_account();
			assert!(proof.verify(message.as_slice(), &account));
			assert!(!proof.verify(b"changed approval".as_slice(), &account));
			assert!(!proof.verify(message.as_slice(), &[0; 32].into()));
		}
	}

	#[test]
	fn p256_approvals_bind_the_payload_and_signer() {
		let message = b"P256 approval";
		let key = SigningKey::from_bytes((&[1; 32]).into()).unwrap();
		let public =
			P256Public::from_sec1_bytes(key.verifying_key().to_encoded_point(true).as_bytes())
				.unwrap();
		let signature: p256::ecdsa::Signature = key.sign(message);
		let proof =
			P256Signature::from_device_signature(message, &signature.to_bytes().into(), &public)
				.unwrap();
		assert_eq!(proof.recover(message).unwrap(), public);
		let proof = ApprovalSignature::P256(proof);
		assert!(proof.verify(message.as_slice(), &public.signer_id()));
		assert!(!proof.verify(b"changed approval".as_slice(), &public.signer_id()));
		assert!(!proof.verify(message.as_slice(), &[0; 32].into()));
		assert!(P256Signature::from_device_signature(
			b"changed approval",
			&signature.to_bytes().into(),
			&public
		)
		.is_err());

		let wrong_key = SigningKey::from_bytes((&[2; 32]).into()).unwrap();
		let wrong_public = P256Public::from_sec1_bytes(
			wrong_key.verifying_key().to_encoded_point(true).as_bytes(),
		)
		.unwrap();
		assert!(P256Signature::from_device_signature(
			message,
			&signature.to_bytes().into(),
			&wrong_public
		)
		.is_err());
	}

	#[test]
	fn p256_rejects_invalid_recovery_ids_and_signature_scalars() {
		let message = b"P256 approval";
		let key = SigningKey::from_bytes((&[1; 32]).into()).unwrap();
		let public =
			P256Public::from_sec1_bytes(key.verifying_key().to_encoded_point(true).as_bytes())
				.unwrap();
		let (signature, recovery_id) = key.sign_recoverable(message).unwrap();
		let mut bytes = [0; 65];
		bytes[..64].copy_from_slice(&signature.to_bytes());
		bytes[64] = recovery_id.to_byte();
		assert!(ApprovalSignature::P256(P256Signature(bytes))
			.verify(message.as_slice(), &public.signer_id()));
		for invalid_id in 4..=u8::MAX {
			bytes[64] = invalid_id;
			assert!(!ApprovalSignature::P256(P256Signature(bytes))
				.verify(message.as_slice(), &public.signer_id()));
		}
		assert!(!ApprovalSignature::P256(P256Signature([0; 65]))
			.verify(message.as_slice(), &public.signer_id()));
	}

	#[test]
	fn p256_encoding_is_approval_only_and_uses_the_compressed_key_identity() {
		let key = SigningKey::from_bytes((&[1; 32]).into()).unwrap();
		let compressed = key.verifying_key().to_encoded_point(true);
		let public = P256Public::from_sec1_bytes(compressed.as_bytes()).unwrap();
		assert_eq!(
			P256Public::from_sec1_bytes(key.verifying_key().to_encoded_point(false).as_bytes())
				.unwrap(),
			public
		);
		assert!(P256Public::from_sec1_bytes(&[0; 33]).is_err());
		assert_eq!(
			public.signer_id(),
			polkadot_sdk::sp_core::hashing::blake2_256(compressed.as_bytes()).into()
		);

		let proof = ApprovalSignature::P256(P256Signature([11; 65]));
		let encoded = proof.encode();
		assert_eq!(encoded[0], 4);
		assert_eq!(&encoded[1..], &[11; 65]);
		assert_eq!(ApprovalSignature::decode(&mut encoded.as_slice()).unwrap(), proof);
		assert_eq!(
			serde_json::from_value::<ApprovalSignature>(serde_json::to_value(&proof).unwrap())
				.unwrap(),
			proof
		);
		assert!(Signature::decode(&mut encoded.as_slice()).is_err());
	}
}
