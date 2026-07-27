use alloy_contract::Error as ContractError;
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_primitives::{address, aliases::I56};
use alloy_provider::{network::Ethereum, RootProvider};
use anyhow::{anyhow, Context, Result};
use argon_primitives::{
	prelude::{frame_support::pallet_prelude::Zero, sp_arithmetic::FixedPointNumber},
	Balance,
};
use polkadot_sdk::*;
use sdk_core::prelude::*;
use sp_runtime::FixedU128;
use std::{collections::HashMap, env, fmt, sync::Arc};
use tokio::sync::Mutex;
use tracing::{error, trace, warn};
use uniswap_lens::bindings::iuniswapv3pool::IUniswapV3Pool::IUniswapV3PoolInstance;
use uniswap_v3_sdk::{entities::TickIndex, prelude::*};

pub fn ethereum_rpc_urls_from_env() -> Result<Vec<String>> {
	let urls = env::var("ETHEREUM_RPC_URLS")
		.context("ETHEREUM_RPC_URLS must be set")?
		.split(',')
		.map(str::trim)
		.filter(|url| !url.is_empty())
		.map(str::to_owned)
		.collect::<Vec<_>>();
	if urls.is_empty() {
		return Err(anyhow!("ETHEREUM_RPC_URLS must contain at least one URL"));
	}
	Ok(urls)
}
pub const USDC_ADDRESS: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
pub(crate) const USDC_ADDRESS_SEPOLIA: Address =
	address!("1c7D4B196Cb0C7B01d743Fbc6116a902379C7238");
pub const SEPOLIA_FACTORY_ADDRESS: Address = address!("0227628f3F023bb0B980b67D528571c95c6DaC1c");
type PoolContract = IUniswapV3PoolInstance<RootProvider<Ethereum>>;

struct EthereumRpcProvider {
	provider: Arc<RootProvider<Ethereum>>,
	pool_cache_by_fee: Mutex<HashMap<FeeAmount, PoolContract>>,
}

#[cfg(test)]
lazy_static::lazy_static! {
	pub static ref MOCK_PRICES: Arc<parking_lot::Mutex<HashMap<Address, Vec<PriceAndLiquidity>>>> = Default::default();
}

#[cfg(test)]
pub(crate) fn use_mock_uniswap_prices(token_address: Address, mut prices: Vec<PriceAndLiquidity>) {
	MOCK_PRICES.lock().entry(token_address).or_default().append(&mut prices)
}

pub struct UniswapOracle {
	providers: Vec<EthereumRpcProvider>,
	factory_address: Address,
	usd_token: Token,
	lookup_token: Token,
	fee_tiers: Vec<FeeAmount>,
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Default)]
pub struct PriceAndLiquidity {
	pub price: FixedU128,
	pub liquidity: Balance,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum UniswapOracleError {
	NoPoolData,
	NoActiveLiquidity,
}

impl fmt::Display for UniswapOracleError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NoPoolData => write!(f, "No pool data across available fee tiers"),
			Self::NoActiveLiquidity => write!(f, "No active liquidity across available fee tiers"),
		}
	}
}

impl std::error::Error for UniswapOracleError {}

impl UniswapOracle {
	pub async fn new(rpc_urls: Vec<String>, usd_token: Token, lookup_token: Token) -> Result<Self> {
		if rpc_urls.is_empty() {
			return Err(anyhow!("At least one Ethereum RPC URL is required"));
		}

		let factory_address = if usd_token.chain_id() == ChainId::SEPOLIA as u64 {
			SEPOLIA_FACTORY_ADDRESS
		} else {
			FACTORY_ADDRESS
		};
		let mut providers = Vec::with_capacity(rpc_urls.len());
		for url in rpc_urls {
			providers.push(EthereumRpcProvider {
				provider: Arc::new(RootProvider::connect(&url).await?),
				pool_cache_by_fee: Default::default(),
			});
		}

		Ok(Self {
			providers,
			factory_address,
			usd_token,
			lookup_token,
			// NOTE: taking high tier out since this will be for pricing a stablecoin. High fees are
			// usually for highly volatile assets
			fee_tiers: vec![FeeAmount::LOW, FeeAmount::MEDIUM],
		})
	}

