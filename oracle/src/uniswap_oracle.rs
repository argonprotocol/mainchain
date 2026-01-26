use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_primitives::{address, aliases::I56};
use alloy_provider::{RootProvider, network::Ethereum};
use anyhow::{Result, anyhow};
use argon_primitives::{
	Balance,
	prelude::{frame_support::pallet_prelude::Zero, sp_arithmetic::FixedPointNumber},
};
use polkadot_sdk::*;
use sdk_core::prelude::*;
use sp_runtime::FixedU128;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing::{error, trace, warn};
use uniswap_lens::bindings::iuniswapv3pool::IUniswapV3Pool::IUniswapV3PoolInstance;
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
	pub static ref MOCK_PRICES: Arc<parking_lot::Mutex<HashMap<Address, Vec<PriceAndLiquidity>>>> = Default::default();
}

#[cfg(test)]
pub(crate) fn use_mock_uniswap_prices(token_address: Address, mut prices: Vec<PriceAndLiquidity>) {
	MOCK_PRICES.lock().entry(token_address).or_default().append(&mut prices)
}

pub struct UniswapOracle {
	provider: Arc<RootProvider<Ethereum>>,
	factory_address: Address,
	usd_token: Token,
	lookup_token: Token,
	fee_tiers: Vec<FeeAmount>,
	pool_cache_by_fee: Mutex<HashMap<FeeAmount, IUniswapV3PoolInstance<RootProvider<Ethereum>>>>, /* Fee -> Pool Contract */
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Default)]
pub struct PriceAndLiquidity {
	pub price: FixedU128,
	pub liquidity: Balance,
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
		let provider = RootProvider::connect(&url).await?;

