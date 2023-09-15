#![feature(slice_take)]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::traits::OneSessionHandler;
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_session::SessionManager;
use sp_core::{Get, U256};
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{Convert, IsMember, UniqueSaturatedInto},
	BoundedBTreeMap,
};
use sp_std::{collections::btree_map::BTreeMap, marker::PhantomData, vec::Vec};

pub use pallet::*;
use ulx_primitives::{
	AuthorityDistance, AuthorityProvider, BlockSealAuthorityId, ValidatorRegistration,
};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

const LOG_TARGET: &str = "runtime::validator_cohorts";

/// Defines cohort groups that are allowed to validate Ulixee Block Proof.
///
/// New cohorts will enter every `BlocksBetweenCohorts` blocks. Each cohort will have
/// `MaxCohortSize`. A maximum of `MaxValidators` will be active at any given time.
///
/// When a new cohort begins, the validators with the corresponding indices will be replaced with
/// the new cohort members (or emptied out).
///
/// TODO: to be eligible for a cohort, you must bond a percent of the total supply of Ulixee tokens
/// 	that have been minted, and then a bid of locked Argons will determine who is selected.
///
/// ### Registration
/// To register for a cohort, you must submit a transaction with the block number of the upcoming
/// cohort, along with your peerId and (TODO: rpc host).
///
/// NOTE: to be an active validator, you must have also submitted "set_keys" to the network using
/// the Session pallet. This is what creates "AuthorityIds", and used for finding XOR closest peers
/// to a CloudNode wishing to prove they can close a block.
///
/// AuthorityIds are created by watching the Session pallet for new sessions and recording the
/// authorityIds matching registered "controller" accounts.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{pallet_prelude::*, BoundedVec};
	use frame_system::pallet_prelude::*;
	use sp_core::OpaquePeerId;
	use sp_runtime::{traits::UniqueSaturatedInto, BoundedBTreeMap};

	use ulx_primitives::{PeerId, ValidatorRegistration};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// The maximum number of validators that the pallet can hold.
		#[pallet::constant]
		type MaxValidators: Get<u32>;
		/// How many new validators can be in each cohort
		#[pallet::constant]
		type MaxCohortSize: Get<u32>;
		/// How many blocks into the future will we take registrations
		#[pallet::constant]
		type MaxPendingCohorts: Get<u32>;
		/// How many blocks transpire between cohorts
		#[pallet::constant]
		type BlocksBetweenCohorts: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn active_validators_by_index)]
	/// Active validators that are active in the current block (post initialize)
	pub(super) type ActiveValidatorsByIndex<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<u32, ValidatorRegistration<T::AccountId>, T::MaxValidators>,
		ValueQuery,
	>;

	#[pallet::storage]
	/// Authorities are the session keys that are actively participating in the network.
	/// The tuple is the authority, and the blake2 256 hash of the authority used for xor lookups
	pub(super) type AuthoritiesByIndex<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<u32, (BlockSealAuthorityId, U256), T::MaxValidators>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn account_indices)]
	/// Lookup by account id to the corresponding index in ActiveValidatorsByIndex and Authorities
	pub(super) type AccountIndexLookup<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, u32, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn queued_cohorts)]
	pub(super) type QueuedCohorts<T: Config> = StorageMap<
		_,
		// Use simple hasher since block number should be safe
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<ValidatorRegistration<T::AccountId>, T::MaxCohortSize>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewValidators {
			start_index: u32,
			cohort_size: u32,
			new_validators: BoundedVec<ValidatorRegistration<T::AccountId>, T::MaxCohortSize>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		CohortBlockNumberNotAnEntrypoint,
		CohortBlockTooFarInFuture,
		CohortBlockTooOld,
		TooManyBlockRegistrants,
		UnableToRotateAuthority,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			// Translating the current block number to number and submit it on-chain
			let block_number_u32: u32 =
				UniqueSaturatedInto::<u32>::unique_saturated_into(block_number);
			let blocks_between_cohorts = T::BlocksBetweenCohorts::get();
			if block_number_u32 % blocks_between_cohorts != 0 {
				return T::DbWeight::get().reads_writes(0, 0)
			}
			let max_validators = T::MaxValidators::get();
			let cohort_size = T::MaxCohortSize::get();

			let start_index_to_replace_validators = Self::get_start_cohort_index(
				block_number_u32,
				blocks_between_cohorts,
				max_validators,
				cohort_size,
			);

			let cohort = QueuedCohorts::<T>::take(block_number);

			let _ = ActiveValidatorsByIndex::<T>::try_mutate(|validators| {
				for i in 0..cohort_size {
					let index = i + start_index_to_replace_validators;

					let current_entry = validators.remove(&index);
					if let Some(entry) = current_entry {
						AccountIndexLookup::<T>::remove(&entry.account_id);
					}

					if let Some(registration) = cohort.get(i as usize) {
						AccountIndexLookup::<T>::insert(&registration.account_id, &index);
						match validators.try_insert(index, registration.clone()) {
							Err(err) => {
								panic!(
									"Error rotating new authorities starting at {:?} at index {}. {:?}",
									start_index_to_replace_validators, i, err
								);
							},
							_ => (),
						};
					}
				}
				Ok::<(), Error<T>>(())
			});
			Pallet::<T>::deposit_event(Event::<T>::NewValidators {
				start_index: start_index_to_replace_validators,
				cohort_size,
				new_validators: cohort,
			});

			T::DbWeight::get().reads_writes(0, 2)
		}

		/// Only allow registered authorities to connect to the kad. This means ONLY bonded
		/// validators will be allowed to query the kad.
		/// NOTE: I think this also means that light clients can't connect to discover peers, which
		/// I don't think we want. We're going to need to actually only use this to restrict
		/// commands to find items from the kad
		fn offchain_worker(now: BlockNumberFor<T>) {
			let network_state = sp_io::offchain::network_state();
			match network_state {
				Err(_) => log::error!(
					target: "runtime::node-authorization",
					"Error: failed to get network state of node at {:?}",
					now,
				),
				Ok(_state) => sp_io::offchain::set_authorized_nodes(
					<ActiveValidatorsByIndex<T>>::get()
						.iter()
						.map(|(_, x)| x.peer_id.0.clone())
						.collect(),
					true,
				),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)] //T::WeightInfo::register_account())]
		pub fn reserve(
			origin: OriginFor<T>,
			block_number: BlockNumberFor<T>,
			peer_id: OpaquePeerId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let block_number_u32 = UniqueSaturatedInto::<u32>::unique_saturated_into(block_number);
			if block_number_u32 % T::BlocksBetweenCohorts::get() != 0 {
				return Err(Error::<T>::CohortBlockNumberNotAnEntrypoint.into())
			}

			let current_block_number = UniqueSaturatedInto::<u32>::unique_saturated_into(
				<frame_system::Pallet<T>>::block_number(),
			);
			if block_number_u32 <= current_block_number {
				return Err(Error::<T>::CohortBlockTooOld.into())
			}
			if current_block_number >= block_number_u32 + T::MaxPendingCohorts::get() {
				return Err(Error::<T>::CohortBlockTooFarInFuture.into())
			}
			<QueuedCohorts<T>>::try_mutate(block_number, |entrants| -> DispatchResult {
				let pos = match entrants.binary_search_by(|x| x.account_id.cmp(&who)) {
					Ok(pos) => pos,
					Err(pos) => pos,
				};
				if let Some(entry) = entrants.get(pos) {
					if entry.account_id == who {
						entrants.remove(pos);
					}
				}

				entrants
					.try_insert(
						pos,
						ValidatorRegistration { account_id: who, peer_id: PeerId(peer_id) },
					)
					.map_err(|_| Error::<T>::TooManyBlockRegistrants)?;
				Ok(())
			})?;
			Ok(())
		}
	}
}

