//! Uniswap principal valuation for individual NFTs and shared liquidity ranges.
use crate::PositionCheckpoint;
use alloy_primitives::{aliases::I24, U256 as AlloyU256};
use argon_primitives::{Balance, UniswapSqrtPriceX96, UniswapTick, MICROGONS_PER_ARGON};
use core::cmp::Ordering;
use pallet_prelude::*;
use polkadot_sdk::sp_core::U512;
use uniswap_v3_sdk_bigint::utils::{
	get_sqrt_ratio_at_tick, get_tick_at_sqrt_ratio, MAX_SQRT_RATIO, MAX_TICK_I32, MIN_SQRT_RATIO,
	MIN_TICK_I32,
};

// A position can shift its full band around a reference up to 1% from target.
// Only principal inside the current reward band counts.
pub(crate) const REWARD_BAND_TOLERANCE: Percent = Percent::from_percent(1);
// At spacing one, shifting the reference across that tolerance crosses at most
// 201 Uniswap ticks at each edge of the band.
pub(crate) const MAX_REWARD_RANGES: u32 = 404;

const PERCENT_SCALE: u8 = Percent::one().deconstruct();

const ARGON_TOKEN_DECIMALS: u32 = 18;
const USDC_TOKEN_DECIMALS: u32 = 6;
const ARGON_USDC_DECIMAL_SCALE: u64 = 10u64.pow(ARGON_TOKEN_DECIMALS - USDC_TOKEN_DECIMALS);

impl<AccountId> PositionCheckpoint<AccountId> {
	/// Value this NFT's principal at a raw Uniswap Q96 sqrt price, excluding fees.
	/// ARGON is converted from 18-decimal token units to microgons before checked narrowing;
	/// USDC already has six decimals. Each amount rounds down, as when removing liquidity.
	pub fn amounts_at_price(
		&self,
		sqrt_price_x96: UniswapSqrtPriceX96,
		argon_is_token0: bool,
	) -> Result<PositionAmounts, PositionAmountError> {
		let amounts = principal_for_liquidity_range(
			self.liquidity.into(),
			self.tick_lower,
			self.tick_upper,
			sqrt_price_x96,
			argon_is_token0,
		)?;
		let argon =
			Balance::try_from(amounts.argon).map_err(|_| PositionAmountError::AmountOverflow)?;
		let usdc =
			Balance::try_from(amounts.usdc).map_err(|_| PositionAmountError::AmountOverflow)?;
		Ok(PositionAmounts { argon, usdc })
	}
}

/// Shared valuation terms for one pool and frame. Positions keep their own exact Uniswap bounds.
/// Both bounds must describe the configured price band at one reference within the allowed drift.
/// Only principal overlapping the current target's band is eligible for rewards.
#[derive(Clone, Copy, Debug)]
pub struct StabilizerPoolRewardBand {
	argon_usd_target_price: FixedU128,
	usdc_usd_price: FixedU128,
	argon_is_token0: bool,
	uniswap_tick_spacing: i32,
	reward_price_band: (Percent, Percent),
	reference_tolerance: Percent,
	sqrt_lower: UniswapSqrtPriceX96,
	sqrt_upper: UniswapSqrtPriceX96,
}

