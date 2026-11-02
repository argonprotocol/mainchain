use alloy_contract::Error as ContractError;
use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_primitives::{address, aliases::I56, B256};
use alloy_provider::{network::Ethereum, Provider, RootProvider};
use anyhow::{anyhow, Context, Result};
use argon_primitives::{
	prelude::{frame_support::pallet_prelude::Zero, sp_arithmetic::FixedPointNumber},
	Balance,
};
use polkadot_sdk::*;
use sdk_core::prelude::*;
use sp_core::U256;
use sp_runtime::FixedU128;
#[cfg(test)]
use std::{collections::HashMap, sync::Arc};
use std::{env, fmt};
use tracing::{error, trace, warn};
use uniswap_lens::bindings::iuniswapv3pool::IUniswapV3Pool::IUniswapV3PoolInstance;
use uniswap_v3_sdk::{entities::TickIndex, prelude::*};
use uniswap_v4_sdk::extensions::PoolManagerLens;

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
alloy_sol_types::sol! {
	/// Uniswap v3-core Solidity 0.8 oracle history error.
	error OLD();
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
	providers: Vec<RootProvider<Ethereum>>,
	usd_token: Token,
	lookup_token: Token,
	oracle_addresses: Vec<Address>,
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
			Self::NoPoolData => write!(f, "No pool data across configured Uniswap sources"),
			Self::NoActiveLiquidity =>
				write!(f, "No active liquidity across configured Uniswap sources"),
		}
	}
}

impl std::error::Error for UniswapOracleError {}

impl UniswapOracle {
	/// Read a single chain-selected v4 pool through the SDK's PoolManager storage lens.
	pub async fn get_pool_sqrt_price_x96(
		&self,
		chain_id: u64,
		pool_manager: Address,
		pool_id: B256,
	) -> Result<U256> {
		let mut last_error = anyhow!("No Ethereum provider configured");
		for provider in &self.providers {
			let observation = async {
				if provider.get_chain_id().await? != chain_id {
					return Err(anyhow!(
						"Ethereum provider does not match the stabilizer pool chain"
					));
				}

				let lens = PoolManagerLens::new(pool_manager, provider);
				let (sqrt_price_x96, ..) = lens.get_slot0(pool_id, None).await?;
				if sqrt_price_x96.is_zero() {
					return Err(anyhow!("Stabilizer pool is not initialized"));
				}

				Ok(U256::from_big_endian(&sqrt_price_x96.to_be_bytes::<20>()))
			}
			.await;
			match observation {
				Ok(price) => return Ok(price),
				Err(error) => last_error = error,
			}
		}
		Err(last_error)
	}

	pub async fn new(rpc_urls: Vec<String>, usd_token: Token, lookup_token: Token) -> Result<Self> {
		if rpc_urls.is_empty() {
			return Err(anyhow!("At least one Ethereum RPC URL is required"));
		}

		let mut providers = Vec::with_capacity(rpc_urls.len());
		for url in rpc_urls {
			providers.push(RootProvider::connect(&url).await?);
		}

		Ok(Self { providers, usd_token, lookup_token, oracle_addresses: vec![] })
	}

