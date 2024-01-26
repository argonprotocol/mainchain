use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::TypeInfo, Deserialize, Serialize};
use sp_core::{ConstU32, H256};
use sp_core_hashing::blake2_256;
use sp_debug_derive::RuntimeDebug;
use sp_runtime::{BoundedBTreeMap, BoundedVec};
use sp_std::{cmp::Ordering, collections::btree_map::BTreeMap, str};

use crate::{data_tld::DataTLD, host::Host, NotaryId};

pub const MAX_DATASTORE_VERSIONS: u32 = 25;
pub const MAX_DOMAIN_NAME_LENGTH: u32 = 50;

pub const DATA_DOMAIN_LEASE_COST: u128 = 1_000;

pub const MIN_DATA_DOMAIN_NAME_LENGTH: usize = 2;

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
pub struct DataDomain {
	pub domain_name: BoundedVec<u8, ConstU32<MAX_DOMAIN_NAME_LENGTH>>,
	pub top_level_domain: DataTLD,
}
impl PartialOrd for DataDomain {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if self.top_level_domain != other.top_level_domain {
			return self.top_level_domain.partial_cmp(&other.top_level_domain);
		}

		Some(self.domain_name.cmp(&other.domain_name))
	}
}

impl Ord for DataDomain {
	fn cmp(&self, other: &Self) -> Ordering {
		self.partial_cmp(other).unwrap()
	}
}

impl DataDomain {
	pub fn new(domain_name: &str, top_level_domain: DataTLD) -> Self {
		Self {
			domain_name: BoundedVec::truncate_from(domain_name.as_bytes().to_vec()),
			top_level_domain,
		}
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
