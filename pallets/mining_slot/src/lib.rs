#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

use alloc::{vec, vec::Vec};
use argon_primitives::{
	block_seal::{
		MinerIndex, MiningAuthority, MiningBidStats, MiningSlotConfig, RewardDestination,
		RewardSharing, SlotId,
	},
	inherents::BlockSealInherent,
	tick::Tick,
	vault::BondedArgonsProvider,
	AuthorityProvider, BlockRewardAccountsProvider, BlockSealEventHandler, MiningSlotProvider,
	RewardShare, TickProvider,
};
use codec::Codec;
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{Inspect, InspectHold, MutateHold},
		tokens::Precision,
	},
};
pub use pallet::*;
use sp_core::{Get, U256};
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{One, OpaqueKeys, UniqueSaturatedInto},
	FixedPointNumber, FixedU128, RuntimeAppPublic, SaturatedConversion, Saturating,
};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
pub mod weights;

/// To register as a Slot 1+ miner, operators must `Bid` on a `Slot`. Each `Slot` allows a
/// `Cohort` of miners to operate for a given number of blocks (an `Era`).
///
/// New miner slots are rotated in every `mining_config.ticks_between_slots` blocks. Each cohort
/// will have `MaxCohortSize` members. A maximum of `MaxMiners` will be active at any given time.
///
/// When a new Slot begins, the Miners with the corresponding Slot indices will be replaced with
/// the new cohort members (or emptied out).
///
/// To be eligible for mining, you must reserve a percent of the total supply of argonots (ownership
/// tokens). `BondedArgons` leased from a vault will allow operators to out-bid others for cohort
/// membership. The percent is configured to aim for `TargetBidsPerSlot`, with a maximum change in
/// ownership tokens needed per slot capped at `ArgonotsPercentAdjustmentDamper` (NOTE: this
/// percent is the max increase or reduction in the amount of ownership issued).
///
/// Options are provided to lease Bonded Argons from the `vault pallet`
///
/// ### Registration
/// To register for a Slot, you must submit a bid. At any given time, only the next Slot is being
/// bid on.
///
/// AuthorityIds are created by watching the Session pallet for new sessions and recording the
/// authorityIds matching registered "controller" accounts.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use codec::FullCodec;
	use core::cmp::Ordering;
	use frame_support::{
		pallet_prelude::*,
		traits::fungible::{Inspect, MutateHold},
		BoundedVec,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, Member, OpaqueKeys, UniqueSaturatedInto},
		BoundedBTreeMap, Percent,
	};

	use argon_primitives::{
		block_seal::{MiningRegistration, RewardDestination},
		prelude::*,
		vault::{BondedArgonsProvider, ObligationError},
		TickProvider,
	};

	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	pub type Registration<T> = MiningRegistration<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
		<T as Config>::Keys,
	>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config
	where
		<Self as Config>::Balance: Into<u128>,
	{
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

		/// The max percent swing for the argonots per slot (from the last percent
		#[pallet::constant]
		type ArgonotsPercentAdjustmentDamper: Get<FixedU128>;
		/// The minimum argonots needed per seat
		#[pallet::constant]
		type MinimumArgonotsPerSeat: Get<Self::Balance>;

		/// The maximum percent of argonots in the network that should be required for
		/// mining seats
		#[pallet::constant]
		type MaximumArgonotProrataPercent: Get<Percent>;

		/// The target number of bids per slot. This will adjust the argonots per seat up or
		/// down to ensure mining slots are filled.
		#[pallet::constant]
		type TargetBidsPerSlot: Get<u32>;

		/// The balance type
		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ core::fmt::Debug
			+ Default
			+ From<u128>
			+ TryInto<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The currency representing ownership (argonots) in the network - aka, rights to validate
		type OwnershipCurrency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Inspect<Self::AccountId, Balance = Self::Balance>;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		type BondedArgonsProvider: BondedArgonsProvider<
			Balance = Self::Balance,
			AccountId = Self::AccountId,
		>;
		/// Handler when a new slot is started
		type SlotEvents: SlotEvents<Self::AccountId>;

		/// How often to rotate grandpas
		type GrandpaRotationBlocks: Get<BlockNumberFor<Self>>;

		/// The mining authority runtime public key
		type MiningAuthorityId: RuntimeAppPublic + FullCodec + Clone + TypeInfo;

		/// The authority signing keys.
		type Keys: OpaqueKeys + Member + Parameter + MaybeSerializeDeserialize;

		/// The current tick
		type TickProvider: TickProvider<Self::Block>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		#[codec(index = 0)]
		RegisterAsMiner,
	}

	#[pallet::storage]
	pub(super) type HasAddedGrandpaRotation<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Miners that are active in the current block (post initialize)
	#[pallet::storage]
	pub(super) type ActiveMinersByIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, MinerIndex, Registration<T>, OptionQuery>;
	#[pallet::storage]
	pub(super) type ActiveMinersCount<T: Config> = StorageValue<_, u16, ValueQuery>;

	/// Authorities are the session keys that are actively participating in the network.
	/// The tuple is the authority, and the blake2 256 hash of the authority used for xor lookups
	#[pallet::storage]
	pub(super) type AuthorityHashByIndex<T: Config> =
		StorageValue<_, BoundedBTreeMap<MinerIndex, U256, T::MaxMiners>, ValueQuery>;

	/// Keys in use
	#[pallet::storage]
	pub(super) type AuthorityIdToMinerId<T: Config> =
		StorageMap<_, Blake2_128Concat, T::MiningAuthorityId, T::AccountId, OptionQuery>;

	/// Argonots that must be locked to take a Miner role
	#[pallet::storage]
	pub(super) type ArgonotsPerMiningSeat<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	/// Lookup by account id to the corresponding index in ActiveMinersByIndex and Authorities
	#[pallet::storage]
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

	/// The number of bids per slot for the last 10 slots (newest first)
	#[pallet::storage]
	pub(super) type HistoricalBidsPerSlot<T: Config> =
		StorageValue<_, BoundedVec<MiningBidStats, ConstU32<10>>, ValueQuery>;

	/// The mining slot configuration set in genesis
	#[pallet::storage]
	pub(super) type MiningConfig<T: Config> = StorageValue<_, MiningSlotConfig, ValueQuery>;

	/// The current slot id
	#[pallet::storage]
	pub(super) type CurrentSlotId<T: Config> = StorageValue<_, SlotId, ValueQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub mining_config: MiningSlotConfig,
		#[serde(skip)]
		pub _phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if self.mining_config.slot_bidding_start_after_ticks == 0 {
				IsNextSlotBiddingOpen::<T>::put(true);
			}
			MiningConfig::<T>::put(self.mining_config.clone());
			ArgonotsPerMiningSeat::<T>::put(T::MinimumArgonotsPerSeat::get());
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
			obligation_id: Option<ObligationId>,
			preserved_argonot_hold: bool,
		},
		ReleasedMinerSeat {
			account_id: T::AccountId,
			obligation_id: Option<ObligationId>,
			preserved_argonot_hold: bool,
		},
		ReleaseMinerSeatError {
			account_id: T::AccountId,
			obligation_id: Option<ObligationId>,
			error: DispatchError,
		},
		MiningConfigurationUpdated {
			ticks_before_bid_end_for_vrf_close: Tick,
			ticks_between_slots: Tick,
			slot_bidding_start_after_ticks: Tick,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		SlotNotTakingBids,
		TooManyBlockRegistrants,
		InsufficientOwnershipTokens,
		BidTooLow,
		CannotRegisterOverlappingSessions,
		// copied from vault
		ObligationNotFound,
		NoMoreObligationIds,
		VaultClosed,
		MinimumObligationAmountNotMet,
		/// There are too many obligations expiring in the given expiration block
		ExpirationAtBlockOverflow,
		InsufficientFunds,
		InsufficientVaultFunds,
		ExpirationTooSoon,
		NoPermissions,
		HoldUnexpectedlyModified,
		UnrecoverableHold,
		VaultNotFound,
		AccountWouldBeBelowMinimum,
		GenericObligationError(ObligationError),
		/// Keys cannot be registered by multiple accounts
		CannotRegisterDuplicateKeys,
		/// Unable to decode the key format
		InvalidKeyFormat,
	}

	impl<T> From<ObligationError> for Error<T> {
		fn from(e: ObligationError) -> Error<T> {
			match e {
				ObligationError::ObligationNotFound => Error::<T>::ObligationNotFound,
				ObligationError::NoMoreObligationIds => Error::<T>::NoMoreObligationIds,
				ObligationError::MinimumObligationAmountNotMet =>
					Error::<T>::MinimumObligationAmountNotMet,
				ObligationError::ExpirationAtBlockOverflow => Error::<T>::ExpirationAtBlockOverflow,
				ObligationError::InsufficientFunds => Error::<T>::InsufficientFunds,
				ObligationError::InsufficientVaultFunds => Error::<T>::InsufficientVaultFunds,
				ObligationError::ExpirationTooSoon => Error::<T>::ExpirationTooSoon,
				ObligationError::NoPermissions => Error::<T>::NoPermissions,
				ObligationError::VaultClosed => Error::<T>::VaultClosed,
				ObligationError::HoldUnexpectedlyModified => Error::<T>::HoldUnexpectedlyModified,
				ObligationError::UnrecoverableHold => Error::<T>::UnrecoverableHold,
				ObligationError::VaultNotFound => Error::<T>::VaultNotFound,
				ObligationError::AccountWouldBeBelowMinimum =>
					Error::<T>::AccountWouldBeBelowMinimum,
				_ => Error::<T>::GenericObligationError(e),
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let current_slot_id = CurrentSlotId::<T>::get();
			let next_slot_id = Self::calculate_slot_id();
			if next_slot_id > current_slot_id {
				log::trace!("Starting Slot {}", next_slot_id);
				Self::adjust_argonots_per_seat();
				Self::start_new_slot(next_slot_id);
				return T::DbWeight::get().reads_writes(0, 2);
			}

			if current_slot_id == 0 &&
				!IsNextSlotBiddingOpen::<T>::get() &&
				Self::is_slot_bidding_started()
			{
				log::trace!(
					"Opening Slot 1 bidding {}",
					MiningConfig::<T>::get().slot_bidding_start_after_ticks
				);
				IsNextSlotBiddingOpen::<T>::put(true);
			}

			// rotate grandpas on off rotations
			let rotate_grandpa_blocks =
				UniqueSaturatedInto::<u32>::unique_saturated_into(T::GrandpaRotationBlocks::get());
			let current_block = UniqueSaturatedInto::<u32>::unique_saturated_into(block_number);
			if !HasAddedGrandpaRotation::<T>::get() || current_block % rotate_grandpa_blocks == 0 {
				T::SlotEvents::rotate_grandpas::<T::Keys>(current_slot_id, vec![], vec![]);
				HasAddedGrandpaRotation::<T>::put(true);
				return T::DbWeight::get().reads_writes(3, 2)
			}

			T::DbWeight::get().reads_writes(2, 0)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit a bid for a mining slot in the next cohort. Once all spots are filled in a slot,
		/// a slot can be supplanted by supplying a higher number of Bonded Argons. Bonded Argon
		/// terms can be found in the `vaults` pallet. You will supply the amount and the
		/// vault id to work with.
		///
		/// Each slot has `MaxCohortSize` spots available.
		///
		/// To be eligible for a slot, you must have the required ownership tokens in this account.
		/// The required amount is calculated as a percentage of the total ownership tokens in the
		/// network. This percentage is adjusted before the beginning of each slot.
		///
		/// If your bid is replaced, a `SlotBidderReplaced` event will be emitted. By monitoring for
		/// this event, you will be able to ensure your bid is accepted.
		///
		/// NOTE: bidding for each slot will be closed at a random block within
		/// `mining_config.ticks_before_bid_end_for_vrf_close` blocks of the slot end time.
		///
		/// The slot duration can be calculated as `BlocksBetweenSlots * MaxMiners / MaxCohortSize`.
		///
		/// Parameters:
		/// - `bonded_argons`: The information to submit for the bid. If `None`, the bid will be
		///  considered a zero-bid.
		/// 	- `vault_id`: The vault id to used. Terms are taken from the vault at time of bid
		///    inclusion in the block.
		///   	- `amount`: The amount to reserve from the vault.
		/// - `reward_destination`: The account_id for the mining rewards, or `Owner` for the
		///   submitting user.
		/// - `keys`: The session "hot" keys for the slot (BlockSealAuthorityId and GrandpaId).
		#[pallet::call_index(0)]
		#[pallet::weight(0)] //T::WeightInfo::hold())]
		pub fn bid(
			origin: OriginFor<T>,
			bonded_argons: Option<MiningSlotBid<VaultId, T::Balance>>,
			reward_destination: RewardDestination<T::AccountId>,
			keys: T::Keys,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(IsNextSlotBiddingOpen::<T>::get(), Error::<T>::SlotNotTakingBids);

			let miner_authority_id = keys
				.get::<T::MiningAuthorityId>(T::MiningAuthorityId::ID)
				.ok_or(Error::<T>::InvalidKeyFormat)?;
			if let Some(registrant) = AuthorityIdToMinerId::<T>::get(&miner_authority_id) {
				ensure!(registrant == who, Error::<T>::CannotRegisterDuplicateKeys);
			}
			let next_cohort_tick = Self::get_next_slot_tick();
			if let Some(current_index) = <AccountIndexLookup<T>>::get(&who) {
				let cohort_start_index = Self::get_next_slot_starting_index();
				let is_in_next_cohort = current_index >= cohort_start_index &&
					current_index < (cohort_start_index + T::MaxCohortSize::get());

				// current_index must be in the set of miners being replaced
				ensure!(is_in_next_cohort, Error::<T>::CannotRegisterOverlappingSessions);
			}

			let current_registration = Self::get_active_registration(&who);

			let mut obligation_id = None;
			let mut reward_sharing = None;
			let mut bid = 0u128.into();

			if let Some(bonded_argons) = bonded_argons {
				let reserve_until_tick = next_cohort_tick + Self::get_mining_window_ticks();
				let modify_obligation_id = NextSlotCohort::<T>::get()
					.iter()
					.find(|x| x.account_id == who)
					.and_then(|x| x.obligation_id);
				let obligation = T::BondedArgonsProvider::create_bonded_argons(
					bonded_argons.vault_id,
					who.clone(),
					bonded_argons.amount,
					reserve_until_tick,
					modify_obligation_id,
				)
				.map_err(Error::<T>::from)?;
				obligation_id = Some(obligation.0);
				reward_sharing = obligation.1;
				bid = obligation.2;
				// if the modified obligation id is not the obligation id we created, need to cancel
				// it
				if let Some(modify_obligation_id) = modify_obligation_id {
					if obligation.0 != modify_obligation_id {
						T::BondedArgonsProvider::cancel_bonded_argons(modify_obligation_id)
							.map_err(Error::<T>::from)?;
					}
				}
			}

			let ownership_tokens = Self::hold_argonots(&who, current_registration)?;
			let next_slot_id = CurrentSlotId::<T>::get().saturating_add(1);
			<NextSlotCohort<T>>::try_mutate(|cohort| -> DispatchResult {
				if let Some(existing_position) = cohort.iter().position(|x| x.account_id == who) {
					cohort.remove(existing_position);
				}

				// sort to lowest position at bid
				let pos = cohort
					.binary_search_by(|x| {
						let comp = bid.cmp(&x.bonded_argons);
						match comp {
							Ordering::Equal => Ordering::Less,
							Ordering::Greater => Ordering::Greater,
							Ordering::Less => Ordering::Less,
						}
					})
					.unwrap_or_else(|pos| pos);

				ensure!(pos < T::MaxCohortSize::get() as usize, Error::<T>::BidTooLow);

				if cohort.is_full() {
					// need to pop-off the lowest bid
					let entry = cohort.pop().expect("should exist, just checked");
					Self::release_failed_bid(entry)?;
				}

				cohort
					.try_insert(
						pos,
						MiningRegistration {
							account_id: who.clone(),
							reward_destination,
							obligation_id,
							bonded_argons: bid,
							argonots: ownership_tokens,
							reward_sharing,
							authority_keys: keys,
							slot_id: next_slot_id,
						},
					)
					.map_err(|_| Error::<T>::TooManyBlockRegistrants)?;

				HistoricalBidsPerSlot::<T>::mutate(|bids| {
					if let Some(bids) = bids.get_mut(0) {
						bids.bids_count += 1;
						bids.bid_amount_max = bids.bid_amount_max.max(bid.into());
						if bids.bids_count == 1 {
							bids.bid_amount_min = bid.into();
						}
						bids.bid_amount_min = bids.bid_amount_min.min(bid.into());
						bids.bid_amount_sum = bids.bid_amount_sum.saturating_add(bid.into());
					}
				});
				AuthorityIdToMinerId::<T>::insert(miner_authority_id.clone(), who.clone());
				Self::deposit_event(Event::<T>::SlotBidderAdded {
					account_id: who.clone(),
					bid_amount: bid,
					index: UniqueSaturatedInto::<u32>::unique_saturated_into(pos),
				});

				Ok(())
			})?;

			Ok(())
		}

		/// Admin function to update the mining slot delay.
		#[pallet::call_index(1)]
		#[pallet::weight(0)] //T::WeightInfo::hold())]
		pub fn configure_mining_slot_delay(
			origin: OriginFor<T>,
			mining_slot_delay: Tick,
		) -> DispatchResult {
			ensure_root(origin)?;

			MiningConfig::<T>::mutate(|a| {
				if a.slot_bidding_start_after_ticks != mining_slot_delay {
					a.slot_bidding_start_after_ticks = mining_slot_delay;
					Self::deposit_event(Event::<T>::MiningConfigurationUpdated {
						ticks_before_bid_end_for_vrf_close: a.ticks_before_bid_end_for_vrf_close,
						ticks_between_slots: a.ticks_between_slots,
						slot_bidding_start_after_ticks: a.slot_bidding_start_after_ticks,
					});
				}
			});

			Ok(())
		}
	}
}
impl<T: Config> BlockRewardAccountsProvider<T::AccountId> for Pallet<T> {
	fn get_rewards_account(
		author: &T::AccountId,
	) -> (Option<T::AccountId>, Option<RewardSharing<T::AccountId>>) {
		let Some(registration) = Self::get_active_registration(author) else {
			return (None, None);
		};

		let reward_account = match registration.reward_destination {
			RewardDestination::Owner => registration.account_id,
			RewardDestination::Account(reward_id) => reward_id,
		};
		(Some(reward_account), registration.reward_sharing.clone())
	}

	fn get_all_rewards_accounts() -> Vec<(T::AccountId, Option<RewardShare>)> {
		let mut result = vec![];
		for (_, registration) in <ActiveMinersByIndex<T>>::iter() {
			let account = match registration.reward_destination {
				RewardDestination::Owner => registration.account_id,
				RewardDestination::Account(reward_id) => reward_id,
			};
			if let Some(reward_sharing) = registration.reward_sharing {
				result.push((
					account,
					Some(FixedU128::one().saturating_sub(reward_sharing.percent_take)),
				));
				result.push((reward_sharing.account_id, Some(reward_sharing.percent_take)));
			} else {
				result.push((account, None));
			}
		}
		result
	}

	/// Compute blocks only get rewards prior to registered mining being active
	fn is_compute_block_eligible_for_rewards() -> bool {
		!Self::is_registered_mining_active()
	}
}

impl<T: Config> AuthorityProvider<T::MiningAuthorityId, T::Block, T::AccountId> for Pallet<T> {
	fn authority_count() -> u32 {
		ActiveMinersCount::<T>::get().into()
	}

	fn get_authority(author: T::AccountId) -> Option<T::MiningAuthorityId> {
		Self::get_mining_authority(&author).map(|x| x.authority_id)
	}

	fn xor_closest_authority(
		nonce: U256,
	) -> Option<MiningAuthority<T::MiningAuthorityId, T::AccountId>> {
		let closest = find_xor_closest(<AuthorityHashByIndex<T>>::get(), nonce)?;

		Self::get_mining_authority_by_index(closest)
	}
}

impl<T: Config> Pallet<T> {
	pub fn is_registered_mining_active() -> bool {
		CurrentSlotId::<T>::get() > 0 && ActiveMinersCount::<T>::get() > 0
	}

	pub fn get_mining_authority(
		account_id: &T::AccountId,
	) -> Option<MiningAuthority<T::MiningAuthorityId, T::AccountId>> {
		let index = <AccountIndexLookup<T>>::get(account_id)?;
		Self::get_mining_authority_by_index(index)
	}

	pub fn get_mining_authority_by_index(
		index: MinerIndex,
	) -> Option<MiningAuthority<T::MiningAuthorityId, T::AccountId>> {
		let miner = ActiveMinersByIndex::<T>::get(index)?;
		miner
			.authority_keys
			.get(T::MiningAuthorityId::ID)
			.map(|authority_id| MiningAuthority {
				authority_id,
				account_id: miner.account_id.clone(),
				authority_index: index.unique_saturated_into(),
			})
	}

	pub(crate) fn start_new_slot(slot_id: SlotId) {
		let max_miners = T::MaxMiners::get();
		let cohort_size = T::MaxCohortSize::get();

		HistoricalBidsPerSlot::<T>::mutate(|bids| {
			if bids.is_full() {
				bids.pop();
			}
			let _ = bids.try_insert(0, MiningBidStats::default());
		});

		let start_index_to_replace_miners =
			Self::get_slot_starting_index(slot_id, max_miners, cohort_size);

		let slot_cohort = NextSlotCohort::<T>::take();
		IsNextSlotBiddingOpen::<T>::put(true);
		let mut active_miners = ActiveMinersCount::<T>::get();
		let mut authority_hash_by_index = AuthorityHashByIndex::<T>::get();
		let mut added_miners = vec![];
		let mut removed_miners = vec![];

		for i in 0..cohort_size {
			let index = i + start_index_to_replace_miners;

			authority_hash_by_index.remove(&index);
			if let Some(entry) = ActiveMinersByIndex::<T>::take(index) {
				let account_id = entry.account_id.clone();
				AccountIndexLookup::<T>::remove(&account_id);
				active_miners -= 1;

				let registered_for_next = slot_cohort.iter().any(|x| x.account_id == account_id);
				if let Some(authority_id) =
					entry.authority_keys.get::<T::MiningAuthorityId>(T::MiningAuthorityId::ID)
				{
					AuthorityIdToMinerId::<T>::remove(authority_id);
				}
				removed_miners.push((account_id, entry.authority_keys.clone()));
				Self::release_mining_seat_obligations(entry, registered_for_next);
			}

			if let Some(entry) = slot_cohort.get(i as usize) {
				AccountIndexLookup::<T>::insert(&entry.account_id, index);
				active_miners += 1;
				ActiveMinersByIndex::<T>::insert(index, entry.clone());
				added_miners.push((entry.account_id.clone(), entry.authority_keys.clone()));
				if let Some(authority_id) =
					entry.authority_keys.get::<T::MiningAuthorityId>(T::MiningAuthorityId::ID)
				{
					let hash = blake2_256(&authority_id.to_raw_vec());
					authority_hash_by_index
						.try_insert(index, U256::from_big_endian(&hash))
						.expect("only insert if we've removed first, ergo, should be impossible");
				}
			}
		}

		<AuthorityHashByIndex<T>>::put(authority_hash_by_index);
		ActiveMinersCount::<T>::put(active_miners);

		Pallet::<T>::deposit_event(Event::<T>::NewMiners {
			start_index: start_index_to_replace_miners,
			new_miners: slot_cohort,
		});
		CurrentSlotId::<T>::put(slot_id);
		T::SlotEvents::rotate_grandpas(slot_id, removed_miners, added_miners);
	}

	/// Adjust the argonots per seat amount based on a rolling 10 slot average of bids.
	///
	/// This should be called before starting a new slot. It will adjust the argonots per seat
	/// amount based on the number of bids in the last 10 slots to reach the target number of bids
	/// per slot. The amount must also be adjusted based on the total ownership tokens in the
	/// network, which will increase in every block.
	///
	/// The max percent swing is 20% over the previous adjustment to the argonots per seat amount.
	pub(crate) fn adjust_argonots_per_seat() {
		let ownership_circulation: u128 = T::OwnershipCurrency::total_issuance().saturated_into();
		if ownership_circulation == 0 {
			return;
		}

		let historical_bids = HistoricalBidsPerSlot::<T>::get();
		let total_bids: u32 = historical_bids.iter().map(|a| a.bids_count).sum();

		let slots = historical_bids.len() as u32;
		let expected_bids_for_period = slots.saturating_mul(T::TargetBidsPerSlot::get());
		if expected_bids_for_period == 0 {
			return;
		}

		let base_ownership_tokens: u128 = ownership_circulation
			.checked_div(T::MaxMiners::get().into())
			.unwrap_or_default();

		let damper = T::ArgonotsPercentAdjustmentDamper::get();
		let one = FixedU128::one();
		let adjustment_percent =
			FixedU128::from_rational(total_bids as u128, expected_bids_for_period as u128)
				.clamp(one.saturating_sub(damper), one.saturating_add(damper));

		if adjustment_percent == FixedU128::one() {
			return;
		}
		let current = ArgonotsPerMiningSeat::<T>::get();

		let min_value = T::MinimumArgonotsPerSeat::get();
		// don't let this go below the minimum (it is in beginning)
		let max_value: T::Balance = T::MaximumArgonotProrataPercent::get()
			.mul_ceil(base_ownership_tokens)
			.unique_saturated_into();
		let mut argonots_needed = adjustment_percent.saturating_mul_int(current);
		if argonots_needed < min_value {
			argonots_needed = min_value;
		} else if argonots_needed > max_value {
			argonots_needed = max_value;
		}

		ArgonotsPerMiningSeat::<T>::put(argonots_needed.saturated_into::<T::Balance>());
	}

	/// Check if the current block is in the closing window for the next slot
	///
	/// This is determined by looking at the block seal vote and using the following VRF formula:
	///  `VRF = blake2(seal_strength)`
	/// If VRF < threshold, then the auction will be ended
	///
	/// The random seal strength is used to ensure that the VRF is unique for each block:
	///  - the block votes was submitted in a previous notebook
	///  - seal strength is the combination of the vote and the "voting key" (a hash of
	///    commit/reveal nonces supplied by each notary for a given tick).
	///  - this seal strength must be cryptographically secure and unique for each block for the
	///    overall network security
	///
	/// Threshold is calculated so that it should be true 1 in
	/// `MiningConfig.ticks_before_bid_end_for_vrf_close` times.
	pub(crate) fn check_for_bidding_close(seal: &BlockSealInherent) -> bool {
		let next_slot_tick = Self::get_next_slot_tick();
		let current_tick = T::TickProvider::current_tick();
		let mining_config = MiningConfig::<T>::get();

		// Are we in the closing eligibility window?
		if next_slot_tick.saturating_sub(current_tick) >
			mining_config.ticks_before_bid_end_for_vrf_close
		{
			return false;
		}

		match seal {
			BlockSealInherent::Vote { seal_strength, .. } => {
				let vrf_hash = blake2_256(seal_strength.encode().as_ref());
				let vrf = U256::from_big_endian(&vrf_hash);
				let ticks_before_close = mining_config.ticks_before_bid_end_for_vrf_close;
				// Calculate the threshold for VRF comparison to achieve a probability of 1 in
				// `MiningConfig.ticks_before_bid_end_for_vrf_close`
				let threshold = U256::MAX / U256::from(ticks_before_close);

				vrf < threshold
			},
			_ => false,
		}
	}

	pub fn slot_1_tick() -> Tick {
		let mining_config = MiningConfig::<T>::get();
		let slot_1_ticks =
			mining_config.slot_bidding_start_after_ticks + mining_config.ticks_between_slots;
		let genesis_tick =
			T::TickProvider::current_tick().saturating_sub(T::TickProvider::elapsed_ticks());
		genesis_tick.saturating_add(slot_1_ticks)
	}

	pub fn ticks_since_mining_start() -> Tick {
		T::TickProvider::current_tick().saturating_sub(Self::slot_1_tick())
	}

	pub(crate) fn get_next_slot_tick() -> Tick {
		Self::tick_for_slot(CurrentSlotId::<T>::get() + 1)
	}

	pub fn get_next_slot_era() -> (Tick, Tick) {
		let start_tick = Self::get_next_slot_tick();
		(start_tick, start_tick + Self::get_mining_window_ticks())
	}

	pub(crate) fn calculate_slot_id() -> SlotId {
		let mining_config = MiningConfig::<T>::get();
		let slot_1_tick_start =
			mining_config.slot_bidding_start_after_ticks + mining_config.ticks_between_slots;
		if T::TickProvider::elapsed_ticks() < slot_1_tick_start {
			return 0
		}
		let ticks_since_mining_start = Self::ticks_since_mining_start();
		let slot_id = ticks_since_mining_start / mining_config.ticks_between_slots;
		slot_id as SlotId + 1
	}

	pub fn tick_for_slot(slot_id: SlotId) -> Tick {
		if slot_id == 0 {
			// return genesis tick for slot 0
			return T::TickProvider::current_tick().saturating_sub(T::TickProvider::elapsed_ticks())
		}
		let slot_1_tick = Self::slot_1_tick();
		let added_ticks = (slot_id - 1) * Self::ticks_between_slots();
		slot_1_tick.saturating_add(added_ticks)
	}

	pub(crate) fn get_slot_starting_index(
		slot_id: SlotId,
		max_miners: u32,
		cohort_size: u32,
	) -> u32 {
		(slot_id as u32 * cohort_size) % max_miners
	}

	pub(crate) fn get_next_slot_starting_index() -> u32 {
		let current_slot_id = CurrentSlotId::<T>::get();
		let cohort_size = T::MaxCohortSize::get();

		Self::get_slot_starting_index(current_slot_id + 1, T::MaxMiners::get(), cohort_size)
	}

	pub fn get_mining_window_ticks() -> Tick {
		let miners = T::MaxMiners::get() as u64;
		let ticks_between_slots = Self::ticks_between_slots();
		let cohort_size = T::MaxCohortSize::get() as u64;

		miners.saturating_mul(ticks_between_slots) / cohort_size
	}

	pub(crate) fn get_active_registration(account_id: &T::AccountId) -> Option<Registration<T>> {
		if let Some(index) = AccountIndexLookup::<T>::get(account_id) {
			return ActiveMinersByIndex::<T>::get(index);
		}
		None
	}

	pub(crate) fn get_next_registration(account_id: &T::AccountId) -> Option<Registration<T>> {
		NextSlotCohort::<T>::get().into_iter().find(|x| x.account_id == *account_id)
	}

	pub(crate) fn hold_argonots(
		who: &T::AccountId,
		current_registration: Option<Registration<T>>,
	) -> Result<T::Balance, DispatchError> {
		let argonots = ArgonotsPerMiningSeat::<T>::get();
		let next_registration = Self::get_next_registration(who);
		let mut argonots_needed = argonots;

		// if we've already held for next, reduce now
		if let Some(next) = next_registration {
			argonots_needed -= next.argonots;
		} else if let Some(current_registration) = current_registration {
			argonots_needed -= current_registration.argonots;
		}

		if argonots_needed == 0u32.into() {
			return Ok(argonots);
		}

		let hold_reason = HoldReason::RegisterAsMiner;
		if T::OwnershipCurrency::balance_on_hold(&hold_reason.into(), who) == 0u32.into() {
			frame_system::Pallet::<T>::inc_providers(who);
		}

		T::OwnershipCurrency::hold(&hold_reason.into(), who, argonots_needed)
			.map_err(|_| Error::<T>::InsufficientOwnershipTokens)?;
		Ok(argonots)
	}

	pub(crate) fn release_failed_bid(registration: Registration<T>) -> DispatchResult {
		let account_id = registration.account_id;

		if let Some(obligation_id) = registration.obligation_id {
			T::BondedArgonsProvider::cancel_bonded_argons(obligation_id)
				.map_err(Error::<T>::from)?;
		}

		let mut held_argonots = false;
		let mut amount_to_unhold: T::Balance = registration.argonots;
		if let Some(active) = Self::get_active_registration(&account_id) {
			amount_to_unhold -= active.argonots;
			held_argonots = true;
		} else if let Some(authority_id) = registration
			.authority_keys
			.get::<T::MiningAuthorityId>(T::MiningAuthorityId::ID)
		{
			AuthorityIdToMinerId::<T>::remove(authority_id);
		}

		Self::release_argonots_hold(&account_id, amount_to_unhold)?;

		Self::deposit_event(Event::<T>::SlotBidderReplaced {
			account_id: account_id.clone(),
			obligation_id: registration.obligation_id,
			preserved_argonot_hold: held_argonots,
		});

		Ok(())
	}

	fn release_argonots_hold(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult {
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

	/// Release the argonots from the mining seat. If the Argonots will be re-used in the next
	/// era, we should not unlock it
	pub(crate) fn release_mining_seat_obligations(
		active_registration: Registration<T>,
		is_registered_for_next: bool,
	) {
		let account_id = active_registration.account_id;
		let active_obligation_id = active_registration.obligation_id;

		let mut preserved_argonot_hold = true;
		if !is_registered_for_next {
			preserved_argonot_hold = false;

			if let Err(e) = Self::release_argonots_hold(&account_id, active_registration.argonots) {
				log::error!("Failed to release argonots from account {:?}. {:?}", account_id, e,);
				Self::deposit_event(Event::<T>::ReleaseMinerSeatError {
					account_id: account_id.clone(),
					obligation_id: active_obligation_id,
					error: e,
				});
				return;
			}
		}

		Self::deposit_event(Event::<T>::ReleasedMinerSeat {
			account_id: account_id.clone(),
			obligation_id: active_obligation_id,
			preserved_argonot_hold,
		});
	}

	fn ticks_between_slots() -> Tick {
		MiningConfig::<T>::get().ticks_between_slots
	}
}

impl<T: Config> MiningSlotProvider for Pallet<T> {
	fn get_next_slot_tick() -> Tick {
		Self::get_next_slot_tick()
	}

	fn mining_window_ticks() -> Tick {
		Self::get_mining_window_ticks()
	}

	fn is_slot_bidding_started() -> bool {
		T::TickProvider::elapsed_ticks() >= MiningConfig::<T>::get().slot_bidding_start_after_ticks
	}
}

impl<T: Config> BlockSealEventHandler for Pallet<T> {
	fn block_seal_read(seal: &BlockSealInherent) {
		// If bids are open, and we're in the closing-period, check if bidding should close.
		// NOTE: This should run first to ensure bids in this block can't be manipulated once
		// this state is known
		if IsNextSlotBiddingOpen::<T>::get() && Self::check_for_bidding_close(seal) {
			IsNextSlotBiddingOpen::<T>::put(false);
		}
	}
}

pub fn find_xor_closest<I>(authorities: I, hash: U256) -> Option<MinerIndex>
where
	I: IntoIterator<Item = (MinerIndex, U256)>,
{
	let mut closest_distance: U256 = U256::MAX;
	let mut closest = None;
	for (index, peer_hash) in authorities.into_iter() {
		let distance = hash ^ peer_hash;
		if distance < closest_distance {
			closest_distance = distance;
			closest = Some(index);
		}
	}
	closest
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

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::MiningAuthorityId;
}

sp_api::decl_runtime_apis! {
	/// This runtime api allows people to query the upcoming mining_slot
	pub trait MiningSlotApi {
		fn next_slot_era() -> (Tick, Tick);
	}
}

pub trait OnNewSlot<AccountId> {
	type Key: Decode + RuntimeAppPublic;
	fn rotate_grandpas(
		current_slot_id: SlotId,
		removed_authorities: Vec<(&AccountId, Self::Key)>,
		added_authorities: Vec<(&AccountId, Self::Key)>,
	);
}

pub trait SlotEvents<AccountId> {
	fn rotate_grandpas<Ks: OpaqueKeys>(
		current_slot_id: SlotId,
		removed_authorities: Vec<(AccountId, Ks)>,
		added_authorities: Vec<(AccountId, Ks)>,
	);
}

#[impl_trait_for_tuples::impl_for_tuples(0, 5)]
#[tuple_types_custom_trait_bound(OnNewSlot<AId>)]
impl<AId> SlotEvents<AId> for Tuple {
	fn rotate_grandpas<Ks: OpaqueKeys>(
		current_slot_id: SlotId,
		removed_authorities: Vec<(AId, Ks)>,
		added_authorities: Vec<(AId, Ks)>,
	) {
		for_tuples!(
		#(
			let removed_keys =
				removed_authorities.iter().filter_map(|k| {
					k.1.get::<Tuple::Key>(<Tuple::Key as RuntimeAppPublic>::ID).map(|k1| (&k.0, k1))
				}).collect::<Vec<_>>();
			let added_keys  =
				added_authorities.iter().filter_map(|k| {
					k.1.get::<Tuple::Key>(<Tuple::Key as RuntimeAppPublic>::ID).map(|k1| (&k.0, k1))
				}).collect::<Vec<_>>();
			Tuple::rotate_grandpas(current_slot_id, removed_keys, added_keys);
		)*
		)
	}
}
