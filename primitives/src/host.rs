use sp_runtime::{BoundedVec,};
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;
use sp_runtime::traits::ConstU32;

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
#[serde(transparent)]
#[repr(transparent)]
pub struct Host(pub BoundedVec<u8, ConstU32<253>>);

#[cfg(feature = "std")]
impl From<String> for Host {
	fn from(host: String) -> Self {
		host.into_bytes().to_vec().into()
	}
}

#[cfg(feature = "std")]
impl From<&str> for Host {
	fn from(host: &str) -> Self {
		host.as_bytes().to_vec().into()
	}
}

#[cfg(feature = "std")]
impl From<Vec<u8>> for Host {
	fn from(host: Vec<u8>) -> Self {
		Self(BoundedVec::truncate_from(host))
	}
}

#[cfg(feature = "std")]
impl TryInto<String> for Host {
	type Error = String;

	fn try_into(self) -> Result<String, Self::Error> {
		String::from_utf8(self.0.into_inner()).map_err(|_| "Invalid UTF-8".to_string())
	}
}
