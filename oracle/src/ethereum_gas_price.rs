use alloy_provider::{network::Ethereum, Provider, RootProvider};
use anyhow::{anyhow, Result};
use tracing::warn;

use crate::uniswap_oracle::ethereum_rpc_urls_from_env;

pub struct EthereumGasPriceLookup {
	providers: Vec<RootProvider<Ethereum>>,
}

impl EthereumGasPriceLookup {
	pub async fn from_env() -> Result<Self> {
		let mut providers = Vec::new();
		for url in ethereum_rpc_urls_from_env()? {
			providers.push(RootProvider::connect(&url).await?);
		}
		Ok(Self { providers })
	}

	pub async fn get_gas_price(&self) -> Result<u128> {
		let mut last_error: Option<anyhow::Error> = None;
		for provider in &self.providers {
			match provider.get_gas_price().await {
				Ok(gas_price) => return Ok(gas_price),
				Err(error) => {
					warn!(?error, "Ethereum gas price lookup failed; trying fallback provider");
					last_error = Some(error.into());
				},
			}
		}

		Err(last_error.unwrap_or_else(|| anyhow!("No Ethereum RPC providers configured")))
	}
}
