//! Purchase reward auctions with account-owned principal holds and scheduled payouts.
//! Treasury funds ordinary rewards; Liquid repayment bonuses are funded independently by burns.
//!
//! Purchases accumulate in frame F. The frame hook reserves funding for bidding in F+1, then
//! closes bidding when F+2 starts. Principal matures at admission frame F+30.
//!
//! At maturity, `on_initialize` allocates ranked bids into payable rewards, then pays due accounts.
//! Both queues advance in bounded steps. Principal can be paid before a delayed reward allocation;
//! that allocation schedules the account again when its reward is ready.
//! Bids consume matching-basis inclusion capacity before their requested rate is applied. Lower
//! rates save reward funding without admitting matching basis beyond the auction's cutoff.
//! Purchases enter at 100% without a submitted bid. Burn bonuses follow the share of each buyer's
//! maximum available allocation awarded; Treasury funding proportionally scales ordinary rewards.
#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec::Vec;
use argon_primitives::{
	amount_rank::AmountRankId,
	bitcoin::{LiquidId, LiquidRepayPrices},
	AmountRankKey, AscendingRankKey, BitcoinFissionsProvider, CrosschainGatewayProvider,
	LiquidRepaymentProvider, MiningFrameTransitionProvider,
	MiningFrameTransitionProviderWeightInfo, OnNewSlot, TickProvider,
};
use frame_support::{
	require_transactional,
	traits::tokens::{DepositConsequence, Provenance},
	transactional,
};
use pallet_prelude::*;
use polkadot_sdk::{
	sp_arithmetic::{helpers_128bit::multiply_by_rational_with_rounding, Rounding},
	sp_runtime::ArithmeticError,
};