impl StabilizerPoolRewardBand {
	/// Prepare inward-rounded price boundaries from the lower and upper percentages of target.
	/// Tolerance shifts the whole band around a shared reference.
	pub fn new(
		argon_usd_target_price: FixedU128,
		usdc_usd_price: FixedU128,
		argon_is_token0: bool,
		uniswap_tick_spacing: i32,
		reward_price_band: (Percent, Percent),
		reference_tolerance: Percent,
	) -> Result<Self, PositionAmountError> {
		if argon_usd_target_price.is_zero() {
			return Err(PositionAmountError::InvalidPrice);
		}
		if usdc_usd_price.is_zero() {
			return Err(PositionAmountError::InvalidPrice);
		}
		if !(1..=32_767).contains(&uniswap_tick_spacing) {
			return Err(PositionAmountError::InvalidTickRange);
		}

		let (lower_percent, upper_percent) = reward_price_band;
		if lower_percent.is_zero() {
			return Err(PositionAmountError::InvalidRewardPriceBand);
		}
		if lower_percent >= upper_percent {
			return Err(PositionAmountError::InvalidRewardPriceBand);
		}

		// Scale both quotes by the percentage denominator so the band edges stay exact.
		// The decimal scale converts the whole-token quote to Uniswap's base-unit ratio.
		let target_usd = U256::from(argon_usd_target_price.into_inner());
		let lower_edge_usd_scaled = target_usd * lower_percent.deconstruct();
		let upper_edge_usd_scaled = target_usd * upper_percent.deconstruct();
		let usdc_usd_scaled =
			U256::from(usdc_usd_price.into_inner()) * ARGON_USDC_DECIMAL_SCALE * PERCENT_SCALE;
		let (sqrt_lower, sqrt_upper) = if argon_is_token0 {
			(
				sqrt_price_from_token_ratio(lower_edge_usd_scaled, usdc_usd_scaled, true)?,
				sqrt_price_from_token_ratio(upper_edge_usd_scaled, usdc_usd_scaled, false)?,
			)
		} else {
			(
				sqrt_price_from_token_ratio(usdc_usd_scaled, upper_edge_usd_scaled, true)?,
				sqrt_price_from_token_ratio(usdc_usd_scaled, lower_edge_usd_scaled, false)?,
			)
		};
		let minimum_sqrt_price = U256::from_big_endian(&MIN_SQRT_RATIO.to_be_bytes::<20>());
		let maximum_sqrt_price = U256::from_big_endian(&MAX_SQRT_RATIO.to_be_bytes::<20>());
		if sqrt_lower < minimum_sqrt_price {
			return Err(PositionAmountError::InvalidPrice);
		}
		if sqrt_upper > maximum_sqrt_price {
			return Err(PositionAmountError::InvalidPrice);
		}
		if sqrt_lower >= sqrt_upper {
			return Err(PositionAmountError::InvalidPrice);
		}

		Ok(Self {
			argon_usd_target_price,
			usdc_usd_price,
			argon_is_token0,
			uniswap_tick_spacing,
			reward_price_band,
			reference_tolerance,
			sqrt_lower,
			sqrt_upper,
		})
	}

	/// Value one NFT or a combined quantity with identical bounds. A nonqualifying band earns
	/// zero. Only the position's overlap with the current reward band contributes.
	pub fn principal_for_range(
		&self,
		liquidity: U256,
		lower_uniswap_tick: UniswapTick,
		upper_uniswap_tick: UniswapTick,
		pool_sqrt_price_x96: UniswapSqrtPriceX96,
	) -> Result<PositionAmounts<U256>, PositionAmountError> {
		let empty = PositionAmounts { argon: U256::zero(), usdc: U256::zero() };
		let Some((position_sqrt_lower, position_sqrt_upper)) =
			self.qualifying_range(lower_uniswap_tick, upper_uniswap_tick)?
		else {
			return Ok(empty);
		};

		let sqrt_lower = position_sqrt_lower.max(self.sqrt_lower);
		let sqrt_upper = position_sqrt_upper.min(self.sqrt_upper);
		if sqrt_lower >= sqrt_upper {
			return Ok(empty);
		}
		principal_for_sqrt_price_range(
			liquidity,
			sqrt_lower,
			sqrt_upper,
			pool_sqrt_price_x96,
			self.argon_is_token0,
		)
	}