impl<T: Config> IsMember<T::AccountId> for Pallet<T> {
	fn is_member(account_id: &T::AccountId) -> bool {
		<ActiveValidatorsByIndex<T>>::get()
			.iter()
			.any(|(_, a)| a.account_id == *account_id)
	}
}

impl<T: Config> AuthorityProvider<BlockSealAuthorityId, T::AccountId> for Pallet<T> {
	fn authorities() -> Vec<BlockSealAuthorityId> {
		Self::authorities_by_index()
			.into_iter()
			.map(|(_, a)| a.clone())
			.collect::<Vec<_>>()
	}

	fn authorities_by_index() -> BTreeMap<u16, BlockSealAuthorityId> {
		<AuthoritiesByIndex<T>>::get()
			.iter()
			.map(|(i, a)| (i.clone().unique_saturated_into(), a.0.clone()))
			.collect()
	}

	fn authority_count() -> u16 {
		Self::authorities().len().unique_saturated_into()
	}

	fn get_authority(author: T::AccountId) -> Option<BlockSealAuthorityId> {
		<AccountIndexLookup<T>>::get(&author).and_then(|index| {
			Self::authorities_by_index()
				.get(&index.unique_saturated_into())
				.map(|a| a.clone())
		})
	}
	fn find_xor_closest_authorities(
		hash: U256,
		closest: u8,
	) -> Vec<AuthorityDistance<BlockSealAuthorityId>> {
		let validators = Self::active_validators_by_index();
		find_xor_closest(<AuthoritiesByIndex<T>>::get(), hash, closest)
			.into_iter()
			.map(|(a, distance, index)| {
				let registration = validators.get(&index.into()).unwrap();
				AuthorityDistance::<_> {
					authority_id: a.clone(),
					authority_index: index.unique_saturated_into(),
					peer_id: registration.peer_id.clone(),
					distance,
				}
			})
			.collect()
	}
}

