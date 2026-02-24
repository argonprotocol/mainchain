use std::{env, fs, time::Duration};

use anyhow::{anyhow, ensure};
use polkadot_sdk::*;
use serde::{Deserialize, Deserializer, Serialize};
use sp_runtime::{
	FixedU128, Saturating,
	traits::{One, Zero},
};
use tokio::{join, time::sleep};
use tracing::info;

use crate::{
	argon_price, argonot_price, coin_usd_prices, coin_usd_prices::PriceProviderKind,
	uniswap_oracle::PriceAndLiquidity, us_cpi::UsCpiRetriever,
};
use argon_client::{
	FetchAt, MainchainClient, ReconnectingClient,
	api::{constants, price_index::calls::types::submit::Index, storage, tx},
	conversion::{from_api_fixed_u128, to_api_fixed_u128},
	signer::{KeystoreSigner, Signer},
};
use argon_primitives::prelude::sp_arithmetic::FixedPointNumber;

fn fixed_u128_from_float<'de, D>(deserializer: D) -> Result<FixedU128, D::Error>
where
	D: Deserializer<'de>,
{
	let value = f64::deserialize(deserializer)?;
	Ok(FixedU128::from_float(value))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PriceIndex {
	#[serde(deserialize_with = "fixed_u128_from_float")]
	pub argon_usd_target_price: FixedU128,
	#[serde(deserialize_with = "fixed_u128_from_float")]
	pub argon_usd_price: FixedU128,
	pub argon_time_weighted_average_liquidity: u128,
	#[serde(deserialize_with = "fixed_u128_from_float")]
	pub argonot_usd_price: FixedU128,
	#[serde(deserialize_with = "fixed_u128_from_float")]
	pub btc_usd_price: FixedU128,
}

pub async fn price_index_loop(
	trusted_rpc_url: String,
	signer: KeystoreSigner,
	coin_price_providers: Vec<PriceProviderKind>,
) -> anyhow::Result<()> {
	let mut reconnecting_client = ReconnectingClient::new(vec![trusted_rpc_url.clone()]);
	let mainchain_client = reconnecting_client.get().await?;

	let mut is_test = false;

	let chain_info = mainchain_client.methods.system_chain().await?;
	if chain_info.contains("Development") || chain_info.contains("Testnet") {
		is_test = true;
	}

	let mut ticker = mainchain_client.lookup_ticker().await?;
	if let Ok(ntp_pool) = env::var("NTP_POOL") {
		if !ntp_pool.is_empty() {
			ticker
				.lookup_ntp_offset(&ntp_pool)
				.await
				.map_err(|e| anyhow!("Unable to synchronize time {e:?}"))?;
		}
	}

	let last_price = mainchain_client
		.fetch_storage(&storage().price_index().current(), FetchAt::Best)
		.await?;

	let constants_client = mainchain_client.live.constants();

	let max_argon_change_per_tick_away_from_target = from_api_fixed_u128(
		constants_client
			.at(&constants().price_index().max_argon_change_per_tick_away_from_target())?,
	);

	let max_argon_target_change_per_tick = from_api_fixed_u128(
		constants_client.at(&constants().price_index().max_argon_target_change_per_tick())?,
	);

	let mut last_attempted_tick = last_price.as_ref().map(|a| a.tick).unwrap_or(0);
	let mut last_target_price = last_price
		.as_ref()
		.map(|a| from_api_fixed_u128(a.argon_usd_target_price.clone()))
		.unwrap_or(FixedU128::one());

	let mut min_sleep_duration = Duration::from_millis(ticker.tick_duration_millis)
		.saturating_sub(Duration::from_secs(10))
		.max(Duration::from_secs(5));
	if cfg!(test) {
		min_sleep_duration = Duration::from_millis(50);
	}

	let mut us_cpi = UsCpiRetriever::new(&ticker).await?;
	let mut usd_price_lookups =
		coin_usd_prices::CoinUsdPriceLookup::new_with_providers(coin_price_providers);

	let mut argon_price_lookup =
		argon_price::ArgonPriceLookup::from_env(&ticker, last_price).await?;
	let argonot_price_lookup = argonot_price::ArgonotPriceLookup::from_env().await?;

	info!("Oracle Started.");
	let account_id = signer.account_id();

	loop {
		let tick = ticker.current();
		if tick == last_attempted_tick {
			let sleep_time = ticker.duration_to_next_tick().min(min_sleep_duration);
			sleep(sleep_time).await;
			continue;
		}
		last_attempted_tick = tick;

		let (usd_price_lookup, _) = join!(usd_price_lookups.get_latest_prices(), us_cpi.refresh());
		let usd_price_lookup = match usd_price_lookup {
			Ok(x) => x,
			Err(e) => {
				tracing::warn!("Couldn't update usd prices {:?}", e);
				continue;
			},
		};
		let us_cpi_ratio = us_cpi.get_us_cpi_ratio(tick);
		let target_price = argon_price_lookup.get_target_price(us_cpi_ratio).clamp(
			last_target_price.saturating_sub(max_argon_target_change_per_tick),
			last_target_price.saturating_add(max_argon_target_change_per_tick),
		);
		let price_result = argon_price_lookup
			.get_latest_price_and_liquidity(
				tick,
				max_argon_change_per_tick_away_from_target,
				usd_price_lookup.usdc,
			)
			.await;

		let argon_usd_price = match price_result {
			Ok(x) => x,
			Err(e) =>
				if is_test {
					tracing::warn!(
						"Couldn't update argon prices. Using target {} {:?}",
						target_price,
						e
					);
					PriceAndLiquidity { price: target_price, liquidity: 0 }
				} else {
					tracing::warn!("Couldn't update argon prices {:?}", e);
					continue;
				},
		};

		let argonot_price_lookup = match argonot_price_lookup
			.get_latest_price(usd_price_lookup.usdc)
			.await
		{
			Ok(x) => x,
			Err(e) =>
				if is_test {
					tracing::warn!("Couldn't update argonot prices, using default of 0 {:?}", e);
					FixedU128::zero()
				} else {
					tracing::warn!("Couldn't update argonot prices {:?}", e);
					continue;
				},
		};

		let argon_liquidity = argon_usd_price.liquidity;
		let argon_usd_price = trunc_fixed_u128(argon_usd_price.price, 3);
		let argonot_usd_price = trunc_fixed_u128(argonot_price_lookup, 3);
		let argon_usd_target_price = trunc_fixed_u128(target_price, 3);
		let bitcoin_usd_price = trunc_fixed_u128(usd_price_lookup.bitcoin, 3);

		info!(
			"Current target price: {:?} vs price {:?}, liquidity {:?}, at tick {:?}",
			argon_usd_target_price.to_float(),
			argon_usd_price.to_float(),
			argon_liquidity,
			tick
		);

		let price_index = tx().price_index().submit(Index {
			argon_usd_target_price: to_api_fixed_u128(argon_usd_target_price),
			tick,
			argon_usd_price: to_api_fixed_u128(argon_usd_price),
			argon_time_weighted_average_liquidity: argon_liquidity,
			argonot_usd_price: to_api_fixed_u128(argonot_usd_price),
			btc_usd_price: to_api_fixed_u128(bitcoin_usd_price),
		});
		{
			let client = reconnecting_client.get().await?;
			let nonce = client.get_account_nonce(&account_id).await?;
			let params =
				MainchainClient::ext_params_builder().nonce(nonce.into()).mortal(5).build();
			let progress = client
				.live
				.tx()
				.sign_and_submit_then_watch(&price_index, &signer, params)
				.await?;
			last_target_price = target_price;

			info!("Submitted price index with progress: {:?}", progress);
			MainchainClient::wait_for_ext_in_block(progress, false).await.map_err(|e| {
				tracing::warn!("Error processing price index!! {:?}", e);
				e
			})?;
		}

		let sleep_time = ticker.duration_to_next_tick().min(min_sleep_duration);
		sleep(sleep_time).await;
	}
}

/// Development feature to load price index data from a file instead of live oracles. Many of the
/// providers are rate limited, and this is the simplest way to simulate specific scenarios
pub async fn price_index_loop_from_file(
	trusted_rpc_url: String,
	signer: KeystoreSigner,
	file_path: String,
) -> anyhow::Result<()> {
	let mut reconnecting_client = ReconnectingClient::new(vec![trusted_rpc_url.clone()]);
	let mainchain_client = reconnecting_client.get().await?;

	let chain_info = mainchain_client.methods.system_chain().await?;
	ensure!(
		chain_info.contains("Development") || chain_info.contains("Testnet"),
		"File-based price index can only be used on development chain"
	);

	let ticker = mainchain_client.lookup_ticker().await?;
	let last_price = mainchain_client
		.fetch_storage(&storage().price_index().current(), FetchAt::Best)
		.await?;

	let mut last_submitted_tick = last_price.as_ref().map(|a| a.tick).unwrap_or(0);

	let mut min_sleep_duration = Duration::from_millis(ticker.tick_duration_millis)
		.saturating_sub(Duration::from_secs(10))
		.max(Duration::from_secs(5));
	if cfg!(test) {
		min_sleep_duration = Duration::from_millis(50);
	}

	info!("Oracle Started.");
	let account_id = signer.account_id();

	loop {
		let tick = ticker.current();
		if tick == last_submitted_tick {
			let sleep_time = ticker.duration_to_next_tick().min(min_sleep_duration);
			sleep(sleep_time).await;
			continue;
		}

		let price_data_raw = fs::read_to_string(&file_path)
			.map_err(|e| anyhow!("Failed to load price data from file: {e:?}"))?;
		let price_data: PriceIndex = serde_json::from_str(&price_data_raw)
			.map_err(|e| anyhow!("Failed to parse price data from file {file_path:?}: {e:?}"))?;

		let price_index = tx().price_index().submit(Index {
			argon_usd_target_price: to_api_fixed_u128(price_data.argon_usd_target_price),
			tick,
			argon_usd_price: to_api_fixed_u128(price_data.argon_usd_price),
			argon_time_weighted_average_liquidity: price_data.argon_time_weighted_average_liquidity,
			argonot_usd_price: to_api_fixed_u128(price_data.argonot_usd_price),
			btc_usd_price: to_api_fixed_u128(price_data.btc_usd_price),
		});

		let client = reconnecting_client.get().await?;
		let nonce = client.get_account_nonce(&account_id).await?;
		let params = MainchainClient::ext_params_builder().nonce(nonce.into()).mortal(5).build();
		let progress = client
			.live
			.tx()
			.sign_and_submit_then_watch(&price_index, &signer, params)
			.await?;
		last_submitted_tick = tick;

		info!("Submitted price index with progress: {:?}", progress);
		MainchainClient::wait_for_ext_in_block(progress, false).await.map_err(|e| {
			tracing::warn!("Error processing price index!! {:?}", e);
			e
		})?;

		let sleep_time = ticker.duration_to_next_tick().min(min_sleep_duration);
		sleep(sleep_time).await;
	}
}

/// Truncates a FixedU128 value to the specified number of decimal places.
/// For example, trunc_fixed_u128(value, 3) will truncate to 3 decimal places.
fn trunc_fixed_u128(value: FixedU128, decimals: u16) -> FixedU128 {
	let drop = FixedU128::accuracy() / (10u128.pow(decimals as u32)); // 10^(18-3)
	FixedU128::from_inner((value.into_inner() / drop) * drop)
}

#[cfg(test)]
mod tests {
	use alloy_primitives::Address;
	use polkadot_sdk::*;
	use sp_core::{
		Pair,
		crypto::{AccountId32, key_types::ACCOUNT},
		sr25519,
	};
	use sp_keystore::{Keystore, testing::MemoryKeystore};
	use sp_runtime::FixedU128;
	use std::{env, str::FromStr};
	use tokio::spawn;

	use argon_client::{api, signer::KeystoreSigner};
	use argon_primitives::CryptoType;
	use argon_testing::start_argon_test_node;

	use crate::{
		coin_usd_prices::{PriceLookups, use_mock_price_lookups},
		price_index_loop,
		uniswap_oracle::{PriceAndLiquidity, use_mock_uniswap_prices},
		us_cpi::use_mock_cpi_values,
	};

	#[tokio::test]
	async fn can_submit_multiple_price_indices() {
		let node = start_argon_test_node().await;
		let keystore = MemoryKeystore::new();
		let keypair = sr25519::Pair::from_string("//Eve", None).unwrap();
		keystore.insert(ACCOUNT, "//Eve", &keypair.public().0).unwrap();
		let account_id: AccountId32 = keypair.public().into();

		const ARGON_TOKEN_ADDRESS: &str = "6b175474e89094c44da98b954eedeac495271d0f";
		const ARGONOT_TOKEN_ADDRESS: &str = "64CBd3aa07d427E385Cb55330406508718E55f01";
		unsafe {
			env::set_var("ARGON_TOKEN_ADDRESS", ARGON_TOKEN_ADDRESS);
			env::set_var("ARGONOT_TOKEN_ADDRESS", ARGONOT_TOKEN_ADDRESS);
			env::set_var("INFURA_PROJECT_ID", "test");
		}

		let mut block_sub = node.client.live.blocks().subscribe_best().await.unwrap();
		let argon_address = Address::from_str(ARGON_TOKEN_ADDRESS).unwrap();
		// Keep enough mocked samples so the loop never falls back to live Uniswap RPC in test.
		let mut argon_prices = Vec::with_capacity(100);
		for i in 0..100 {
			let price = match i % 3 {
				0 => 1.0,
				1 => 1.01,
				_ => 1.02,
			};
			argon_prices.push(PriceAndLiquidity {
				price: FixedU128::from_float(price),
				liquidity: 100_000_000,
			});
		}
		use_mock_uniswap_prices(argon_address, argon_prices);

		let argonot_address = Address::from_str(ARGONOT_TOKEN_ADDRESS).unwrap();
		let mut argonot_prices = Vec::with_capacity(100);
		for i in 0..100 {
			let price = match i % 3 {
				0 => 2.0,
				1 => 2.01,
				_ => 2.02,
			};
			argonot_prices.push(PriceAndLiquidity {
				price: FixedU128::from_float(price),
				liquidity: 1_000_000,
			});
		}
		use_mock_uniswap_prices(argonot_address, argonot_prices);
		use_mock_price_lookups(PriceLookups {
			bitcoin: FixedU128::from_float(62_000.23),
			usdc: FixedU128::from_float(1.0),
		});
		use_mock_cpi_values(vec![0.2, 0.1, -0.1, 0.3]).await;
		let signer = KeystoreSigner::new(keystore.into(), account_id, CryptoType::Sr25519);
		spawn(price_index_loop(node.client.url.clone(), signer, vec![]));
		let mut counter = 0;
		let mut blocks = 0;
		while let Some(Ok(block)) = block_sub.next().await {
			blocks += 1;
			let price_index = block
				.events()
				.await
				.unwrap()
				.find_first::<api::price_index::events::NewIndex>()
				.unwrap();
			if price_index.is_some() {
				counter += 1;
				if counter > 3 {
					break;
				}
			}
			if blocks > 10 {
				break;
			}
		}
		assert!(counter >= 3);
	}
}
