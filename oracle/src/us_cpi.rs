use crate::{
	us_cpi_schedule::{CpiSchedule, load_cpi_schedule},
	utils::{parse_date, parse_f64, to_fixed_i128},
};
use anyhow::{Result, anyhow, bail, ensure};
use argon_primitives::tick::{Tick, Ticker};
use chrono::{DateTime, Utc};
use directories::BaseDirs;
use lazy_static::lazy_static;
use polkadot_sdk::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sp_runtime::{FixedI128, FixedPointNumber, FixedU128, traits::One};
#[cfg(test)]
use std::sync::{Arc, Mutex};
use std::{env, fs::File, path::PathBuf};
use tokio::time::Duration;
use tracing::info;
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
	/// These were loaded into Excel, and then the difference for each month was calculated from the
	/// previous (for Feb-Dec, `=((C2 - B2) / B2) * 100`, for Jan, `=((B3 - M2) /M2) * 100`)
	///
	/// The 5th and 95th percentile were calculated for all differences using the following (the
	/// columns are adjacent to the original values):
	/// - bottom 5%: `=PERCENTILE.INC(P2:AA113, 0.05)`
	/// - top 5%: `=PERCENTILE.INC(P2:AA113, 0.95)`
	static ref CPI_5TH_PERCENTILE: FixedI128 = FixedI128::from_float(-0.7092); // -0.709219858
	static ref CPI_95TH_PERCENTILE: FixedI128 = FixedI128::from_float(1.2429); // 1.242880338
	static ref BASELINE_CPI: FixedU128 = FixedU128::from_float(315.605); // CPI for 2024-12-01
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UsCpiRetriever {
	pub schedule: Vec<CpiSchedule>,
	pub current_cpi_release_tick: Tick,
	pub current_cpi_duration_ticks: Tick,
	pub current_cpi_end_value: FixedU128,
	pub current_cpi_ref_month: DateTime<Utc>,
	pub previous_us_cpi: FixedU128,
	pub cpi_change_per_tick: FixedI128,
	pub last_schedule_check: DateTime<Utc>,
	pub cpi_backoff: Option<DateTime<Utc>>,
	pub last_cpi_check: DateTime<Utc>,
	#[serde(skip)]
	pub ticker: Ticker,
}

impl UsCpiRetriever {
	fn get_state_file_path() -> Result<PathBuf> {
		if let Ok(path) = env::var("ORACLE_CPI_CACHE_PATH") {
			return Ok(PathBuf::from(path));
		}

		let state_file = BaseDirs::new()
			.ok_or(anyhow!("No home directory"))?
			.cache_dir()
			.join("argon/oracle/cpi_state.json");
		Ok(state_file)
	}

	pub fn save_state(&self) -> Result<()> {
		let state_file = Self::get_state_file_path()?;
		if !state_file.exists() {
			let dir = state_file.parent().ok_or(anyhow!("No parent directory to build"))?;
			std::fs::create_dir_all(dir)?;
		}
		let file = File::create(state_file)?;
		serde_json::to_writer(file, &self)?;
		Ok(())
	}

	pub fn load_state(ticker: &Ticker) -> Result<Self> {
		match env::var("ORACLE_CPI_CACHE_DISABLED").unwrap_or_default().as_str() {
			"1" | "true" => return Err(anyhow!("Cache disabled")),
			_ => {},
		}
		if cfg!(test) {
			return Err(anyhow!("Cache disabled in tests"));
		}

		let state_file = Self::get_state_file_path()?;
		if state_file.exists() {
			let file = File::open(state_file)?;
			let mut retriever: Self = serde_json::from_reader(file)?;
			retriever.ticker = *ticker;
			Ok(retriever)
		} else {
			Err(anyhow!("State file not found"))
		}
	}

	pub async fn new(ticker: &Ticker) -> Result<Self> {
		if let Ok(retriever) = Self::load_state(ticker) {
			return Ok(retriever);
		}

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
			last_schedule_check: Utc::now(),
			ticker: *ticker,
			last_cpi_check: Utc::now(),
			cpi_change_per_tick: FixedI128::from_u32(0),
			cpi_backoff: None,
		};
		entry.cpi_change_per_tick = entry.calculate_cpi_change_per_tick();
		entry.save_state()?;

