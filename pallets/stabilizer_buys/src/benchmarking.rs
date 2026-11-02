use super::*;
use argon_primitives::{PriceIndex, PriceIndexUpdated};
use frame_support::traits::fungible::Mutate;
use frame_system::RawOrigin;
use pallet_prelude::benchmarking::set_benchmark_treasury_reward_payment_failure;
use polkadot_sdk::frame_benchmarking::v2::*;

const NOW: Tick = Tick::MAX - Tick::MAX % 60 - 120;
type CurrencyOf<T> = <T as Config>::Currency;

fn clock<T: Config + pallet_mining_slot::Config + pallet_ticks::Config>(
	frame: FrameId,
	tick: Tick,
) {
	pallet_mining_slot::NextFrameId::<T>::put(frame + 1);
	pallet_ticks::CurrentTick::<T>::put(tick);
}

fn index(tick: Tick) -> PriceIndex {
	PriceIndex {
		tick,
		btc_usd_price: FixedU128::from_inner(u128::MAX),
		argonot_usd_price: FixedU128::one(),
		argon_usd_price: FixedU128::from_inner(u128::MAX),
		argon_usd_target_price: FixedU128::one(),
		argon_time_weighted_average_liquidity: 1,
		usdc_usd_price: Some(FixedU128::one()),
		stabilizer_pool_sqrt_price_x96: Some(U256::one() << 96),
	}
}

fn prices<T: Config>() {
	for i in 0..T::MaxRecentPriceTicks::get() {
		let tick = NOW - u64::from(i);
		RecentPriceIndices::<T>::insert(tick.to_ordered(), index(tick));
	}
	for i in 0..T::MaxHourlyPriceReferences::get() {
		let tick = NOW - u64::from(i) * T::PriceHistoryBucketTicks::get();
		HourlyPriceReferences::<T>::insert(
			tick.to_ordered(),
			HourlyPriceReference {
				minimum_argon_target_usd: FixedU128::one(),
				usdc: Some((tick, FixedU128::one())),
			},
		);
	}
}

fn pool() -> PurchasePool {
	PurchasePool {
		chain_id: 1,
		pool_manager: H160::repeat_byte(1),
		pool_id: H256::repeat_byte(2),
		usdc_token: H160::repeat_byte(3),
		argon_is_token0: true,
	}
}

fn purchase() -> UniswapPurchase {
	let pool = pool();
	UniswapPurchase {
		chain_id: pool.chain_id,
		pool_manager: pool.pool_manager,
		pool_id: pool.pool_id,
		usdc_token: pool.usdc_token,
		usdc_spent: U256::from(900_000u64),
		principal: 1_000_000,
		sqrt_price_before_x96: U256::one() << 76,
		sqrt_price_after_x96: U256::one() << 77,
		tick: NOW,
	}
}

/// One full payout batch plus a queued bucket, backed by real recipient-owned holds.
fn account_buys<T: Config>(owner: &T::AccountId, mature: bool, offset: u64) -> DispatchResult
where
	CurrencyOf<T>: Mutate<T::AccountId>,
{
	let principal = CurrencyOf::<T>::minimum_balance().max(10_000);
	let count = T::MaxPrincipalPayouts::get() + 1;
	CurrencyOf::<T>::mint_into(owner, principal * (u128::from(count) + 100))?;
	frame_system::Pallet::<T>::inc_providers(owner);
	CurrencyOf::<T>::hold(
		&HoldReason::PurchasePrincipal.into(),
		owner,
		principal * u128::from(count),
	)?;
	let buys = AccountBuys::default();
	for frame in 1..=count {
		Auctions::<T>::insert(
			u64::from(frame) + offset,
			Auction {
				matching_rate: Permill::one(),
				total_matching_basis: principal / 10,
				budget: Some(0),
				remaining: 0,
				matures_at_tick: Some(if mature { NOW } else { NOW + 1 }),
			},
		);
		PurchasesByAccount::<T>::insert(
			owner,
			(u64::from(frame) + offset).to_ordered(),
			BuyPayout {
				principal,
				captured_discount: principal / 10,
				bid_rate: Percent::zero(),
				positive_bid_count: 0,
			},
		);
	}
	Pallet::<T>::save_and_schedule_account(owner, buys);
	if mature {
		// Model an overdue visit so rescheduling must remove and insert distinct keys.
		PayoutQueue::<T>::remove(AscendingRankKey::new(NOW, owner.clone()));
		PayoutQueue::<T>::insert(AscendingRankKey::new(NOW - 1, owner.clone()), ());
		BuysByAccount::<T>::mutate(owner, |buys| buys.next_payout_tick = Some(NOW - 1));
	}
	Ok(())
}