	/// Enumerate the exact pairs obtained as the shared reference moves through its tolerance.
	/// At each boundary only the edge that reached its next grid price advances.
	pub(crate) fn reward_ranges(
		&self,
	) -> Result<Vec<(UniswapTick, UniswapTick)>, PositionAmountError> {
		if self.reference_tolerance > REWARD_BAND_TOLERANCE {
			return Err(PositionAmountError::InvalidRewardPriceBand);
		}

		let references = BandReferences::new(self);
		let last_grid_index = MAX_TICK_I32 / self.uniswap_tick_spacing;
		let mut lower = references.grid_index_at_minimum(BandEdge::Lower)?;
		let mut upper = references.grid_index_at_minimum(BandEdge::Upper)?;
		let first_overlapping_upper_tick =
			rounded_uniswap_tick(self.sqrt_lower, false)?.saturating_add(1);
		let last_overlapping_lower_tick =
			rounded_uniswap_tick(self.sqrt_upper, true)?.saturating_sub(1);
		let mut ranges = Vec::new();

		for _ in 0..MAX_REWARD_RANGES {
			if lower >= -last_grid_index && upper > lower && upper <= last_grid_index {
				let (lower_tick, upper_tick) = references.uniswap_range(lower, upper);
				if upper_tick >= first_overlapping_upper_tick &&
					lower_tick <= last_overlapping_lower_tick
				{
					ranges.push((lower_tick, upper_tick));
				}
			}

			let next_lower =
				references.next_grid_reference_key(lower, last_grid_index, BandEdge::Lower)?;
			let next_upper =
				references.next_grid_reference_key(upper, last_grid_index, BandEdge::Upper)?;
			if references
				.compare_reference_keys(
					references.earlier_reference_key(next_lower, next_upper),
					references.maximum_reference_key,
				)
				.is_gt()
			{
				return Ok(ranges);
			}

			let next_edge = references.compare_reference_keys(next_lower, next_upper);
			if !next_edge.is_gt() {
				lower = lower.saturating_add(1);
			}
			if !next_edge.is_lt() {
				upper = upper.saturating_add(1);
			}
		}

		Err(PositionAmountError::InvalidRewardPriceBand)
	}

	/// Each endpoint rounds down in USD to a usable Uniswap tick. Their reference intervals
	/// must overlap each other and the allowed drift around target.
	fn qualifying_range(
		&self,
		lower_uniswap_tick: UniswapTick,
		upper_uniswap_tick: UniswapTick,
	) -> Result<Option<(UniswapSqrtPriceX96, UniswapSqrtPriceX96)>, PositionAmountError> {
		if lower_uniswap_tick >= upper_uniswap_tick {
			return Err(PositionAmountError::InvalidTickRange);
		}
		if lower_uniswap_tick % self.uniswap_tick_spacing != 0 {
			return Err(PositionAmountError::InvalidTickRange);
		}
		if upper_uniswap_tick % self.uniswap_tick_spacing != 0 {
			return Err(PositionAmountError::InvalidTickRange);
		}
		let sqrt_lower = sqrt_price_at_uniswap_tick(lower_uniswap_tick)?;
		let sqrt_upper = sqrt_price_at_uniswap_tick(upper_uniswap_tick)?;

		let references = BandReferences::new(self);
		let (lower_grid_index, upper_grid_index) = if self.argon_is_token0 {
			(
				lower_uniswap_tick / self.uniswap_tick_spacing,
				upper_uniswap_tick / self.uniswap_tick_spacing,
			)
		} else {
			(
				-upper_uniswap_tick / self.uniswap_tick_spacing,
				-lower_uniswap_tick / self.uniswap_tick_spacing,
			)
		};
		let (lower_price, upper_price) =
			if self.argon_is_token0 { (sqrt_lower, sqrt_upper) } else { (sqrt_upper, sqrt_lower) };
		let lower_start = references.edge_reference_key(lower_price, BandEdge::Lower);
		let upper_start = references.edge_reference_key(upper_price, BandEdge::Upper);
		let last_grid_index = MAX_TICK_I32 / self.uniswap_tick_spacing;
		let lower_end = references.next_grid_reference_key(
			lower_grid_index,
			last_grid_index,
			BandEdge::Lower,
		)?;
		let upper_end = references.next_grid_reference_key(
			upper_grid_index,
			last_grid_index,
			BandEdge::Upper,
		)?;
		let start = if references.compare_reference_keys(lower_start, upper_start).is_gt() {
			lower_start
		} else {
			upper_start
		};
		let end = references.earlier_reference_key(lower_end, upper_end);

		if !references.compare_reference_keys(start, end).is_lt() {
			return Ok(None);
		}
		if references
			.compare_reference_keys(start, references.maximum_reference_key)
			.is_gt()
		{
			return Ok(None);
		}
		if !references.compare_reference_keys(end, references.minimum_reference_key).is_gt() {
			return Ok(None);
		}
		Ok(Some((sqrt_lower, sqrt_upper)))
	}
}

