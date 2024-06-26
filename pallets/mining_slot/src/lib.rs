#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{InspectHold, MutateHold},
		tokens::Precision,
		OneSessionHandler,
	},
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_session::SessionManager;
use sp_core::{Get, U256};
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{Convert, UniqueSaturatedInto},
	BoundedBTreeMap,
};
use sp_std::{marker::PhantomData, vec::Vec};

pub use pallet::*;
use ulx_primitives::{
	block_seal::{BlockSealAuthorityId, MinerIndex, MiningAuthority, RewardDestination},
	bond::BondProvider,
	AuthorityProvider,
};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

const LOG_TARGET: &str = "runtime::mining_slot";

/// To register as a Proof of Block miner, operators must `Bid` on a `Slot`. Each `Slot` allows a
/// `Cohort` of miners to operate for a given number of blocks (an `Era`).
///
/// New miner slots are rotated in every `BlocksBetweenSlots` blocks. Each cohort will have
/// `MaxCohortSize` members. A maximum of `MaxMiners` will be active at any given time.
///
/// When a new Slot begins, the Miners with the corresponding Slot indices will be replaced with
/// the new cohort members (or emptied out).
///
/// To be eligible for mining, you must bond a percent of the total supply of Ulixee tokens. A
/// `Bond` of locked Argons will allow operators to out-bid others for cohort membership. The
/// percent is configured with `OwnershipPercentDamper`. We might want to make this percent
/// adjustable via governance in the future.
///
/// Options are provided to lease a bond from a fund (see the bond pallet).
///
/// ### Registration
/// To register for a Slot, you must submit a bid. At any given time, only the next Slot is being
/// bid on.
///
/// NOTE: to be an active miner, you must have also submitted "Session.set_keys" to the network
/// using the Session pallet. This is what creates "AuthorityIds", and used for finding XOR closest
/// peers to block votes.
///
/// AuthorityIds are created by watching the Session pallet for new sessions and recording the
/// authorityIds matching registered "controller" accounts.
///
/// TODO: add VRF to pick block end for bid registrations
/// TODO: add bid_and_bond, bid_and_lease_bond calls (or make bid::bond_id an enum of bond
/// 	creation options)
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::fungible::{Inspect, MutateHold},
		BoundedVec,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, UniqueSaturatedInto},
		BoundedBTreeMap, SaturatedConversion,
	};
	use sp_std::cmp::{max, Ordering};

	use ulx_primitives::{
		block_seal::{MiningRegistration, RewardDestination},
		bond::{BondError, BondProvider},
		BondId, VaultId,
	};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	pub type Registration<T> =
		MiningRegistration<<T as frame_system::Config>::AccountId, <T as Config>::Balance>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// The maximum number of Miners that the pallet can hold.
		#[pallet::constant]
		type MaxMiners: Get<u32>;
		/// How many new miners can be in the cohort for each slot
		#[pallet::constant]
		type MaxCohortSize: Get<u32>;
		/// How many blocks transpire between slots
		#[pallet::constant]
		type BlocksBetweenSlots: Get<u32>;
		/// How many session indexes to keep session history
		#[pallet::constant]
		type SessionIndicesToKeepInHistory: Get<u32>;

		/// How many blocks buffer shall we use to stop accepting bids for the next period
		#[pallet::constant]
		type BlocksBufferToStopAcceptingBids: Get<u32>;

		/// The reduction in percent of ownership currency required to secure a slot
		#[pallet::constant]
		type OwnershipPercentDamper: Get<u32>;

		/// The balance type
		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ sp_std::fmt::Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The currency representing ownership in the network - aka, rights to validate
		type OwnershipCurrency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Inspect<Self::AccountId, Balance = Self::Balance>;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		type BondProvider: BondProvider<
			Balance = Self::Balance,
			AccountId = Self::AccountId,
			BlockNumber = BlockNumberFor<Self>,
		>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		#[codec(index = 0)]
		RegisterAsMiner,
	}

	/// Miners that are active in the current block (post initialize)
	#[pallet::storage]
	#[pallet::getter(fn active_miners_by_index)]
	pub(super) type ActiveMinersByIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, MinerIndex, Registration<T>, OptionQuery>;
	#[pallet::storage]
	pub(super) type ActiveMinersCount<T: Config> = StorageValue<_, u16, ValueQuery>;

	/// Authorities are the session keys that are actively participating in the network.
	/// The tuple is the authority, and the blake2 256 hash of the authority used for xor lookups
	#[pallet::storage]
	pub(super) type AuthoritiesByIndex<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<MinerIndex, (BlockSealAuthorityId, U256), T::MaxMiners>,
		ValueQuery,
	>;

	/// Tokens that must be bonded to take a Miner role
	#[pallet::storage]
	#[pallet::getter(fn ownership_bond_amount)]
	pub(super) type OwnershipBondAmount<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	/// Lookup by account id to the corresponding index in ActiveMinersByIndex and Authorities
	#[pallet::storage]
	#[pallet::getter(fn account_indices)]
	pub(super) type AccountIndexLookup<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, MinerIndex, OptionQuery>;

	/// The cohort set to go into effect in the next slot. The Vec has all
	/// registrants with their bid amount
	#[pallet::storage]
	pub(super) type NextSlotCohort<T: Config> =
		StorageValue<_, BoundedVec<Registration<T>, T::MaxCohortSize>, ValueQuery>;

	/// Is the next slot still open for bids
	#[pallet::storage]
	pub(super) type IsNextSlotBiddingOpen<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The configuration for a miner to supply if there are no registered miners
	#[pallet::storage]
	#[pallet::getter(fn rescue_miner)]
	pub(super) type MinerZero<T: Config> = StorageValue<_, Registration<T>, OptionQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub miner_zero: Option<Registration<T>>,
		#[serde(skip)]
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(miner) = &self.miner_zero {
				<MinerZero<T>>::put(miner);
				Pallet::<T>::on_initialize(0u32.into());
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewMiners {
			start_index: MinerIndex,
			new_miners: BoundedVec<Registration<T>, T::MaxCohortSize>,
		},
		SlotBidderAdded {
			account_id: T::AccountId,
			bid_amount: T::Balance,
			index: u32,
		},
		SlotBidderReplaced {
			account_id: T::AccountId,
			bond_id: Option<BondId>,
			kept_ownership_bond: bool,
		},
		UnbondedMiner {
			account_id: T::AccountId,
			bond_id: Option<BondId>,
			kept_ownership_bond: bool,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		SlotNotTakingBids,
		TooManyBlockRegistrants,
		InsufficientOwnershipTokens,
		BidTooLow,
		/// A Non-Mining bond was submitted as part of a bid
		CannotRegisterOverlappingSessions,
		// copied from bond
		BondNotFound,
		NoMoreBondIds,
		VaultClosed,
		MinimumBondAmountNotMet,
		/// There are too many bond or bond funds expiring in the given expiration block
		ExpirationAtBlockOverflow,
		InsufficientFunds,
		InsufficientVaultFunds,
		ExpirationTooSoon,
		NoPermissions,
		HoldUnexpectedlyModified,
		UnrecoverableHold,
		VaultNotFound,
		BondAlreadyClosed,
		/// The fee for this bond exceeds the amount of the bond, which is unsafe
		FeeExceedsBondAmount,
		AccountWouldBeBelowMinimum,
		GenericBondError(BondError),
	}

	impl<T> From<BondError> for Error<T> {
		fn from(e: BondError) -> Error<T> {
			match e {
				BondError::BondNotFound => Error::<T>::BondNotFound,
				BondError::NoMoreBondIds => Error::<T>::NoMoreBondIds,
				BondError::MinimumBondAmountNotMet => Error::<T>::MinimumBondAmountNotMet,
				BondError::ExpirationAtBlockOverflow => Error::<T>::ExpirationAtBlockOverflow,
				BondError::InsufficientFunds => Error::<T>::InsufficientFunds,
				BondError::InsufficientVaultFunds => Error::<T>::InsufficientVaultFunds,
				BondError::ExpirationTooSoon => Error::<T>::ExpirationTooSoon,
				BondError::NoPermissions => Error::<T>::NoPermissions,
				BondError::VaultClosed => Error::<T>::VaultClosed,
				BondError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				BondError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				BondError::VaultNotFound => Error::<T>::VaultNotFound,
				BondError::FeeExceedsBondAmount => Error::<T>::FeeExceedsBondAmount,
				BondError::AccountWouldBeBelowMinimum => Error::<T>::AccountWouldBeBelowMinimum,
				_ => Error::<T>::GenericBondError(e),
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let max_miners = T::MaxMiners::get();
			let cohort_size = T::MaxCohortSize::get();

			let ownership_circulation: u128 =
				T::OwnershipCurrency::total_issuance().saturated_into();

			let ownership_needed: u128 = ownership_circulation
				.checked_div(T::MaxMiners::get().into())
				.unwrap_or(0u128)
				.saturating_mul(T::OwnershipPercentDamper::get().into())
				.checked_div(100u128)
				.unwrap_or(0u128);

			OwnershipBondAmount::<T>::put(max::<T::Balance>(
				ownership_needed.saturated_into(),
				1u128.into(),
			));

			// Translating the current block number to number and submit it on-chain
			let block_number_u32: u32 =
				UniqueSaturatedInto::<u32>::unique_saturated_into(block_number);
			let blocks_between_slots = T::BlocksBetweenSlots::get();
			if block_number_u32 % blocks_between_slots != 0 {
				return T::DbWeight::get().reads_writes(0, 0);
			}

			let start_index_to_replace_miners = Self::get_slot_starting_index(
				block_number_u32,
				blocks_between_slots,
				max_miners,
				cohort_size,
			);

			let slot_cohort = NextSlotCohort::<T>::take();
			IsNextSlotBiddingOpen::<T>::put(true);
			let mut active_miners = ActiveMinersCount::<T>::get();

			for i in 0..cohort_size {
				let index = i + start_index_to_replace_miners;

				if let Some(entry) = ActiveMinersByIndex::<T>::take(index) {
					let account_id = entry.account_id.clone();
					AccountIndexLookup::<T>::remove(&account_id);
					active_miners -= 1;

					let next = slot_cohort.iter().find(|x| x.account_id == account_id).cloned();
					match Self::unbond_account(entry, next) {
						Err(err) => {
							log::error!(
								target: LOG_TARGET,
								"Failed to unbond account {:?}. {:?}",
								account_id,
								err,
							);
						},
						_ => (),
					}
				}

				if let Some(registration) = slot_cohort.get(i as usize) {
					AccountIndexLookup::<T>::insert(&registration.account_id, index);
					active_miners += 1;
					ActiveMinersByIndex::<T>::insert(index, registration.clone());
				}
			}

			if active_miners == 0 {
				if let Some(miner) = MinerZero::<T>::get() {
					let index = start_index_to_replace_miners;
					AccountIndexLookup::<T>::insert(&miner.account_id, index);
					active_miners = 1;
					ActiveMinersByIndex::<T>::insert(index, miner.clone());
				}
			}

			ActiveMinersCount::<T>::put(active_miners);

			Pallet::<T>::deposit_event(Event::<T>::NewMiners {
				start_index: start_index_to_replace_miners,
				new_miners: slot_cohort,
			});

			T::DbWeight::get().reads_writes(0, 2)
		}

		fn on_finalize(block_number: BlockNumberFor<T>) {
			// TODO: vrf for closing bids
			if Self::get_next_slot_block_number() - block_number <
				T::BlocksBufferToStopAcceptingBids::get().into()
			{
				IsNextSlotBiddingOpen::<T>::put(false);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)] //T::WeightInfo::hold())]
		pub fn bid(
			origin: OriginFor<T>,
			bond_info: Option<MiningSlotBid<VaultId, T::Balance>>,
			reward_destination: RewardDestination<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(IsNextSlotBiddingOpen::<T>::get(), Error::<T>::SlotNotTakingBids);

			let next_cohort_block_number = Self::get_next_slot_block_number();
			if let Some(current_index) = <AccountIndexLookup<T>>::get(&who) {
				let cohort_start_index = Self::get_next_slot_starting_index();
				let is_in_next_cohort = current_index >= cohort_start_index &&
					current_index < (cohort_start_index + T::MaxCohortSize::get());

				// current_index must be in the set of miners being replaced
				ensure!(is_in_next_cohort, Error::<T>::CannotRegisterOverlappingSessions);
			}

			let current_registration = Self::get_active_registration(&who);

			let (bond_id, bid) = if let Some(bond_info) = bond_info {
				let bond_end_block = next_cohort_block_number + Self::get_mining_window_blocks();
				let bond_id = T::BondProvider::bond_mining_slot(
					bond_info.vault_id,
					who.clone(),
					bond_info.amount,
					bond_end_block,
				)
				.map_err(Error::<T>::from)?;
				(Some(bond_id), bond_info.amount)
			} else {
				(None, 0u128.into())
			};

			let ownership_tokens = Self::hold_ownership_bond(&who, current_registration)?;

			<NextSlotCohort<T>>::try_mutate(|cohort| -> DispatchResult {
				if let Some(existing_position) = cohort.iter().position(|x| x.account_id == who) {
					cohort.remove(existing_position);
				}

				// sort to lowest position at bid
				let pos = cohort
					.binary_search_by(|x| {
						let comp = bid.cmp(&x.bond_amount);
						match comp {
							Ordering::Equal => Ordering::Less,
							Ordering::Greater => Ordering::Greater,
							Ordering::Less => Ordering::Less,
						}
					})
					.unwrap_or_else(|pos| pos);

				ensure!(pos < T::MaxCohortSize::get() as usize, Error::<T>::BidTooLow);

				if UniqueSaturatedInto::<u32>::unique_saturated_into(cohort.len()) >=
					T::MaxCohortSize::get()
				{
					// need to pop-off the lowest bid
					let entry = cohort.pop().unwrap();
					Self::release_failed_bid(entry)?;
				}

				cohort
					.try_insert(
						pos,
						MiningRegistration {
							account_id: who.clone(),
							reward_destination,
							bond_id,
							bond_amount: bid,
							ownership_tokens,
						},
					)
					.map_err(|_| Error::<T>::TooManyBlockRegistrants)?;

				Self::deposit_event(Event::<T>::SlotBidderAdded {
					account_id: who.clone(),
					bid_amount: bid,
					index: UniqueSaturatedInto::<u32>::unique_saturated_into(pos),
				});

				Ok(())
			})?;

			Ok(())
		}
	}
}

impl<T: Config> AuthorityProvider<BlockSealAuthorityId, T::Block, T::AccountId> for Pallet<T> {
	fn get_authority(author: T::AccountId) -> Option<BlockSealAuthorityId> {
		<AccountIndexLookup<T>>::get(&author)
			.and_then(|index| AuthoritiesByIndex::<T>::get().get(&index).map(|x| x.0.clone()))
	}

	fn get_rewards_account(author: T::AccountId) -> Option<T::AccountId> {
		Self::get_active_registration(&author).map(|x| match x.reward_destination {
			RewardDestination::Owner => x.account_id,
			RewardDestination::Account(reward_id) => reward_id,
		})
	}

	fn get_all_rewards_accounts() -> Vec<T::AccountId> {
		<ActiveMinersByIndex<T>>::iter()
			.map(|(_, registration)| match registration.reward_destination {
				RewardDestination::Owner => registration.account_id,
				RewardDestination::Account(reward_id) => reward_id,
			})
			.collect()
	}

	fn xor_closest_authority(
		nonce: U256,
	) -> Option<MiningAuthority<BlockSealAuthorityId, T::AccountId>> {
		let closest = find_xor_closest(<AuthoritiesByIndex<T>>::get(), nonce);

		closest.map(|(authority_id, index)| {
			let registration = Self::active_miners_by_index(index).unwrap();
			MiningAuthority {
				authority_id,
				account_id: registration.account_id.clone(),
				authority_index: index.unique_saturated_into(),
			}
		})
	}
}

impl<T: Config> Pallet<T> {
	pub fn get_mining_authority(
		account_id: &T::AccountId,
	) -> Option<MiningAuthority<BlockSealAuthorityId, T::AccountId>> {
		let Some(index) = <AccountIndexLookup<T>>::get(account_id) else {
			return None;
		};
		AuthoritiesByIndex::<T>::get()
			.get(&index)
			.map(|(authority_id, _)| MiningAuthority {
				authority_id: authority_id.clone(),
				account_id: account_id.clone(),
				authority_index: index.unique_saturated_into(),
			})
	}

	pub(crate) fn get_next_slot_block_number() -> BlockNumberFor<T> {
		let current_block_number = UniqueSaturatedInto::<u32>::unique_saturated_into(
			<frame_system::Pallet<T>>::block_number(),
		);
		let offset_blocks = current_block_number % T::BlocksBetweenSlots::get();
		(current_block_number + (T::BlocksBetweenSlots::get() - offset_blocks)).into()
	}

	pub fn get_slot_era() -> (BlockNumberFor<T>, BlockNumberFor<T>) {
		let next_block = Self::get_next_slot_block_number();
		(next_block, next_block + Self::get_mining_window_blocks())
	}

	pub(crate) fn get_slot_starting_index(
		block_number: u32,
		blocks_between_slots: u32,
		max_miners: u32,
		cohort_size: u32,
	) -> u32 {
		let cohort = block_number / blocks_between_slots;
		(cohort * cohort_size) % max_miners
	}

	pub(crate) fn get_next_slot_starting_index() -> u32 {
		let block_number = UniqueSaturatedInto::<u32>::unique_saturated_into(
			<frame_system::Pallet<T>>::block_number(),
		);
		let cohort_size = T::MaxCohortSize::get();
		cohort_size +
			Self::get_slot_starting_index(
				block_number,
				T::BlocksBetweenSlots::get(),
				T::MaxMiners::get(),
				cohort_size,
			)
	}

	pub(crate) fn get_mining_window_blocks() -> BlockNumberFor<T> {
		let miners = T::MaxMiners::get();
		let blocks_between_slots = T::BlocksBetweenSlots::get();
		let cohort_size = T::MaxCohortSize::get();

		let blocks_per_miner = miners.saturating_mul(blocks_between_slots) / cohort_size;
		blocks_per_miner.into()
	}

	pub(crate) fn get_active_registration(account_id: &T::AccountId) -> Option<Registration<T>> {
		if let Some(index) = AccountIndexLookup::<T>::get(account_id) {
			return ActiveMinersByIndex::<T>::get(index);
		}
		None
	}

	pub(crate) fn get_miner_accounts() -> Vec<T::AccountId> {
		<ActiveMinersByIndex<T>>::iter().map(|(_, a)| a.account_id).collect()
	}

	pub(crate) fn get_next_registration(account_id: &T::AccountId) -> Option<Registration<T>> {
		NextSlotCohort::<T>::get().into_iter().find(|x| x.account_id == *account_id)
	}

	pub(crate) fn load_session_keys<'a>(
		miners_with_keys: impl Iterator<Item = (&'a T::AccountId, BlockSealAuthorityId)>,
	) {
		let mut next_authorities =
			BoundedBTreeMap::<u32, (BlockSealAuthorityId, U256), T::MaxMiners>::new();
		for (account_id, authority_id) in miners_with_keys {
			if let Some(account_index) = <AccountIndexLookup<T>>::get(account_id) {
				let hash = blake2_256(&authority_id.clone().into_inner().0);
				// this should not be possible to fail. The bounds equal the source lookup
				next_authorities
					.try_insert(account_index, (authority_id, U256::from(hash)))
					.expect("should not be possible to fail next_authorities insert");
			}
		}

		if next_authorities.len() != <ActiveMinersCount<T>>::get() as usize {
			let no_key_miners = ActiveMinersByIndex::<T>::iter()
				.filter(|(index, _)| !next_authorities.contains_key(index))
				.map(|a| a.1.account_id)
				.collect::<Vec<_>>();
			if !no_key_miners.is_empty() {
				log::warn!(
					target: LOG_TARGET,
					"The following registered miner accounts do not have session keys: {:?}",
					no_key_miners
				);
			}
		}

		let last_authorities = <AuthoritiesByIndex<T>>::get();
		if last_authorities != next_authorities {
			<AuthoritiesByIndex<T>>::put(next_authorities);
		}
	}

	pub(crate) fn hold_ownership_bond(
		who: &T::AccountId,
		current_registration: Option<Registration<T>>,
	) -> Result<T::Balance, DispatchError> {
		let ownership_tokens = OwnershipBondAmount::<T>::get();
		let next_registration = Self::get_next_registration(who);
		let mut ownership_bond_needed = ownership_tokens;

		// if we've already held for next, reduce now
		if let Some(next) = next_registration {
			ownership_bond_needed -= next.ownership_tokens;
		} else if let Some(current_registration) = current_registration {
			ownership_bond_needed -= current_registration.ownership_tokens;
		}

		if ownership_bond_needed == 0u32.into() {
			return Ok(ownership_tokens);
		}

		let hold_reason = HoldReason::RegisterAsMiner;
		if T::OwnershipCurrency::balance_on_hold(&hold_reason.into(), who) == 0u32.into() {
			frame_system::Pallet::<T>::inc_providers(who);
		}

		T::OwnershipCurrency::hold(&hold_reason.into(), who, ownership_bond_needed)
			.map_err(|_| Error::<T>::InsufficientOwnershipTokens)?;
		Ok(ownership_tokens)
	}

	pub(crate) fn release_failed_bid(registration: Registration<T>) -> DispatchResult {
		let account_id = registration.account_id;

		if let Some(bond_id) = registration.bond_id {
			T::BondProvider::cancel_bond(bond_id).map_err(Error::<T>::from)?;
		}

		let mut kept_ownership_bond = false;
		let mut amount_to_unhold: T::Balance = registration.ownership_tokens;
		if let Some(active) = Self::get_active_registration(&account_id) {
			amount_to_unhold -= active.ownership_tokens;
			kept_ownership_bond = true;
		}

		Self::release_ownership_hold(&account_id, amount_to_unhold)?;

		Self::deposit_event(Event::<T>::SlotBidderReplaced {
			account_id: account_id.clone(),
			bond_id: registration.bond_id,
			kept_ownership_bond,
		});

		Ok(())
	}

	fn release_ownership_hold(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult {
		let reason = HoldReason::RegisterAsMiner;
		if amount == 0u32.into() {
			return Ok(());
		}
		T::OwnershipCurrency::release(&reason.into(), account_id, amount, Precision::Exact)
			.map_err(|_| Error::<T>::UnrecoverableHold)?;

		if T::OwnershipCurrency::balance_on_hold(&reason.into(), account_id) == 0u32.into() {
			let _ = frame_system::Pallet::<T>::dec_providers(account_id);
		}
		Ok(())
	}

	/// Unbond the account. If the argon bond will be re-used in the next era, we should not unlock
	/// it
	pub(crate) fn unbond_account(
		active_registration: Registration<T>,
		next_registration: Option<Registration<T>>,
	) -> DispatchResult {
		let account_id = active_registration.account_id;
		let active_bond_id = active_registration.bond_id;

		let mut kept_ownership_bond = true;
		if next_registration.is_none() {
			Self::release_ownership_hold(&account_id, active_registration.ownership_tokens)?;
			kept_ownership_bond = false;
		}

		Self::deposit_event(Event::<T>::UnbondedMiner {
			account_id: account_id.clone(),
			bond_id: active_bond_id,
			kept_ownership_bond,
		});

		Ok(())
	}
}

pub fn find_xor_closest<I>(authorities: I, hash: U256) -> Option<(BlockSealAuthorityId, MinerIndex)>
where
	I: IntoIterator<Item = (MinerIndex, (BlockSealAuthorityId, U256))>,
{
	let mut closest_distance: U256 = U256::MAX;
	let mut closest = None;
	for (index, (a, peer_hash)) in authorities.into_iter() {
		let distance = hash ^ peer_hash;
		if distance < closest_distance {
			closest_distance = distance;
			closest = Some((a, index));
		}
	}
	closest
}

// Lookup needed for pallet_session
pub struct ValidatorIdOf<T>(PhantomData<T>);

impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for ValidatorIdOf<T> {
	fn convert(account_id: T::AccountId) -> Option<T::AccountId> {
		if <AccountIndexLookup<T>>::contains_key(&account_id) {
			Some(account_id)
		} else {
			None
		}
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct MinerHistory {
	pub authority_index: MinerIndex,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct MiningSlotBid<VaultId: Codec, Balance: Codec> {
	pub vault_id: VaultId,
	pub amount: Balance,
}

/// What to track in history
pub struct FullIdentificationOf<T>(PhantomData<T>);

impl<T: Config> Convert<T::AccountId, Option<MinerHistory>> for FullIdentificationOf<T> {
	fn convert(miner: T::AccountId) -> Option<MinerHistory> {
		if let Some(index) = <AccountIndexLookup<T>>::get(&miner) {
			return Some(MinerHistory { authority_index: index });
		}
		None
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = BlockSealAuthorityId;
}

impl<T: Config> SessionManager<T::AccountId> for Pallet<T> {
	fn new_session(_: u32) -> Option<Vec<T::AccountId>> {
		let block_number_u32: u32 = UniqueSaturatedInto::<u32>::unique_saturated_into(
			<frame_system::Pallet<T>>::block_number(),
		);
		// only rotate miners on cohort changeover. The keys representing the authority ids will
		// auto-change
		if block_number_u32 % T::BlocksBetweenSlots::get() != 0 {
			return None;
		}
		Some(Self::get_miner_accounts())
	}
	fn new_session_genesis(_: u32) -> Option<Vec<T::AccountId>> {
		Some(Self::get_miner_accounts())
	}
	fn end_session(_: u32) {}
	fn start_session(_: u32) {}
}

impl<T: Config> pallet_session::historical::SessionManager<T::AccountId, MinerHistory> for Pallet<T>
where
	T: pallet_session::historical::Config<
		FullIdentification = MinerHistory,
		FullIdentificationOf = FullIdentificationOf<T>,
	>,
{
	fn new_session(new_index: u32) -> Option<Vec<(T::AccountId, MinerHistory)>> {
		<Self as SessionManager<_>>::new_session(new_index).map(|miners| {
			miners
				.into_iter()
				.filter_map(|v| {
					if let Some(miner) = FullIdentificationOf::<T>::convert(v.clone()) {
						return Some((v, miner));
					}
					None
				})
				.collect::<Vec<_>>()
		})
	}

	fn new_session_genesis(_: u32) -> Option<Vec<(T::AccountId, MinerHistory)>> {
		<Self as pallet_session::historical::SessionManager<_, _>>::new_session(0)
	}

	fn start_session(_: u32) {}
	fn end_session(index: u32) {
		let first_session = index.saturating_sub(T::SessionIndicesToKeepInHistory::get());
		<pallet_session::historical::Pallet<T>>::prune_up_to(first_session);
	}
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = BlockSealAuthorityId;

	fn on_genesis_session<'a, I: 'a>(miners: I)
	where
		I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
		T::AccountId: 'a,
	{
		Self::load_session_keys(miners);
	}

	fn on_new_session<'a, I: 'a>(changed: bool, miners_with_keys: I, _queued_miners: I)
	where
		I: Iterator<Item = (&'a T::AccountId, BlockSealAuthorityId)>,
	{
		if changed {
			Self::load_session_keys(miners_with_keys);
		}
	}

	fn on_disabled(_miner_index: u32) {}
}

sp_api::decl_runtime_apis! {
	/// This runtime api allows people to query the upcoming mining_slot
	pub trait MiningSlotApi<BlockNumber> where
		BlockNumber: Codec {
		fn next_slot_era() -> (BlockNumber, BlockNumber);
	}
}