	pub async fn get_current_price(&self) -> Result<PriceAndLiquidity> {
		#[cfg(test)]
		{
			if let Some(mock_tokens) = MOCK_PRICES.lock().get_mut(&self.lookup_token.address()) &&
				let Some(price) = mock_tokens.pop()
			{
				return Ok(price);
			}
		}
		let (price, liquidity) = self
			.get_aggregated_twap()
			.await?
			.ok_or_else(|| anyhow!("Failed to get price, using default"))?;
		let scaled_numerator = price.adjusted_for_decimals().to_decimal() * FixedU128::accuracy();
		let float = scaled_numerator.to_u128().map_err(|_| anyhow!("Failed to convert to u128"))?;

		Ok(PriceAndLiquidity {
			price: FixedU128::from_inner(float),
			liquidity: Balance::try_from(liquidity)
				.map_err(|e| anyhow!("Failed to convert liquidity  {e:?}"))?,
		})
	}

	/// Calculate time-weighted average price and liquidity for a given fee tier.
	async fn get_twap_and_liquidity_basis(
		&self,
		provider: &EthereumRpcProvider,
		fee: FeeAmount,
	) -> Result<(Price<Token, Token>, BigInt)> {
		let block_id = BlockId::Number(BlockNumberOrTag::Latest);
		let pool_contract = self.get_cached_pool_contract(provider, fee).await?;

		let mut backup_second_options = vec![60 * 60, 30 * 60, 10 * 60, 5 * 60, 60];
		let mut time_window_seconds = backup_second_options.remove(0);

		// Fetch tick_cumulatives and liquidity_cumulatives
		let result = loop {
			match pool_contract.observe(vec![time_window_seconds, 0]).block(block_id).call().await {
				Ok(res) => break res,
				Err(ContractError::ZeroData(..)) =>
					return Err(UniswapOracleError::NoPoolData.into()),
				Err(e) => {
					let error_msg = format!("{e:?}");
					let is_old_observation = error_msg.contains("execution reverted: OLD");
					if is_old_observation && !backup_second_options.is_empty() {
						time_window_seconds = backup_second_options.remove(0);
						trace!(fee = ?fee, new_time_window = ?time_window_seconds, error = ?e, "Reducing time window and retrying observe");
						continue;
					}
					if is_old_observation {
						return Err(anyhow!("All time windows exhausted for fee tier {fee:?}"));
					}
					error!(fee = ?fee, error = ?e, "Error calling observe on fee tier, returning error");
					return Err(e).context("Error calling observe");
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

		// The same observations contain cumulative seconds per liquidity. Inverting the change
		// across the selected window produces Uniswap's harmonic mean liquidity (TWAL).
		let liquidity_diff = result.secondsPerLiquidityCumulativeX128s[1] -
			result.secondsPerLiquidityCumulativeX128s[0];
		let average_liquidity = if liquidity_diff.is_zero() {
			BigInt::zero()
		} else {
			let seconds_between_x128 = BigInt::from(time_window_seconds) << 128;
			seconds_between_x128 / liquidity_diff.to_big_int()
		};

		// Uniswap liquidity for an 18-decimal token paired with 6-decimal USDC has 12 decimal
		// places. Mainchain balances have 6, so remove the remaining 6 decimal places.
		let liquidity_mainchain_units = average_liquidity / BigInt::from(10u128.pow(6));

		Ok((price, liquidity_mainchain_units))
	}

	/// Aggregate TWAPs across fee tiers, weighted by TWAL
	async fn get_aggregated_twap(&self) -> Result<Option<(Price<Token, Token>, BigInt)>> {
		let mut total_numerator = BigInt::zero();
		let mut total_denominator = BigInt::zero();
		let mut total_liquidity = BigInt::zero();
		let mut no_pool_data_fee_tiers = 0usize;
		let mut successful_fee_tiers = 0usize;
		let mut had_other_errors = false;
		let mut last_error = None;

		for &fee in &self.fee_tiers {
			let mut fee_result = None;
			let mut fee_error = None;
			for provider in &self.providers {
				match self.get_twap_and_liquidity_basis(provider, fee).await {
					Ok(result) => {
						fee_result = Some(result);
						break;
					},
					Err(e) if has_internal_rpc_error(&e) => {
						warn!(fee = ?fee, "Ethereum RPC returned an internal error; trying fallback provider");
						fee_error = Some(e);
					},
					Err(e) => {
						fee_error = Some(e);
						break;
					},
				}
			}

			if let Some((price, current_liquidity)) = fee_result {
				successful_fee_tiers += 1;
				trace!(
					fee = ?fee,
					price = %price.to_fixed(3, None),
					current_liquidity = ?current_liquidity,
					"Got TWAP and liquidity basis"
				);
				total_liquidity += current_liquidity;
				total_numerator += price.numerator * current_liquidity;
				total_denominator += price.denominator * current_liquidity;
				continue;
			}

			let Some(e) = fee_error else {
				continue;
			};
			let oracle_error =
				e.chain().find_map(|cause| cause.downcast_ref::<UniswapOracleError>());
			if matches!(oracle_error, Some(UniswapOracleError::NoPoolData)) {
				no_pool_data_fee_tiers += 1;
				continue;
			}
			had_other_errors = true;
			warn!(fee = ?fee, message = e.to_string(), "Could not get TWAP and liquidity basis for fee tier, skipping");
			if last_error.is_none() {
				last_error = Some(e);
			}
		}

		if total_denominator == BigInt::zero() {
			if no_pool_data_fee_tiers == self.fee_tiers.len() {
				return Err(UniswapOracleError::NoPoolData.into());
			}
			if !had_other_errors && successful_fee_tiers > 0 && total_liquidity == BigInt::zero() {
				return Err(UniswapOracleError::NoActiveLiquidity.into());
			}
			if let Some(error) = last_error {
				return Err(error);
			}
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

	async fn get_cached_pool_contract(
		&self,
		provider: &EthereumRpcProvider,
		fee: FeeAmount,
	) -> Result<PoolContract> {
		let mut cache = provider.pool_cache_by_fee.lock().await;
		if let Some(pool) = cache.get(&fee) {
			return Ok(pool.clone());
		}

		let pool_address = compute_pool_address(
			self.factory_address,
			self.usd_token.address(),
			self.lookup_token.address(),
			fee,
			None,
			None,
		);
		let pool_contract =
			IUniswapV3PoolInstance::new(pool_address, provider.provider.as_ref().clone());
		cache.insert(fee, pool_contract.clone());

		Ok(pool_contract)
	}
}

fn is_internal_rpc_error(error: &ContractError) -> bool {
	matches!(
		error,
		ContractError::TransportError(error)
			if error.as_error_resp().is_some_and(|response| {
				response.code == -32603 && response.data.is_none()
			})
	)
}

fn has_internal_rpc_error(error: &anyhow::Error) -> bool {
	error
		.chain()
		.any(|cause| cause.downcast_ref::<ContractError>().is_some_and(is_internal_rpc_error))
}

#[cfg(test)]
mod test {
	use super::*;

	use alloy_primitives::{aliases::U160, Bytes};
	use alloy_provider::ProviderBuilder;
	use alloy_sol_types::SolCall;
	use alloy_transport::mock::Asserter;
	use tracing::warn;
	use uniswap_lens::bindings::iuniswapv3pool::IUniswapV3Pool::{observeCall, observeReturn};
	use uniswap_sdk_core::token;

	#[allow(dead_code)]
	const ARGON_ADDRESS: &str = "0xf3D6b714dc93bc6C44bc766cc92F4A0D99344932";
	#[allow(dead_code)]
	const ARGONOT_ADDRESS: &str = "0x6B93a120829558C18f8CD54a96E8024EF973cE52";

	#[tokio::test]
	async fn uses_observation_history_for_time_weighted_average_liquidity() {
		let asserter = Asserter::new();
		let provider = EthereumRpcProvider {
			provider: Arc::new(ProviderBuilder::default().connect_mocked_client(asserter.clone())),
			pool_cache_by_fee: Default::default(),
		};
		let oracle = UniswapOracle {
			providers: vec![],
			factory_address: FACTORY_ADDRESS,
			usd_token: token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			lookup_token: token!(ChainId::MAINNET as u64, ARGON_ADDRESS, 18, "ARGON"),
			fee_tiers: vec![FeeAmount::LOW],
		};

		asserter.push_success(&Bytes::from(observeCall::abi_encode_returns(&observeReturn {
			tickCumulatives: vec![I56::ZERO, I56::ZERO],
			secondsPerLiquidityCumulativeX128s: vec![U160::ZERO, U160::from(3_600u128 << 64)],
		})));

		let (_, time_weighted_average_liquidity) =
			oracle.get_twap_and_liquidity_basis(&provider, FeeAmount::LOW).await.unwrap();

		assert_eq!(time_weighted_average_liquidity, BigInt::from((1u128 << 64) / 1_000_000));
	}

	#[tokio::test]
	async fn uses_fallback_provider_when_any_fee_tier_returns_internal_error() {
		let primary = Asserter::new();
		let fallback = Asserter::new();
		let oracle = UniswapOracle {
			providers: vec![
				EthereumRpcProvider {
					provider: Arc::new(
						ProviderBuilder::default().connect_mocked_client(primary.clone()),
					),
					pool_cache_by_fee: Default::default(),
				},
				EthereumRpcProvider {
					provider: Arc::new(
						ProviderBuilder::default().connect_mocked_client(fallback.clone()),
					),
					pool_cache_by_fee: Default::default(),
				},
			],
			factory_address: FACTORY_ADDRESS,
			usd_token: token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			lookup_token: token!(ChainId::MAINNET as u64, ARGON_ADDRESS, 18, "ARGON"),
			fee_tiers: vec![FeeAmount::LOW, FeeAmount::MEDIUM],
		};

		primary.push_success(&Bytes::from(observeCall::abi_encode_returns(&observeReturn {
			tickCumulatives: vec![I56::ZERO, I56::ZERO],
			secondsPerLiquidityCumulativeX128s: vec![U160::ZERO, U160::from(3_600u128 << 64)],
		})));
		primary.push_failure_msg("Internal error");
		fallback.push_success(&Bytes::from(observeCall::abi_encode_returns(&observeReturn {
			tickCumulatives: vec![I56::ZERO, I56::try_from(1_800).unwrap()],
			secondsPerLiquidityCumulativeX128s: vec![U160::ZERO, U160::from(3_600u128 << 64)],
		})));

		let result = oracle
			.get_current_price()
			.await
			.expect("the fallback provider should return the price");

		assert_eq!(result.liquidity, 2 * ((1u128 << 64) / 1_000_000));
	}

	#[tokio::test]
	#[ignore]
	async fn test_rpc_fallback_reaches_live_pool() {
		dotenv::dotenv().ok();
		dotenv::from_filename("oracle/.env").ok();
		let _ = env_logger::try_init();
		let Ok(mut rpc_urls) = ethereum_rpc_urls_from_env() else {
			warn!("ETHEREUM_RPC_URLS not set, skipping test");
			return;
		};
		let Some(fallback_url) = rpc_urls.pop() else {
			warn!("ETHEREUM_RPC_URLS contains no URLs, skipping test");
			return;
		};

		let primary = Asserter::new();
		primary.push_failure_msg("Internal error");
		let oracle = UniswapOracle {
			providers: vec![
				EthereumRpcProvider {
					provider: Arc::new(ProviderBuilder::default().connect_mocked_client(primary)),
					pool_cache_by_fee: Default::default(),
				},
				EthereumRpcProvider {
					provider: Arc::new(
						RootProvider::connect(&fallback_url)
							.await
							.expect("Failed to connect to fallback provider"),
					),
					pool_cache_by_fee: Default::default(),
				},
			],
			factory_address: FACTORY_ADDRESS,
			usd_token: token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			lookup_token: token!(ChainId::MAINNET as u64, ARGONOT_ADDRESS, 18, "ARGONOT"),
			fee_tiers: vec![FeeAmount::LOW, FeeAmount::MEDIUM],
		};

		if let Err(error) = oracle.get_current_price().await {
			let oracle_error =
				error.chain().find_map(|cause| cause.downcast_ref::<UniswapOracleError>());
			assert_eq!(
				oracle_error,
				Some(&UniswapOracleError::NoActiveLiquidity),
				"the configured fallback RPC failed before returning live pool state: {error:?}"
			);
		}
	}

	#[tokio::test]
	#[ignore] // only activate when turned on
	async fn test_ethereum_rpc_lookup() {
		dotenv::dotenv().ok();
		dotenv::from_filename("oracle/.env").ok();
		let _ = env_logger::try_init();
		let Ok(rpc_urls) = ethereum_rpc_urls_from_env() else {
			warn!("ETHEREUM_RPC_URLS not set, skipping test");
			return;
		};

		for (address, symbol) in [(ARGON_ADDRESS, "ARGON"), (ARGONOT_ADDRESS, "ARGONOT")] {
			let oracle = UniswapOracle::new(
				rpc_urls.clone(),
				token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
				token!(ChainId::MAINNET as u64, address, 18, symbol),
			)
			.await
			.expect("Failed to create oracle");
			let price = oracle
				.get_current_price()
				.await
				.inspect_err(|e| {
					error!(symbol, "Error getting price: {:?}", e);
				})
				.expect("Failed to get price");
			println!("{symbol}: {price:?}");
			assert!(!price.price.is_zero());
		}
	}
}
