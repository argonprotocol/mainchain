//! Uniswap principal valuation for individual NFTs and shared liquidity ranges.
use crate::PositionCheckpoint;
use alloy_primitives::aliases::I24;
use argon_primitives::{Balance, MICROGONS_PER_ARGON};
use pallet_prelude::*;
use polkadot_sdk::sp_core::U512;
use uniswap_v3_sdk_bigint::utils::{
	get_sqrt_ratio_at_tick, MAX_SQRT_RATIO, MAX_TICK_I32, MIN_SQRT_RATIO, MIN_TICK_I32,
};

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
			U256::from(self.liquidity),
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
	if !(MIN_TICK_I32..=MAX_TICK_I32).contains(&lower_uniswap_tick) {
		return Err(PositionAmountError::InvalidTickRange);
	}
	if !(MIN_TICK_I32..=MAX_TICK_I32).contains(&upper_uniswap_tick) {
		return Err(PositionAmountError::InvalidTickRange);
	}

	let lower_uniswap_tick =
		I24::try_from(lower_uniswap_tick).expect("validated Uniswap tick fits int24");
	let upper_uniswap_tick =
		I24::try_from(upper_uniswap_tick).expect("validated Uniswap tick fits int24");
	let sqrt_lower = get_sqrt_ratio_at_tick(lower_uniswap_tick)
		.map_err(|_| PositionAmountError::InvalidTickRange)?;
	let sqrt_upper = get_sqrt_ratio_at_tick(upper_uniswap_tick)
		.map_err(|_| PositionAmountError::InvalidTickRange)?;
	let sqrt_lower = U256::from_big_endian(&sqrt_lower.to_be_bytes::<20>());
	let sqrt_upper = U256::from_big_endian(&sqrt_upper.to_be_bytes::<20>());

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

	let token_units_per_microgon = U512::from(10u128.pow(18) / MICROGONS_PER_ARGON);
	let argon = U256::try_from(argon / token_units_per_microgon)
		.map_err(|_| PositionAmountError::AmountOverflow)?;
	let usdc = U256::try_from(usdc).map_err(|_| PositionAmountError::AmountOverflow)?;
	Ok(PositionAmounts { argon, usdc })
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
	/// Uniswap tick bounds are reversed, equal, or outside the supported range.
	InvalidTickRange,
	/// The raw Uniswap sqrt price is outside the supported domain.
	InvalidPrice,
	/// Converted principal exceeds a position's 128-bit balance or a shared range's 256-bit
	/// amount.
	AmountOverflow,
}
