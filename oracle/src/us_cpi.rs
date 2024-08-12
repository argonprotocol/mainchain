use crate::{
	us_cpi_schedule::{load_cpi_schedule, CpiSchedule},
	utils::{parse_date, parse_f64, to_fixed_i128},
};
use anyhow::{anyhow, ensure, Result};
use argon_primitives::tick::{Tick, Ticker};
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::One, FixedI128, FixedU128};
use std::env;
#[cfg(test)]
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, Instant};
use url::Url;

const MAX_SCHEDULE_DAYS: u32 = 35;
const MIN_SCHEDULE_DAYS: u32 = 28;

const ONE_HOUR: u64 = 60 * 60;
const ONE_DAY: u64 = 24 * ONE_HOUR;

lazy_static! {
	/// Banding of data is created to prevent data manipulation from breaking the system.
	/// To analyze the cpi data, the full historical data was downloaded from the CPI here:
	/// https://data.bls.gov/timeseries/CUUR0000SA0?years_option=all_years
	///
	/// These were loaded into excel, and then the a difference was made for each month from the
	/// previous (for Feb-Dec, `=((C2 - B2) / B2) * 100`, for Jan, `=((B3 - M2) /M2) * 100`)
	///
	/// The 5th and 95th percentile were calculated for all differences using the following (the
	/// columns are adjacent to the original values):
	/// - bottom 5%: `=PERCENTILE.INC(P2:AA113, 0.05)`
	/// - top 5%: `=PERCENTILE.INC(P2:AA113, 0.95)`
	static ref CPI_5TH_PERCENTILE: FixedI128 = FixedI128::from_float(-0.7092); // -0.709219858
	static ref CPI_95TH_PERCENTILE: FixedI128 = FixedI128::from_float(1.2429); // 1.242880338
	// May 2024
	static ref BASELINE_CPI: FixedU128 = FixedU128::from_float(314.069);
}

pub struct UsCpiRetriever {
	pub schedule: Vec<CpiSchedule>,
	pub current_cpi_release_tick: Tick,
	pub current_cpi_duration_ticks: Tick,
	pub current_cpi_end_value: FixedU128,
	pub current_cpi_ref_month: DateTime<Utc>,
	pub previous_us_cpi: FixedU128,
	pub cpi_change_per_tick: FixedI128,
	pub last_schedule_check: Instant,
	pub last_cpi_check: Instant,
	pub ticker: Ticker,
}

impl UsCpiRetriever {
	pub async fn new(ticker: &Ticker) -> Result<Self> {
		let schedule = load_cpi_schedule().await?;
		let cpis = get_raw_cpis().await?;
		let current = cpis.first().ok_or(anyhow!("No CPI data"))?;
		let previous = cpis.get(1).ok_or(anyhow!("No previous CPI data"))?;
		let current_cpi_release_tick =
			Self::get_release_date_tick(ticker, &schedule, current.ref_month)
				.ok_or(anyhow!("No release date found for current CPI"))?;
		let current_cpi_duration_ticks =
			Self::ticks_to_next_cpi(ticker, &schedule, current.ref_month);

		let mut entry = Self {
			schedule,
			current_cpi_duration_ticks,
			current_cpi_release_tick,
			current_cpi_end_value: current.value,
			current_cpi_ref_month: current.ref_month,
			previous_us_cpi: previous.value,
			last_schedule_check: Instant::now(),
			ticker: *ticker,
			last_cpi_check: Instant::now(),
			cpi_change_per_tick: FixedI128::from_u32(0),
		};
		entry.cpi_change_per_tick = entry.calculate_cpi_change_per_tick();

		Ok(entry)
	}

