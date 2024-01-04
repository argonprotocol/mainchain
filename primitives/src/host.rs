use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::TypeInfo, Deserialize, Serialize};
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
		format!(
			"{}://{}:{}",
			if self.is_secure { "wss" } else { "ws" },
			std::net::Ipv4Addr::from(self.ip),
			self.port
		)
	}
}
