use crate::block_creator::ProposalMeta;
use argon_primitives::{
	tick::{Tick, Ticker},
	NotaryId,
};
use prometheus_endpoint::{
	prometheus, register, CounterVec, GaugeVec, HistogramOpts, HistogramVec, Opts, PrometheusError,
	Registry, U64,
};
use std::time::Instant;

/// Metrics for the node consensus engine
#[derive(Debug, Clone)]
pub struct ConsensusMetrics {
	/// Histogram over compute hashes need
	compute_hashes_total: HistogramVec,
	/// Blocks created with compute
	compute_blocks_created_total: CounterVec<U64>,
	/// Number of times we had to reset the compute state
	compute_resets_from_notebooks: CounterVec<U64>,
	/// Blocks created with votes
	vote_blocks_created_total: CounterVec<U64>,
	/// Block time created after tick
	block_time_after_tick: HistogramVec,
	/// Notebook queue depth
	notebook_queue_depth: GaugeVec<U64>,
	/// Notebook time after tick notification was received
	notebook_notification_after_tick_time: HistogramVec,
	/// Notebook processed time after tick
	notebook_audited_after_tick_time: HistogramVec,
	/// Notebook total processing time
	notebook_processing_time: HistogramVec,
}

impl ConsensusMetrics {
	/// Create an instance of metrics
	pub fn new(metrics_registry: &Registry) -> Result<Self, PrometheusError> {
		Ok(Self {
			compute_hashes_total: register(
				HistogramVec::new(
					HistogramOpts::new(
						"argon_compute_hashes_total",
						"Total number of compute hashes applied to a block",
					)
					.buckets(prometheus::exponential_buckets(10.0, 10.0, 12)?),
					&[],
				)?,
				metrics_registry,
			)?,
			compute_blocks_created_total: register(
				CounterVec::new(
					Opts::new("argon_compute_blocks_created_total", "Blocks created with compute"),
					&["has_notebooks"],
				)?,
				metrics_registry,
			)?,
			compute_resets_from_notebooks: register(
				CounterVec::new(
					Opts::new(
						"argon_compute_resets_from_notebooks",
						"Number of times we reset the compute state due to new notebooks",
					),
					&[],
				)?,
				metrics_registry,
			)?,
			vote_blocks_created_total: register(
				CounterVec::new(
					Opts::new("argon_vote_blocks_created_total", "Blocks created with votes"),
					&["has_notebooks"],
				)?,
				metrics_registry,
			)?,
			block_time_after_tick: register(
				HistogramVec::new(
					HistogramOpts::new(
						"argon_blocks_time_after_tick",
						"Total time [μs] after a tick that a block is created",
					)
					.buckets(prometheus::exponential_buckets(100_000.0, 2.5, 10)?),
					&[],
				)?,
				metrics_registry,
			)?,
			notebook_queue_depth: register(
				GaugeVec::new(
					Opts::new("argon_notebook_queue_depth", "Notebook queue depth"),
					&["notary_id"],
				)?,
				metrics_registry,
			)?,
			notebook_notification_after_tick_time: register(
				HistogramVec::new(
					HistogramOpts::new(
						"argon_notebook_notification_after_tick_time",
						"Total time [μs] after a tick that a notebook notification was received",
					)
					.buckets(prometheus::exponential_buckets(10_000.0, 2.5, 10)?),
					&["notary_id"],
				)?,
				metrics_registry,
			)?,
			notebook_audited_after_tick_time: register(
				HistogramVec::new(
					HistogramOpts::new(
						"argon_notebook_audited_after_tick_time",
						"Total time [μs] after a tick that a notebook was audited",
					)
					.buckets(prometheus::exponential_buckets(100_000.0, 3.0, 12)?),
					&["notary_id"],
				)?,
				metrics_registry,
			)?,
			notebook_processing_time: register(
				HistogramVec::new(
					HistogramOpts::new(
						"argon_notebook_processing_time",
						"Total time [μs] to process a notebook",
					)
					.buckets(prometheus::exponential_buckets(100_000.0, 1.3, 20)?),
					&["notary_id"],
				)?,
				metrics_registry,
			)?,
		})
	}

	pub(crate) fn record_compute_hashes(&self, hashes: u64) {
		self.compute_hashes_total.with_label_values(&[]).observe(hashes as f64);
	}

	pub(crate) fn on_block_created(&self, ticker: &Ticker, proposal_meta: &ProposalMeta) {
		let expected_tick_time = ticker.duration_after_tick_starts(proposal_meta.tick);
		let time_after_tick = expected_tick_time.as_micros() as f64;
		let has_notebooks = if proposal_meta.notebooks > 0 { "true" } else { "false" };
		self.block_time_after_tick.with_label_values(&[]).observe(time_after_tick);
		if proposal_meta.is_compute {
			self.compute_blocks_created_total.with_label_values(&[has_notebooks]).inc();
		} else {
			self.vote_blocks_created_total.with_label_values(&[has_notebooks]).inc();
		}
	}

	pub(crate) fn did_reset_compute_for_notebooks(&self) {
		self.compute_resets_from_notebooks.with_label_values(&[]).inc();
	}

	pub(crate) fn notebook_processed(
		&self,
		notary_id: NotaryId,
		tick: Tick,
		enqueue_time: Instant,
		ticker: &Ticker,
	) {
		let time = enqueue_time.elapsed().as_micros() as f64;
		let expected_tick_time = ticker.micros_for_tick(tick);
		let time_after_tick = enqueue_time.elapsed().as_micros().saturating_sub(expected_tick_time);

		self.notebook_audited_after_tick_time
			.with_label_values(&[&notary_id.to_string()])
			.observe(time_after_tick as f64);
		self.notebook_processing_time
			.with_label_values(&[&notary_id.to_string()])
			.observe(time);
	}

	pub(crate) fn notebook_notification_received(
		&self,
		notary_id: NotaryId,
		tick: Tick,
		ticker: &Ticker,
	) {
		let duration_after_tick = ticker.duration_after_tick_ends(tick);

		let time_after_tick = duration_after_tick.as_micros() as f64;
		self.notebook_notification_after_tick_time
			.with_label_values(&[&notary_id.to_string()])
			.observe(time_after_tick);
	}

	pub(crate) fn record_queue_depth(&self, notary_id: NotaryId, depth: u64) {
		self.notebook_queue_depth
			.with_label_values(&[&notary_id.to_string()])
			.set(depth);
	}
}