impl<T: Config> Pallet<T> {
	pub fn upcoming_cohort_blocks() -> Vec<BlockNumberFor<T>> {
		let current_block_number = UniqueSaturatedInto::<u32>::unique_saturated_into(
			<frame_system::Pallet<T>>::block_number(),
		);
		let count = T::MaxPendingCohorts::get().unique_saturated_into();
		let mut blocks = Vec::new();
		let start = current_block_number - (current_block_number % T::BlocksBetweenCohorts::get());
		for i in 0..count {
			let block_number = start + i * T::BlocksBetweenCohorts::get();
			blocks.push(block_number.into());
		}
		blocks
	}

	pub fn get_start_cohort_index(
		block_number: u32,
		blocks_between_cohorts: u32,
		max_validators: u32,
		cohort_size: u32,
	) -> u32 {
		if block_number < blocks_between_cohorts {
			return 0
		}
		let cohort = block_number / blocks_between_cohorts;
		(cohort * cohort_size) % max_validators
	}
}
pub fn find_xor_closest<I>(
	authorities: I,
	hash: U256,
	closest: u8,
) -> Vec<(BlockSealAuthorityId, U256, u32)>
where
	I: IntoIterator<Item = (u32, (BlockSealAuthorityId, U256))>,
{
	let mut authority_xor_distances = authorities
		.into_iter()
		.map(|(index, (a, peer_hash))| {
			let distance = hash ^ peer_hash;
			(a, distance, index)
		})
		.collect::<Vec<_>>();

	// sort shortest on top
	authority_xor_distances.sort_by(|a, b| a.1.cmp(&b.1));
	authority_xor_distances.truncate(closest as usize);
	authority_xor_distances
}

pub struct StashOf<T>(PhantomData<T>);

impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for StashOf<T> {
	fn convert(controller: T::AccountId) -> Option<T::AccountId> {
		if <AccountIndexLookup<T>>::contains_key(&controller) {
			// TODO: create a stash mapping
			Some(controller)
		} else {
			None
		}
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = BlockSealAuthorityId;
}

impl<T: Config> SessionManager<T::AccountId> for Pallet<T> {
	fn new_session_genesis(_: u32) -> Option<Vec<T::AccountId>> {
		None
	}
	fn new_session(_: u32) -> Option<Vec<T::AccountId>> {
		let block_number_u32: u32 = UniqueSaturatedInto::<u32>::unique_saturated_into(
			<frame_system::Pallet<T>>::block_number(),
		);
		// only rotate validators on cohort changeover. The keys representing the authority ids will
		// auto-change
		if block_number_u32 % T::BlocksBetweenCohorts::get() != 0 {
			return None
		}
		Some(
			<ActiveValidatorsByIndex<T>>::get()
				.into_iter()
				.filter_map(|(_, a)| a.account_id.try_into().ok())
				.collect(),
		)
	}
	fn start_session(_: u32) {}
	fn end_session(_: u32) {}
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = BlockSealAuthorityId;

	fn on_genesis_session<'a, I: 'a>(_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		T::AccountId: 'a,
	{
	}

	fn on_new_session<'a, I: 'a>(changed: bool, validators_with_keys: I, _queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, BlockSealAuthorityId)>,
	{
		// instant changes
		if changed {
			let mut next_authorities =
				BoundedBTreeMap::<u32, (BlockSealAuthorityId, U256), T::MaxValidators>::new();
			for (account_id, authority_id) in validators_with_keys {
				if let Some(account_index) = <AccountIndexLookup<T>>::get(&account_id) {
					let hash = blake2_256(&sp_runtime::RuntimeAppPublic::to_raw_vec(&authority_id));
					if let None = next_authorities
						.try_insert(account_index, (authority_id, U256::from(hash)))
						.ok()
					{
						// TODO: should we burn bonds when this happens?
						log::warn!(
							target: LOG_TARGET,
							"Could not insert authority {:?} at index {:?} into next_authorities",
							account_id,
							account_index
						);
					}
				}
			}

			let active_validators = <ActiveValidatorsByIndex<T>>::get();
			if next_authorities.len() != active_validators.len() {
				let no_key_validators = active_validators
					.into_iter()
					.filter(|(index, _)| !next_authorities.contains_key(&index))
					.map(|a| a.1.account_id)
					.collect::<Vec<_>>();
				log::warn!(
							target: LOG_TARGET,
					"The following registered validator accounts do not have session keys: {:?}",
					no_key_validators
				);
			}

			let last_authorities = <AuthoritiesByIndex<T>>::get();
			if last_authorities != next_authorities {
				<AuthoritiesByIndex<T>>::put(next_authorities);
			}
		}
	}

	fn on_disabled(_validator_index: u32) {}
}

sp_api::decl_runtime_apis! {
	/// This runtime api allows people to query the upcoming cohorts
	pub trait ValidatorCohortsApi<BlockNumber, AccountId> where
		BlockNumber: Codec, AccountId:Codec {
		fn get_cohort(at: BlockNumber) -> Vec<ValidatorRegistration<AccountId>>;
		fn upcoming_cohort_blocks() -> Vec<BlockNumber>;
	}
}
