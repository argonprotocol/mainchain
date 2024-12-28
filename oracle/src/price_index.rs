use std::{env, time::Duration};

use anyhow::{anyhow, ensure};
use sp_runtime::{traits::One, FixedU128, Saturating};
use tokio::{join, time::sleep};
use tracing::{info, warn};

use crate::{argon_price, coin_usd_prices, us_cpi::UsCpiRetriever};
use argon_client::{
	api::{constants, price_index::calls::types::submit::Index, storage, tx},
	conversion::{from_api_fixed_u128, to_api_fixed_u128},
	signer::{KeystoreSigner, Signer},
	MainchainClient, ReconnectingClient,
};

pub async fn price_index_loop(
	trusted_rpc_url: String,
	signer: KeystoreSigner,
	use_simulated_schedule: bool,
) -> anyhow::Result<()> {
	let baseline_cpi = {
		let Some(baseline_cpi) = env::var("BASELINE_CPI").ok() else {
			warn!("No baseline CPI provided. Will restart in an hour to retry.");
			tokio::time::sleep(Duration::from_secs(60 * 60)).await;
			panic!("No baseline CPI provided. Exiting.");
		};
		baseline_cpi.parse::<f64>().expect("Baseline CPI must be a float")
	};
	let baseline_cpi = FixedU128::from_float(baseline_cpi);

	let mut reconnecting_client = ReconnectingClient::new(vec![trusted_rpc_url.clone()]);
	let mainchain_client = reconnecting_client.get().await?;

	if use_simulated_schedule {
		let chain_info = mainchain_client.methods.system_chain().await?;
		ensure!(
			chain_info.contains("Development") || chain_info.contains("Testnet"),
			"Simulated schedule can only be used on development chain"
		);
		#[cfg(not(feature = "simulated-prices"))]
		panic!("Simulated prices not enabled")
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

	let best_block = mainchain_client.best_block_hash().await?;
	let last_price = mainchain_client
		.fetch_storage(&storage().price_index().current(), Some(best_block))
		.await?;

	let constants_client = mainchain_client.live.constants();

	let max_argon_change_per_tick_away_from_target = from_api_fixed_u128(
		constants_client
			.at(&constants().price_index().max_argon_change_per_tick_away_from_target())?,
	);

	let max_argon_target_change_per_tick = from_api_fixed_u128(
		constants_client.at(&constants().price_index().max_argon_target_change_per_tick())?,
	);

	let mut last_submitted_tick = last_price.as_ref().map(|a| a.tick).unwrap_or(0);
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

	let mut us_cpi = UsCpiRetriever::new(&ticker, baseline_cpi).await?;
	let mut usd_price_lookups = coin_usd_prices::CoinUsdPriceLookup::new();

	let mut argon_price_lookup =
		argon_price::ArgonPriceLookup::from_env(&ticker, last_price).await?;

	info!("Oracle Started.");
	let account_id = signer.account_id();

	loop {
		let (usd_price_lookup, _) = join!(usd_price_lookups.get_latest_prices(), us_cpi.refresh());
		let usd_price_lookup = match usd_price_lookup {
			Ok(x) => x,
			Err(e) => {
				tracing::warn!("Couldn't update usd prices {:?}", e);
				continue;
			},
		};

		let tick = ticker.current();
		if tick == last_submitted_tick {
			let sleep_time = ticker.duration_to_next_tick().min(min_sleep_duration);
			sleep(sleep_time).await;
			continue;
		}
		let us_cpi_ratio = us_cpi.get_us_cpi_ratio(tick);
		let target_price = argon_price_lookup.get_target_price(us_cpi_ratio).clamp(
			last_target_price.saturating_sub(max_argon_target_change_per_tick),
			last_target_price.saturating_add(max_argon_target_change_per_tick),
		);
		let price_result = if use_simulated_schedule {
			#[cfg(not(feature = "simulated-prices"))]
			{
				unreachable!("Simulated prices not enabled")
			}
			#[cfg(feature = "simulated-prices")]
			argon_price_lookup
				.get_simulated_price(target_price, tick, max_argon_change_per_tick_away_from_target)
				.await
		} else {
			argon_price_lookup
				.get_latest_price(
					tick,
					max_argon_change_per_tick_away_from_target,
					usd_price_lookup.usdc,
				)
				.await
		};

		let argon_usd_price = match price_result {
			Ok(x) => x,
			Err(e) => {
				tracing::warn!("Couldn't update argon prices {:?}", e);
				continue;
			},
		};

		info!(
			"Current target price: {:?}, argon price {:?} at tick {:?}",
			target_price, argon_usd_price, tick
		);

		let price_index = tx().price_index().submit(Index {
			argon_usd_target_price: to_api_fixed_u128(target_price),
			tick,
			argon_usd_price: to_api_fixed_u128(argon_usd_price),
			btc_usd_price: to_api_fixed_u128(usd_price_lookup.bitcoin),
		});
		{
			let client = reconnecting_client.get().await?;
			let nonce = client.get_account_nonce(&account_id).await?;
			let params = MainchainClient::ext_params_builder().nonce(nonce.into()).build();
			let progress = client
				.live
				.tx()
				.sign_and_submit_then_watch(&price_index, &signer, params)
				.await?;
			last_submitted_tick = tick;
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

#[cfg(test)]
mod tests {
	use sp_core::{
		crypto::{key_types::ACCOUNT, AccountId32},
		sr25519, Pair,
	};
	use sp_keystore::{testing::MemoryKeystore, Keystore};
	use sp_runtime::FixedU128;
	use tokio::spawn;

	use argon_client::{api, signer::KeystoreSigner};
	use argon_primitives::CryptoType;
	use argon_testing::start_argon_test_node;

	use crate::{
		coin_usd_prices::{use_mock_price_lookups, PriceLookups},
		price_index_loop,
		uniswap_oracle::use_mock_argon_prices,
		us_cpi::use_mock_cpi_values,
	};

	#[tokio::test]
	async fn can_submit_multiple_price_indices() {
		let node = start_argon_test_node().await;
		let keystore = MemoryKeystore::new();
		let keypair = sr25519::Pair::from_string("//Eve", None).unwrap();
		keystore.insert(ACCOUNT, "//Eve", &keypair.public().0).unwrap();
		let account_id: AccountId32 = keypair.public().into();

		let signer = KeystoreSigner::new(keystore.into(), account_id, CryptoType::Sr25519);
		spawn(price_index_loop(node.client.url.clone(), signer, false));

		let mut block_sub = node.client.live.blocks().subscribe_best().await.unwrap();

		use_mock_argon_prices(vec![
			FixedU128::from_float(1.0),
			FixedU128::from_float(1.01),
			FixedU128::from_float(1.02),
		]);
		use_mock_price_lookups(PriceLookups {
			bitcoin: FixedU128::from_float(62_000.23),
			usdc: FixedU128::from_float(1.0),
		});
		use_mock_cpi_values(vec![0.2, 0.1, -0.1, 0.3]).await;
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
