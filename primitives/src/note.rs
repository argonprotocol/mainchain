use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ConstU32, RuntimeDebug};
use sp_runtime::{format_runtime_string, BoundedVec, RuntimeString};

use crate::{prod_or_fast, AccountId, DataDomain};

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

pub const CHANNEL_EXPIRATION_NOTEBOOKS: u32 = prod_or_fast!(60, 2);
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
	/// Return funds to the mainchain in the next notebook
	SendToMainchain,
	/// Claim funds that were sent from a mainchain account to localchain via the chain_transfer
	/// pallet
	ClaimFromMainchain {
		#[codec(compact)]
		account_nonce: u32,
	},
	/// Claim funds from a note (must be in the series of balance changes)
	Claim,
	/// Transfer funds to another address
	Send {
		/// Recipient addresses (address of recipient party)
		to: Option<BoundedVec<AccountId, MaxNoteRecipients>>,
	},
	/// Lease a data domain
	LeaseDomain,
	/// Pay a fee to a notary or mainchain service
	Fee,
	/// This note is a tax note
	Tax,
	/// Send this tax to a BlockVote
	SendToVote,
	/// Channel hold notes
	ChannelHold {
		/// The account id of the recipient
		recipient: AccountId,
		/// The data domain that this channel is created for
		data_domain: Option<DataDomain>,
	},
	/// Channel settlement note - applied to channel hold creator balance
	ChannelSettle,
	/// Claim funds from one or more channels - must be the recipient of the hold
	ChannelClaim,
}
