use codec::{Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{bounded::BoundedVec, crypto::AccountId32, ed25519, ConstU32, RuntimeDebug, H256};
use sp_core_hashing::blake2_256;
use sp_runtime::scale_info::TypeInfo;

use crate::{AccountOrigin, AccountType, BalanceChange};

pub const PINNED_BLOCKS_OFFSET: u32 = 100u32;
pub const MAX_TRANSFERS: u32 = 10_000;
pub type MaxTransfers = ConstU32<MAX_TRANSFERS>;
pub type MaxBalanceChanges = ConstU32<100_000>;
pub type NotaryId = u32;
pub type AccountOriginUid = u32;
pub type NotebookNumber = u32;

pub type RequiredNotebookAuditors = ConstU32<10>;
pub const NOTEBOOK_VERSION: u16 = 1;

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
pub struct Notebook {
	pub header: NotebookHeader,
	pub balance_changes:
		BoundedVec<BoundedVec<BalanceChange, MaxBalanceChanges>, MaxBalanceChanges>,
	pub new_account_origins: BoundedVec<NewAccountOrigin, MaxBalanceChanges>,
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
pub struct NewAccountOrigin {
	pub account_id: AccountId32,
	pub account_type: AccountType,
	#[codec(compact)]
	pub account_uid: AccountOriginUid,
}
impl NewAccountOrigin {
	pub fn new(
		account_id: AccountId32,
		account_type: AccountType,
		account_uid: AccountOriginUid,
	) -> Self {
		Self { account_id, account_type, account_uid }
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
pub struct NotebookHeader {
	#[codec(compact)]
	pub version: u16,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	#[codec(compact)]
	pub finalized_block_number: u32,
	#[codec(compact)]
	pub pinned_to_block_number: u32,

	pub start_time: u64,
	pub end_time: u64,
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub chain_transfers: BoundedVec<ChainTransfer, MaxTransfers>,
	/// A merkle root for all account balances changed in this notebook.
	/// Nodes are in the order of when each account is seen in the notebook.
	/// Nodes contain the account id, account_type, nonce, balance and account origin.
	/// If a node is in the balance changes twice, only the last entry will be used.
	/// Nodes are encoded as Scale and hashed with Blake2 256  
	pub changed_accounts_root: H256,
	/// All ids that are changed in this notebook. A notebook id is a tuple of (origin notebook,
	/// counter)
	pub changed_account_origins: BoundedVec<AccountOrigin, ConstU32<{ u32::MAX }>>,
}

impl NotebookHeader {
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
pub struct AuditedNotebook {
	/// Hash of the notebook header that should have been signed by the auditors.
	pub header_hash: H256,
	pub header: NotebookHeader,
	pub auditors: BoundedVec<(ed25519::Public, ed25519::Signature), RequiredNotebookAuditors>,
}
impl AuditedNotebook {
	/// Calculate a final hash including auditors
	pub fn hash(&self) -> H256 {
		blake2_256(
			&[self.header_hash.as_ref(), self.auditors.using_encoded(blake2_256).as_ref()].concat(),
		)
		.into()
	}
}

#[derive(
	Clone,
	PartialEq,
	PartialOrd,
	Ord,
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
#[serde(tag = "type")]
pub enum ChainTransfer {
	#[serde(rename_all = "camelCase")]
	ToMainchain {
		account_id: AccountId32,
		#[codec(compact)]
		#[cfg_attr(feature = "std", serde(with = "serialize_u128_as_string"))]
		amount: u128,
	},
	#[serde(rename_all = "camelCase")]
	ToLocalchain {
		account_id: AccountId32,
		#[codec(compact)]
		account_nonce: u32,
	},
}

#[cfg(feature = "std")]
mod serialize_u128_as_string {
	use serde::{Deserialize, Deserializer, Serializer};

	pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = (*value).to_string();
		serializer.serialize_str(&s)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		s.parse::<u128>().map_err(serde::de::Error::custom)
	}
}
