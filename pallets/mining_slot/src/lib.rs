#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;

use argon_primitives::{
	SlotEvents, TickProvider,
	block_seal::{MinerIndex, MiningAuthority, MiningBidStats, MiningSlotConfig},
	inherents::BlockSealInherent,
	providers::*,
	vault::MiningBidPoolProvider,
};
pub use pallet::*;
use pallet_prelude::*;
use sp_runtime::{RuntimeAppPublic, traits::OpaqueKeys};
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
/// `Cohort` of miners to operate for a given number of `Frames` (the period between bidding start,
/// and the start of mining). The frames that a miner operates in are determined by
/// `config.FramesPerMiningTerm`.
///
/// New miner slots are rotated in every `mining_config.ticks_between_slots` ticks. Each cohort
/// will have `NextCohortSize` members, a number that can be a min of `config.MinCohortSize` and a
/// max of `config.MaxCohortSize`. A maximum of `MaxMiners` will be active at any given time.
///
/// When a new Slot begins, the Miners from `config.FramesPerMiningTerm` will be retired.
///
/// To be eligible for mining, you must reserve a percentage of the argonot issuance (ownership
/// tokens). The percentage is configured to aim for `TargetBidsPerSlot`, with a
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
/// Treasury.
#[frame_support::pallet]
pub mod pallet {
	use codec::FullCodec;
	use core::cmp::Ordering;
	use sp_runtime::{BoundedBTreeMap, Percent};

