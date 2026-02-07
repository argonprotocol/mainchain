use alloc::{format, string::String};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;

use crate::AccountId;

#[derive(
	Clone,
	PartialEq,
	Ord,
	PartialOrd,
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
/// A unique identifier for an account
pub struct LocalchainAccountId {
	/// The account id of the account
	pub account_id: AccountId,
	pub account_type: AccountType,
}

impl LocalchainAccountId {
	pub fn new(account_id: AccountId, account_type: AccountType) -> Self {
		Self { account_id, account_type }
	}
	pub fn is_tax(&self) -> bool {
		self.account_type == AccountType::Tax
	}
	pub fn is_deposit(&self) -> bool {
		self.account_type == AccountType::Deposit
	}
}

impl core::hash::Hash for LocalchainAccountId {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		state.write(self.account_id.as_ref());
		state.write_i32(self.account_type as i32);
	}
}

#[derive(
	PartialEq,
	Eq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
)]
#[cfg_attr(all(feature = "std", not(feature = "uniffi")), derive(clap::ValueEnum))]
#[cfg_attr(not(feature = "napi"), derive(Clone, Copy))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "napi", napi_derive::napi(string_enum = "camelCase"))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum AccountType {
	Tax,
	Deposit,
}

impl AccountType {
	pub fn as_str(&self) -> &'static str {
		match self {
			AccountType::Tax => "tax",
			AccountType::Deposit => "deposit",
		}
	}
}

impl TryFrom<i32> for AccountType {
	type Error = String;

	fn try_from(value: i32) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(AccountType::Tax),
			1 => Ok(AccountType::Deposit),
			_ => Err(format!("Invalid account_type value {value}")),
		}
	}
}

impl From<i64> for AccountType {
	fn from(value: i64) -> Self {
		if value == 0 { AccountType::Tax } else { AccountType::Deposit }
	}
}