	pub async fn refresh(&mut self) -> Result<()> {
		let now = Instant::now();
		if now.duration_since(self.last_schedule_check) > Duration::from_secs(10 * ONE_DAY) {
			self.schedule = load_cpi_schedule().await?;
			self.last_schedule_check = now;
		}
		if now.duration_since(self.last_cpi_check) > Duration::from_secs(ONE_HOUR) {
			let next_cpi = get_raw_cpi().await?;
			if next_cpi.ref_month == self.current_cpi_ref_month {
				return Ok(());
			}
			self.previous_us_cpi = self.current_cpi_end_value;
			self.current_cpi_end_value = next_cpi.value;
			self.current_cpi_ref_month = next_cpi.ref_month;
			self.current_cpi_duration_ticks =
				Self::ticks_to_next_cpi(&self.ticker, &self.schedule, self.current_cpi_ref_month);
			self.current_cpi_release_tick = Self::get_release_date_tick(
				&self.ticker,
				&self.schedule,
				self.current_cpi_ref_month,
			)
			.ok_or(anyhow!("No release date found for current CPI"))?;
			self.last_cpi_check = now;
			self.cpi_change_per_tick = self.calculate_cpi_change_per_tick();
		}
		Ok(())
	}

	/// Returns the ratio of the current CPI to the baseline CPI (minus 1).
	pub fn get_us_cpi_ratio(&self, tick: Tick) -> FixedI128 {
		let current_cpi = self.calculate_smoothed_us_cpi_ratio(tick);
		let baseline = to_fixed_i128(*BASELINE_CPI);

		(current_cpi / baseline) - FixedI128::one()
	}

	fn calculate_smoothed_us_cpi_ratio(&self, tick: Tick) -> FixedI128 {
		let ticks = FixedI128::from_u32(self.ticks_since_last_cpi(tick));
		let previous_cpi = to_fixed_i128(self.previous_us_cpi);
		(self.cpi_change_per_tick * ticks) + previous_cpi
	}

	fn ticks_since_last_cpi(&self, tick: Tick) -> Tick {
		tick.saturating_sub(self.current_cpi_release_tick)
	}

	fn calculate_cpi_change_per_tick(&self) -> FixedI128 {
		let ticks = FixedI128::from_float(self.current_cpi_duration_ticks as f64);

		Self::get_clamped_cpi_change(self.previous_us_cpi, self.current_cpi_end_value) / ticks
	}

	fn get_clamped_cpi_change(start: FixedU128, end: FixedU128) -> FixedI128 {
		let cpi_diff = to_fixed_i128(end) - to_fixed_i128(start);

		let clamped_diff = cpi_diff.clamp(*CPI_5TH_PERCENTILE, *CPI_95TH_PERCENTILE);
		if cpi_diff != clamped_diff {
			tracing::warn!(
				"CPI change was outside historical 5-95% band (diff={:?}, flattened to {:?})",
				cpi_diff,
				clamped_diff
			);
		}
		clamped_diff
	}

	fn ticks_to_next_cpi(
		ticker: &Ticker,
		schedule: &[CpiSchedule],
		ref_month: DateTime<Utc>,
	) -> Tick {
		let Some(index) = schedule.iter().position(|s| s.ref_month == ref_month) else {
			let duration = Duration::from_secs(MIN_SCHEDULE_DAYS as u64 * ONE_DAY);
			return ticker.ticks_for_duration(duration);
		};
		if index == schedule.len() - 1 {
			let duration = Duration::from_secs(MIN_SCHEDULE_DAYS as u64 * ONE_DAY);
			return ticker.ticks_for_duration(duration);
		}

		let release_date = schedule[index].release_date;
		let next_release_date = schedule[index + 1].release_date;

		let days = next_release_date
			.signed_duration_since(release_date)
			.num_days()
			.min(MAX_SCHEDULE_DAYS as i64)
			.max(MIN_SCHEDULE_DAYS as i64);

		let days_duration = Duration::from_secs(days as u64 * ONE_DAY);
		ticker.ticks_for_duration(days_duration)
	}

	fn get_release_date_tick(
		ticker: &Ticker,
		schedule: &[CpiSchedule],
		ref_month: DateTime<Utc>,
	) -> Option<Tick> {
		let index = schedule.iter().position(|s| s.ref_month == ref_month)?;
		let date = schedule[index].release_date;
		let millis = date.timestamp_millis() as u64;
		Some(ticker.tick_for_time(millis))
	}
}

#[cfg(test)]
lazy_static! {
	static ref MOCK_RAW_CPIS: Arc<Mutex<Option<Vec<RawCpiValue>>>> = Arc::new(Mutex::new(None));
}