#[benchmarks(where T: pallet_mining_slot::Config + pallet_ticks::Config, CurrencyOf<T>: Mutate<T::AccountId>)]
mod benchmarks {
	use super::*;

	// Both histories are full so adding a new observation also measures oldest-entry eviction.
	#[benchmark]
	fn record_prices() {
		prices::<T>();
		let tick = NOW + <T as Config>::PriceHistoryBucketTicks::get();
		clock::<T>(10, tick);
		#[block]
		{
			Pallet::<T>::on_price_index(&index(tick));
		}
		assert!(RecentPriceIndices::<T>::contains_key(tick.to_ordered()));
		assert!(HourlyPriceReferences::<T>::contains_key(tick.to_ordered()));
	}

	// Admission reads the matching price entries while retaining an older principal backlog.
	#[benchmark]
	fn record_purchase() -> Result<(), BenchmarkError> {
		let owner = account("buyer", 0, 0);
		clock::<T>(u64::from(<T as Config>::MaxPrincipalPayouts::get() + 1) + 10, NOW);
		prices::<T>();
		account_buys::<T>(&owner, true, 0)?;
		let pool = pool();
		ApprovedPool::<T>::put(&pool);
		let purchase = purchase();
		#[block]
		{
			Pallet::<T>::record_transferred_purchase(&owner, &purchase)?;
		}
		assert_eq!(
			PurchasesByAccount::<T>::iter_prefix(&owner).count(),
			(<T as Config>::MaxPrincipalPayouts::get() + 2) as usize
		);
		Ok(())
	}