pub use argon_primitives::{
	ethereum::UniswapPurchase, treasury::TreasuryRewardProvider, UniswapPurchaseProvider,
};
pub use pallet::*;
pub use uniswap_purchase::PurchasePool;
pub use weights::WeightInfo;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
mod price_history;
#[cfg(test)]
mod tests;
mod uniswap_purchase;
pub use price_history::{PriceObservation, PurchasePriceReference};
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<RuntimeEvent: From<Event<Self>>, AccountId: AmountRankId>
	{
		/// ARGON balances and recipient-owned principal holds, in microgons.
		type Currency: Inspect<Self::AccountId, Balance = Balance>
			+ Mutate<Self::AccountId>
			+ MutateHold<Self::AccountId, Reason: From<HoldReason>>;
		/// Frame transitions must invoke this pallet's OnNewSlot exactly once for every frame.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;
		/// Network clock used for purchase pricing and source-time history retention.
		type TickProvider: TickProvider<Self::Block>;
		/// Latest accepted source-chain gateway state; this pallet owns its retention range.
		type CrosschainGatewayProvider: CrosschainGatewayProvider<Balance>;
		/// Maximum observation age at purchase time; also bounds historical lookup reads.
		#[pallet::constant]
		type MaxPriceAgeTicks: Get<Tick>;
		/// Maximum obsolete price entries removed per block after gateway sync advances.
		#[pallet::constant]
		type MaxPriceHistoryPrunesPerBlock: Get<u32>;
		/// Maximum frame visits per repayment attempt; a partial frame visit also reads one tick.
		#[pallet::constant]
		type MaxPriceWindowSteps: Get<u32>;
		/// Treasury owns available funding, outstanding rewards and payout delivery.
		type TreasuryRewardProvider: TreasuryRewardProvider<Self::AccountId, Balance = Balance>;
		/// Fissions owns position selection, repayment pricing and actual burns.
		type FissionsProvider: BitcoinFissionsProvider<Self::AccountId, Balance>;
		/// Maximum account/frame purchase buckets visited in one account payout.
		/// This bounds work, without limiting the number of purchases an account can hold.
		#[pallet::constant]
		type MaxFramesPerPayout: Get<u32>;
		/// Maximum work in each automatic stage per block: ranked bid decisions, then account
		/// payout attempts. Failures and empty-auction completion consume work; excess stays
		/// queued.
		#[pallet::constant]
		type MaxPayoutsPerBlock: Get<u32>;
		/// Positive submissions per account and auction; four allows an initial bid and three
		/// revisions.
		#[pallet::constant]
		type MaxPositiveBids: Get<u8>;
		/// Fraction of total matching basis admitted to rewards, independent of bid rates.
		/// Configure 90%; reserve up to that amount, bounded separately by available Treasury
		/// funding.
		#[pallet::constant]
		type AuctionInclusionPercent: Get<Percent>;
		/// Total mining-frame hold from admission, including the bidding window.
		#[pallet::constant]
		type PurchaseHoldFrames: Get<FrameId>;
		/// Additional burn-funded reward before applying the purchase's auction result.
		#[pallet::constant]
		type LiquidRepaymentRewardPercent: Get<Percent>;
		/// Maximum Liquid repayments attempted across all account payouts in one block.
		#[pallet::constant]
		type MaxLiquidRepaysPerBlock: Get<u32>;
		/// Measured runtime execution costs.
		type WeightInfo: WeightInfo;
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Purchased ARGON awaiting maturity, repayment and principal release.
		PurchasePrincipal,
	}

	#[pallet::type_value]
	pub fn DefaultMatchingRate() -> Permill {
		Permill::one()
	}

	/// Execution venue eligible for purchase rewards. Unset until governance selects a pool.
	#[pallet::storage]
	pub type ApprovedPool<T> = StorageValue<_, PurchasePool>;

	/// Pending repayment commitments, ordered by gateway nonce within each recipient.
	/// The existing account payout queue selects work; this adds no separate settlement queue.
	#[pallet::storage]
	pub type LiquidRepaysByAccountAndGatewayNonce<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Identity,
		<u64 as AmountRankId>::Ordered,
		LiquidRepay,
	>;

	/// Unsettled purchase count for an owner's Liquid. Removed after its final settlement;
	/// lets Fissions block direct close without scanning purchases. Counts use the gateway's
	/// u64 nonce range so every authenticated purchase can be represented.
	#[pallet::storage]
	pub type PendingLiquidRepayCountByAccountAndLiquid<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		LiquidId,
		u64,
		ValueQuery,
	>;

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	/// One authenticated purchase committed to repay the recipient's Liquid.
	pub struct LiquidRepay {
		/// Nonzero recipient-local Liquid selected at purchase time.
		pub liquid_id: LiquidId,
		/// Argon frame in which the proven purchase was applied.
		pub admission_frame: FrameId,
		/// Purchased microgons held until repayment and principal release settle together.
		pub microgons: Balance,
		/// Fraction of the actual burn eligible for an additional reward, before bidding.
		/// Frozen at admission; zero when this purchase is ineligible for buy rewards.
		pub eligible_reward_percent: Percent,
		/// Authenticated purchase time rounded to an Argon tick; no extra backward tolerance.
		pub purchase_tick: Tick,
		/// Target-valued microgons per whole BTC captured at purchase, with fractional microgons.
		pub microgons_at_target_per_btc: FixedU128,
		/// ARGON market-to-target ratio captured at purchase; admission requires a positive quote.
		pub purchase_r_value: FixedU128,
		/// Bounded progress over observations before admission frame F+30. Once complete,
		/// its high-water value remains fixed through payout delays and failures.
		pub price_window: LiquidRepayPriceWindow,
	}

	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	/// Progress calculating one purchase's recovery high water through its repayment window.
	pub struct LiquidRepayPriceWindow {
		/// First unconsumed frame; None means recovery finished or no premium needed a scan.
		pub next_frame: Option<FrameId>,
		/// First observation tick not yet consumed within the purchase's partial frame.
		pub next_tick: Tick,
		/// Highest valid market-to-target ratio in the purchase-to-maturity window.
		pub r_value_high_water: FixedU128,
	}

	/// Number of incomplete windows starting at each purchase tick. Ordered keys expose the
	/// oldest history still needed without scanning purchases; counts share the u64 nonce range.
	#[pallet::storage]
	pub type PendingPriceWindowCountByTick<T> =
		StorageMap<_, Identity, <Tick as AmountRankId>::Ordered, u64, ValueQuery>;

	/// Compact observations keyed by ordered ticks. An unavailable purchase quote stops historical
	/// lookup. Retention follows gateway sync and any incomplete repayment windows.
	#[pallet::storage]
	pub type PriceHistoryByTick<T: Config> =
		StorageMap<_, Identity, <Tick as AmountRankId>::Ordered, PriceObservation>;

	/// Matching fraction for new auctions; existing auctions retain their captured policy.
	#[pallet::storage]
	pub type MatchingRate<T> = StorageValue<_, Permill, ValueQuery, DefaultMatchingRate>;

	/// Auction policy and funding keyed by admission frame F; bidding takes place during F+1.
	#[pallet::storage]
	pub type Auctions<T> = StorageMap<_, Twox64Concat, FrameId, Auction>;

	/// Unpaid ordinary rewards and the account's single scheduled payout visit, across frames.
	#[pallet::storage]
	pub type PayoutStateByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, AccountPayoutState, ValueQuery>;

	/// Aggregated discounts, shared bid and ordinary principal for each account/admission frame.
	/// Ordered frame bytes let payout visits select the oldest buckets with bounded work.
	#[pallet::storage]
	pub type BuysByAccountAndFrame<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Identity,
		<FrameId as AmountRankId>::Ordered,
		AccountFrameBuys,
	>;

	/// Positive bids ordered by rate ascending, matching basis descending, then account ascending.
	/// Keys retain each frozen matching basis in microgons even after principal is released.
	#[pallet::storage]
	pub type BidsByAuction<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		FrameId,
		Identity,
		(Percent, AmountRankKey<T::AccountId>),
		(),
	>;

	/// Closed auctions awaiting reward allocation, ordered by scheduled processing frame and
	/// admission frame. A failed allocation moves only its queue entry; the auction retains its
	/// original maturity.
	#[pallet::storage]
	pub type AuctionAllocationQueue<T> =
		StorageMap<_, Identity, AscendingRankKey<FrameId>, (), OptionQuery>;

	/// One scheduled visit per account, ordered by frame then account. Purchase buckets and the
	/// reward record retain unpaid funds while this index selects bounded work.
	#[pallet::storage]
	pub type PayoutQueue<T: Config> =
		StorageMap<_, Identity, AscendingRankKey<T::AccountId>, (), OptionQuery>;

	/// Fixed due-frame cutoff and last visited key for a bounded pass through the payout queue.
	/// Waiting repayments keep their maturity rank without blocking accounts behind them.
	#[pallet::storage]
	pub type PayoutQueueCursor<T: Config> =
		StorageValue<_, (FrameId, AscendingRankKey<T::AccountId>), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PurchaseRecorded {
			/// Argon recipient owning the held principal and any reward.
			account: T::AccountId,
			/// Successful admission frame, independent of Ethereum purchase time.
			admission_frame: FrameId,
			/// Added principal microgons.
			principal: Balance,
			/// Added matching basis microgons, after account aggregation and rounding.
			matching_basis: Balance,
		},
		PurchaseIneligible {
			/// Recipient whose purchase earns no auction reward; repayment commitments stay held.
			account: T::AccountId,
			/// Authenticated purchase tick used for historical pricing.
			purchase_tick: Tick,
			/// Reason this purchase earns no auction eligibility.
			reason: IneligibleReason,
		},
		PayoutCompleted {
			/// Owner receiving the mature funds.
			account: T::AccountId,
			/// Microgons released from the principal hold.
			principal: Balance,
			/// Total microgons paid from ordinary buy rewards and the Liquid repayment bonus.
			reward: Balance,
		},
		PayoutFailed {
			/// Owner whose amounts remain intact for retry.
			account: T::AccountId,
			/// Transactional payout failure; the queue retries no earlier than the next frame.
			error: DispatchError,
		},
		AuctionOpened {
			/// Admission frame whose bidding window just opened.
			admission_frame: FrameId,
			/// Unissued microgons reserved once.
			budget: Balance,
		},
		AuctionFundingFailed {
			/// Admission frame whose reservation failed.
			admission_frame: FrameId,
			/// The frame-start reservation failed; this auction remains closed to bids.
			error: DispatchError,
		},
		AuctionAllocationFailed {
			/// Admission frame whose next bid or unused-budget return failed.
			admission_frame: FrameId,
			/// The allocation rolled back and will retry no earlier than the next frame.
			error: DispatchError,
		},
		BidSubmitted {
			/// Eligible recipient placing or revising their bid.
			account: T::AccountId,
			/// Admission frame identifying the auction.
			admission_frame: FrameId,
			/// Requested whole percentage of matching basis; zero opts out.
			reward_rate: Percent,
			/// Accepted positive submissions, unchanged by opting out.
			positive_bid_count: u8,
		},
		RewardAwarded {
			/// Recipient of the funded reward.
			account: T::AccountId,
			/// Admission frame identifying the auction.
			admission_frame: FrameId,
			/// Exact microgons committed to the reward, still unissued.
			amount: Balance,
			/// Original maturity frame shared with principal.
			matures_at_frame: FrameId,
		},
		AuctionSettled {
			/// Admission frame whose ranked bids have all been consumed.
			admission_frame: FrameId,
			/// Total microgons committed to rewards.
			awarded: Balance,
			/// Unused microgons returned to available funding.
			returned: Balance,
		},
		ApprovedPoolChanged {
			pool: Option<PurchasePool>,
		},
		MatchingRateChanged {
			/// Matching fraction for subsequently created auctions.
			rate: Permill,
		},
		/// The purchase lacks valid BTC/R prices, or the Liquid is empty, was recently ratcheted,
		/// or lacks the hold plus release window.
		/// Delivered ARGON is processed under the ordinary purchase rules.
		LiquidRepaymentInvalid {
			/// Recipient whose purchase follows the ordinary buy rules.
			account: T::AccountId,
			/// Authenticated Ethereum activity carrying the ignored selection.
			gateway_activity_nonce: u64,
			/// Recipient-local Liquid without a sufficient repayment window.
			liquid_id: LiquidId,
		},

		/// Repayment and its remaining principal were settled in the account payout transaction.
		LiquidRepaymentCompleted {
			/// Argon recipient whose held purchase settled.
			account: T::AccountId,
			/// Authenticated Ethereum gateway activity identifying the purchase.
			gateway_activity_nonce: u64,
			/// Recipient-local Liquid selected when purchasing.
			liquid_id: LiquidId,
			/// Purchased microgons burned at maturity.
			microgons_burned: Balance,
		},
		/// Additional reward funded by completed repayment burns, using the same auction result.
		/// This does not consume the Treasury funding reserved for ordinary buy rewards.
		LiquidRepaymentRewardAwarded {
			/// Recipient of the additional ARGON reward.
			account: T::AccountId,
			/// Purchase cohort whose auction determines the awarded fraction.
			admission_frame: FrameId,
			/// Additional microgons paid in this settlement, independently of Treasury funding.
			amount: Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No auction exists for the supplied admission frame.
		AuctionNotFound,
		/// The account has no purchase bucket for this auction.
		EntryNotFound,
		/// The call is outside F+1 or the auction no longer accepts bids.
		BiddingClosed,
		/// Allocation was requested before the auction's Treasury budget was finalized.
		AuctionNotReady,
		/// A repayment hold has not matured, its price window is incomplete, or its auction
		/// result is not available yet.
		PayoutNotReady,
		/// A positive bid repeats the account's current rate.
		BidUnchanged,
		/// A positive rate produces zero whole microgons against the matching basis.
		BidRoundsToZero,
		/// Positive submissions are exhausted; zero may still opt out.
		BidLimitReached,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			Self::prune_price_history();
			// Allocate due auction bids before selecting this block's account payouts.
			let now = T::MiningFrameTransitionProvider::get_current_frame_id();
			for _ in 0..T::MaxPayoutsPerBlock::get() {
				let Some(key) = AuctionAllocationQueue::<T>::iter_keys().next() else {
					break;
				};
				if key.rank() > now {
					break;
				}

				let admission_frame = key.holder_id();
				if let Err(error) = Self::allocate_reward(&key) {
					AuctionAllocationQueue::<T>::remove(key);
					AuctionAllocationQueue::<T>::insert(
						AscendingRankKey::new(now.saturating_add(1), admission_frame),
						(),
					);
					Self::deposit_event(Event::AuctionAllocationFailed { admission_frame, error });
				}
			}

			// Snapshot due accounts so payments and retries cannot change this block's selected
			// work.
			let due = Self::next_payout_accounts(now, T::MaxPayoutsPerBlock::get());
			let mut repayments = 0u32;
			for rank in due {
				let account = rank.holder_id();
				let mut liquid_repay = None;
				if let Some((nonce, mut repay)) =
					LiquidRepaysByAccountAndGatewayNonce::<T>::iter_prefix(&account).next() &&
					repay.admission_frame.saturating_add(T::PurchaseHoldFrames::get()) <= now
				{
					// A due repayment keeps this account's entire payout together.
					if repayments >= T::MaxLiquidRepaysPerBlock::get() {
						continue;
					}
					repayments += 1;

					if repay.price_window.next_frame.is_some() {
						Self::advance_price_window(&mut repay, T::MaxPriceWindowSteps::get());
						LiquidRepaysByAccountAndGatewayNonce::<T>::insert(&account, nonce, &repay);
					}
					if repay.price_window.next_frame.is_some() {
						continue;
					}
					liquid_repay = Some((u64::from_ordered(&nonce), repay));
				}

				if let Err(error) = Self::payout_account(&account, liquid_repay) {
					let mut payout_state = PayoutStateByAccount::<T>::get(&account);

					let retry_frame = now.saturating_add(1);
					PayoutQueue::<T>::remove(rank);
					PayoutQueue::<T>::insert(
						AscendingRankKey::new(retry_frame, account.clone()),
						(),
					);
					payout_state.next_payout_frame = Some(retry_frame);
					PayoutStateByAccount::<T>::insert(&account, payout_state);
					Self::deposit_event(Event::PayoutFailed { account, error });
				}
			}

			let frame_weight = <T::MiningFrameTransitionProvider as MiningFrameTransitionProvider>::Weights::get_current_frame_id();
			T::WeightInfo::prune_prices(T::MaxPriceHistoryPrunesPerBlock::get())
				.saturating_add(frame_weight)
				.saturating_add(
					T::WeightInfo::process_liquid_repayment().saturating_mul(repayments.into()),
				)
				.saturating_add(
					T::WeightInfo::process_repayment_prices(T::MaxPriceWindowSteps::get())
						.saturating_mul(repayments.into()),
				)
				.saturating_add(T::WeightInfo::process_auctions(T::MaxPayoutsPerBlock::get()))
				.saturating_add(
					T::WeightInfo::process_payouts(T::MaxPayoutsPerBlock::get())
						.max(T::WeightInfo::process_payouts_failed(T::MaxPayoutsPerBlock::get())),
				)
		}

		fn integrity_test() {
			assert!(T::MaxFramesPerPayout::get() > 0, "frame buckets must make progress");
			assert!(T::MaxPayoutsPerBlock::get() > 0, "payout processing must make progress");
			assert!(T::MaxLiquidRepaysPerBlock::get() > 0, "Liquid repayment must make progress");
			assert!(T::PurchaseHoldFrames::get() >= 2, "the hold must include bidding");
			assert!(
				T::MaxPriceHistoryPrunesPerBlock::get() > 0,
				"price pruning must make progress"
			);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit a whole-percent bid during F+1. Larger matching bases break equal-rate ties;
		/// account ID orders bids with the same rate and matching basis.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::bid())]
		pub fn bid(
			origin: OriginFor<T>,
			admission_frame: FrameId,
			reward_rate: Percent,
		) -> DispatchResult {
			let account = ensure_signed(origin)?;
			let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
			let previous_frame = current_frame.checked_sub(1).ok_or(Error::<T>::BiddingClosed)?;
			ensure!(admission_frame == previous_frame, Error::<T>::BiddingClosed);

			let auction = Auctions::<T>::get(admission_frame).ok_or(Error::<T>::AuctionNotFound)?;
			ensure!(auction.budget.is_some(), Error::<T>::BiddingClosed);

			let frame_key = admission_frame.to_ordered();
			let mut frame_buys = BuysByAccountAndFrame::<T>::get(&account, frame_key)
				.ok_or(Error::<T>::EntryNotFound)?;
			let matching_basis = auction.matching_rate.mul_floor(frame_buys.captured_discount);
			if !reward_rate.is_zero() {
				if frame_buys.positive_bid_count > 0 {
					ensure!(reward_rate != frame_buys.bid_rate, Error::<T>::BidUnchanged);
				}
				ensure!(reward_rate.mul_floor(matching_basis) > 0, Error::<T>::BidRoundsToZero);
				ensure!(
					frame_buys.positive_bid_count < T::MaxPositiveBids::get(),
					Error::<T>::BidLimitReached
				);
				frame_buys.positive_bid_count += 1;
			}

			if !frame_buys.bid_rate.is_zero() {
				BidsByAuction::<T>::remove(
					admission_frame,
					(frame_buys.bid_rate, AmountRankKey::new(matching_basis, account.clone())),
				);
			}
			if !reward_rate.is_zero() {
				BidsByAuction::<T>::insert(
					admission_frame,
					(reward_rate, AmountRankKey::new(matching_basis, account.clone())),
					(),
				);
			}
			frame_buys.bid_rate = reward_rate;
			frame_buys.liquid_repayment_reward_rate =
				if reward_rate.is_zero() { Some(FixedU128::zero()) } else { None };
			let positive_bid_count = frame_buys.positive_bid_count;
			BuysByAccountAndFrame::<T>::insert(&account, frame_key, frame_buys);

			Self::deposit_event(Event::BidSubmitted {
				account,
				admission_frame,
				reward_rate,
				positive_bid_count,
			});
			Ok(())
		}

		/// Select the reward pool, or disable new purchase eligibility.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_approved_pool())]
		pub fn set_approved_pool(
			origin: OriginFor<T>,
			pool: Option<PurchasePool>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ApprovedPool::<T>::set(pool.clone());
			Self::deposit_event(Event::ApprovedPoolChanged { pool });
			Ok(())
		}

		/// Change matching for newly created auctions; existing policy remains frozen.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_matching_rate())]
		pub fn set_matching_rate(origin: OriginFor<T>, rate: Permill) -> DispatchResult {
			ensure_root(origin)?;
			MatchingRate::<T>::put(rate);
			Self::deposit_event(Event::MatchingRateChanged { rate });
			Ok(())
		}
	}
}

