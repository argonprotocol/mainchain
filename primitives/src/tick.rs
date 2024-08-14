#[cfg(feature = "std")]
use core::time::Duration;

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use rsntp::SntpClient;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::RuntimeDebug;

pub type Tick = u32;

#[derive(
	Encode,
	Decode,
	Serialize,
	Deserialize,
	RuntimeDebug,
	TypeInfo,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct Ticker {
	#[codec(compact)]
	pub tick_duration_millis: u64,
	#[codec(compact)]
	pub genesis_utc_time: u64,
	#[codec(compact)]
	pub escrow_expiration_ticks: Tick,
	#[codec(skip)]
	ntp_offset_millis: i64,
}

impl Ticker {
	#[cfg(feature = "std")]
	pub fn start(tick_duration: Duration, escrow_expiration_ticks: Tick) -> Self {
		let current_time = now();
		let offset = current_time % tick_duration.as_millis() as u64;
		let genesis_utc_time = current_time - offset;
		Self {
			tick_duration_millis: tick_duration.as_millis() as u64,
			genesis_utc_time,
			escrow_expiration_ticks,
			ntp_offset_millis: 0,
		}
	}

	#[cfg(feature = "std")]
	pub async fn lookup_ntp_offset(&mut self, ntp_host: &str) -> Result<(), String> {
		let client = SntpClient::new();
		let result = client.synchronize(ntp_host).map_err(|e| e.to_string())?;
		let offset = result.clock_offset();
		let mut offset_millis = offset.abs_as_std_duration().unwrap_or_default().as_millis() as i64;
		if offset.signum() < 0 {
			offset_millis *= -1;
		}

		self.ntp_offset_millis = offset_millis;
		Ok(())
	}

	pub fn new(
		tick_duration_millis: u64,
		genesis_utc_time: u64,
		escrow_expiration_ticks: Tick,
	) -> Self {
		Self {
			tick_duration_millis,
			genesis_utc_time,
			escrow_expiration_ticks,
			ntp_offset_millis: 0,
		}
	}

	#[cfg(feature = "std")]
	pub fn ticks_for_duration(&self, duration: Duration) -> Tick {
		(duration.as_millis() / self.tick_duration_millis as u128) as Tick
	}

	#[cfg(feature = "std")]
	pub fn current(&self) -> Tick {
		self.tick_for_time(now())
	}

	pub fn tick_for_time(&self, timestamp_millis: u64) -> Tick {
		let now = timestamp_millis
			.checked_add_signed(self.ntp_offset_millis)
			.unwrap_or(timestamp_millis);
		let offset = now.saturating_sub(self.genesis_utc_time);
		let tick = offset / self.tick_duration_millis;
		tick as Tick
	}

	pub fn time_for_tick(&self, tick: Tick) -> u64 {
		let base = self.genesis_utc_time + (tick as u64 * self.tick_duration_millis);
		base.checked_add_signed(self.ntp_offset_millis).unwrap_or(base)
	}

	#[cfg(feature = "std")]
	pub fn duration_to_next_tick(&self) -> Duration {
		let now = now();
		let current_tick = self.current();
		let next_tick_time = self.time_for_tick(current_tick + 1);
		let duration_to_next_tick = next_tick_time.saturating_sub(now);
		Duration::from_millis(duration_to_next_tick)
	}

	#[cfg(feature = "std")]
	pub fn next(&self) -> Tick {
		self.current() + 1
	}
}

#[cfg(feature = "std")]
fn now() -> u64 {
	use std::time::SystemTime;

	let current_time: u128 = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
		Ok(n) => n.as_millis(),
		Err(_) => 0,
	};
	current_time as u64
}

#[cfg(test)]
mod test {
	use std::time::Duration;

	use crate::tick::Ticker;

	#[test]
	fn it_should_calculate_genesis() {
		use chrono::{DurationRound, Utc};

		let ticker = Ticker::start(Duration::from_secs(1), 2);
		let beginning_of_second =
			Utc::now().duration_trunc(chrono::Duration::try_seconds(1).unwrap()).unwrap();
		assert_eq!(ticker.genesis_utc_time, beginning_of_second.timestamp_millis() as u64);

		let ticker = Ticker::start(Duration::from_secs(60), 2);
		let beginning_of_minute =
			Utc::now().duration_trunc(chrono::Duration::try_minutes(1).unwrap()).unwrap();
		assert_eq!(ticker.genesis_utc_time, beginning_of_minute.timestamp_millis() as u64);
	}

	#[test]
	fn it_should_create_next_ticks() {
		let ticker = Ticker::start(Duration::from_secs(30), 2);

		let start = ticker.genesis_utc_time;
		let current_tick = ticker.current();
		assert_eq!(current_tick, 0);

		assert_eq!(ticker.tick_for_time(start + 15_000), 0);
		assert_eq!(ticker.tick_for_time(start + 30_001), 1);
	}

	#[tokio::test]
	#[ignore]
	async fn it_should_calculate_ntp_offset() {
		let mut ticker = Ticker::start(Duration::from_secs(30), 2);

		let time_for_next_tick = ticker.time_for_tick(2);
		ticker.lookup_ntp_offset("pool.ntp.org").await.expect("should not die");
		assert_ne!(ticker.ntp_offset_millis, 0);
		assert_ne!(time_for_next_tick, ticker.time_for_tick(2))
	}
}