	/// Select a nonempty list of v3 pool or v4 hook addresses to observe together.
	/// Validate every source's token pair before replacing the current selection.
	pub async fn set_oracle_addresses(&mut self, addresses: &str) -> Result<()> {
		let mut oracle_addresses = Vec::new();
		for value in addresses.split(',').map(str::trim).filter(|value| !value.is_empty()) {
			let address =
				value.parse::<Address>().context("Invalid Uniswap oracle source address")?;
			if address.is_zero() || oracle_addresses.contains(&address) {
				return Err(anyhow!("Uniswap oracle source addresses must be nonzero and unique"));
			}

			let mut validated = false;
			for (index, provider) in self.providers.iter().enumerate() {
				let contract = IUniswapV3PoolInstance::new(address, provider.clone());
				let tokens = async {
					Ok::<_, ContractError>((
						contract.token0().call().await?,
						contract.token1().call().await?,
					))
				}
				.await;
				match tokens {
					Ok((token0, token1)) => {
						let mut expected = [self.usd_token.address(), self.lookup_token.address()];
						expected.sort();
						if [token0, token1] != expected {
							return Err(anyhow!(
								"Uniswap oracle source {address} has the wrong token pair"
							));
						}
						validated = true;
						break;
					},
					Err(error)
						if is_internal_rpc_error(&error) && index + 1 < self.providers.len() =>
						continue,
					Err(error) =>
						return Err(error).context("Validating Uniswap oracle source tokens"),
				}
			}
			if !validated {
				return Err(anyhow!(
					"No Ethereum RPC provider available to validate Uniswap oracle source"
				));
			}
			oracle_addresses.push(address);
		}
		if oracle_addresses.is_empty() {
			return Err(anyhow!("At least one Uniswap oracle source address is required"));
		}
		self.oracle_addresses = oracle_addresses;
		Ok(())
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
			.ok_or_else(|| anyhow!("No price from configured Uniswap sources"))?;
		let scaled_numerator = price.adjusted_for_decimals().to_decimal() * FixedU128::accuracy();
		let float = scaled_numerator.to_u128().map_err(|_| anyhow!("Failed to convert to u128"))?;

		Ok(PriceAndLiquidity {
			price: FixedU128::from_inner(float),
			liquidity: Balance::try_from(liquidity)
				.map_err(|e| anyhow!("Failed to convert liquidity  {e:?}"))?,
		})
	}

