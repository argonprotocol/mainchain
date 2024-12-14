#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use argon_primitives::DecodeDigestError;
use codec::Decode;
pub use pallet::*;
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

/// This pallet decodes system digests and temporarily provides them to the runtime.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use argon_primitives::{digests::*, tick::Tick, VotingKey};
	use codec::{Codec, EncodeLike};
	use frame_support::{pallet_prelude::*, traits::FindAuthor};
	use frame_system::pallet_prelude::*;
	use log::warn;
	use sp_runtime::{ConsensusEngineId, Digest};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type NotebookVerifyError: Codec + EncodeLike + Clone + TypeInfo;
	}

	#[pallet::storage]
	pub(super) type TempDigests<T: Config> =
		StorageValue<_, Digestset<T::NotebookVerifyError, T::AccountId>, OptionQuery>;

	#[pallet::event]
	pub enum Event<T> {}

	#[pallet::error]
	pub enum Error<T> {
		/// Duplicate BlockVoteDigest found
		DuplicateBlockVoteDigest,
		/// Duplicate AuthorDigest found
		DuplicateAuthorDigest,
		/// Duplicate TickDigest found
		DuplicateTickDigest,
		/// Duplicate ParentVotingKeyDigest found
		DuplicateParentVotingKeyDigest,
		/// Duplicate NotebookDigest found
		DuplicateNotebookDigest,
		/// Duplicate ForkPowerDigest found
		DuplicateForkPowerDigest,
		/// Missing BlockVoteDigest
		MissingBlockVoteDigest,
		/// Missing AuthorDigest
		MissingAuthorDigest,
		/// Missing TickDigest
		MissingTickDigest,
		/// Missing ParentVotingKeyDigest
		MissingParentVotingKeyDigest,
		/// Missing NotebookDigest
		MissingNotebookDigest,
		/// Failed to decode digests
		CouldNotDecodeDigest,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			match Self::read_digest() {
				Ok(digests) => {
					TempDigests::<T>::put(digests);
				},
				Err(e) => panic!("Could not load digests: {:?}", e),
			}

			T::DbWeight::get().reads_writes(1, 0)
		}

		fn on_finalize(_n: BlockNumberFor<T>) {
			<TempDigests<T>>::kill();
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn decode(
			digest: &Digest,
		) -> Result<Digestset<T::NotebookVerifyError, T::AccountId>, Error<T>> {
			Digestset::<T::NotebookVerifyError, T::AccountId>::try_from(digest.clone())
				.map_err(Into::into)
		}

		pub fn read_digest() -> Result<Digestset<T::NotebookVerifyError, T::AccountId>, Error<T>> {
			if let Some(entry) = TempDigests::<T>::get() {
				return Ok(entry);
			}
			let digest = frame_system::Pallet::<T>::digest();
			let digests = Self::decode(&digest).inspect_err(|e| {
				warn!("Could not decode digests: {:?}", e);
			})?;
			Ok(digests)
		}

		pub fn decode_voting_author(
			digest: &Digest,
		) -> Result<(T::AccountId, Tick, Option<VotingKey>), DispatchError> {
			let decoded = Self::decode(digest).map_err(|e| {
				log::error!("Could not load digests: {:?}", e);
				Error::<T>::CouldNotDecodeDigest
			})?;
			let mut parent_voting_key = None;
			if let Some(voting_key) = decoded.voting_key {
				parent_voting_key = voting_key.parent_voting_key;
			}

			Ok((decoded.author, decoded.tick.0, parent_voting_key))
		}
	}

	impl<T: Config> Get<Result<Digestset<T::NotebookVerifyError, T::AccountId>, DispatchError>>
		for Pallet<T>
	{
		fn get() -> Result<Digestset<T::NotebookVerifyError, T::AccountId>, DispatchError> {
			// this can get called before on_initialize, so we need to make sure we have the digests
			Self::read_digest().map_err(|e| {
				log::error!("Could not load digests: {:?}", e);
				Error::<T>::CouldNotDecodeDigest.into()
			})
		}
	}

	impl<T: Config> FindAuthor<T::AccountId> for Pallet<T> {
		fn find_author<'a, I>(digests: I) -> Option<T::AccountId>
		where
			I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
		{
			// if this is called after initialize, we're fine, but it might not be
			if let Some(digests) = <TempDigests<T>>::get() {
				return Some(digests.author);
			}

			for (id, data) in digests {
				if id == AUTHOR_DIGEST_ID {
					if let Ok(author) = T::AccountId::decode(&mut &data[..]) {
						return Some(author);
					}
				}
			}

			None
		}
	}

	impl<T: Config> From<DecodeDigestError> for Error<T> {
		fn from(value: DecodeDigestError) -> Self {
			match value {
				DecodeDigestError::DuplicateBlockVoteDigest => Error::<T>::DuplicateBlockVoteDigest,
				DecodeDigestError::DuplicateAuthorDigest => Error::<T>::DuplicateAuthorDigest,
				DecodeDigestError::DuplicateTickDigest => Error::<T>::DuplicateTickDigest,
				DecodeDigestError::DuplicateParentVotingKeyDigest =>
					Error::<T>::DuplicateParentVotingKeyDigest,
				DecodeDigestError::DuplicateNotebookDigest => Error::<T>::DuplicateNotebookDigest,
				DecodeDigestError::DuplicateForkPowerDigest => Error::<T>::DuplicateForkPowerDigest,
				DecodeDigestError::MissingBlockVoteDigest => Error::<T>::MissingBlockVoteDigest,
				DecodeDigestError::MissingAuthorDigest => Error::<T>::MissingAuthorDigest,
				DecodeDigestError::MissingTickDigest => Error::<T>::MissingTickDigest,
				DecodeDigestError::MissingParentVotingKeyDigest =>
					Error::<T>::MissingParentVotingKeyDigest,
				DecodeDigestError::MissingNotebookDigest => Error::<T>::MissingNotebookDigest,
				DecodeDigestError::CouldNotDecodeDigest => Error::<T>::CouldNotDecodeDigest,
			}
		}
	}
}