#[derive(Clone, Copy)]
enum BandEdge {
	Lower,
	Upper,
}

// Common scaled prices for comparing the two edges' implied reference and the tolerance limits.
// With ARGON as token1, the keys are inverse prices, so USD order is reversed.
struct BandReferences {
	argon_is_token0: bool,
	spacing: i32,
	lower_percent: u8,
	upper_percent: u8,
	edge_scale: U512,
	minimum_reference_key: U512,
	maximum_reference_key: U512,
}

impl BandReferences {
	fn new(band: &StabilizerPoolRewardBand) -> Self {
		let (lower, upper) = band.reward_price_band;
		let (lower_percent, upper_percent) = (lower.deconstruct(), upper.deconstruct());
		let usdc = U512::from(band.usdc_usd_price.into_inner()) *
			ARGON_USDC_DECIMAL_SCALE *
			PERCENT_SCALE *
			PERCENT_SCALE;
		let q192 = U512::one() << 192;
		let target = U512::from(band.argon_usd_target_price.into_inner());
		let tolerance = band.reference_tolerance.deconstruct();
		let (edge_scale, reference_scale) = if band.argon_is_token0 {
			(usdc, q192 * lower_percent * upper_percent)
		} else {
			(target, usdc * q192)
		};
		let at_drift = |percent| {
			if band.argon_is_token0 {
				target * percent * reference_scale
			} else if percent == 0 {
				U512::MAX
			} else {
				// An edge key is integral. Round up so an exact equality or a lower
				// implied USD price compares at or below this reference.
				let percent = U512::from(percent);
				(reference_scale + percent - U512::one()) / percent
			}
		};
		Self {
			argon_is_token0: band.argon_is_token0,
			spacing: band.uniswap_tick_spacing,
			lower_percent,
			upper_percent,
			edge_scale,
			minimum_reference_key: at_drift(PERCENT_SCALE - tolerance),
			maximum_reference_key: at_drift(PERCENT_SCALE + tolerance),
		}
	}

	fn compare_reference_keys(&self, left: U512, right: U512) -> Ordering {
		if self.argon_is_token0 {
			left.cmp(&right)
		} else {
			right.cmp(&left)
		}
	}

	fn earlier_reference_key(&self, left: U512, right: U512) -> U512 {
		if self.compare_reference_keys(left, right).is_gt() {
			right
		} else {
			left
		}
	}

	fn edge_reference_scale(&self, edge: BandEdge) -> U512 {
		let percent = match (self.argon_is_token0, edge) {
			(true, BandEdge::Upper) | (false, BandEdge::Lower) => self.lower_percent,
			(true, BandEdge::Lower) | (false, BandEdge::Upper) => self.upper_percent,
		};
		self.edge_scale * percent
	}

	fn edge_reference_key(&self, sqrt_price: UniswapSqrtPriceX96, edge: BandEdge) -> U512 {
		// uint160 squared, uint128 USD quote, decimal scale and percentage factors
		// need at most 508 bits together.
		let sqrt_price = U512::from(sqrt_price);
		sqrt_price * sqrt_price * self.edge_reference_scale(edge)
	}

	fn grid_price(&self, grid_index: i32, edge: BandEdge) -> Result<U512, PositionAmountError> {
		let usd_ordered_tick =
			grid_index.saturating_mul(self.spacing).clamp(MIN_TICK_I32, MAX_TICK_I32);
		let uniswap_tick = if self.argon_is_token0 { usd_ordered_tick } else { -usd_ordered_tick };
		Ok(self.edge_reference_key(sqrt_price_at_uniswap_tick(uniswap_tick)?, edge))
	}

	fn next_grid_reference_key(
		&self,
		grid_index: i32,
		last_grid_index: i32,
		edge: BandEdge,
	) -> Result<U512, PositionAmountError> {
		if grid_index > last_grid_index {
			return Ok(if self.argon_is_token0 { U512::MAX } else { U512::zero() });
		}
		self.grid_price(grid_index.saturating_add(1), edge)
	}

