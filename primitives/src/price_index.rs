use crate::{tick::Tick, ArgonCPI, Balance};
use codec::{Decode, DecodeAll, DecodeWithMemTracking, Encode, MaxEncodedLen};
use polkadot_sdk::{
	sp_arithmetic::FixedPointNumber,
	sp_core::U256,
	sp_runtime::{
		traits::{CheckedDiv, One, Zero},
		FixedI128, FixedU128,
	},
};
use scale_info::TypeInfo;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Eq,
	PartialEq,
	Clone,
	Copy,
	Ord,
	PartialOrd,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PriceIndex {
	/// USD per Bitcoin
	#[codec(compact)]
	pub btc_usd_price: FixedU128,
	/// USD per Argon ownership token (ARGONOT)
	pub argonot_usd_price: FixedU128,
	/// USD per ARGON
	#[codec(compact)]
	pub argon_usd_price: FixedU128,
	/// The target price for argon based on inflation since start
	pub argon_usd_target_price: FixedU128,
	/// The argon liquidity in the pool
	pub argon_time_weighted_average_liquidity: Balance,
	/// Tick of price index
	#[codec(compact)]
	pub tick: Tick,
	/// Observed USD per USDC. None means no exchange observation was available.
	pub usdc_usd_price: Option<FixedU128>,
	/// Raw sqrt(token1/token0) * 2^96 for the chain-configured v4 stabilizer pool.
	/// None means the pool is unconfigured or no current observation was available.
	pub stabilizer_pool_sqrt_price_x96: Option<U256>,
}

impl PriceIndex {
	/// Read either supported wire layout during an oracle/runtime upgrade.
	pub fn decode_compatible(bytes: &[u8]) -> Result<Self, codec::Error> {
		Self::decode_all(&mut &bytes[..])
			.or_else(|_| PriceIndexV1::decode_all(&mut &bytes[..]).map(Self::from))
	}

	pub fn argon_cpi(&self) -> ArgonCPI {
		// if the difference is less than 0.001, treat it as normal market slippage/noise
		let fixed_i_argon_usd_price =
			FixedI128::from_inner(self.argon_usd_price.into_inner() as i128);
		let fixed_i_argon_usd_target_price =
			FixedI128::from_inner(self.argon_usd_target_price.into_inner() as i128);
		if (fixed_i_argon_usd_price - fixed_i_argon_usd_target_price).saturating_abs() <
			FixedI128::from_rational(1, 1000)
		{
			return ArgonCPI::zero();
		}
		let ratio = fixed_i_argon_usd_target_price
			.checked_div(&fixed_i_argon_usd_price)
			.unwrap_or(FixedI128::one());
		ratio - FixedI128::one()
	}

	pub fn redemption_r_value(&self) -> FixedU128 {
		self.argon_usd_price
			.checked_div(&self.argon_usd_target_price)
			.unwrap_or(FixedU128::one())
	}
}

/// Pre-USDC wire layout retained for oracle/runtime upgrades.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Eq,
	PartialEq,
	Clone,
	Copy,
	Ord,
	PartialOrd,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PriceIndexV1 {
	/// USD per Bitcoin
	#[codec(compact)]
	pub btc_usd_price: FixedU128,
	/// USD per Argon ownership token (ARGONOT)
	pub argonot_usd_price: FixedU128,
	/// USD per ARGON
	#[codec(compact)]
	pub argon_usd_price: FixedU128,
	/// The target price for argon based on inflation since start
	pub argon_usd_target_price: FixedU128,
	/// The argon liquidity in the pool
	pub argon_time_weighted_average_liquidity: Balance,
	/// Tick of price index
	#[codec(compact)]
	pub tick: Tick,
}

impl From<PriceIndexV1> for PriceIndex {
	fn from(old: PriceIndexV1) -> Self {
		Self {
			btc_usd_price: old.btc_usd_price,
			argonot_usd_price: old.argonot_usd_price,
			argon_usd_price: old.argon_usd_price,
			argon_usd_target_price: old.argon_usd_target_price,
			argon_time_weighted_average_liquidity: old.argon_time_weighted_average_liquidity,
			tick: old.tick,
			usdc_usd_price: None,
			stabilizer_pool_sqrt_price_x96: None,
		}
	}
}