	/// Calculate time-weighted average price and liquidity from a v3 pool or v4 hook.
	async fn get_twap_and_liquidity_basis(
		&self,
		provider: &RootProvider<Ethereum>,
		pool_address: Address,
	) -> Result<(Price<Token, Token>, BigInt)> {
		let block_id = BlockId::Number(BlockNumberOrTag::Latest);
		let pool_contract = IUniswapV3PoolInstance::new(pool_address, provider.clone());

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
					let is_old_observation = e.as_decoded_error::<OLD>().is_some() ||
						error_msg.contains("execution reverted: OLD") ||
						e.as_decoded_error::<alloy_sol_types::Revert>()
							.is_some_and(|revert| revert.reason == "OLD");
					if is_old_observation && !backup_second_options.is_empty() {
						time_window_seconds = backup_second_options.remove(0);
						trace!(pool = %pool_address, new_time_window = ?time_window_seconds, error = ?e, "Reducing time window and retrying observe");
						continue;
					}
					if is_old_observation {
						return Err(anyhow!("All time windows exhausted for pool {pool_address}"));
					}
					error!(pool = %pool_address, error = ?e, "Error calling observe on pool, returning error");
					return Err(e).context("Error calling observe");
				},
			}
		};

		// Compute tick cumulative difference
		if result.tickCumulatives.len() != 2 || result.secondsPerLiquidityCumulativeX128s.len() != 2
		{
			return Err(anyhow!("Pool {pool_address} returned invalid observation lengths"));
		}
		let tick_cumulatives = result.tickCumulatives;
		let tick_diff = tick_cumulatives[1].wrapping_sub(tick_cumulatives[0]);

		// Calculate time-weighted average tick (fixed-point division)
		let tick_twap = {
			let seconds_as_i56 = I56::try_from(time_window_seconds)?;
			let mut mean_tick = tick_diff / seconds_as_i56;
			// Uniswap ticks round toward negative infinity, including negative fractional ticks.
			if tick_diff < I56::ZERO && tick_diff % seconds_as_i56 != I56::ZERO {
				mean_tick -= I56::try_from(1)?;
			}
			mean_tick.to_i24()
		};

		// Convert tick to sqrtPriceX96
		let price = tick_to_price(self.lookup_token.clone(), self.usd_token.clone(), tick_twap)?;

		// The same observations contain cumulative seconds per liquidity. Inverting the change
		// across the selected window produces Uniswap's harmonic mean liquidity (TWAL).
		let liquidity_diff = result.secondsPerLiquidityCumulativeX128s[1]
			.wrapping_sub(result.secondsPerLiquidityCumulativeX128s[0]);
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

	/// Aggregate TWAPs across the selected v3 pools and v4 hooks, weighted by TWAL.
	async fn get_aggregated_twap(&self) -> Result<Option<(Price<Token, Token>, BigInt)>> {
		if self.oracle_addresses.is_empty() {
			return Err(anyhow!("At least one Uniswap oracle source address is required"));
		}

		// Normalize every raw token price to the same denominator before weighting. SDK prices
		// otherwise have a tick-dependent denominator when the lookup token sorts after USDC.
		let price_scale = BigInt::from(1u128) << 128;
		let mut weighted_price_numerator = BigInt::zero();
		let mut total_liquidity = BigInt::zero();
		let mut no_pool_data_sources = 0usize;
		let mut successful_sources = 0usize;
		let mut had_other_errors = false;
		let mut last_error = None;

		for &pool_address in &self.oracle_addresses {
			let mut pool_result = None;
			let mut pool_error = None;
			for provider in &self.providers {
				match self.get_twap_and_liquidity_basis(provider, pool_address).await {
					Ok(result) => {
						pool_result = Some(result);
						break;
					},
					Err(e) if has_internal_rpc_error(&e) => {
						warn!(pool = %pool_address, "Ethereum RPC returned an internal error; trying fallback provider");
						pool_error = Some(e);
					},
					Err(e) => {
						pool_error = Some(e);
						break;
					},
				}
			}

			if let Some((price, current_liquidity)) = pool_result {
				successful_sources += 1;
				trace!(
					pool = %pool_address,
					price = %price.to_fixed(3, None),
					current_liquidity = ?current_liquidity,
					"Got TWAP and liquidity basis"
				);
				total_liquidity += current_liquidity;
				let scaled_price = price.numerator * price_scale / price.denominator;
				weighted_price_numerator += scaled_price * current_liquidity;
				continue;
			}

			let Some(e) = pool_error else {
				continue;
			};
			let oracle_error =
				e.chain().find_map(|cause| cause.downcast_ref::<UniswapOracleError>());
			if matches!(oracle_error, Some(UniswapOracleError::NoPoolData)) {
				no_pool_data_sources += 1;
				continue;
			}
			had_other_errors = true;
			warn!(pool = %pool_address, message = e.to_string(), "Could not get TWAP and liquidity basis for pool, skipping");
			if last_error.is_none() {
				last_error = Some(e);
			}
		}

		if total_liquidity == BigInt::zero() {
			if no_pool_data_sources == self.oracle_addresses.len() {
				return Err(UniswapOracleError::NoPoolData.into());
			}
			if !had_other_errors && successful_sources > 0 {
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
				total_liquidity * price_scale,
				weighted_price_numerator,
			),
			total_liquidity,
		)))
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

	use alloy_primitives::{aliases::U160, Bytes, U64};
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
	async fn configured_v4_pool_price_skips_providers_on_another_chain() {
		let wrong_chain = Asserter::new();
		let matching_chain = Asserter::new();
		let oracle = UniswapOracle {
			providers: vec![
				ProviderBuilder::default().connect_mocked_client(wrong_chain.clone()),
				ProviderBuilder::default().connect_mocked_client(matching_chain.clone()),
			],
			usd_token: token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			lookup_token: token!(ChainId::MAINNET as u64, ARGON_ADDRESS, 18, "ARGON"),
			oracle_addresses: vec![],
		};
		wrong_chain.push_success(&U64::from(11155111));
		matching_chain.push_success(&U64::from(1));

		// Slot0 also contains fee/tick bits; only the low 160 bits are the sqrt price.
		let mut slot0 = [0u8; 32];
		slot0[0] = 0x42;
		slot0[12..].copy_from_slice(&U160::from(1u128 << 96).to_be_bytes::<20>());
		matching_chain.push_success(&Bytes::from(slot0.to_vec()));
		let price = oracle
			.get_pool_sqrt_price_x96(1, Address::repeat_byte(1), B256::repeat_byte(2))
			.await
			.unwrap();
		assert_eq!(price, U256::one() << 96);

		// A configured but uninitialized pool provides no usable observation.
		wrong_chain.push_success(&U64::from(11155111));
		matching_chain.push_success(&U64::from(1));
		matching_chain.push_success(&Bytes::from(vec![0; 32]));
		assert!(oracle
			.get_pool_sqrt_price_x96(1, Address::repeat_byte(1), B256::repeat_byte(2),)
			.await
			.is_err());
	}

	#[tokio::test]
	async fn uses_observation_history_for_time_weighted_average_liquidity() {
		let asserter = Asserter::new();
		let provider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
		let oracle = UniswapOracle {
			providers: vec![],
			usd_token: token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			lookup_token: token!(ChainId::MAINNET as u64, ARGON_ADDRESS, 18, "ARGON"),
			oracle_addresses: vec![],
		};

		asserter.push_success(&Bytes::from(observeCall::abi_encode_returns(&observeReturn {
			tickCumulatives: vec![I56::ZERO, I56::ZERO],
			secondsPerLiquidityCumulativeX128s: vec![U160::ZERO, U160::from(3_600u128 << 64)],
		})));

		let (_, time_weighted_average_liquidity) = oracle
			.get_twap_and_liquidity_basis(&provider, Address::repeat_byte(1))
			.await
			.unwrap();

		assert_eq!(time_weighted_average_liquidity, BigInt::from((1u128 << 64) / 1_000_000));
	}

	#[tokio::test]
	async fn retries_a_configured_source_through_another_rpc_provider() {
		let primary = Asserter::new();
		let fallback = Asserter::new();
		let oracle = UniswapOracle {
			providers: vec![
				ProviderBuilder::default().connect_mocked_client(primary.clone()),
				ProviderBuilder::default().connect_mocked_client(fallback.clone()),
			],
			usd_token: token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			lookup_token: token!(ChainId::MAINNET as u64, ARGON_ADDRESS, 18, "ARGON"),
			oracle_addresses: vec![Address::repeat_byte(1), Address::repeat_byte(5)],
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
	async fn does_not_read_unlisted_sources_when_selected_history_is_unavailable() {
		use alloy_sol_types::SolError;
		let asserter = Asserter::new();
		let mut oracle = UniswapOracle::new(
			vec!["http://localhost:8545".into()],
			token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			token!(ChainId::MAINNET as u64, ARGON_ADDRESS, 18, "ARGON"),
		)
		.await
		.unwrap();
		oracle.providers = vec![ProviderBuilder::default().connect_mocked_client(asserter.clone())];
		let hook = Address::repeat_byte(5);
		oracle.oracle_addresses = vec![hook];
		for _ in 0..5 {
			asserter.push_failure(
				serde_json::from_value(serde_json::json!({
					"code": 3, "message": "execution reverted", "data": Bytes::from(OLD {}.abi_encode()),
				}))
				.unwrap(),
			);
		}
		// A usable response for another source must not mask failure of the selected hook.
		asserter.push_success(&Bytes::from(observeCall::abi_encode_returns(&observeReturn {
			tickCumulatives: vec![I56::ZERO, I56::ZERO],
			secondsPerLiquidityCumulativeX128s: vec![U160::ZERO, U160::from(3_600u128 << 64)],
		})));
		let error = oracle.get_current_price().await.unwrap_err();
		assert!(error
			.to_string()
			.contains(&format!("All time windows exhausted for pool {hook}")));
		assert_eq!(asserter.read_q().len(), 1);
	}

	#[tokio::test]
	async fn rejects_sources_for_another_pair_and_invalid_addresses() {
		use uniswap_lens::bindings::iuniswapv3pool::IUniswapV3Pool::{token0Call, token1Call};
		let asserter = Asserter::new();
		let mut oracle = UniswapOracle::new(
			vec!["http://localhost:8545".into()],
			token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			token!(ChainId::MAINNET as u64, ARGON_ADDRESS, 18, "ARGON"),
		)
		.await
		.unwrap();
		oracle.providers = vec![ProviderBuilder::default().connect_mocked_client(asserter.clone())];
		let error = oracle.get_current_price().await.unwrap_err();
		assert!(error.to_string().contains("At least one Uniswap oracle source address"));
		for empty in ["", " ", ", ,"] {
			assert!(oracle.set_oracle_addresses(empty).await.is_err());
		}
		assert!(oracle.set_oracle_addresses("not an address").await.is_err());
		assert!(oracle.set_oracle_addresses(&Address::ZERO.to_string()).await.is_err());
		asserter.push_success(&Bytes::from(token0Call::abi_encode_returns(&USDC_ADDRESS)));
		asserter
			.push_success(&Bytes::from(token1Call::abi_encode_returns(&Address::repeat_byte(7))));
		assert!(oracle
			.set_oracle_addresses(&Address::repeat_byte(5).to_string())
			.await
			.unwrap_err()
			.to_string()
			.contains("wrong token pair"));
		assert!(oracle.oracle_addresses.is_empty());

		let hook = Address::repeat_byte(5);
		oracle.oracle_addresses = vec![hook];
		asserter.push_success(&Bytes::from(token0Call::abi_encode_returns(&USDC_ADDRESS)));
		asserter.push_success(&Bytes::from(token1Call::abi_encode_returns(
			&oracle.lookup_token.address(),
		)));
		assert!(oracle.set_oracle_addresses(&format!("{hook},{hook}")).await.is_err());
		assert_eq!(oracle.oracle_addresses, vec![hook]);
		assert!(oracle.set_oracle_addresses("").await.is_err());
		assert_eq!(oracle.oracle_addresses, vec![hook]);
	}

	#[tokio::test]
	async fn reads_v4_only_history_after_old_revert_and_rounds_negative_ticks_down() {
		use alloy_sol_types::SolError;
		let asserter = Asserter::new();
		let mut oracle = UniswapOracle::new(
			vec!["http://localhost:8545".into()],
			token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			token!(ChainId::MAINNET as u64, ARGON_ADDRESS, 18, "ARGON"),
		)
		.await
		.unwrap();
		oracle.providers = vec![ProviderBuilder::default().connect_mocked_client(asserter.clone())];
		oracle.oracle_addresses = vec![Address::repeat_byte(5)];
		asserter.push_failure(
			serde_json::from_value(serde_json::json!({
				"code": 3, "message": "execution reverted", "data": Bytes::from(OLD {}.abi_encode()),
			}))
			.unwrap(),
		);
		asserter.push_success(&Bytes::from(observeCall::abi_encode_returns(&observeReturn {
			tickCumulatives: vec![I56::ZERO, I56::try_from(-1_801).unwrap()],
			secondsPerLiquidityCumulativeX128s: vec![U160::ZERO, U160::from(1_800u128 << 64)],
		})));
		let (price, liquidity) = oracle.get_aggregated_twap().await.unwrap().unwrap();
		let expected =
			tick_to_price(oracle.lookup_token.clone(), oracle.usd_token.clone(), (-2i32).to_i24())
				.unwrap();
		assert_eq!(price.to_fixed(12, None), expected.to_fixed(12, None));
		assert_eq!(liquidity, BigInt::from((1u128 << 64) / 1_000_000));
	}

	#[tokio::test]
	async fn reads_wrapped_cumulative_counters_and_rejects_malformed_history() {
		let asserter = Asserter::new();
		let provider = ProviderBuilder::default().connect_mocked_client(asserter.clone());
		let oracle = UniswapOracle::new(
			vec!["http://localhost:8545".into()],
			token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			token!(ChainId::MAINNET as u64, ARGON_ADDRESS, 18, "ARGON"),
		)
		.await
		.unwrap();
		let hook = Address::repeat_byte(5);
		asserter.push_success(&Bytes::from(observeCall::abi_encode_returns(&observeReturn {
			tickCumulatives: vec![I56::MAX, I56::MIN + I56::try_from(3_599).unwrap()],
			secondsPerLiquidityCumulativeX128s: vec![
				U160::ZERO.wrapping_sub(U160::from(3_600u128 << 64)),
				U160::ZERO,
			],
		})));
		let (price, liquidity) =
			oracle.get_twap_and_liquidity_basis(&provider, hook).await.unwrap();
		let expected =
			tick_to_price(oracle.lookup_token.clone(), oracle.usd_token.clone(), 1i32.to_i24())
				.unwrap();
		assert_eq!(price.numerator * expected.denominator, expected.numerator * price.denominator);
		assert_eq!(liquidity, BigInt::from((1u128 << 64) / 1_000_000));

		asserter.push_success(&Bytes::from(observeCall::abi_encode_returns(&observeReturn {
			tickCumulatives: vec![I56::ZERO, I56::ZERO],
			secondsPerLiquidityCumulativeX128s: vec![],
		})));
		let error = oracle.get_twap_and_liquidity_basis(&provider, hook).await.unwrap_err();
		assert!(error.to_string().contains("invalid observation lengths"));
	}

	#[tokio::test]
	async fn combines_configured_v3_and_v4_sources_for_either_token_order() {
		use uniswap_lens::bindings::iuniswapv3pool::IUniswapV3Pool::{token0Call, token1Call};
		for (lookup_address, ticks) in [
			(Address::repeat_byte(1), [-276_324i32, -262_461i32]),
			(Address::repeat_byte(255), [276_324i32, 262_461i32]),
		] {
			let asserter = Asserter::new();
			let mut oracle = UniswapOracle::new(
				vec!["http://localhost:8545".into()],
				token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
				token!(ChainId::MAINNET as u64, lookup_address, 18, "ARGON"),
			)
			.await
			.unwrap();
			oracle.providers =
				vec![ProviderBuilder::default().connect_mocked_client(asserter.clone())];
			let v3_pool = compute_pool_address(
				FACTORY_ADDRESS,
				USDC_ADDRESS,
				lookup_address,
				FeeAmount::HIGH,
				None,
				None,
			);
			let v4_hook = Address::repeat_byte(5);
			let mut expected_tokens = [USDC_ADDRESS, lookup_address];
			expected_tokens.sort();
			for _ in 0..2 {
				asserter.push_success(&Bytes::from(token0Call::abi_encode_returns(
					&expected_tokens[0],
				)));
				asserter.push_success(&Bytes::from(token1Call::abi_encode_returns(
					&expected_tokens[1],
				)));
			}
			oracle.set_oracle_addresses(&format!("{v3_pool},{v4_hook}")).await.unwrap();
			assert_eq!(oracle.oracle_addresses, vec![v3_pool, v4_hook]);

			// The selected v3 1% pool and v4 hook price the token at about $1 and $4, with equal
			// TWAL.
			for tick in ticks {
				asserter.push_success(&Bytes::from(observeCall::abi_encode_returns(
					&observeReturn {
						tickCumulatives: vec![
							I56::ZERO,
							I56::try_from(i64::from(tick) * 3_600).unwrap(),
						],
						secondsPerLiquidityCumulativeX128s: vec![
							U160::ZERO,
							U160::from(3_600u128 << 64),
						],
					},
				)));
			}
			let result = oracle.get_current_price().await.unwrap();
			// Arithmetic weighting is about $2.50. Weighting the raw fraction fields gives
			// about $1.60 for the reversed token order instead.
			assert!(asserter.read_q().is_empty());
			assert!((result.price.to_float() - 2.49987927).abs() < 0.000001);
			assert_eq!(result.liquidity, 2 * ((1u128 << 64) / 1_000_000));
		}
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

		let addresses = env::var("ARGONOT_UNISWAP_ORACLE_ADDRESSES")
			.expect("ARGONOT_UNISWAP_ORACLE_ADDRESSES must be set for live tests");
		let mut oracle = UniswapOracle::new(
			vec![fallback_url],
			token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
			token!(ChainId::MAINNET as u64, ARGONOT_ADDRESS, 18, "ARGONOT"),
		)
		.await
		.unwrap();
		oracle.set_oracle_addresses(&addresses).await.unwrap();
		let primary = Asserter::new();
		primary.push_failure_msg("Internal error");
		oracle
			.providers
			.insert(0, ProviderBuilder::default().connect_mocked_client(primary));

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
			let mut oracle = UniswapOracle::new(
				rpc_urls.clone(),
				token!(ChainId::MAINNET as u64, USDC_ADDRESS, 6, "USDC"),
				token!(ChainId::MAINNET as u64, address, 18, symbol),
			)
			.await
			.expect("Failed to create oracle");
			let addresses = env::var(format!("{symbol}_UNISWAP_ORACLE_ADDRESSES"))
				.expect("Explicit oracle addresses must be set for live tests");
			oracle.set_oracle_addresses(&addresses).await.unwrap();
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
