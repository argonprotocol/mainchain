use codec::{Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{bounded::BoundedVec, ConstU32, RuntimeDebug, H256};
use sp_core_hashing::blake2_256;
use sp_runtime::scale_info::TypeInfo;

use crate::{AccountId, AccountOriginUid, Chain, Note};

pub const MAX_BALANCESET_CHANGES: u32 = 25;

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
pub struct BalanceChange {
	/// Localchain account
	pub account_id: AccountId,
	/// Which chain (tax or argon)
	pub chain: Chain,
	/// Nonce that must increment for each change for an account
	#[codec(compact)]
	pub nonce: u32,
	/// New balance after change
	#[codec(compact)]
	pub balance: u128,
	/// A balance change must include the previous balance
	#[codec(compact)]
	pub previous_balance: u128,
	/// A balance change must provide proof of a previous balance if the nonce is non-zero
	pub previous_balance_proof: Option<BalanceProof>,
	/// Sources of the changes
	pub notes: BoundedVec<Note, ConstU32<100>>,
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
pub struct BalanceProof {
	/// The notebook where this proof can be verified
	#[codec(compact)]
	pub notebook_number: u32,
	/// The notary where this proof can be verified
	#[codec(compact)]
	pub notary_id: u32,
	/// The first recording of the given account
	pub account_origin: AccountOrigin,
	/// Proof items (does not contain the leaf hash, nor the root obviously).
	///
	/// This vec contains all inner node hashes necessary to reconstruct the root hash given the
	/// leaf hash.
	pub proof: BoundedVec<H256, ConstU32<{ u32::MAX }>>,
	/// Number of leaves in the original tree.
	///
	/// This is needed to detect a case where we have an odd number of leaves that "get promoted"
	/// to upper layers.
	#[codec(compact)]
	pub number_of_leaves: u32,
	/// Index of the leaf the proof is for (0-based).
	#[codec(compact)]
	pub leaf_index: u32,
}

#[derive(Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
pub struct BalanceTip {
	pub account_id: AccountId,
	pub chain: Chain,
	pub nonce: u32,
	pub balance: u128,
	pub account_origin: AccountOrigin,
}

impl BalanceTip {
	pub fn compute_tip(nonce: u32, balance: u128, account_origin: AccountOrigin) -> [u8; 32] {
		BalanceTipValue { nonce, balance, account_origin }.using_encoded(blake2_256)
	}

	pub fn create_key(account_id: &AccountId, chain: &Chain) -> [u8; 32] {
		blake2_256(&[&account_id.as_ref(), &chain.encode()[..]].concat())
	}
}

#[derive(Encode, Decode)]
struct BalanceTipValue {
	pub nonce: u32,
	pub balance: u128,
	pub account_origin: AccountOrigin,
}

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
pub struct AccountOrigin {
	/// The notebook where this account was first seen
	#[codec(compact)]
	pub notebook_number: u32,
	/// A unique identifier for an account
	#[codec(compact)]
	pub account_uid: AccountOriginUid,
}
