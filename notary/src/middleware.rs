use crate::{metrics::*, server::RpcConfig};
use futures::future::{BoxFuture, FutureExt};
use governor::{
	clock::{Clock, DefaultClock, QuantaClock},
	middleware::NoOpMiddleware,
	state::{InMemoryState, NotKeyed},
	Jitter, Quota,
};
use jsonrpsee::{
	server::middleware::rpc::RpcServiceT,
	types::{ErrorObject, Id, Request},
	MethodResponse,
};
use prometheus::Registry;
use prometheus_endpoint::init_prometheus;
use std::{
	net::{Ipv4Addr, SocketAddr},
	num::NonZeroU32,
	sync::Arc,
	time::{Duration, Instant},
};

const MAX_JITTER: Duration = Duration::from_millis(50);
const MAX_RETRIES: usize = 10;

pub(crate) fn register_prometheus_metrics(
	registry: Registry,
	rpc_config: &RpcConfig,
) -> anyhow::Result<Option<MiddlewareLayer>> {
	let metrics = RpcMetrics::new(Some(&registry))?.map(|m| {
		MiddlewareLayer::new()
			.with_metrics(Metrics::new(m, "http"))
			.with_rate_limit_per_minute(rpc_config.rate_limit_per_minute)
	});
	let prometheus_port = rpc_config.prometheus_port.unwrap_or(9116);
	tokio::spawn(async move {
		// serve the prometheus metrics on the given port so that it can be read
		let _ = init_prometheus(
			SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), prometheus_port),
			registry,
		)
		.await;
	});

	Ok(metrics)
}

/// JSON-RPC middleware layer.
#[derive(Debug, Clone, Default)]
pub struct MiddlewareLayer {
	rate_limit: Option<RateLimit>,
	metrics: Option<Metrics>,
}

impl MiddlewareLayer {
	/// Create an empty MiddlewareLayer.
	pub fn new() -> Self {
		Self::default()
	}

	/// Enable new rate limit middleware enforced per minute.
	pub fn with_rate_limit_per_minute(self, n: NonZeroU32) -> Self {
		Self { rate_limit: Some(RateLimit::per_minute(n)), metrics: self.metrics }
	}

	/// Enable metrics middleware.
	pub fn with_metrics(self, metrics: Metrics) -> Self {
		Self { rate_limit: self.rate_limit, metrics: Some(metrics) }
	}

	/// Register a new websocket connection.
	pub fn ws_connect(&self) {
		if let Some(m) = self.metrics.as_ref() {
			m.ws_connect()
		}
	}

	/// Register that a websocket connection was closed.
	pub fn ws_disconnect(&self, now: Instant) {
		if let Some(m) = self.metrics.as_ref() {
			m.ws_disconnect(now)
		}
	}
}

impl<S> tower::Layer<S> for MiddlewareLayer {
	type Service = Middleware<S>;

	fn layer(&self, service: S) -> Self::Service {
		Middleware { service, rate_limit: self.rate_limit.clone(), metrics: self.metrics.clone() }
	}
}

/// JSON-RPC middleware that handles metrics
/// and rate-limiting.
///
/// These are part of the same middleware
/// because the metrics needs to know whether
/// a call was rate-limited or not because
/// it will impact the roundtrip for a call.
pub struct Middleware<S> {
	service: S,
	rate_limit: Option<RateLimit>,
	metrics: Option<Metrics>,
}

impl<'a, S> RpcServiceT<'a> for Middleware<S>
where
	S: Send + Sync + RpcServiceT<'a> + Clone + 'static,
{
	type Future = BoxFuture<'a, MethodResponse>;

	fn call(&self, req: Request<'a>) -> Self::Future {
		let now = Instant::now();

		if let Some(m) = self.metrics.as_ref() {
			m.on_call(&req)
		}

		let service = self.service.clone();
		let rate_limit = self.rate_limit.clone();
		let metrics = self.metrics.clone();

		async move {
			let mut is_rate_limited = false;

			if let Some(limit) = rate_limit.as_ref() {
				let mut attempts = 0;
				let jitter = Jitter::up_to(MAX_JITTER);

				loop {
					if attempts >= MAX_RETRIES {
						return reject_too_many_calls(req.id);
					}

					if let Err(rejected) = limit.inner.check() {
						tokio::time::sleep(jitter + rejected.wait_time_from(limit.clock.now()))
							.await;
					} else {
						break;
					}

					is_rate_limited = true;
					attempts += 1;
				}
			}

			let rp = service.call(req.clone()).await;
			if let Some(m) = metrics.as_ref() {
				m.on_response(&req, &rp, is_rate_limited, now)
			}

			rp
		}
		.boxed()
	}
}

fn reject_too_many_calls(id: Id) -> MethodResponse {
	MethodResponse::error(id, ErrorObject::owned(-32999, "RPC rate limit exceeded", None::<()>))
}

type RateLimitInner = governor::RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// Rate limit.
#[derive(Debug, Clone)]
pub struct RateLimit {
	pub(crate) inner: Arc<RateLimitInner>,
	pub(crate) clock: QuantaClock,
}

impl RateLimit {
	/// Create a new `RateLimit` per minute.
	pub fn per_minute(n: NonZeroU32) -> Self {
		let clock = QuantaClock::default();
		Self {
			inner: Arc::new(RateLimitInner::direct_with_clock(Quota::per_minute(n), clock.clone())),
			clock,
		}
	}
}
