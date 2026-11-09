use super::*;
use argon_primitives::{PriceIndex, PriceIndexUpdated};
use frame_support::traits::fungible::Mutate;
use frame_system::RawOrigin;
use pallet_prelude::benchmarking::{
	set_benchmark_bitcoin_locks_runtime_state, set_benchmark_treasury_reward_payment_failure,
	BenchmarkBitcoinLocksRuntimeState,
};
use polkadot_sdk::frame_benchmarking::v2::*;

const CURRENT_FRAME: FrameId = 30_000;
const NOW: Tick = Tick::MAX - Tick::MAX % 60 - 120;
type CurrencyOf<T> = <T as Config>::Currency;

fn clock(frame: FrameId, tick: Tick) {
	set_benchmark_bitcoin_locks_runtime_state(BenchmarkBitcoinLocksRuntimeState {
		current_frame_id: frame,
		current_tick: tick,
		did_start_new_frame: false,
	});
}

fn index(tick: Tick) -> PriceIndex {
	PriceIndex {
		tick,
		btc_usd_price: FixedU128::from_u32(60_000),
		argonot_usd_price: FixedU128::one(),
		argon_usd_price: FixedU128::from_inner(u128::MAX),
		argon_usd_target_price: FixedU128::one(),
		argon_time_weighted_average_liquidity: 1,
		usdc_usd_price: Some(FixedU128::one()),
		stabilizer_pool_sqrt_price_x96: Some(U256::one() << 96),
	}
}

fn prices<T: Config>() {
	// Admission measures the full bounded lookback, with only the oldest eligible tick present.
	let tick = NOW.saturating_sub(T::MaxPriceAgeTicks::get());
	PriceHistoryByTick::<T>::insert(
		tick.to_ordered(),
		PriceObservation {
			frame: 0,
			r_value: FixedU128::from_rational(6, 10),
			prices: Some(PurchasePriceReference {
				r_value: FixedU128::from_rational(6, 10),
				..PurchasePriceReference::from_price_index(&index(tick)).unwrap()
			}),
		},
	);
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
		gateway_activity_nonce: 1,
		repay_liquid_id: Some(1),
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
	let count = T::MaxFramesPerPayout::get() + 1;
	CurrencyOf::<T>::mint_into(owner, principal * (u128::from(count) + 100))?;
	CurrencyOf::<T>::hold(
		&HoldReason::PurchasePrincipal.into(),
		owner,
		principal * u128::from(count),
	)?;
	let payout_state = AccountPayoutState::default();
	for frame in 1..=count {
		Auctions::<T>::insert(
			u64::from(frame) + offset,
			Auction {
				matching_rate: Permill::one(),
				total_matching_basis: principal / 10,
				budget: Some(0),
				remaining_budget: 0,
				remaining_matching_basis: 0,
				matures_at_frame: if mature { CURRENT_FRAME } else { CURRENT_FRAME + 1 },
			},
		);
		BuysByAccountAndFrame::<T>::insert(
			owner,
			(u64::from(frame) + offset).to_ordered(),
			AccountFrameBuys {
				principal,
				captured_discount: principal / 10,
				bid_rate: Percent::zero(),
				positive_bid_count: 0,

				pending_liquid_repays: 0,
				liquid_repayment_reward_rate: None,
			},
		);
	}
	with_storage_layer(|| Pallet::<T>::save_and_schedule_account(owner, payout_state))?;
	if mature {
		// Model an overdue visit so rescheduling must remove and insert distinct keys.
		if let Some(frame) = PayoutStateByAccount::<T>::get(owner).next_payout_frame {
			PayoutQueue::<T>::remove(AscendingRankKey::new(frame, owner.clone()));
		}
		PayoutQueue::<T>::insert(AscendingRankKey::new(CURRENT_FRAME - 1, owner.clone()), ());
		PayoutStateByAccount::<T>::mutate(owner, |payout_state| {
			payout_state.next_payout_frame = Some(CURRENT_FRAME - 1)
		});
	}
	Ok(())
}