impl<T: Config> UniswapPurchaseProvider<T::AccountId> for Pallet<T> {
	type Weights = weights::ProviderWeightAdapter<T>;

	/// Called once after gateway delivery, with admission isolated from the delivered funds.
	/// An error rolls back admission and leaves delivery spendable; the gateway still consumes
	/// the nonce. Successful repayment commitments remain held until maturity.
	#[require_transactional]
	fn record_transferred_purchase(
		account: &T::AccountId,
		purchase: &UniswapPurchase,
	) -> DispatchResult {
		let admission_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
		let price_reference = Self::price_reference(purchase.tick);
		let reward_prices = price_reference.map(|(_, prices)| prices);
		let discount = Self::eligible_discount(purchase, reward_prices);
		let eligible_reward_percent =
			if discount.is_ok() { T::LiquidRepaymentRewardPercent::get() } else { Percent::zero() };

		// Repayment needs a usable purchase-time quote as well as a valid Liquid.
		let liquid_repayment = purchase.repay_liquid_id.and_then(|liquid_id| {
			let (capture_frame, prices) = price_reference?;
			if !T::FissionsProvider::is_liquid_repayment_valid(
				account,
				liquid_id,
				purchase.tick,
				T::PurchaseHoldFrames::get(),
			) {
				return None;
			}
			let terms = LiquidRepayPrices::new(
				prices.microgons_at_target_per_btc,
				prices.r_value,
				prices.r_value,
			)?;

			Some(LiquidRepay {
				liquid_id,
				admission_frame,
				microgons: purchase.principal,
				eligible_reward_percent,
				purchase_tick: purchase.tick,
				microgons_at_target_per_btc: prices.microgons_at_target_per_btc,
				purchase_r_value: prices.r_value,
				price_window: LiquidRepayPriceWindow {
					next_frame: terms.has_refundable_premium().then_some(capture_frame),
					next_tick: purchase.tick,
					r_value_high_water: FixedU128::zero(),
				},
			})
		});
		if let Some(liquid_id) = purchase.repay_liquid_id &&
			liquid_repayment.is_none()
		{
			Self::deposit_event(Event::LiquidRepaymentInvalid {
				account: account.clone(),
				gateway_activity_nonce: purchase.gateway_activity_nonce,
				liquid_id,
			});
		}

		// Reward ineligibility does not cancel a valid Liquid repayment commitment.
		let captured_discount = match discount {
			Ok(amount) => amount,
			Err(reason) => {
				Self::deposit_event(Event::PurchaseIneligible {
					account: account.clone(),
					purchase_tick: purchase.tick,
					reason,
				});
				if liquid_repayment.is_none() {
					return Ok(());
				}
				0
			},
		};

		if !PayoutStateByAccount::<T>::contains_key(account) {
			Self::save_account_payout_state(account, Some(AccountPayoutState::default()))?;
		}

		let mut auction = Auctions::<T>::get(admission_frame).unwrap_or(Auction {
			matching_rate: MatchingRate::<T>::get(),
			total_matching_basis: 0,
			budget: None,
			remaining_budget: 0,
			remaining_matching_basis: 0,
			matures_at_frame: admission_frame.saturating_add(T::PurchaseHoldFrames::get()),
		});

		let frame_key = admission_frame.to_ordered();
		let mut frame_buys =
			BuysByAccountAndFrame::<T>::get(account, frame_key).unwrap_or(AccountFrameBuys {
				principal: 0,
				captured_discount: 0,
				bid_rate: Percent::one(),
				positive_bid_count: 0,

				pending_liquid_repays: 0,
				liquid_repayment_reward_rate: Some(FixedU128::zero()),
			});

		let previous_basis = auction.matching_rate.mul_floor(frame_buys.captured_discount);
		frame_buys.captured_discount.saturating_accrue(captured_discount);

		if let Some(repay) = liquid_repayment {
			// Only a refundable premium needs recovery history. Every repayment keeps its hold.
			if repay.price_window.next_frame.is_some() {
				PendingPriceWindowCountByTick::<T>::mutate(purchase.tick.to_ordered(), |count| {
					count.saturating_accrue(1)
				});
			}

			frame_buys.pending_liquid_repays.saturating_accrue(1);

			PendingLiquidRepayCountByAccountAndLiquid::<T>::mutate(
				account,
				repay.liquid_id,
				|count| count.saturating_accrue(1),
			);

			LiquidRepaysByAccountAndGatewayNonce::<T>::insert(
				account,
				purchase.gateway_activity_nonce.to_ordered(),
				repay,
			);
		} else {
			frame_buys.principal.saturating_accrue(purchase.principal);
		}

		let matching_basis = auction.matching_rate.mul_floor(frame_buys.captured_discount);
		let added_basis = matching_basis - previous_basis;
		auction.total_matching_basis.saturating_accrue(added_basis);

		if added_basis > 0 {
			if previous_basis > 0 {
				BidsByAuction::<T>::remove(
					admission_frame,
					(frame_buys.bid_rate, AmountRankKey::new(previous_basis, account.clone())),
				);
			}
			BidsByAuction::<T>::insert(
				admission_frame,
				(frame_buys.bid_rate, AmountRankKey::new(matching_basis, account.clone())),
				(),
			);
			frame_buys.liquid_repayment_reward_rate = None;
		}

		T::Currency::hold(&HoldReason::PurchasePrincipal.into(), account, purchase.principal)?;

		Auctions::<T>::insert(admission_frame, auction);
		BuysByAccountAndFrame::<T>::insert(account, frame_key, frame_buys);
		Self::save_and_schedule_account(account, PayoutStateByAccount::<T>::get(account))?;

		Self::deposit_event(Event::PurchaseRecorded {
			account: account.clone(),
			admission_frame,
			principal: purchase.principal,
			matching_basis: added_basis,
		});
		Ok(())
	}
}

