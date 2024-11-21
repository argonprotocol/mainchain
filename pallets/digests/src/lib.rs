#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

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
	use argon_primitives::{
		digests::*,
		fork_power::ForkPower,
		tick::{Tick, TickDigest},
		VotingKey,
	};
	use codec::{Codec, EncodeLike};
	use frame_support::{pallet_prelude::*, traits::FindAuthor};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{ConsensusEngineId, Digest, DigestItem};

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
			let mut author = None;

			let mut block_vote = None;
			let mut tick = None;
			let mut notebooks = None;
			let mut parent_voting_key = None;
			let mut fork_power = None;

			for log in digest.logs() {
				match log {
					DigestItem::PreRuntime(BLOCK_VOTES_DIGEST_ID, v) => {
						ensure!(block_vote.is_none(), Error::<T>::DuplicateBlockVoteDigest);
						let digest = BlockVoteDigest::decode(&mut &v[..])
							.map_err(|_| Error::<T>::CouldNotDecodeDigest)?;
						block_vote = Some(digest);
					},
					DigestItem::PreRuntime(AUTHOR_DIGEST_ID, v) => {
						ensure!(author.is_none(), Error::<T>::DuplicateAuthorDigest);
						let digest = T::AccountId::decode(&mut &v[..])
							.map_err(|_| Error::<T>::CouldNotDecodeDigest)?;
						author = Some(digest);
					},
					DigestItem::PreRuntime(TICK_DIGEST_ID, v) => {
						ensure!(tick.is_none(), Error::<T>::DuplicateTickDigest);
						let digest = TickDigest::decode(&mut &v[..])
							.map_err(|_| Error::<T>::CouldNotDecodeDigest)?;
						tick = Some(digest);
					},
					DigestItem::Consensus(PARENT_VOTING_KEY_DIGEST, v) => {
						ensure!(
							parent_voting_key.is_none(),
							Error::<T>::DuplicateParentVotingKeyDigest
						);
						let digest = ParentVotingKeyDigest::decode(&mut &v[..])
							.map_err(|_| Error::<T>::CouldNotDecodeDigest)?;
						parent_voting_key = Some(digest);
					},
					DigestItem::Consensus(FORK_POWER_DIGEST, v) => {
						ensure!(fork_power.is_none(), Error::<T>::DuplicateForkPowerDigest);
						let digest = ForkPower::decode(&mut &v[..])
							.map_err(|_| Error::<T>::CouldNotDecodeDigest)?;
						fork_power = Some(digest);
					},
					DigestItem::PreRuntime(NOTEBOOKS_DIGEST_ID, v) => {
						ensure!(notebooks.is_none(), Error::<T>::DuplicateNotebookDigest);
						let digest = NotebookDigest::<T::NotebookVerifyError>::decode(&mut &v[..])
							.map_err(|_| Error::<T>::CouldNotDecodeDigest)?;
						notebooks = Some(digest);
					},
					_ => {},
				}
			}

			Ok(Digestset {
				block_vote: block_vote.ok_or(Error::<T>::MissingBlockVoteDigest)?,
				tick: tick.ok_or(Error::<T>::MissingTickDigest)?,
				author: author.ok_or(Error::<T>::MissingAuthorDigest)?,
				notebooks: notebooks.ok_or(Error::<T>::MissingNotebookDigest)?,
				voting_key: parent_voting_key,
				fork_power,
			})
		}

		pub fn read_digest() -> Result<Digestset<T::NotebookVerifyError, T::AccountId>, Error<T>> {
			if let Some(entry) = TempDigests::<T>::get() {
				return Ok(entry);
			}
			let digest = frame_system::Pallet::<T>::digest();
			let digests = Self::decode(&digest)?;
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
}
