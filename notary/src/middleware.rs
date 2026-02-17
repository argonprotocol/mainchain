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
	types::{ErrorObject, Request},
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
				rpc_config.rate_limit_max_slowdowns,
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
	rate_limit_max_slowdowns: usize,
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
		rate_limit_max_slowdowns: usize,
	) -> Self {
		Self {
			rate_limit: Some(RateLimit::per_minute(n, rate_limit_mode)),
			rate_limit_max_slowdowns,
			metrics: self.metrics,
		}
	}

	/// Enable metrics middleware.
	pub fn with_metrics(self, metrics: Metrics) -> Self {
		Self {
			rate_limit: self.rate_limit,
			rate_limit_max_slowdowns: self.rate_limit_max_slowdowns,
			metrics: Some(metrics),
		}
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
		Middleware {
			service,
			rate_limit: self.rate_limit.clone(),
			rate_limit_max_slowdowns: self.rate_limit_max_slowdowns,
			metrics: self.metrics.clone(),
		}
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
	rate_limit_max_slowdowns: usize,
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
		let rate_limit_max_slowdowns = self.rate_limit_max_slowdowns;
		let client_key = req.extensions().get::<ClientRateLimitKey>().cloned();
		let connection_id = req.extensions().get::<ConnectionId>().copied();
		let connection_key = connection_id.map(|conn| ClientRateLimitKey::from_connection(conn.0));

		async move {
			let mut is_rate_limited = false;

			if let Some(limit) = rate_limit.as_ref() {
				let jitter = Jitter::up_to(MAX_JITTER);

				// Check the request key against the configured limiter.
				// If over quota, wait for the rejected duration plus jitter and retry.
				// We keep delaying up to the configured threshold before returning an explicit
				// rate-limit error.
				let mut rate_limit_retries = 0;
				loop {
					let rate_limit_retry_after = match &limit.inner {
						RateLimitInner::PerConnection(inner) => {
							let Some(connection_id) = connection_id else {
								return MethodResponse::error(
									req.id,
									ErrorObject::owned(
										-32998,
										"RPC request is missing ConnectionId for rate limiting",
										None::<()>,
									),
								);
							};
							inner.check_key(&connection_id).err().map(|rejected| {
								jitter + rejected.wait_time_from(inner.clock().now())
							})
						},
						RateLimitInner::PerIp(inner) => {
							let Some(client_key) = client_key.as_ref().or(connection_key.as_ref())
							else {
								return MethodResponse::error(
									req.id,
									ErrorObject::owned(
										-32998,
										"RPC request is missing client key for rate limiting",
										None::<()>,
									),
								);
							};
							inner.check_key(client_key).err().map(|rejected| {
								jitter + rejected.wait_time_from(inner.clock().now())
							})
						},
					};

					let Some(retry_after) = rate_limit_retry_after else {
						break;
					};
					is_rate_limited = true;
					if rate_limit_retries >= rate_limit_max_slowdowns {
						return MethodResponse::error(
							req.id,
							ErrorObject::owned(-32999, "RPC rate limit exceeded", None::<()>),
						);
					}
					rate_limit_retries += 1;
					tokio::time::sleep(retry_after).await;
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct ClientRateLimitKey(String);

impl ClientRateLimitKey {
	fn from_ip(ip: &str) -> Self {
		Self::from_parts("ip:", ip)
	}

	fn from_connection(connection_id: usize) -> Self {
		Self(format!("connection:{connection_id}"))
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
pub struct ClientRateLimitKeyLayer {
	trust_proxy_headers: bool,
}

impl ClientRateLimitKeyLayer {
	pub fn new(trust_proxy_headers: bool) -> Self {
		Self { trust_proxy_headers }
	}
}

impl<S> tower::Layer<S> for ClientRateLimitKeyLayer {
	type Service = ClientRateLimitKeyMiddleware<S>;

	fn layer(&self, service: S) -> Self::Service {
		ClientRateLimitKeyMiddleware { service, trust_proxy_headers: self.trust_proxy_headers }
	}
}

#[derive(Clone)]
pub struct ClientRateLimitKeyMiddleware<S> {
	service: S,
	trust_proxy_headers: bool,
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
		let client_key =
			get_client_rate_limit_key_from_http_request(&request, self.trust_proxy_headers);
		if let Some(client_key) = client_key {
			request.extensions_mut().insert(client_key);
		}
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
}

fn get_client_rate_limit_key_from_http_request<B>(
	request: &HttpRequest<B>,
	trust_proxy_headers: bool,
) -> Option<ClientRateLimitKey> {
	// Proxy headers are only read when trust is explicitly enabled.
	// This assumes the node is deployed behind trusted proxies and should only be
	// enabled when the request path is controlled.
	if trust_proxy_headers {
		if let Some(key) = get_forwarded_ip(request).or_else(|| get_header_ip(request, "x-real-ip"))
		{
			return Some(key);
		}
	}

	request
		.extensions()
		.get::<ConnectionId>()
		.map(|id| ClientRateLimitKey::from_connection(id.0))
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
	use futures::future::ready;
	use jsonrpsee::{
		server::{HttpRequest, ResponsePayload},
		types::Id,
	};
	use std::sync::atomic::{AtomicUsize, Ordering};
	use tokio::time::timeout;

	#[test]
	fn extracts_forwarded_ip_when_trust_proxy_headers_enabled() {
		let mut request = HttpRequest::new(());
		request
			.headers_mut()
			.insert("x-forwarded-for", "203.0.113.9, 198.51.100.3".parse().expect("valid header"));

		let key = get_client_rate_limit_key_from_http_request(&request, true);
		assert_eq!(key, Some(ClientRateLimitKey::from_ip("203.0.113.9")));
	}

	#[test]
	fn falls_back_to_connection_id_when_proxy_headers_disabled() {
		let mut request = HttpRequest::new(());
		request.extensions_mut().insert(ConnectionId(42));

		let key = get_client_rate_limit_key_from_http_request(&request, false);
		assert_eq!(key, Some(ClientRateLimitKey::from_connection(42)));
	}

	#[test]
	fn extracts_no_key_when_proxy_headers_enabled_but_no_client_or_connection_key() {
		let request = HttpRequest::new(());
		let key = get_client_rate_limit_key_from_http_request(&request, true);
		assert_eq!(key, None);
	}

	#[test]
	fn falls_back_to_connection_id_when_proxy_headers_enabled_but_missing_proxy_header() {
		let mut request = HttpRequest::new(());
		request.extensions_mut().insert(ConnectionId(42));

		let key = get_client_rate_limit_key_from_http_request(&request, true);
		assert_eq!(key, Some(ClientRateLimitKey::from_connection(42)));
	}

	#[derive(Debug, Clone)]
	struct CountingService {
		calls: Arc<AtomicUsize>,
	}

	impl CountingService {
		fn new() -> Self {
			Self { calls: Arc::new(AtomicUsize::new(0)) }
		}

		fn calls(&self) -> usize {
			self.calls.load(Ordering::SeqCst)
		}
	}

	impl<'a> RpcServiceT<'a> for CountingService {
		type Future = futures::future::Ready<MethodResponse>;

		fn call(&self, req: Request<'a>) -> Self::Future {
			self.calls.fetch_add(1, Ordering::SeqCst);
			let _ = &req;
			ready(MethodResponse::response(req.id.clone(), ResponsePayload::success(()), 1024))
		}
	}

	#[tokio::test]
	async fn middleware_allows_first_request_when_rate_available() {
		let service = CountingService::new();
		let rate_limit = RateLimit {
			inner: RateLimitInner::PerConnection(Arc::new(PerConnectionRateLimitInner::keyed(
				Quota::per_minute(NonZeroU32::new(10).unwrap()),
			))),
		};
		let middleware = Middleware {
			service: service.clone(),
			rate_limit: Some(rate_limit),
			rate_limit_max_slowdowns: 10,
			metrics: None,
		};

		let request = {
			let mut request = Request::new("rpc.health".into(), None, Id::Number(1));
			request.extensions_mut().insert(ConnectionId(99));
			request.extensions_mut().insert(ClientRateLimitKey::from_ip("203.0.113.11"));
			request
		};

		let response = middleware.call(request).await;
		assert!(response.is_success());
		assert_eq!(service.calls(), 1);
	}

	#[tokio::test]
	async fn middleware_delays_connection_rate_limited_request() {
		let service = CountingService::new();
		let rate_limit = RateLimit {
			inner: RateLimitInner::PerConnection(Arc::new(PerConnectionRateLimitInner::keyed(
				Quota::per_second(NonZeroU32::new(1).unwrap()),
			))),
		};

		let middleware = Middleware {
			service: service.clone(),
			rate_limit: Some(rate_limit),
			rate_limit_max_slowdowns: 10,
			metrics: None,
		};

		let mut first_request = Request::new("rpc.health".into(), None, Id::Number(1));
		first_request.extensions_mut().insert(ConnectionId(1));
		first_request
			.extensions_mut()
			.insert(ClientRateLimitKey::from_ip("203.0.113.10"));

		let second_request = Request::new("rpc.health".into(), None, Id::Number(2));
		let mut second_request = second_request;
		second_request.extensions_mut().insert(ConnectionId(1));
		second_request
			.extensions_mut()
			.insert(ClientRateLimitKey::from_ip("203.0.113.10"));

		let first = middleware.call(first_request).await;
		assert!(first.is_success());
		assert_eq!(service.calls(), 1);

		// Same connection and key should be rate-limited; this would complete immediately if keying
		// is broken.
		assert!(
			timeout(Duration::from_millis(100), middleware.call(second_request))
				.await
				.is_err()
		);
		assert_eq!(service.calls(), 1);

		let third_request = Request::new("rpc.health".into(), None, Id::Number(3));
		let mut third_request = third_request;
		third_request.extensions_mut().insert(ConnectionId(1));
		third_request
			.extensions_mut()
			.insert(ClientRateLimitKey::from_ip("203.0.113.10"));

		let third = timeout(Duration::from_millis(1500), middleware.call(third_request))
			.await
			.unwrap();
		assert!(third.is_success());
		assert_eq!(service.calls(), 2);
	}

	#[tokio::test]
	async fn middleware_connection_mode_rejects_missing_connection_id() {
		let service = CountingService::new();
		let rate_limit = RateLimit {
			inner: RateLimitInner::PerConnection(Arc::new(PerConnectionRateLimitInner::keyed(
				Quota::per_second(NonZeroU32::new(1).unwrap()),
			))),
		};
		let middleware = Middleware {
			service: service.clone(),
			rate_limit: Some(rate_limit),
			rate_limit_max_slowdowns: 10,
			metrics: None,
		};

		let request = Request::new("rpc.health".into(), None, Id::Number(1));
		let response = middleware.call(request).await;
		assert!(!response.is_success());
		assert_eq!(service.calls(), 0);
	}

	#[tokio::test]
	async fn middleware_rejects_request_when_max_slowdowns_is_zero() {
		let service = CountingService::new();
		let rate_limit = RateLimit {
			inner: RateLimitInner::PerConnection(Arc::new(PerConnectionRateLimitInner::keyed(
				Quota::per_second(NonZeroU32::new(1).unwrap()),
			))),
		};

		let middleware = Middleware {
			service: service.clone(),
			rate_limit: Some(rate_limit),
			rate_limit_max_slowdowns: 0,
			metrics: None,
		};

		let first_request = {
			let mut request = Request::new("rpc.health".into(), None, Id::Number(1));
			request.extensions_mut().insert(ConnectionId(1));
			request
		};
		let second_request = {
			let mut request = Request::new("rpc.health".into(), None, Id::Number(2));
			request.extensions_mut().insert(ConnectionId(1));
			request
		};

		let first = middleware.call(first_request).await;
		let second = middleware.call(second_request).await;

		assert!(first.is_success());
		assert!(!second.is_success());
		assert_eq!(service.calls(), 1);
	}

	#[tokio::test]
	async fn middleware_per_ip_rate_limit_shares_across_connections_for_same_ip() {
		let service = CountingService::new();
		let rate_limit = RateLimit {
			inner: RateLimitInner::PerIp(Arc::new(PerIpRateLimitInner::keyed(Quota::per_second(
				NonZeroU32::new(1).unwrap(),
			)))),
		};
		let middleware = Middleware {
			service: service.clone(),
			rate_limit: Some(rate_limit),
			rate_limit_max_slowdowns: 10,
			metrics: None,
		};

		let mut first_request = Request::new("rpc.health".into(), None, Id::Number(1));
		first_request.extensions_mut().insert(ConnectionId(1));
		first_request
			.extensions_mut()
			.insert(ClientRateLimitKey::from_ip("203.0.113.10"));

		let mut second_request = Request::new("rpc.health".into(), None, Id::Number(2));
		second_request.extensions_mut().insert(ConnectionId(2));
		// Same IP but different connection; should be counted in the same per-IP bucket.
		second_request
			.extensions_mut()
			.insert(ClientRateLimitKey::from_ip("203.0.113.10"));

		let third_request = Request::new("rpc.health".into(), None, Id::Number(3));
		let mut third_request = third_request;
		third_request.extensions_mut().insert(ConnectionId(3));
		third_request
			.extensions_mut()
			.insert(ClientRateLimitKey::from_ip("198.51.100.20"));

		let first = middleware.call(first_request).await;
		assert!(first.is_success());

		let second = timeout(Duration::from_millis(100), middleware.call(second_request)).await;
		assert!(second.is_err(), "same IP across connections should be per-ip rate limited");

		let third = timeout(Duration::from_millis(100), middleware.call(third_request)).await;
		assert!(third.is_ok(), "different IP should not be coupled");
		assert_eq!(service.calls(), 2);
	}

	#[tokio::test]
	async fn middleware_uses_isolated_ip_keys() {
		let service = CountingService::new();
		let rate_limit = RateLimit {
			inner: RateLimitInner::PerIp(Arc::new(PerIpRateLimitInner::keyed(Quota::per_second(
				NonZeroU32::new(1).unwrap(),
			)))),
		};
		let middleware = Middleware {
			service: service.clone(),
			rate_limit: Some(rate_limit),
			rate_limit_max_slowdowns: 10,
			metrics: None,
		};

		let mut first_request = Request::new("rpc.health".into(), None, Id::Number(1));
		first_request
			.extensions_mut()
			.insert(ClientRateLimitKey::from_ip("203.0.113.10"));
		let mut second_request = Request::new("rpc.health".into(), None, Id::Number(2));
		second_request
			.extensions_mut()
			.insert(ClientRateLimitKey::from_ip("198.51.100.20"));
		let mut third_request = Request::new("rpc.health".into(), None, Id::Number(3));
		third_request
			.extensions_mut()
			.insert(ClientRateLimitKey::from_ip("203.0.113.10"));

		let first = middleware.call(first_request).await;
		assert!(first.is_success());

		let second = timeout(Duration::from_millis(200), middleware.call(second_request)).await;
		assert!(second.is_ok(), "different ips should not share quota");

		let third = timeout(Duration::from_millis(200), middleware.call(third_request)).await;
		assert!(third.is_err(), "same ip should be rate-limited");

		assert_eq!(service.calls(), 2);
	}
}
