use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ConstU32, RuntimeDebug};
use sp_runtime::{format_runtime_string, BoundedVec, RuntimeString};

use crate::{prod_or_fast, AccountId, DataDomainHash};

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

	pub fn calculate_transfer_tax(amount: u128) -> u128 {
		if amount < 1000 {
			round_up(amount, TAX_PERCENT_BASE)
		} else {
			TRANSFER_TAX_CAP
		}
	}

	pub fn calculate_escrow_tax(amount: u128) -> u128 {
		round_up(amount, TAX_PERCENT_BASE)
	}
}

pub fn round_up(value: u128, percentage: u128) -> u128 {
	let numerator = value * percentage;

	let round = if numerator % 100 == 0 { 0 } else { 1 };

	numerator.saturating_div(100) + round
}

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
#[cfg_attr(not(feature = "napi"), derive(Clone))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "napi", napi_derive::napi)]
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
impl From<i64> for AccountType {
	fn from(value: i64) -> Self {
		if value == 0 {
			AccountType::Tax
		} else {
			AccountType::Deposit
		}
	}
}

pub const ESCROW_EXPIRATION_TICKS: u32 = prod_or_fast!(60, 2);
pub const ESCROW_CLAWBACK_TICKS: u32 = 15; // 15 after expiration
pub const MIN_ESCROW_NOTE_MILLIGONS: u128 = 5;
pub type MaxNoteRecipients = ConstU32<10>;

pub const TAX_PERCENT_BASE: u128 = 20;
pub const TRANSFER_TAX_CAP: u128 = 200;

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
#[serde(tag = "action")]
#[serde(rename_all = "camelCase")]
pub enum NoteType {
	/// Return funds to the mainchain in the next notebook
	SendToMainchain,
	/// Claim funds that were sent from a mainchain account to localchain via the chain_transfer
	/// pallet
	#[serde(rename_all = "camelCase")]
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
	/// Escrow hold notes
	#[serde(rename_all = "camelCase")]
	EscrowHold {
		/// The account id of the recipient
		recipient: AccountId,
		/// The data domain that this escrow is created for
		data_domain_hash: Option<DataDomainHash>,
	},
	/// Escrow settlement note - applied to escrow hold creator balance
	EscrowSettle,
	/// Claim funds from one or more escrows - must be the recipient of the hold
	EscrowClaim,
}
