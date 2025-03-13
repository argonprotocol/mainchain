use crate::{argon_price::get_usdc_token, uniswap_oracle::UniswapOracle};
use anyhow::Result;
use sp_runtime::FixedU128;
use std::env;
use uniswap_sdk_core::{prelude::*, token};

#[allow(dead_code)]
pub struct ArgonotPriceLookup {
	pub uniswap_oracle: UniswapOracle,
}

impl ArgonotPriceLookup {
	pub async fn new(project_id: String, usd_token: Token, lookup_token: Token) -> Result<Self> {
		let uniswap_oracle = UniswapOracle::new(project_id, usd_token, lookup_token).await?;

		Ok(Self { uniswap_oracle })
	}

	pub async fn from_env() -> Result<Self> {
		let use_sepolia = env::var("USE_SEPOLIA").unwrap_or_default() == "true";
		let argonot_token_address =
			env::var("ARGONOT_TOKEN_ADDRESS").expect("ARGONOT_TOKEN_ADDRESS must be set");
		let network = if use_sepolia { ChainId::SEPOLIA } else { ChainId::MAINNET };
		let project_id = env::var("INFURA_PROJECT_ID").expect("INFURA_PROJECT_ID must be set");

		let usdc_token = get_usdc_token(network);
		let lookup_token = token!(network as u64, argonot_token_address, 18);
		Self::new(project_id, usdc_token, lookup_token).await
	}

	pub async fn get_latest_price(&self, usd_token_price: FixedU128) -> Result<FixedU128> {
		let price = self.uniswap_oracle.get_current_price().await?;
		// ARGONOT/USDC * USDC/USD = ARGONOT/USD
		Ok(price.price * usd_token_price)
	}
}
