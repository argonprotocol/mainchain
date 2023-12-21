use codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::RuntimeDebug;

use crate::prod_or_fast;
#[cfg(feature = "std")]
use sp_std::time::Duration;

pub type Tick = u32;

#[derive(
	Encode, Decode, Serialize, Deserialize, RuntimeDebug, TypeInfo, Clone, Copy, PartialEq, Eq,
)]
pub struct Ticker {
	pub tick_duration_millis: u64,
	pub genesis_utc_time: u64,
}

pub const TICK_MILLIS: u64 = prod_or_fast!(60_000, 2_000);

impl Ticker {
	#[cfg(feature = "std")]
	pub fn start(tick_duration: Duration) -> Self {
		let current_time = now();
		let offset = current_time % tick_duration.as_millis() as u64;
		let genesis_utc_time = current_time - offset;
		Self {
			tick_duration_millis: tick_duration.as_millis() as u64,
			genesis_utc_time: genesis_utc_time as u64,
		}
	}

	pub fn new(tick_duration_millis: u64, genesis_utc_time: u64) -> Self {
		Self { tick_duration_millis, genesis_utc_time }
	}

	#[cfg(feature = "std")]
	pub fn current(&self) -> Tick {
		self.tick_for_time(now())
	}

	pub fn tick_for_time(&self, timestamp_millis: u64) -> Tick {
		let offset = timestamp_millis - self.genesis_utc_time;
		let tick = offset / self.tick_duration_millis;
		tick as Tick
	}

	pub fn time_for_tick(&self, tick: Tick) -> u64 {
		self.genesis_utc_time + (tick as u64 * self.tick_duration_millis)
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

#[test]
fn it_should_calculate_genesis() {
	use chrono::{DurationRound, Utc};

	let ticker = Ticker::start(Duration::from_secs(1));
	let beginning_of_second = Utc::now().duration_trunc(chrono::Duration::seconds(1)).unwrap();
	assert_eq!(ticker.genesis_utc_time, beginning_of_second.timestamp_millis() as u64);

	let ticker = Ticker::start(Duration::from_secs(60));
	let beginning_of_minute = Utc::now().duration_trunc(chrono::Duration::minutes(1)).unwrap();
	assert_eq!(ticker.genesis_utc_time, beginning_of_minute.timestamp_millis() as u64);
}

#[test]
fn it_should_create_next_ticks() {
	let ticker = Ticker::start(Duration::from_secs(30));

	let start = ticker.genesis_utc_time;
	let current_tick = ticker.current();
	assert_eq!(current_tick, 0);

	assert_eq!(ticker.tick_for_time(start + 15_000), 0);
	assert_eq!(ticker.tick_for_time(start + 30_001), 1);
}
