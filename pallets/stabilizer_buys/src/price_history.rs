use crate::*;
use argon_primitives::{PriceIndex, PriceIndexUpdated};

impl<T: Config> PriceIndexUpdated for Pallet<T> {
	fn on_price_index(index: &PriceIndex) {
		// Keep exact targets for recent deliveries. Updating a stored tick must not evict another.
		let tick_key = index.tick.to_ordered();
		if !RecentPriceIndices::<T>::contains_key(tick_key) &&
			RecentPriceIndices::<T>::count() >= T::MaxRecentPriceTicks::get()
		{
			// Advancing the ordered drain once removes one entry and updates the FRAME counter.
			let _ = RecentPriceIndices::<T>::drain().next();
		}
		RecentPriceIndices::<T>::insert(tick_key, index);

		let bucket_ticks = T::PriceHistoryBucketTicks::get().max(1);
		let bucket_tick = index.tick - index.tick % bucket_ticks;
		let now = T::CurrentTick::get();
		let current_bucket_tick = now - now % bucket_ticks;
		// Closed hours stay fixed so new observations cannot revalue delayed deliveries.
		if bucket_tick != current_bucket_tick {
			return;
		}

		let bucket_key = bucket_tick.to_ordered();
		if let Some(mut hour) = HourlyPriceReferences::<T>::get(bucket_key) {
			hour.record(index);
			HourlyPriceReferences::<T>::insert(bucket_key, hour);
			return;
		}

		// Retain observed hours only; an oracle gap does not create synthetic references.
		if HourlyPriceReferences::<T>::count() >= T::MaxHourlyPriceReferences::get() {
			let _ = HourlyPriceReferences::<T>::drain().next();
		}
		let hour = HourlyPriceReference {
			minimum_argon_target_usd: index.argon_usd_target_price,
			usdc: index.usdc_usd_price.map(|price| (index.tick, price)),
		};
		HourlyPriceReferences::<T>::insert(bucket_key, hour);
	}

	fn on_price_index_weight() -> Weight {
		T::WeightInfo::record_prices()
	}
}

impl<T: Config> Pallet<T> {
	pub fn price_reference(tick: Tick) -> Option<PurchasePriceReference> {
		let now = T::CurrentTick::get();
		if tick > now {
			return None;
		}
		let bucket_ticks = T::PriceHistoryBucketTicks::get().max(1);
		let bucket_tick = tick - tick % bucket_ticks;
		let hour = HourlyPriceReferences::<T>::get(bucket_tick.to_ordered())?;
		let (usdc_tick, usdc_usd) = hour.usdc?;
		// In the open hour, a newer quote is valid; an older one must be close to the purchase.
		let current_bucket_tick = now - now % bucket_ticks;
		if bucket_tick == current_bucket_tick {
			let quote_age = tick.saturating_sub(usdc_tick);
			if quote_age > T::MaxCurrentHourPriceAgeTicks::get() {
				return None;
			}
		}

		// Late deliveries fall back to the hourly minimum rather than overstate the discount.
		let argon_target_usd;
		if let Some(reference) = RecentPriceIndices::<T>::get(tick.to_ordered()) {
			argon_target_usd = reference.argon_usd_target_price;
		} else {
			argon_target_usd = hour.minimum_argon_target_usd;
		}
		Some(PurchasePriceReference { argon_target_usd, usdc_usd })
	}
}

/// Valuation references, distinct from the authenticated execution price of a purchase.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PurchasePriceReference {
	pub argon_target_usd: FixedU128,
	pub usdc_usd: FixedU128,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct HourlyPriceReference {
	pub minimum_argon_target_usd: FixedU128,
	/// Tick and USD price of the USDC observation closest to $1.
	pub usdc: Option<(Tick, FixedU128)>,
}

