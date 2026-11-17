//! Sampling the existing oracle spot price over closed mining frames.
use crate::{
	uniswap::sqrt_price_at_uniswap_tick, Config, Pallet, StabilizerPoolPriceSamplingState,
	StabilizerPoolPricesByFrame, WeightInfo,
};
use alloy_primitives::{U160, U256 as AlloyU256};
use argon_primitives::{
	block_seal::FrameId, PriceIndex, PriceIndexUpdated, StabilizerPoolPriceSource,
	UniswapPoolIdentity, UniswapSqrtPriceX96, UniswapTick,
};
use frame_support::DefaultNoBound;
use pallet_prelude::*;
use uniswap_v3_sdk_bigint::utils::get_tick_at_sqrt_ratio;

#[derive(
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct StabilizerPoolFramePrice {
	/// Representative raw Uniswap price, sqrt(token1/token0) scaled by 2^96.
	pub sqrt_price_x96: UniswapSqrtPriceX96,
	/// Target USD price per ARGON from the first accepted spot with usable USD quotes.
	pub argon_usd_target_price: FixedU128,
	/// Observed USD price per USDC from the same observation.
	pub usdc_usd_price: FixedU128,
	/// Ethereum pool sampled for this frame.
	pub pool: UniswapPoolIdentity,
	/// Whether the sampled pool quotes ARGON as token0.
	pub argon_is_token0: bool,
}

#[derive(
	Clone,
	Copy,
	Debug,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
)]
pub(crate) struct StabilizerPoolUsdQuotes {
	pub(crate) argon_usd_target_price: FixedU128,
	pub(crate) usdc_usd_price: FixedU128,
}

#[derive(DefaultNoBound, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxBuckets))]
pub struct StabilizerPoolPriceSampling<MaxBuckets: Get<u32>> {
	active_frame: Option<ActiveFramePriceSamples<MaxBuckets>>,
	// A later block at the same clock tick can open a new frame. Hold this observation
	// until a frame boundary or a newer tick establishes where it belongs.
	latest_spot_price: Option<StabilizerPoolSpotPrice>,
}

#[derive(Clone, Copy, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
struct StabilizerPoolSpotPrice {
	tick: Tick,
	sqrt_price_x96: UniswapSqrtPriceX96,
	usd_quotes: Option<StabilizerPoolUsdQuotes>,
	pool_source: StabilizerPoolPriceSource,
}

impl<T: Config> PriceIndexUpdated for Pallet<T> {
	fn on_price_index(index: &PriceIndex, pool_source: Option<StabilizerPoolPriceSource>) {
		let Some(pool_source) = pool_source else { return };
		let current_tick = T::CurrentTick::get();
		if index.tick > current_tick {
			return;
		}

		let Some(sqrt_price_x96) = index.stabilizer_pool_sqrt_price_x96 else { return };
		let usd_quotes = index
			.usdc_usd_price
			.filter(|usdc_usd_price| {
				!usdc_usd_price.is_zero() && !index.argon_usd_target_price.is_zero()
			})
			.map(|usdc_usd_price| StabilizerPoolUsdQuotes {
				argon_usd_target_price: index.argon_usd_target_price,
				usdc_usd_price,
			});

		// The sampler can still accept a delayed observation while its frame is open.
		Self::record_stabilizer_pool_spot_price(
			index.tick,
			sqrt_price_x96,
			usd_quotes,
			pool_source,
		);
	}

