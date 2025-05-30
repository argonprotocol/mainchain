use alloc::{
	string::{String, ToString},
	vec::Vec,
};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;
use sp_runtime::{traits::ConstU32, BoundedVec};

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[repr(transparent)]
pub struct Host(pub BoundedVec<u8, ConstU32<253>>);

impl From<String> for Host {
	fn from(host: String) -> Self {
		host.into_bytes().to_vec().into()
	}
}

impl From<&str> for Host {
	fn from(host: &str) -> Self {
		host.as_bytes().to_vec().into()
	}
}

impl From<Vec<u8>> for Host {
	fn from(host: Vec<u8>) -> Self {
		Self(BoundedVec::truncate_from(host))
	}
}

impl TryInto<String> for Host {
	type Error = String;

	fn try_into(self) -> Result<String, Self::Error> {
		String::from_utf8(self.0.into_inner()).map_err(|_| "Invalid UTF-8".to_string())
	}
}

impl Serialize for Host {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let string_value: String = self.clone().try_into().map_err(serde::ser::Error::custom)?;
		serializer.serialize_str(&string_value)
	}
}

impl<'de> Deserialize<'de> for Host {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		Ok(Host::from(s))
	}
}
