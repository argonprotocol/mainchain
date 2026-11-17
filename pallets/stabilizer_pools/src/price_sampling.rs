//! Representative Uniswap price from a frame of spot observations.
use crate::uniswap::sqrt_price_at_uniswap_tick;
use alloy_primitives::{U160, U256 as AlloyU256};
use pallet_prelude::*;
use uniswap_v3_sdk_bigint::utils::get_tick_at_sqrt_ratio;

/// Spot price observations grouped into time buckets from the actual start of one mining frame.
pub struct StabilizerPoolPriceSamples<MaxBuckets: Get<u32>> {
	frame_start_tick: Tick,
	ticks_per_bucket: Tick,
	last_sample_tick: Option<Tick>,
	min_bucket_coverage: Percent,
	buckets: BoundedVec<PriceBucket, MaxBuckets>,
}

impl<MaxBuckets: Get<u32>> StabilizerPoolPriceSamples<MaxBuckets> {
	/// Size buckets using the configured frame duration. A zero bucket limit disables sampling.
	pub fn new(
		frame_start_tick: Tick,
		ticks_per_frame: Tick,
		min_bucket_coverage: Percent,
	) -> Option<Self> {
		let max_buckets = Tick::from(MaxBuckets::get());
		if max_buckets == 0 {
			return None;
		}
		Some(Self {
			frame_start_tick,
			ticks_per_bucket: ticks_per_frame.div_ceil(max_buckets).max(1),
			last_sample_tick: None,
			min_bucket_coverage,
			buckets: BoundedVec::default(),
		})
	}

	/// Count at most one valid pool price per Mainchain tick, in chronological order.
	pub fn record(&mut self, sample_tick: Tick, sqrt_price_x96: U256) {
		let Some(elapsed_ticks) = sample_tick.checked_sub(self.frame_start_tick) else {
			return;
		};
		if self.last_sample_tick.is_some_and(|last| sample_tick <= last) {
			return;
		}
		// TickMath narrows to uint160 before it checks the supported price range.
		if sqrt_price_x96.bits() > U160::BITS {
			return;
		}
		let price = AlloyU256::from_be_bytes(sqrt_price_x96.to_big_endian());
		let Ok(uniswap_tick) = get_tick_at_sqrt_ratio(price) else {
			return;
		};

		let Some(bucket_index) = self.bucket_for_elapsed_ticks(elapsed_ticks) else {
			return;
		};
		if bucket_index >= self.buckets.len() {
			// Keep any intervening buckets empty so missing observations count against coverage.
			self.buckets
				.bounded_resize(bucket_index.saturating_add(1), PriceBucket::default());
		}
		let Some(bucket) = self.buckets.get_mut(bucket_index) else {
			return;
		};
		let sample =
			PriceBucket { uniswap_tick_sum: i128::from(uniswap_tick.as_i32()), sample_count: 1 };

		*bucket = bucket.saturating_add(sample);
		self.last_sample_tick = Some(sample_tick);
	}

	/// Return a price only if every bucket in the closed frame meets the coverage requirement.
	/// Averaging Uniswap ticks gives a geometric mean price. The median across buckets
	/// limits how much a short price spike can affect the frame. Bucket means and the
	/// median round down to whole Uniswap ticks.
	pub fn finish(mut self, frame_end_tick: Tick) -> Option<U256> {
		let frame_ticks = frame_end_tick.checked_sub(self.frame_start_tick)?;
		let last_elapsed_tick = frame_ticks.checked_sub(1)?;
		if self.last_sample_tick? >= frame_end_tick {
			return None;
		}
		let last_bucket = self.bucket_for_elapsed_ticks(last_elapsed_tick)?;
		let buckets = self.buckets.get(..=last_bucket)?;
		let mut means = Vec::with_capacity(buckets.len());

		let mut remaining_ticks = frame_ticks;
		for bucket in buckets {
			let bucket_ticks = remaining_ticks.min(self.ticks_per_bucket);
			if bucket.sample_count < self.min_bucket_coverage.mul_ceil(bucket_ticks) {
				return None;
			}

			let mean_tick =
				bucket.uniswap_tick_sum.checked_div_euclid(i128::from(bucket.sample_count))?;
			means.push(i32::try_from(mean_tick).ok()?);
			remaining_ticks = remaining_ticks.saturating_sub(bucket_ticks);
		}

		means.sort_unstable();
		let middle = means.len() / 2;
		let upper_middle = *means.get(middle)?;
		let median = if means.len().is_multiple_of(2) {
			let lower_middle = *means.get(middle.saturating_sub(1))?;
			lower_middle.saturating_add(upper_middle).div_euclid(2)
		} else {
			upper_middle
		};
		sqrt_price_at_uniswap_tick(median).ok()
	}

	/// Find the bucket for an elapsed tick, merging adjacent buckets if the frame ran long.
	fn bucket_for_elapsed_ticks(&mut self, elapsed_ticks: Tick) -> Option<usize> {
		let max_buckets = Tick::from(MaxBuckets::get());
		if max_buckets == 1 {
			// With no internal boundary to preserve, the bucket can grow to fit the frame.
			self.ticks_per_bucket = self.ticks_per_bucket.max(elapsed_ticks.saturating_add(1));
			return Some(0);
		}

		let bucket_index = elapsed_ticks.checked_div(self.ticks_per_bucket)?;
		if bucket_index < max_buckets {
			return usize::try_from(bucket_index).ok();
		}

		// The new width must fit this tick. A power of two keeps the bucket boundaries
		// identical whether the frame length grows a little at a time or in a single jump.
		let minimum_merge_factor = bucket_index.checked_div(max_buckets)?.saturating_add(1);
		let merge_factor = minimum_merge_factor.checked_next_power_of_two()?;
		// Keep the exact multiple so existing observations stay in the correct bucket.
		let ticks_per_bucket = self.ticks_per_bucket.checked_mul(merge_factor)?;
		let buckets_per_group = usize::try_from(merge_factor.min(max_buckets)).ok()?;
		let merged_bucket_index = usize::try_from(bucket_index.checked_div(merge_factor)?).ok()?;
		let mut merged_buckets = BoundedVec::default();
		for group in self.buckets.chunks(buckets_per_group) {
			let bucket =
				group.iter().copied().fold(PriceBucket::default(), PriceBucket::saturating_add);
			merged_buckets.try_push(bucket).ok()?;
		}

		self.buckets = merged_buckets;
		self.ticks_per_bucket = ticks_per_bucket;
		Some(merged_bucket_index)
	}
}

#[derive(Clone, Copy, Default)]
struct PriceBucket {
	// One bounded i32 Uniswap tick per u64 Mainchain tick fits in i128.
	uniswap_tick_sum: i128,
	sample_count: Tick,
}

impl PriceBucket {
	fn saturating_add(self, other: Self) -> Self {
		Self {
			uniswap_tick_sum: self.uniswap_tick_sum.saturating_add(other.uniswap_tick_sum),
			sample_count: self.sample_count.saturating_add(other.sample_count),
		}
	}
}
