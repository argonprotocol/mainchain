use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ConstU32, RuntimeDebug};
use sp_runtime::{format_runtime_string, BoundedVec, RuntimeString};

use crate::AccountId;

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
pub struct Note {
	/// Number of milligons transferred
	#[codec(compact)]
	pub milligons: u128,
	/// Type
	pub note_type: NoteType,
}

impl Note {
	pub fn create(milligons: u128, note_type: NoteType) -> Self {
		Self { milligons, note_type }
	}
}

#[derive(
	Clone,
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
#[serde(rename_all = "camelCase")]
pub enum AccountType {
	Tax = 0,
	Deposit = 1,
}

impl TryFrom<i32> for AccountType {
	type Error = RuntimeString;

	fn try_from(value: i32) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(AccountType::Tax),
			1 => Ok(AccountType::Deposit),
			_ => Err(format_runtime_string!("Invalid account_type value {}", value)),
		}
	}
}

pub const CHANNEL_EXPIRATION_NOTEBOOKS: u32 = 60;
pub const CHANNEL_CLAWBACK_NOTEBOOKS: u32 = 10; // 10 after expiration
pub const MIN_CHANNEL_NOTE_MILLIGONS: u128 = 5;
pub type MaxNoteRecipients = ConstU32<10>;

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
#[serde(tag = "op")]
#[serde(rename_all = "camelCase")]
pub enum NoteType {
	SendToMainchain,
	#[serde(rename_all = "camelCase")]
	ClaimFromMainchain {
		#[codec(compact)]
		account_nonce: u32,
	},
	/// Claim funds from a note (must be in the series of balance changes)
	Claim,
	/// Transfer funds to another address
	#[serde(rename_all = "camelCase")]
	Send {
		/// Recipient addresses (address of recipient party)
		to: Option<BoundedVec<AccountId, MaxNoteRecipients>>,
	},
	/// Pay a fee to a notary or mainchain service
	Fee,
	/// This note is a tax note
	Tax,
	/// Channel hold notes
	ChannelHold {
		/// The account id of the recipient
		recipient: AccountId,
	},
	/// Channel settlement note - sent to recipient of the original hold
	ChannelSettle,
	/// Claim funds from one or more channels - must be the recipient of the hold
	ChannelClaim,
}
