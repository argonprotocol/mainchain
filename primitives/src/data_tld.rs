use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;

#[derive(
	PartialEq,
	Eq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(not(feature = "napi"), derive(Clone, Copy))]
#[cfg_attr(feature = "napi", napi_derive::napi)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum DataTLD {
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
