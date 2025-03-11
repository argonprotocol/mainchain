#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

use alloc::{vec, vec::Vec};
use argon_primitives::{
	block_seal::{
		CohortId, MinerIndex, MiningAuthority, MiningBidStats, MiningSlotConfig, RewardDestination,
	},
	inherents::BlockSealInherent,
	tick::Tick,
	vault::BondedBitcoinsBidPoolProvider,
	AuthorityProvider, BlockRewardAccountsProvider, BlockSealEventHandler, MiningSlotProvider,
	SlotEvents, TickProvider,
};
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{Inspect, InspectHold, Mutate, MutateHold},
		tokens::{Fortitude, Precision, Preservation},
	},
};
use log::info;
pub use pallet::*;
use sp_core::{Get, U256};
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{One, OpaqueKeys, UniqueSaturatedInto, Zero},
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
/// `Cohort` of miners to operate for a given number of ticks (the `MiningWindow`).
///
/// New miner slots are rotated in every `mining_config.ticks_between_slots` ticks. Each cohort
/// will have `MaxCohortSize` members. A maximum of `MaxMiners` will be active at any given time.
///
/// When a new Slot begins, the Miners with the corresponding Slot Indices will be replaced with
/// the new cohort members (or emptied out). A Slot Index is similar to a Mining "Seat", but
/// 0-based.
///
/// To be eligible for mining, you must reserve a percent of the total supply of argonots (ownership
/// tokens). The percent is configured to aim for `TargetBidsPerSlot`, with a
/// maximum change in ownership tokens needed per slot capped at `ArgonotsPercentAdjustmentDamper`
/// (NOTE: this percent is the max increase or reduction in the amount of ownership issued).
///
/// You can out-bid others for cohort membership by submitting a bid amount in `BidIncrements` argon
/// to supplant other entrants (eg, 1.01 argon, 1.02 argons, etc.).
///
/// ### Registration
/// To register for a Slot, you must submit a bid. At any given time, only the next Slot is being
/// bid on. Bids are eligible at 1 argon increments. If you are outbid, your funds are returned
/// immediately. Once bidding ends, the winning bids are distributed to participating Vaults with
/// BondedBitcoins.
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use codec::FullCodec;
	use core::cmp::Ordering;
	use frame_support::{
		pallet_prelude::*,
		traits::fungible::{Inspect, Mutate, MutateHold},
		BoundedVec,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, Member, OpaqueKeys, UniqueSaturatedInto},
		BoundedBTreeMap, Percent,
	};

	use super::*;
	use argon_primitives::{
		block_seal::{MiningRegistration, RewardDestination},
		vault::BondedBitcoinsBidPoolProvider,
		SlotEvents, TickProvider,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

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
			+ Into<u128>
			+ TypeInfo
			+ MaxEncodedLen;

		/// The currency representing ownership (argonots) in the network - aka, rights to validate
		type OwnershipCurrency: MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>
			+ Inspect<Self::AccountId, Balance = Self::Balance>;

		/// The currency representing argons
		type ArgonCurrency: Mutate<Self::AccountId, Balance = Self::Balance>;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		type BidPoolProvider: BondedBitcoinsBidPoolProvider<
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

		/// The increment that bids can be on (for instance, one cent increments)
		type BidIncrements: Get<Self::Balance>;
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

	/// The last activated cohort id
	#[pallet::storage]
	pub(super) type LastActivatedCohortId<T: Config> = StorageValue<_, CohortId, ValueQuery>;

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
			cohort_id: CohortId,
		},
		SlotBidderAdded {
			account_id: T::AccountId,
			bid_amount: T::Balance,
			index: u32,
		},
		SlotBidderDropped {
			account_id: T::AccountId,
			preserved_argonot_hold: bool,
		},
		ReleasedMinerSeat {
			account_id: T::AccountId,
			preserved_argonot_hold: bool,
		},
		ReleaseMinerSeatError {
			account_id: T::AccountId,
			error: DispatchError,
		},
		MiningConfigurationUpdated {
			ticks_before_bid_end_for_vrf_close: Tick,
			ticks_between_slots: Tick,
			slot_bidding_start_after_ticks: Tick,
		},
		/// Bids are closed due to the VRF randomized function triggering
		MiningBidsClosed {
			cohort_id: CohortId,
		},
		ReleaseBidError {
			account_id: T::AccountId,
			error: DispatchError,
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
		NoPermissions,
		HoldUnexpectedlyModified,
		UnrecoverableHold,
		VaultNotFound,
		AccountWouldBeBelowMinimum,
		/// Keys cannot be registered by multiple accounts
		CannotRegisterDuplicateKeys,
		/// Unable to decode the key format
		InvalidKeyFormat,
		/// The mining bid cannot be reduced
		BidCannotBeReduced,
		/// Bids must be in allowed increments
		InvalidBidAmount,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let current_cohort_id = LastActivatedCohortId::<T>::get();
			let next_cohort_id = Self::calculate_cohort_id();
			if next_cohort_id > current_cohort_id {
				log::trace!("Starting Slot {}", next_cohort_id);
				Self::adjust_argonots_per_seat();
				Self::start_new_slot(next_cohort_id);
				return T::DbWeight::get().reads_writes(0, 2);
			}

			if current_cohort_id == 0 &&
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
				T::SlotEvents::rotate_grandpas::<T::Keys>(current_cohort_id, vec![], vec![]);
				HasAddedGrandpaRotation::<T>::put(true);
				return T::DbWeight::get().reads_writes(3, 2)
			}

			T::DbWeight::get().reads_writes(2, 0)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit a bid for a mining slot in the next cohort. Once all spots are filled in a slot,
		/// a slot can be supplanted by supplying a higher bid.
		///
		/// Each slot has `MaxCohortSize` spots available.
		///
		/// To be eligible for a slot, you must have the required ownership tokens (argonots) in
		/// this account. The required amount is calculated as a percentage of the total ownership
		/// tokens in the network. This percentage is adjusted before the beginning of each slot.
		///
		/// If your bid is no longer winning, a `SlotBidderOut` event will be emitted. By monitoring
		/// for this event, you will be able to ensure your bid is accepted.
		///
		/// NOTE: bidding for each slot will be closed at a random block within
		/// `mining_config.ticks_before_bid_end_for_vrf_close` blocks of the slot end time.
		///
		/// The slot duration can be calculated as `BlocksBetweenSlots * MaxMiners / MaxCohortSize`.
		///
		/// Parameters:
		/// - `bid`: The amount of argons to bid
		/// - `reward_destination`: The account_id for the mining rewards, or `Owner` for the
		///   submitting user.
		/// - `keys`: The session "hot" keys for the slot (BlockSealAuthorityId and GrandpaId).
		/// - `mining_account_id`: This account_id allows you to operate as this miner account id,
		///   but use funding (argonots and bid) from the submitting account
		#[pallet::call_index(0)]
		#[pallet::weight(0)] //T::WeightInfo::hold())]
		pub fn bid(
			origin: OriginFor<T>,
			bid: T::Balance,
			reward_destination: RewardDestination<T::AccountId>,
			keys: T::Keys,
			mining_account_id: Option<T::AccountId>,
		) -> DispatchResult {
			let funding_account = ensure_signed(origin)?;
			let miner_account_id = mining_account_id.unwrap_or(funding_account.clone());

			ensure!(IsNextSlotBiddingOpen::<T>::get(), Error::<T>::SlotNotTakingBids);
			ensure!(
				bid % T::BidIncrements::get() == T::Balance::zero(),
				Error::<T>::InvalidBidAmount
			);

			let miner_authority_id = keys
				.get::<T::MiningAuthorityId>(T::MiningAuthorityId::ID)
				.ok_or(Error::<T>::InvalidKeyFormat)?;
			if let Some(registrant) = AuthorityIdToMinerId::<T>::get(&miner_authority_id) {
				ensure!(registrant == miner_account_id, Error::<T>::CannotRegisterDuplicateKeys);
			}

			if let Some(current_index) = <AccountIndexLookup<T>>::get(&miner_account_id) {
				let cohort_start_index = Self::get_next_slot_starting_index();
				let is_in_next_cohort = current_index >= cohort_start_index &&
					current_index < (cohort_start_index + T::MaxCohortSize::get());

				// current_index must be in the set of miners being replaced
				ensure!(is_in_next_cohort, Error::<T>::CannotRegisterOverlappingSessions);
			}

			Self::send_argons_to_pool(&funding_account, bid)?;

			let current_registration = Self::get_active_registration(&miner_account_id);
			let ownership_tokens = Self::hold_argonots(&funding_account, current_registration)?;
			let next_cohort_id = LastActivatedCohortId::<T>::get().saturating_add(1);
			<NextSlotCohort<T>>::try_mutate(|cohort| -> DispatchResult {
				if let Some(existing_position) =
					cohort.iter().position(|x| x.account_id == miner_account_id)
				{
					cohort.remove(existing_position);
				}

				// sort to lowest position at bid
				let pos = cohort
					.binary_search_by(|x| {
						let comp = bid.cmp(&x.bid);
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
					Self::release_failed_bid(&entry)?;
				}

				cohort
					.try_insert(
						pos,
						MiningRegistration {
							account_id: miner_account_id.clone(),
							reward_destination,
							external_funding_account: if miner_account_id == funding_account {
								None
							} else {
								Some(funding_account)
							},
							bid,
							argonots: ownership_tokens,
							authority_keys: keys,
							cohort_id: next_cohort_id,
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
				AuthorityIdToMinerId::<T>::insert(
					miner_authority_id.clone(),
					miner_account_id.clone(),
				);
				Self::deposit_event(Event::<T>::SlotBidderAdded {
					account_id: miner_account_id.clone(),
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
			mining_slot_delay: Option<Tick>,
			ticks_before_bid_end_for_vrf_close: Option<Tick>,
		) -> DispatchResult {
			ensure_root(origin)?;

			MiningConfig::<T>::mutate(|a| {
				let mut has_update = false;
				if let Some(ticks_before_bid_end_for_vrf_close) = ticks_before_bid_end_for_vrf_close
				{
					if a.ticks_before_bid_end_for_vrf_close != ticks_before_bid_end_for_vrf_close {
						a.ticks_before_bid_end_for_vrf_close = ticks_before_bid_end_for_vrf_close;
						has_update = true;
					}
				}

				if let Some(mining_slot_delay) = mining_slot_delay {
					if a.slot_bidding_start_after_ticks != mining_slot_delay {
						a.slot_bidding_start_after_ticks = mining_slot_delay;
						has_update = true;
					}
				}

				if has_update {
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
	fn get_rewards_account(author: &T::AccountId) -> Option<(T::AccountId, CohortId)> {
		let registration = Self::get_active_registration(author)?;

		let reward_account = match registration.reward_destination {
			RewardDestination::Owner => registration.account_id,
			RewardDestination::Account(reward_id) => reward_id,
		};
		Some((reward_account, registration.cohort_id))
	}

	fn get_all_rewards_accounts() -> Vec<T::AccountId> {
		let mut result = vec![];
		for (_, registration) in <ActiveMinersByIndex<T>>::iter() {
			let account = match registration.reward_destination {
				RewardDestination::Owner => registration.account_id,
				RewardDestination::Account(reward_id) => reward_id,
			};
			result.push(account);
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
		seal_proof: U256,
	) -> Option<MiningAuthority<T::MiningAuthorityId, T::AccountId>> {
		let closest = find_xor_closest(<AuthorityHashByIndex<T>>::get(), seal_proof)?;

		Self::get_mining_authority_by_index(closest)
	}
}

impl<T: Config> Pallet<T> {
	pub fn is_registered_mining_active() -> bool {
		LastActivatedCohortId::<T>::get() > 0 && ActiveMinersCount::<T>::get() > 0
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

	pub(crate) fn start_new_slot(cohort_id: CohortId) {
		let max_miners = T::MaxMiners::get();
		let cohort_size = T::MaxCohortSize::get();

		HistoricalBidsPerSlot::<T>::mutate(|bids| {
			if bids.is_full() {
				bids.pop();
			}
			let _ = bids.try_insert(0, MiningBidStats::default());
		});

		let start_index_to_replace_miners =
			Self::get_slot_starting_index(cohort_id, max_miners, cohort_size);

		let slot_cohort = NextSlotCohort::<T>::take();

		// if we never vrf closed, we need to rotate the pool
		let needs_vault_bid_pool_rotation = IsNextSlotBiddingOpen::<T>::get();

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
			cohort_id,
		});
		LastActivatedCohortId::<T>::put(cohort_id);
		T::SlotEvents::rotate_grandpas(cohort_id, removed_miners, added_miners);
		T::SlotEvents::on_new_cohort(cohort_id);
		if needs_vault_bid_pool_rotation {
			let end_tick = T::TickProvider::current_tick() + Self::mining_window_ticks();
			T::BidPoolProvider::distribute_and_rotate_bid_pool(cohort_id, end_tick);
		}
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
	///  `VRF = blake2(seal_proof)`
	/// If VRF < threshold, then the auction will be ended
	///
	/// The random seal strength is used to ensure that the VRF is unique for each block:
	///  - the block votes was submitted in a previous notebook
	///  - seal proof is the combination of the vote and the "voting key" (a hash of commit/reveal
	///    nonces supplied by each notary for a given tick).
	///  - this seal proof must be cryptographically secure and unique for each block for the
	///    overall network security
	///
	/// Threshold is calculated so that it should be true 1 in
	/// `MiningConfig.ticks_before_bid_end_for_vrf_close` times.
	///
	/// NOTE: seal_strength should not be used as it is a non-uniform distributed value (must be
	/// seal_proof)
	pub(crate) fn check_for_bidding_close(vote_seal_proof: U256) -> bool {
		let next_slot_tick = Self::get_next_slot_tick();
		let current_tick = T::TickProvider::current_tick();
		let mining_config = MiningConfig::<T>::get();

		// Are we in the closing eligibility window?
		if next_slot_tick.saturating_sub(current_tick) >
			mining_config.ticks_before_bid_end_for_vrf_close
		{
			return false;
		}

		let ticks_before_close = mining_config.ticks_before_bid_end_for_vrf_close;
		// Calculate the threshold for VRF comparison to achieve a probability of 1 in
		// `MiningConfig.ticks_before_bid_end_for_vrf_close`
		let threshold = U256::MAX / U256::from(ticks_before_close);

		if vote_seal_proof < threshold {
			info!("VRF Close triggered: {:?} < {:?}", vote_seal_proof, threshold);
			let cohort_id = LastActivatedCohortId::<T>::get() + 1;
			Self::deposit_event(Event::<T>::MiningBidsClosed { cohort_id });
            let end_tick = Self::get_next_slot_tick() + Self::mining_window_ticks();
            T::BidPoolProvider::distribute_and_rotate_bid_pool(cohort_id, end_tick);
            return true
		}

		false
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
		Self::tick_for_slot(LastActivatedCohortId::<T>::get() + 1)
	}

	pub fn get_next_slot_era() -> (Tick, Tick) {
		let start_tick = Self::get_next_slot_tick();
		(start_tick, start_tick + Self::get_mining_window_ticks())
	}

	pub(crate) fn calculate_cohort_id() -> CohortId {
		let mining_config = MiningConfig::<T>::get();
		let slot_1_tick_start =
			mining_config.slot_bidding_start_after_ticks + mining_config.ticks_between_slots;
		if T::TickProvider::elapsed_ticks() < slot_1_tick_start {
			return 0
		}
		let ticks_since_mining_start = Self::ticks_since_mining_start();
		let cohort_id = ticks_since_mining_start / mining_config.ticks_between_slots;
		cohort_id as CohortId + 1
	}

	pub fn tick_for_slot(cohort_id: CohortId) -> Tick {
		if cohort_id == 0 {
			// return genesis tick for slot 0
			return T::TickProvider::current_tick().saturating_sub(T::TickProvider::elapsed_ticks())
		}
		let slot_1_tick = Self::slot_1_tick();
		let added_ticks = (cohort_id - 1) * Self::ticks_between_slots();
		slot_1_tick.saturating_add(added_ticks)
	}

	pub(crate) fn get_slot_starting_index(
		cohort_id: CohortId,
		max_miners: u32,
		cohort_size: u32,
	) -> u32 {
		(cohort_id as u32 * cohort_size) % max_miners
	}

	pub(crate) fn get_next_slot_starting_index() -> u32 {
		let current_cohort_id = LastActivatedCohortId::<T>::get();
		let cohort_size = T::MaxCohortSize::get();

		Self::get_slot_starting_index(current_cohort_id + 1, T::MaxMiners::get(), cohort_size)
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

	pub(crate) fn get_pending_cohort_registration(
		account_id: &T::AccountId,
	) -> Option<Registration<T>> {
		NextSlotCohort::<T>::get().into_iter().find(|x| x.account_id == *account_id)
	}

	pub(crate) fn hold_argonots(
		who: &T::AccountId,
		current_registration: Option<Registration<T>>,
	) -> Result<T::Balance, DispatchError> {
		let argonots = ArgonotsPerMiningSeat::<T>::get();
		let next_registration = Self::get_pending_cohort_registration(who);
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

	pub(crate) fn send_argons_to_pool(
		miner_funding_account: &T::AccountId,
		bid_amount: T::Balance,
	) -> Result<T::Balance, DispatchError> {
		let bid_registration = Self::get_pending_cohort_registration(miner_funding_account);
		let mut needed = bid_amount;

		// if we've already held for next, reduce now
		if let Some(existing) = bid_registration {
			ensure!(bid_amount > existing.bid, Error::<T>::BidCannotBeReduced);
			needed -= existing.bid;
		}
		if needed > 0u32.into() {
			let pool_account = T::BidPoolProvider::get_bid_pool_account();
			ensure!(
				T::ArgonCurrency::reducible_balance(
					miner_funding_account,
					Preservation::Preserve,
					Fortitude::Force
				) >= needed,
				Error::<T>::InsufficientFunds
			);
			T::ArgonCurrency::transfer(
				miner_funding_account,
				&pool_account,
				needed,
				Preservation::Preserve,
			)?;
		}
		Ok(needed)
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

		let funding_account =
			active_registration.external_funding_account.unwrap_or(account_id.clone());

		let mut preserved_argonot_hold = true;
		if !is_registered_for_next {
			preserved_argonot_hold = false;

			if let Some(authority_id) = active_registration
				.authority_keys
				.get::<T::MiningAuthorityId>(T::MiningAuthorityId::ID)
			{
				AuthorityIdToMinerId::<T>::remove(authority_id);
			}

			if let Err(e) =
				Self::release_argonots_hold(&funding_account, active_registration.argonots)
			{
				log::error!(
					"Failed to release argonots from funding account {:?} (account {:?}). {:?}",
					funding_account,
					account_id,
					e,
				);
				Self::deposit_event(Event::<T>::ReleaseMinerSeatError {
					account_id: account_id.clone(),
					error: e,
				});
				return;
			}
		}

		Self::deposit_event(Event::<T>::ReleasedMinerSeat {
			account_id: account_id.clone(),
			preserved_argonot_hold,
		});
	}

	pub(crate) fn release_failed_bid(registration: &Registration<T>) -> DispatchResult {
		let funding_account = registration
			.external_funding_account
			.clone()
			.unwrap_or(registration.account_id.clone());
		let account_id = registration.account_id.clone();

		if registration.bid > T::Balance::zero() {
			let pool_account = T::BidPoolProvider::get_bid_pool_account();
			T::ArgonCurrency::transfer(
				&pool_account,
				&funding_account,
				registration.bid,
				Preservation::Expendable,
			)?;
		}

		let mut held_argonots = false;
		let mut argonots_to_unhold: T::Balance = registration.argonots;
		if let Some(active) = Self::get_active_registration(&account_id) {
			argonots_to_unhold -= active.argonots;
			held_argonots = true;
		} else if let Some(authority_id) = registration
			.authority_keys
			.get::<T::MiningAuthorityId>(T::MiningAuthorityId::ID)
		{
			AuthorityIdToMinerId::<T>::remove(authority_id);
		}

		Self::release_argonots_hold(&funding_account, argonots_to_unhold)?;

		Self::deposit_event(Event::<T>::SlotBidderDropped {
			account_id,
			preserved_argonot_hold: held_argonots,
		});

		Ok(())
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
		T::TickProvider::elapsed_ticks() >= MiningConfig::<T>::get().slot_bidding_start_after_ticks ||
			IsNextSlotBiddingOpen::<T>::get()
	}
}

impl<T: Config> BlockSealEventHandler for Pallet<T> {
	fn block_seal_read(seal: &BlockSealInherent, vote_seal_proof: Option<U256>) {
		if !matches!(seal, BlockSealInherent::Vote { .. }) {
			return
		}
		// If bids are open, and we're in the closing-period, check if bidding should close.
		// NOTE: This should run first to ensure bids in this block can't be manipulated once
		// this state is known
		if let Some(proof) = vote_seal_proof {
			if IsNextSlotBiddingOpen::<T>::get() && Self::check_for_bidding_close(proof) {
				IsNextSlotBiddingOpen::<T>::put(false);
			}
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

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::MiningAuthorityId;
}

sp_api::decl_runtime_apis! {
	/// This runtime api allows people to query the upcoming mining_slot
	pub trait MiningSlotApi {
		fn next_slot_era() -> (Tick, Tick);
	}
}