	fn on_price_index_weight() -> Weight {
		<T as Config>::WeightInfo::read_price_index_context()
			.saturating_add(<T as Config>::WeightInfo::record_stabilizer_pool_spot_price())
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn record_stabilizer_pool_spot_price(
		tick: Tick,
		sqrt_price_x96: UniswapSqrtPriceX96,
		usd_quotes: Option<StabilizerPoolUsdQuotes>,
		pool_source: StabilizerPoolPriceSource,
	) {
		if T::MaxPriceBucketsPerFrame::get() == 0 {
			return;
		}

		StabilizerPoolPriceSamplingState::<T>::mutate(|sampling| {
			if let Some(previous_price) = sampling.latest_spot_price {
				if tick <= previous_price.tick {
					return;
				}
				if let Some(active_frame) = &mut sampling.active_frame {
					active_frame.record(previous_price);
				}
			}
			sampling.latest_spot_price =
				Some(StabilizerPoolSpotPrice { tick, sqrt_price_x96, usd_quotes, pool_source });
		});
	}

	/// Open sampling and return the previous frame's representative price when available.
	pub(crate) fn start_stabilizer_pool_price_sampling(
		frame_id: FrameId,
		start_tick: Tick,
	) -> Option<StabilizerPoolFramePrice> {
		let mut sampling = StabilizerPoolPriceSamplingState::<T>::get();
		if sampling.active_frame.as_ref().is_some_and(|active| active.frame_id >= frame_id) {
			return None;
		}

		let closed_frame = sampling
			.active_frame
			.take()
			.filter(|frame| frame.frame_id.checked_add(1) == Some(frame_id));
		let closed_frame_price = closed_frame.and_then(|mut frame| {
			if let Some(spot_price) =
				sampling.latest_spot_price.filter(|price| price.tick < start_tick)
			{
				frame.record(spot_price);
			}
			let source = frame.pool_source?;
			let quotes = frame.usd_quotes?;
			let price = StabilizerPoolFramePrice {
				argon_usd_target_price: quotes.argon_usd_target_price,
				usdc_usd_price: quotes.usdc_usd_price,
				sqrt_price_x96: frame.samples.finish(start_tick)?,
				pool: source.pool,
				argon_is_token0: source.argon_is_token0,
			};
			Some((frame.frame_id, price))
		});

		let oldest_retained_frame =
			frame_id.saturating_sub(FrameId::from(T::MaxSampledPriceHistoryFrames::get()));
		StabilizerPoolPricesByFrame::<T>::mutate(|history| {
			history.retain(|frame, _| *frame >= oldest_retained_frame);
			if let Some((closed_frame, price)) =
				closed_frame_price.filter(|(frame, ..)| *frame >= oldest_retained_frame)
			{
				history
					.try_insert(closed_frame, price)
					.expect("one price per retained closed frame fits the history bound");
			}
		});

		sampling.active_frame = StabilizerPoolPriceSamples::new(
			start_tick,
			T::TicksPerFrame::get(),
			T::MinPriceBucketCoverage::get(),
		)
		.map(|samples| ActiveFramePriceSamples {
			frame_id,
			samples,
			usd_quotes: None,
			pool_source: None,
		});
		sampling.latest_spot_price =
			sampling.latest_spot_price.filter(|price| price.tick == start_tick);
		StabilizerPoolPriceSamplingState::<T>::put(sampling);
		closed_frame_price.map(|(_, price)| price)
	}
}

#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxBuckets))]
struct ActiveFramePriceSamples<MaxBuckets: Get<u32>> {
	frame_id: FrameId,
	samples: StabilizerPoolPriceSamples<MaxBuckets>,
	usd_quotes: Option<StabilizerPoolUsdQuotes>,
	pool_source: Option<StabilizerPoolPriceSource>,
}

impl<MaxBuckets: Get<u32>> ActiveFramePriceSamples<MaxBuckets> {
	fn record(&mut self, price: StabilizerPoolSpotPrice) {
		let first_sample = self.samples.last_sample_tick.is_none();
		if !self.samples.record(price.tick, price.sqrt_price_x96) {
			return;
		}
		if first_sample {
			self.pool_source = Some(price.pool_source);
		} else if self.pool_source != Some(price.pool_source) {
			// A frame that sampled two approved revisions has no representative pool price.
			self.pool_source = None;
		}
		self.usd_quotes = self.usd_quotes.or(price.usd_quotes);
	}
}

/// Spot price observations grouped into time buckets from the actual start of one mining frame.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxBuckets))]
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

	/// Return whether the pool price counted toward coverage. Only one price per tick can count.
	pub fn record(&mut self, sample_tick: Tick, sqrt_price_x96: UniswapSqrtPriceX96) -> bool {
		let Some(elapsed_ticks) = sample_tick.checked_sub(self.frame_start_tick) else {
			return false;
		};
		if self.last_sample_tick.is_some_and(|last| sample_tick <= last) {
			return false;
		}
		// TickMath narrows to uint160 before it checks the supported price range.
		if sqrt_price_x96.bits() > U160::BITS {
			return false;
		}
		let price = AlloyU256::from_be_bytes(sqrt_price_x96.to_big_endian());
		let Ok(uniswap_tick) = get_tick_at_sqrt_ratio(price) else {
			return false;
		};

		let Some(bucket_index) = self.bucket_for_elapsed_ticks(elapsed_ticks) else {
			return false;
		};
		if bucket_index >= self.buckets.len() {
			// Keep any intervening buckets empty so missing observations count against coverage.
			self.buckets
				.bounded_resize(bucket_index.saturating_add(1), PriceBucket::default());
		}
		let Some(bucket) = self.buckets.get_mut(bucket_index) else {
			return false;
		};
		let sample =
			PriceBucket { uniswap_tick_sum: i128::from(uniswap_tick.as_i32()), sample_count: 1 };

		*bucket = bucket.saturating_add(sample);
		self.last_sample_tick = Some(sample_tick);
		true
	}

	/// Return a price only if every bucket in the closed frame meets the coverage requirement.
	/// Averaging Uniswap ticks gives a geometric mean price. The median across buckets
	/// limits how much a short price spike can affect the frame. Bucket means and the
	/// median round down to whole Uniswap ticks.
	pub fn finish(mut self, frame_end_tick: Tick) -> Option<UniswapSqrtPriceX96> {
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
			means.push(UniswapTick::try_from(mean_tick).ok()?);
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

#[derive(Clone, Copy, Default, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
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