impl<T: Config> LiquidRepaymentProvider<T::AccountId> for Pallet<T> {
	type Weights = weights::ProviderWeightAdapter<T>;

	fn has_pending_repayment(account: &T::AccountId, liquid_id: LiquidId) -> bool {
		PendingLiquidRepayCountByAccountAndLiquid::<T>::contains_key(account, liquid_id)
	}

	fn oldest_pending_purchase_tick() -> Option<Tick> {
		PendingPriceWindowCountByTick::<T>::iter_keys()
			.next()
			.map(|key| Tick::from_ordered(&key))
	}
}

impl<T: Config> OnNewSlot<T::AccountId> for Pallet<T> {
	type Key = argon_primitives::BlockSealAuthorityId;
	fn on_frame_start(frame: FrameId) {
		// Close bidding; its two-frame window is already included in the admission hold.
		if let Some(admission_frame) = frame.checked_sub(2) &&
			let Some(mut auction) = Auctions::<T>::get(admission_frame)
		{
			if auction.budget.is_none() {
				// Failed Treasury funding must still allocate the burn-funded rewards.
				auction.budget = Some(0);
				auction.remaining_matching_basis =
					T::AuctionInclusionPercent::get().mul_floor(auction.total_matching_basis);
				Auctions::<T>::insert(admission_frame, &auction);
			}
			AuctionAllocationQueue::<T>::insert(
				AscendingRankKey::new(auction.matures_at_frame, admission_frame),
				(),
			);
		}

		// Open bidding for purchases admitted in the previous frame.
		let Some(admission_frame) = frame.checked_sub(1) else {
			return;
		};
		let opening = with_storage_layer(|| -> DispatchResult {
			let Some(mut auction) = Auctions::<T>::get(admission_frame) else {
				return Ok(());
			};

			let available = T::TreasuryRewardProvider::available_budget();
			let inclusion_limit =
				T::AuctionInclusionPercent::get().mul_floor(auction.total_matching_basis);
			let budget = available.min(inclusion_limit);

			T::TreasuryRewardProvider::reserve_budget(budget)?;
			auction.budget = Some(budget);
			auction.remaining_budget = budget;
			auction.remaining_matching_basis = inclusion_limit;
			Auctions::<T>::insert(admission_frame, auction);
			Self::deposit_event(Event::AuctionOpened { admission_frame, budget });
			Ok(())
		});
		if let Err(error) = opening {
			Self::deposit_event(Event::AuctionFundingFailed { admission_frame, error });
		}
	}

