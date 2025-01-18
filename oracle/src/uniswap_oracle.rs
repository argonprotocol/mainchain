use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_primitives::{aliases::I56, Address, U160};
use alloy_provider::RootProvider;
use alloy_transport::BoxTransport;
use anyhow::{anyhow, Result};
use sp_runtime::{FixedPointNumber, FixedU128};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uniswap_lens::bindings::iuniswapv3pool::IUniswapV3Pool::IUniswapV3PoolInstance;
use uniswap_sdk_core::prelude::*;
use uniswap_v3_sdk::{entities::TickIndex, prelude::*};

pub fn get_infura_url(use_sepolia: bool, project_id: String) -> String {
	if use_sepolia {
		format!("https://sepolia.infura.io/v3/{}", project_id)
	} else {
		format!("https://mainnet.infura.io/v3/{}", project_id)
	}
}
pub const USDC_ADDRESS: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
pub(crate) const USDC_ADDRESS_SEPOLIA: Address =
	address!("74ce1e12998fB861A612CD6C65244f8620e2937A");
pub const SEPOLIA_FACTORY_ADDRESS: Address = address!("0227628f3F023bb0B980b67D528571c95c6DaC1c");

#[cfg(test)]
lazy_static::lazy_static! {
	static ref MOCK_PRICES: Arc<parking_lot::Mutex<Vec<FixedU128>>> = Default::default();
}

#[cfg(test)]
pub(crate) fn use_mock_argon_prices(mut prices: Vec<FixedU128>) {
	MOCK_PRICES.lock().append(&mut prices)
}

pub struct UniswapOracle {
	provider: Arc<RootProvider<BoxTransport>>,
	factory_address: Address,
	usd_token: Token,
	lookup_token: Token,
	fee_tiers: Vec<FeeAmount>,
	pool_cache_by_fee:
		Mutex<HashMap<FeeAmount, IUniswapV3PoolInstance<BoxTransport, RootProvider<BoxTransport>>>>, /* Fee -> Pool Contract */
}

impl UniswapOracle {
	pub async fn new(project_id: String, usd_token: Token, lookup_token: Token) -> Result<Self> {
		let mut use_sepolia = false;
		let factory_address = if usd_token.chain_id() == ChainId::SEPOLIA as u64 {
			use_sepolia = true;
			SEPOLIA_FACTORY_ADDRESS
		} else {
			FACTORY_ADDRESS
		};
		let url = get_infura_url(use_sepolia, project_id);
		let provider = RootProvider::connect_builtin(&url).await?;

		Ok(Self {
			provider: Arc::new(provider),
			factory_address,
			usd_token,
			lookup_token,
			// NOTE: taking high tier out since this will be for pricing a stablecoin. High fees are
			// usually for highly volatile assets
			fee_tiers: vec![FeeAmount::LOWEST, FeeAmount::LOW, FeeAmount::MEDIUM],
			pool_cache_by_fee: Default::default(),
		})
	}

	pub async fn get_current_price(&self) -> Result<FixedU128> {
		#[cfg(test)]
		{
			if let Some(price) = MOCK_PRICES.lock().pop() {
				return Ok(price);
			}
		}
		let price = self
			.get_aggregated_twap(60 * 60)
			.await?
			.ok_or(anyhow!("Failed to get price, using default"))?;
		let scaled_numerator = price.adjusted_for_decimals().to_decimal() * FixedU128::accuracy();
		let float = scaled_numerator.to_u128().ok_or(anyhow!("Failed to convert to u128"))?;
		Ok(FixedU128::from_inner(float))
	}

	/// Calculate TWAP and TWAL for a given fee tier
	async fn get_twap_and_twal(
		&self,
		fee: FeeAmount,
		seconds_ago: u32,
	) -> Result<(Price<Token, Token>, BigInt)> {
		if seconds_ago == 0 {
			return Err(anyhow!("seconds_ago must be greater than 0"));
		}
		let block_id = BlockId::Number(BlockNumberOrTag::Latest);
		let pool_contract = self.get_cached_pool_contract(fee).await?;

		// Fetch tick_cumulatives and liquidity_cumulatives
		let result = pool_contract.observe(vec![seconds_ago, 0]).block(block_id).call().await?;
		let tick_cumulatives = result.tickCumulatives;
		let liquidity_cumulatives = result.secondsPerLiquidityCumulativeX128s;

		// Compute tick cumulative difference
		let tick_diff = tick_cumulatives[1] - tick_cumulatives[0];

		// Calculate time-weighted average tick (fixed-point division)
		let tick_twap = {
			let seconds_as_i56 = I56::try_from(seconds_ago)?;
			(tick_diff / seconds_as_i56).to_i24()
		};

		// Convert tick to sqrtPriceX96
		let price = tick_to_price(self.lookup_token.clone(), self.usd_token.clone(), tick_twap)?;

		// Compute average liquidity
		let liquidity_diff = liquidity_cumulatives[1] - liquidity_cumulatives[0];
		let average_liquidity = {
			let seconds_as_u256: U160 = seconds_ago.try_into()?;
			(liquidity_diff / seconds_as_u256).to_big_int()
		};

		Ok((price, average_liquidity))
	}

	/// Aggregate TWAPs across fee tiers, weighted by TWAL
	async fn get_aggregated_twap(&self, seconds_ago: u32) -> Result<Option<Price<Token, Token>>> {
		let mut total_numerator = BigInt::zero();
		let mut total_denominator = BigInt::zero();

		for &fee in &self.fee_tiers {
			if let Ok((price, average_liquidity)) = self.get_twap_and_twal(fee, seconds_ago).await {
				total_numerator += price.numerator * average_liquidity.clone();
				total_denominator += price.denominator * average_liquidity;
			}
		}

		if total_denominator == BigInt::zero() {
			return Ok(None);
		}

		// Combine prices into a single aggregated Price
		Ok(Some(Price::new(
			self.lookup_token.clone(),
			self.usd_token.clone(),
			total_denominator,
			total_numerator,
		)))
	}

	/// Get or cache the pool contract
	async fn get_cached_pool_contract(
		&self,
		fee: FeeAmount,
	) -> Result<IUniswapV3PoolInstance<BoxTransport, RootProvider<BoxTransport>>> {
		let mut cache = self.pool_cache_by_fee.lock().await;

		if let Some(pool) = cache.get(&fee) {
			return Ok((*pool).clone());
		}

		let pool_address = compute_pool_address(
			self.factory_address,
			self.usd_token.address(),
			self.lookup_token.address(),
			fee,
			None,
			None,
		);

		let pool_contract = IUniswapV3PoolInstance::new(pool_address, (*self.provider).clone());
		cache.insert(fee, pool_contract.clone());

		Ok(pool_contract)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::env;
	use tracing::warn;
	use uniswap_sdk_core::token;

	#[tokio::test]
	async fn test_infura_lookup() {
		dotenv::dotenv().ok();
		let Ok(project_id) = env::var("INFURA_PROJECT_ID") else {
			warn!("INFURA_PROJECT_ID not set, skipping test");
			return;
		};

		const DAI_ADDRESS: &str = "6b175474e89094c44da98b954eedeac495271d0f";

		let oracle = UniswapOracle::new(
			project_id,
			token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			token!(ChainId::MAINNET as u64, DAI_ADDRESS, 18, "DAI"),
		)
		.await
		.expect("Failed to create oracle");
		let price = oracle.get_current_price().await.unwrap();
		println!("Price: {:?}", price);
		// should be around 1.0
		assert!((price.to_float() - 1.0).abs() < 0.1);
	}
}
