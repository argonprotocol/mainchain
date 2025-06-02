//! Metrics about the notary itself
use crate::rpc_metrics::HISTOGRAM_BUCKETS;
use argon_primitives::Balance;
use polkadot_sdk::*;
use sp_runtime::traits::UniqueSaturatedInto;
use std::time::Instant;
use substrate_prometheus_endpoint::{
	CounterVec, HistogramOpts, HistogramVec, Opts, PrometheusError, Registry, U64, register,
};

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
					&[],
				)?,
				metrics_registry,
			)?,
			notebook_close_time_after_tick: register(
				HistogramVec::new(
					HistogramOpts::new(
						"notary_notebook_close_time_after_tick",
						"Total time [μs] to close notebooks after tick ends (eg, next has begun)",
					)
					.buckets(HISTOGRAM_BUCKETS.to_vec()),
					&[],
				)?,
				metrics_registry,
			)?,
			notebook_bytes: register(
				HistogramVec::new(
					HistogramOpts::new("notary_notebook_bytes", "Size of notebooks").buckets(
						prometheus::exponential_buckets(100.0, 10.0, 10)
							.expect("parameters are always valid values; qed"),
					),
					&[],
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
					&[],
				)?,
				metrics_registry,
			)?,
			notarizations_total: register(
				CounterVec::new(
					Opts::new("notary_notarizations_total", "Number of notarizations processed"),
					&["is_error"],
				)?,
				metrics_registry,
			)?,
			notebooks_total: register(
				CounterVec::new(
					Opts::new("notary_notebooks_total", "Number of notebooks created"),
					&[],
				)?,
				metrics_registry,
			)?,
			balance_changes_total: register(
				CounterVec::new(
					Opts::new("notary_balance_changes_total", "Number of balance changes"),
					&[],
				)?,
				metrics_registry,
			)?,
			block_votes_total: register(
				CounterVec::new(
					Opts::new("notary_block_votes_total", "Number of block votes"),
					&[],
				)?,
				metrics_registry,
			)?,
			domains_total: register(
				CounterVec::new(Opts::new("notary_domains_total", "Number of domains"), &[])?,
				metrics_registry,
			)?,
			tax_total: register(
				CounterVec::new(Opts::new("notary_tax_total", "Total tax"), &[])?,
				metrics_registry,
			)?,
		})
	}

	pub(crate) fn on_notebook_close(
		&self,
		now: Instant,
		start_time: Instant,
		time_after_tick_micros: u128,
		notebook_bytes: usize,
		header_bytes: usize,
	) {
		self.notebooks_total.with_label_values(&[]).inc();
		self.notebook_close_time
			.with_label_values(&[])
			.observe(now.duration_since(start_time).as_micros() as f64);
		self.notebook_close_time_after_tick
			.with_label_values(&[])
			.observe(time_after_tick_micros as f64);
		self.notebook_bytes.with_label_values(&[]).observe(notebook_bytes as f64);
		self.notebook_header_bytes.with_label_values(&[]).observe(header_bytes as f64);
	}

	pub(crate) fn on_notarization(
		&self,
		balance_changes: usize,
		block_votes: usize,
		domains: usize,
		tax: Balance,
	) {
		self.notarizations_total.with_label_values(&["false"]).inc();

		self.balance_changes_total.with_label_values(&[]).inc_by(balance_changes as u64);
		self.block_votes_total.with_label_values(&[]).inc_by(block_votes as u64);
		self.domains_total.with_label_values(&[]).inc_by(domains as u64);
		self.tax_total.with_label_values(&[]).inc_by(tax.unique_saturated_into());
	}

	pub(crate) fn on_notarization_error(&self) {
		self.notarizations_total.with_label_values(&["true"]).inc();
	}
}
