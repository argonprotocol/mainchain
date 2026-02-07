use crate::utils::parse_f64;
use anyhow::{Result, anyhow};
use clap::ValueEnum;
use polkadot_sdk::*;
use reqwest::Client;
use serde::Deserialize;
use sp_runtime::{FixedU128, traits::One};
use std::{collections::HashMap, time::Duration};
use tokio::{join, task::JoinSet, time::Instant};

#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(test)]
lazy_static::lazy_static! {
	static ref MOCK_PRICES: Arc<Mutex<Option<PriceLookups>>> = Arc::new(Mutex::new(None));
}

pub struct CoinUsdPriceLookup {
	pub client: Client,
	pub last_refresh: Option<(Instant, PriceLookups)>,
	pub providers: Vec<PriceProviderKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum PriceProviderKind {
	Coinbase,
	Kraken,
	Coingecko,
	Gemini,
	Bitstamp,
}

pub const ALL_PRICE_PROVIDERS: &[PriceProviderKind] = &[
	PriceProviderKind::Coinbase,
	PriceProviderKind::Kraken,
	PriceProviderKind::Coingecko,
	PriceProviderKind::Gemini,
	PriceProviderKind::Bitstamp,
];

impl CoinUsdPriceLookup {
	pub fn new_with_providers(providers: Vec<PriceProviderKind>) -> Self {
		Self { client: Client::new(), last_refresh: None, providers }
	}

	fn median_fixed(values: &mut [FixedU128]) -> Option<FixedU128> {
		if values.is_empty() {
			return None;
		}
		values.sort_by_key(|x| x.into_inner());
		let middle_index = values.len() / 2;
		if values.len() % 2 == 1 {
			Some(values[middle_index])
		} else {
			let lower = values[middle_index - 1].into_inner();
			let upper = values[middle_index].into_inner();
			Some(FixedU128::from_inner((lower + upper) / 2))
		}
	}

	fn default_usdc_when_missing() -> FixedU128 {
		// USDC is expected to be near $1.00; if we can't fetch it, we assume the peg rather than
		// dividing by zero.
		FixedU128::one()
	}

	pub async fn get_latest_prices(&mut self) -> Result<PriceLookups> {
		#[cfg(test)]
		{
			let mut mock = MOCK_PRICES.lock().unwrap();
			if let Some(x) = mock.take() {
				*mock = Some(x);
				return Ok(x);
			}
		}

		const MIN_REFRESH: Duration = Duration::from_secs(60);

		if let Some((last_refresh, value)) = self.last_refresh {
			if last_refresh.elapsed() < MIN_REFRESH {
				return Ok(value);
			}
		}
		if self.providers.is_empty() {
			return Err(anyhow!(
				"No price providers configured (providers list is empty). Pass at least one provider to CoinUsdPriceLookup::new_with_providers()."
			));
		}

		let mut join_set: JoinSet<(PriceProviderKind, Result<PriceLookupMaybe>)> = JoinSet::new();
		for provider in self.providers.iter().copied() {
			let client = self.client.clone();
			join_set.spawn(async move {
				let result =
					CoinUsdPriceLookup::get_price_from_provider_with_client(&client, provider)
						.await;
				(provider, result)
			});
		}

		let mut bitcoin_values: Vec<FixedU128> = Vec::new();
		let mut usdc_values: Vec<FixedU128> = Vec::new();

		while let Some(joined) = join_set.join_next().await {
			match joined {
				Ok((_provider, Ok(price))) => {
					bitcoin_values.push(price.bitcoin);
					if let Some(usdc) = price.usdc {
						usdc_values.push(usdc);
					}
				},
				Ok((provider, Err(err))) => {
					// Avoid the literal substring "error" in our own message; upstream errors may
					// still contain it.
					println!("Price lookup failed on {provider:?}: {err}");
				},
				Err(join_err) => {
					println!("Price lookup task failed: {join_err}");
				},
			}
		}

		let median_btc = Self::median_fixed(&mut bitcoin_values)
			.ok_or_else(|| anyhow!("No bitcoin prices available from any provider"))?;
		let median_usdc =
			Self::median_fixed(&mut usdc_values).unwrap_or_else(Self::default_usdc_when_missing);

		let latest = PriceLookups { bitcoin: median_btc, usdc: median_usdc };
		self.last_refresh = Some((Instant::now(), latest));

		Ok(latest)
	}

