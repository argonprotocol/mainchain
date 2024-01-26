use codec::{Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{bounded::BoundedVec, ConstU32, RuntimeDebug, H256};
use sp_core_hashing::blake2_256;
use sp_runtime::{scale_info::TypeInfo, traits::Verify, MultiSignature};
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use sp_core::crypto::Pair;

use crate::{
	notary::NotaryId, tick::Tick, AccountId, AccountOriginUid, AccountType, Note, NoteType,
	NotebookNumber,
};

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
	/// Which type (tax or deposit)
	pub account_type: AccountType,
	#[codec(compact)]
	pub change_number: u32,
	/// New balance after change
	#[codec(compact)]
	pub balance: u128,
	/// A balance change must provide proof of a previous balance if the change_number is non-zero
	pub previous_balance_proof: Option<BalanceProof>,
	/// If this account is on hold, the hold note and index
	pub channel_hold_note: Option<Note>,
	/// Sources of the changes
	pub notes: BoundedVec<Note, ConstU32<100>>,
	/// Signature of the balance change hash
	pub signature: MultiSignature,
}

#[derive(Encode)]
struct BalanceChangeHashMessage {
	pub account_id: AccountId,
	pub account_type: AccountType,
	pub change_number: u32,
	pub balance: u128,
	pub channel_hold_note: Option<Note>,
	pub notes: Vec<Note>,
}

impl BalanceChange {
	pub fn push_note(&mut self, milligons: u128, note_type: NoteType) -> &mut Self {
		if let Some(existing) = self.notes.iter_mut().find(|n| n.note_type == note_type) {
			existing.milligons += milligons;
			return self;
		}
		let note = Note::create(milligons, note_type);
		self.notes.try_push(note).expect("Should be able to push note");
		self
	}

	#[cfg(feature = "std")]
	pub fn sign<S: Pair>(&mut self, pair: S) -> &Self
	where
		S::Signature: Into<MultiSignature>,
	{
		self.signature = pair.sign(&self.hash()[..]).into();
		self
	}

	pub fn end(&mut self) -> &Self {
		&*self
	}

	pub fn hash(&self) -> H256 {
		let hash = BalanceChangeHashMessage {
			account_id: self.account_id.clone(),
			account_type: self.account_type.clone(),
			change_number: self.change_number,
			balance: self.balance,
			channel_hold_note: self.channel_hold_note.clone(),
			notes: self.notes.to_vec(),
		};
		hash.using_encoded(blake2_256).into()
	}

	pub fn verify_signature(&self) -> bool {
		self.signature.verify(&self.hash()[..], &self.account_id)
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
pub struct BalanceProof {
	/// The notary where this proof can be verified
	#[codec(compact)]
	pub notary_id: NotaryId,
	/// The notebook where this proof can be verified
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	/// The tick where this proof can be verified
	#[codec(compact)]
	pub tick: Tick,
	/// The source balance being proven
	#[codec(compact)]
	pub balance: u128,
	/// The id created during the first balance change for the given account
	pub account_origin: AccountOrigin,
	/// Merkle proof from a closed notebook.
	///
	/// NOTE: This proof MUST be populated for the first entry in a notebook.
	pub notebook_proof: Option<MerkleProof>,
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
	Default,
)]
#[serde(rename_all = "camelCase")]
pub struct MerkleProof {
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

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
pub struct BalanceTip {
	pub account_id: AccountId,
	pub account_type: AccountType,
	pub change_number: u32,
	pub balance: u128,
	pub account_origin: AccountOrigin,
	pub channel_hold_note: Option<Note>,
}

impl BalanceTip {
	pub fn tip(&self) -> [u8; 32] {
		Self::compute_tip(
			self.change_number,
			self.balance,
			self.account_origin.clone(),
			self.channel_hold_note.clone(),
		)
	}

	pub fn compute_tip(
		change_number: u32,
		balance: u128,
		account_origin: AccountOrigin,
		channel_hold_note: Option<Note>,
	) -> [u8; 32] {
		BalanceTipValue { change_number, balance, account_origin, channel_hold_note }
			.using_encoded(blake2_256)
	}

	pub fn create_key(account_id: &AccountId, account_type: &AccountType) -> [u8; 32] {
		blake2_256(&[&account_id.as_ref(), &account_type.encode()[..]].concat())
	}
}

#[derive(Encode, Decode)]
struct BalanceTipValue {
	pub change_number: u32,
	pub balance: u128,
	pub account_origin: AccountOrigin,
	pub channel_hold_note: Option<Note>,
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
	pub notebook_number: NotebookNumber,
	/// A unique identifier for an account
	#[codec(compact)]
	pub account_uid: AccountOriginUid,
}
