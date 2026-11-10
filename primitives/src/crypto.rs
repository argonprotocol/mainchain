//! Native transaction signatures and their account identities.

use ::ecdsa::RecoveryId;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use p256::ecdsa::{Error as EcdsaError, Signature as EcdsaSignature, VerifyingKey};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use sp_core::hexdisplay::HexDisplay;
use sp_core::{
	crypto::{FromEntropy, UncheckedFrom},
	ecdsa, ed25519,
	hashing::blake2_256,
	sr25519, H256,
};
use sp_runtime::{
	traits::{IdentifyAccount, Lazy, Verify},
	AccountId32, MultiSignature as SdkSignature, MultiSigner as SdkSigner,
};

/// A compressed SEC1 P-256 public key. Its Blake2-256 hash identifies the native account.
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
	/// The 33-byte compressed SEC1 encoding; use [`P256Public::from_sec1_bytes`] for external
	/// keys.
	#[serde(with = "fixed_bytes")]
	pub [u8; 33],
);

/// A P-256 ECDSA signature over SHA-256 of the transaction signing payload.
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

/// Native transaction signatures. Existing SCALE indices are part of the chain's wire format.
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
pub enum Signature {
	/// An Ed25519 signature over the signing payload.
	#[codec(index = 0)]
	Ed25519(ed25519::Signature),
	/// An Sr25519 signature using Substrate's signing context.
	#[codec(index = 1)]
	Sr25519(sr25519::Signature),
	/// A secp256k1 signature over Blake2-256 of the signing payload.
	#[codec(index = 2)]
	Ecdsa(ecdsa::Signature),
	/// A secp256k1 signature over Keccak-256 using the SDK's Ethereum account derivation.
	#[codec(index = 3)]
	Eth(ecdsa::KeccakSignature),
	/// A P-256 signature over SHA-256 of the bytes supplied by the transaction signing API.
	#[codec(index = 4)]
	P256(P256Signature),
}

/// Public signing identities. P-256 adds an account scheme without changing existing accounts.
#[derive(
	Clone,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	TypeInfo,
	Serialize,
	Deserialize,
)]
pub enum MultiSigner {
	/// An Ed25519 public key, used directly as the 32-byte account ID.
	#[codec(index = 0)]
	Ed25519(ed25519::Public),
	/// An Sr25519 public key, used directly as the 32-byte account ID.
	#[codec(index = 1)]
	Sr25519(sr25519::Public),
	/// A compressed secp256k1 key whose Blake2-256 hash identifies the account.
	#[codec(index = 2)]
	Ecdsa(ecdsa::Public),
	/// A secp256k1 key using the SDK's Ethereum address-to-account mapping.
	#[codec(index = 3)]
	Eth(ecdsa::KeccakPublic),
	/// A compressed P-256 key whose Blake2-256 hash identifies the account.
	#[codec(index = 4)]
	P256(P256Public),
}

impl Verify for Signature {
	type Signer = MultiSigner;

	fn verify<L: Lazy<[u8]>>(&self, mut message: L, signer: &AccountId32) -> bool {
		match self {
			Self::Ed25519(signature) => SdkSignature::Ed25519(*signature).verify(message, signer),
			Self::Sr25519(signature) => SdkSignature::Sr25519(*signature).verify(message, signer),
			Self::Ecdsa(signature) => SdkSignature::Ecdsa(*signature).verify(message, signer),
			Self::Eth(signature) => SdkSignature::Eth(*signature).verify(message, signer),
			Self::P256(signature) => signature
				.recover(message.get())
				.is_ok_and(|public| &MultiSigner::P256(public).into_account() == signer),
		}
	}
}

impl IdentifyAccount for MultiSigner {
	type AccountId = AccountId32;

	fn into_account(self) -> AccountId32 {
		match self {
			Self::Ed25519(public) => SdkSigner::Ed25519(public).into_account(),
			Self::Sr25519(public) => SdkSigner::Sr25519(public).into_account(),
			Self::Ecdsa(public) => SdkSigner::Ecdsa(public).into_account(),
			Self::Eth(public) => SdkSigner::Eth(public).into_account(),
			Self::P256(public) => blake2_256(public.as_ref()).into(),
		}
	}
}

impl P256Public {
	/// Validate a compressed or uncompressed SEC1 key and retain its canonical compressed encoding.
	pub fn from_sec1_bytes(bytes: &[u8]) -> Result<Self, EcdsaError> {
		let public = VerifyingKey::from_sec1_bytes(bytes)?;
		let compressed = public.to_encoded_point(true);
		let mut bytes = [0; 33];
		bytes.copy_from_slice(compressed.as_bytes());
		Ok(Self(bytes))
	}
}