	/// Solve the minimum reference key for sqrt price. Inverting the token order reverses
	/// the inequality, so USD round-down then requires rounding the Uniswap price up.
	fn grid_index_at_minimum(&self, edge: BandEdge) -> Result<i32, PositionAmountError> {
		let floor_sqrt_price =
			(self.minimum_reference_key / self.edge_reference_scale(edge)).integer_sqrt();
		let mut sqrt_price =
			U256::try_from(floor_sqrt_price).map_err(|_| PositionAmountError::InvalidPrice)?;
		if !self.argon_is_token0 &&
			self.edge_reference_key(sqrt_price, edge) < self.minimum_reference_key
		{
			sqrt_price = sqrt_price.saturating_add(U256::one());
		}
		let tick = rounded_uniswap_tick(sqrt_price, !self.argon_is_token0)?;
		let usd_ordered_tick = if self.argon_is_token0 { tick } else { -tick };
		Ok(usd_ordered_tick.div_euclid(self.spacing))
	}

	fn uniswap_range(&self, lower: i32, upper: i32) -> (UniswapTick, UniswapTick) {
		if self.argon_is_token0 {
			(lower * self.spacing, upper * self.spacing)
		} else {
			(-upper * self.spacing, -lower * self.spacing)
		}
	}
}

/// Value summed raw liquidity with identical Uniswap bounds at one pool price.
/// The wider liquidity/result types let frame totals cover multiple NFTs without narrowing
/// their sum to one NFT's uint128 capacity. Eligibility and reward policy belong to the caller.
pub(crate) fn principal_for_liquidity_range(
	liquidity: U256,
	lower_uniswap_tick: UniswapTick,
	upper_uniswap_tick: UniswapTick,
	sqrt_price_x96: UniswapSqrtPriceX96,
	argon_is_token0: bool,
) -> Result<PositionAmounts<U256>, PositionAmountError> {
	if lower_uniswap_tick >= upper_uniswap_tick {
		return Err(PositionAmountError::InvalidTickRange);
	}
	let sqrt_lower = sqrt_price_at_uniswap_tick(lower_uniswap_tick)?;
	let sqrt_upper = sqrt_price_at_uniswap_tick(upper_uniswap_tick)?;
	principal_for_sqrt_price_range(
		liquidity,
		sqrt_lower,
		sqrt_upper,
		sqrt_price_x96,
		argon_is_token0,
	)
}

/// Value principal between already validated Uniswap sqrt-price boundaries. Reward-band clipping
/// may place these boundaries between usable grid ticks; it must not expand the original range.
fn principal_for_sqrt_price_range(
	liquidity: U256,
	sqrt_lower: UniswapSqrtPriceX96,
	sqrt_upper: UniswapSqrtPriceX96,
	sqrt_price_x96: UniswapSqrtPriceX96,
	argon_is_token0: bool,
) -> Result<PositionAmounts<U256>, PositionAmountError> {
	let minimum_sqrt_price = U256::from_big_endian(&MIN_SQRT_RATIO.to_be_bytes::<20>());
	let maximum_sqrt_price = U256::from_big_endian(&MAX_SQRT_RATIO.to_be_bytes::<20>());
	if sqrt_price_x96 < minimum_sqrt_price {
		return Err(PositionAmountError::InvalidPrice);
	}
	if sqrt_price_x96 >= maximum_sqrt_price {
		return Err(PositionAmountError::InvalidPrice);
	}

	// Below the range all principal is token0; above it all principal is token1.
	let sqrt_price = U512::from(sqrt_price_x96.clamp(sqrt_lower, sqrt_upper));
	let sqrt_lower = U512::from(sqrt_lower);
	let sqrt_upper = U512::from(sqrt_upper);
	let liquidity = U512::from(liquidity);
	let q96 = U512::one() << 96;

	// Uniswap's round-down delta equations, extended beyond the SDK's uint128 liquidity.
	// uint256 liquidity * uint160 price difference * Q96 fits in 512 bits. This width is
	// for the fixed-point intermediate, not a native account balance.
	let amount0 = liquidity * (sqrt_upper - sqrt_price) * q96 / (sqrt_price * sqrt_upper);
	let amount1 = liquidity * (sqrt_price - sqrt_lower) / q96;
	let (argon, usdc) = if argon_is_token0 { (amount0, amount1) } else { (amount1, amount0) };

	let token_units_per_microgon = 10u128.pow(ARGON_TOKEN_DECIMALS) / MICROGONS_PER_ARGON;
	let argon = U256::try_from(argon / token_units_per_microgon)
		.map_err(|_| PositionAmountError::AmountOverflow)?;
	let usdc = U256::try_from(usdc).map_err(|_| PositionAmountError::AmountOverflow)?;
	Ok(PositionAmounts { argon, usdc })
}

