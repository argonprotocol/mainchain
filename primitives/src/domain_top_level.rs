use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

#[derive(
	PartialEq,
	Eq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(not(feature = "napi"), derive(Clone, Copy))]
#[cfg_attr(feature = "napi", napi_derive::napi)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum DomainTopLevel {
	Analytics,
	Automotive,
	Bikes,
	Business,
	Cars,
	Communication,
	Entertainment,
	Finance,
	Flights,
	Health,
	Hotels,
	Jobs,
	News,
	RealEstate,
	Restaurants,
	Shopping,
	Sports,
	Transportation,
	Travel,
	Weather,
}
