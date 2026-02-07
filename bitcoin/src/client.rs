use bitcoin::hashes::serde;
use bitcoincore_rpc::{
	Auth, Error, RpcApi, jsonrpc,
	jsonrpc::{Request, Response, base64, minreq, minreq_http::HttpError},
};
use log::{
	Level::{Debug, Trace, Warn},
	debug, log_enabled, trace,
};
use serde::Deserialize;
use serde_json::Value;
use std::{fmt, io::Read, sync::atomic, time::Duration};

/// This class is a modified version of the minreq client built into bitcoincore-rpc-rs, but with
/// the rust tls feature activated
pub struct Client {
	pub timeout: Duration,
	url: String,
	basic_auth: Option<String>,
	nonce: atomic::AtomicUsize,
}

impl fmt::Debug for Client {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "bitcoincore_rpc::Client({:?})", self.url)
	}
}

impl Client {
	/// Creates a client to a bitcoind JSON-RPC server.
	pub fn new(url: &str, auth: Auth) -> anyhow::Result<Self> {
		let mut basic = None;
		match auth {
			Auth::None => {},
			Auth::UserPass(u, p) => {
				let auth = format!("{u}:{p}");
				basic = Some(auth.into_bytes());
			},
			Auth::CookieFile(file) => {
				let mut cookie = std::fs::File::open(file)?;
				let mut buf = vec![];
				cookie.read_to_end(&mut buf)?;
				basic = Some(buf);
			},
		}
		let basic_auth = basic.map(|buf| format!("Basic {}", base64::encode(buf)));

		let timeout = Duration::from_secs(15);

		let client = Client {
			basic_auth,
			url: url.to_string(),
			nonce: atomic::AtomicUsize::new(0),
			timeout,
		};
		Ok(client)
	}

	fn send_request(&self, req: Request) -> Result<Response, jsonrpc::minreq_http::Error> {
		let mut client = minreq::Request::new(minreq::Method::Post, &self.url)
			.with_timeout(self.timeout.as_secs());

		if let Some(auth) = &self.basic_auth {
			client = client.with_header("Authorization", auth);
		}
		let resp = client.with_json(&req)?.send()?;
		match resp.json() {
			Ok(json) => Ok(json),
			Err(minreq_err) =>
				if resp.status_code != 200 {
					Err(jsonrpc::minreq_http::Error::Http(HttpError {
						status_code: resp.status_code,
						body: resp.as_str().unwrap_or("").to_string(),
					}))
				} else {
					Err(jsonrpc::minreq_http::Error::Minreq(minreq_err))
				},
		}
	}
}

impl RpcApi for Client {
	fn call<T: for<'a> Deserialize<'a>>(
		&self,
		cmd: &str,
		args: &[Value],
	) -> bitcoincore_rpc::Result<T> {
		let raw = serde_json::value::to_raw_value(args)?;
		let nonce = self.nonce.fetch_add(1, atomic::Ordering::Relaxed);
		let req = Request {
			method: cmd,
			params: Some(&raw),
			id: serde_json::Value::from(nonce),
			jsonrpc: Some("2.0"),
		};
		if log_enabled!(Debug) {
			debug!(target: "bitcoincore_rpc", "JSON-RPC request: {} {}", cmd, serde_json::Value::from(args));
		}

		let resp = self
			.send_request(req)
			.map_err(|e| Error::JsonRpc(jsonrpc::Error::Transport(Box::new(e))));
		log_response(cmd, &resp);
		Ok(resp?.result()?)
	}
}

fn log_response(cmd: &str, resp: &bitcoincore_rpc::Result<jsonrpc::Response>) {
	if log_enabled!(Warn) || log_enabled!(Debug) || log_enabled!(Trace) {
		match resp {
			Err(e) =>
				if log_enabled!(Debug) {
					debug!(target: "bitcoincore_rpc", "JSON-RPC failed parsing reply of {cmd}: {e:?}");
				},
			Ok(resp) =>
				if let Some(ref e) = resp.error {
					if log_enabled!(Debug) {
						debug!(target: "bitcoincore_rpc", "JSON-RPC error for {cmd}: {e:?}");
					}
				} else if log_enabled!(Trace) {
					let def =
						serde_json::value::to_raw_value(&serde_json::value::Value::Null).unwrap();
					let result = resp.result.as_ref().unwrap_or(&def);
					trace!(target: "bitcoincore_rpc", "JSON-RPC response for {cmd}: {result}");
				},
		}
	}
}
