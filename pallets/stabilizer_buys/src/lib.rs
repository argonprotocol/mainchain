//! Purchase reward auctions with account-owned principal holds and scheduled payouts.
//! Treasury owns reward funding; its payout adapter can later credit exact amounts to Collect.
//!
//! Purchases accumulate in frame F. The frame hook reserves funding for bidding in F+1, then
//! fixes the maturity tick when F+2 starts. Principal stays held until that tick.
//!
//! At maturity, `on_initialize` allocates ranked bids into payable rewards, then pays due accounts.
//! Both queues advance in bounded steps. Principal can be paid before a delayed reward allocation;
//! that allocation schedules the account again when its reward is ready.
#![deny(warnings)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::vec::Vec;
use argon_primitives::{
	amount_rank::AmountRankId, AscendingRankKey, MiningFrameTransitionProvider, OnNewSlot,
	PriceIndex,
};
use frame_support::{storage::with_storage_layer, transactional};
use pallet_prelude::*;

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
pub use price_history::{HourlyPriceReference, PurchasePriceReference};
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
			+ MutateHold<Self::AccountId, Reason: From<HoldReason>>;
		/// Frame transitions must invoke this pallet's OnNewSlot exactly once for every frame.
		type MiningFrameTransitionProvider: MiningFrameTransitionProvider;
		/// Network tick used to anchor principal and reward maturity at bidding close.
		type CurrentTick: Get<Tick>;
		/// Most recent tick observations retained for exact ARGON targets; configure 360.
		#[pallet::constant]
		type MaxRecentPriceTicks: Get<u32>;
		/// Tick width of each older reference bucket; configure 60.
		#[pallet::constant]
		type PriceHistoryBucketTicks: Get<Tick>;
		/// Maximum hourly reference buckets retained; configure 720.
		#[pallet::constant]
		type MaxHourlyPriceReferences: Get<u32>;
		/// In the open bucket, allow USDC observations up to ten ticks before the purchase.
		#[pallet::constant]
		type MaxCurrentHourPriceAgeTicks: Get<Tick>;
		/// Treasury owns available funding, outstanding rewards and payout delivery.
		type TreasuryRewardProvider: TreasuryRewardProvider<Self::AccountId, Balance = Balance>;
		/// Maximum mature principal buckets released in one account payout visit.
		/// This bounds work, without limiting the number of purchases an account can hold.
		#[pallet::constant]
		type MaxPrincipalPayouts: Get<u32>;
		/// Maximum work in each automatic stage per block: ranked bid decisions, then account
		/// payout attempts. Failures and empty-auction completion consume work; excess stays
		/// queued.
		#[pallet::constant]
		type MaxPayoutsPerBlock: Get<u32>;
		/// Positive submissions per account and auction; four allows an initial bid and three
		/// revisions.
		#[pallet::constant]
		type MaxPositiveBids: Get<u8>;
		/// Fraction of min(available funding, matching basis) reserved once; configure 90%.
		#[pallet::constant]
		type AuctionBudgetPercent: Get<Percent>;
		/// Ticks after bidding close, configured for 30 elapsed days on the network.
		#[pallet::constant]
		type RewardMaturityTicks: Get<Tick>;
		/// Measured runtime execution costs.
		type WeightInfo: WeightInfo;
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// All admitted principal, retained until its auction's full maturity interval elapses.
		PurchasePrincipal,
	}

	#[pallet::type_value]
	pub fn DefaultMatchingRate() -> Permill {
		Permill::one()
	}

	/// Execution venue eligible for purchase rewards. Unset until governance selects a pool.
	#[pallet::storage]
	pub type ApprovedPool<T> = StorageValue<_, PurchasePool>;

	/// Exact observations keyed by the oracle tick in big-endian order, for oldest-first pruning.
	#[pallet::storage]
	pub type RecentPriceIndices<T: Config> =
		CountedStorageMap<_, Identity, <Tick as AmountRankId>::Ordered, PriceIndex>;

	/// Hourly references keyed by the bucket-start tick in the same oldest-first order.
	#[pallet::storage]
	pub type HourlyPriceReferences<T: Config> =
		CountedStorageMap<_, Identity, <Tick as AmountRankId>::Ordered, HourlyPriceReference>;

	/// Matching fraction for new auctions; existing auctions retain their captured policy.
	#[pallet::storage]
	pub type MatchingRate<T> = StorageValue<_, Permill, ValueQuery, DefaultMatchingRate>;

	/// Auction policy and funding keyed by admission frame F; bidding takes place during F+1.
	#[pallet::storage]
	pub type Auctions<T> = StorageMap<_, Twox64Concat, FrameId, Auction>;

	/// Scheduled rewards and the account's single payout queue entry.
	#[pallet::storage]
	pub type BuysByAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, AccountBuys, ValueQuery>;

	/// One principal bucket per account and admission frame, retained until release succeeds.
	/// Ordered frame bytes let payout visits select the oldest buckets with bounded work.
	#[pallet::storage]
	pub type PurchasesByAccount<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Identity,
		<FrameId as AmountRankId>::Ordered,
		BuyPayout,
	>;

	/// Positive bids ordered by rate ascending, then account ascending. Values are frozen matching
	/// bases in microgons, allowing settlement after the account's principal bucket is gone.
	#[pallet::storage]
	pub type BidsByAuction<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		FrameId,
		Identity,
		AscendingRankKey<T::AccountId>,
		Balance,
	>;

	/// Funded auctions awaiting automatic reward allocation, ordered by maturity tick and frame.
	/// A failed allocation moves only its retry tick; the auction retains its original maturity.
	#[pallet::storage]
	pub type MaturingAuctions<T> =
		StorageMap<_, Identity, AscendingRankKey<FrameId>, (), OptionQuery>;

	/// One scheduled visit per account, ordered by tick then account. Purchase buckets and the
	/// reward record retain unpaid funds while this index selects bounded work.
	#[pallet::storage]
	pub type PayoutQueue<T: Config> =
		StorageMap<_, Identity, AscendingRankKey<T::AccountId>, (), OptionQuery>;

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
			/// Recipient whose delivered principal remains spendable.
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
			/// Funded microgons delivered through the Treasury payout adapter.
			reward: Balance,
		},
		PayoutFailed {
			/// Owner whose amounts remain intact for retry.
			account: T::AccountId,
			/// Transactional payout failure; the queue retries no earlier than the next tick.
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
			/// The allocation rolled back and will retry no earlier than the next tick.
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
			/// Original maturity tick shared with principal.
			matures_at_tick: Tick,
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
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No auction exists for the supplied admission frame.
		AuctionNotFound,
		/// The account has no purchase bucket for this auction.
		EntryNotFound,
		/// The call is outside F+1 or the auction no longer accepts bids.
		BiddingClosed,
		/// An auction scheduled for allocation lacks its reservation or maturity anchor.
		AuctionNotReady,
		/// The scheduled account has no mature principal or committed reward to pay.
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
			// Allocate due auction bids before selecting this block's account payouts.
			let now = T::CurrentTick::get();
			for _ in 0..T::MaxPayoutsPerBlock::get() {
				let Some(key) = MaturingAuctions::<T>::iter_keys().next() else {
					break;
				};
				if key.rank() > now {
					break;
				}

				let admission_frame = key.holder_id();
				if let Err(error) = Self::allocate_reward(&key) {
					MaturingAuctions::<T>::remove(key);
					MaturingAuctions::<T>::insert(
						AscendingRankKey::new(now.saturating_add(1), admission_frame),
						(),
					);
					Self::deposit_event(Event::AuctionAllocationFailed { admission_frame, error });
				}
			}

			// Snapshot due accounts so payments and retries cannot change this block's selected
			// work.
			let due: Vec<_> = PayoutQueue::<T>::iter_keys()
				.take_while(|rank| rank.rank() <= now)
				.take(T::MaxPayoutsPerBlock::get() as usize)
				.collect();
			for rank in due {
				let account = rank.holder_id();
				if let Err(error) = Self::payout_account(&account) {
					let mut buys = BuysByAccount::<T>::get(&account);
					if error == Error::<T>::PayoutNotReady.into() {
						Self::save_and_schedule_account(&account, buys);
						continue;
					}

					let retry_tick = now.saturating_add(1);
					PayoutQueue::<T>::remove(rank);
					PayoutQueue::<T>::insert(
						AscendingRankKey::new(retry_tick, account.clone()),
						(),
					);
					buys.next_payout_tick = Some(retry_tick);
					BuysByAccount::<T>::insert(&account, buys);
					Self::deposit_event(Event::PayoutFailed { account, error });
				}
			}

			T::WeightInfo::process_auctions(T::MaxPayoutsPerBlock::get()).saturating_add(
				T::WeightInfo::process_payouts(T::MaxPayoutsPerBlock::get())
					.max(T::WeightInfo::process_payouts_failed(T::MaxPayoutsPerBlock::get())),
			)
		}

		fn integrity_test() {
			assert!(T::MaxPrincipalPayouts::get() > 0, "principal payouts must make progress");
			assert!(T::MaxPayoutsPerBlock::get() > 0, "payout processing must make progress");
			assert!(T::MaxRecentPriceTicks::get() > 0, "recent price history must retain ticks");
			assert!(
				T::MaxHourlyPriceReferences::get() > 0,
				"hourly price history must retain buckets"
			);
			assert!(
				T::PriceHistoryBucketTicks::get() > 0,
				"price history buckets must contain ticks"
			);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit a whole-percent bid during F+1. Account ID breaks equal-rate ties.
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
			let mut payout = PurchasesByAccount::<T>::get(&account, frame_key)
				.ok_or(Error::<T>::EntryNotFound)?;
			let matching_basis = auction.matching_rate.mul_floor(payout.captured_discount);
			if !reward_rate.is_zero() {
				ensure!(reward_rate != payout.bid_rate, Error::<T>::BidUnchanged);
				ensure!(reward_rate.mul_floor(matching_basis) > 0, Error::<T>::BidRoundsToZero);
				ensure!(
					payout.positive_bid_count < T::MaxPositiveBids::get(),
					Error::<T>::BidLimitReached
				);
				payout.positive_bid_count += 1;
			}

			if !payout.bid_rate.is_zero() {
				BidsByAuction::<T>::remove(
					admission_frame,
					AscendingRankKey::new(
						u64::from(payout.bid_rate.deconstruct()),
						account.clone(),
					),
				);
			}
			if !reward_rate.is_zero() {
				BidsByAuction::<T>::insert(
					admission_frame,
					AscendingRankKey::new(u64::from(reward_rate.deconstruct()), account.clone()),
					matching_basis,
				);
			}
			payout.bid_rate = reward_rate;
			let positive_bid_count = payout.positive_bid_count;
			PurchasesByAccount::<T>::insert(&account, frame_key, payout);

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

	fn record_transferred_purchase(
		account: &T::AccountId,
		purchase: &UniswapPurchase,
	) -> DispatchResult {
		Self::record_transferred_purchase(account, purchase)
	}
}