	fn on_frame_start_weight(_: FrameId) -> Weight {
		T::WeightInfo::on_frame_start()
	}
}

impl<T: Config> Pallet<T> {
	/// Visit the due queue in bounded passes, rotating past accounts still waiting on settlement.
	fn next_payout_accounts(now: FrameId, limit: u32) -> Vec<AscendingRankKey<T::AccountId>> {
		let cursor = PayoutQueueCursor::<T>::get();
		let mut cutoff = cursor.as_ref().map_or(now, |(frame, _)| *frame);
		let mut entries = if let Some((_, key)) = &cursor {
			PayoutQueue::<T>::iter_keys_from(PayoutQueue::<T>::hashed_key_for(key))
		} else {
			PayoutQueue::<T>::iter_keys()
		};
		let mut due: Vec<_> = entries
			.by_ref()
			.take_while(|key| key.rank() <= cutoff)
			.take(limit as usize)
			.collect();

		// Finish the old pass before admitting newly due frames. Do not select a key twice.
		if let Some((_, previous_key)) = &cursor &&
			due.len() < limit as usize
		{
			let previous_key = previous_key.encode();
			let tail_is_empty = due.is_empty();
			cutoff = now;
			let remaining = limit as usize - due.len();
			due.extend(
				PayoutQueue::<T>::iter_keys()
					.take_while(|key| key.rank() <= cutoff)
					.take_while(|key| tail_is_empty || key.encode() <= previous_key)
					.take(remaining),
			);
		}

		if let Some(last_key) = due.last() {
			PayoutQueueCursor::<T>::put((cutoff, last_key));
		} else if cursor.is_some() {
			PayoutQueueCursor::<T>::kill();
		}
		due
	}

