use argon_primitives::inherents::BitcoinUtxoSync;
use polkadot_sdk::substrate_prometheus_endpoint::{
	prometheus, register, CounterVec, Gauge, HistogramOpts, HistogramVec, Opts, PrometheusError,
	Registry, U64,
};
use std::time::Instant;

/// Metrics for the bitcoin utxos
#[derive(Debug, Clone)]
pub struct BitcoinMetrics {
	/// Time to verify bitcoin utxos
	bitcoin_utxos_verify_time: HistogramVec,
	/// Number of bitcoin utxos being tracked
	bitcoin_utxos_tracked_total: Gauge<U64>,
	/// Amount of utxos being tracked
	bitcoin_utxos_satoshis_total: Gauge<U64>,
	/// Bitcoins spent
	bitcoin_utxos_spent_total: CounterVec<U64>,
	/// Bitcoins verified
	bitcoin_utxos_verified_total: CounterVec<U64>,
}

impl BitcoinMetrics {
	/// Create an instance of metrics
	pub fn new(metrics_registry: &Registry) -> Result<Self, PrometheusError> {
		Ok(Self {
			bitcoin_utxos_verify_time: register(
				HistogramVec::new(
					HistogramOpts::new(
						"argon_bitcoin_utxos_verify_time",
						"Total time [μs] to verify all bitcoin utxos",
					)
					.buckets(prometheus::exponential_buckets(10.0, 5.0, 12)?),
					&[],
				)?,
				metrics_registry,
			)?,
			bitcoin_utxos_tracked_total: register(
				Gauge::new(
					"argon_bitcoin_utxos_tracked_total",
					"Number of bitcoin utxos being tracked",
				)?,
				metrics_registry,
			)?,
			bitcoin_utxos_satoshis_total: register(
				Gauge::new("argon_bitcoin_utxos_satoshis_total", "Amount of utxos being tracked")?,
				metrics_registry,
			)?,
			bitcoin_utxos_spent_total: register(
				CounterVec::new(
					Opts::new("argon_bitcoin_utxos_spent_total", "Bitcoins spent"),
					&[],
				)?,
				metrics_registry,
			)?,
			bitcoin_utxos_verified_total: register(
				CounterVec::new(
					Opts::new("argon_bitcoin_utxos_verified_total", "Bitcoins verified"),
					&[],
				)?,
				metrics_registry,
			)?,
		})
	}

	pub fn track(
		&self,
		sync_state: &BitcoinUtxoSync,
		count: u64,
		satoshis: u64,
		start_time: Instant,
	) {
		self.bitcoin_utxos_verified_total
			.with_label_values(&[])
			.inc_by(sync_state.verified.len() as u64);
		self.bitcoin_utxos_spent_total
			.with_label_values(&[])
			.inc_by(sync_state.spent.len() as u64);
		let elapsed = start_time.elapsed().as_micros() as f64;
		self.bitcoin_utxos_verify_time.with_label_values(&[]).observe(elapsed);
		self.bitcoin_utxos_tracked_total.set(count);
		self.bitcoin_utxos_satoshis_total.set(satoshis);
	}
}
