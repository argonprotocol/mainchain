#![cfg_attr(not(feature = "std"), no_std)]
//! Chain-backed storage for encrypted recovery and endpoint payloads.
//!
//! Payload bytes are encrypted by contract. This pallet enforces authorization, ownership, and
//! length limits, but does not parse the payloads or verify their encryption.

extern crate alloc;

pub use pallet::*;
use pallet_prelude::*;
pub use weights::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use polkadot_sdk::sp_core::ed25519;
	use sp_runtime::traits::Verify;

	/// Domain separator for recovery payload authorization proofs.
	pub const RECOVERY_PROOF_MESSAGE_KEY: &[u8] = b"bootstrap_recovery";

	/// Bootstrap payload storage and authorization.
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// Maximum number of bytes in an encrypted recovery or endpoint payload.
		#[pallet::constant]
		type MaxEncryptedPayloadLen: Get<u32>;

		/// Weight information for this pallet's calls.
		type WeightInfo: WeightInfo;
	}

	/// Public lookup key identifying an encrypted endpoint payload.
	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		Debug,
		MaxEncodedLen,
		Default,
	)]
	pub struct EndpointPubkey(pub [u8; 32]);

	/// Context-specific Ed25519 public key identifying and authorizing a recovery payload.
	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		Debug,
		MaxEncodedLen,
		Default,
	)]
	pub struct RecoveryPubkey(pub [u8; 32]);

	/// Proof that the recovery key authorizes a writer to submit an encrypted recovery payload.
	#[derive(
		Decode, DecodeWithMemTracking, Encode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	pub struct RecoveryProof {
		/// Signature over the writer, recovery public key, and encrypted payload hash.
		pub signature: ed25519::Signature,
	}

	impl RecoveryProof {
		pub fn verify<AccountId: Encode>(
			&self,
			writer: &AccountId,
			recovery_pubkey: &RecoveryPubkey,
			encrypted_recovery_payload: &[u8],
		) -> bool {
			let message = (
				RECOVERY_PROOF_MESSAGE_KEY,
				writer,
				recovery_pubkey,
				blake2_256(encrypted_recovery_payload),
			)
				.using_encoded(blake2_256);
			self.signature
				.verify(message.as_slice(), &ed25519::Public::from_raw(recovery_pubkey.0))
		}
	}

	#[pallet::storage]
	/// Encrypted recovery payload authorized by the corresponding recovery key.
	///
	/// The pallet enforces authorization and length but does not parse or verify encryption.
	pub type EncryptedRecoveryPayloadByPubkey<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		RecoveryPubkey,
		BoundedVec<u8, T::MaxEncryptedPayloadLen>,
		OptionQuery,
	>;

	#[pallet::storage]
	/// Encrypted endpoint payload keyed by its public key.
	///
	/// The pallet enforces ownership and length but does not parse or verify encryption.
	pub type EncryptedEndpointByPubkey<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		EndpointPubkey,
		BoundedVec<u8, T::MaxEncryptedPayloadLen>,
		OptionQuery,
	>;

	#[pallet::storage]
	/// Account authorized to update an encrypted endpoint payload.
	///
	/// The first successful [`Pallet::set_endpoint`] call for a public key establishes its owner.
	pub type EndpointOwnerByPubkey<T: Config> =
		StorageMap<_, Blake2_128Concat, EndpointPubkey, T::AccountId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An encrypted recovery payload was stored or replaced.
		RecoveryPayloadUpdated { writer: T::AccountId, recovery_pubkey: RecoveryPubkey },

		/// An encrypted endpoint payload was stored or replaced by its owner.
		EndpointUpdated { endpoint_owner: T::AccountId, endpoint_pubkey: EndpointPubkey },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The encrypted payload exceeds [`Config::MaxEncryptedPayloadLen`].
		EncryptedPayloadTooLong,

		/// The recovery proof does not authorize this writer, public key, and payload.
		InvalidRecoveryProof,

		/// The endpoint public key is already owned by a different account.
		EndpointOwnedByAnotherAccount,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Store or replace an encrypted recovery payload.
		///
		/// The proof must be signed by `recovery_pubkey` and binds the submitting `writer`,
		/// `recovery_pubkey`, and the encrypted payload hash.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_recovery_payload())]
		pub fn set_recovery_payload(
			origin: OriginFor<T>,
			recovery_pubkey: RecoveryPubkey,
			recovery_proof: RecoveryProof,
			encrypted_recovery_payload: Vec<u8>,
		) -> DispatchResult {
			let writer = ensure_signed(origin)?;
			let encrypted_recovery_payload: BoundedVec<u8, T::MaxEncryptedPayloadLen> =
				encrypted_recovery_payload
					.try_into()
					.map_err(|_| Error::<T>::EncryptedPayloadTooLong)?;
			ensure!(
				recovery_proof.verify(&writer, &recovery_pubkey, &encrypted_recovery_payload),
				Error::<T>::InvalidRecoveryProof
			);

			EncryptedRecoveryPayloadByPubkey::<T>::insert(
				&recovery_pubkey,
				encrypted_recovery_payload,
			);
			Self::deposit_event(Event::RecoveryPayloadUpdated { writer, recovery_pubkey });
			Ok(())
		}

		/// Store or replace an encrypted endpoint payload.
		///
		/// The first writer for `endpoint_pubkey` becomes its owner. Later updates require the
		/// same signing account.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_endpoint())]
		pub fn set_endpoint(
			origin: OriginFor<T>,
			endpoint_pubkey: EndpointPubkey,
			encrypted_endpoint: Vec<u8>,
		) -> DispatchResult {
			let endpoint_owner = ensure_signed(origin)?;
			let encrypted_endpoint: BoundedVec<u8, T::MaxEncryptedPayloadLen> =
				encrypted_endpoint.try_into().map_err(|_| Error::<T>::EncryptedPayloadTooLong)?;

			EndpointOwnerByPubkey::<T>::try_mutate(
				&endpoint_pubkey,
				|maybe_owner| -> DispatchResult {
					if let Some(owner) = maybe_owner.as_ref() {
						ensure!(
							owner == &endpoint_owner,
							Error::<T>::EndpointOwnedByAnotherAccount
						);
					} else {
						*maybe_owner = Some(endpoint_owner.clone());
					}
					Ok(())
				},
			)?;
			EncryptedEndpointByPubkey::<T>::insert(&endpoint_pubkey, encrypted_endpoint);
			Self::deposit_event(Event::EndpointUpdated { endpoint_owner, endpoint_pubkey });
			Ok(())
		}
	}
}
