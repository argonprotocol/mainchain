//! Uniswap principal valuation for individual NFTs and shared liquidity ranges.
use crate::PositionCheckpoint;
use alloy_primitives::aliases::I24;
use argon_primitives::{Balance, MICROGONS_PER_ARGON};
use pallet_prelude::*;
use polkadot_sdk::sp_core::U512;
use uniswap_v3_sdk_bigint::utils::{
	get_sqrt_ratio_at_tick, MAX_SQRT_RATIO, MAX_TICK_I32, MIN_SQRT_RATIO, MIN_TICK_I32,
};

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
		sqrt_price_x96: U256,
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
	sqrt_lower: U256,
	sqrt_upper: U256,
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
		lower_uniswap_tick: i32,
		upper_uniswap_tick: i32,
		pool_sqrt_price_x96: U256,
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

	/// Each endpoint rounds down in USD to a usable Uniswap tick. Their reference intervals must
	/// overlap, and that overlap must intersect the allowed target drift. Compare exact ratios:
	/// rounding each endpoint to a USD integer could accept a shortened band.
	fn qualifying_range(
		&self,
		lower_uniswap_tick: i32,
		upper_uniswap_tick: i32,
	) -> Result<Option<(U256, U256)>, PositionAmountError> {
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
		let toward_higher_usd_price = if self.argon_is_token0 {
			self.uniswap_tick_spacing
		} else {
			-self.uniswap_tick_spacing
		};
		// The last grid interval ends at Uniswap's supported price limit. Its synthetic next
		// boundary can fall outside that domain even when the NFT's own boundaries are valid.
		let next_lower_uniswap_tick =
			(lower_uniswap_tick + toward_higher_usd_price).clamp(MIN_TICK_I32, MAX_TICK_I32);
		let next_upper_uniswap_tick =
			(upper_uniswap_tick + toward_higher_usd_price).clamp(MIN_TICK_I32, MAX_TICK_I32);
		let next_lower = sqrt_price_at_uniswap_tick(next_lower_uniswap_tick)?;
		let next_upper = sqrt_price_at_uniswap_tick(next_upper_uniswap_tick)?;

		let [lower_square, upper_square, next_lower_square, next_upper_square] =
			[sqrt_lower, sqrt_upper, next_lower, next_upper].map(|sqrt_price| {
				let sqrt_price = U512::from(sqrt_price);
				sqrt_price * sqrt_price
			});
		let target_usd = U512::from(self.argon_usd_target_price.into_inner());
		let usdc_usd_scaled =
			U512::from(self.usdc_usd_price.into_inner()) * ARGON_USDC_DECIMAL_SCALE;
		let tolerance = self.reference_tolerance.deconstruct();
		let minimum_reference_usd_percent_scaled = target_usd * (PERCENT_SCALE - tolerance);
		let maximum_reference_usd_percent_scaled = target_usd * (PERCENT_SCALE + tolerance);
		let (lower_percent, upper_percent) = self.reward_price_band;
		let lower_percent = lower_percent.deconstruct();
		let upper_percent = upper_percent.deconstruct();
		// One percentage scale recovers the reference price from an edge. The other
		// matches the scale of the allowed reference prices above.
		let reference_usd_scale = usdc_usd_scaled * PERCENT_SCALE * PERCENT_SCALE;
		let q192 = U512::one() << 192;

		// Squared uint160 prices, uint128 USD quotes, the 10^12 token scale, and these
		// percentage factors need at most 508 bits. They cannot overflow U512.
		if self.argon_is_token0 {
			// Recover the reference by dividing each edge by its own percentage.
			// Use a common denominator to compare without rounding either edge.
			let reference_price_denominator = q192 * lower_percent * upper_percent;
			let minimum_reference_numerator =
				(lower_square * upper_percent).max(upper_square * lower_percent);
			let exclusive_maximum_reference_numerator =
				(next_lower_square * upper_percent).min(next_upper_square * lower_percent);
			if minimum_reference_numerator >= exclusive_maximum_reference_numerator {
				return Ok(None);
			}
			if minimum_reference_numerator * reference_usd_scale >
				maximum_reference_usd_percent_scaled * reference_price_denominator
			{
				return Ok(None);
			}
			if exclusive_maximum_reference_numerator * reference_usd_scale <=
				minimum_reference_usd_percent_scaled * reference_price_denominator
			{
				return Ok(None);
			}
		} else {
			// The lower Uniswap tick is the upper USD edge. Inverted prices share a
			// numerator, so the larger denominator means a lower USD reference.
			let minimum_reference_denominator =
				(lower_square * upper_percent).min(upper_square * lower_percent);
			let exclusive_maximum_reference_denominator =
				(next_lower_square * upper_percent).max(next_upper_square * lower_percent);
			if minimum_reference_denominator <= exclusive_maximum_reference_denominator {
				return Ok(None);
			}
			let reference_numerator = reference_usd_scale * q192;
			if reference_numerator >
				maximum_reference_usd_percent_scaled * minimum_reference_denominator
			{
				return Ok(None);
			}
			if reference_numerator <=
				minimum_reference_usd_percent_scaled * exclusive_maximum_reference_denominator
			{
				return Ok(None);
			}
		}
		Ok(Some((sqrt_lower, sqrt_upper)))
	}
}

/// Value summed raw liquidity with identical Uniswap bounds at one pool price.
/// The wider liquidity/result types let frame totals cover multiple NFTs without narrowing
/// their sum to one NFT's uint128 capacity. Eligibility and reward policy belong to the caller.
pub(crate) fn principal_for_liquidity_range(
	liquidity: U256,
	lower_uniswap_tick: i32,
	upper_uniswap_tick: i32,
	sqrt_price_x96: U256,
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
	sqrt_lower: U256,
	sqrt_upper: U256,
	sqrt_price_x96: U256,
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

pub(crate) fn sqrt_price_at_uniswap_tick(uniswap_tick: i32) -> Result<U256, PositionAmountError> {
	if !(MIN_TICK_I32..=MAX_TICK_I32).contains(&uniswap_tick) {
		return Err(PositionAmountError::InvalidTickRange);
	}
	let uniswap_tick = I24::try_from(uniswap_tick).expect("validated Uniswap tick fits int24");
	let sqrt_price =
		get_sqrt_ratio_at_tick(uniswap_tick).map_err(|_| PositionAmountError::InvalidTickRange)?;
	Ok(U256::from_big_endian(&sqrt_price.to_be_bytes::<20>()))
}

/// Encode token1/token0 as Q96. Round inward when selecting either end of a reward band.
fn sqrt_price_from_token_ratio(
	numerator: U256,
	denominator: U256,
	round_up: bool,
) -> Result<U256, PositionAmountError> {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
