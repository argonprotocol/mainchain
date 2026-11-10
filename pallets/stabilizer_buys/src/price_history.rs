use crate::*;
use argon_primitives::{PriceIndex, PriceIndexUpdated, SourceChain, MICROGONS_PER_ARGON};
use polkadot_sdk::sp_arithmetic::{helpers_128bit::multiply_by_rational_with_rounding, Rounding};

impl<T: Config> PriceIndexUpdated for Pallet<T> {
	fn on_price_index(index: &PriceIndex) {
		// Keep invalid observations too, so purchase lookup cannot revive an older quote.
		let frame = T::MiningFrameTransitionProvider::is_new_frame_started()
			.unwrap_or_else(T::MiningFrameTransitionProvider::get_current_frame_id);
		let mut observation = PriceObservation { frame, r_value: FixedU128::zero(), prices: None };
		if index.tick <= T::TickProvider::current_tick() {
			observation.prices = PurchasePriceReference::from_price_index(index);
			if index.argon_time_weighted_average_liquidity > 0 {
				observation.r_value = index
					.argon_usd_price
					.checked_div(&index.argon_usd_target_price)
					.unwrap_or_default();
			}
		}
		PriceHistoryByTick::<T>::insert(index.tick.to_ordered(), observation);
	}

	fn on_price_index_weight() -> Weight {
		T::WeightInfo::record_prices()
	}
}

impl<T: Config> Pallet<T> {
	/// Use the latest observation at or before the purchase, within the configured freshness bound.
	/// An unavailable observation stops lookup rather than reviving an older quote.
	pub fn price_reference(purchase_tick: Tick) -> Option<(FrameId, PurchasePriceReference)> {
		if purchase_tick > T::TickProvider::current_tick() {
			return None;
		}

		let oldest_allowed_tick = purchase_tick.saturating_sub(T::MaxPriceAgeTicks::get());
		for observation_tick in (oldest_allowed_tick..=purchase_tick).rev() {
			if let Some(observation) = PriceHistoryByTick::<T>::get(observation_tick.to_ordered()) {
				return observation.prices.map(|prices| (observation.frame, prices));
			}
		}
		None
	}

	/// Prune only observations outside the lookback still needed by unprocessed purchases.
	pub(crate) fn prune_price_history() {
		let Some(gateway_state) =
			T::CrosschainGatewayProvider::last_state_by_chain(SourceChain::Ethereum)
		else {
			return;
		};
		let Some(source_block_time) = gateway_state.last_synced_source_block_time else {
			return;
		};
		let synced_tick = T::TickProvider::ticker().tick_for_time(source_block_time);
		if synced_tick > T::TickProvider::current_tick() {
			return;
		}

		// Retain prices for delayed purchases and pending repayments' recovery windows.
		let mut oldest_needed_tick = synced_tick.saturating_sub(T::MaxPriceAgeTicks::get());
		if let Some(purchase_tick) = Self::oldest_pending_purchase_tick() {
			oldest_needed_tick = oldest_needed_tick.min(purchase_tick);
		}

		// Remove older observations in bounded batches.
		for tick_key in PriceHistoryByTick::<T>::iter_keys()
			.take(T::MaxPriceHistoryPrunesPerBlock::get() as usize)
		{
			if Tick::from_ordered(&tick_key) >= oldest_needed_tick {
				break;
			}
			PriceHistoryByTick::<T>::remove(tick_key);
		}
	}