	/// Burn one held purchase's repayment and return its unused principal and burn-funded bonus.
	/// The caller releases principal and pays any receivable bonus in the same payout transaction.
	#[require_transactional]
	fn settle_liquid_repay(
		account: &T::AccountId,
		gateway_activity_nonce: u64,
		repay: LiquidRepay,
	) -> Result<(Balance, Balance), DispatchError> {
		let frame = T::MiningFrameTransitionProvider::get_current_frame_id();
		ensure!(
			frame >= repay.admission_frame.saturating_add(T::PurchaseHoldFrames::get()),
			Error::<T>::PayoutNotReady
		);
		ensure!(repay.price_window.next_frame.is_none(), Error::<T>::PayoutNotReady);

		let frame_key = repay.admission_frame.to_ordered();
		let mut frame_buys =
			BuysByAccountAndFrame::<T>::get(account, frame_key).ok_or(Error::<T>::EntryNotFound)?;
		let awarded_reward_fraction =
			frame_buys.liquid_repayment_reward_rate.ok_or(Error::<T>::PayoutNotReady)?;
		let repayment_prices = LiquidRepayPrices::new(
			repay.microgons_at_target_per_btc,
			repay.purchase_r_value,
			repay.price_window.r_value_high_water,
		)
		.ok_or(ArithmeticError::Overflow)?;

		let burned_microgons = T::FissionsProvider::repay_liquid(
			account,
			repay.liquid_id,
			repay.microgons,
			repayment_prices,
			|amount| {
				T::Currency::burn_held(
					&HoldReason::PurchasePrincipal.into(),
					account,
					amount,
					Precision::Exact,
					Fortitude::Force,
				)
				.map(|_| ())
			},
		)?;
		let unused_principal = repay
			.microgons
			.checked_sub(burned_microgons)
			.ok_or(ArithmeticError::Underflow)?;
		let liquid_reward = awarded_reward_fraction
			.saturating_mul_int(repay.eligible_reward_percent.mul_floor(burned_microgons));

		frame_buys.pending_liquid_repays.saturating_reduce(1);
		BuysByAccountAndFrame::<T>::insert(account, frame_key, frame_buys);
		LiquidRepaysByAccountAndGatewayNonce::<T>::remove(
			account,
			gateway_activity_nonce.to_ordered(),
		);
		PendingLiquidRepayCountByAccountAndLiquid::<T>::mutate_exists(
			account,
			repay.liquid_id,
			|count| {
				let pending_repays = count.unwrap_or_default().saturating_sub(1);
				*count = (pending_repays > 0).then_some(pending_repays);
			},
		);
		Self::deposit_event(Event::LiquidRepaymentCompleted {
			account: account.clone(),
			gateway_activity_nonce,
			liquid_id: repay.liquid_id,
			microgons_burned: burned_microgons,
		});
		Ok((unused_principal, liquid_reward))
	}

	fn eligible_discount(
		purchase: &UniswapPurchase,
		prices: Option<PurchasePriceReference>,
	) -> Result<Balance, IneligibleReason> {
		// A whole-frame release must be depositable even if the owner spends all liquid funds.
		if purchase.principal < T::Currency::minimum_balance() {
			return Err(IneligibleReason::PrincipalBelowMinimum);
		}
		let approved_pool = ApprovedPool::<T>::get().ok_or(IneligibleReason::UnapprovedPool)?;
		if !approved_pool.matches_purchase(purchase) {
			return Err(IneligibleReason::UnapprovedPool);
		}

		let prices = prices.ok_or(IneligibleReason::PricesUnavailable)?;
		if prices.argon_usd_target_price.is_zero() {
			return Err(IneligibleReason::PricesUnavailable);
		}
		if prices.microgons_at_target_per_usdc.is_zero() {
			return Err(IneligibleReason::PricesUnavailable);
		}

		let discount = approved_pool
			.captured_discount(purchase, prices)
			.map_err(|_| IneligibleReason::InvalidEvidence)?;
		if discount == 0 {
			return Err(IneligibleReason::NoDiscount);
		}
		Ok(discount)
	}