#[benchmarks(where CurrencyOf<T>: Mutate<T::AccountId>)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn oldest_pending_purchase_tick() {
		PendingPriceWindowCountByTick::<T>::insert(1u64.to_ordered(), 1);
		let oldest;
		#[block]
		{
			oldest = Pallet::<T>::oldest_pending_purchase_tick();
		}
		assert_eq!(oldest, Some(1));
	}

	#[benchmark]
	fn process_repayment_prices(p: Linear<1, 64>) {
		let owner: T::AccountId = account("window", 0, 0);
		let mut repay = LiquidRepay {
			liquid_id: 1,
			admission_frame: 1,
			microgons: 1_000_000,
			eligible_reward_percent: Percent::zero(),
			purchase_tick: 1,
			microgons_at_target_per_btc: PurchasePriceReference::from_price_index(&index(1))
				.unwrap()
				.microgons_at_target_per_btc,
			purchase_r_value: FixedU128::from_rational(6, 10),
			price_window: LiquidRepayPriceWindow {
				next_frame: Some(0),
				next_tick: 1,
				r_value_high_water: FixedU128::zero(),
			},
		};
		PendingPriceWindowCountByTick::<T>::insert(1u64.to_ordered(), 1);
		for tick in 1..=u64::from(p) {
			PriceHistoryByTick::<T>::insert(
				tick.to_ordered(),
				PriceObservation {
					frame: 0,
					r_value: FixedU128::from_rational(7, 10),
					prices: PurchasePriceReference::from_price_index(&index(tick)),
				},
			);
		}
		#[block]
		{
			// Include completion and the durable checkpoint as well as the partial-frame reads.
			Pallet::<T>::advance_price_window(&mut repay, p + 2);
			LiquidRepaysByAccountAndGatewayNonce::<T>::insert(&owner, 1u64.to_ordered(), &repay);
		}
		assert_eq!(repay.price_window.next_frame, None);
		assert_eq!(repay.price_window.r_value_high_water, FixedU128::from_rational(7, 10));
		assert!(!PendingPriceWindowCountByTick::<T>::contains_key(1u64.to_ordered()));
	}

	#[benchmark]
	fn record_prices() {
		clock(10, NOW);
		#[block]
		{
			Pallet::<T>::on_price_index(&index(NOW));
		}
		assert!(PriceHistoryByTick::<T>::get(NOW.to_ordered()).unwrap().prices.is_some());
	}

	#[benchmark]
	fn prune_prices(p: Linear<1, 16>) {
		clock(10, 1_000);
		PendingPriceWindowCountByTick::<T>::insert(900u64.to_ordered(), 1);

		for tick in 1..=u64::from(p) {
			PriceHistoryByTick::<T>::insert(
				tick.to_ordered(),
				PriceObservation {
					frame: 0,
					r_value: FixedU128::one(),
					prices: PurchasePriceReference::from_price_index(&index(tick)),
				},
			);
		}
		#[block]
		{
			Pallet::<T>::prune_price_history();
		}
		assert!(PriceHistoryByTick::<T>::iter_keys().next().is_none());
	}

	// Admission reads the matching price entries while retaining an older principal backlog.
	#[benchmark]
	fn record_purchase() -> Result<(), BenchmarkError> {
		let owner = account("buyer", 0, 0);
		clock(u64::from(<T as Config>::MaxFramesPerPayout::get() + 1) + 10, NOW);
		prices::<T>();
		account_buys::<T>(&owner, true, 0)?;
		let pool = pool();
		ApprovedPool::<T>::put(&pool);
		let mut purchase = purchase();
		CurrencyOf::<T>::mint_into(&owner, purchase.principal)?;
		with_storage_layer(|| Pallet::<T>::record_transferred_purchase(&owner, &purchase))?;
		purchase.gateway_activity_nonce += 1;
		#[block]
		{
			with_storage_layer(|| Pallet::<T>::record_transferred_purchase(&owner, &purchase))?;
		}
		assert_eq!(
			BuysByAccountAndFrame::<T>::iter_prefix(&owner).count(),
			(<T as Config>::MaxFramesPerPayout::get() + 2) as usize
		);
		Ok(())
	}

	// First admission owns the recipient reference before attempting repayment.
	#[benchmark]
	fn record_purchase_first_hold() -> Result<(), BenchmarkError> {
		let owner = account("first_hold", 0, 0);
		clock(10, NOW);
		prices::<T>();
		CurrencyOf::<T>::mint_into(&owner, 1_000_000)?;
		ApprovedPool::<T>::put(pool());
		let purchase = purchase();
		#[block]
		{
			with_storage_layer(|| Pallet::<T>::record_transferred_purchase(&owner, &purchase))?;
		}
		assert_eq!(
			CurrencyOf::<T>::balance_on_hold(&HoldReason::PurchasePrincipal.into(), &owner),
			purchase.principal
		);
		assert!(PayoutStateByAccount::<T>::contains_key(&owner));
		assert_eq!(PayoutQueue::<T>::iter_keys().count(), 1);
		Ok(())
	}

	#[benchmark]
	fn bid() -> Result<(), BenchmarkError> {
		let owner = account("bidder", 0, 0);
		let frame = u64::from(<T as Config>::MaxFramesPerPayout::get() + 1);
		clock(frame + 1, NOW);
		account_buys::<T>(&owner, false, 0)?;
		BuysByAccountAndFrame::<T>::mutate(&owner, frame.to_ordered(), |frame_buys| {
			let frame_buys = frame_buys.as_mut().unwrap();
			frame_buys.bid_rate = Percent::from_percent(100);
			frame_buys.positive_bid_count = <T as Config>::MaxPositiveBids::get() - 1;
		});
		let matching_basis = BuysByAccountAndFrame::<T>::get(&owner, frame.to_ordered())
			.unwrap()
			.captured_discount;
		BidsByAuction::<T>::insert(
			frame,
			(Percent::one(), AmountRankKey::new(matching_basis, owner.clone())),
			(),
		);
		#[extrinsic_call]
		_(RawOrigin::Signed(owner.clone()), frame, Percent::from_percent(50));
		assert_eq!(
			BuysByAccountAndFrame::<T>::get(&owner, frame.to_ordered()).unwrap().bid_rate,
			Percent::from_percent(50)
		);
		Ok(())
	}

	#[benchmark]
	fn process_auctions(b: Linear<1, 4>) -> Result<(), BenchmarkError> {
		clock(30_000, NOW);
		let basis = CurrencyOf::<T>::minimum_balance().max(10_000) / 10;
		let budget = <T as Config>::AuctionInclusionPercent::get().mul_floor(basis);
		let award = Percent::from_percent(50).mul_floor(budget);
		for i in 0..b {
			let owner = account("award", i, 0);
			let offset = u64::from(i + 1) * u64::from(<T as Config>::MaxFramesPerPayout::get() + 1);
			account_buys::<T>(&owner, true, offset)?;
			let frame = offset + 1;
			BuysByAccountAndFrame::<T>::mutate(&owner, frame.to_ordered(), |frame_buys| {
				let frame_buys = frame_buys.as_mut().unwrap();
				frame_buys.pending_liquid_repays = 1;
			});
			Auctions::<T>::insert(
				frame,
				Auction {
					matching_rate: Permill::one(),
					total_matching_basis: basis,
					budget: Some(budget),
					remaining_budget: budget,
					remaining_matching_basis: budget,
					matures_at_frame: CURRENT_FRAME,
				},
			);
			BidsByAuction::<T>::insert(
				frame,
				(Percent::from_percent(50), AmountRankKey::new(basis, owner)),
				(),
			);
			AuctionAllocationQueue::<T>::insert(AscendingRankKey::new(CURRENT_FRAME, frame), ());
		}
		#[block]
		{
			for _ in 0..b {
				let next = AuctionAllocationQueue::<T>::iter_keys().next().unwrap();
				Pallet::<T>::allocate_reward(&next)?;
			}
		}
		assert!(AuctionAllocationQueue::<T>::iter_keys().next().is_none());
		assert_eq!(
			PayoutStateByAccount::<T>::iter_values()
				.map(|payout_state| payout_state.payable_reward)
				.sum::<Balance>(),
			award * u128::from(b)
		);
		Ok(())
	}

	#[benchmark]
	fn has_pending_liquid_repayment() {
		let owner = account("pending-repay", 0, 0);
		PendingLiquidRepayCountByAccountAndLiquid::<T>::insert(&owner, 1, 1);
		let pending;
		#[block]
		{
			pending = Pallet::<T>::has_pending_repayment(&owner, 1);
		}
		assert!(pending);
	}

	#[benchmark]
	fn process_liquid_repayment() -> Result<(), BenchmarkError> {
		clock(CURRENT_FRAME, NOW);
		let owner = account("liquid-repay", 0, 0);
		account_buys::<T>(&owner, true, 0)?;
		let amount = CurrencyOf::<T>::minimum_balance().max(10_000);
		BuysByAccountAndFrame::<T>::mutate(&owner, 1u64.to_ordered(), |frame_buys| {
			let frame_buys = frame_buys.as_mut().unwrap();
			frame_buys.principal = 0;
			frame_buys.pending_liquid_repays = 1;
			frame_buys.liquid_repayment_reward_rate = Some(FixedU128::one());
		});
		LiquidRepaysByAccountAndGatewayNonce::<T>::insert(
			&owner,
			1u64.to_ordered(),
			LiquidRepay {
				liquid_id: 1,
				purchase_tick: NOW,
				price_window: LiquidRepayPriceWindow {
					next_frame: None,
					next_tick: NOW,
					r_value_high_water: FixedU128::from_rational(8, 1000),
				},
				admission_frame: 1,
				microgons: amount,
				eligible_reward_percent: T::LiquidRepaymentRewardPercent::get(),
				microgons_at_target_per_btc: PurchasePriceReference::from_price_index(&index(NOW))
					.unwrap()
					.microgons_at_target_per_btc,
				purchase_r_value: FixedU128::from_rational(5, 1000),
			},
		);
		PendingLiquidRepayCountByAccountAndLiquid::<T>::insert(&owner, 1, 1);
		#[block]
		{
			let (nonce, repay) =
				LiquidRepaysByAccountAndGatewayNonce::<T>::iter_prefix(&owner).next().unwrap();
			let admission_frame = repay.admission_frame;
			with_storage_layer(|| {
				let (_, reward) =
					Pallet::<T>::settle_liquid_repay(&owner, u64::from_ordered(&nonce), repay)?;
				if reward > 0 {
					CurrencyOf::<T>::mint_into(&owner, reward)?;
					Pallet::<T>::deposit_event(Event::LiquidRepaymentRewardAwarded {
						account: owner.clone(),
						admission_frame,
						amount: reward,
					});
				}
				Ok::<_, DispatchError>(())
			})?;
		}
		assert!(!LiquidRepaysByAccountAndGatewayNonce::<T>::contains_key(
			&owner,
			1u64.to_ordered()
		));
		assert_eq!(
			BuysByAccountAndFrame::<T>::get(&owner, 1u64.to_ordered())
				.unwrap()
				.pending_liquid_repays,
			0
		);
		Ok(())
	}

	#[benchmark]
	fn process_reward_payouts(a: Linear<1, 4>) -> Result<(), BenchmarkError> {
		set_benchmark_treasury_reward_payment_failure(false);
		clock(30_000, NOW);
		let reward = CurrencyOf::<T>::minimum_balance().max(10_000);
		for i in 0..a {
			let owner = account("repayment-payout", i, 0);
			account_buys::<T>(&owner, true, u64::from(i) * 100)?;
			for frame in 1..=T::MaxFramesPerPayout::get() {
				let frame = u64::from(frame) + u64::from(i) * 100;
				BuysByAccountAndFrame::<T>::mutate(&owner, frame.to_ordered(), |frame_buys| {
					let frame_buys = frame_buys.as_mut().unwrap();
					let remainder: Balance = (frame == u64::from(i) * 100 + 1).into();
					CurrencyOf::<T>::release(
						&HoldReason::PurchasePrincipal.into(),
						&owner,
						frame_buys.principal - remainder,
						Precision::Exact,
					)
					.unwrap();
					frame_buys.principal = remainder;
					frame_buys.bid_rate = Percent::from_percent(50);
				});
				Auctions::<T>::mutate(frame, |a| a.as_mut().unwrap().remaining_budget = reward);
				let auction = Auctions::<T>::get(frame).unwrap();
				let frame_buys =
					BuysByAccountAndFrame::<T>::get(&owner, frame.to_ordered()).unwrap();
				let matching_basis = auction.matching_rate.mul_floor(frame_buys.captured_discount);
				BidsByAuction::<T>::insert(
					frame,
					(Percent::from_percent(50), AmountRankKey::new(matching_basis, owner.clone())),
					(),
				);
			}
			// The reward provider is measured separately and does not credit this fixture.
			// Leave enough free balance for the one-microgon release to reach the minimum.
			let free = CurrencyOf::<T>::balance(&owner);
			let retained = CurrencyOf::<T>::minimum_balance() - 1;
			CurrencyOf::<T>::burn_from(
				&owner,
				free - retained,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)?;
			PayoutStateByAccount::<T>::mutate(&owner, |payout_state| {
				payout_state.payable_reward = reward
			});
		}
		PayoutQueueCursor::<T>::put((
			CURRENT_FRAME,
			AscendingRankKey::new(CURRENT_FRAME, account::<T::AccountId>("queue-cursor", 0, 0)),
		));
		#[block]
		{
			let due = Pallet::<T>::next_payout_accounts(CURRENT_FRAME, a);
			for key in due {
				let _ =
					LiquidRepaysByAccountAndGatewayNonce::<T>::iter_prefix(key.holder_id()).next();
				Pallet::<T>::payout_account(&key.holder_id(), None)?;
			}
		}
		Ok(())
	}

	#[benchmark]
	fn process_payouts(a: Linear<1, 4>) -> Result<(), BenchmarkError> {
		set_benchmark_treasury_reward_payment_failure(false);
		clock(30_000, NOW);
		let reward = CurrencyOf::<T>::minimum_balance().max(10_000);

		for i in 0..a {
			let owner = account("payout", i, 0);
			account_buys::<T>(
				&owner,
				true,
				u64::from(i) * u64::from(<T as Config>::MaxFramesPerPayout::get() + 1),
			)?;
			PayoutStateByAccount::<T>::mutate(&owner, |payout_state| {
				payout_state.payable_reward = reward
			});
		}
		PayoutQueueCursor::<T>::put((
			CURRENT_FRAME,
			AscendingRankKey::new(CURRENT_FRAME, account::<T::AccountId>("queue-cursor", 0, 0)),
		));
		#[block]
		{
			let due = Pallet::<T>::next_payout_accounts(CURRENT_FRAME, a);
			for key in due {
				let _ =
					LiquidRepaysByAccountAndGatewayNonce::<T>::iter_prefix(key.holder_id()).next();
				Pallet::<T>::payout_account(&key.holder_id(), None)?;
			}
		}
		assert!(PayoutStateByAccount::<T>::iter_values()
			.all(|payout_state| payout_state.payable_reward == 0));
		assert_eq!(PayoutQueue::<T>::iter_keys().count(), a as usize);
		for i in 0..a {
			let owner: T::AccountId = account("payout", i, 0);
			assert_eq!(BuysByAccountAndFrame::<T>::iter_prefix(&owner).count(), 1);
		}
		Ok(())
	}

	#[benchmark]
	fn process_payouts_failed(a: Linear<1, 4>) -> Result<(), BenchmarkError> {
		clock(30_000, NOW);
		let reward = CurrencyOf::<T>::minimum_balance().max(10_000);

		for i in 0..a {
			let owner = account("payout", i, 0);
			account_buys::<T>(
				&owner,
				true,
				u64::from(i) * u64::from(<T as Config>::MaxFramesPerPayout::get() + 1),
			)?;
			PayoutStateByAccount::<T>::mutate(&owner, |payout_state| {
				payout_state.payable_reward = reward
			});
		}
		set_benchmark_treasury_reward_payment_failure(true);
		PayoutQueueCursor::<T>::put((
			CURRENT_FRAME,
			AscendingRankKey::new(CURRENT_FRAME, account::<T::AccountId>("queue-cursor", 0, 0)),
		));
		#[block]
		{
			Pallet::<T>::on_initialize(1u32.into());
		}
		assert_eq!(
			PayoutStateByAccount::<T>::iter_values()
				.map(|payout_state| payout_state.payable_reward)
				.sum::<Balance>(),
			reward * u128::from(a)
		);
		set_benchmark_treasury_reward_payment_failure(false);
		assert_eq!(PayoutQueue::<T>::iter_keys().next().unwrap().rank(), CURRENT_FRAME + 1);
		Ok(())
	}

	#[benchmark]
	fn on_frame_start() {
		clock(12, NOW);
		for frame in [10, 11] {
			Auctions::<T>::insert(
				frame,
				Auction {
					matching_rate: Permill::one(),
					total_matching_basis: 1_000_000,
					budget: None,
					remaining_budget: 0,
					remaining_matching_basis: 0,
					matures_at_frame: frame + T::PurchaseHoldFrames::get(),
				},
			);
		}
		#[block]
		{
			<Pallet<T> as OnNewSlot<T::AccountId>>::on_frame_start(12);
		}
		assert!(AuctionAllocationQueue::<T>::contains_key(AscendingRankKey::new(
			10 + T::PurchaseHoldFrames::get(),
			10
		)));
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
