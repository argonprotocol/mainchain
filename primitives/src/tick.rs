use codec::{Decode, Encode, MaxEncodedLen};
#[cfg(feature = "std")]
use core::time::Duration;
use polkadot_sdk::sp_core::RuntimeDebug;
#[cfg(feature = "std")]
use rsntp::SntpClient;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use std::time::SystemTime;

pub type Tick = u64;
pub const MAX_BLOCKS_PER_TICK: u32 = 60;

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
	pub channel_hold_expiration_ticks: Tick,
	#[codec(skip)]
	ntp_offset_millis: i64,
}

/// Unit type wrapper that represents a tick digest (format is fixed due to compatibility with
/// aura).
#[derive(
	Debug,
	Encode,
	MaxEncodedLen,
	Decode,
	Serialize,
	Deserialize,
	Eq,
	Clone,
	Copy,
	Default,
	Ord,
	Hash,
	TypeInfo,
	PartialOrd,
	PartialEq,
)]
#[repr(transparent)]
pub struct TickDigest(pub Tick);

impl Ticker {
	#[cfg(feature = "std")]
	pub fn start(tick_duration: Duration, channel_hold_expiration_ticks: Tick) -> Self {
		Self {
			tick_duration_millis: tick_duration.as_millis() as u64,
			channel_hold_expiration_ticks,
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

	pub fn new(tick_duration_millis: u64, channel_hold_expiration_ticks: Tick) -> Self {
		Self { tick_duration_millis, channel_hold_expiration_ticks, ntp_offset_millis: 0 }
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
		(now / self.tick_duration_millis) as Tick
	}

	#[cfg(feature = "std")]
	pub fn time_for_tick(&self, tick: Tick) -> u64 {
		let base = tick * self.tick_duration_millis;
		base.checked_add_signed(self.ntp_offset_millis).unwrap_or(base)
	}

	#[cfg(feature = "std")]
	pub fn micros_for_tick(&self, tick: Tick) -> u128 {
		self.time_for_tick(tick) as u128 * 1_000u128
	}

	#[cfg(feature = "std")]
	pub fn duration_after_tick_ends(&self, tick: Tick) -> Duration {
		self.duration_after_tick_starts(tick + 1)
	}

	#[cfg(feature = "std")]
	pub fn duration_after_tick_starts(&self, tick: Tick) -> Duration {
		let current_time = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
			Ok(n) => n.as_micros(),
			Err(_) => 0,
		};
		let tick_time = self.micros_for_tick(tick);
		let micros = current_time.saturating_sub(tick_time);
		Duration::from_micros(micros as u64)
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

	#[cfg(feature = "std")]
	pub fn now_adjusted_to_ntp(&self) -> u64 {
		let now = now();
		now.checked_add_signed(self.ntp_offset_millis).unwrap_or(now)
	}
}

#[cfg(feature = "std")]
fn now() -> u64 {
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
	fn it_should_create_next_ticks() {
		let ticker = Ticker::start(Duration::from_secs(60), 2);
		assert_eq!(ticker.tick_for_time(0), 0);
		assert_eq!(ticker.tick_for_time(15_000), 0);
		assert_eq!(ticker.tick_for_time(60_001), 1);

		let current_tick = ticker.current();
		assert!(current_tick > 28_861_638);
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
