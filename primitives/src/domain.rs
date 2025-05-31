use alloc::{
	collections::btree_map::BTreeMap,
	string::{String, ToString},
};
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::{cmp::Ordering, str};
use polkadot_sdk::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ConstU32, H256};
use sp_crypto_hashing::blake2_256;
use sp_debug_derive::RuntimeDebug;
use sp_runtime::{BoundedBTreeMap, BoundedVec};

use crate::{Balance, NotaryId, domain_top_level::DomainTopLevel, host::Host};

pub const MAX_DATASTORE_VERSIONS: u32 = 25;

pub const DOMAIN_LEASE_COST: Balance = 1_000_000;

pub const MIN_DOMAIN_NAME_LENGTH: usize = 2;

pub type DomainHash = H256;

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	Serialize,
	Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
	pub name: String,
	pub top_level: DomainTopLevel,
}

impl Domain {
	pub fn hash(&self) -> DomainHash {
		self.using_encoded(blake2_256).into()
	}
}

impl PartialOrd for Domain {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Domain {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.top_level != other.top_level {
			return self.top_level.cmp(&other.top_level);
		}

		self.name.cmp(&other.name)
	}
}

impl Domain {
	pub fn new(name: &'static str, top_level: DomainTopLevel) -> Self {
		Self { name: name.to_string(), top_level }
	}

	#[cfg(feature = "std")]
	pub fn from_string(name: String, top_level: DomainTopLevel) -> Self {
		Self { name, top_level }
	}

	#[cfg(feature = "std")]
	pub fn parse(domain: String) -> Result<Self, String> {
		let parts: Vec<&str> = domain.split('.').collect();
		if parts.len() < 2 {
			return Err("Invalid domain".to_string());
		}
		let top_level = parts[1];
		let domain_name = parts[0].to_lowercase();
		let tld_str = format!("\"{}\"", top_level);
		let mut parsed_tld = serde_json::from_str(&tld_str).ok();
		if parsed_tld.is_none() {
			let tld_str = top_level[0..1].to_uppercase() + &top_level[1..];
			let tld_str = format!("\"{}\"", tld_str);
			parsed_tld = serde_json::from_str(&tld_str).ok();
		}
		if parsed_tld.is_none() {
			let tld_str = format!("\"{}\"", top_level.to_lowercase());
			parsed_tld = serde_json::from_str(&tld_str).ok();
		}
		let Some(parsed_tld) = parsed_tld else {
			return Err("Invalid top_level".to_string());
		};
		Ok(Self::from_string(domain_name, parsed_tld))
	}
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	Serialize,
	Deserialize,
)]
#[serde(rename_all = "camelCase")]
/// ZoneRecords track versions of a domain and the host addresses where they can be accessed.
pub struct ZoneRecord<AccountId>
where
	AccountId: Codec,
{
	pub payment_account: AccountId,
	/// The notary that payments must be notarized through
	pub notary_id: NotaryId,
	/// A mapping of versions to host addresses.
	pub versions: BTreeMap<Semver, VersionHost>,
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
/// ZoneRecords track versions of a domain and the host addresses where they can be accessed.
pub struct BoundZoneRecord {
	/// A mapping of versions to host addresses.
	pub versions: BoundedBTreeMap<Semver, VersionHost, ConstU32<MAX_DATASTORE_VERSIONS>>,
}

impl<A: Codec> ZoneRecord<A> {
	pub fn hash(&self) -> H256 {
		self.using_encoded(blake2_256).into()
	}
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct VersionHost {
	/// Datastore id is a 2-50 char string that uniquely identifies a domain.
	pub datastore_id: BoundedVec<u8, ConstU32<50>>,
	/// The host address where the domain can be accessed.
	pub host: Host,
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Semver {
	pub major: u32,
	pub minor: u32,
	pub patch: u32,
}

impl Semver {
	pub fn new(major: u32, minor: u32, patch: u32) -> Self {
		Self { major, minor, patch }
	}
}

impl PartialOrd for Semver {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Semver {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.major != other.major {
			return self.major.cmp(&other.major);
		}
		if self.minor != other.minor {
			return self.minor.cmp(&other.minor);
		}
		self.patch.cmp(&other.patch)
	}
}
