use std::{thread::spawn, time::Duration};

use anyhow::anyhow;
use sp_runtime::Saturating;
use tokio::{join, time::sleep};
use tracing::info;

use crate::{argon_price, btc_price, us_cpi::UsCpiRetriever};
use ulixee_client::{
	api::{
		constants, price_index::calls::types::submit::Index,
		runtime_types::sp_arithmetic::fixed_point::FixedU128, storage, tx,
	},
	signer::{KeystoreSigner, Signer},
	MainchainClient, ReconnectingClient,
};

pub async fn price_index_loop(
	trusted_rpc_url: String,
	signer: KeystoreSigner,
	use_simulated_schedule: bool,
) -> anyhow::Result<()> {
	let mut mainchain_client = ReconnectingClient::new(vec![trusted_rpc_url.clone()]);
	let mut ticker = mainchain_client.get().await?.lookup_ticker().await?;
	if !cfg!(test) {
		ticker
			.lookup_ntp_offset("pool.ntp.org")
			.await
			.map_err(|e| anyhow!("Unable to synchronize time {e:?}"))?;
	}
	let best_block = mainchain_client.get().await?.best_block_hash().await?;
	let last_price = mainchain_client
		.get()
		.await?
		.fetch_storage(&storage().price_index().current(), Some(best_block))
		.await?;

	let max_argon_change_per_tick_away_from_target = mainchain_client
		.get()
		.await?
		.live
		.constants()
		.at(&constants().price_index().max_argon_change_per_tick_away_from_target())?;
	let max_argon_change_per_tick_away_from_target =
		sp_runtime::FixedU128::from_inner(max_argon_change_per_tick_away_from_target.0);
	let max_argon_target_change_per_tick = mainchain_client
		.get()
		.await?
		.live
		.constants()
		.at(&constants().price_index().max_argon_target_change_per_tick())?;
	let max_argon_target_change_per_tick =
		sp_runtime::FixedU128::from_inner(max_argon_target_change_per_tick.0);

	let mut last_submitted_tick = last_price.as_ref().map(|a| a.tick).unwrap_or(0);
	let mut last_target_price = last_price
		.as_ref()
		.map(|a| sp_runtime::FixedU128::from_inner(a.argon_usd_target_price.0))
		.unwrap_or_default();

	let min_sleep_duration = Duration::from_millis(ticker.tick_duration_millis)
		.saturating_sub(Duration::from_secs(10))
		.max(Duration::from_secs(5));

	let mut us_cpi = UsCpiRetriever::new(&ticker).await?;
	let btc_price_lookup = btc_price::BtcPriceLookup::new();
	let mut argon_price_lookup =
		argon_price::ArgonPriceLookup::new(use_simulated_schedule, &ticker, last_price);

	info!("Oracle Started.");
	let account_id = signer.account_id();

	loop {
		let (btc_price, _) = join!(btc_price_lookup.get_btc_price(), us_cpi.refresh());
		let btc_price = match btc_price {
			Ok(x) => x,
			Err(e) => {
				tracing::warn!("Couldn't update btc prices {:?}", e);
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
		let argon_usd_price = match argon_price_lookup
			.get_argon_price(target_price, tick, max_argon_change_per_tick_away_from_target)
			.await
		{
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
			argon_usd_target_price: FixedU128(target_price.into_inner()),
			tick,
			argon_usd_price: FixedU128(argon_usd_price.into_inner()),
			btc_usd_price: FixedU128(btc_price.into_inner()),
		});
		{
			let client = mainchain_client.get().await?;

			let nonce = client.get_account_nonce_subxt(&account_id).await?;
			let params = MainchainClient::ext_params_builder().nonce(nonce.into()).build();
			let progress = client
				.live
				.tx()
				.sign_and_submit_then_watch(&price_index, &signer, params)
				.await?;
			last_submitted_tick = tick;
			last_target_price = target_price;

			info!("Submitted price index with progress: {:?}", progress);
			spawn(move || {
				if let Err(res) =
					futures::executor::block_on(MainchainClient::wait_for_ext_in_block(progress))
				{
					panic!("Error processing price index!! {:?}", res)
				}
			});
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

	use ulixee_client::{api, signer::KeystoreSigner};
	use ulx_primitives::CryptoType;
	use ulx_testing::start_ulx_test_node;

	use crate::{btc_price::use_mock_btc_price, price_index_loop, us_cpi::use_mock_cpi_values};

	#[tokio::test]
	async fn can_submit_multiple_price_indices() {
		let node = start_ulx_test_node().await;
		let keystore = MemoryKeystore::new();
		let keypair = sr25519::Pair::from_string("//Alice", None).unwrap();
		keystore.insert(ACCOUNT, "//Alice", &keypair.public().0).unwrap();
		let account_id: AccountId32 = keypair.public().into();

		let signer = KeystoreSigner::new(keystore.into(), account_id, CryptoType::Sr25519);
		spawn(price_index_loop(node.client.url.clone(), signer, true));

		let mut block_sub = node.client.live.blocks().subscribe_best().await.unwrap();

		use_mock_btc_price(FixedU128::from_float(62_000.23));
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
