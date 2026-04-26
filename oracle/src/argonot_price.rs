use crate::{argon_price::get_usdc_token, uniswap_oracle::UniswapOracle};
use anyhow::Result;
use polkadot_sdk::*;
use sp_runtime::FixedU128;
use std::env;
use uniswap_sdk_core::{prelude::*, token};

#[allow(dead_code)]
pub struct ArgonotPriceLookup {
	pub last_price: FixedU128,
	pub uniswap_oracle: UniswapOracle,
}

impl ArgonotPriceLookup {
	pub async fn new(
		project_id: String,
		usd_token: Token,
		lookup_token: Token,
		last_price: FixedU128,
	) -> Result<Self> {
		let uniswap_oracle = UniswapOracle::new(project_id, usd_token, lookup_token).await?;

		Ok(Self { last_price, uniswap_oracle })
	}

	pub async fn from_env(last_price: FixedU128) -> Result<Self> {
		let use_sepolia = env::var("USE_SEPOLIA").unwrap_or_default() == "true";
		let argonot_token_address =
			env::var("ARGONOT_TOKEN_ADDRESS").expect("ARGONOT_TOKEN_ADDRESS must be set");
		let network = if use_sepolia { ChainId::SEPOLIA } else { ChainId::MAINNET };
		let project_id = env::var("INFURA_PROJECT_ID").expect("INFURA_PROJECT_ID must be set");

		let usdc_token = get_usdc_token(network);
		let lookup_token = token!(network as u64, argonot_token_address, 18);
		Self::new(project_id, usdc_token, lookup_token, last_price).await
	}

	pub async fn get_latest_price(&mut self, usd_token_price: FixedU128) -> Result<FixedU128> {
		let price = self.uniswap_oracle.get_current_price().await?;
		// ARGONOT/USDC * USDC/USD = ARGONOT/USD
		let price = price.price * usd_token_price;
		self.last_price = price;
		Ok(price)
	}

	pub fn hold_last_price(&self) -> FixedU128 {
		self.last_price
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::uniswap_oracle::{PriceAndLiquidity, use_mock_uniswap_prices};
	use std::str::FromStr;

	const MOCK_ARGONOT_TOKEN_ADDRESS: &str = "64CBd3aa07d427E385Cb55330406508718E55f01";

	fn before_each() {
		dotenv::dotenv().ok();
		unsafe {
			env::set_var("USE_SEPOLIA", "true");
			env::set_var("INFURA_PROJECT_ID", "test");
			env::set_var("ARGONOT_TOKEN_ADDRESS", MOCK_ARGONOT_TOKEN_ADDRESS);
		}
	}

	#[tokio::test]
	async fn holds_last_price_without_resetting_it() {
		before_each();
		let mut argonot_price_lookup =
			ArgonotPriceLookup::from_env(FixedU128::from_float(0.017)).await.unwrap();

		let argonot_usdc_price = FixedU128::from_float(0.02);
		let address = Address::from_str(MOCK_ARGONOT_TOKEN_ADDRESS).unwrap();
		use_mock_uniswap_prices(
			address,
			vec![PriceAndLiquidity { price: argonot_usdc_price, liquidity: 100_000_000 }],
		);

		let latest = argonot_price_lookup
			.get_latest_price(FixedU128::from_float(0.99))
			.await
			.unwrap();
		assert_eq!(latest, FixedU128::from_float(0.0198));
		assert_eq!(argonot_price_lookup.hold_last_price(), FixedU128::from_float(0.0198));
	}
}
