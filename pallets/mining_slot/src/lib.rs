#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
extern crate core;
use alloc::collections::{BTreeMap, BTreeSet};
use core::cmp::Ordering;

use argon_primitives::{
	block_seal::{MinerIndex, MiningAuthority, MiningBidStats, MiningSlotConfig},
	inherents::BlockSealInherent,
	providers::*,
	ArgonDigests, SlotEvents, TickProvider,
};
pub use pallet::*;
use pallet_prelude::*;
use sp_runtime::{traits::OpaqueKeys, RuntimeAppPublic};
pub use weights::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod auction;
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
/// To be eligible for mining, you must reserve enough argonots to cover 2x (configurable) of the
/// previous day's lower-median winning bid using the previous frame's average Argonot price.
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

	use sp_runtime::BoundedBTreeMap;

	use super::*;
	use argon_primitives::{
		block_seal::MiningRegistration, digests::FrameInfo, ArgonDigests, SlotEvents, TickProvider,
	};
	use pallet_prelude::argon_primitives::FRAME_INFO_DIGEST;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(12);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	pub type Registration<T> = MiningRegistration<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
		<T as Config>::Keys,
	>;

	#[derive(
		Clone,
		Copy,
		Debug,
		Default,
		PartialEq,
		Eq,
		Encode,
		Decode,
		DecodeWithMemTracking,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct SeatPriceStats<Balance> {
		pub seats: u32,
		pub total_bids: Balance,
	}

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

		/// Maximum commitments, including public bids, admitted per block.
		#[pallet::constant]
		type MaxBidsPerBlock: Get<u32>;

		#[pallet::constant]
		type BidRevealBlocks: Get<u32>;

		#[pallet::constant]
		type BidSettlementBuffer: Get<u32>;

		/// The initial argonots needed per seat before a prior-day median is available.
		#[pallet::constant]
		type InitialArgonotsPerSeat: Get<Self::Balance>;

		/// The multiple of the prior-day lower median winning bid that must be collateralized.
		#[pallet::constant]
		type ArgonotBidCollateralMultiple: Get<u128>;

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

		/// Price source for frame-average Argonot valuations.
		type PriceProvider: PriceProvider<Self::Balance>;

		/// The hold reason when reserving funds for entering or extending the safe-mode.
		type RuntimeHoldReason: From<HoldReason>;

		/// Account that receives mining bid funds before treasury distribution.
		#[pallet::constant]
		type MiningBidPoolAccount: Get<Self::AccountId>;
		type OperationalAccountProvider: OperationalAccountProvider<Self::AccountId>;
		type OperationalAccountsHook: OperationalAccountsHook<Self::AccountId, Self::Balance>;
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

		// Find the author of a block
		type SealerInfo: BlockSealerProvider<Self::AccountId, Self::MiningAuthorityId>;
	}

	/// A reason for the pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		#[codec(index = 0)]
		RegisterAsMiner,
	}

	/// Transient data is separate from the stable MiningRegistration encoding.
	#[derive(Encode, Decode, DecodeWithMemTracking, CloneNoBound, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct PendingBid<T: Config> {
		pub block: BlockNumberFor<T>,
		pub frame: FrameId,
		pub funding: T::AccountId,
		pub miner: T::AccountId,
		pub max_hash: Option<[u8; 32]>,
		pub registration: Registration<T>,
		pub bid: T::Balance,
		pub max: Option<T::Balance>,
		pub extra_argonots: T::Balance,
	}

	#[pallet::storage]
	pub type PendingBids<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, PendingBid<T>>;

	/// Vector position is canonical commitment order, including calls inside utility batches.
	#[pallet::storage]
	pub type BidBatches<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<T::AccountId, T::MaxBidsPerBlock>,
		ValueQuery,
	>;

	/// Public maximum retained only while the miner is a winner of the upcoming auction.
	#[derive(Encode, Decode, DecodeWithMemTracking, CloneNoBound, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct StandingBid<T: Config> {
		pub bid: T::Balance,
		pub max: T::Balance,
		pub fixed: bool,
		pub submitted_at: BlockNumberFor<T>,
		pub position: u32,
	}

	#[pallet::storage]
	pub type StandingBids<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StandingBid<T>>;

	/// Zero on fresh chains. Upgrades finish the deployed fixed-price auction first.
	#[pallet::storage]
	pub type AuctionRulesStartAt<T: Config> = StorageValue<_, FrameId, ValueQuery>;

	#[pallet::storage]
	pub type LastBidBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	#[pallet::storage]
	pub type AuctionAdmissionClosed<T: Config> = StorageValue<_, bool, ValueQuery>;

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

	/// This is a lookup of each miner's nonce to use when picking a best authority to submit a
	/// block. It's a blake2 256 hash of the miner account id and the block hash at time of
	/// activation.
	#[pallet::storage]
	pub type MinerNonceScoringByCohort<T: Config> = StorageValue<
		_,
		BoundedBTreeMap<
			FrameId,
			BoundedVec<MinerNonceScoring<T>, T::MaxCohortSize>,
			T::FramesPerMiningTerm,
		>,
		ValueQuery,
	>;

	#[pallet::type_value]
	pub fn DefaultArgonotsPerMiningSeat<T: Config>() -> T::Balance {
		T::InitialArgonotsPerSeat::get()
	}

	/// Argonots that must be locked to take a Miner role
	#[pallet::storage]
	pub type ArgonotsPerMiningSeat<T: Config> =
		StorageValue<_, T::Balance, ValueQuery, DefaultArgonotsPerMiningSeat<T>>;

	/// Lookup by account id to the corresponding index in MinersByCohort and MinerNoncesByCohort
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

	/// Bid activity and accepted prices for the last 10 auctions (newest first).
	/// Activity counts committed submissions and automatic standing-bid price increases.
	#[pallet::storage]
	pub type HistoricalBidsPerSlot<T: Config> =
		StorageValue<_, BoundedVec<MiningBidStats, ConstU32<10>>, ValueQuery>;

	/// Whether the current auction has an accepted price, including a valid zero price.
	#[pallet::storage]
	pub type HasAcceptedBidPrice<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Winning bid totals and seat counts for averaging prices over the last 10 frames (newest
	/// first).
	#[pallet::storage]
	pub type AveragePricePerSeat<T: Config> =
		StorageValue<_, BoundedVec<SeatPriceStats<T::Balance>, ConstU32<10>>, ValueQuery>;

	/// The mining slot configuration set in genesis
	#[pallet::storage]
	pub type MiningConfig<T: Config> = StorageValue<_, MiningSlotConfig, ValueQuery>;

	/// The next frameId. A frame in argon is the 24 hours between the start of two different mining
	/// cohorts.
	#[pallet::storage]
	pub type NextFrameId<T: Config> = StorageValue<_, FrameId, ValueQuery>;

	/// Is a new frame started in this block
	#[pallet::storage]
	pub type NewlyStartedFrameId<T: Config> = StorageValue<_, FrameId, OptionQuery>;

	/// The number of allow miners to bid for the next mining cohort
	#[pallet::storage]
	pub type NextCohortSize<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The upcoming changes scheduled for cohort size by frame.
	/// Capacity of 11 allows for 10 frames worth of scheduled changes plus one working slot
	/// to schedule a new change during frame finalization.
	#[pallet::storage]
	pub type ScheduledCohortSizeChangeByFrame<T: Config> =
		StorageValue<_, BoundedBTreeMap<FrameId, u32, ConstU32<11>>, ValueQuery>;

	/// The number of reward ticks remaining in the frame
	#[pallet::storage]
	pub type FrameRewardTicksRemaining<T> = StorageValue<_, Tick, ValueQuery>;

	/// The previous 10 frame start block numbers
	#[pallet::storage]
	pub type FrameStartBlockNumbers<T: Config> =
		StorageValue<_, BoundedVec<BlockNumberFor<T>, ConstU32<10>>, ValueQuery>;
	/// The previous 10 frame start ticks
	#[pallet::storage]
	pub type FrameStartTicks<T: Config> =
		StorageValue<_, BoundedBTreeMap<FrameId, Tick, ConstU32<10>>, ValueQuery>;

	/// Temporary store the frame info digest
	#[pallet::storage]
	pub type TempFrameInfoDigest<T: Config> = StorageValue<_, FrameInfo, OptionQuery>;

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
			FrameRewardTicksRemaining::<T>::put(self.mining_config.ticks_between_slots);
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
		BidCommitted {
			account_id: T::AccountId,
			target_frame_id: FrameId,
		},
		BidRevealed {
			account_id: T::AccountId,
		},
		BidRejected {
			account_id: T::AccountId,
		},
		WinningBidPriceChanged {
			account_id: T::AccountId,
			bid_amount: T::Balance,
		},
		BidLost {
			account_id: T::AccountId,
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
		/// Cannot re-register an account with a different funding account
		CannotChangeFundingAccount,
		/// The funding account does not have enough funds to cover the bid
		InsufficientFunds,
		/// The mining bid cannot be reduced
		BidCannotBeReduced,
		/// Bids must be in allowed increments
		InvalidBidAmount,
		/// Mining slot bidding currently requires a prior operational-account upgrade.
		OperationalAccountRegistrationRequired,
		/// The argonots on hold cannot be released
		UnrecoverableHold,
		/// The bid targets a different auction frame than the current one.
		WrongBidFrame,
		/// The miner has no pending bid with a committed maximum to reveal.
		MissingBidCommitment,
		/// A maximum must be revealed in a block after submission and within the reveal window.
		InvalidRevealBlock,
		/// The pending bid's maximum has already been revealed.
		BidAlreadyRevealed,
		/// The revealed maximum and salt do not match the pending bid's hash.
		InvalidBidCommitment,
		/// The miner already has a pending bid awaiting settlement.
		DuplicateBidCommitment,
		/// This block has reached the maximum number of pending bid submissions.
		TooManyBidCommitments,
		/// The revealed maximum is lower than the public bid.
		InvalidBidRange,
		/// Bidding with a committed maximum is not enabled for the current auction frame.
		RangeAuctionNotActive,
		/// The bid batch cannot settle until its reveal window has ended.
		BatchNotMature,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> Weight {
			if StorageVersion::get::<Pallet<T>>() < STORAGE_VERSION {
				// Only fixed-price bidding was deployed. Never change an auction in progress.
				AuctionRulesStartAt::<T>::put(NextFrameId::<T>::get().saturating_add(1));
				// Deployed counters contain only accepted bids, including zero-priced bids.
				HasAcceptedBidPrice::<T>::put(
					HistoricalBidsPerSlot::<T>::get()
						.first()
						.is_some_and(|stats| stats.bids_count > 0),
				);
				STORAGE_VERSION.put::<Pallet<T>>();
				T::DbWeight::get().reads_writes(3, 3)
			} else {
				T::DbWeight::get().reads(1)
			}
		}

		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			// Reset this block-scoped flag before we compute transition state for the current
			// block.
			NewlyStartedFrameId::<T>::take();
			let digest = <frame_system::Pallet<T>>::digest();
			for log in digest.logs.iter() {
				if let Some(frame_info) = log.as_frame_info() {
					assert!(
						!TempFrameInfoDigest::<T>::exists(),
						"Frame info digest can only be provided once!"
					);

					TempFrameInfoDigest::<T>::put(frame_info.clone());
					break;
				}
			}

			let next_frame_id = NextFrameId::<T>::get();
			if next_frame_id == 1 &&
				!IsNextSlotBiddingOpen::<T>::get() &&
				!AuctionAdmissionClosed::<T>::get() &&
				Self::is_seat_bidding_started()
			{
				log::trace!(
					"Opening Slot 1 bidding {}",
					MiningConfig::<T>::get().slot_bidding_start_after_ticks
				);
				IsNextSlotBiddingOpen::<T>::put(true);
			}

			// clear out previous miners
			ReleasedMinersByAccountId::<T>::take();

			let has_vote_seal = Self::has_vote_seal_digest();
			// Worst-case bookkeeping done directly in `on_initialize` before adding delegated
			// benchmarked path weights below.
			let previous = n.saturating_sub(T::BidRevealBlocks::get().into());
			let count = BidBatches::<T>::decode_len(previous).unwrap_or(0) as u32;
			let mut weight = T::WeightInfo::auction_bookkeeping(1)
				.saturating_add(T::WeightInfo::settle_bids(count, T::MaxCohortSize::get()));
			if has_vote_seal {
				weight = weight.saturating_add(T::WeightInfo::on_finalize_record_block_author());
			}
			if let Some(frame_id) = Self::evaluate_newly_started_frame(has_vote_seal) {
				let cohort_size = NextCohortSize::<T>::get();
				weight = weight
					.saturating_add(T::WeightInfo::start_new_frame(cohort_size))
					.saturating_add(T::WeightInfo::on_finalize_frame_adjustments())
					.saturating_add(T::SlotEvents::on_frame_start_weight(frame_id))
					.saturating_add(
						<T::OperationalAccountsHook as OperationalAccountsHook<
							T::AccountId,
							T::Balance,
						>>::Weights::mining_seat_won()
						.saturating_mul(cohort_size as u64),
					);
			} else if Self::should_rotate_grandpas(n) {
				weight = weight.saturating_add(T::WeightInfo::on_finalize_grandpa_rotation());
			}

			if Self::range_auction_enabled() &&
				NextFrameId::<T>::get() > 1 &&
				FrameRewardTicksRemaining::<T>::get() <=
					(T::BidRevealBlocks::get() + T::BidSettlementBuffer::get()) as Tick
			{
				Self::close_auction_admission();
			}
			weight
		}

		fn on_finalize(n: BlockNumberFor<T>) {
			if Self::has_vote_seal_digest() {
				// Sealer info is only available after the block seal inherents (eg, between
				// initialize and finalize)
				let author = T::SealerInfo::get_sealer_info().block_author_account_id;
				Self::record_block_author(author);
			}

			// Settlement precedes collateral sampling, cohort activation, and treasury callbacks.
			// Funding is allocated from the post-extrinsic snapshot before any prices are applied.
			if n >= T::BidRevealBlocks::get().into() {
				Self::settle_bids(n - T::BidRevealBlocks::get().into())
					.expect("mining auction accounting invariant");
			}

			// if it's time for the cohort to start, do it
			if let Some(activating_frame_id) = Self::get_newly_started_frame() {
				log::info!("Starting Frame {activating_frame_id}");
				Self::update_argonots_per_seat(activating_frame_id);
				Self::start_new_frame(activating_frame_id);
				// we use the current price as part of calculations
				Self::adjust_number_of_seats();
			}
			// rotate grandpas on off rotations
			else if Self::should_rotate_grandpas(n) {
				let current_frame_id = Self::current_frame_id();
				T::SlotEvents::rotate_grandpas::<T::Keys>(current_frame_id, vec![], vec![]);
				HasAddedGrandpaRotation::<T>::put(true);
			}
			if let Some(frame_info) = TempFrameInfoDigest::<T>::take() {
				assert_eq!(
					frame_info.frame_id,
					Self::current_frame_id(),
					"Frame id in digest must match current frame id"
				);
				assert_eq!(
					frame_info.frame_reward_ticks_remaining as Tick,
					FrameRewardTicksRemaining::<T>::get(),
					"Frame reward ticks remaining in digest must match storage"
				);
				assert_eq!(
					frame_info.is_new_frame,
					NewlyStartedFrameId::<T>::get().is_some(),
					"is_new_frame in digest must match storage"
				);
			} else {
				<frame_system::Pallet<T>>::deposit_log(DigestItem::Consensus(
					FRAME_INFO_DIGEST,
					FrameInfo {
						frame_reward_ticks_remaining: FrameRewardTicksRemaining::<T>::get()
							.unique_saturated_into(),
						frame_id: Self::current_frame_id(),
						is_new_frame: NewlyStartedFrameId::<T>::get().is_some(),
					}
					.encode(),
				));
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit a funded public bid, optionally committing to a hidden maximum.
		/// Every bid competes at its public amount even if no maximum is revealed.
		/// An omitted target frame selects the current auction at dispatch time.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::bid())]
		pub fn bid(
			origin: OriginFor<T>,
			bid: T::Balance,
			keys: T::Keys,
			mining_account_id: Option<T::AccountId>,
			target_frame_id: Option<FrameId>,
			max_hash: Option<[u8; 32]>,
		) -> DispatchResult {
			let funding = ensure_signed(origin)?;
			let next_frame_id = NextFrameId::<T>::get();
			ensure!(
				target_frame_id.is_none_or(|frame| frame == next_frame_id),
				Error::<T>::WrongBidFrame
			);
			let miner = mining_account_id.unwrap_or_else(|| funding.clone());
			if !Self::range_auction_enabled() {
				ensure!(max_hash.is_none(), Error::<T>::RangeAuctionNotActive);
				return Self::legacy_bid(funding, bid, keys, miner);
			}
			Self::check_public_bid(bid)?;
			Self::queue_bid(funding, miner, bid, keys, max_hash)
		}

		// Index 2 belonged to the unreleased commit_bid call and must not be reused.
		/// Disclose a committed maximum during the five blocks following submission.
		/// The target frame must match the original bid and the current auction.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::bid_max())]
		pub fn bid_max(
			origin: OriginFor<T>,
			max: T::Balance,
			salt: [u8; 32],
			mining_account_id: Option<T::AccountId>,
			target_frame_id: FrameId,
		) -> DispatchResult {
			let funding = ensure_signed(origin)?;
			let miner = mining_account_id.unwrap_or_else(|| funding.clone());
			PendingBids::<T>::try_mutate(&miner, |pending| -> DispatchResult {
				let pending = pending.as_mut().ok_or(Error::<T>::MissingBidCommitment)?;
				ensure!(pending.funding == funding, Error::<T>::CannotChangeFundingAccount);
				ensure!(pending.max_hash.is_some(), Error::<T>::MissingBidCommitment);
				ensure!(pending.max.is_none(), Error::<T>::BidAlreadyRevealed);
				ensure!(
					frame_system::Pallet::<T>::block_number() > pending.block &&
						frame_system::Pallet::<T>::block_number() <=
							pending.block.saturating_add(T::BidRevealBlocks::get().into()),
					Error::<T>::InvalidRevealBlock
				);
				ensure!(
					pending.frame == target_frame_id && target_frame_id == NextFrameId::<T>::get(),
					Error::<T>::WrongBidFrame
				);
				ensure!(pending.bid <= max, Error::<T>::InvalidBidRange);
				ensure!(
					max % T::BidIncrements::get() == Zero::zero(),
					Error::<T>::InvalidBidAmount
				);
				let hash = Self::bid_commitment(
					pending.frame,
					&funding,
					&miner,
					pending.bid,
					max,
					&pending.registration.authority_keys,
					salt,
				);
				ensure!(pending.max_hash == Some(hash), Error::<T>::InvalidBidCommitment);
				Self::check_bid_identity(&funding, &miner)?;
				let existing = Self::get_pending_cohort_registration(&miner);
				if let Some(old) = &existing {
					let old_max = StandingBids::<T>::get(&miner).map(|s| s.max).unwrap_or(old.bid);
					ensure!(max > old_max, Error::<T>::BidCannotBeReduced);
				}
				let credit = existing.map(|r| r.bid).unwrap_or_default();
				ensure!(
					T::ArgonCurrency::reducible_balance(
						&funding,
						Preservation::Protect,
						Fortitude::Polite
					) >= max.saturating_sub(credit),
					Error::<T>::InsufficientFunds
				);
				pending.max = Some(max);
				Self::deposit_event(Event::BidRevealed { account_id: miner.clone() });
				Ok(())
			})
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
				if let Some(ticks_before_bid_end_for_vrf_close) = ticks_before_bid_end_for_vrf_close &&
					a.ticks_before_bid_end_for_vrf_close != ticks_before_bid_end_for_vrf_close
				{
					a.ticks_before_bid_end_for_vrf_close = ticks_before_bid_end_for_vrf_close;
					has_update = true;
				}

				if let Some(mining_slot_delay) = mining_slot_delay &&
					a.slot_bidding_start_after_ticks != mining_slot_delay
				{
					a.slot_bidding_start_after_ticks = mining_slot_delay;
					has_update = true;
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
impl<T: Config> Pallet<T> {
	/// Shared SCALE encoding; clients must use a fresh unpredictable salt for every commitment.
	pub fn bid_commitment(
		frame: FrameId,
		funding: &T::AccountId,
		miner: &T::AccountId,
		bid: T::Balance,
		max: T::Balance,
		keys: &T::Keys,
		salt: [u8; 32],
	) -> [u8; 32] {
		argon_primitives::block_seal::mining_bid_max_hash(
			frame_system::Pallet::<T>::block_hash(BlockNumberFor::<T>::zero()),
			frame,
			funding,
			miner,
			bid,
			max,
			keys,
			salt,
		)
	}

	fn check_bid_identity(funding: &T::AccountId, miner: &T::AccountId) -> DispatchResult {
		ensure!(
			T::OperationalAccountProvider::is_eligible(funding),
			Error::<T>::OperationalAccountRegistrationRequired
		);
		for registration in
			[Self::get_pending_cohort_registration(miner), Self::get_active_registration(miner)]
				.into_iter()
				.flatten()
		{
			ensure!(
				registration
					.external_funding_account
					.as_ref()
					.unwrap_or(&registration.account_id) ==
					funding,
				Error::<T>::CannotChangeFundingAccount
			);
		}
		if let Some(active) = Self::get_active_registration(miner) {
			ensure!(
				active.starting_frame_id + T::FramesPerMiningTerm::get() as FrameId ==
					NextFrameId::<T>::get(),
				Error::<T>::CannotRegisterOverlappingSessions
			);
		}
		Ok(())
	}

	pub fn range_auction_enabled() -> bool {
		NextFrameId::<T>::get() >= AuctionRulesStartAt::<T>::get()
	}

	fn close_auction_admission() {
		AuctionAdmissionClosed::<T>::put(true);
		if IsNextSlotBiddingOpen::<T>::take() {
			Self::deposit_event(Event::MiningBidsClosed { frame_id: NextFrameId::<T>::get() });
		}
	}

	fn auction_ready_to_activate() -> bool {
		Self::close_auction_admission();
		LastBidBlock::<T>::get().is_none_or(|block| {
			frame_system::Pallet::<T>::block_number() >
				block
					.saturating_add(T::BidRevealBlocks::get().into())
					.saturating_add(T::BidSettlementBuffer::get().into())
		})
	}

	/// Allocate each wallet once, preserving each incumbent's own paid credit.
	fn funded_standing_offers(
		old: &[Registration<T>],
		available: &mut BTreeMap<T::AccountId, T::Balance>,
	) -> Vec<(Registration<T>, StandingBid<T>, T::Balance)> {
		let mut offers = old
			.iter()
			.enumerate()
			.map(|(i, r)| {
				let standing = StandingBids::<T>::get(&r.account_id).unwrap_or(StandingBid::<T> {
					bid: r.bid,
					max: r.bid,
					fixed: true,
					submitted_at: Zero::zero(),
					position: i as u32,
				});
				(r.clone(), standing, r.bid)
			})
			.collect::<Vec<_>>();
		offers.sort_by(|a, b| {
			a.1.submitted_at.cmp(&b.1.submitted_at).then(a.1.position.cmp(&b.1.position))
		});
		let increment = T::BidIncrements::get();
		for (r, standing, budget) in &mut offers {
			let account = r.external_funding_account.as_ref().unwrap_or(&r.account_id);
			let free = available.entry(account.clone()).or_insert_with(|| {
				T::ArgonCurrency::reducible_balance(
					account,
					Preservation::Protect,
					Fortitude::Polite,
				)
			});
			let extra = standing.max.saturating_sub(r.bid).min(*free / increment * increment);
			*free -= extra;
			*budget = r.bid.saturating_add(extra);
		}
		offers
	}

	fn check_public_bid(bid: T::Balance) -> DispatchResult {
		let increment = T::BidIncrements::get();
		ensure!(
			!increment.is_zero() && bid % increment == Zero::zero(),
			Error::<T>::InvalidBidAmount
		);
		let winners = BidsForNextSlotCohort::<T>::get();
		if !winners.is_empty() && winners.len() >= NextCohortSize::<T>::get() as usize {
			let offers = Self::funded_standing_offers(&winners, &mut Default::default());
			let minimum = offers.iter().map(|(_, _, budget)| *budget).min().expect("nonempty");
			ensure!(bid > minimum, Error::<T>::BidTooLow);
		}
		Ok(())
	}

	fn legacy_bid(
		funding_account: T::AccountId,
		bid: T::Balance,
		keys: T::Keys,
		miner_account_id: T::AccountId,
	) -> DispatchResult {
		ensure!(
			T::OperationalAccountProvider::is_eligible(&funding_account),
			Error::<T>::OperationalAccountRegistrationRequired
		);
		ensure!(IsNextSlotBiddingOpen::<T>::get(), Error::<T>::SlotNotTakingBids);
		ensure!(bid % T::BidIncrements::get() == T::Balance::zero(), Error::<T>::InvalidBidAmount);

		let existing_bid = Self::get_pending_cohort_registration(&miner_account_id);
		if let Some(existing) = existing_bid.as_ref() {
			let existing_funding = existing
				.external_funding_account
				.clone()
				.unwrap_or_else(|| miner_account_id.clone());
			ensure!(existing_funding == funding_account, Error::<T>::CannotChangeFundingAccount);
		}
		let current_registration = Self::get_active_registration(&miner_account_id);
		let next_frame_id = NextFrameId::<T>::get();
		if let Some(ref registration) = current_registration {
			let current_funding_account = registration
				.external_funding_account
				.as_ref()
				.unwrap_or(&registration.account_id);
			ensure!(
				current_funding_account == &funding_account,
				Error::<T>::CannotChangeFundingAccount
			);
			let frames = T::FramesPerMiningTerm::get() as FrameId;
			// ensure we are not overlapping sessions
			ensure!(
				registration.starting_frame_id + frames == next_frame_id,
				Error::<T>::CannotRegisterOverlappingSessions
			);
		}
		if let Some(ref existing_registration) = existing_bid {
			let existing_funding_account = existing_registration
				.external_funding_account
				.as_ref()
				.unwrap_or(&existing_registration.account_id);
			ensure!(
				existing_funding_account == &funding_account,
				Error::<T>::CannotChangeFundingAccount
			);
			ensure!(bid > existing_registration.bid, Error::<T>::BidCannotBeReduced);
		}

		Self::send_argons_to_pool(&funding_account, &existing_bid, bid)?;
		let ownership_tokens =
			Self::hold_argonots(&funding_account, &existing_bid, &current_registration)?;
		BidsForNextSlotCohort::<T>::try_mutate(|cohort| -> DispatchResult {
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
					Registration::<T> {
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

			Self::record_bid_stats(1, &[bid]);

			Self::deposit_event(Event::<T>::SlotBidderAdded {
				account_id: miner_account_id.clone(),
				bid_amount: bid,
				index: UniqueSaturatedInto::<u32>::unique_saturated_into(pos),
			});

			Ok(())
		})?;

		Ok(())
	}

	#[frame_support::transactional]
	fn queue_bid(
		funding: T::AccountId,
		miner: T::AccountId,
		bid: T::Balance,
		keys: T::Keys,
		max_hash: Option<[u8; 32]>,
	) -> DispatchResult {
		ensure!(
			IsNextSlotBiddingOpen::<T>::get() &&
				!AuctionAdmissionClosed::<T>::get() &&
				NewlyStartedFrameId::<T>::get().is_none(),
			Error::<T>::SlotNotTakingBids
		);
		Self::check_bid_identity(&funding, &miner)?;
		ensure!(!PendingBids::<T>::contains_key(&miner), Error::<T>::DuplicateBidCommitment);
		let existing = Self::get_pending_cohort_registration(&miner);
		if max_hash.is_none() &&
			let Some(old) = &existing
		{
			let old_max = StandingBids::<T>::get(&miner).map(|s| s.max).unwrap_or(old.bid);
			ensure!(bid > old_max, Error::<T>::BidCannotBeReduced);
		}
		let credit = existing.as_ref().map(|r| r.bid).unwrap_or_default();
		ensure!(
			T::ArgonCurrency::reducible_balance(&funding, Preservation::Protect, Fortitude::Polite) >=
				bid.saturating_sub(credit),
			Error::<T>::InsufficientFunds
		);
		let active = Self::get_active_registration(&miner);
		let argonots = Self::hold_argonots(&funding, &existing, &active)?;
		let previous =
			existing.as_ref().or(active.as_ref()).map(|r| r.argonots).unwrap_or_default();
		let block = frame_system::Pallet::<T>::block_number();
		let pending = PendingBid::<T> {
			block,
			frame: NextFrameId::<T>::get(),
			funding: funding.clone(),
			miner: miner.clone(),
			max_hash,
			bid,
			max: None,
			extra_argonots: argonots.saturating_sub(previous),
			registration: Registration::<T> {
				account_id: miner.clone(),
				external_funding_account: (funding != miner).then_some(funding),
				bid,
				argonots,
				authority_keys: keys,
				starting_frame_id: NextFrameId::<T>::get(),
				bid_at_tick: T::TickProvider::current_tick(),
			},
		};
		BidBatches::<T>::try_mutate(block, |batch| {
			batch.try_push(miner.clone()).map_err(|_| Error::<T>::TooManyBidCommitments)
		})?;
		Self::deposit_event(Event::BidCommitted {
			account_id: miner.clone(),
			target_frame_id: pending.frame,
		});
		LastBidBlock::<T>::put(block);
		PendingBids::<T>::insert(miner, pending);
		Self::record_bid_stats(1, &[]);
		Ok(())
	}

	/// Count committed activity independently of accepted-price observations.
	fn record_bid_stats(new_bids: u32, accepted_prices: &[T::Balance]) {
		if new_bids == 0 && accepted_prices.is_empty() {
			return;
		}
		let initialize_price = !accepted_prices.is_empty() && !HasAcceptedBidPrice::<T>::get();
		HistoricalBidsPerSlot::<T>::mutate(|history| {
			if history.is_empty() {
				history.try_push(MiningBidStats::default()).expect("empty history has capacity");
			}
			let stats = &mut history[0];
			stats.bids_count = stats.bids_count.saturating_add(new_bids);
			for (index, price) in accepted_prices.iter().copied().enumerate() {
				let price = price.into();
				stats.bid_amount_min = if initialize_price && index == 0 {
					price
				} else {
					stats.bid_amount_min.min(price)
				};
				stats.bid_amount_max = stats.bid_amount_max.max(price);
				stats.bid_amount_sum = stats.bid_amount_sum.saturating_add(price);
			}
		});
		if initialize_price {
			HasAcceptedBidPrice::<T>::put(true);
		}
	}

	/// Atomic settlement owns all cohort, payment and collateral side effects.
	#[frame_support::transactional]
	pub(crate) fn settle_bids(block: BlockNumberFor<T>) -> DispatchResult {
		let batch = BidBatches::<T>::take(block);
		if batch.is_empty() {
			return Ok(());
		}
		ensure!(
			frame_system::Pallet::<T>::block_number() >=
				block.saturating_add(T::BidRevealBlocks::get().into()),
			Error::<T>::BatchNotMature
		);
		let mut pending = Vec::new();
		for (position, miner) in batch.into_iter().enumerate() {
			let entry = PendingBids::<T>::take(&miner).ok_or(Error::<T>::MissingBidCommitment)?;
			ensure!(entry.frame == NextFrameId::<T>::get(), Error::<T>::WrongBidFrame);
			pending.push((entry, position as u32));
		}
		let old = BidsForNextSlotCohort::<T>::get();
		let funder = |r: &Registration<T>| {
			r.external_funding_account.clone().unwrap_or_else(|| r.account_id.clone())
		};
		let mut available = BTreeMap::<T::AccountId, T::Balance>::new();
		let mut offers = Self::funded_standing_offers(&old, &mut available);
		for (entry, _) in &pending {
			available.entry(entry.funding.clone()).or_insert_with(|| {
				T::ArgonCurrency::reducible_balance(
					&entry.funding,
					Preservation::Protect,
					Fortitude::Polite,
				)
			});
		}
		let increment = T::BidIncrements::get();
		let mut accepted = BTreeSet::new();
		// Freeze eligibility BEFORE considering ANY bid from this submission block.
		let seats = NextCohortSize::<T>::get() as usize;
		let threshold = if !offers.is_empty() && offers.len() >= seats {
			offers.iter().map(|(_, _, budget)| *budget).min()
		} else {
			None
		};
		for (entry, position) in &pending {
			if threshold.is_some_and(|cutoff| entry.bid <= cutoff) {
				Self::deposit_event(Event::BidRejected { account_id: entry.miner.clone() });
				continue;
			}
			let replacement = offers.iter().position(|(r, _, _)| r.account_id == entry.miner);
			let allocated = replacement.map(|i| offers[i].2).unwrap_or_default();
			let free = available.get_mut(&entry.funding).expect("funder inserted");
			// A disclosed maximum is optional. If it cannot be fully funded, preserve the base bid.
			let mut max = entry.max.unwrap_or(entry.bid);
			if *free < max.saturating_sub(allocated) {
				max = entry.bid;
			}
			if *free < max.saturating_sub(allocated) ||
				replacement.is_some_and(|i| max <= offers[i].1.max || max < offers[i].0.bid)
			{
				Self::deposit_event(Event::BidRejected { account_id: entry.miner.clone() });
				continue;
			}
			*free -= max.saturating_sub(allocated);
			let standing = StandingBid::<T> {
				bid: entry.bid,
				max,
				fixed: entry.max_hash.is_none(),
				submitted_at: replacement.map(|i| offers[i].1.submitted_at).unwrap_or(block),
				position: replacement.map(|i| offers[i].1.position).unwrap_or(*position),
			};
			let mut registration = entry.registration.clone();
			registration.bid = replacement.map(|i| offers[i].0.bid).unwrap_or_default();
			accepted.insert(entry.miner.clone());
			if let Some(i) = replacement {
				offers[i] = (registration, standing, max);
			} else {
				offers.push((registration, standing, max));
			}
		}
		offers.sort_by(|a, b| {
			a.1.submitted_at.cmp(&b.1.submitted_at).then(a.1.position.cmp(&b.1.position))
		});
		let candidates = offers
			.iter()
			.map(|(r, s, budget)| crate::auction::Candidate {
				bid: s.bid.max(r.bid).into(),
				max: (*budget).into(),
				fixed: s.fixed,
			})
			.collect::<Vec<_>>();
		let allocation = crate::auction::clear(&candidates, seats, increment.into());
		let mut winners = Vec::new();
		let mut accepted_prices = Vec::new();
		let mut automatic_bids = 0u32;
		for (index, price) in allocation {
			let (r, standing, _) = &offers[index];
			let mut registration = r.clone();
			registration.bid = price.into();
			Self::send_argons_to_pool(
				&funder(r),
				&old.iter().find(|x| x.account_id == r.account_id).cloned(),
				registration.bid,
			)?;
			StandingBids::<T>::insert(&r.account_id, standing.clone());
			let new_submission = accepted.contains(&r.account_id);
			let automatic_increase = !new_submission && registration.bid > r.bid;
			if automatic_increase {
				automatic_bids = automatic_bids.saturating_add(1);
			}
			if new_submission || automatic_increase {
				accepted_prices.push(registration.bid);
			}
			if registration.bid != r.bid {
				Self::deposit_event(Event::WinningBidPriceChanged {
					account_id: r.account_id.clone(),
					bid_amount: registration.bid,
				});
			}
			winners.push(registration);
		}
		for (entry, _) in &pending {
			// A rejected replacement does not replace a still-winning incumbent's keys/offer.
			let accepted_winner = winners.iter().any(|r| r.account_id == entry.miner) &&
				accepted.contains(&entry.miner);
			if !accepted_winner {
				Self::release_argonots_hold(&entry.funding, entry.extra_argonots)?;
				Self::deposit_event(Event::BidLost { account_id: entry.miner.clone() });
			}
		}
		for registration in &old {
			if !winners.iter().any(|r| r.account_id == registration.account_id) {
				StandingBids::<T>::remove(&registration.account_id);
				Self::release_failed_bid(registration)?;
			}
		}
		// Display order follows paid prices; stable ties retain original commitment priority.
		let priorities = offers
			.iter()
			.map(|(r, s, _)| (r.account_id.clone(), (s.submitted_at, s.position)))
			.collect::<BTreeMap<_, _>>();
		winners.sort_by(|a, b| {
			b.bid
				.cmp(&a.bid)
				.then_with(|| priorities[&a.account_id].cmp(&priorities[&b.account_id]))
		});
		for (index, winner) in winners.iter().enumerate() {
			if pending
				.iter()
				.any(|(p, _)| p.miner == winner.account_id && accepted.contains(&p.miner))
			{
				Self::deposit_event(Event::SlotBidderAdded {
					account_id: winner.account_id.clone(),
					bid_amount: winner.bid,
					index: index as u32,
				});
			}
		}
		BidsForNextSlotCohort::<T>::put(
			BoundedVec::<Registration<T>, T::MaxCohortSize>::try_from(winners)
				.map_err(|_| Error::<T>::TooManyBlockRegistrants)?,
		);
		Self::record_bid_stats(automatic_bids, &accepted_prices);
		Ok(())
	}
}

impl<T: Config> BlockRewardAccountsProvider<T::AccountId> for Pallet<T> {
	type Weights = crate::weights::ProviderWeightAdapter<T>;

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
		for (frame_id, cohort) in <MinersByCohort<T>>::iter() {
			if let Some(just_activated_frame) = Self::get_newly_started_frame() &&
				frame_id == just_activated_frame
			{
				// skip just activated frame
				continue;
			}
			for registration in cohort {
				let account = registration.rewards_account();
				result.push((account, registration.starting_frame_id));
			}
		}
		// ensure we add released miners if they've already been processed
		for (_, registration) in ReleasedMinersByAccountId::<T>::get() {
			let account = registration.rewards_account();
			result.push((account, registration.starting_frame_id));
		}
		result
	}

	/// Compute blocks only get rewards prior to registered mining being active
	fn is_compute_block_eligible_for_rewards() -> bool {
		!Self::is_registered_mining_active()
	}
}

impl<T: Config> MiningSlotProvider<T::AccountId> for Pallet<T> {
	type Weights = crate::weights::ProviderWeightAdapter<T>;

	fn has_active_rewards_account_seat(account_id: &T::AccountId) -> bool {
		if T::OwnershipCurrency::balance_on_hold(&HoldReason::RegisterAsMiner.into(), account_id) ==
			0u32.into()
		{
			return false;
		}

		MinersByCohort::<T>::iter().any(|(_, cohort)| {
			cohort.iter().any(|registration| registration.rewards_account() == *account_id)
		})
	}
}

impl<T: Config> AuthorityProvider<T::MiningAuthorityId, T::Block, T::AccountId> for Pallet<T> {
	fn authority_count() -> u32 {
		ActiveMinersCount::<T>::get().into()
	}

	fn get_authority(author: T::AccountId) -> Option<T::MiningAuthorityId> {
		Self::get_mining_authority(&author).map(|x| x.authority_id)
	}

	fn get_winning_managed_authority(
		seal_proof: U256,
		signing_key: Option<T::MiningAuthorityId>,
		best_score: Option<U256>,
	) -> Option<(MiningAuthority<T::MiningAuthorityId, T::AccountId>, U256, Permill)> {
		let nonces = <MinerNonceScoringByCohort<T>>::get();
		let mut best_score = best_score.unwrap_or(U256::MAX);
		let mut best = None;
		let mut scores = vec![];

		let expected_per_miner = Self::get_average_vote_blocks_won_per_miner();
		let current_block = frame_system::Pallet::<T>::block_number();
		for (frame_id, cohort) in nonces {
			for (i, miner_scoring) in cohort.into_iter().enumerate() {
				let score = ClosestMiner::new(seal_proof, miner_scoring)
					.get_score(expected_per_miner, current_block);
				scores.push(score);
				if score < best_score {
					let authority = Self::get_mining_authority_by_index((frame_id, i as u32))?;
					if let Some(ref limit_to_authority) = signing_key &&
						&authority.authority_id != limit_to_authority
					{
						continue;
					}
					best_score = score;
					best = Some(authority);
				}
			}
		}
		let closest = best?;

		scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
		let index = scores
			.iter()
			.position(|&x| x == best_score)
			.expect("should exist, just checked");
		Some((closest, best_score, Permill::from_rational(index as u32, scores.len() as u32)))
	}

	fn get_authority_score(
		seal_proof: U256,
		authority_id: &T::MiningAuthorityId,
		account_id: &T::AccountId,
		ref_block_number: BlockNumberFor<T>,
	) -> Option<U256> {
		let miner_index = AccountIndexLookup::<T>::get(account_id)?;
		let authority = Self::get_mining_authority_by_index(miner_index)?;
		if authority.authority_id != *authority_id {
			return None;
		}
		let nonces = MinerNonceScoringByCohort::<T>::get();
		let scoring_for_frame = nonces.get(&miner_index.0)?;
		let miner_scoring = scoring_for_frame.get(miner_index.1 as usize)?;
		let expected_per_miner = Self::get_average_vote_blocks_won_per_miner();
		let score = ClosestMiner::new(seal_proof, miner_scoring.clone())
			.get_score(expected_per_miner, ref_block_number);
		Some(score)
	}
}

impl<T: Config> Pallet<T> {
	pub fn current_frame_id() -> FrameId {
		NextFrameId::<T>::get().saturating_sub(1)
	}

	pub fn get_newly_started_frame() -> Option<FrameId> {
		NewlyStartedFrameId::<T>::get()
	}

	fn evaluate_newly_started_frame(has_vote_seal: bool) -> Option<FrameId> {
		if let Some(frame_id) = NewlyStartedFrameId::<T>::get() {
			return Some(frame_id);
		}
		let next_frame_id = NextFrameId::<T>::get();
		if next_frame_id == 1 {
			let current_tick = T::TickProvider::current_tick();
			if current_tick >= Self::frame_1_begins_tick() && Self::auction_ready_to_activate() {
				NewlyStartedFrameId::<T>::put(next_frame_id);
				return Some(next_frame_id);
			}
		}
		let mut remaining_frame_ticks = FrameRewardTicksRemaining::<T>::get();
		if Self::is_frame_reward_tick(has_vote_seal) {
			remaining_frame_ticks.saturating_reduce(1u64);
			FrameRewardTicksRemaining::<T>::put(remaining_frame_ticks);
		}
		if remaining_frame_ticks.is_zero() && Self::auction_ready_to_activate() {
			NewlyStartedFrameId::<T>::put(next_frame_id);
			return Some(next_frame_id);
		}
		None
	}

	/// if this is a vote block OR we've fallen into a zero-registered-miner hole, increment
	/// ticks
	fn is_frame_reward_tick(has_vote_seal: bool) -> bool {
		let no_registered_miners =
			ActiveMinersCount::<T>::get().is_zero() && NextFrameId::<T>::get() > 1;
		no_registered_miners || has_vote_seal
	}

	fn has_vote_seal_digest() -> bool {
		if let Some(seal) = <frame_system::Pallet<T>>::digest()
			.logs
			.iter()
			.find_map(|log| log.as_block_seal())
		{
			return seal.is_vote();
		}
		T::SealerInfo::is_block_vote_seal()
	}

	fn should_rotate_grandpas(n: BlockNumberFor<T>) -> bool {
		let rotate_grandpa_blocks =
			UniqueSaturatedInto::<u32>::unique_saturated_into(T::GrandpaRotationBlocks::get());
		let current_block = UniqueSaturatedInto::<u32>::unique_saturated_into(n);
		!HasAddedGrandpaRotation::<T>::get() || current_block.is_multiple_of(rotate_grandpa_blocks)
	}

	pub fn is_registered_mining_active() -> bool {
		NextFrameId::<T>::get() > 1 && ActiveMinersCount::<T>::get() > 0u16
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

	pub(crate) fn record_block_author(author: T::AccountId) {
		let Some(miner_index) = <AccountIndexLookup<T>>::get(&author) else {
			return;
		};
		MinerNonceScoringByCohort::<T>::mutate(|a| {
			if let Some(cohort_nonces) = a.get_mut(&miner_index.0) &&
				let Some(miner_scoring) = cohort_nonces.get_mut(miner_index.1 as usize)
			{
				miner_scoring.blocks_won_in_frame.saturating_accrue(1);
				miner_scoring.last_win_block = Some(frame_system::Pallet::<T>::block_number());
			}
		});
	}

	pub(crate) fn start_new_frame(frame_id: FrameId) {
		HasAcceptedBidPrice::<T>::kill();
		HistoricalBidsPerSlot::<T>::mutate(|bids| {
			if bids.is_full() {
				bids.pop();
			}
			let _ = bids.try_insert(0, MiningBidStats::default());
		});

		let config = MiningConfig::<T>::get();
		FrameRewardTicksRemaining::<T>::put(config.ticks_between_slots);
		let slot_cohort = BidsForNextSlotCohort::<T>::take();
		for winner in &slot_cohort {
			StandingBids::<T>::remove(&winner.account_id);
		}
		LastBidBlock::<T>::kill();
		AuctionAdmissionClosed::<T>::put(false);

		IsNextSlotBiddingOpen::<T>::put(true);

		Self::reset_miner_nonce_scoring();
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
				active_miners.saturating_reduce(1);
				let _ = released_miners_by_account_id.try_insert(account_id.clone(), miner.clone());
				removed_miners.push((account_id, miner.authority_keys.clone()));
				let next_registration =
					slot_cohort.iter().find(|x| x.account_id == miner.account_id);
				Self::release_mining_seat_argonots(&miner, next_registration);
			}
		}

		// After all accounts are removed, we can add the new cohort members (otherwise, we can add
		// and then remove after if sorted differently with same account)
		MinersByCohort::<T>::insert(frame_id, slot_cohort.clone());

		let mut total_bids = T::Balance::zero();
		let mut miner_scoring = vec![];
		for (i, entry) in slot_cohort.iter().enumerate() {
			AccountIndexLookup::<T>::insert(&entry.account_id, (frame_id, i as u32));
			let mining_account =
				entry.external_funding_account.as_ref().unwrap_or(&entry.account_id);
			T::OperationalAccountsHook::mining_seat_won(mining_account);
			added_miners.push((entry.account_id.clone(), entry.authority_keys.clone()));
			active_miners += 1;
			total_bids = total_bids.saturating_add(entry.bid);
			let nonce =
				MinerNonce::<T> { account_id: entry.account_id.clone(), block_hash: parent_hash }
					.generate();
			let scoring = MinerNonceScoring {
				nonce,
				blocks_won_in_frame: 0,
				last_win_block: None,
				frame_start_blocks_won_surplus: 0,
			};

			miner_scoring.push(scoring);
		}

		MinerNonceScoringByCohort::<T>::mutate(|a| {
			if frame_id >= frames_per_term {
				a.retain(|t, _v| *t > frame_id - frames_per_term);
			}
			a.try_insert(frame_id, BoundedVec::truncate_from(miner_scoring))
				.expect("Should be valid since we removed from this map previously");
		});
		AveragePricePerSeat::<T>::mutate(|a| {
			if a.is_full() {
				a.pop();
			}
			let _ = a.try_insert(0, SeatPriceStats { seats: slot_cohort.len() as u32, total_bids });
		});

		ActiveMinersCount::<T>::put(active_miners);
		Pallet::<T>::deposit_event(Event::<T>::NewMiners {
			new_miners: slot_cohort,
			released_miners: removed_miners.len() as u32,
			frame_id,
		});

		ScheduledCohortSizeChangeByFrame::<T>::mutate(|x| {
			if let Some(size) = x.remove(&frame_id) {
				NextCohortSize::<T>::put(size);
			}
			x.retain(|f, _| *f > frame_id);
		});

		ReleasedMinersByAccountId::<T>::put(released_miners_by_account_id);
		NextFrameId::<T>::put(frame_id + 1);
		FrameStartBlockNumbers::<T>::mutate(|a| {
			if a.is_full() {
				a.pop();
			}
			let _ = a.try_insert(0, frame_system::Pallet::<T>::block_number());
		});
		FrameStartTicks::<T>::mutate(|a| {
			if a.is_full() {
				let first_key = *a.keys().next().expect("should have at least one entry");
				a.remove(&first_key);
			}
			let current_tick = T::TickProvider::current_tick();
			let _ = a.try_insert(frame_id, current_tick);
		});

		T::SlotEvents::rotate_grandpas(frame_id, removed_miners, added_miners);
		T::SlotEvents::on_frame_start(frame_id);
	}

	pub(crate) fn get_average_vote_blocks_won_per_miner() -> u16 {
		let starting_miners = ActiveMinersCount::<T>::get();
		if starting_miners == 0 {
			return 0;
		}
		let mut total_block_mined = 0;
		for (_, miners) in MinerNonceScoringByCohort::<T>::get() {
			for miner in miners {
				total_block_mined += miner.blocks_won_in_frame;
			}
		}
		let expected_blocks =
			FixedU128::from_rational(total_block_mined as u128, starting_miners as u128);
		expected_blocks.round().saturating_mul_int(1u16)
	}

	pub(crate) fn reset_miner_nonce_scoring() {
		let starting_miners = ActiveMinersCount::<T>::get();
		if starting_miners == 0 {
			return;
		}
		MinerNonceScoringByCohort::<T>::mutate(|a| {
			let expected_blocks = Self::get_average_vote_blocks_won_per_miner();
			for (_frame_id, miners) in a.iter_mut() {
				for miner in miners.iter_mut() {
					miner.frame_start_blocks_won_surplus +=
						miner.blocks_won_in_frame as i16 - expected_blocks as i16;
					miner.frame_start_blocks_won_surplus = miner
						.frame_start_blocks_won_surplus
						.clamp(-MAX_BLOCK_OFFSET, MAX_BLOCK_OFFSET);
					miner.blocks_won_in_frame = 0;
				}
			}
		});
	}

	/// Update the argonots per seat requirement using the prior-day lower median winning bid and
	/// the previous frame's average Argonot price.
	pub(crate) fn update_argonots_per_seat(activating_frame_id: FrameId) {
		let winning_bids = BidsForNextSlotCohort::<T>::get();
		if winning_bids.is_empty() {
			return;
		}

		let previous_frame_id = activating_frame_id.saturating_sub(1);
		let Some(microgons_per_argonot) =
			T::PriceProvider::get_average_microgons_per_argonot(previous_frame_id)
		else {
			return;
		};
		if microgons_per_argonot.is_zero() {
			return;
		}

		// Winning bids are stored from highest to lowest, so `len / 2` yields the middle bid for
		// odd cohort sizes and the lower median for even cohort sizes.
		let median_bid: u128 = winning_bids[winning_bids.len() / 2].bid.into();
		let required_value_in_microgons =
			median_bid.saturating_mul(T::ArgonotBidCollateralMultiple::get());
		let price_of_one_whole_argonot_in_microgons: u128 = microgons_per_argonot.into();
		// `ArgonotsPerMiningSeat` is stored in base units, so scale the numerator by 1e6 to keep
		// micronot precision while dividing by the price of one whole Argonot.
		let micronots_needed = required_value_in_microgons
			.saturating_mul(argon_primitives::MICROGONS_PER_ARGON)
			.saturating_add(price_of_one_whole_argonot_in_microgons.saturating_sub(1))
			.checked_div(price_of_one_whole_argonot_in_microgons)
			.unwrap_or_default();
		let argonots_needed = T::Balance::from(micronots_needed);

		ArgonotsPerMiningSeat::<T>::put(argonots_needed.max(T::InitialArgonotsPerSeat::get()));
	}

	/// Forecast seats from the winning bids and seat counts of the same completed auctions.
	/// Apply the price adjustment limit to the observed mean cohort size, then announce only a
	/// previously unscheduled change. Existing announcements remain fixed.
	pub(crate) fn adjust_number_of_seats() {
		let frames_per_term = T::FramesPerMiningTerm::get() as FrameId;
		let next_frame_id = NextFrameId::<T>::get();
		let scheduled_frame_id = next_frame_id + frames_per_term;
		let mut schedule = ScheduledCohortSizeChangeByFrame::<T>::get();
		if schedule.contains_key(&scheduled_frame_id) {
			return;
		}

		let price_history = AveragePricePerSeat::<T>::get();
		let observed_frames = price_history.len() as u128;
		if observed_frames == 0 {
			return;
		}

		let mut total_seats = 0u128;
		let mut total_bids = T::Balance::zero();
		for auction in price_history {
			total_seats += auction.seats as u128;
			total_bids = total_bids.saturating_add(auction.total_bids);
		}

		// Mean cohort size * seat-weighted mean bid / target = mean auction bids / target.
		// Clamp the aggregate seat estimate before averaging, preserving the existing price
		// limit without multiplying historical prices by an unrelated future announcement.
		let seats_at_target_price: u128 =
			total_bids.checked_div(&T::TargetPricePerSeat::get()).unwrap_or_default().into();
		let damper = T::PricePerSeatAdjustmentDamper::get();
		let one = FixedU128::one();
		let minimum_adjusted_seats = one.saturating_sub(damper).saturating_mul_int(total_seats);
		let maximum_adjusted_seats = one.saturating_add(damper).saturating_mul_int(total_seats);
		let mean_cohort_size = seats_at_target_price
			.clamp(minimum_adjusted_seats, maximum_adjusted_seats) /
			observed_frames;
		let next_cohort_size = mean_cohort_size
			.saturated_into::<u32>()
			.clamp(T::MinCohortSize::get(), T::MaxCohortSize::get());

		if schedule.try_insert(scheduled_frame_id, next_cohort_size).is_err() {
			log::error!(
				"Unable to schedule cohort size change to {next_cohort_size} at frame {scheduled_frame_id}",
			);
			return;
		}
		ScheduledCohortSizeChangeByFrame::<T>::put(schedule);
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
		let mining_config = MiningConfig::<T>::get();

		let ticks_before_close = mining_config.ticks_before_bid_end_for_vrf_close;
		if ticks_before_close == 0 {
			return false;
		}
		// Are we in the closing eligibility window?
		if FrameRewardTicksRemaining::<T>::get() > ticks_before_close {
			return false;
		}

		// Calculate the threshold for VRF comparison to achieve a probability of 1 in
		// `MiningConfig.ticks_before_bid_end_for_vrf_close`
		let threshold = U256::MAX / U256::from(ticks_before_close);

		if vote_seal_proof < threshold {
			log::info!("VRF Close triggered: {vote_seal_proof:?} < {threshold:?}");
			let frame_id = NextFrameId::<T>::get();
			Self::deposit_event(Event::<T>::MiningBidsClosed { frame_id });
			return true;
		}

		false
	}

	pub fn frame_1_begins_tick() -> Tick {
		let mining_config = MiningConfig::<T>::get();
		let slot_1_ticks =
			mining_config.slot_bidding_start_after_ticks + mining_config.ticks_between_slots;
		let genesis_tick =
			T::TickProvider::current_tick().saturating_sub(T::TickProvider::elapsed_ticks());
		genesis_tick.saturating_add(slot_1_ticks)
	}

	pub(crate) fn get_next_frame_tick() -> Tick {
		T::TickProvider::current_tick() + FrameRewardTicksRemaining::<T>::get()
	}

	pub fn next_frame_id() -> FrameId {
		NextFrameId::<T>::get()
	}

	pub fn get_next_mining_epoch() -> (Tick, Tick) {
		let start_tick = Self::get_next_frame_tick();
		(start_tick, start_tick + Self::get_mining_window_ticks())
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
			ensure!(bid_amount >= existing.bid, Error::<T>::BidCannotBeReduced);
			needed -= existing.bid;
		}
		if needed > 0u32.into() {
			let pool_account = T::MiningBidPoolAccount::get();
			Self::ensure_bid_pool_unreapable(&pool_account);
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
			argonots_needed.saturating_reduce(current_registration.argonots);
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
					"Error recovering mining slot hold for {account_id:?}. Amount {amount:?}. {e:?}"
				);
				Error::<T>::UnrecoverableHold
			})?;

		if T::OwnershipCurrency::balance_on_hold(&reason.into(), account_id) == 0u32.into() {
			let _ = frame_system::Pallet::<T>::dec_providers(account_id);
		}
		Ok(())
	}

	/// Release the argonots from the mining seat. If the Argonots will be re-used in the next
	/// era, we should not unlock the difference.
	pub(crate) fn release_mining_seat_argonots(
		active_registration: &Registration<T>,
		next_registration: Option<&Registration<T>>,
	) {
		let account_id = &active_registration.account_id;

		let funding_account =
			active_registration.external_funding_account.as_ref().unwrap_or(account_id);

		// Only decrease the amount on hold when the miner renews with a lower requirement.
		let amount_to_release = if let Some(next_registration) = next_registration {
			active_registration.argonots.saturating_sub(next_registration.argonots)
		} else {
			active_registration.argonots
		};

		if amount_to_release == T::Balance::zero() {
			return;
		}
		if let Err(e) = Self::release_argonots_hold(funding_account, amount_to_release) {
			log::error!(
				"Failed to release argonots from funding account {funding_account:?} (account {account_id:?}). {e:?}",
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
			.as_ref()
			.unwrap_or(&bid_registration.account_id);
		let account_id = &bid_registration.account_id;

		if bid_registration.bid > T::Balance::zero() {
			let pool_account = T::MiningBidPoolAccount::get();
			Self::ensure_bid_pool_unreapable(&pool_account);
			T::ArgonCurrency::transfer(
				&pool_account,
				funding_account,
				bid_registration.bid,
				Preservation::Expendable,
			)?;
		}

		if let Some(mut pending) = PendingBids::<T>::get(account_id) {
			// The replacement takes over the old collateral; do not release it below.
			pending.extra_argonots = pending.registration.argonots.saturating_sub(
				Self::get_active_registration(account_id)
					.map(|x| x.argonots)
					.unwrap_or_default(),
			);
			PendingBids::<T>::insert(account_id, pending);
			Self::deposit_event(Event::SlotBidderDropped {
				account_id: account_id.clone(),
				preserved_argonot_hold: true,
			});
			return Ok(());
		}

		let mut held_argonots = false;
		let mut argonots_to_unhold: T::Balance = bid_registration.argonots;
		if let Some(active) = Self::get_active_registration(account_id) {
			argonots_to_unhold.saturating_reduce(active.argonots);
			held_argonots = true;
		}

		Self::release_argonots_hold(funding_account, argonots_to_unhold)?;

		Self::deposit_event(Event::<T>::SlotBidderDropped {
			account_id: account_id.clone(),
			preserved_argonot_hold: held_argonots,
		});

		Ok(())
	}

	fn ticks_between_slots() -> Tick {
		MiningConfig::<T>::get().ticks_between_slots
	}

	pub fn bid_pool_balance() -> T::Balance {
		let account_id = T::MiningBidPoolAccount::get();
		T::ArgonCurrency::balance(&account_id)
	}

	fn ensure_bid_pool_unreapable(account_id: &T::AccountId) {
		let providers = frame_system::Pallet::<T>::providers(account_id);
		for _ in providers..2 {
			frame_system::Pallet::<T>::inc_providers(account_id);
		}
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, DebugNoBound, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct MinerNonceScoring<T: Config> {
	pub nonce: U256,
	pub last_win_block: Option<BlockNumberFor<T>>,
	/// Number of blocks won in the current frame
	pub blocks_won_in_frame: u16,
	/// Number of blocks off target set at start of each frame
	pub frame_start_blocks_won_surplus: i16,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, DebugNoBound)]
pub struct MinerNonce<T: Config> {
	pub account_id: T::AccountId,
	pub block_hash: T::Hash,
}

impl<T: Config> MinerNonce<T> {
	pub fn generate(&self) -> U256 {
		let hash = self.using_encoded(blake2_256);
		U256::from_big_endian(&hash)
	}
}

/// The maximum allowed deviation (offset) from the expected number of block wins for a miner in a
/// frame. This limit helps ensure fairness by allowing minor statistical fluctuations, but prevents
/// significant deviations that could indicate manipulation or unfair advantage. The value 5 was
/// chosen given that normal usage stays within a 3-4% threshold and the max miners is 144.
const MAX_BLOCK_OFFSET: i16 = 5;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct ClosestMiner<T: Config> {
	pub seal_proof: U256,
	pub miner_nonce: U256,
	#[codec(skip)]
	pub scoring: MinerNonceScoring<T>,
}

impl<T: Config> ClosestMiner<T> {
	fn new(seal_proof: U256, scoring: MinerNonceScoring<T>) -> Self {
		Self { seal_proof, miner_nonce: scoring.nonce, scoring }
	}

	fn get_score(&self, expected_wins_at_tick: u16, ref_block_number: BlockNumberFor<T>) -> U256 {
		// 1. Create a full U256 from the concatenation of the seal_proof and the miner_nonce
		let hash_bytes = self.using_encoded(blake2_256);

		let mut r2 = [0u8; 2];
		for (i, b) in hash_bytes.iter().enumerate() {
			r2[i & 1] ^= *b;
		}
		let random_base_score = u16::from_le_bytes(r2);

		let wins_against_expected = (self.scoring.blocks_won_in_frame as i16)
			.saturating_sub(expected_wins_at_tick as i16)
			.saturating_add(self.scoring.frame_start_blocks_won_surplus);

		// Apply a penalty to recently closed blocks to prevent miners turning off until the end of
		// a mining term and then turning back on to gain an advantage. Between the cap and this
		// penalty for recency, it should continue to distribute evenly in normal cases, but avoid
		// late-frame monopolization.
		let mut recent_block_penalty = 0i16;
		if let Some(last_win_block) = self.scoring.last_win_block {
			let miners = ActiveMinersCount::<T>::get();
			let ten_percent_of_miners = Percent::from_percent(10).mul_floor(miners as u32);
			if ref_block_number.saturating_sub(last_win_block) <= ten_percent_of_miners.into() {
				recent_block_penalty = 2i16;
			}
		}

		let fairness_baseline = wins_against_expected.saturating_add(recent_block_penalty) as i64;
		let fairness_score =
			fairness_baseline.clamp(-(MAX_BLOCK_OFFSET as i64), MAX_BLOCK_OFFSET as i64);

		// Apply a bounded, *additive* correction based on deviation. Do not scale by
		// `random_base_score` or we risk R + R*multiplier == 0 (or negative wrap),
		// which collapses the distribution. We keep the jitter (`random_base_score`)
		// independent and nudge it by a fixed gain per win delta.
		let multiplier = fairness_score.abs() << 10;
		let score_adjustment = fairness_score.saturating_mul(multiplier);

		let final_score = (random_base_score as i64).saturating_add(score_adjustment);
		let shifted = final_score.saturating_add(i64::MAX / 2); // move negative region upward
		U256::from(shifted as u64)
	}
}

impl<T: Config> MiningFrameProvider for Pallet<T> {
	fn get_next_frame_tick() -> Tick {
		Self::get_next_frame_tick()
	}

	fn is_seat_bidding_started() -> bool {
		T::TickProvider::elapsed_ticks() >= MiningConfig::<T>::get().slot_bidding_start_after_ticks ||
			IsNextSlotBiddingOpen::<T>::get()
	}

	fn get_tick_range_for_frame(frame_id: FrameId) -> Option<(Tick, Tick)> {
		let frame_ticks = FrameStartTicks::<T>::get();
		let starting_tick = frame_ticks.get(&frame_id)?;
		let ending_tick = frame_ticks.get(&(frame_id + 1))?;
		Some((*starting_tick, *ending_tick))
	}
}

impl<T: Config> MiningFrameTransitionProvider for Pallet<T> {
	fn is_new_frame_started() -> Option<FrameId> {
		Self::get_newly_started_frame()
	}
	fn get_current_frame_id() -> FrameId {
		Self::current_frame_id()
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
		if let Some(proof) = vote_seal_proof &&
			IsNextSlotBiddingOpen::<T>::get() &&
			Self::check_for_bidding_close(proof)
		{
			IsNextSlotBiddingOpen::<T>::put(false);
			AuctionAdmissionClosed::<T>::put(true);
		}
	}

	fn block_seal_read_weight(seal: &BlockSealInherent) -> Weight {
		if matches!(seal, BlockSealInherent::Vote { .. }) {
			T::WeightInfo::block_seal_read_vote()
		} else {
			Weight::zero()
		}
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::MiningAuthorityId;
}

sp_api::decl_runtime_apis! {
	/// This runtime api allows people to query the upcoming mining_slot
	pub trait MiningSlotApi<Balance> where Balance: Codec {
		fn next_mining_epoch() -> (Tick, Tick);
		fn bid_pool() -> Balance;
	}
}