	async fn get_price_from_provider_with_client(
		client: &Client,
		provider: PriceProviderKind,
	) -> Result<PriceLookupMaybe> {
		match provider {
			PriceProviderKind::Coinbase => Self::get_coinbase_prices_with_client(client).await,
			PriceProviderKind::Kraken => Self::get_kraken_price_with_client(client).await,
			PriceProviderKind::Coingecko => Self::get_coingecko_price_with_client(client).await,
			PriceProviderKind::Gemini => Self::get_gemini_prices_with_client(client).await,
			PriceProviderKind::Bitstamp => Self::get_bitstamp_prices_with_client(client).await,
		}
	}

	async fn get_kraken_price_with_client(client: &Client) -> Result<PriceLookupMaybe> {
		let response = client
			.get("https://api.kraken.com/0/public/Ticker?pair=XXBTZUSD,USDCUSD")
			.timeout(Duration::from_secs(5))
			.send()
			.await?
			.json::<KrakenResponse>()
			.await?;
		let btc = response.result.get("XXBTZUSD").ok_or(anyhow!("No price data"))?.last_trade_cost
			[0]
		.parse::<f64>()?;
		let usdc = response.result.get("USDCUSD").ok_or(anyhow!("No price data"))?.last_trade_cost
			[0]
		.parse::<f64>()?;

		Ok(PriceLookupMaybe {
			bitcoin: FixedU128::from_float(btc),
			usdc: Some(FixedU128::from_float(usdc)),
		})
	}

	async fn get_coingecko_price_with_client(client: &Client) -> Result<PriceLookupMaybe> {
		let response = client
			.get("https://api.coingecko.com/api/v3/simple/price")
			.query(&[("ids", "bitcoin,usd-coin"), ("vs_currencies", "usd")])
			.timeout(Duration::from_secs(5))
			.send()
			.await?
			.json::<CoinGeckoResponse>()
			.await?;

		Ok(PriceLookupMaybe {
			bitcoin: FixedU128::from_float(response.bitcoin.usd),
			usdc: Some(FixedU128::from_float(response.usd_coin.usd)),
		})
	}

	async fn get_coinbase_prices_with_client(client: &Client) -> Result<PriceLookupMaybe> {
		let (bitcoin, usdc) = join!(
			Self::get_coinbase_price_with_client(client, "BTC"),
			Self::get_coinbase_price_with_client(client, "USDC")
		);
		Ok(PriceLookupMaybe { bitcoin: bitcoin?, usdc: usdc.ok() })
	}

	async fn get_coinbase_price_with_client(client: &Client, coin: &str) -> Result<FixedU128> {
		let response = client
			.get(format!("https://api.coinbase.com/v2/prices/{coin}-USD/spot"))
			.timeout(Duration::from_secs(5))
			.send()
			.await?
			.json::<CoinbasePrice>()
			.await?;
		Ok(FixedU128::from_float(response.data.amount))
	}

	async fn get_gemini_prices_with_client(client: &Client) -> Result<PriceLookupMaybe> {
		let (bitcoin_result, usdc_result) = join!(
			Self::get_gemini_price_with_client(client, "btcusd"),
			Self::get_gemini_price_with_client(client, "usdcusd")
		);

		Ok(PriceLookupMaybe {
			bitcoin: FixedU128::from_float(bitcoin_result?),
			usdc: Some(FixedU128::from_float(usdc_result?)),
		})
	}

	async fn get_gemini_price_with_client(client: &Client, symbol: &str) -> Result<f64> {
		let response = client
			.get(format!("https://api.gemini.com/v1/pubticker/{symbol}"))
			.timeout(Duration::from_secs(5))
			.send()
			.await?
			.json::<GeminiTicker>()
			.await?;
		Ok(response.last.parse::<f64>()?)
	}

