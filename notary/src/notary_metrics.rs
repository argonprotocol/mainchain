//! Metrics about the notary itself

use crate::rpc_metrics::HISTOGRAM_BUCKETS;
use argon_primitives::{tick::Tick, Balance};
use prometheus_endpoint::{
	register, CounterVec, HistogramOpts, HistogramVec, Opts, PrometheusError, Registry, U64,
};
use sp_runtime::traits::UniqueSaturatedInto;
use std::time::Instant;

/// Metrics for the notary processing
#[derive(Debug, Clone)]
pub struct NotaryMetrics {
	/// Histogram over notebook close time.
	notebook_close_time: HistogramVec,
	/// Histogram over notebook time after tick that it finishes
	notebook_close_time_after_tick: HistogramVec,
	/// Size of notebooks
	notebook_bytes: HistogramVec,
	/// Size of notebook headers
	notebook_header_bytes: HistogramVec,
	/// Number of notarizations processed.
	notarizations_total: CounterVec<U64>,
	/// Number of notebooks created
	notebooks_total: CounterVec<U64>,
	/// Balance changes
	balance_changes_total: CounterVec<U64>,
	/// Block votes
	block_votes_total: CounterVec<U64>,
	/// Domains
	domains_total: CounterVec<U64>,
	/// Tax
	tax_total: CounterVec<U64>,
}

impl NotaryMetrics {
	/// Create an instance of metrics
	pub fn new(metrics_registry: &Registry) -> Result<Self, PrometheusError> {
		Ok(Self {
			notebook_close_time: register(
				HistogramVec::new(
					HistogramOpts::new(
						"notary_notebook_close_time",
						"Total time [μs] to close notebooks",
					)
					.buckets(HISTOGRAM_BUCKETS.to_vec()),
					&["tick"],
				)?,
				metrics_registry,
			)?,
			notebook_close_time_after_tick: register(
				HistogramVec::new(
					HistogramOpts::new(
						"notary_notebook_close_time_after_tick",
						"Total time [μs] to close notebooks after tick",
					)
					.buckets(HISTOGRAM_BUCKETS.to_vec()),
					&["tick"],
				)?,
				metrics_registry,
			)?,
			notebook_bytes: register(
				HistogramVec::new(
					HistogramOpts::new("notary_notebook_bytes", "Size of notebooks").buckets(
						prometheus::exponential_buckets(100.0, 10.0, 10)
							.expect("parameters are always valid values; qed"),
					),
					&["tick"],
				)?,
				metrics_registry,
			)?,
			notebook_header_bytes: register(
				HistogramVec::new(
					HistogramOpts::new("notary_notebook_header_bytes", "Size of notebook headers")
						.buckets(
							prometheus::exponential_buckets(100.0, 10.0, 10)
								.expect("parameters are always valid values; qed"),
						),
					&["tick"],
				)?,
				metrics_registry,
			)?,
			notarizations_total: register(
				CounterVec::new(
					Opts::new("notary_notarizations_total", "Number of notarizations processed"),
					&["tick", "is_error"],
				)?,
				metrics_registry,
			)?,
			notebooks_total: register(
				CounterVec::new(
					Opts::new("notary_notebooks_total", "Number of notebooks created"),
					&["tick"],
				)?,
				metrics_registry,
			)?,
			balance_changes_total: register(
				CounterVec::new(
					Opts::new("notary_balance_changes_total", "Number of balance changes"),
					&["tick"],
				)?,
				metrics_registry,
			)?,
			block_votes_total: register(
				CounterVec::new(
					Opts::new("notary_block_votes_total", "Number of block votes"),
					&["tick"],
				)?,
				metrics_registry,
			)?,
			domains_total: register(
				CounterVec::new(Opts::new("notary_domains_total", "Number of domains"), &["tick"])?,
				metrics_registry,
			)?,
			tax_total: register(
				CounterVec::new(Opts::new("notary_tax_total", "Total tax"), &["tick"])?,
				metrics_registry,
			)?,
		})
	}

	pub(crate) fn on_notebook_close(
		&self,
		now: Instant,
		start_time: Instant,
		tick: Tick,
		time_after_tick_micros: u128,
		notebook_bytes: usize,
		header_bytes: usize,
	) {
		let tick = tick.to_string();
		self.notebooks_total.with_label_values(&[&tick]).inc();
		self.notebook_close_time
			.with_label_values(&[&tick])
			.observe(now.duration_since(start_time).as_micros() as f64);
		self.notebook_close_time_after_tick
			.with_label_values(&[&tick])
			.observe(time_after_tick_micros as f64);
		self.notebook_bytes.with_label_values(&[&tick]).observe(notebook_bytes as f64);
		self.notebook_header_bytes
			.with_label_values(&[&tick])
			.observe(header_bytes as f64);
	}

	pub(crate) fn on_notarization(
		&self,
		tick: Tick,
		balance_changes: usize,
		block_votes: usize,
		domains: usize,
		tax: Balance,
	) {
		self.notarizations_total.with_label_values(&[&tick.to_string(), "false"]).inc();

		self.balance_changes_total
			.with_label_values(&[&tick.to_string()])
			.inc_by(balance_changes as u64);
		self.block_votes_total
			.with_label_values(&[&tick.to_string()])
			.inc_by(block_votes as u64);
		self.domains_total
			.with_label_values(&[&tick.to_string()])
			.inc_by(domains as u64);
		self.tax_total
			.with_label_values(&[&tick.to_string()])
			.inc_by(tax.unique_saturated_into());
	}

	pub(crate) fn on_notarization_error(&self, tick: Tick) {
		self.notarizations_total.with_label_values(&[&tick.to_string(), "true"]).inc();
	}
}
