use crate::utils::parse_f64;
use anyhow::{Result, anyhow};
use polkadot_sdk::*;
use reqwest::Client;
use serde::Deserialize;
use sp_runtime::{
	FixedU128,
	traits::{One, Zero},
};
use std::collections::HashMap;
use tokio::{join, time::Instant};

pub struct CoinUsdPriceLookup {
	pub client: Client,
	pub last_refresh: Option<(Instant, PriceLookups)>,
}

#[cfg(test)]
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(test)]
lazy_static::lazy_static! {
	static ref MOCK_PRICES: Arc<Mutex<Option<PriceLookups>>> = Arc::new(Mutex::new(None));
}

impl CoinUsdPriceLookup {
	pub fn new() -> Self {
		Self { client: Client::new(), last_refresh: None }
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

		let (a, b, c) =
			join![self.get_coinbase_prices(), self.get_kraken_price(), self.get_coingecko_price()];

		let mut usdc_total = FixedU128::zero();
		let mut usdc_count = FixedU128::zero();
		let mut btc_total = FixedU128::zero();
		let mut btc_count = FixedU128::zero();
		for price in [a, b, c] {
			match price {
				Ok(price) => {
					if let Some(usdc) = price.usdc {
						usdc_total = usdc_total.add(usdc);
						usdc_count = usdc_count + FixedU128::one();
					}
					btc_total = btc_total.add(price.bitcoin);
					btc_count = btc_count + FixedU128::one();
				},
				Err(err) => {
					println!("Failed to get price: {}", err);
				},
			}
		}

		let average_btc = btc_total / btc_count;
		let average_usdc = usdc_total / usdc_count;
		let latest = PriceLookups { bitcoin: average_btc, usdc: average_usdc };
		self.last_refresh = Some((Instant::now(), latest));

		Ok(latest)
	}

	async fn get_kraken_price(&self) -> Result<PriceLookupMaybe> {
		let response = self
			.client
			.get("https://api.kraken.com/0/public/Ticker?pair=XXBTZUSD,USDCUSD")
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
	async fn get_coingecko_price(&self) -> Result<PriceLookupMaybe> {
		let response = self
			.client
			.get("https://api.coingecko.com/api/v3/simple/price")
			.query(&[("ids", "bitcoin,usd-coin"), ("vs_currencies", "usd")])
			.send()
			.await?
			.json::<CoinGeckoResponse>()
			.await?;

		Ok(PriceLookupMaybe {
			bitcoin: FixedU128::from_float(response.bitcoin.usd),
			usdc: Some(FixedU128::from_float(response.usd_coin.usd)),
		})
	}

	async fn get_coinbase_prices(&self) -> Result<PriceLookupMaybe> {
		let (bitcoin, usdc) = join!(
			self.get_coinbase_price("BTC".to_string()),
			self.get_coinbase_price("USDC".to_string())
		);
		Ok(PriceLookupMaybe { bitcoin: bitcoin?, usdc: usdc.ok() })
	}

	async fn get_coinbase_price(&self, coin: String) -> Result<FixedU128> {
		let response = self
			.client
			.get(format!("https://api.coinbase.com/v2/prices/{coin}-USD/spot"))
			.send()
			.await?
			.json::<CoinbasePrice>()
			.await?;
		Ok(FixedU128::from_float(response.data.amount))
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
#[cfg(test)]
pub(crate) fn use_mock_price_lookups(prices: PriceLookups) {
	*MOCK_PRICES.lock().unwrap() = Some(prices);
}

#[cfg(test)]
mod tests {
	use super::*;
	#[tokio::test]
	async fn test_get_price_lookups() {
		let price_lookups = CoinUsdPriceLookup::new().get_latest_prices().await.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc < FixedU128::from_float(1.1));
	}

	#[tokio::test]
	async fn test_get_kraken_price() {
		let price_lookups = CoinUsdPriceLookup::new().get_kraken_price().await.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc.unwrap() > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc.unwrap() < FixedU128::from_float(1.1));
	}

	#[tokio::test]
	#[ignore] // Coingecko has rate limits
	async fn test_get_coingecko_price() {
		let price_lookups = CoinUsdPriceLookup::new().get_coingecko_price().await.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc.unwrap() > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc.unwrap() < FixedU128::from_float(1.1));
	}

	#[tokio::test]
	async fn test_get_coinbase_price() {
		let price_lookups = CoinUsdPriceLookup::new().get_coinbase_prices().await.unwrap();
		assert!(price_lookups.bitcoin > FixedU128::from_float(30_000.0));
		assert!(price_lookups.usdc.unwrap() > FixedU128::from_float(0.9));
		assert!(price_lookups.usdc.unwrap() < FixedU128::from_float(1.1));
	}
}
