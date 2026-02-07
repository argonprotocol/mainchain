use crate::AccountHistoryLookupError;
use alloc::{string::String, vec::Vec};
use argon_primitives::{
	AccountType, MINIMUM_CHANNEL_HOLD_SETTLEMENT,
	prelude::{frame_support::BoundedVec, sp_core::ConstU32},
	tick::Tick,
};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::fmt::Display;
use polkadot_sdk::*;
use serde::{Deserialize, Serialize};
use sp_core::crypto::AccountId32;
use sp_runtime::scale_info::TypeInfo;
use thiserror::Error;

#[derive(
	Debug,
	PartialEq,
	Clone,
	TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Serialize,
	Deserialize,
	MaxEncodedLen,
)]
#[repr(transparent)]
#[serde(transparent)]
pub struct MaxLengthString(pub BoundedVec<u8, ConstU32<100>>);

impl Display for MaxLengthString {
	fn fmt(&self, f: &mut alloc::fmt::Formatter<'_>) -> alloc::fmt::Result {
		let str_value = String::from_utf8(self.0.clone().into_inner()).unwrap_or_default();
		write!(f, "{str_value}")
	}
}

impl TryFrom<String> for MaxLengthString {
	type Error = Vec<u8>;
	fn try_from(s: String) -> Result<Self, Self::Error> {
		let vec = BoundedVec::try_from(s.into_bytes())?;
		Ok(Self(vec))
	}
}

#[derive(
	Debug,
	PartialEq,
	Clone,
	Error,
	TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Serialize,
	Deserialize,
	MaxEncodedLen,
)]
pub enum VerifyError {
	#[error("Missing account origin {account_id:?}, {account_type:?}")]
	MissingAccountOrigin { account_id: AccountId32, account_type: AccountType },
	#[error("Account history lookup error {source}")]
	HistoryLookupError {
		#[from]
		source: AccountHistoryLookupError,
	},
	#[error("Invalid account changelist")]
	InvalidAccountChangelist,
	#[error("Invalid chain transfers list")]
	InvalidChainTransfersList,
	#[error("Invalid balance change root")]
	InvalidBalanceChangeRoot,

	#[error("Invalid notebook tax recorded")]
	InvalidHeaderTaxRecorded,

	#[error("Invalid previous nonce")]
	InvalidPreviousNonce,
	#[error("Invalid previous balance")]
	InvalidPreviousBalance,
	#[error("Invalid previous account origin")]
	InvalidPreviousAccountOrigin,

	#[error("Invalid previous balance change notebook")]
	InvalidPreviousBalanceChangeNotebook,

	#[error("Invalid net balance change calculated")]
	InvalidBalanceChange,

	#[error("Invalid balance change signature #{change_index}")]
	InvalidBalanceChangeSignature { change_index: u16 },

	#[error("Some notes with restricted recipients did not balance to zero.")]
	InvalidNoteRecipients,

	#[error("An invalid balance change was submitted (#{change_index}.{note_index}): {message}")]
	BalanceChangeError { change_index: u16, note_index: u16, message: MaxLengthString },

	#[error("Invalid net balance changeset. Must account for all funds.")]
	InvalidNetBalanceChangeset,

