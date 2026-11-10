use crate::{Config, Pallet, RValueHighWaterByFrame, WeightInfo};
use argon_primitives::{
	amount_rank::AmountRankId, CrosschainGatewayProvider, LiquidRepaymentProvider,
	MiningFrameTransitionProvider, PriceIndex, PriceIndexUpdated, RValueHighWaterMark, SourceChain,
	TickProvider,
};
use pallet_prelude::*;

impl<T: Config> PriceIndexUpdated for Pallet<T> {
	fn on_price_index(index: &PriceIndex) {
		// Only valid ARGON observations contribute to a frame's high-water mark.
		if index.argon_time_weighted_average_liquidity == 0 {
			return;
		}
		if index.argon_usd_price.is_zero() {
			return;
		}
		if index.argon_usd_target_price.is_zero() {
			return;
		}

		let Some(r_value) = index.argon_usd_price.checked_div(&index.argon_usd_target_price) else {
			return;
		};
		if r_value.is_zero() {
			return;
		}

		let current_tick = T::TickProvider::current_tick();
		if index.tick > current_tick {
			return;
		}

		// Record the maximum R-value and earliest observation tick for the active frame.
		let active_frame = T::MiningFrameTransitionProvider::is_new_frame_started()
			.unwrap_or_else(T::MiningFrameTransitionProvider::get_current_frame_id);
		RValueHighWaterByFrame::<T>::mutate(active_frame.to_ordered(), |entry| {
			if let Some(high_water_mark) = entry {
				high_water_mark.r_value = high_water_mark.r_value.max(r_value);
				high_water_mark.first_tick = high_water_mark.first_tick.min(index.tick);
			} else {
				*entry =
					Some(RValueHighWaterMark { r_value, first_tick: index.tick, end_tick: None });
			}
		});
	}

	fn on_price_index_weight() -> Weight {
		T::WeightInfo::record_r_value()
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn prune_r_value_history() {
		// Close the previous frame even while gateway sync is stalled.
		let new_frame = T::MiningFrameTransitionProvider::is_new_frame_started();
		let previous_frame = new_frame.and_then(|frame| frame.checked_sub(1));
		if let Some(previous_frame) = previous_frame {
			RValueHighWaterByFrame::<T>::mutate_exists(previous_frame.to_ordered(), |entry| {
				if let Some(high_water_mark) = entry {
					high_water_mark.end_tick.get_or_insert_with(T::TickProvider::current_tick);
				}
			});
		}

		// Keep unsynced intervals and history pinned by incomplete recovery scans.
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
		let oldest_needed_tick = T::LiquidRepaymentProvider::oldest_pending_purchase_tick()
			.map_or(synced_tick, |purchase_tick| purchase_tick.min(synced_tick));

		// Always retain the configured trailing frames, even after Ethereum catches up.
		let active_frame =
			new_frame.unwrap_or_else(T::MiningFrameTransitionProvider::get_current_frame_id);
		let history_frames = u64::from(T::RValueHistoryFrames::get());
		let oldest_retained_frame = active_frame.saturating_sub(history_frames.saturating_sub(1));
		let prune_limit = T::MaxRValueHistoryPrunesPerBlock::get() as usize;

		// Prune older, closed frames only when their full interval is no longer needed.
		for (frame_key, high_water_mark) in RValueHighWaterByFrame::<T>::iter().take(prune_limit) {
			if FrameId::from_ordered(&frame_key) >= oldest_retained_frame {
				break;
			}

			let Some(end_tick) = high_water_mark.end_tick else {
				break;
			};
			if end_tick >= oldest_needed_tick {
				break;
			}

			RValueHighWaterByFrame::<T>::remove(frame_key);
		}
	}
}
