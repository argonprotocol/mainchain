use crate::uniswap_oracle::{PriceAndLiquidity, USDC_ADDRESS, USDC_ADDRESS_SEPOLIA, UniswapOracle};
use anyhow::Result;
use argon_client::api::runtime_types::pallet_price_index::PriceIndex;
use argon_primitives::{
	ARGON_TOKEN_SYMBOL,
	tick::{Tick, Ticker},
};
use polkadot_sdk::*;
use sp_runtime::{FixedI128, FixedPointNumber, FixedU128, Saturating, traits::One};
use std::env;
use uniswap_sdk_core::{prelude::*, token};

#[allow(dead_code)]
pub struct ArgonPriceLookup {
	pub ticker: Ticker,
	pub last_price: PriceAndLiquidity,
	pub last_price_tick: Tick,
	pub uniswap_oracle: UniswapOracle,
}

pub fn get_usdc_token(chain: ChainId) -> Token {
	let address = if chain == ChainId::SEPOLIA { USDC_ADDRESS_SEPOLIA } else { USDC_ADDRESS };

	token!(chain as u64, address, 6, "USDC", "USD Coin")
}

impl ArgonPriceLookup {
	pub async fn new(
		ticker: &Ticker,
		last_price: Option<PriceIndex>,
		project_id: String,
		usd_token: Token,
		lookup_token: Token,
	) -> Result<Self> {
		let uniswap_oracle = UniswapOracle::new(project_id, usd_token, lookup_token).await?;

		Ok(Self {
			uniswap_oracle,
			ticker: *ticker,
			last_price: last_price
				.as_ref()
				.map(|a| PriceAndLiquidity {
					price: FixedU128::from_inner(a.argon_usd_price.0),
					liquidity: a.argon_time_weighted_average_liquidity,
				})
				.unwrap_or(PriceAndLiquidity { price: FixedU128::from_u32(1), liquidity: 0 }),
			last_price_tick: last_price
				.map(|a| a.tick)
				.unwrap_or(ticker.current().saturating_sub(1)),
		})
	}

	pub async fn from_env(ticker: &Ticker, last_price: Option<PriceIndex>) -> Result<Self> {
		let use_sepolia = env::var("USE_SEPOLIA").unwrap_or_default() == "true";
		let argon_token_address =
			env::var("ARGON_TOKEN_ADDRESS").expect("ARGON_TOKEN_ADDRESS must be set");
		let network = if use_sepolia { ChainId::SEPOLIA } else { ChainId::MAINNET };
		let project_id = env::var("INFURA_PROJECT_ID").expect("INFURA_PROJECT_ID must be set");

		let usdc_token = get_usdc_token(network);
		let lookup_token =
			token!(network as u64, argon_token_address, 18, ARGON_TOKEN_SYMBOL, "Argon");
		Self::new(ticker, last_price, project_id, usdc_token, lookup_token).await
	}

	/// Calculates the expected cost of an Argon in USD based on the starting and current U.S. CPI.
	pub fn get_target_price(&self, us_cpi_ratio: FixedI128) -> FixedU128 {
		let cpi_as_u128 = FixedI128::one() + us_cpi_ratio;
		FixedU128::from_inner(cpi_as_u128.into_inner() as u128)
	}

	pub async fn get_latest_price_and_liquidity(
		&mut self,
		tick: Tick,
		max_argon_change_per_tick_away_from_target: FixedU128,
		usd_token_price: FixedU128,
	) -> Result<PriceAndLiquidity> {
		let mut price = self.uniswap_oracle.get_current_price().await?;
		// ARGON/USDC * USDC/USD = ARGON/USD
		price.price = price.price * usd_token_price;

		price.price =
			self.clamp_price(price.price, tick, max_argon_change_per_tick_away_from_target);
		self.last_price = price;
		self.last_price_tick = tick;

		Ok(price)
	}

	fn clamp_price(
		&self,
		price: FixedU128,
		tick: Tick,
		max_argon_change_per_tick_away_from_target: FixedU128,
	) -> FixedU128 {
		if self.last_price_tick == 0 {
			return price;
		}

		let start_price = self.last_price.price;
		let ticks = tick.saturating_sub(self.last_price_tick).min(10);
		let max_change =
			max_argon_change_per_tick_away_from_target * FixedU128::saturating_from_integer(ticks);
		if price > start_price {
			price.min(start_price.saturating_add(max_change))
		} else {
			price.max(start_price.saturating_sub(max_change))
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::uniswap_oracle::use_mock_uniswap_prices;
	use std::{str::FromStr, time::Duration};

	const DAI_ADDRESS_SEPOLIA: &str = "6b175474e89094c44da98b954eedeac495271d0f";

	fn before_each() {
		dotenv::dotenv().ok();
		unsafe {
			env::set_var("USE_SEPOLIA", "true");
			env::set_var("INFURA_PROJECT_ID", "test");
			env::set_var("ARGON_TOKEN_ADDRESS", DAI_ADDRESS_SEPOLIA);
			env::set_var("ARGONOT_TOKEN_ADDRESS", DAI_ADDRESS_SEPOLIA);
		}
	}

	#[tokio::test]
	async fn test_get_target_price() {
		before_each();
		let ticker = Ticker::start(Duration::from_secs(60), 2);
		let argon_price_lookup = ArgonPriceLookup::from_env(&ticker, None).await.unwrap();
		let us_cpi_ratio = FixedI128::from_float(0.00);
		assert_eq!(argon_price_lookup.get_target_price(us_cpi_ratio), FixedU128::from_u32(1));
	}

	#[tokio::test]
	async fn test_get_target_price_with_cpi() {
		before_each();
		let ticker = Ticker::start(Duration::from_secs(60), 2);
		let argon_price_lookup = ArgonPriceLookup::from_env(&ticker, None).await.unwrap();
		let us_cpi_ratio = FixedI128::from_float(0.1);
		assert_eq!(argon_price_lookup.get_target_price(us_cpi_ratio).to_float(), 1.1);
	}

	#[tokio::test]
	async fn adjusts_price_by_usdc_price() {
		before_each();
		let ticker = Ticker::start(Duration::from_secs(60), 2);
		let mut argon_price_lookup = ArgonPriceLookup::from_env(&ticker, None).await.unwrap();

		let argon_usdc_price = FixedU128::from_float(0.99);
		let address = Address::from_str(DAI_ADDRESS_SEPOLIA).unwrap();
		use_mock_uniswap_prices(
			address,
			vec![PriceAndLiquidity { price: argon_usdc_price, liquidity: 100_000_000 }],
		);
		let usdc_usd_price = FixedU128::from_float(0.99);

		// If the argon/usdc price is 0.99 usdc, and the usdc/usd price is 0.99 usd, the argon/usd
		// price should be 0.99 * 0.99 = 0.9801

		assert_eq!(
			argon_price_lookup
				.get_latest_price_and_liquidity(
					ticker.current() + 1,
					FixedU128::from_float(0.01),
					usdc_usd_price,
				)
				.await
				.unwrap()
				.price,
			FixedU128::from_float(0.9801)
		);
	}
}