pub(crate) fn sqrt_price_at_uniswap_tick(
	uniswap_tick: UniswapTick,
) -> Result<UniswapSqrtPriceX96, PositionAmountError> {
	if !(MIN_TICK_I32..=MAX_TICK_I32).contains(&uniswap_tick) {
		return Err(PositionAmountError::InvalidTickRange);
	}
	let uniswap_tick = I24::try_from(uniswap_tick).expect("validated Uniswap tick fits int24");
	let sqrt_price =
		get_sqrt_ratio_at_tick(uniswap_tick).map_err(|_| PositionAmountError::InvalidTickRange)?;
	Ok(U256::from_big_endian(&sqrt_price.to_be_bytes::<20>()))
}

/// Convert a price to its enclosing Uniswap tick. A price past the supported domain uses
/// the adjacent out-of-range tick so callers can tell it never reaches a usable boundary.
fn rounded_uniswap_tick(
	sqrt_price_x96: UniswapSqrtPriceX96,
	round_up: bool,
) -> Result<UniswapTick, PositionAmountError> {
	let min_price = U256::from_big_endian(&MIN_SQRT_RATIO.to_be_bytes::<20>());
	let max_price = U256::from_big_endian(&MAX_SQRT_RATIO.to_be_bytes::<20>());
	if sqrt_price_x96 < min_price {
		return Ok(MIN_TICK_I32.saturating_sub(1));
	}
	if sqrt_price_x96 > max_price {
		return Ok(MAX_TICK_I32.saturating_add(1));
	}
	if sqrt_price_x96 == max_price {
		return Ok(MAX_TICK_I32);
	}

	let price = AlloyU256::from_be_bytes(sqrt_price_x96.to_big_endian());
	let floor_tick = get_tick_at_sqrt_ratio(price)
		.map_err(|_| PositionAmountError::InvalidPrice)?
		.as_i32();
	if round_up && sqrt_price_at_uniswap_tick(floor_tick)? < sqrt_price_x96 {
		return Ok(floor_tick.saturating_add(1));
	}
	Ok(floor_tick)
}

/// Encode token1/token0 as Q96. Round inward when selecting either end of a reward band.
fn sqrt_price_from_token_ratio(
	numerator: U256,
	denominator: U256,
	round_up: bool,
) -> Result<UniswapSqrtPriceX96, PositionAmountError> {
	if denominator.is_zero() {
		return Err(PositionAmountError::InvalidPrice);
	}
	let numerator = U512::from(numerator) << 192;
	let denominator = U512::from(denominator);
	let mut sqrt_price = (numerator / denominator).integer_sqrt();
	if round_up && sqrt_price * sqrt_price * denominator < numerator {
		sqrt_price += U512::one();
	}
	U256::try_from(sqrt_price).map_err(|_| PositionAmountError::AmountOverflow)
}

/// Withdrawable principal, excluding fees. NFTs use native balances; shared ranges use U256.
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
pub struct PositionAmounts<Amount = Balance> {
	/// ARGON principal in native microgons (6 decimals), rounded down from 18-decimal token units.
	pub argon: Amount,
	/// USDC principal in Ethereum token base units (6 decimals).
	pub usdc: Amount,
}

/// Invalid inputs or principal that cannot be represented by the result's integer type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionAmountError {
	/// Uniswap tick bounds are reversed, equal, outside the supported domain, or off the pool's
	/// grid. A configured tick spacing outside 1..=32767 is also invalid.
	InvalidTickRange,
	/// A USD quote is zero or the raw Uniswap sqrt price is outside the supported domain.
	InvalidPrice,
	/// Converted principal exceeds a position's 128-bit balance or a shared range's 256-bit
	/// amount.
	AmountOverflow,
	/// The reward band's lower percentage is zero or at or above its upper percentage.
	InvalidRewardPriceBand,
}
