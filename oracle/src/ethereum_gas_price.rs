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

#[cfg(test)]
mod tests {
	use super::*;
	use alloy_primitives::U128;
	use alloy_provider::ProviderBuilder;
	use alloy_transport::mock::Asserter;

	#[tokio::test]
	async fn falls_back_after_rpc_failure_and_reports_the_last_failure() {
		let primary = Asserter::new();
		let fallback = Asserter::new();
		let lookup = EthereumGasPriceLookup {
			providers: vec![
				ProviderBuilder::default().connect_mocked_client(primary.clone()),
				ProviderBuilder::default().connect_mocked_client(fallback.clone()),
			],
		};

		primary.push_failure_msg("primary unavailable");
		fallback.push_success(&U128::from(123_456));
		assert_eq!(lookup.get_gas_price().await.unwrap(), 123_456);

		primary.push_failure_msg("primary unavailable");
		fallback.push_failure_msg("fallback unavailable");
		assert!(lookup
			.get_gas_price()
			.await
			.unwrap_err()
			.to_string()
			.contains("fallback unavailable"));
	}
}