	/// Allocate one ranked bid and return any final unused budget in the same transaction.
	/// A failed attempt leaves bid priority and funding intact.
	#[transactional]
	fn allocate_reward(key: &AscendingRankKey<FrameId>) -> DispatchResult {
		let admission_frame = key.holder_id();
		let mut auction = Auctions::<T>::get(admission_frame).ok_or(Error::<T>::AuctionNotFound)?;
		let reserved_budget = auction.budget.ok_or(Error::<T>::AuctionNotReady)?;
		let matures_at_frame = auction.matures_at_frame;
		let matching_basis_limit =
			T::AuctionInclusionPercent::get().mul_floor(auction.total_matching_basis);

		let next_bid = BidsByAuction::<T>::iter_key_prefix(admission_frame).next();
		if let Some((rate, rank)) = next_bid {
			let account = rank.holder_id();
			let included_basis = rank.amount().min(auction.remaining_matching_basis);
			auction.remaining_matching_basis.saturating_reduce(included_basis);

			let awarded_basis = rate.mul_floor(included_basis);
			// Every award uses the original reservation's funding ratio.
			let ordinary_reward = if matching_basis_limit > 0 {
				multiply_by_rational_with_rounding(
					awarded_basis,
					reserved_budget,
					matching_basis_limit,
					Rounding::Down,
				)
				.ok_or(ArithmeticError::Overflow)?
				.min(auction.remaining_budget)
			} else {
				0
			};
			let frame_key = admission_frame.to_ordered();
			if let Some(mut frame_buys) = BuysByAccountAndFrame::<T>::get(&account, frame_key) &&
				frame_buys.pending_liquid_repays > 0
			{
				// Scale the burn bonus by the buyer's awarded share, independently of Treasury
				// funding. A sole 100% bidder can earn the full bonus despite the auction's
				// inclusion cap.
				let maximum_buyer_basis = rank.amount().min(matching_basis_limit);
				let awarded_reward_fraction = if maximum_buyer_basis > 0 {
					FixedU128::checked_from_rational(awarded_basis, maximum_buyer_basis)
						.ok_or(ArithmeticError::DivisionByZero)?
						.min(FixedU128::one())
				} else {
					FixedU128::zero()
				};

				frame_buys.liquid_repayment_reward_rate = Some(awarded_reward_fraction);
				BuysByAccountAndFrame::<T>::insert(&account, frame_key, frame_buys);
			}

			if ordinary_reward > 0 {
				T::TreasuryRewardProvider::commit_reward(ordinary_reward)?;
				let mut payout_state = PayoutStateByAccount::<T>::get(&account);
				payout_state.payable_reward.saturating_accrue(ordinary_reward);
				Self::save_and_schedule_account(&account, payout_state)?;
				auction.remaining_budget.saturating_reduce(ordinary_reward);
				Self::deposit_event(Event::RewardAwarded {
					account: account.clone(),
					admission_frame,
					amount: ordinary_reward,
					matures_at_frame,
				});
			}
			BidsByAuction::<T>::remove(admission_frame, (rate, rank));
		}

		if BidsByAuction::<T>::iter_prefix(admission_frame).next().is_none() {
			T::TreasuryRewardProvider::return_budget(auction.remaining_budget)?;
			Self::deposit_event(Event::AuctionSettled {
				admission_frame,
				awarded: reserved_budget - auction.remaining_budget,
				returned: auction.remaining_budget,
			});
			auction.remaining_budget = 0;
			AuctionAllocationQueue::<T>::remove(key);
		}
		Auctions::<T>::insert(admission_frame, auction);
		Ok(())
	}

	/// Pay an account after the hook has excluded any due repayment still waiting for its window
	/// or this block's work allowance. All financial effects share this transaction.
	/// `None` means the account has no due repayment.
	#[transactional]
	fn payout_account(
		account: &T::AccountId,
		liquid_repay: Option<(u64, LiquidRepay)>,
	) -> DispatchResult {
		let current_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
		if let Some((_, repay)) = liquid_repay.as_ref() {
			let frame_buys =
				BuysByAccountAndFrame::<T>::get(account, repay.admission_frame.to_ordered())
					.ok_or(Error::<T>::EntryNotFound)?;
			if frame_buys.liquid_repayment_reward_rate.is_none() {
				return Ok(());
			}
		}

		// Settle one Liquid repayment, then collect the account's mature ordinary-buy principal.
		let (mut principal, liquid_reward) = if let Some((nonce, repay)) = liquid_repay {
			let admission_frame = repay.admission_frame;
			let (principal, liquid_reward) = Self::settle_liquid_repay(account, nonce, repay)?;
			(principal, Some((admission_frame, liquid_reward)))
		} else {
			(0, None)
		};

		let mut mature_frame_buys = Vec::new();
		for (frame_key, frame_buys) in BuysByAccountAndFrame::<T>::iter_prefix(account)
			.take(T::MaxFramesPerPayout::get() as usize)
		{
			let auction = Auctions::<T>::get(FrameId::from_ordered(&frame_key))
				.ok_or(Error::<T>::AuctionNotFound)?;
			if auction.matures_at_frame > current_frame {
				break;
			}
			principal.saturating_accrue(frame_buys.principal);

			let awaiting_reward = frame_buys.principal == 0 &&
				!frame_buys.bid_rate.is_zero() &&
				auction.remaining_budget > 0 &&
				BidsByAuction::<T>::contains_key(
					FrameId::from_ordered(&frame_key),
					(
						frame_buys.bid_rate,
						AmountRankKey::new(
							auction.matching_rate.mul_floor(frame_buys.captured_discount),
							account.clone(),
						),
					),
				);
			mature_frame_buys.push((frame_key, frame_buys, awaiting_reward));
		}

		let mut payout_state = PayoutStateByAccount::<T>::get(account);
		let treasury_reward = payout_state.payable_reward;
		let payout_attempted = principal > 0 ||
			treasury_reward > 0 ||
			liquid_reward.is_some_and(|(_, amount)| amount > 0);
		if !payout_attempted && mature_frame_buys.is_empty() {
			return Self::save_and_schedule_account(account, payout_state);
		}

		// Deliver principal and both reward sources, accounting for unreceivable dust.
		let (principal, total_reward) =
			Self::pay_principal_and_rewards(account, principal, treasury_reward, liquid_reward)?;

		// Clear settled principal, retaining records needed by repayments or reward allocation.
		for (frame_key, mut frame_buys, awaiting_reward) in mature_frame_buys {
			if awaiting_reward || frame_buys.pending_liquid_repays > 0 {
				frame_buys.principal = 0;
				BuysByAccountAndFrame::<T>::insert(account, frame_key, frame_buys);
			} else {
				BuysByAccountAndFrame::<T>::remove(account, frame_key);
			}
		}

		payout_state.payable_reward = 0;
		Self::save_and_schedule_account(account, payout_state)?;
		if payout_attempted {
			Self::deposit_event(Event::PayoutCompleted {
				account: account.clone(),
				principal,
				reward: total_reward,
			});
		}
		Ok(())
	}

