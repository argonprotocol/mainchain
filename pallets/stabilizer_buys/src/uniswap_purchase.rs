use crate::PurchasePriceReference;
use argon_primitives::{ethereum::UniswapPurchase, MICROGONS_PER_ARGON};
use pallet_prelude::*;
use polkadot_sdk::{sp_core::U512, sp_runtime::ArithmeticError};

/// Pool identity commits to the tokens, fee and hook. ARGON has 18 decimals and USDC has six.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq, TypeInfo, MaxEncodedLen,
)]
pub struct PurchasePool {
	/// Approved Ethereum execution chain ID.
	pub chain_id: u64,
	/// Approved Uniswap v4 PoolManager contract on that chain.
	pub pool_manager: H160,
	/// Approved pool ID within that manager, including its token, fee and hook configuration.
	pub pool_id: H256,
	/// Approved six-decimal USDC spending token on that chain.
	pub usdc_token: H160,
	/// Whether 18-decimal ARGON is token0; determines which direction raises its USD price.
	pub argon_is_token0: bool,
}

impl PurchasePool {
	/// Match the complete execution venue and spending asset, not a pool ID across all chains.
	pub fn matches_purchase(&self, purchase: &UniswapPurchase) -> bool {
		purchase.chain_id == self.chain_id &&
			purchase.pool_manager == self.pool_manager &&
			purchase.pool_id == self.pool_id &&
			purchase.usdc_token == self.usdc_token
	}

	/// Whole-purchase net discount in target-valued microgons. Round target value down, spending
	/// up, and the final conversion down so fractional precision cannot increase a reward.
	pub fn captured_discount(
		&self,
		purchase: &UniswapPurchase,
		prices: PurchasePriceReference,
	) -> Result<Balance, ArithmeticError> {
		if !self.matches_purchase(purchase) {
			return Ok(0);
		}
		if prices.argon_target_usd.is_zero() {
			return Err(ArithmeticError::DivisionByZero);
		}
		if prices.usdc_usd.is_zero() {
			return Err(ArithmeticError::DivisionByZero);
		}
		if purchase.sqrt_price_before_x96.is_zero() {
			return Err(ArithmeticError::DivisionByZero);
		}
		if purchase.sqrt_price_after_x96.is_zero() {
			return Err(ArithmeticError::DivisionByZero);
		}
		if purchase.sqrt_price_before_x96.bits() > 160 {
			return Err(ArithmeticError::Overflow);
		}
		if purchase.sqrt_price_after_x96.bits() > 160 {
			return Err(ArithmeticError::Overflow);
		}

		if self.argon_is_token0 {
			if purchase.sqrt_price_after_x96 <= purchase.sqrt_price_before_x96 {
				return Ok(0);
			}
		} else if purchase.sqrt_price_after_x96 >= purchase.sqrt_price_before_x96 {
			return Ok(0);
		}

		let Some(maximum_start_price) =
			prices.argon_target_usd.into_inner().checked_sub(FixedU128::DIV / 100)
		else {
			return Ok(0);
		};
		let starting_sqrt_price = U512::from(purchase.sqrt_price_before_x96);
		let squared_start_price = starting_sqrt_price
			.checked_mul(starting_sqrt_price)
			.ok_or(ArithmeticError::Overflow)?;
		let q192 = U512::one() << 192;
		let token_decimal_scale = U512::from(1_000_000_000_000u64);
		let usdc_usd = U512::from(prices.usdc_usd.into_inner());
		let (numerator, denominator) = if self.argon_is_token0 {
			(
				squared_start_price
					.checked_mul(token_decimal_scale)
					.and_then(|scaled_price| scaled_price.checked_mul(usdc_usd))
					.ok_or(ArithmeticError::Overflow)?,
				q192,
			)
		} else {
			(
				q192.checked_mul(token_decimal_scale)
					.and_then(|scaled_price| scaled_price.checked_mul(usdc_usd))
					.ok_or(ArithmeticError::Overflow)?,
				squared_start_price,
			)
		};
		if divide_rounded_up(numerator, denominator)? > U512::from(maximum_start_price) {
			return Ok(0);
		}

		let microgons_per_argon = U512::from(MICROGONS_PER_ARGON);
		let argon_target_usd = U512::from(prices.argon_target_usd.into_inner());
		let target_usd_value = U512::from(purchase.principal)
			.checked_mul(argon_target_usd)
			.ok_or(ArithmeticError::Overflow)? /
			microgons_per_argon;
		let usdc_value = U512::from(purchase.usdc_spent)
			.checked_mul(usdc_usd)
			.ok_or(ArithmeticError::Overflow)?;
		let usd_spent = divide_rounded_up(usdc_value, microgons_per_argon)?;
		if usd_spent >= target_usd_value {
			return Ok(0);
		}

		let discount = (target_usd_value - usd_spent)
			.checked_mul(microgons_per_argon)
			.ok_or(ArithmeticError::Overflow)? /
			argon_target_usd;
		if discount > U512::from(u128::MAX) {
			return Err(ArithmeticError::Overflow);
		}
		Ok(discount.low_u128())
	}
}

fn divide_rounded_up(numerator: U512, denominator: U512) -> Result<U512, ArithmeticError> {
	if denominator.is_zero() {
		return Err(ArithmeticError::DivisionByZero);
	}
	let quotient = numerator / denominator;
	if numerator % denominator == U512::zero() {
		return Ok(quotient);
	}
	quotient.checked_add(U512::one()).ok_or(ArithmeticError::Overflow)
}
