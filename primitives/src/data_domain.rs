use codec::{Codec, Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ConstU32, H256};
use sp_crypto_hashing::blake2_256;
use sp_debug_derive::RuntimeDebug;
use sp_runtime::{BoundedBTreeMap, BoundedVec, RuntimeString};
use sp_std::{cmp::Ordering, collections::btree_map::BTreeMap, str};

use crate::{data_tld::DataTLD, host::Host, NotaryId};

pub const MAX_DATASTORE_VERSIONS: u32 = 25;

pub const DATA_DOMAIN_LEASE_COST: u128 = 1_000;

pub const MIN_DATA_DOMAIN_NAME_LENGTH: usize = 2;

pub type DataDomainHash = H256;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataDomain {
	pub domain_name: RuntimeString,
	pub top_level_domain: DataTLD,
}

impl DataDomain {
	pub fn hash(&self) -> DataDomainHash {
		self.using_encoded(blake2_256).into()
	}
}

impl PartialOrd for DataDomain {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if self.top_level_domain != other.top_level_domain {
			return self.top_level_domain.partial_cmp(&other.top_level_domain);
		}

		Some(self.domain_name.as_ref().cmp(other.domain_name.as_ref()))
	}
}

impl Ord for DataDomain {
	fn cmp(&self, other: &Self) -> Ordering {
		self.partial_cmp(other).unwrap()
	}
}

impl DataDomain {
	pub fn new(domain_name: &'static str, top_level_domain: DataTLD) -> Self {
		Self { domain_name: RuntimeString::Borrowed(domain_name), top_level_domain }
	}

	#[cfg(feature = "std")]
	pub fn from_string(domain_name: String, top_level_domain: DataTLD) -> Self {
		Self { domain_name: RuntimeString::Owned(domain_name.to_lowercase()), top_level_domain }
	}

	#[cfg(feature = "std")]
	pub fn parse(domain: String) -> Result<Self, String> {
		let parts: Vec<&str> = domain.split('.').collect();
		if parts.len() < 2 {
			return Err("Invalid domain".to_string());
		}
		let tld = parts[1];
		let domain_name = parts[0].to_lowercase();
		let tld_str = format!("\"{}\"", tld);
		let mut parsed_tld = serde_json::from_str(&tld_str).ok();
		if parsed_tld.is_none() {
			let tld_str = tld[0..1].to_uppercase() + &tld[1..];
			let tld_str = format!("\"{}\"", tld_str);
			parsed_tld = serde_json::from_str(&tld_str).ok();
		}
		if parsed_tld.is_none() {
			let tld_str = format!("\"{}\"", tld.to_lowercase());
			parsed_tld = serde_json::from_str(&tld_str).ok();
		}
		let Some(parsed_tld) = parsed_tld else {
			return Err("Invalid tld".to_string());
		};
		Ok(Self::from_string(domain_name, parsed_tld))
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// ZoneRecords track versions of a data domain and the host addresses where they can be accessed.
pub struct ZoneRecord<AccountId: Codec> {
	pub payment_account: AccountId,
	/// The notary that payments must be notarized through
	pub notary_id: NotaryId,
	/// A mapping of versions to host addresses.
	pub versions: BTreeMap<Semver, VersionHost>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
/// ZoneRecords track versions of a data domain and the host addresses where they can be accessed.
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
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct VersionHost {
	/// Datastore id is a 2-50 char string that uniquely identifies a data domain.
	pub datastore_id: BoundedVec<u8, ConstU32<50>>,
	/// The host address where the data domain can be accessed.
	pub host: Host,
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
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
		if self.major != other.major {
			return self.major.partial_cmp(&other.major);
		}
		if self.minor != other.minor {
			return self.minor.partial_cmp(&other.minor);
		}
		self.patch.partial_cmp(&other.patch)
	}
}

impl Ord for Semver {
	fn cmp(&self, other: &Self) -> Ordering {
		self.partial_cmp(other).unwrap()
	}
}