async fn get_raw_cpis() -> Result<Vec<RawCpiValue>> {
	#[cfg(test)]
	{
		let mut mock = MOCK_RAW_CPIS.lock().unwrap();
		if let Some(results) = mock.take() {
			*mock = Some(results.clone());
			return Ok(results.clone());
		}
	}
	let mut request_url =
		Url::parse("https://api.bls.gov/publicAPI/v2/timeseries/data/CUUR0000SA0")?;

	if let Ok(key) = env::var("BLS_API_KEY") {
		request_url.query_pairs_mut().append_pair("registrationkey", &key);
	}

	let client = Client::new();
	let resp_price = client
		.get(request_url)
		.header("Content-Type", "application/json")
		.send()
		.await?
		.text()
		.await?;

	parse_cpi_results(&resp_price).map_err(|e| {
		tracing::error!("Failed to get CPI data: {:?}. JSON is {}", e, resp_price);
		e
	})
}

fn parse_cpi_results(resp_price: &str) -> Result<Vec<RawCpiValue>> {
	let resp_price: CpiResult = serde_json::from_str(resp_price)?;
	ensure!(resp_price.status == "REQUEST_SUCCEEDED", "Failed to get CPI data");
	let resp_price = resp_price.results.series.first().ok_or(anyhow!("No series data"))?;
	ensure!(!resp_price.data.is_empty(), "Failed to get CPI data");

	let mut cpis = vec![];
	for data in &resp_price.data {
		let price = FixedU128::from_float(data.value);
		cpis.push(RawCpiValue { value: price, ref_month: data.parse_period()? });
	}
	Ok(cpis)
}

async fn get_raw_cpi() -> Result<RawCpiValue> {
	#[cfg(test)]
	{
		let mut mock = MOCK_RAW_CPIS.lock().unwrap();
		if let Some(results) = mock.take() {
			*mock = Some(results.clone());
			return Ok(results[0].clone());
		}
	}
	let mut request_url =
		Url::parse("https://api.bls.gov/publicAPI/v2/timeseries/data/CUUR0000SA0?latest=true")?;

	if let Ok(key) = env::var("BLS_API_KEY") {
		request_url.query_pairs_mut().append_pair("registrationkey", &key);
	}

	let client = Client::new();
	let resp_price = client
		.get(request_url)
		.header("Content-Type", "application/json")
		.send()
		.await?
		.text()
		.await?;
	let result = parse_cpi_results(&resp_price)?;
	Ok(result.first().ok_or(anyhow!("No CPI data"))?.clone())
}

