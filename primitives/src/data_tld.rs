use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::TypeInfo, Deserialize, Serialize};
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
#[cfg_attr(not(feature = "napi"), derive(Clone))]
#[cfg_attr(feature = "napi", napi_derive::napi)]
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