	/// Fold immutable frame highs, reading individual ticks only where the purchase cuts a frame.
	/// This runs before the payout transaction: a failed payout keeps the completed price window.
	pub(crate) fn advance_price_window(repay: &mut LiquidRepay, max_steps: u32) {
		let maturity_frame = repay.admission_frame.saturating_add(T::PurchaseHoldFrames::get());
		let window = &mut repay.price_window;

		// Resume the recovery scan, stopping before the maturity frame.
		for _ in 0..max_steps {
			let Some(next_frame) = window.next_frame else { break };
			let Some((history_frame, frame_high_water)) =
				T::FissionsProvider::next_r_value_high_water(next_frame)
			else {
				window.next_frame = None;
				break;
			};
			if history_frame >= maturity_frame {
				window.next_frame = None;
				break;
			}
			window.next_frame = Some(history_frame);

			// Use the frame high when its observations start at or after the purchase.
			if frame_high_water.first_tick >= repay.purchase_tick {
				window.r_value_high_water = window.r_value_high_water.max(frame_high_water.r_value);
				window.next_frame = history_frame.checked_add(1);
			} else if frame_high_water.end_tick.is_some_and(|tick| tick < repay.purchase_tick) {
				// Skip frames that ended before the purchase.
				window.next_frame = history_frame.checked_add(1);
			} else {
				// The purchase cuts this frame: read observations from the saved tick cursor.
				let mut observations = PriceHistoryByTick::<T>::iter();
				if let Some(previous_tick) = window.next_tick.checked_sub(1) {
					observations = PriceHistoryByTick::<T>::iter_from(
						PriceHistoryByTick::<T>::hashed_key_for(previous_tick.to_ordered()),
					);
				}
				let Some((tick_key, observation)) = observations.next() else {
					window.next_frame = history_frame.checked_add(1);
					continue;
				};
				if observation.frame > history_frame {
					window.next_frame = history_frame.checked_add(1);
					continue;
				}
				if observation.frame == history_frame {
					window.r_value_high_water = window.r_value_high_water.max(observation.r_value);
				}
				if let Some(next_tick) = Tick::from_ordered(&tick_key).checked_add(1) {
					window.next_tick = next_tick;
				} else {
					window.next_frame = history_frame.checked_add(1);
				}
			}

			// Once the premium is gone, later prices cannot increase the recovery refund.
			if LiquidRepayPrices::new(
				repay.microgons_at_target_per_btc,
				repay.purchase_r_value,
				window.r_value_high_water,
			)
			.is_some_and(|prices| !prices.has_refundable_premium())
			{
				window.next_frame = None;
				break;
			}
		}

		// Completed scans no longer need to pin per-tick history.
		if window.next_frame.is_none() {
			PendingPriceWindowCountByTick::<T>::mutate_exists(
				repay.purchase_tick.to_ordered(),
				|count| {
					let pending_windows = count.unwrap_or_default().saturating_sub(1);
					*count = (pending_windows > 0).then_some(pending_windows);
				},
			);
		}
	}
}

/// One accepted oracle observation. Its R-value remains usable when USDC pricing is unavailable.
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
pub struct PriceObservation {
	/// Reward frame active when the oracle observation was accepted.
	pub frame: FrameId,
	/// Market-to-target ratio, or zero when the ARGON observation is invalid.
	pub r_value: FixedU128,
	/// Purchase valuation inputs; None prevents lookup from reviving an older quote.
	pub prices: Option<PurchasePriceReference>,
}

/// Purchase-time valuation inputs from one accepted oracle observation.
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
	/// ARGON market USD price divided by its contemporaneous target; zero means no market quote.
	pub r_value: FixedU128,
	/// USD per whole ARGON at target, with 18 decimal places; also anchors absolute USD
	/// eligibility.
	pub argon_usd_target_price: FixedU128,
	/// Target-valued microgons per whole BTC, retaining fractional microgons.
	pub microgons_at_target_per_btc: FixedU128,
	/// Target-valued microgons per whole USDC, rounded up to avoid understating spending.
	/// Zero means unavailable for rewards; BTC and R-value remain usable for repayment.
	pub microgons_at_target_per_usdc: FixedU128,
}

impl PurchasePriceReference {
	pub fn from_price_index(index: &PriceIndex) -> Option<Self> {
		let argon_usd_target_price = index.argon_usd_target_price;
		if argon_usd_target_price.is_zero() {
			return None;
		}
		let usdc_usd_price = index.usdc_usd_price.unwrap_or_default();

		// Convert BTC and USDC prices to target-valued microgons, retaining fractional units.
		let microgon_fixed_point_scale = MICROGONS_PER_ARGON.checked_mul(FixedU128::DIV)?;
		let btc_microgons_inner = multiply_by_rational_with_rounding(
			index.btc_usd_price.into_inner(),
			microgon_fixed_point_scale,
			argon_usd_target_price.into_inner(),
			Rounding::Down,
		)?;

		// Round spending up; unavailable USDC pricing only disables buy rewards.
		let usdc_microgons_inner = multiply_by_rational_with_rounding(
			usdc_usd_price.into_inner(),
			microgon_fixed_point_scale,
			argon_usd_target_price.into_inner(),
			Rounding::Up,
		)
		.unwrap_or_default();

		// Capture R independently so recovery can work without a USDC quote.
		let r_value = if index.argon_time_weighted_average_liquidity == 0 {
			FixedU128::zero()
		} else {
			index.argon_usd_price.checked_div(&argon_usd_target_price)?
		};
		Some(Self {
			r_value,
			argon_usd_target_price,
			microgons_at_target_per_btc: FixedU128::from_inner(btc_microgons_inner),
			microgons_at_target_per_usdc: FixedU128::from_inner(usdc_microgons_inner),
		})
	}
}
