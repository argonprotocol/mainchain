use crate::utils::parse_f64;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use sp_runtime::{
	traits::{One, Zero},
	FixedU128,
};
use std::collections::HashMap;
use tokio::join;

pub struct BtcPriceLookup {
	pub client: Client,
}

#[cfg(test)]
use std::sync::{Arc, Mutex};
#[cfg(test)]
lazy_static::lazy_static! {
	static ref MOCK_BTC_PRICE: Arc<Mutex<Option<FixedU128>>> = Arc::new(Mutex::new(None));
}

impl BtcPriceLookup {
	pub fn new() -> Self {
		Self { client: Client::new() }
	}

	pub async fn get_btc_price(&self) -> Result<FixedU128> {
		#[cfg(test)]
		{
			let mut mock = MOCK_BTC_PRICE.lock().unwrap();
			if let Some(x) = mock.take() {
				*mock = Some(x.clone());
				return Ok(x.clone())
			}
		}
		let (a, b, c, d) = join![
			self.get_coinbase_price(),
			self.get_coindesk_price(),
			self.get_kraken_price(),
			self.get_coingecko_price()
		];

		let mut total = FixedU128::zero();
		let mut count = FixedU128::zero();
		for price in [a, b, c, d] {
			match price {
				Ok(price) => {
					total = total.add(price);
					count = count + FixedU128::one();
				},
				Err(err) => {
					println!("Failed to get price: {}", err);
				},
			}
		}

		let average = total.div(count);

		Ok(average)
	}

	async fn get_coindesk_price(&self) -> Result<FixedU128> {
		let response = self
			.client
			.get("https://api.coindesk.com/v1/bpi/currentprice.json")
			.send()
			.await?
			.json::<CoinDeskResponse>()
			.await?;
		Ok(FixedU128::from_float(response.bpi.usd.rate_float))
	}
	async fn get_kraken_price(&self) -> Result<FixedU128> {
		let response = self
			.client
			.get("https://api.kraken.com/0/public/Ticker?pair=XXBTZUSD")
			.send()
			.await?
			.json::<KrakenResponse>()
			.await?;
		let first = response.result.get("XXBTZUSD").ok_or(anyhow!("No price data"))?;

		Ok(FixedU128::from_float(first.last_trade_cost[0].parse::<f64>()?))
	}
	async fn get_coingecko_price(&self) -> Result<FixedU128> {
		let response = self
			.client
			.get("https://api.coingecko.com/api/v3/simple/price")
			.query(&[("ids", "bitcoin"), ("vs_currencies", "usd")])
			.send()
			.await?
			.json::<CoinGeckoResponse>()
			.await?;

		Ok(FixedU128::from_float(response.bitcoin.usd))
	}
	async fn get_coinbase_price(&self) -> Result<FixedU128> {
		let response = self
			.client
			.get("https://api.coinbase.com/v2/prices/BTC-USD/spot")
			.send()
			.await?
			.json::<CoinbasePrice>()
			.await?;
		Ok(FixedU128::from_float(response.data.amount))
	}
}

#[derive(Deserialize)]
struct CoinGeckoResponse {
	bitcoin: CoinGeckoPriceData,
}

#[derive(Deserialize)]
struct CoinGeckoPriceData {
	usd: f64,
}

#[derive(Deserialize)]
struct CoinDeskResponse {
	bpi: CoindeskBpi,
}

#[derive(Deserialize)]
struct CoindeskBpi {
	#[serde(rename = "USD")]
	usd: CoindeskBpiData,
}

#[derive(Deserialize)]
struct CoindeskBpiData {
	rate_float: f64,
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
pub(crate) fn use_mock_btc_price(price: FixedU128) {
	*MOCK_BTC_PRICE.lock().unwrap() = Some(price);
}

#[cfg(test)]
mod tests {
	use super::*;
	#[tokio::test]
	async fn test_get_btc_price() {
		let btc_price = BtcPriceLookup::new().get_btc_price().await.unwrap();
		assert!(btc_price > FixedU128::zero());
	}

	#[tokio::test]
	async fn test_get_coindesk_price() {
		let btc_price = BtcPriceLookup::new().get_coindesk_price().await.unwrap();
		assert!(btc_price > FixedU128::zero());
	}

	#[tokio::test]
	async fn test_get_kraken_price() {
		let btc_price = BtcPriceLookup::new().get_kraken_price().await.unwrap();
		assert!(btc_price > FixedU128::zero());
	}

	#[tokio::test]
	async fn test_get_coingecko_price() {
		let btc_price = BtcPriceLookup::new().get_coingecko_price().await.unwrap();
		assert!(btc_price > FixedU128::zero());
	}

	#[tokio::test]
	async fn test_get_coinbase_price() {
		let btc_price = BtcPriceLookup::new().get_coinbase_price().await.unwrap();
		assert!(btc_price > FixedU128::zero());
	}
}