impl P256Signature {
	/// Convert a device's fixed-width `r || s` signature into the native recoverable encoding.
	///
	/// The device must sign SHA-256 of `message`. DER signatures must first be decoded into
	/// their 64-byte fixed-width representation. A mismatched key or message is rejected.
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

impl AsRef<[u8]> for MultiSigner {
	fn as_ref(&self) -> &[u8] {
		match self {
			Self::Ed25519(public) => public.as_ref(),
			Self::Sr25519(public) => public.as_ref(),
			Self::Ecdsa(public) => public.as_ref(),
			Self::Eth(public) => public.as_ref(),
			Self::P256(public) => public.as_ref(),
		}
	}
}

impl From<SdkSignature> for Signature {
	fn from(signature: SdkSignature) -> Self {
		match signature {
			SdkSignature::Ed25519(signature) => Self::Ed25519(signature),
			SdkSignature::Sr25519(signature) => Self::Sr25519(signature),
			SdkSignature::Ecdsa(signature) => Self::Ecdsa(signature),
			SdkSignature::Eth(signature) => Self::Eth(signature),
		}
	}
}

impl TryFrom<Signature> for SdkSignature {
	type Error = ();

	fn try_from(signature: Signature) -> Result<Self, Self::Error> {
		match signature {
			Signature::Ed25519(signature) => Ok(Self::Ed25519(signature)),
			Signature::Sr25519(signature) => Ok(Self::Sr25519(signature)),
			Signature::Ecdsa(signature) => Ok(Self::Ecdsa(signature)),
			Signature::Eth(signature) => Ok(Self::Eth(signature)),
			Signature::P256(_) => Err(()),
		}
	}
}

impl From<SdkSigner> for MultiSigner {
	fn from(signer: SdkSigner) -> Self {
		match signer {
			SdkSigner::Ed25519(public) => Self::Ed25519(public),
			SdkSigner::Sr25519(public) => Self::Sr25519(public),
			SdkSigner::Ecdsa(public) => Self::Ecdsa(public),
			SdkSigner::Eth(public) => Self::Eth(public),
		}
	}
}

impl TryFrom<MultiSigner> for SdkSigner {
	type Error = ();

	fn try_from(signer: MultiSigner) -> Result<Self, Self::Error> {
		match signer {
			MultiSigner::Ed25519(public) => Ok(Self::Ed25519(public)),
			MultiSigner::Sr25519(public) => Ok(Self::Sr25519(public)),
			MultiSigner::Ecdsa(public) => Ok(Self::Ecdsa(public)),
			MultiSigner::Eth(public) => Ok(Self::Eth(public)),
			MultiSigner::P256(_) => Err(()),
		}
	}
}

impl<T: Into<H256>> UncheckedFrom<T> for MultiSigner {
	fn unchecked_from(value: T) -> Self {
		ed25519::Public::unchecked_from(value.into()).into()
	}
}

impl FromEntropy for MultiSigner {
	fn from_entropy(input: &mut impl codec::Input) -> Result<Self, codec::Error> {
		Ok(match input.read_byte()? % 5 {
			0 => Self::Ed25519(FromEntropy::from_entropy(input)?),
			1 => Self::Sr25519(FromEntropy::from_entropy(input)?),
			2 => Self::Ecdsa(FromEntropy::from_entropy(input)?),
			3 => Self::Eth(FromEntropy::from_entropy(input)?),
			_ => {
				let mut bytes = [0; 33];
				input.read(&mut bytes)?;
				Self::P256(P256Public(bytes))
			},
		})
	}
}

#[cfg(feature = "std")]
impl core::fmt::Display for MultiSigner {
	fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
		match self {
			Self::Ed25519(public) => write!(formatter, "ed25519: {public}"),
			Self::Sr25519(public) => write!(formatter, "sr25519: {public}"),
			Self::Ecdsa(public) => write!(formatter, "ecdsa: {public}"),
			Self::Eth(public) => write!(formatter, "eth: {public}"),
			Self::P256(public) => write!(formatter, "p256: {}", HexDisplay::from(&public.as_ref())),
		}
	}
}