	#[error(
		"Insufficient balance for account  (balance: {balance}, amount: {amount}) (change: #{change_index}.{note_index})"
	)]
	InsufficientBalance { balance: u128, amount: u128, note_index: u16, change_index: u16 },

	#[error(
		"Exceeded max balance for account (pre-balance: {balance}, amount: {amount}), (change: #{change_index}.{note_index})"
	)]
	ExceededMaxBalance { balance: u128, amount: u128, note_index: u16, change_index: u16 },
	#[error(
		"Balance change mismatch (provided_balance: {provided_balance}, calculated_balance: {calculated_balance}) (#{change_index})"
	)]
	BalanceChangeMismatch { change_index: u16, provided_balance: u128, calculated_balance: i128 },

	#[error("Balance change not net zero (sent: {sent} vs claimed: {claimed})")]
	BalanceChangeNotNetZero { sent: u128, claimed: u128 },

	#[error("Invalid domain lease allocation")]
	InvalidDomainLeaseAllocation,

	#[error("Tax balance changes not net zero (sent: {sent} vs claimed: {claimed})")]
	TaxBalanceChangeNotNetZero { sent: u128, claimed: u128 },

	#[error("Must include proof of previous balance")]
	MissingBalanceProof,

	#[error("Invalid previous balance proof")]
	InvalidPreviousBalanceProof,

	#[error("Invalid notebook hash")]
	InvalidNotebookHash,

	#[error("Invalid notebook header hash")]
	InvalidNotebookHeaderHash,

	#[error("Duplicate chain transfer")]
	DuplicateChainTransfer,

	#[error("Duplicated account origin uid")]
	DuplicatedAccountOriginUid,

	#[error("Invalid notary signature")]
	InvalidNotarySignature,

	#[error("Invalid secret revealed")]
	InvalidSecretProvided,

	#[error("Submitted notebook older than most recent in storage")]
	NotebookTooOld,

	#[error("Missing needed catchup notebooks")]
	CatchupNotebooksMissing,

	#[error("Error decoding notebook")]
	DecodeError,

	#[error("Account does not have a channel hold")]
	AccountChannelHoldDoesntExist,

	#[error("Account already has a channel hold")]
	AccountAlreadyHasChannelHold,

	#[error(
		"Channel hold not ready for claim. Current tick: {current_tick}, claim tick: {claim_tick}"
	)]
	ChannelHoldNotReadyForClaim { current_tick: Tick, claim_tick: Tick },

	#[error("This account is locked with a channel hold")]
	AccountLocked,

	#[error("A channel hold note is required to unlock this account")]
	MissingChannelHoldNote,

	#[error("Invalid channel hold note")]
	InvalidChannelHoldNote,

	#[error("Invalid channel_hold claimers")]
	InvalidChannelHoldClaimers,

	#[error(
		"This channel hold note is below the minimum amount required ({MINIMUM_CHANNEL_HOLD_SETTLEMENT})"
	)]
	ChannelHoldNoteBelowMinimum,

	#[error("Tax notes can only be created from deposit accounts")]
	InvalidTaxNoteAccount,

	#[error("Invalid tax account operation")]
	InvalidTaxOperation,

	#[error(
		"Invalid tax amount included (sent: {tax_sent}, owed: {tax_owed}) for account {account_id:?}"
	)]
	InsufficientTaxIncluded { tax_sent: u128, tax_owed: u128, account_id: AccountId32 },

	#[error("Insufficient tax allocated for the given block votes")]
	InsufficientBlockVoteTax,

	#[error("The account voting does not have any tax funds available")]
	IneligibleTaxVoter,

	#[error("Invalid block vote signature")]
	BlockVoteInvalidSignature,

	#[error("Invalid block vote allocation")]
	InvalidBlockVoteAllocation,

	#[error("Invalid block votes root")]
	InvalidBlockVoteRoot,

	#[error("Invalid block votes count")]
	InvalidBlockVotesCount,

	#[error("Invalid block voting power")]
	InvalidBlockVotingPower,

	#[error("Invalid block vote list")]
	InvalidBlockVoteList,

	#[error("Invalid block vote compute nonce provided")]
	InvalidComputeProof,

	#[error("Invalid block vote")]
	InvalidBlockVoteSource,

	#[error("Minimums were not met for a block vote")]
	InsufficientBlockVoteMinimum,

	#[error("Invalid block vote tick. Vote tick: {tick}, notebook tick: {notebook_tick}")]
	InvalidBlockVoteTick { tick: Tick, notebook_tick: Tick },

	#[error("Default block vote only eligible if no other votes")]
	InvalidDefaultBlockVote,

	#[error(
		"Default block vote created by invalid author. Expected: {expected:?}, found: {author:?}"
	)]
	InvalidDefaultBlockVoteAuthor { author: AccountId32, expected: AccountId32 },

	#[error("No default block vote included")]
	NoDefaultBlockVote,
}
