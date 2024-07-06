use anyhow::anyhow;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer};
use sp_runtime::{FixedI128, FixedU128};

#[allow(dead_code)]

pub fn to_fixed_u128(value: FixedI128) -> FixedU128 {
	FixedU128::from_inner(value.into_inner() as u128)
}

pub fn to_fixed_i128(value: FixedU128) -> FixedI128 {
	FixedI128::from_inner(value.into_inner() as i128)
}

pub fn parse_f64<'de, D>(deserializer: D) -> anyhow::Result<f64, D::Error>
where
	D: Deserializer<'de>,
{
	let s = String::deserialize(deserializer)?;
	s.parse::<f64>().map_err(serde::de::Error::custom)
}

pub fn parse_date(date: &str, formats: Vec<&str>) -> anyhow::Result<DateTime<Utc>> {
	for format in formats {
		let Some(date) = NaiveDate::parse_from_str(date, format).ok() else {
			continue;
		};
		return date
			.and_hms_opt(0, 0, 0)
			.ok_or(anyhow!("Failed to parse date"))
			.map(|d| d.and_utc());
	}
	Err(anyhow!("Failed to parse date"))
}
