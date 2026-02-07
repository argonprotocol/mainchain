use crate::{aux_client::AuxKey, aux_data::AuxData, block_creator::ProposalMeta};
use argon_primitives::{
	Balance, NotaryId,
	prelude::sp_runtime::Saturating,
	tick::{Tick, Ticker},
};
use codec::{Decode, Encode};
use polkadot_sdk::*;
use sc_client_api::AuxStore;
use sp_arithmetic::traits::UniqueSaturatedInto;
use std::{sync::Arc, time::Instant};
use substrate_prometheus_endpoint::{
	CounterVec, GaugeVec, HistogramOpts, HistogramVec, Opts, PrometheusError, Registry, U64,
	prometheus, register,
};

#[derive(Debug, Clone, Decode, Encode, Default)]
pub struct BlockMetrics {
	#[codec(compact)]
	pub compute_blocks_created: u64,
	#[codec(compact)]
	pub compute_blocks_created_w_notebooks: u64,
	#[codec(compact)]
	pub vote_blocks_created: u64,
	#[codec(compact)]
	pub vote_blocks_created_w_notebooks: u64,
	#[codec(compact)]
	pub finalized_blocks_created: u64,
	#[codec(compact)]
	pub mined_ownership_tokens_total: u64,
	#[codec(compact)]
	pub mined_argons_total: u64,
}

/// Metrics for the node consensus engine
#[derive(Clone)]
pub struct ConsensusMetrics<C: AuxStore> {
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
	/// Total ownership tokens mined
	mined_ownership_tokens_total: CounterVec<U64>,
	/// Total argons mined
	mined_argons_total: CounterVec<U64>,
	/// Total finalized blocks created
	finalized_blocks_created_total: CounterVec<U64>,
	aux_data: AuxData<BlockMetrics, C>,
}

impl<C: AuxStore> ConsensusMetrics<C> {
	/// Create an instance of metrics
	pub fn new(metrics_registry: &Registry, aux_client: Arc<C>) -> Result<Self, PrometheusError> {
		let aux_data = AuxData::<BlockMetrics, C>::new(aux_client, AuxKey::BlockMetrics);
		let start_data = aux_data.get();
		let start = Self {
			aux_data,
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
			mined_ownership_tokens_total: register(
				CounterVec::new(
					Opts::new("argon_mined_ownership_tokens_total", "Total ownership tokens mined"),
					&[],
				)?,
				metrics_registry,
			)?,
			mined_argons_total: register(
				CounterVec::new(Opts::new("argon_mined_argons_total", "Total argons mined"), &[])?,
				metrics_registry,
			)?,
			finalized_blocks_created_total: register(
				CounterVec::new(
					Opts::new(
						"argon_finalized_blocks_created_total",
						"Total finalized blocks created",
					),
					&[],
				)?,
				metrics_registry,
			)?,
		};
		if start_data.compute_blocks_created > 0 {
			start
				.compute_blocks_created_total
				.with_label_values(&["false"])
				.inc_by(start_data.compute_blocks_created);
		}
		if start_data.compute_blocks_created_w_notebooks > 0 {
			start
				.compute_blocks_created_total
				.with_label_values(&["true"])
				.inc_by(start_data.compute_blocks_created_w_notebooks);
		}
		if start_data.vote_blocks_created > 0 {
			start
				.vote_blocks_created_total
				.with_label_values(&["false"])
				.inc_by(start_data.vote_blocks_created);
		}
		if start_data.vote_blocks_created_w_notebooks > 0 {
			start
				.vote_blocks_created_total
				.with_label_values(&["true"])
				.inc_by(start_data.vote_blocks_created_w_notebooks);
		}
		if start_data.finalized_blocks_created > 0 {
			start
				.finalized_blocks_created_total
				.with_label_values(&[])
				.inc_by(start_data.finalized_blocks_created);
		}
		if start_data.mined_ownership_tokens_total > 0 {
			start
				.mined_ownership_tokens_total
				.with_label_values(&[])
				.inc_by(start_data.mined_ownership_tokens_total);
		}
		if start_data.mined_argons_total > 0 {
			start
				.mined_argons_total
				.with_label_values(&[])
				.inc_by(start_data.mined_argons_total);
		}
		Ok(start)
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
		self.aux_data
			.mutate(|data| {
				if proposal_meta.is_compute {
					if proposal_meta.notebooks > 0 {
						data.compute_blocks_created_w_notebooks =
							data.compute_blocks_created_w_notebooks.saturating_add(1);
					} else {
						data.compute_blocks_created = data.compute_blocks_created.saturating_add(1);
					}
				} else if proposal_meta.notebooks > 0 {
					data.vote_blocks_created_w_notebooks =
						data.vote_blocks_created_w_notebooks.saturating_add(1);
				} else {
					data.vote_blocks_created = data.vote_blocks_created.saturating_add(1);
				}
			})
			.inspect_err(|e| log::error!("Error updating block metrics: {e:?}"))
			.ok();
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

	pub(crate) fn record_finalized_block(&self, ownership_tokens: Balance, argons: Balance) {
		let ownership_tokens: u64 = ownership_tokens.unique_saturated_into();
		let argons: u64 = argons.unique_saturated_into();
		self.mined_ownership_tokens_total
			.with_label_values(&[])
			.inc_by(ownership_tokens);
		self.mined_argons_total.with_label_values(&[]).inc_by(argons);
		self.finalized_blocks_created_total.with_label_values(&[]).inc();

		self.aux_data
			.mutate(|data| {
				data.finalized_blocks_created.saturating_accrue(1);
				data.mined_ownership_tokens_total.saturating_accrue(ownership_tokens);
				data.mined_argons_total.saturating_accrue(argons);
			})
			.inspect_err(|e| log::error!("Error updating block metrics: {e:?}"))
			.ok();
	}
}