#[derive(Clone, Debug)]
pub struct RawCpiValue {
	pub value: FixedU128,
	pub ref_month: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CpiResult {
	pub status: String,
	#[serde(rename = "Results")]
	pub results: CpiResultset,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CpiResultset {
	pub series: Vec<CpiResultSeriesEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CpiResultSeriesEntry {
	pub data: Vec<CpiResultSeriesData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CpiResultSeriesData {
	pub year: String,
	pub period: String,
	#[serde(rename = "periodName")]
	pub period_name: String, // "January"
	#[serde(deserialize_with = "parse_f64")]
	pub value: f64,
}

impl CpiResultSeriesData {
	pub fn parse_period(&self) -> Result<DateTime<Utc>> {
		let date = format!("1 {} {}", self.period_name, self.year);
		parse_date(&date, vec!["%d %B %Y"])
	}
}

#[cfg(test)]
pub(crate) async fn use_mock_cpi_values(cpi_offsets: Vec<f64>) {
	let schedule = load_cpi_schedule().await.expect("should load schedule");

	let mut mock = MOCK_RAW_CPIS.lock().unwrap();
	*mock = Some(
		cpi_offsets
			.into_iter()
			.enumerate()
			.map(|(idx, a)| RawCpiValue {
				value: FixedU128::from_inner(
					(to_fixed_i128(*BASELINE_CPI) + FixedI128::from_float(a)).into_inner() as u128,
				),
				ref_month: schedule[idx].ref_month,
			})
			.collect::<Vec<_>>(),
	);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_parse_raw_cpis() {
		let json = r#"{"status":"REQUEST_SUCCEEDED","responseTime":175,"message":[],"Results":{
"series":
[{"seriesID":"CUUR0000SA0","data":[{"year":"2024","period":"M05","periodName":"May","latest":"true","value":"314.069","footnotes":[{}]},{"year":"2024","period":"M04","periodName":"April","value":"313.548","footnotes":[{}]},{"year":"2024","period":"M03","periodName":"March","value":"312.332","footnotes":[{}]},{"year":"2024","period":"M02","periodName":"February","value":"310.326","footnotes":[{}]},{"year":"2024","period":"M01","periodName":"January","value":"308.417","footnotes":[{}]},{"year":"2023","period":"M12","periodName":"December","value":"306.746","footnotes":[{}]},{"year":"2023","period":"M11","periodName":"November","value":"307.051","footnotes":[{}]},{"year":"2023","period":"M10","periodName":"October","value":"307.671","footnotes":[{}]},{"year":"2023","period":"M09","periodName":"September","value":"307.789","footnotes":[{}]},{"year":"2023","period":"M08","periodName":"August","value":"307.026","footnotes":[{}]},{"year":"2023","period":"M07","periodName":"July","value":"305.691","footnotes":[{}]},{"year":"2023","period":"M06","periodName":"June","value":"305.109","footnotes":[{}]},{"year":"2023","period":"M05","periodName":"May","value":"304.127","footnotes":[{}]},{"year":"2023","period":"M04","periodName":"April","value":"303.363","footnotes":[{}]},{"year":"2023","period":"M03","periodName":"March","value":"301.836","footnotes":[{}]},{"year":"2023","period":"M02","periodName":"February","value":"300.840","footnotes":[{}]},{"year":"2023","period":"M01","periodName":"January","value":"299.170","footnotes":[{}]},{"year":"2022","period":"M12","periodName":"December","value":"296.797","footnotes":[{}]},{"year":"2022","period":"M11","periodName":"November","value":"297.711","footnotes":[{}]},{"year":"2022","period":"M10","periodName":"October","value":"298.012","footnotes":[{}]},{"year":"2022","period":"M09","periodName":"September","value":"296.808","footnotes":[{}]},{"year":"2022","period":"M08","periodName":"August","value":"296.171","footnotes":[{}]},{"year":"2022","period":"M07","periodName":"July","value":"296.276","footnotes":[{}]},{"year":"2022","period":"M06","periodName":"June","value":"296.311","footnotes":[{}]},{"year":"2022","period":"M05","periodName":"May","value":"292.296","footnotes":[{}]},{"year":"2022","period":"M04","periodName":"April","value":"289.109","footnotes":[{}]},{"year":"2022","period":"M03","periodName":"March","value":"287.504","footnotes":[{}]},{"year":"2022","period":"M02","periodName":"February","value":"283.716","footnotes":[{}]},{"year":"2022","period":"M01","periodName":"January","value":"281.148","footnotes":[{}]}]}]
}}"#;
		let cpis = parse_cpi_results(json).unwrap();
		assert_eq!(cpis.len(), 29);
		assert!(cpis[0].value >= FixedU128::from_u32(200));
		assert!(cpis[0].ref_month > cpis[1].ref_month);
	}

	#[tokio::test]
	#[ignore]
	async fn test_can_get_raw_cpi() {
		let cpi = get_raw_cpi().await.unwrap();
		assert!(cpi.value >= FixedU128::from_u32(200));
	}

	#[test]
	fn test_can_smooth_out_cpi() {
		let previous_cpi = *BASELINE_CPI;
		let start_time = parse_date("1 April 2024", vec!["%d %B %Y"]).unwrap();
		let ticker = Ticker::new(60_000, start_time.timestamp_millis() as u64, 2);

		let mut retriever = UsCpiRetriever {
			schedule: vec![],
			current_cpi_end_value: previous_cpi + FixedU128::from_float(100.0),
			current_cpi_duration_ticks: ticker
				.ticks_for_duration(Duration::from_secs(28 * ONE_DAY)),
			current_cpi_ref_month: parse_date("1 April 2024", vec!["%d %B %Y"]).unwrap(),
			current_cpi_release_tick: ticker.tick_for_time(
				parse_date("May 15, 2024", vec!["%b %d, %Y", "%b. %d, %Y"])
					.unwrap()
					.timestamp_millis() as u64,
			),
			previous_us_cpi: previous_cpi,
			cpi_change_per_tick: FixedI128::from_u32(0),
			last_schedule_check: Instant::now(),
			last_cpi_check: Instant::now(),
			ticker,
		};
		retriever.schedule = vec![
			CpiSchedule {
				release_date: parse_date("May 15, 2024", vec!["%b %d, %Y", "%b. %d, %Y"]).unwrap(),
				ref_month: parse_date("1 April 2024", vec!["%d %B %Y"]).unwrap(),
			},
			CpiSchedule {
				release_date: parse_date("Jun. 12, 2024", vec!["%b %d, %Y", "%b. %d, %Y"]).unwrap(),
				ref_month: parse_date("1 May 2024", vec!["%d %B %Y"]).unwrap(),
			},
		];
		assert_eq!(
			UsCpiRetriever::ticks_to_next_cpi(
				&ticker,
				&retriever.schedule,
				retriever.current_cpi_ref_month
			),
			ticker.ticks_for_duration(Duration::from_secs(ONE_DAY * 28))
		);
		// should clamp the difference to the 5-95% band
		assert_eq!(
			UsCpiRetriever::get_clamped_cpi_change(
				retriever.previous_us_cpi,
				retriever.current_cpi_end_value
			),
			*CPI_95TH_PERCENTILE
		);

		// should clamp to minimum
		assert_eq!(
			UsCpiRetriever::get_clamped_cpi_change(
				retriever.previous_us_cpi,
				previous_cpi - FixedU128::from_float(10.0)
			),
			*CPI_5TH_PERCENTILE
		);

		retriever.current_cpi_end_value = previous_cpi + FixedU128::one();
		assert_eq!(
			UsCpiRetriever::get_clamped_cpi_change(
				retriever.previous_us_cpi,
				retriever.current_cpi_end_value
			),
			FixedI128::one()
		);

		let ticks = retriever.current_cpi_duration_ticks;
		retriever.cpi_change_per_tick = retriever.calculate_cpi_change_per_tick();
		assert_eq!(retriever.cpi_change_per_tick, FixedI128::one().div(FixedI128::from_u32(ticks)));

		assert_eq!(retriever.ticks_since_last_cpi(retriever.current_cpi_release_tick + 1), 1);
		assert_eq!(
			retriever.get_us_cpi_ratio(retriever.current_cpi_release_tick + 10),
			((to_fixed_i128(retriever.previous_us_cpi) +
				(FixedI128::from_u32(10) * retriever.cpi_change_per_tick)) /
				to_fixed_i128(retriever.previous_us_cpi)) -
				FixedI128::one()
		);
	}

	#[test]
	fn calculates_intervals_elapsed() {
		let start_time = parse_date("1 April 2024", vec!["%d %B %Y"]).unwrap();
		let ticker = Ticker::new(60_000, start_time.timestamp_millis() as u64, 2);
		let retriever = UsCpiRetriever {
			schedule: vec![],
			current_cpi_end_value: FixedU128::from_u32(300),
			current_cpi_duration_ticks: ticker
				.ticks_for_duration(Duration::from_secs(28 * ONE_DAY)),
			current_cpi_release_tick: ticker.tick_for_time(
				parse_date("May 15, 2024", vec!["%b %d, %Y", "%b. %d, %Y"])
					.unwrap()
					.timestamp_millis() as u64,
			),
			current_cpi_ref_month: parse_date("1 April 2024", vec!["%d %B %Y"]).unwrap(),
			previous_us_cpi: FixedU128::from_u32(200),
			cpi_change_per_tick: FixedI128::from_u32(0),
			last_schedule_check: Instant::now(),
			last_cpi_check: Instant::now(),
			ticker,
		};
		let timestamp =
			parse_date("15 May 2024", vec!["%d %B %Y"]).unwrap().timestamp_millis() as u64;
		let tick = ticker.tick_for_time(timestamp);
		assert_eq!(retriever.ticks_since_last_cpi(tick), 0);
		assert_eq!(retriever.ticks_since_last_cpi(tick + 1), 1);
		assert_eq!(retriever.ticks_since_last_cpi(tick + 7), 7);
		assert_eq!(retriever.ticks_since_last_cpi(tick + 30), 30);
		assert_eq!(retriever.ticks_since_last_cpi(tick + 60), 60);
		assert_eq!(retriever.ticks_since_last_cpi(tick + 60 * 24), 24 * 60);
	}
}
