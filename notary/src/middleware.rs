use crate::{
	rpc_metrics::*,
	server::{RpcConfig, RpcRateLimitMode},
};
use futures::future::{BoxFuture, FutureExt};
use governor::{Jitter, Quota, clock::Clock, middleware::NoOpMiddleware};
use jsonrpsee::{
	MethodResponse,
	core::server::ConnectionId,
	server::{HttpRequest, middleware::rpc::RpcServiceT},
	types::{ErrorObject, Id, Request},
};
use polkadot_sdk::*;
use prometheus::Registry;
use std::{
	net::{Ipv4Addr, SocketAddr},
	num::NonZeroU32,
	sync::Arc,
	time::{Duration, Instant},
};
use substrate_prometheus_endpoint::init_prometheus;

const MAX_JITTER: Duration = Duration::from_millis(50);
const MAX_RETRIES: usize = 10;
const MAX_CLIENT_RATE_LIMIT_KEY_LEN: usize = 128;

pub(crate) fn register_prometheus_metrics(
	registry: Registry,
	rpc_config: &RpcConfig,
) -> anyhow::Result<Option<MiddlewareLayer>> {
	let metrics = RpcMetrics::new(Some(&registry))?.map(|m| {
		MiddlewareLayer::new()
			.with_metrics(Metrics::new(m, "http"))
			.with_rate_limit_per_minute(
				rpc_config.rate_limit_per_minute,
				rpc_config.rate_limit_mode,
			)
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
	pub fn with_rate_limit_per_minute(
		self,
		n: NonZeroU32,
		rate_limit_mode: RpcRateLimitMode,
	) -> Self {
		Self { rate_limit: Some(RateLimit::per_minute(n, rate_limit_mode)), metrics: self.metrics }
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
		let conn_id = req.extensions().get::<ConnectionId>().copied();
		let client_key = get_client_rate_limit_key(&req);

		async move {
			let mut is_rate_limited = false;

			if let Some(limit) = rate_limit.as_ref() {
				let jitter = Jitter::up_to(MAX_JITTER);

				// Check the request key against the configured limiter.
				// If over quota, wait for the rejected duration plus jitter and retry.
				// We keep retrying up to MAX_RETRIES before returning an explicit rate-limit error.
				let mut attempts = 0;
				while attempts < MAX_RETRIES {
					let Some(retry_after) = limit.retry_after(conn_id, &client_key, jitter) else {
						break;
					};
					is_rate_limited = true;
					attempts += 1;
					tokio::time::sleep(retry_after).await;
				}

				if attempts >= MAX_RETRIES {
					return reject_too_many_calls(req.id);
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct ClientRateLimitKey(String);

impl ClientRateLimitKey {
	fn from_ip(ip: &str) -> Self {
		Self::from_parts("ip:", ip)
	}

	fn from_connection(connection_id: usize) -> Self {
		Self(format!("connection:{connection_id}"))
	}

	fn unknown() -> Self {
		Self("unknown".into())
	}

	fn from_parts(prefix: &str, value: &str) -> Self {
		let mut key = String::with_capacity(prefix.len() + value.len());
		key.push_str(prefix);
		key.push_str(value);
		if key.len() > MAX_CLIENT_RATE_LIMIT_KEY_LEN {
			key.truncate(MAX_CLIENT_RATE_LIMIT_KEY_LEN);
		}
		Self(key)
	}
}

#[derive(Debug, Clone, Default)]
pub struct ClientRateLimitKeyLayer;

impl<S> tower::Layer<S> for ClientRateLimitKeyLayer {
	type Service = ClientRateLimitKeyMiddleware<S>;

	fn layer(&self, service: S) -> Self::Service {
		ClientRateLimitKeyMiddleware { service }
	}
}

#[derive(Clone)]
pub struct ClientRateLimitKeyMiddleware<S> {
	service: S,
}

impl<S, B> tower::Service<HttpRequest<B>> for ClientRateLimitKeyMiddleware<S>
where
	S: tower::Service<HttpRequest<B>>,
{
	type Response = S::Response;
	type Error = S::Error;
	type Future = S::Future;

	fn poll_ready(
		&mut self,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Result<(), Self::Error>> {
		self.service.poll_ready(cx)
	}

	fn call(&mut self, mut request: HttpRequest<B>) -> Self::Future {
		let client_key = get_client_rate_limit_key_from_http_request(&request);
		request.extensions_mut().insert(client_key);
		self.service.call(request)
	}
}

type PerConnectionRateLimitInner = governor::DefaultKeyedRateLimiter<ConnectionId, NoOpMiddleware>;
type PerIpRateLimitInner = governor::DefaultKeyedRateLimiter<ClientRateLimitKey, NoOpMiddleware>;

#[derive(Debug, Clone)]
enum RateLimitInner {
	PerConnection(Arc<PerConnectionRateLimitInner>),
	PerIp(Arc<PerIpRateLimitInner>),
}

/// Rate limit.
#[derive(Debug, Clone)]
pub struct RateLimit {
	inner: RateLimitInner,
}

impl RateLimit {
	/// Create a new `RateLimit` per minute.
	pub fn per_minute(n: NonZeroU32, rate_limit_mode: RpcRateLimitMode) -> Self {
		let inner = match rate_limit_mode {
			RpcRateLimitMode::PerConnection => RateLimitInner::PerConnection(Arc::new(
				PerConnectionRateLimitInner::keyed(Quota::per_minute(n)),
			)),
			RpcRateLimitMode::PerIp =>
				RateLimitInner::PerIp(Arc::new(PerIpRateLimitInner::keyed(Quota::per_minute(n)))),
		};

		Self { inner }
	}

	fn retry_after(
		&self,
		conn_id: Option<ConnectionId>,
		client_key: &ClientRateLimitKey,
		jitter: Jitter,
	) -> Option<Duration> {
		match &self.inner {
			RateLimitInner::PerConnection(inner) => {
				let conn_id = conn_id.unwrap_or(ConnectionId(0));
				inner
					.check_key(&conn_id)
					.err()
					.map(|rejected| jitter + rejected.wait_time_from(inner.clock().now()))
			},
			RateLimitInner::PerIp(inner) => inner
				.check_key(client_key)
				.err()
				.map(|rejected| jitter + rejected.wait_time_from(inner.clock().now())),
		}
	}
}

fn get_client_rate_limit_key(req: &Request<'_>) -> ClientRateLimitKey {
	req.extensions()
		.get::<ClientRateLimitKey>()
		.cloned()
		.or_else(|| {
			req.extensions()
				.get::<ConnectionId>()
				.map(|id| ClientRateLimitKey::from_connection(id.0))
		})
		.unwrap_or_else(ClientRateLimitKey::unknown)
}

fn get_client_rate_limit_key_from_http_request<B>(request: &HttpRequest<B>) -> ClientRateLimitKey {
	get_forwarded_ip(request)
		.or_else(|| get_header_ip(request, "x-real-ip"))
		.or_else(|| get_header_ip(request, "cf-connecting-ip"))
		.or_else(|| {
			request
				.extensions()
				.get::<ConnectionId>()
				.map(|id| ClientRateLimitKey::from_connection(id.0))
		})
		.unwrap_or_else(ClientRateLimitKey::unknown)
}

fn get_forwarded_ip<B>(request: &HttpRequest<B>) -> Option<ClientRateLimitKey> {
	let forwarded_for = get_header_value(request, "x-forwarded-for")?;
	let first_ip = forwarded_for.split(',').next().map(str::trim).filter(|ip| !ip.is_empty())?;
	Some(ClientRateLimitKey::from_ip(first_ip))
}

fn get_header_ip<B>(request: &HttpRequest<B>, header: &str) -> Option<ClientRateLimitKey> {
	get_header_value(request, header).map(|ip| ClientRateLimitKey::from_ip(&ip))
}

fn get_header_value<B>(request: &HttpRequest<B>, header: &str) -> Option<String> {
	request
		.headers()
		.get(header)
		.and_then(|value| value.to_str().ok())
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
	use super::*;
	use jsonrpsee::server::HttpRequest;

	#[test]
	fn extracts_first_forwarded_ip() {
		let mut request = HttpRequest::new(());
		request
			.headers_mut()
			.insert("x-forwarded-for", "203.0.113.9, 198.51.100.3".parse().expect("valid header"));

		let key = get_client_rate_limit_key_from_http_request(&request);
		assert_eq!(key, ClientRateLimitKey::from_ip("203.0.113.9"));
	}

	#[test]
	fn falls_back_to_connection_id() {
		let mut request = HttpRequest::new(());
		request.extensions_mut().insert(ConnectionId(42));

		let key = get_client_rate_limit_key_from_http_request(&request);
		assert_eq!(key, ClientRateLimitKey::from_connection(42));
	}

	#[test]
	fn falls_back_to_unknown() {
		let request = HttpRequest::new(());

		let key = get_client_rate_limit_key_from_http_request(&request);
		assert_eq!(key, ClientRateLimitKey::unknown());
	}
}