impl<T: Config> OnNewSlot<T::AccountId> for Pallet<T> {
	type Key = argon_primitives::BlockSealAuthorityId;
	fn on_frame_start(frame: FrameId) {
		// Close bidding for purchases admitted two frames ago and anchor their maturity.
		if let Some(admission_frame) = frame.checked_sub(2) {
			Auctions::<T>::mutate(admission_frame, |record| {
				let Some(auction) = record else {
					return;
				};
				let maturity = T::CurrentTick::get().saturating_add(T::RewardMaturityTicks::get());
				auction.matures_at_tick = Some(maturity);
				if auction.budget.is_some() {
					MaturingAuctions::<T>::insert(
						AscendingRankKey::new(maturity, admission_frame),
						(),
					);
				} else {
					// Failed funding never extends bidding or the principal hold.
					Self::deposit_event(Event::AuctionSettled {
						admission_frame,
						awarded: 0,
						returned: 0,
					});
				}
			});
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
			let budget = T::AuctionBudgetPercent::get()
				.mul_floor(available.min(auction.total_matching_basis));

			T::TreasuryRewardProvider::reserve_budget(budget)?;
			auction.budget = Some(budget);
			auction.remaining = budget;
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
	/// Called once after authenticated gateway delivery, inside its per-activity transaction.
	/// Gateway replay protection owns nonce consumption. Ineligible purchases stay spendable.
	#[transactional]
	pub fn record_transferred_purchase(
		account: &T::AccountId,
		purchase: &UniswapPurchase,
	) -> DispatchResult {
		let discount = match Self::eligible_discount(purchase) {
			Ok(amount) => amount,
			Err(reason) => {
				Self::deposit_event(Event::PurchaseIneligible {
					account: account.clone(),
					purchase_tick: purchase.tick,
					reason,
				});
				return Ok(());
			},
		};
		let admission_frame = T::MiningFrameTransitionProvider::get_current_frame_id();
		let mut auction = Auctions::<T>::get(admission_frame).unwrap_or(Auction {
			matching_rate: MatchingRate::<T>::get(),
			total_matching_basis: 0,
			budget: None,
			remaining: 0,
			matures_at_tick: None,
		});

		let frame_key = admission_frame.to_ordered();
		let mut payout = PurchasesByAccount::<T>::get(account, frame_key).unwrap_or(BuyPayout {
			principal: 0,
			captured_discount: 0,
			bid_rate: Percent::zero(),
			positive_bid_count: 0,
		});
		let previous_basis = auction.matching_rate.mul_floor(payout.captured_discount);
		payout.captured_discount.saturating_accrue(discount);
		payout.principal.saturating_accrue(purchase.principal);
		let added_basis =
			auction.matching_rate.mul_floor(payout.captured_discount) - previous_basis;
		auction.total_matching_basis.saturating_accrue(added_basis);

		let reason = HoldReason::PurchasePrincipal.into();
		if T::Currency::balance_on_hold(&reason, account).is_zero() {
			// Preserve an account whose entire delivered balance moves into the principal hold.
			frame_system::Pallet::<T>::inc_providers(account);
		}
		T::Currency::hold(&reason, account, purchase.principal)?;
		Auctions::<T>::insert(admission_frame, auction);
		PurchasesByAccount::<T>::insert(account, frame_key, payout);
		Self::save_and_schedule_account(account, BuysByAccount::<T>::get(account));

		Self::deposit_event(Event::PurchaseRecorded {
			account: account.clone(),
			admission_frame,
			principal: purchase.principal,
			matching_basis: added_basis,
		});
		Ok(())
	}

	fn eligible_discount(purchase: &UniswapPurchase) -> Result<Balance, IneligibleReason> {
		// A whole-frame release must be depositable even if the owner spends all liquid funds.
		if purchase.principal < T::Currency::minimum_balance() {
			return Err(IneligibleReason::PrincipalBelowMinimum);
		}
		let approved_pool = ApprovedPool::<T>::get().ok_or(IneligibleReason::UnapprovedPool)?;
		if !approved_pool.matches_purchase(purchase) {
			return Err(IneligibleReason::UnapprovedPool);
		}

		let prices =
			Self::price_reference(purchase.tick).ok_or(IneligibleReason::PricesUnavailable)?;
		if prices.argon_target_usd.is_zero() {
			return Err(IneligibleReason::PricesUnavailable);
		}
		if prices.usdc_usd.is_zero() {
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
		let budget = auction.budget.ok_or(Error::<T>::AuctionNotReady)?;
		let matures_at_tick = auction.matures_at_tick.ok_or(Error::<T>::AuctionNotReady)?;

		let next_bid = BidsByAuction::<T>::iter_prefix(admission_frame).next();
		if let Some((rank, matching_basis)) = next_bid {
			let account = rank.holder_id();
			let rate = Percent::from_percent(rank.rank() as u8);
			let amount = rate.mul_floor(matching_basis).min(auction.remaining);
			if amount > 0 {
				T::TreasuryRewardProvider::commit_reward(amount)?;
				let mut buys = BuysByAccount::<T>::get(&account);
				buys.payable_reward.saturating_accrue(amount);
				Self::save_and_schedule_account(&account, buys);
				auction.remaining.saturating_reduce(amount);
				Self::deposit_event(Event::RewardAwarded {
					account,
					admission_frame,
					amount,
					matures_at_tick,
				});
			}
			BidsByAuction::<T>::remove(admission_frame, rank);
		}

		if BidsByAuction::<T>::iter_prefix(admission_frame).next().is_none() {
			T::TreasuryRewardProvider::return_budget(auction.remaining)?;
			Self::deposit_event(Event::AuctionSettled {
				admission_frame,
				awarded: budget - auction.remaining,
				returned: auction.remaining,
			});
			auction.remaining = 0;
			MaturingAuctions::<T>::remove(key);
		}
		Auctions::<T>::insert(admission_frame, auction);
		Ok(())
	}

	#[transactional]
	fn payout_account(account: &T::AccountId) -> DispatchResult {
		let mut buys = BuysByAccount::<T>::get(account);
		let now = T::CurrentTick::get();
		let mut principal: Balance = 0;
		let mut mature_frames = Vec::new();
		for (frame_key, payout) in PurchasesByAccount::<T>::iter_prefix(account)
			.take(T::MaxPrincipalPayouts::get() as usize)
		{
			let auction = Auctions::<T>::get(FrameId::from_ordered(&frame_key))
				.ok_or(Error::<T>::AuctionNotFound)?;
			let Some(maturity) = auction.matures_at_tick else {
				break;
			};
			if maturity > now {
				break;
			}
			principal.saturating_accrue(payout.principal);
			mature_frames.push(frame_key);
		}
		let reward = buys.payable_reward;
		ensure!(principal > 0 || reward > 0, Error::<T>::PayoutNotReady);

		if principal > 0 {
			let reason = HoldReason::PurchasePrincipal.into();
			T::Currency::release(&reason, account, principal, Precision::Exact)?;
			if T::Currency::balance_on_hold(&reason, account).is_zero() {
				frame_system::Pallet::<T>::dec_providers(account)?;
			}
		}
		if reward > 0 {
			T::TreasuryRewardProvider::pay_reward(account, reward)?;
		}

		for frame_key in mature_frames {
			PurchasesByAccount::<T>::remove(account, frame_key);
		}
		buys.payable_reward = 0;
		Self::save_and_schedule_account(account, buys);

		Self::deposit_event(Event::PayoutCompleted { account: account.clone(), principal, reward });
		Ok(())
	}

	/// Persist account amounts and replace its queue entry together. Before bidding closes,
	/// schedule a lower-bound visit; that visit reschedules against the actual close-anchored
	/// maturity.
	fn save_and_schedule_account(account: &T::AccountId, mut buys: AccountBuys) {
		if let Some(tick) = buys.next_payout_tick.take() {
			PayoutQueue::<T>::remove(AscendingRankKey::new(tick, account.clone()));
		}

		let now = T::CurrentTick::get();
		let next_tick;
		if buys.payable_reward > 0 {
			next_tick = now;
		} else if let Some((frame_key, _)) = PurchasesByAccount::<T>::iter_prefix(account).next() {
			next_tick = Auctions::<T>::get(FrameId::from_ordered(&frame_key))
				.and_then(|auction| auction.matures_at_tick)
				.unwrap_or_else(|| {
					now.saturating_add(T::RewardMaturityTicks::get()).max(now.saturating_add(1))
				});
		} else {
			BuysByAccount::<T>::remove(account);
			return;
		}

		buys.next_payout_tick = Some(next_tick);
		PayoutQueue::<T>::insert(AscendingRankKey::new(next_tick, account.clone()), ());
		BuysByAccount::<T>::insert(account, buys);
	}
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct Auction {
	/// Matching fraction frozen at the first purchase assigned to this auction.
	pub matching_rate: Permill,
	/// Sum of account matching bases in microgons; maintained as purchases aggregate.
	pub total_matching_basis: Balance,
	/// Original reserved microgons. None means reservation has not succeeded; Some(0) is funded.
	pub budget: Option<Balance>,
	/// Reserved microgons still available for rewards, zero after unused funding is returned.
	pub remaining: Balance,
	/// Tick when principal and rewards become payable, captured once when bidding closes.
	pub matures_at_tick: Option<Tick>,
}

#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct BuyPayout {
	/// Delivered microgons still held for this account and auction.
	pub principal: Balance,
	/// Whole-purchase net discount in target-valued microgons, aggregated before matching.
	pub captured_discount: Balance,
	/// Current requested whole percentage of the matching basis; zero opts out.
	pub bid_rate: Percent,
	/// Accepted positive submissions, including revisions; opting out does not reset this.
	pub positive_bid_count: u8,
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
pub struct AccountBuys {
	/// Mature funded microgons awaiting delivery through Treasury's payout adapter.
	pub payable_reward: Balance,
	/// Tick of this account's single queue entry; includes retry scheduling after a failure.
	pub next_payout_tick: Option<Tick>,
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
