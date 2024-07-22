use anyhow::Result;
use sp_runtime::{traits::One, FixedI128, FixedU128};
use ulixee_client::api::runtime_types::pallet_price_index::PriceIndex;
use ulx_primitives::tick::{Tick, Ticker};

#[allow(dead_code)]
pub struct ArgonPriceLookup {
	pub ticker: Ticker,
	pub use_simulated_schedule: bool,
	pub last_price: FixedU128,
	pub last_price_tick: Tick,
}

impl ArgonPriceLookup {
	pub fn new(
		use_simulated_schedule: bool,
		ticker: &Ticker,
		last_price: Option<PriceIndex>,
	) -> Self {
		Self {
			use_simulated_schedule,
			ticker: ticker.clone(),
			last_price: last_price
				.as_ref()
				.map(|a| FixedU128::from_inner(a.argon_usd_price.0))
				.unwrap_or(FixedU128::from_u32(1)),
			last_price_tick: last_price
				.map(|a| a.tick)
				.unwrap_or(ticker.current().saturating_sub(1)),
		}
	}

	/// Calculates the expected cost of an Argon in USD based on the starting and current U.S. CPI.
	pub fn get_target_price(&self, us_cpi_ratio: FixedI128) -> FixedU128 {
		let cpi_as_u128 = FixedI128::one() + us_cpi_ratio;
		FixedU128::from_inner(cpi_as_u128.into_inner() as u128)
	}

	pub async fn get_argon_price(
		&mut self,
		target_price: FixedU128,
		tick: Tick,
		max_argon_change_per_tick_away_from_target: FixedU128,
	) -> Result<FixedU128> {
		let price = self
			.get_latest_price(target_price, tick, max_argon_change_per_tick_away_from_target)
			.await?;
		self.last_price = price;
		self.last_price_tick = tick;
		Ok(price)
	}

	#[allow(unused_variables)]
	pub async fn get_latest_price(
		&self,
		target_price: FixedU128,
		tick: Tick,
		max_argon_change_per_tick_away_from_target: FixedU128,
	) -> Result<FixedU128> {
		if self.use_simulated_schedule {
			#[cfg(feature = "fast-runtime")]
			{
				return Ok(self.simulate_price_change(
					target_price,
					tick,
					max_argon_change_per_tick_away_from_target,
				))
			}
		}

		// Eventually, we'll want to hit asset hub and moonbeam directly for pricing. Maybe
		// ethereum too if it ends up on there.
		Ok(target_price)
	}
}

#[cfg(feature = "fast-runtime")]
mod dev {
	use crate::argon_price::ArgonPriceLookup;
	use chrono::{TimeZone, Timelike};
	use rand::Rng;
	use sp_runtime::{FixedU128, Saturating};
	use ulx_primitives::tick::Tick;

	impl ArgonPriceLookup {
		pub(crate) fn simulate_price_change(
			&self,
			target_price: FixedU128,
			tick: Tick,
			max_argon_change_per_tick_away_from_target: FixedU128,
		) -> FixedU128 {
			let ticks = if self.last_price_tick == 0 {
				1
			} else {
				tick.saturating_sub(self.last_price_tick) as u64 / self.ticker.tick_duration_millis
			}
			.min(10);
			let mut last_price = self.last_price;
			let tz_offset = chrono::FixedOffset::west_opt(5 * 3600).unwrap();

			let tick_millis = self.ticker.time_for_tick(tick) as i64;
			let est = tz_offset.timestamp_millis_opt(tick_millis).unwrap();
			let one_milligon = FixedU128::from_rational(1, 1000);
			let one_centagon = FixedU128::from_rational(1, 100);

			for _ in 0..ticks {
				match est.hour() {
					0..=6 => {
						// Hold at 1 cent for 15 minutes
						last_price = one_centagon
					},
					7..=8 => {
						// Increase to 1.99 on the minute
						if est.second() < 5 || est.second() > 55 {
							if last_price < FixedU128::from_rational(199, 100) {
								last_price = last_price.saturating_add(one_centagon);
							}
						}
					},
					9..=10 => {
						// Drop back to target
						if last_price > target_price {
							last_price = last_price.saturating_sub(one_centagon);
						}
					},
					11..=13 => {
						match est.minute() {
							// Fluctuate 5 cents up per hour and hold for 15 minutes
							15..=20 => {
								last_price =
									last_price.saturating_add(FixedU128::from_rational(5, 100));
							},
							35..=40 => {
								last_price =
									last_price.saturating_sub(FixedU128::from_rational(5, 100));
							},
							0 | 59 => last_price = target_price,
							_ => {},
						}
					},
					14..=15 => {
						// Randomize price swing for one hour
						let mut rng = rand::thread_rng();
						let direction = rng.gen_range(-1..=1);
						match direction {
							-1 => {
								last_price =
									last_price.saturating_sub(FixedU128::from_rational(5, 100));
							},
							1 => {
								last_price =
									last_price.saturating_add(FixedU128::from_rational(5, 100));
							},
							_ => {},
						}
					},
					16..=18 => {
						// increase 1 milligon per tick
						last_price = last_price.saturating_sub(one_milligon);
					},
					19.. => {
						// Drop 1 cent per tick to 1 cent
						if last_price > one_centagon {
							last_price = last_price.saturating_sub(one_centagon);
						}
					},
				}
			}
			let mut price =
				last_price.clamp(FixedU128::from_rational(1, 1000), FixedU128::from_u32(2));
			let start_price = self.last_price;

			// TODO: how do we clamp this only when cpi is same direction
			if price > start_price {
				price = price
					.min(start_price.saturating_add(max_argon_change_per_tick_away_from_target));
			} else {
				price = price
					.max(start_price.saturating_sub(max_argon_change_per_tick_away_from_target));
			}

			price
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::time::Duration;

	#[test]
	fn test_get_target_price() {
		let ticker = Ticker::start(Duration::from_secs(60));
		let argon_price_lookup = ArgonPriceLookup::new(false, &ticker, None);
		let us_cpi_ratio = FixedI128::from_float(0.00);
		assert_eq!(argon_price_lookup.get_target_price(us_cpi_ratio), FixedU128::from_u32(1));
	}

	#[test]
	fn test_get_target_price_with_cpi() {
		let ticker = Ticker::start(Duration::from_secs(60));
		let argon_price_lookup = ArgonPriceLookup::new(false, &ticker, None);
		let us_cpi_ratio = FixedI128::from_float(0.1);
		assert_eq!(argon_price_lookup.get_target_price(us_cpi_ratio).to_float(), 1.1);
	}

	#[test]
	#[cfg(feature = "fast-runtime")]
	fn can_use_simulated_schedule() {
		let ticker = Ticker::start(Duration::from_secs(60));
		let mut argon_price_lookup = ArgonPriceLookup::new(true, &ticker, None);

		argon_price_lookup.last_price = FixedU128::from_float(1.01);
		argon_price_lookup.last_price_tick = ticker.current();
		let ts = argon_price_lookup.last_price_tick + 1000;
		let price = argon_price_lookup.simulate_price_change(
			FixedU128::from_u32(1),
			ts,
			FixedU128::from_rational(1, 100),
		);
		assert_ne!(price, FixedU128::from_u32(0));
	}
}