		Ok(Self {
			provider: Arc::new(provider),
			factory_address,
			usd_token,
			lookup_token,
			// NOTE: taking high tier out since this will be for pricing a stablecoin. High fees are
			// usually for highly volatile assets
			fee_tiers: vec![FeeAmount::LOW, FeeAmount::MEDIUM],
			pool_cache_by_fee: Default::default(),
		})
	}

	pub async fn get_current_price(&self) -> Result<PriceAndLiquidity> {
		#[cfg(test)]
		{
			if let Some(mock_tokens) = MOCK_PRICES.lock().get_mut(&self.lookup_token.address()) {
				if let Some(price) = mock_tokens.pop() {
					return Ok(price);
				}
			}
		}
		let (price, liquidity) = self
			.get_aggregated_twap()
			.await?
			.ok_or(anyhow!("Failed to get price, using default"))?;
		let scaled_numerator = price.adjusted_for_decimals().to_decimal() * FixedU128::accuracy();
		let float = scaled_numerator.to_u128().map_err(|_| anyhow!("Failed to convert to u128"))?;

		// Liquidity basis is computed in raw ARGON units (18 decimals). Scale to mainchain units (6
		// decimals).
		let liquidity_mainchain_units = liquidity / BigInt::from(10u128.pow(12));

		Ok(PriceAndLiquidity {
			price: FixedU128::from_inner(float),
			liquidity: Balance::try_from(liquidity_mainchain_units)
				.map_err(|e| anyhow!("Failed to convert liquidity  {:?}", e))?,
		})
	}

	async fn get_active_liquidity_reserves(&self, fee: FeeAmount) -> Result<(U256, U256)> {
		let pool = self.get_cached_pool_contract(fee).await?;
		let slot0 = pool.slot0().call().await?;

		let tick = slot0.tick;
		let liquidity_u128: u128 = pool.liquidity().call().await?;

		let spacing = fee.tick_spacing();
		let tick_lower = tick - (tick % spacing);
		let tick_upper = tick_lower + spacing;

		let sqrt_lower = U256::from(get_sqrt_ratio_at_tick(tick_lower)?);
		let sqrt_upper = U256::from(get_sqrt_ratio_at_tick(tick_upper)?);

		let token_0_amount = get_amount_0_delta(sqrt_lower, sqrt_upper, liquidity_u128, false)?;
		let token_1_amount = get_amount_1_delta(sqrt_lower, sqrt_upper, liquidity_u128, false)?;

		// Keep reserves in raw token units as returned by Uniswap math:
		// - token0 (ARGON) is 18 decimals
		// - token1 (USDC) is 6 decimals
		// Converting ARGON down to 6 decimals here can truncate small in-range amounts to 0,
		// which then makes our liquidity basis collapse right after swaps.
		let argon_raw = token_0_amount;
		let usdc_raw = token_1_amount;

		Ok((usdc_raw, argon_raw))
	}

	/// Calculate time weighted average price (twap) and liquidity for a given fee tier
	async fn get_twap_and_liquidity_basis(
		&self,
		fee: FeeAmount,
	) -> Result<(Price<Token, Token>, BigInt)> {
		let block_id = BlockId::Number(BlockNumberOrTag::Latest);
		let pool_contract = self.get_cached_pool_contract(fee).await?;

		let mut backup_second_options = vec![60 * 60, 30 * 60, 10 * 60, 5 * 60, 60];
		let mut time_window_seconds = backup_second_options.remove(0);

		// Fetch tick_cumulatives and liquidity_cumulatives
		let result = loop {
			match pool_contract.observe(vec![time_window_seconds, 0]).block(block_id).call().await {
				Ok(res) => break res,
				Err(e) => {
					let error_msg = format!("{:?}", e);
					if error_msg.contains("ZeroData") {
						return Err(anyhow!("No data for fee tier {:?}: {:?}", fee, e));
					}
					if error_msg.contains("execution reverted: OLD") {
						if backup_second_options.is_empty() {
							return Err(anyhow!(
								"All time windows exhausted for fee tier {:?}",
								fee
							));
						}
						time_window_seconds = backup_second_options.remove(0);
						trace!(fee = ?fee, new_time_window = ?time_window_seconds, "Reducing time window and retrying observe due to OLD error");
						continue;
					}
					error!(fee = ?fee, error = ?e, "Error calling observe on fee tier, returning error");
					return Err(anyhow!("Error calling observe: {:?}", e));
				},
			}
		};

		// Compute tick cumulative difference
		let tick_cumulatives = result.tickCumulatives;
		let tick_diff = tick_cumulatives[1] - tick_cumulatives[0];

		// Calculate time-weighted average tick (fixed-point division)
		let tick_twap = {
			let seconds_as_i56 = I56::try_from(time_window_seconds)?;
			(tick_diff / seconds_as_i56).to_i24()
		};

		// Convert tick to sqrtPriceX96
		let price = tick_to_price(self.lookup_token.clone(), self.usd_token.clone(), tick_twap)?;

		// Compute real-time reserves and effective liquidity in ARGON units
		let (usdc_reserve, argon_reserve) = self.get_active_liquidity_reserves(fee).await?;

		// Convert USDC reserve (token1) into ARGON units (token0) using the Price fraction.
		// `price` here is USDC per 1 ARGON (quote/base). So:
		//   argon_from_usdc_raw = usdc_raw * price.denominator / price.numerator
		// This avoids decimal rounding and preserves small-but-real in-range liquidity.
		let usdc_as_big_int = usdc_reserve.to_big_int();
		let usdc_in_argon_raw = if price.numerator != BigInt::zero() {
			U256::from_big_int((usdc_as_big_int * price.denominator) / price.numerator)
		} else {
			U256::ZERO
		};

		// Effective liquidity is the minimum of ARGON reserve and USDC (converted into ARGON units)
		let effective_liquidity_argon = argon_reserve.min(usdc_in_argon_raw);

		Ok((price, effective_liquidity_argon.to_big_int()))
	}

	/// Aggregate TWAPs across fee tiers, weighted by TWAL
	async fn get_aggregated_twap(&self) -> Result<Option<(Price<Token, Token>, BigInt)>> {
		let mut total_numerator = BigInt::zero();
		let mut total_denominator = BigInt::zero();
		let mut total_liquidity = BigInt::zero();

		for &fee in &self.fee_tiers {
			match self.get_twap_and_liquidity_basis(fee).await {
				Err(e) => {
					warn!(fee = ?fee, message = e.to_string(), "Could not get TWAP and liquidity basis for fee tier, skipping");
					continue;
				},
				Ok((price, current_liquidity)) => {
					trace!(
						fee = ?fee,
						price = %price.to_fixed(3, None),
						current_liquidity = ?current_liquidity,
						"Got TWAP and liquidity basis"
					);
					total_liquidity += current_liquidity;
					total_numerator += price.numerator * current_liquidity;
					total_denominator += price.denominator * current_liquidity;
				},
			}
		}

		if total_denominator == BigInt::zero() {
			return Ok(None);
		}

		// Combine prices into a single aggregated Price
		Ok(Some((
			Price::new(
				self.lookup_token.clone(),
				self.usd_token.clone(),
				total_denominator,
				total_numerator,
			),
			total_liquidity,
		)))
	}

	/// Get or cache the pool contract
	async fn get_cached_pool_contract(
		&self,
		fee: FeeAmount,
	) -> Result<IUniswapV3PoolInstance<RootProvider<Ethereum>>> {
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

	#[allow(dead_code)]
	const ARGON_ADDRESS: &str = "0x6A9143639D8b70D50b031fFaD55d4CC65EA55155";
	#[allow(dead_code)]
	const ARGONOT_ADDRESS: &str = "0x64cbd3aa07d427e385cb55330406508718e55f01";

	#[tokio::test]
	#[ignore] // only activate when turned on
	async fn test_infura_lookup() {
		dotenv::dotenv().ok();
		dotenv::from_filename("oracle/.env").ok();
		let _ = env_logger::try_init();
		let Ok(project_id) = env::var("INFURA_PROJECT_ID") else {
			warn!("INFURA_PROJECT_ID not set, skipping test");
			return;
		};
		if project_id == "test" {
			warn!("INFURA_PROJECT_ID is set to 'test', skipping test");
			return;
		}

		const LOOKUP_TOKEN_ADDRESS: &str = ARGONOT_ADDRESS;

		let oracle = UniswapOracle::new(
			project_id,
			token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			token!(ChainId::MAINNET as u64, LOOKUP_TOKEN_ADDRESS, 18, "ARGON"),
		)
		.await
		.expect("Failed to create oracle");
		let price = oracle
			.get_current_price()
			.await
			.inspect_err(|e| {
				error!("Error getting price: {:?}", e);
			})
			.expect("Failed to get price");
		println!("Price: {:?}", price);
		if LOOKUP_TOKEN_ADDRESS == ARGONOT_ADDRESS {
			// ARGONOT is a floating coin
			assert!(price.price.to_float() < 0.1);
		} else {
			// should be around 1.0
			assert!((price.price.to_float() - 1.0).abs() < 0.1);
			assert!(price.liquidity > 1000);
		}
	}
}