	// A late reward can already be queued when the account starts a fresh principal hold.
	#[benchmark]
	fn record_purchase_first_hold() -> Result<(), BenchmarkError> {
		let owner = account("first_hold", 0, 0);
		clock::<T>(10, NOW);
		prices::<T>();
		CurrencyOf::<T>::mint_into(&owner, 1_000_000)?;
		let reward = CurrencyOf::<T>::minimum_balance().max(10_000);
		BuysByAccount::<T>::insert(
			&owner,
			AccountBuys { payable_reward: reward, next_payout_tick: Some(NOW - 1) },
		);
		PayoutQueue::<T>::insert(AscendingRankKey::new(NOW - 1, owner.clone()), ());
		let pool = pool();
		ApprovedPool::<T>::put(&pool);
		let purchase = purchase();
		#[block]
		{
			Pallet::<T>::record_transferred_purchase(&owner, &purchase)?;
		}
		assert_eq!(
			CurrencyOf::<T>::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &owner),
			purchase.principal
		);
		assert_eq!(BuysByAccount::<T>::get(&owner).payable_reward, reward);
		assert_eq!(PayoutQueue::<T>::iter_keys().next().unwrap().rank(), NOW);
		Ok(())
	}

	#[benchmark]
	fn bid() -> Result<(), BenchmarkError> {
		let owner = account("bidder", 0, 0);
		let frame = u64::from(<T as Config>::MaxPrincipalPayouts::get() + 1);
		clock::<T>(frame + 1, NOW);
		account_buys::<T>(&owner, false, 0)?;
		PurchasesByAccount::<T>::mutate(&owner, frame.to_ordered(), |payout| {
			let payout = payout.as_mut().unwrap();
			payout.bid_rate = Percent::from_percent(100);
			payout.positive_bid_count = <T as Config>::MaxPositiveBids::get() - 1;
		});
		BidsByAuction::<T>::insert(frame, AscendingRankKey::new(100, owner.clone()), 1_000);
		#[extrinsic_call]
		_(RawOrigin::Signed(owner.clone()), frame, Percent::from_percent(50));
		assert_eq!(
			PurchasesByAccount::<T>::get(&owner, frame.to_ordered()).unwrap().bid_rate,
			Percent::from_percent(50)
		);
		Ok(())
	}

	#[benchmark]
	fn process_auctions(b: Linear<1, 4>) -> Result<(), BenchmarkError> {
		clock::<T>(30_000, NOW);
		let basis = CurrencyOf::<T>::minimum_balance().max(10_000) / 10;
		let award = Percent::from_percent(50).mul_floor(basis);
		let budget = <T as Config>::AuctionBudgetPercent::get().mul_floor(basis);
		for i in 0..b {
			let owner = account("award", i, 0);
			account_buys::<T>(
				&owner,
				true,
				u64::from(i + 1) * u64::from(<T as Config>::MaxPrincipalPayouts::get() + 1),
			)?;
			let frame = u64::from(i) + 1;
			Auctions::<T>::insert(
				frame,
				Auction {
					matching_rate: Permill::one(),
					total_matching_basis: basis,
					budget: Some(budget),
					remaining: budget,
					matures_at_tick: Some(NOW),
				},
			);
			BidsByAuction::<T>::insert(frame, AscendingRankKey::new(50, owner), basis);
			MaturingAuctions::<T>::insert(AscendingRankKey::new(NOW, frame), ());
		}
		#[block]
		{
			for _ in 0..b {
				let next = MaturingAuctions::<T>::iter_keys().next().unwrap();
				Pallet::<T>::allocate_reward(&next)?;
			}
		}
		assert!(MaturingAuctions::<T>::iter_keys().next().is_none());
		assert_eq!(
			BuysByAccount::<T>::iter_values()
				.map(|buys| buys.payable_reward)
				.sum::<Balance>(),
			award * u128::from(b)
		);
		Ok(())
	}

	#[benchmark]
	fn process_payouts(a: Linear<1, 4>) -> Result<(), BenchmarkError> {
		set_benchmark_treasury_reward_payment_failure(false);
		clock::<T>(30_000, NOW);
		let reward = CurrencyOf::<T>::minimum_balance().max(10_000);

		for i in 0..a {
			let owner = account("payout", i, 0);
			account_buys::<T>(
				&owner,
				true,
				u64::from(i) * u64::from(<T as Config>::MaxPrincipalPayouts::get() + 1),
			)?;
			BuysByAccount::<T>::mutate(&owner, |buys| buys.payable_reward = reward);
		}
		#[block]
		{
			let due: Vec<_> = PayoutQueue::<T>::iter_keys().take(a as usize).collect();
			for key in due {
				Pallet::<T>::payout_account(&key.holder_id())?;
			}
		}
		assert!(BuysByAccount::<T>::iter_values().all(|buys| buys.payable_reward == 0));
		assert_eq!(PayoutQueue::<T>::iter_keys().count(), a as usize);
		for i in 0..a {
			let owner: T::AccountId = account("payout", i, 0);
			assert_eq!(PurchasesByAccount::<T>::iter_prefix(&owner).count(), 1);
		}
		Ok(())
	}

	#[benchmark]
	fn process_payouts_failed(a: Linear<1, 4>) -> Result<(), BenchmarkError> {
		clock::<T>(30_000, NOW);
		let reward = CurrencyOf::<T>::minimum_balance().max(10_000);

		for i in 0..a {
			let owner = account("payout", i, 0);
			account_buys::<T>(
				&owner,
				true,
				u64::from(i) * u64::from(<T as Config>::MaxPrincipalPayouts::get() + 1),
			)?;
			BuysByAccount::<T>::mutate(&owner, |buys| buys.payable_reward = reward);
		}
		set_benchmark_treasury_reward_payment_failure(true);
		#[block]
		{
			Pallet::<T>::on_initialize(1u32.into());
		}
		assert_eq!(
			BuysByAccount::<T>::iter_values()
				.map(|buys| buys.payable_reward)
				.sum::<Balance>(),
			reward * u128::from(a)
		);
		set_benchmark_treasury_reward_payment_failure(false);
		assert_eq!(PayoutQueue::<T>::iter_keys().next().unwrap().rank(), NOW + 1);
		Ok(())
	}

	#[benchmark]
	fn on_frame_start() {
		clock::<T>(12, NOW);
		for frame in [10, 11] {
			Auctions::<T>::insert(
				frame,
				Auction {
					matching_rate: Permill::one(),
					total_matching_basis: 1_000_000,
					budget: Some(0),
					remaining: 0,
					matures_at_tick: None,
				},
			);
		}
		#[block]
		{
			<Pallet<T> as OnNewSlot<T::AccountId>>::on_frame_start(12);
		}
		assert!(Auctions::<T>::get(10).unwrap().matures_at_tick.is_some());
		assert!(Auctions::<T>::get(11).unwrap().budget.unwrap() > 0);
	}

	#[benchmark]
	fn set_matching_rate() {
		#[extrinsic_call]
		_(RawOrigin::Root, Permill::from_percent(50));
		assert_eq!(MatchingRate::<T>::get(), Permill::from_percent(50));
	}

	#[benchmark]
	fn set_approved_pool() {
		#[extrinsic_call]
		_(RawOrigin::Root, Some(pool()));
		assert_eq!(ApprovedPool::<T>::get(), Some(pool()));
	}
}
