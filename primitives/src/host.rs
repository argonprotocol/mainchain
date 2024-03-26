use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Deserialize,
	Serialize,
)]
pub struct Host {
	#[codec(compact)]
	/// ip encoded as u32 big endian (eg, from octets)
	pub ip: u32,
	#[codec(compact)]
	pub port: u16,
	pub is_secure: bool,
}

impl Host {
	#[cfg(feature = "std")]
	pub fn get_url(&self) -> String {
		Self::format_url(self.is_secure, self.ip, self.port)
	}
	#[cfg(feature = "std")]
	pub fn format_url(is_secure: bool, ip: u32, port: u16) -> String {
		format!(
			"{}://{}:{}",
			if is_secure { "wss" } else { "ws" },
			std::net::Ipv4Addr::from(ip),
			port
		)
	}
}