	/// Order payments so one can make another receivable; burn a principal remainder only if
	/// it still cannot be released after rewards. Return amounts actually delivered.
	#[require_transactional]
	fn pay_principal_and_rewards(
		account: &T::AccountId,
		mut principal: Balance,
		treasury_reward: Balance,
		liquid_reward: Option<(FrameId, Balance)>,
	) -> Result<(Balance, Balance), DispatchError> {
		let (liquid_admission_frame, liquid_reward) = match liquid_reward {
			Some((frame, amount)) => (Some(frame), amount),
			None => (None, 0),
		};
		let mut total_reward = treasury_reward.saturating_add(liquid_reward);
		let free_after_release = T::Currency::balance(account).saturating_add(principal);
		let release_principal_last =
			principal > 0 && free_after_release < T::Currency::minimum_balance();
		let reason = HoldReason::PurchasePrincipal.into();

		// A small remainder may need its rewards credited before it can be released.
		if principal > 0 && !release_principal_last {
			T::Currency::release(&reason, account, principal, Precision::Exact)?;
		}

		// Credit the larger reward first so it can make the smaller one receivable.
		let pay_treasury_first = treasury_reward >= liquid_reward;
		if treasury_reward > 0 && pay_treasury_first {
			let paid = T::TreasuryRewardProvider::pay_reward(account, treasury_reward)?;
			total_reward.saturating_reduce(treasury_reward.saturating_sub(paid));
		}
		if let Some(admission_frame) = liquid_admission_frame &&
			liquid_reward > 0
		{
			match T::Currency::mint_into(account, liquid_reward) {
				Ok(_) => Self::deposit_event(Event::LiquidRepaymentRewardAwarded {
					account: account.clone(),
					admission_frame,
					amount: liquid_reward,
				}),
				Err(error) if error == TokenError::BelowMinimum.into() => {
					// The repayment already burned these funds; leave unreceivable dust unissued.
					total_reward.saturating_reduce(liquid_reward);
				},
				Err(error) => return Err(error),
			}
		}
		if treasury_reward > 0 && !pay_treasury_first {
			let paid = T::TreasuryRewardProvider::pay_reward(account, treasury_reward)?;
			total_reward.saturating_reduce(treasury_reward.saturating_sub(paid));
		}

		if release_principal_last {
			if T::Currency::can_deposit(account, principal, Provenance::Extant) ==
				DepositConsequence::BelowMinimum
			{
				T::Currency::burn_held(
					&reason,
					account,
					principal,
					Precision::Exact,
					Fortitude::Force,
				)?;
				principal = 0;
			} else {
				T::Currency::release(&reason, account, principal, Precision::Exact)?;
			}
		}

		Ok((principal, total_reward))
	}

	/// Persist the account and its single frame-scheduled payout entry inside its caller's
	/// transaction.
	#[require_transactional]
	fn save_and_schedule_account(
		account: &T::AccountId,
		mut payout_state: AccountPayoutState,
	) -> DispatchResult {
		if let Some(frame) = payout_state.next_payout_frame.take() {
			PayoutQueue::<T>::remove(AscendingRankKey::new(frame, account.clone()));
		}

		let now = T::MiningFrameTransitionProvider::get_current_frame_id();
		let mut next_frame;
		if payout_state.payable_reward > 0 {
			next_frame = now;
		} else if let Some((frame_key, frame_buys)) =
			BuysByAccountAndFrame::<T>::iter_prefix(account).next()
		{
			// Maturity follows the admission frame, independently of auction bookkeeping.
			next_frame =
				FrameId::from_ordered(&frame_key).saturating_add(T::PurchaseHoldFrames::get());
			if frame_buys.principal == 0 &&
				frame_buys.pending_liquid_repays == 0 &&
				next_frame <= now
			{
				next_frame = now.saturating_add(1);
			}
		} else {
			return Self::save_account_payout_state(account, None);
		}

		payout_state.next_payout_frame = Some(next_frame);
		PayoutQueue::<T>::insert(AscendingRankKey::new(next_frame, account.clone()), ());
		Self::save_account_payout_state(account, Some(payout_state))
	}

	/// Each pending buy account owns one provider reference until its principal and rewards
	/// are settled. Acquire it before holding funds, and release it with the final account row.
	fn save_account_payout_state(
		account: &T::AccountId,
		payout_state: Option<AccountPayoutState>,
	) -> DispatchResult {
		PayoutStateByAccount::<T>::try_mutate_exists(account, |stored| {
			match (&*stored, &payout_state) {
				(None, Some(_)) => {
					frame_system::Pallet::<T>::inc_providers(account);
				},
				(Some(_), None) => {
					frame_system::Pallet::<T>::dec_providers(account)?;
				},
				_ => {},
			}

			*stored = payout_state;
			Ok(())
		})
	}
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
/// Shared reward auction for purchases admitted in one mining frame.
pub struct Auction {
	/// Matching fraction frozen at the first purchase assigned to this auction.
	pub matching_rate: Permill,
	/// Sum of account matching bases in microgons; maintained as purchases aggregate.
	pub total_matching_basis: Balance,
	/// Frozen Treasury reservation in microgons. None while unresolved; Some(0) means no funds
	/// were reserved, including when funding failed before bidding closed.
	pub budget: Option<Balance>,
	/// Reserved microgons still available for rewards, zero after unused funding is returned.
	pub remaining_budget: Balance,
	/// Matching-basis microgons still admitted before applying bid rates. Cheap bids never restore
	/// this capacity; the final included bid may use only part of its matching basis.
	pub remaining_matching_basis: Balance,
	/// Admission frame plus the total hold; bidding does not extend it.
	pub matures_at_frame: FrameId,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
/// All purchases admitted for one account in one frame, sharing the same auction bid.
pub struct AccountFrameBuys {
	/// Ordinary purchase microgons still held for this account and auction; repayment funds
	/// remain on their individual LiquidRepay records until settlement.
	pub principal: Balance,
	/// Whole-purchase net discount in target-valued microgons, aggregated before matching.
	pub captured_discount: Balance,
	/// Requested whole percentage of the matching basis; starts at 100%, and zero opts out.
	pub bid_rate: Percent,
	/// Accepted positive submissions, including revisions; opting out does not reset this.
	pub positive_bid_count: u8,

	/// Number of held Liquid repayments in this frame; cannot exceed the u64 gateway nonce.
	pub pending_liquid_repays: u64,
	/// Awarded matching basis divided by the buyer's maximum available basis under the inclusion
	/// cap, independent of Treasury funding. None while the request awaits allocation; reused by
	/// each Liquid repayment in this admission frame. A zero basis has no reward entitlement.
	pub liquid_repayment_reward_rate: Option<FixedU128>,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Debug,
	Default,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
/// Unpaid ordinary rewards and payout scheduling for one account across all admission frames.
pub struct AccountPayoutState {
	/// Mature funded microgons awaiting delivery through Treasury's payout adapter.
	pub payable_reward: Balance,
	/// Frame of this account's single queue entry, including retries after a failed payout.
	pub next_payout_frame: Option<FrameId>,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub enum IneligibleReason {
	/// Principal is below the currency minimum, so release into an empty account could fail.
	PrincipalBelowMinimum,
	/// The source chain, pool manager, pool ID or spending token differs from the approved pool.
	UnapprovedPool,
	/// A purchase-time target or USDC/USD observation is absent or zero.
	PricesUnavailable,
	/// A pool square-root price is zero, exceeds uint160, or cannot be converted safely.
	InvalidEvidence,
	/// The starting gap, price direction or rounded net discount does not qualify.
	NoDiscount,
}