		Ok(entry)
	}

	pub async fn refresh(&mut self) -> Result<()> {
		let now = Utc::now();
		let mut should_save = false;
		if now.signed_duration_since(self.last_schedule_check).num_seconds() as u64 > 10 * ONE_DAY {
			self.schedule = load_cpi_schedule().await?;
			self.last_schedule_check = now;
			should_save = true;
		}
		if self.cpi_backoff.is_some() && now > self.cpi_backoff.unwrap() {
			self.cpi_backoff = None;
		}
		if self.cpi_backoff.is_none() &&
			now.signed_duration_since(self.last_cpi_check).num_seconds() as u64 > ONE_HOUR
		{
			let next_cpi = get_raw_cpi().await.inspect_err(|e| {
				if e.to_string().contains("REQUEST_NOT_PROCESSED") {
					info!("Failed to get CPI data. Backing off for 1 hour");
					self.cpi_backoff = Some(now + Duration::from_secs(ONE_HOUR));
				}
			})?;
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
			should_save = true;
		}
		if should_save {
			self.save_state()?;
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
		let ticks = FixedI128::saturating_from_integer(self.ticks_since_last_cpi(tick));
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
	if resp_price.contains("REQUEST_NOT_PROCESSED") {
		bail!("REQUEST_NOT_PROCESSED");
	}
	let resp_price: CpiResult = serde_json::from_str(resp_price)?;
	ensure!(resp_price.status == "REQUEST_SUCCEEDED", "Failed to get CPI data");
	let resp_price = resp_price.results.series.first().ok_or(anyhow!("No series data"))?;
	ensure!(!resp_price.data.is_empty(), "Failed to get CPI data");

	let mut cpis = vec![];
	for data in &resp_price.data {
		if data.period_name == "Annual" {
			continue;
		}
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

	let result = get_raw_cpis().await?;
	ensure!(!result.is_empty(), "No CPI data");
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
	let base_cpi = *BASELINE_CPI;
	*mock = Some(
		cpi_offsets
			.into_iter()
			.enumerate()
			.map(|(idx, a)| RawCpiValue {
				value: FixedU128::from_inner(
					(to_fixed_i128(base_cpi) + FixedI128::from_float(a)).into_inner() as u128,
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
	async fn test_ignores_annual() {
		let json = r#"{"status":"REQUEST_SUCCEEDED","responseTime":162,"message":["No Data Available for Series CUUR0000SA0 Year: 2025"],"Results":{
"series":
[{"seriesID":"CUUR0000SA0","data":[{"year":"2024","period":"M13","periodName":"Annual","latest":"true","value":"313.689","footnotes":[{}]}]}]
}}"#;
		let cpis = parse_cpi_results(json).unwrap();
		assert!(cpis.is_empty());
	}

	#[tokio::test]
	#[ignore]
	async fn test_can_get_raw_cpi() {
		let cpi = get_raw_cpi().await.unwrap();
		println!("CPI: {:?}", cpi);
		assert!(cpi.value >= FixedU128::from_u32(200));
	}

	#[test]
	fn test_can_smooth_out_cpi() {
		let previous_cpi = *BASELINE_CPI;
		let ticker = Ticker::new(60_000, 2);

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
			last_schedule_check: Utc::now(),
			last_cpi_check: Utc::now(),
			cpi_backoff: None,
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
		assert_eq!(
			retriever.cpi_change_per_tick,
			FixedI128::one().div(FixedI128::saturating_from_integer(ticks))
		);

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
		let ticker = Ticker::new(60_000, 2);
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
			last_schedule_check: Utc::now(),
			last_cpi_check: Utc::now(),
			cpi_backoff: None,
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

	#[test]
	fn test_cpi_cache() {
		let cpi: UsCpiRetriever  = serde_json::from_str(r#"{
			"schedule":[{"ref_month":"2024-10-01T00:00:00Z","release_date":"2024-11-13T00:00:00Z"},{"ref_month":"2024-11-01T00:00:00Z","release_date":"2024-12-11T00:00:00Z"},{"ref_month":"2024-12-01T00:00:00Z","release_date":"2025-01-15T00:00:00Z"},{"ref_month":"2025-01-01T00:00:00Z","release_date":"2025-02-12T00:00:00Z"},{"ref_month":"2025-02-01T00:00:00Z","release_date":"2025-03-12T00:00:00Z"},{"ref_month":"2025-03-01T00:00:00Z","release_date":"2025-04-10T00:00:00Z"},{"ref_month":"2025-04-01T00:00:00Z","release_date":"2025-05-13T00:00:00Z"},{"ref_month":"2025-05-01T00:00:00Z","release_date":"2025-06-11T00:00:00Z"},{"ref_month":"2025-06-01T00:00:00Z","release_date":"2025-07-15T00:00:00Z"},{"ref_month":"2025-07-01T00:00:00Z","release_date":"2025-08-12T00:00:00Z"},{"ref_month":"2025-08-01T00:00:00Z","release_date":"2025-09-11T00:00:00Z"},{"ref_month":"2025-09-01T00:00:00Z","release_date":"2025-10-15T00:00:00Z"},{"ref_month":"2025-10-01T00:00:00Z","release_date":"2025-11-13T00:00:00Z"},{"ref_month":"2025-11-01T00:00:00Z","release_date":"2025-12-10T00:00:00Z"}],
			"current_cpi_release_tick":28948320,
			"current_cpi_duration_ticks":40320,
			"current_cpi_end_value":"315605000000000032768",
			"current_cpi_ref_month":"2024-12-01T00:00:00Z",
			"previous_us_cpi":"315492999999999967232",
			"cpi_change_per_tick":"2777777777779",
			"last_schedule_check":"2025-01-16T01:33:05.613573128Z",
			"last_cpi_check":"2025-01-16T01:33:05.613573667Z"}"#
		).unwrap();
		let x = cpi.calculate_smoothed_us_cpi_ratio(28948321);
		assert_eq!(x, FixedI128::from_inner(315492999999999967232 + 2777777777779));

		let cpi: UsCpiRetriever  = serde_json::from_str(r#"{
			"schedule":[{"ref_month":"2024-10-01T00:00:00Z","release_date":"2024-11-13T00:00:00Z"},{"ref_month":"2024-11-01T00:00:00Z","release_date":"2024-12-11T00:00:00Z"},{"ref_month":"2024-12-01T00:00:00Z","release_date":"2025-01-15T00:00:00Z"},{"ref_month":"2025-01-01T00:00:00Z","release_date":"2025-02-12T00:00:00Z"},{"ref_month":"2025-02-01T00:00:00Z","release_date":"2025-03-12T00:00:00Z"},{"ref_month":"2025-03-01T00:00:00Z","release_date":"2025-04-10T00:00:00Z"},{"ref_month":"2025-04-01T00:00:00Z","release_date":"2025-05-13T00:00:00Z"},{"ref_month":"2025-05-01T00:00:00Z","release_date":"2025-06-11T00:00:00Z"},{"ref_month":"2025-06-01T00:00:00Z","release_date":"2025-07-15T00:00:00Z"},{"ref_month":"2025-07-01T00:00:00Z","release_date":"2025-08-12T00:00:00Z"},{"ref_month":"2025-08-01T00:00:00Z","release_date":"2025-09-11T00:00:00Z"},{"ref_month":"2025-09-01T00:00:00Z","release_date":"2025-10-15T00:00:00Z"},{"ref_month":"2025-10-01T00:00:00Z","release_date":"2025-11-13T00:00:00Z"},{"ref_month":"2025-11-01T00:00:00Z","release_date":"2025-12-10T00:00:00Z"}],
			"current_cpi_release_tick":28948320,
			"current_cpi_duration_ticks":40320,
			"current_cpi_end_value":"315492999999999967232",
			"current_cpi_ref_month":"2024-12-01T00:00:00Z",
			"previous_us_cpi":"315492999999999967232",
			"cpi_change_per_tick":"0",
			"last_schedule_check":"2025-01-16T01:33:05.613573128Z",
			"last_cpi_check":"2025-01-16T01:33:05.613573667Z"}"#
		).unwrap();
		let x = cpi.calculate_smoothed_us_cpi_ratio(28949321);
		assert_eq!(x, FixedI128::from_inner(315492999999999967232));
	}
}