macro_rules! impl_variant_conversion {
	($enum:ident, $variant:ident, $inner:ty) => {
		impl From<$inner> for $enum {
			fn from(value: $inner) -> Self {
				Self::$variant(value)
			}
		}

		impl TryFrom<$enum> for $inner {
			type Error = ();

			fn try_from(value: $enum) -> Result<Self, Self::Error> {
				if let $enum::$variant(value) = value {
					Ok(value)
				} else {
					Err(())
				}
			}
		}
	};
}

impl_variant_conversion!(Signature, Ed25519, ed25519::Signature);
impl_variant_conversion!(Signature, Sr25519, sr25519::Signature);
impl_variant_conversion!(Signature, Ecdsa, ecdsa::Signature);
impl_variant_conversion!(Signature, Eth, ecdsa::KeccakSignature);
impl_variant_conversion!(Signature, P256, P256Signature);
impl_variant_conversion!(MultiSigner, Ed25519, ed25519::Public);
impl_variant_conversion!(MultiSigner, Sr25519, sr25519::Public);
impl_variant_conversion!(MultiSigner, Ecdsa, ecdsa::Public);
impl_variant_conversion!(MultiSigner, Eth, ecdsa::KeccakPublic);
impl_variant_conversion!(MultiSigner, P256, P256Public);

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
	use super::{MultiSigner, P256Public, P256Signature};
	use crate::Signature;
	use codec::{Decode, Encode};
	use p256::ecdsa::{signature::Signer, SigningKey};
	use polkadot_sdk::{
		sp_core::{ecdsa, ed25519, sr25519, Pair},
		sp_runtime::{
			traits::{IdentifyAccount, Verify},
			MultiSignature as SdkSignature, MultiSigner as SdkSigner,
		},
	};

	#[test]
	fn existing_signature_and_signer_encodings_and_accounts_are_unchanged() {
		let signatures = [
			SdkSignature::Ed25519([7; 64].into()),
			SdkSignature::Sr25519([8; 64].into()),
			SdkSignature::Ecdsa([9; 65].into()),
			SdkSignature::Eth([10; 65].into()),
		];
		let signers = [
			SdkSigner::Ed25519([7; 32].into()),
			SdkSigner::Sr25519([8; 32].into()),
			SdkSigner::Ecdsa([9; 33].into()),
			SdkSigner::Eth([10; 33].into()),
		];

		for (index, (sdk_signature, sdk_signer)) in signatures.into_iter().zip(signers).enumerate()
		{
			let signature = Signature::from(sdk_signature.clone());
			let signer = MultiSigner::from(sdk_signer.clone());
			let encoded_signature = sdk_signature.encode();
			let encoded_signer = sdk_signer.encode();
			assert_eq!(encoded_signature[0], index as u8);
			assert_eq!(encoded_signer[0], index as u8);
			assert_eq!(signature.encode(), encoded_signature);
			assert_eq!(signer.encode(), encoded_signer);
			assert_eq!(Signature::decode(&mut &encoded_signature[..]).unwrap(), signature);
			assert_eq!(MultiSigner::decode(&mut &encoded_signer[..]).unwrap(), signer);
			assert_eq!(signer.clone().into_account(), sdk_signer.clone().into_account());
			assert_eq!(SdkSignature::try_from(signature.clone()), Ok(sdk_signature.clone()));
			assert_eq!(SdkSigner::try_from(signer.clone()), Ok(sdk_signer.clone()));
			assert_eq!(
				serde_json::to_value(&signature).unwrap(),
				serde_json::to_value(&sdk_signature).unwrap()
			);
			assert_eq!(
				serde_json::to_value(&signer).unwrap(),
				serde_json::to_value(&sdk_signer).unwrap()
			);
		}
	}

	#[test]
	fn existing_schemes_still_verify_through_native_signature() {
		let message = b"native transaction signing";
		let ed25519 = ed25519::Pair::from_seed(&[1; 32]);
		let sr25519 = sr25519::Pair::from_seed(&[2; 32]);
		let ecdsa = ecdsa::Pair::from_seed(&[3; 32]);
		let ethereum = ecdsa::KeccakPair::from_seed(&[4; 32]);
		let signatures = [
			(Signature::from(ed25519.sign(message)), MultiSigner::from(ed25519.public())),
			(Signature::from(sr25519.sign(message)), MultiSigner::from(sr25519.public())),
			(Signature::from(ecdsa.sign(message)), MultiSigner::from(ecdsa.public())),
			(Signature::from(ethereum.sign(message)), MultiSigner::from(ethereum.public())),
		];

		for (signature, signer) in signatures {
			let account = signer.into_account();
			assert!(signature.verify(&message[..], &account));
			assert!(!signature.verify(&b"changed transaction"[..], &account));
			assert!(!signature.verify(&message[..], &[0; 32].into()));
		}
	}

	#[test]
	fn p256_verifies_only_the_signed_payload_and_account() {
		let message = b"native P256 transaction";
		let signing_key = SigningKey::from_bytes((&[1; 32]).into()).unwrap();
		let public = P256Public::from_sec1_bytes(
			signing_key.verifying_key().to_encoded_point(true).as_bytes(),
		)
		.unwrap();
		let device_signature: p256::ecdsa::Signature = signing_key.sign(message);
		let signature = P256Signature::from_device_signature(
			message,
			&device_signature.to_bytes().into(),
			&public,
		)
		.unwrap();
		assert_eq!(signature.recover(message).unwrap(), public);

		let account = MultiSigner::P256(public).into_account();
		let signature = Signature::P256(signature);

		assert!(signature.verify(&message[..], &account));
		assert!(!signature.verify(&b"tampered transaction"[..], &account));
		assert!(!signature.verify(&message[..], &[0; 32].into()));
		assert!(P256Signature::from_device_signature(
			b"tampered transaction",
			&device_signature.to_bytes().into(),
			&public,
		)
		.is_err());

		let wrong_key = SigningKey::from_bytes((&[2; 32]).into()).unwrap();
		let wrong_public = P256Public::from_sec1_bytes(
			wrong_key.verifying_key().to_encoded_point(true).as_bytes(),
		)
		.unwrap();
		assert!(P256Signature::from_device_signature(
			message,
			&device_signature.to_bytes().into(),
			&wrong_public,
		)
		.is_err());
	}

	#[test]
	fn p256_rejects_invalid_recovery_ids_and_signature_scalars() {
		let message = b"native P256 transaction";
		let signing_key = SigningKey::from_bytes((&[1; 32]).into()).unwrap();
		let public = P256Public::from_sec1_bytes(
			signing_key.verifying_key().to_encoded_point(true).as_bytes(),
		)
		.unwrap();
		let (signature, recovery_id) = signing_key.sign_recoverable(message).unwrap();
		let mut bytes = [0; 65];
		bytes[..64].copy_from_slice(&signature.to_bytes());
		bytes[64] = recovery_id.to_byte();
		let account = MultiSigner::P256(public).into_account();
		assert!(Signature::P256(P256Signature(bytes)).verify(&message[..], &account));

		for invalid_id in 4..=u8::MAX {
			bytes[64] = invalid_id;
			assert!(!Signature::P256(P256Signature(bytes)).verify(&message[..], &account));
		}
		assert!(!Signature::P256(P256Signature([0; 65])).verify(&message[..], &account));
	}

	#[test]
	fn p256_appends_tag_four_and_uses_the_compressed_key_account() {
		let signing_key = SigningKey::from_bytes((&[1; 32]).into()).unwrap();
		let compressed = signing_key.verifying_key().to_encoded_point(true);
		let uncompressed = signing_key.verifying_key().to_encoded_point(false);
		let public = P256Public::from_sec1_bytes(compressed.as_bytes()).unwrap();
		assert_eq!(P256Public::from_sec1_bytes(uncompressed.as_bytes()).unwrap(), public);
		assert!(P256Public::from_sec1_bytes(&[0; 33]).is_err());

		let signer = MultiSigner::P256(public);
		let account = polkadot_sdk::sp_core::hashing::blake2_256(compressed.as_bytes());
		assert_eq!(signer.clone().into_account(), account.into());
		let encoded = signer.encode();
		assert_eq!(encoded[0], 4);
		assert_eq!(&encoded[1..], compressed.as_bytes());
		assert_eq!(MultiSigner::decode(&mut &encoded[..]).unwrap(), signer);
		let decoded: MultiSigner =
			serde_json::from_value(serde_json::to_value(&signer).unwrap()).unwrap();
		assert_eq!(signer, decoded);

		let signature = Signature::P256(P256Signature([11; 65]));
		let encoded = signature.encode();
		assert_eq!(encoded[0], 4);
		assert_eq!(&encoded[1..], &[11; 65]);
		assert_eq!(Signature::decode(&mut &encoded[..]).unwrap(), signature);
		let decoded: Signature =
			serde_json::from_value(serde_json::to_value(&signature).unwrap()).unwrap();
		assert_eq!(signature, decoded);
		assert!(SdkSignature::try_from(signature).is_err());
		assert!(SdkSigner::try_from(signer).is_err());
	}
}