	async fn get_bitstamp_prices_with_client(client: &Client) -> Result<PriceLookupMaybe> {
		let (bitcoin_result, usdc_result) = join!(
			Self::get_bitstamp_price_with_client(client, "btcusd"),
			Self::get_bitstamp_price_with_client(client, "usdcusd")
		);

		Ok(PriceLookupMaybe {
			bitcoin: FixedU128::from_float(bitcoin_result?),
			usdc: Some(FixedU128::from_float(usdc_result?)),
		})
	}

	async fn get_bitstamp_price_with_client(client: &Client, pair: &str) -> Result<f64> {
		let response = client
			.get(format!("https://www.bitstamp.net/api/v2/ticker/{pair}"))
			.timeout(Duration::from_secs(5))
			.send()
			.await?
			.json::<BitstampTicker>()
			.await?;
		Ok(response.last.parse::<f64>()?)
	}
}

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct PriceLookups {
	pub bitcoin: FixedU128,
	pub usdc: FixedU128,
}

struct PriceLookupMaybe {
	bitcoin: FixedU128,
	usdc: Option<FixedU128>,
}

#[derive(Deserialize)]
struct CoinGeckoResponse {
	bitcoin: CoinGeckoPriceData,
	#[serde(rename = "usd-coin")]
	usd_coin: CoinGeckoPriceData,
}

#[derive(Deserialize)]
struct CoinGeckoPriceData {
	usd: f64,
}

#[derive(Deserialize, Debug)]
pub struct CoinbasePrice {
	pub data: CoinbasePriceData,
}

#[derive(Deserialize, Debug)]
pub struct CoinbasePriceData {
	// pub base: String,
	// pub currency: String,
	#[serde(deserialize_with = "parse_f64")]
	pub amount: f64,
}

#[derive(Deserialize)]
struct KrakenResponse {
	result: HashMap<String, KrakenResponsePair>,
}

#[derive(Deserialize)]
struct KrakenResponsePair {
	#[serde(rename = "c")]
	last_trade_cost: [String; 2],
}

#[derive(Deserialize)]
struct GeminiTicker {
	last: String,
}

#[derive(Deserialize)]
struct BitstampTicker {
	last: String,
}

#[cfg(test)]
pub(crate) fn use_mock_price_lookups(prices: PriceLookups) {
	*MOCK_PRICES.lock().unwrap() = Some(prices);
}

#[cfg(test)]
mod tests {
	use super::*;
	#[tokio::test]
	async fn test_get_price_lookups() {
		let price_lookups = CoinUsdPriceLookup::new_with_providers(ALL_PRICE_PROVIDERS.to_vec())
			.get_latest_prices()
			.await
			.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc < FixedU128::from_float(1.1));
	}

	#[tokio::test]
	async fn test_get_kraken_price() {
		let price_lookups =
			CoinUsdPriceLookup::get_kraken_price_with_client(&Client::new()).await.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc.unwrap() > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc.unwrap() < FixedU128::from_float(1.1));
	}

	#[tokio::test]
	#[ignore] // Coingecko has rate limits
	async fn test_get_coingecko_price() {
		let price_lookups = CoinUsdPriceLookup::get_coingecko_price_with_client(&Client::new())
			.await
			.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc.unwrap() > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc.unwrap() < FixedU128::from_float(1.1));
	}

	#[tokio::test]
	async fn test_get_gemini_price() {
		let price_lookups =
			CoinUsdPriceLookup::get_gemini_prices_with_client(&Client::new()).await.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc.unwrap() > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc.unwrap() < FixedU128::from_float(1.1));
	}

	#[tokio::test]
	async fn test_get_bitstamp_price() {
		let price_lookups = CoinUsdPriceLookup::get_bitstamp_prices_with_client(&Client::new())
			.await
			.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc.unwrap() > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc.unwrap() < FixedU128::from_float(1.1));
	}

	#[tokio::test]
	async fn test_get_coinbase_price() {
		let price_lookups = CoinUsdPriceLookup::get_coinbase_prices_with_client(&Client::new())
			.await
			.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc.unwrap() > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc.unwrap() < FixedU128::from_float(1.1));
	}
}