impl HourlyPriceReference {
	fn record(&mut self, index: &PriceIndex) {
		self.minimum_argon_target_usd =
			self.minimum_argon_target_usd.min(index.argon_usd_target_price);

		let Some(usdc_usd) = index.usdc_usd_price else { return };
		if let Some((_, previous)) = self.usdc {
			let distance = usdc_usd.into_inner().abs_diff(FixedU128::DIV);
			let previous_distance = previous.into_inner().abs_diff(FixedU128::DIV);
			// Prefer the quote closest to $1 to reduce rewards from brief USDC price moves.
			// Keep the first different quote on a tie; repeated identical quotes refresh its tick.
			if usdc_usd != previous && distance >= previous_distance {
				return;
			}
		}
		self.usdc = Some((index.tick, usdc_usd));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{mock, mock::*};

	#[test]
	fn recent_targets_are_exact_then_fall_back_to_the_hourly_minimum() {
		new_test_ext().execute_with(|| {
			let _ = RecentPriceIndices::<Test>::clear(u32::MAX, None);
			let _ = HourlyPriceReferences::<Test>::clear(u32::MAX, None);
			let mut index = argon_primitives::PriceIndex {
				btc_usd_price: FixedU128::from_u32(60_000),
				argonot_usd_price: FixedU128::one(),
				argon_usd_price: FixedU128::one(),
				argon_usd_target_price: FixedU128::one(),
				argon_time_weighted_average_liquidity: 1,
				tick: 0,
				usdc_usd_price: None,
				stabilizer_pool_sqrt_price_x96: None,
			};
			for (tick, usdc, target) in
				[(1, 970, 1000), (12, 1010, 1010), (20, 990, 1020), (30, 995, 1030)]
			{
				CurrentTick::set(tick);
				index.tick = tick;
				index.argon_usd_target_price = FixedU128::from_rational(target, 1000);
				index.usdc_usd_price = Some(FixedU128::from_rational(usdc, 1000));
				assert_ok!(mock::PriceIndex::submit(RuntimeOrigin::signed(1), index, None));
				if tick == 20 {
					assert_eq!(
						HourlyPriceReferences::<Test>::get(0u64.to_ordered())
							.unwrap()
							.usdc
							.unwrap()
							.0,
						12
					);
				}
			}
			index.tick = 35;
			index.argon_usd_target_price = FixedU128::from_rational(99, 100);
			index.usdc_usd_price = None;
			CurrentTick::set(35);
			assert_ok!(mock::PriceIndex::submit(RuntimeOrigin::signed(1), index, None));
			assert!(RecentPriceIndices::<Test>::contains_key(35u64.to_ordered()));
			assert_eq!(
				HourlyPriceReferences::<Test>::get(0u64.to_ordered()).unwrap().usdc.unwrap().0,
				30
			);

			CurrentTick::set(50);
			let reference = StabilizerBuys::price_reference(20).unwrap();
			assert_eq!(reference.argon_target_usd, FixedU128::from_rational(1020, 1000));
			assert_eq!(reference.usdc_usd, FixedU128::from_rational(995, 1000));
			assert!(StabilizerBuys::price_reference(40).is_some());
			assert!(StabilizerBuys::price_reference(41).is_none());
			assert!(StabilizerBuys::price_reference(51).is_none());
			let mut delivered = purchase();
			delivered.tick = 35;
			assert_ok!(StabilizerBuys::record_transferred_purchase(&1, &delivered));
			// (100 * $0.99 - 90 * $0.995) / $0.99, rounded down to microgons.
			assert_eq!(
				PurchasesByAccount::<Test>::get(1, 1u64.to_ordered()).unwrap().captured_discount,
				9_545_454
			);

			for tick in 51..=411 {
				CurrentTick::set(tick);
				index.tick = tick;
				index.usdc_usd_price = Some(FixedU128::from_rational(98, 100));
				assert_ok!(mock::PriceIndex::submit(RuntimeOrigin::signed(1), index, None));
			}
			assert_eq!(RecentPriceIndices::<Test>::count(), 360);
			assert!(!RecentPriceIndices::<Test>::contains_key(20u64.to_ordered()));
			let late = StabilizerBuys::price_reference(20).unwrap();
			assert_eq!(late.argon_target_usd, FixedU128::from_rational(99, 100));
			assert_eq!(late.usdc_usd, reference.usdc_usd);
			delivered.tick = 20;
			assert_ok!(StabilizerBuys::record_transferred_purchase(&2, &delivered));
			assert_eq!(
				PurchasesByAccount::<Test>::get(2, 1u64.to_ordered()).unwrap().captured_discount,
				9_545_454
			);
		});
	}

	#[test]
	fn hourly_history_uses_rounded_ticks_and_truncates_without_filling_gaps() {
		new_test_ext().execute_with(|| {
			let _ = RecentPriceIndices::<Test>::clear(u32::MAX, None);
			let _ = HourlyPriceReferences::<Test>::clear(u32::MAX, None);
			for hour in core::iter::once(0).chain(2..=722) {
				let tick = hour * 60 + 7;
				CurrentTick::set(tick);
				let index = argon_primitives::PriceIndex {
					btc_usd_price: FixedU128::from_u32(60_000),
					argonot_usd_price: FixedU128::one(),
					argon_usd_price: FixedU128::one(),
					argon_usd_target_price: FixedU128::one(),
					argon_time_weighted_average_liquidity: 1,
					tick,
					usdc_usd_price: Some(FixedU128::from_rational(98, 100)),
					stabilizer_pool_sqrt_price_x96: None,
				};
				assert_ok!(mock::PriceIndex::submit(RuntimeOrigin::signed(1), index, None));
			}
			assert_eq!(HourlyPriceReferences::<Test>::count(), 720);
			assert!(HourlyPriceReferences::<Test>::iter_keys()
				.all(|key| Tick::from_ordered(&key) % 60 == 0));
			assert!(StabilizerBuys::price_reference(7).is_none());
			assert!(StabilizerBuys::price_reference(67).is_none());
			assert!(StabilizerBuys::price_reference(127).is_none());
			assert_eq!(
				StabilizerBuys::price_reference(187).unwrap().usdc_usd,
				FixedU128::from_rational(98, 100)
			);
			let mut index = RecentPriceIndices::<Test>::iter_values().last().unwrap();
			index.tick = 722 * 60 + 20;
			CurrentTick::set(index.tick);
			assert_ok!(mock::PriceIndex::submit(RuntimeOrigin::signed(1), index, None));
			assert!(StabilizerBuys::price_reference(index.tick - 2).is_some());
		});
	}
}