	use super::*;
	use argon_primitives::{
		SlotEvents, TickProvider, block_seal::MiningRegistration, vault::MiningBidPoolProvider,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(7);

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
	pub trait Config: polkadot_sdk::frame_system::Config
	where
		<Self as Config>::Balance: Into<u128>,
	{
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// The number of frames a miner operates for
		#[pallet::constant]
		type FramesPerMiningTerm: Get<u32>;
		/// The minimum number of miners per cohort
		#[pallet::constant]
		type MinCohortSize: Get<u32>;
		/// How many new miners can be in the cohort for each slot. The actual maximum will adjust
		/// dynamically
		#[pallet::constant]
		type MaxCohortSize: Get<u32>;

		/// The max percent swing for the argonots per slot (from the last percent)
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

		/// The target price per seat.
		#[pallet::constant]
		type TargetPricePerSeat: Get<Self::Balance>;

		/// The damper on the price per seat adjustment (from the last price)
		#[pallet::constant]
		type PricePerSeatAdjustmentDamper: Get<FixedU128>;

		/// The balance type
		type Balance: AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ DecodeWithMemTracking
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

		type BidPoolProvider: MiningBidPoolProvider<Balance = Self::Balance, AccountId = Self::AccountId>;
		/// Handler when a new slot is started
		type SlotEvents: SlotEvents<Self::AccountId>;

		/// How often to rotate grandpas
		type GrandpaRotationBlocks: Get<BlockNumberFor<Self>>;

		/// The mining authority runtime public key
		type MiningAuthorityId: RuntimeAppPublic + FullCodec + Clone + TypeInfo + PartialEq;

		/// The authority signing keys.
		type Keys: OpaqueKeys + Member + Parameter + MaybeSerializeDeserialize + MaxEncodedLen;

		/// The current tick
		type TickProvider: TickProvider<Self::Block>;

		/// The increment that bids can be on (for instance, one cent increments)
		#[pallet::constant]
		type BidIncrements: Get<Self::Balance>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		#[codec(index = 0)]
		RegisterAsMiner,
	}

	#[pallet::storage]
	pub type HasAddedGrandpaRotation<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Miners that are active in the current block (post initialize) by their starting frame
	#[pallet::storage]
	pub type MinersByCohort<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		FrameId,
		BoundedVec<Registration<T>, T::MaxCohortSize>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type ActiveMinersCount<T: Config> = StorageValue<_, u16, ValueQuery>;

	/// This is a lookup of each miner's XOR key to use. It's a blake2 256 hash of the miner account
	/// id and the block hash at time of activation.
	#[pallet::storage]
	pub type MinerXorKeysByCohort<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<FrameId, BoundedVec<U256, T::MaxCohortSize>, T::FramesPerMiningTerm>,
		ValueQuery,
	>;

	/// Argonots that must be locked to take a Miner role
	#[pallet::storage]
	pub type ArgonotsPerMiningSeat<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	/// Lookup by account id to the corresponding index in MinersByCohort and MinerXorKeysByCohort
	#[pallet::storage]
	pub type AccountIndexLookup<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, MinerIndex, OptionQuery>;

	/// The cohort set to go into effect in the next slot. The Vec has all
	/// registrants with their bid amount
	#[pallet::storage]
	pub type BidsForNextSlotCohort<T: Config> =
		StorageValue<_, BoundedVec<Registration<T>, T::MaxCohortSize>, ValueQuery>;

	/// The miners released in the last block (only kept for a single block)
	#[pallet::storage]
	pub type ReleasedMinersByAccountId<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<T::AccountId, Registration<T>, T::MaxCohortSize>,
		ValueQuery,
	>;

	/// Is the next slot still open for bids
	#[pallet::storage]
	pub type IsNextSlotBiddingOpen<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The number of bids per slot for the last 10 slots (newest first)
	#[pallet::storage]
	pub type HistoricalBidsPerSlot<T: Config> =
		StorageValue<_, BoundedVec<MiningBidStats, ConstU32<10>>, ValueQuery>;

	/// The average price per seat for the last 10 frames (newest first)
	#[pallet::storage]
	pub type AveragePricePerSeat<T: Config> =
		StorageValue<_, BoundedVec<T::Balance, ConstU32<10>>, ValueQuery>;

	/// The mining slot configuration set in genesis
	#[pallet::storage]
	pub type MiningConfig<T: Config> = StorageValue<_, MiningSlotConfig, ValueQuery>;

	/// The next frameId. A frame in argon is the 24 hours between the start of two different mining
	/// cohorts.
	#[pallet::storage]
	pub type NextFrameId<T: Config> = StorageValue<_, FrameId, ValueQuery>;

	/// The number of allow miners to bid for the next mining cohort
	#[pallet::storage]
	pub type NextCohortSize<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Did this block activate a new frame
	#[pallet::storage]
	pub type DidStartNewCohort<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// The previous 10 frame start block numbers
	#[pallet::storage]
	pub type FrameStartBlockNumbers<T: Config> =
		StorageValue<_, BoundedVec<BlockNumberFor<T>, ConstU32<10>>, ValueQuery>;

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
			NextFrameId::<T>::put(1);
			NextCohortSize::<T>::put(T::MinCohortSize::get());
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewMiners {
			new_miners: BoundedVec<Registration<T>, T::MaxCohortSize>,
			released_miners: u32,
			frame_id: FrameId,
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
			frame_id: FrameId,
		},
		ReleaseBidError {
			account_id: T::AccountId,
			error: DispatchError,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Bidding for the next cohort has closed
		SlotNotTakingBids,
		/// The cohort registration overflowed
		TooManyBlockRegistrants,
		/// This funding account does not hold the minimum argonots needed
		InsufficientOwnershipTokens,
		/// The given bid isn't high enough to be in the cohort
		BidTooLow,
		/// An account can only have one active registration
		CannotRegisterOverlappingSessions,
		/// The funding account does not have enough funds to cover the bid
		InsufficientFunds,
		/// The mining bid cannot be reduced
		BidCannotBeReduced,
		/// Bids must be in allowed increments
		InvalidBidAmount,
		/// The argonots on hold cannot be released
		UnrecoverableHold,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			DidStartNewCohort::<T>::take();
			let next_frame_id = NextFrameId::<T>::get();
			if next_frame_id == 1 &&
				!IsNextSlotBiddingOpen::<T>::get() &&
				Self::is_slot_bidding_started()
			{
				log::trace!(
					"Opening Slot 1 bidding {}",
					MiningConfig::<T>::get().slot_bidding_start_after_ticks
				);
				IsNextSlotBiddingOpen::<T>::put(true);
			}

			// clear out previous miners
			ReleasedMinersByAccountId::<T>::take();

			T::DbWeight::get().reads_writes(2, 2)
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			let next_frame_id = NextFrameId::<T>::get();
			let calculated_frame_id = Self::calculated_frame_id();
			// if it's time for the cohort to start, do it
			if calculated_frame_id >= next_frame_id {
				log::trace!("Starting Slot {}", next_frame_id);
				Self::adjust_argonots_per_seat();
				Self::start_new_frame(calculated_frame_id);
				// we use the current price as part of calculations
				Self::adjust_number_of_seats();
				// new slot will rotate grandpas. Return so we don't do it again below
				return;
			}

			let rotate_grandpa_blocks =
				UniqueSaturatedInto::<u32>::unique_saturated_into(T::GrandpaRotationBlocks::get());
			let current_block = UniqueSaturatedInto::<u32>::unique_saturated_into(n);
			// rotate grandpas on off rotations
			if !HasAddedGrandpaRotation::<T>::get() || current_block % rotate_grandpa_blocks == 0 {
				T::SlotEvents::rotate_grandpas::<T::Keys>(calculated_frame_id, vec![], vec![]);
				HasAddedGrandpaRotation::<T>::put(true);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit a bid for a mining slot in the next cohort. Once all spots are filled in the next
		/// cohort, a bidder can be supplanted by supplying a higher bid.
		///
		/// Each slot has `MaxCohortSize` spots available.
		///
		/// To be eligible for a slot, you must have the required ownership tokens (argonots) in
		/// this account. The required amount is calculated as a percentage of the total ownership
		/// tokens in the network. This percentage is adjusted before the beginning of each slot.
		///
		/// If your bid is no longer winning, a `SlotBidderDropped` event will be emitted. By
		/// monitoring for this event, you will be able to ensure your bid is accepted.
		///
		/// NOTE: bidding for each slot will be closed at a random block within
		/// `mining_config.ticks_before_bid_end_for_vrf_close` blocks of the slot end time.
		///
		/// The slot duration can be calculated as `BlocksBetweenSlots * MaxMiners / MaxCohortSize`.
		///
		/// Parameters:
		/// - `bid`: The amount of argons to bid
		/// - `keys`: The session "hot" keys for the slot (BlockSealAuthorityId and GrandpaId).
		/// - `mining_account_id`: This account_id allows you to operate as this miner account id,
		///   but use funding (argonots and bid) from the submitting account
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::bid())]
		pub fn bid(
			origin: OriginFor<T>,
			bid: T::Balance,
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

			let existing_bid = Self::get_pending_cohort_registration(&miner_account_id);
			let current_registration = Self::get_active_registration(&miner_account_id);
			let next_frame_id = NextFrameId::<T>::get();
			if let Some(ref registration) = current_registration {
				let frames = T::FramesPerMiningTerm::get() as FrameId;
				// ensure we are not overlapping sessions
				ensure!(
					registration.starting_frame_id + frames == next_frame_id,
					Error::<T>::CannotRegisterOverlappingSessions
				);
			}

			Self::send_argons_to_pool(&funding_account, &existing_bid, bid)?;
			let ownership_tokens =
				Self::hold_argonots(&funding_account, &existing_bid, &current_registration)?;
			<BidsForNextSlotCohort<T>>::try_mutate(|cohort| -> DispatchResult {
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

				let next_cohort_size = NextCohortSize::<T>::get() as usize;
				if cohort.len() >= next_cohort_size {
					ensure!(pos < next_cohort_size, Error::<T>::BidTooLow);
					// need to pop-off the lowest bid
					let entry = cohort.pop().expect("should exist, just checked");
					Self::release_failed_bid(&entry)?;
				}

				cohort
					.try_insert(
						pos,
						MiningRegistration {
							account_id: miner_account_id.clone(),
							external_funding_account: if miner_account_id == funding_account {
								None
							} else {
								Some(funding_account)
							},
							bid,
							argonots: ownership_tokens,
							authority_keys: keys,
							starting_frame_id: next_frame_id,
							bid_at_tick: T::TickProvider::current_tick(),
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
		#[pallet::weight(T::WeightInfo::configure_mining_slot_delay())]
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
	fn get_block_rewards_account(author: &T::AccountId) -> Option<(T::AccountId, FrameId)> {
		let released_miners = ReleasedMinersByAccountId::<T>::get();
		if let Some(x) = released_miners.get(author) {
			return Some((x.rewards_account(), x.starting_frame_id));
		}

		let registration = Self::get_active_registration(author)?;
		Some((registration.rewards_account(), registration.starting_frame_id))
	}

	fn get_mint_rewards_accounts() -> Vec<(T::AccountId, FrameId)> {
		let mut result = vec![];
		for (_, cohort) in <MinersByCohort<T>>::iter() {
			for registration in cohort {
				let account = registration.rewards_account();
				result.push((account, registration.starting_frame_id));
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
		seal_proof: U256,
	) -> Option<MiningAuthority<T::MiningAuthorityId, T::AccountId>> {
		let xor_keys = <MinerXorKeysByCohort<T>>::get();
		let mut closest_distance: U256 = U256::MAX;
		let mut closest = None;
		for (frame_id, cohort) in xor_keys {
			for (i, peer_hash) in cohort.into_iter().enumerate() {
				let distance = seal_proof ^ peer_hash;
				if distance < closest_distance {
					closest_distance = distance;
					closest = Some((frame_id, i as u32));
				}
			}
		}

		Self::get_mining_authority_by_index(closest?)
	}

	fn xor_closest_managed_authority(
		seal_proof: U256,
		signing_key: &T::MiningAuthorityId,
		xor_distance: Option<U256>,
	) -> Option<(MiningAuthority<T::MiningAuthorityId, T::AccountId>, U256, Permill)> {
		let xor_keys = <MinerXorKeysByCohort<T>>::get();
		let mut closest_distance = xor_distance.unwrap_or(U256::MAX);
		let mut closest = None;
		let mut distances = vec![];

		for (frame_id, cohort) in xor_keys {
			for (i, peer_hash) in cohort.into_iter().enumerate() {
				let distance = seal_proof ^ peer_hash;
				distances.push(distance);
				if distance < closest_distance {
					let authority = Self::get_mining_authority_by_index((frame_id, i as u32))?;
					if &authority.authority_id == signing_key {
						closest_distance = distance;
						closest = Some(authority);
					}
				}
			}
		}
		let closest = closest?;

		distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
		let index = distances
			.iter()
			.position(|&x| x == closest_distance)
			.expect("should exist, just checked");
		Some((
			closest,
			closest_distance,
			Permill::from_rational(index as u32, distances.len() as u32),
		))
	}

	fn get_authority_distance(
		seal_proof: U256,
		authority_id: &T::MiningAuthorityId,
		account_id: &T::AccountId,
	) -> Option<U256> {
		let miner_index = AccountIndexLookup::<T>::get(account_id)?;
		let authority = Self::get_mining_authority_by_index(miner_index)?;
		if authority.authority_id != *authority_id {
			return None;
		}
		let xor_keys = MinerXorKeysByCohort::<T>::get();
		let peer_hash = xor_keys.get(&miner_index.0)?.get(miner_index.1 as usize)?;
		let distance = seal_proof ^ (*peer_hash);
		Some(distance)
	}
}

impl<T: Config> Pallet<T> {
	pub fn current_frame_id() -> FrameId {
		NextFrameId::<T>::get().saturating_sub(1)
	}

	pub fn is_registered_mining_active() -> bool {
		NextFrameId::<T>::get() > 1 && ActiveMinersCount::<T>::get() > 0
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
		let miner = Self::get_miner_by_index(index)?;
		miner
			.authority_keys
			.get(T::MiningAuthorityId::ID)
			.map(|authority_id| MiningAuthority {
				authority_id,
				account_id: miner.account_id.clone(),
				authority_index: index.unique_saturated_into(),
			})
	}

	pub fn get_miner_by_index(index: MinerIndex) -> Option<Registration<T>> {
		let cohort = MinersByCohort::<T>::get(index.0);
		cohort.get(index.1 as usize).cloned()
	}

	pub(crate) fn get_active_registration(account_id: &T::AccountId) -> Option<Registration<T>> {
		if let Some(index) = AccountIndexLookup::<T>::get(account_id) {
			return Self::get_miner_by_index(index);
		}
		None
	}

	pub(crate) fn start_new_frame(frame_id: FrameId) {
		HistoricalBidsPerSlot::<T>::mutate(|bids| {
			if bids.is_full() {
				bids.pop();
			}
			let _ = bids.try_insert(0, MiningBidStats::default());
		});

		let slot_cohort = BidsForNextSlotCohort::<T>::take();

		IsNextSlotBiddingOpen::<T>::put(true);

		let mut active_miners = ActiveMinersCount::<T>::get();
		let mut added_miners = vec![];
		let mut removed_miners = vec![];
		let mut released_miners_by_account_id = BoundedBTreeMap::new();

		let parent_hash: T::Hash = frame_system::Pallet::<T>::parent_hash();

		let frames_per_term = T::FramesPerMiningTerm::get() as FrameId;

		if frame_id >= frames_per_term {
			let retiring_frame_id = frame_id - frames_per_term;
			let rotating_out = MinersByCohort::<T>::take(retiring_frame_id);
			for miner in rotating_out {
				let account_id = miner.account_id.clone();
				AccountIndexLookup::<T>::remove(&account_id);
				active_miners -= 1;
				let _ = released_miners_by_account_id.try_insert(account_id.clone(), miner.clone());
				removed_miners.push((account_id, miner.authority_keys.clone()));
				let is_in_next = slot_cohort.iter().any(|x| x.account_id == miner.account_id);
				Self::release_mining_seat_argonots(&miner, is_in_next);
			}
		}

		// After all accounts are removed, we can add the new cohort members (otherwise, we can add
		// and then remove after if sorted differently with same account)
		MinersByCohort::<T>::insert(frame_id, slot_cohort.clone());

		let mut total_price_per_seat = T::Balance::zero();
		let mut xor = vec![];
		for (i, entry) in slot_cohort.iter().enumerate() {
			AccountIndexLookup::<T>::insert(&entry.account_id, (frame_id, i as u32));
			added_miners.push((entry.account_id.clone(), entry.authority_keys.clone()));
			active_miners += 1;
			total_price_per_seat += entry.bid;
			let hash =
				MinerXor::<T> { account_id: entry.account_id.clone(), block_hash: parent_hash }
					.using_encoded(blake2_256);

			xor.push(U256::from_big_endian(&hash));
		}

		MinerXorKeysByCohort::<T>::mutate(|a| {
			if frame_id >= frames_per_term {
				a.retain(|t, _v| *t > frame_id - frames_per_term);
			}
			a.try_insert(frame_id, BoundedVec::truncate_from(xor))
				.expect("Should be valid since we removed from this map previously");
		});
		let average_price_per_seat = total_price_per_seat
			.checked_div(&T::Balance::from(slot_cohort.len() as u32))
			.unwrap_or_default();
		AveragePricePerSeat::<T>::mutate(|a| {
			if a.is_full() {
				a.pop();
			}
			let _ = a.try_insert(0, average_price_per_seat);
		});

		ActiveMinersCount::<T>::put(active_miners);
		Pallet::<T>::deposit_event(Event::<T>::NewMiners {
			new_miners: slot_cohort,
			released_miners: removed_miners.len() as u32,
			frame_id,
		});

		ReleasedMinersByAccountId::<T>::put(released_miners_by_account_id);
		DidStartNewCohort::<T>::put(true);
		NextFrameId::<T>::put(frame_id + 1);
		FrameStartBlockNumbers::<T>::mutate(|a| {
			if a.is_full() {
				a.pop();
			}
			let _ = a.try_insert(0, frame_system::Pallet::<T>::block_number());
		});

		T::SlotEvents::rotate_grandpas(frame_id, removed_miners, added_miners);
		T::SlotEvents::on_frame_start(frame_id);
	}

	/// Adjust the argonots per seat amount based on a rolling 10-frame average of bids.
	///
	/// This should be called before starting a new slot. It will adjust the argonots per seat
	/// amount based on the number of bids in the last 10 frames to reach the target number of bids
	/// per cohort. The amount must also be adjusted based on the total ownership tokens in the
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

		let current_max_miners = NextCohortSize::<T>::get() * T::FramesPerMiningTerm::get();
		let base_ownership_tokens: u128 =
			ownership_circulation.checked_div(current_max_miners.into()).unwrap_or_default();

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
		// don't let this go below the minimum (it is in the beginning)
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

	/// Adjust the number of seats in the next cohort based on the average price per seat vs the
	/// target price per seat.
	///
	/// Swings are dampened by the `PricePerSeatAdjustmentDamper` constant and in the range of
	/// `MinCohortSize` => value <= `MaxCohortSize`.
	pub(crate) fn adjust_number_of_seats() {
		let price_target: u128 = T::TargetPricePerSeat::get().into();
		let trailing_average_price = AveragePricePerSeat::<T>::get();
		let prices = trailing_average_price.len() as u128;
		let price_sum: u128 = trailing_average_price.into_iter().map(Into::into).sum();
		let average_price = price_sum.checked_div(prices).unwrap_or_default();
		let max_cohort_size = T::MaxCohortSize::get();
		let min_cohort_size = T::MinCohortSize::get();
		let damper = T::PricePerSeatAdjustmentDamper::get();
		let one = FixedU128::one();
		let adjustment_percent = FixedU128::from_rational(average_price, price_target)
			.clamp(one.saturating_sub(damper), one.saturating_add(damper));

		if adjustment_percent == FixedU128::one() {
			return;
		}
		let mut next_cohort_size =
			adjustment_percent.saturating_mul_int(NextCohortSize::<T>::get());
		if next_cohort_size < min_cohort_size {
			next_cohort_size = min_cohort_size;
		} else if next_cohort_size > max_cohort_size {
			next_cohort_size = max_cohort_size;
		}

		NextCohortSize::<T>::put(next_cohort_size);
	}

	/// Check if the current block is in the closing window for the next slot
	///
	/// This is determined by looking at the block seal vote and using the following VRF formula:
	///  `VRF = blake2(seal_proof)`
	/// If VRF < threshold, then the auction will be ended
	///
	/// The random seal strength is used to ensure that the VRF is unique for each block:
	///  - the block vote was submitted in a previous notebook
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
		let next_slot_tick = Self::get_next_frame_tick();
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
			log::info!("VRF Close triggered: {:?} < {:?}", vote_seal_proof, threshold);
			let frame_id = NextFrameId::<T>::get();
			Self::deposit_event(Event::<T>::MiningBidsClosed { frame_id });
			return true;
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

	pub(crate) fn get_next_frame_tick() -> Tick {
		Self::tick_for_frame(Self::next_frame_id())
	}

	pub fn next_frame_id() -> FrameId {
		NextFrameId::<T>::get()
	}

	pub fn get_next_slot_era() -> (Tick, Tick) {
		let start_tick = Self::get_next_frame_tick();
		(start_tick, start_tick + Self::get_mining_window_ticks())
	}

	pub(crate) fn calculated_frame_id() -> FrameId {
		let mining_config = MiningConfig::<T>::get();
		let current_tick = T::TickProvider::current_tick();
		let genesis_tick = current_tick.saturating_sub(T::TickProvider::elapsed_ticks());
		let bidding_start_tick =
			genesis_tick.saturating_add(mining_config.slot_bidding_start_after_ticks);

		let ticks_since_mining_start = current_tick.saturating_sub(bidding_start_tick);
		ticks_since_mining_start / mining_config.ticks_between_slots
	}

	pub fn tick_for_frame(frame_id: FrameId) -> Tick {
		if frame_id == 0 {
			// return genesis tick for slot 0
			return T::TickProvider::current_tick().saturating_sub(T::TickProvider::elapsed_ticks());
		}
		let slot_1_tick = Self::slot_1_tick();
		let added_ticks = (frame_id - 1) * Self::ticks_between_slots();
		slot_1_tick.saturating_add(added_ticks)
	}

	pub fn get_mining_window_ticks() -> Tick {
		let ticks_between_slots = Self::ticks_between_slots();
		let term_frames = T::FramesPerMiningTerm::get() as Tick;

		term_frames.saturating_mul(ticks_between_slots)
	}

	pub(crate) fn get_pending_cohort_registration(
		account_id: &T::AccountId,
	) -> Option<Registration<T>> {
		BidsForNextSlotCohort::<T>::get()
			.into_iter()
			.find(|x| x.account_id == *account_id)
	}

	pub(crate) fn send_argons_to_pool(
		miner_funding_account: &T::AccountId,
		existing_bid: &Option<Registration<T>>,
		bid_amount: T::Balance,
	) -> Result<T::Balance, DispatchError> {
		let mut needed = bid_amount;

		// if we've already held for next, reduce now
		if let Some(existing) = existing_bid {
			ensure!(bid_amount > existing.bid, Error::<T>::BidCannotBeReduced);
			needed -= existing.bid;
		}
		if needed > 0u32.into() {
			let pool_account = T::BidPoolProvider::get_bid_pool_account();
			ensure!(
				T::ArgonCurrency::reducible_balance(
					miner_funding_account,
					Preservation::Protect,
					Fortitude::Force
				) >= needed,
				Error::<T>::InsufficientFunds
			);
			T::ArgonCurrency::transfer(
				miner_funding_account,
				&pool_account,
				needed,
				Preservation::Expendable,
			)?;
		}
		Ok(needed)
	}

	pub(crate) fn hold_argonots(
		funding_account_id: &T::AccountId,
		existing_bid: &Option<Registration<T>>,
		current_registration: &Option<Registration<T>>,
	) -> Result<T::Balance, DispatchError> {
		// if we have an existing bid, we don't need to hold again
		if let Some(existing) = existing_bid {
			return Ok(existing.argonots);
		}

		let argonots = ArgonotsPerMiningSeat::<T>::get();
		let mut argonots_needed = argonots;

		// if we've already held for next, reduce now
		if let Some(current_registration) = current_registration {
			argonots_needed -= current_registration.argonots;
		}

		if argonots_needed == 0u32.into() {
			return Ok(argonots);
		}

		let hold_reason = HoldReason::RegisterAsMiner;
		if T::OwnershipCurrency::balance_on_hold(&hold_reason.into(), funding_account_id) ==
			0u32.into()
		{
			frame_system::Pallet::<T>::inc_providers(funding_account_id);
		}

		T::OwnershipCurrency::hold(&hold_reason.into(), funding_account_id, argonots_needed)
			.map_err(|_| Error::<T>::InsufficientOwnershipTokens)?;
		Ok(argonots)
	}

	fn release_argonots_hold(account_id: &T::AccountId, amount: T::Balance) -> DispatchResult {
		let reason = HoldReason::RegisterAsMiner;
		if amount == 0u32.into() {
			return Ok(());
		}
		T::OwnershipCurrency::release(&reason.into(), account_id, amount, Precision::Exact)
			.map_err(|e| {
				log::warn!(
					"Error recovering mining slot hold for {account_id:?}. Amount {amount:?}. {:?}",
					e
				);
				Error::<T>::UnrecoverableHold
			})?;

		if T::OwnershipCurrency::balance_on_hold(&reason.into(), account_id) == 0u32.into() {
			let _ = frame_system::Pallet::<T>::dec_providers(account_id);
		}
		Ok(())
	}

	/// Release the argonots from the mining seat. If the Argonots will be re-used in the next
	/// era, we should not unlock it
	pub(crate) fn release_mining_seat_argonots(
		active_registration: &Registration<T>,
		is_registered_for_next: bool,
	) {
		if is_registered_for_next {
			return;
		}
		let account_id = &active_registration.account_id;

		let funding_account = active_registration
			.external_funding_account
			.clone()
			.unwrap_or(account_id.clone());

		if let Err(e) = Self::release_argonots_hold(&funding_account, active_registration.argonots)
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
		}
	}

	pub(crate) fn release_failed_bid(bid_registration: &Registration<T>) -> DispatchResult {
		let funding_account = bid_registration
			.external_funding_account
			.clone()
			.unwrap_or(bid_registration.account_id.clone());
		let account_id = bid_registration.account_id.clone();

		if bid_registration.bid > T::Balance::zero() {
			let pool_account = T::BidPoolProvider::get_bid_pool_account();
			T::ArgonCurrency::transfer(
				&pool_account,
				&funding_account,
				bid_registration.bid,
				Preservation::Expendable,
			)?;
		}

		let mut held_argonots = false;
		let mut argonots_to_unhold: T::Balance = bid_registration.argonots;
		if let Some(active) = Self::get_active_registration(&account_id) {
			argonots_to_unhold.saturating_reduce(active.argonots);
			held_argonots = true;
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

	pub fn bid_pool_balance() -> T::Balance {
		let account_id = T::BidPoolProvider::get_bid_pool_account();
		T::ArgonCurrency::balance(&account_id)
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct MinerXor<T: Config> {
	pub account_id: T::AccountId,
	pub block_hash: T::Hash,
}

impl<T: Config> MiningSlotProvider for Pallet<T> {
	fn get_next_slot_tick() -> Tick {
		Self::get_next_frame_tick()
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
			return;
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

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::MiningAuthorityId;
}

sp_api::decl_runtime_apis! {
	/// This runtime api allows people to query the upcoming mining_slot
	pub trait MiningSlotApi<Balance> where Balance: Codec {
		fn next_slot_era() -> (Tick, Tick);
		fn bid_pool() -> Balance;
	}
}
