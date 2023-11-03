use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use sp_core::crypto::AccountId32;
use sp_runtime::{scale_info::TypeInfo, RuntimeString};

use crate::AccountHistoryLookupError;
use ulx_notary_primitives::AccountType;

const MIN_CHANNEL_NOTE_MILLIGONS: u128 = ulx_notary_primitives::MIN_CHANNEL_NOTE_MILLIGONS;
#[derive(Debug, PartialEq, Clone, Snafu, TypeInfo, Encode, Decode, Serialize, Deserialize)]
pub enum VerifyError {
	#[snafu(display("Missing account origin {account_id:?}, {account_type:?}"))]
	MissingAccountOrigin { account_id: AccountId32, account_type: AccountType },
	#[snafu(display("Account history lookup error {source}"))]
	HistoryLookupError {
		#[snafu(source(from(AccountHistoryLookupError, AccountHistoryLookupError::from)))]
		source: AccountHistoryLookupError,
	},
	#[snafu(display("Invalid account changelist"))]
	InvalidAccountChangelist,
	#[snafu(display("Invalid chain transfers list"))]
	InvalidChainTransfersList,
	#[snafu(display("Invalid balance change root"))]
	InvalidBalanceChangeRoot,

	#[snafu(display("Invalid previous nonce"))]
	InvalidPreviousNonce,
	#[snafu(display("Invalid previous balance"))]
	InvalidPreviousBalance,
	#[snafu(display("Invalid previous account origin"))]
	InvalidPreviousAccountOrigin,

	#[snafu(display("Invalid previous balance change notebook"))]
	InvalidPreviousBalanceChangeNotebook,

	#[snafu(display("Invalid net balance change calculated"))]
	InvalidBalanceChange,

	#[snafu(display("Invalid balance change signature {change_index}"))]
	InvalidBalanceChangeSignature { change_index: u16 },

	#[snafu(display("Invalid note recipients for a claimed note"))]
	InvalidNoteRecipients,

	#[snafu(display(
		"An invalid balance change was submitted ({change_index}.{note_index}): {message:?}"
	))]
	BalanceChangeError { change_index: u16, note_index: u16, message: RuntimeString },

	#[snafu(display("Invalid net balance changeset. Must account for all funds."))]
	InvalidNetBalanceChangeset,

	#[snafu(display("Insufficient balance for account  (balance: {balance}, amount: {amount}) (change: {change_index}.{note_index})"))]
	InsufficientBalance { balance: u128, amount: u128, note_index: u16, change_index: u16 },

	#[snafu(display("Exceeded max balance for account (pre-balance: {balance}, amount: {amount}), (change: {change_index}.{note_index})"))]
	ExceededMaxBalance { balance: u128, amount: u128, note_index: u16, change_index: u16 },
	#[snafu(display("Balance change mismatch (provided_balance: {provided_balance}, calculated_balance: {calculated_balance}) (#{change_index})"))]
	BalanceChangeMismatch { change_index: u16, provided_balance: u128, calculated_balance: i128 },

	#[snafu(display("Balance change not net zero (sent: {sent} vs claimed: {claimed})"))]
	BalanceChangeNotNetZero { sent: u128, claimed: u128 },
	#[snafu(display("Tax balance changes not net zero (sent: {sent} vs claimed: {claimed})"))]
	TaxBalanceChangeNotNetZero { sent: u128, claimed: u128 },

	#[snafu(display("Must include proof of previous balance"))]
	MissingBalanceProof,
	#[snafu(display("Invalid previous balance proof"))]
	InvalidPreviousBalanceProof,
	#[snafu(display("Invalid notebook hash"))]
	InvalidNotebookHash,

	#[snafu(display("Duplicate chain transfer"))]
	DuplicateChainTransfer,

	#[snafu(display("Duplicated account origin uid"))]
	DuplicatedAccountOriginUid,

	#[snafu(display("Invalid notary signature"))]
	InvalidNotarySignature,

	#[snafu(display("Submitted notebook older than most recent in storage"))]
	NotebookTooOld,

	#[snafu(display("Error decoding notebook"))]
	DecodeError,

	#[snafu(display("Account does not have a channel hold"))]
	AccountChannelHoldDoesntExist,

	#[snafu(display("Account already has a channel hold"))]
	AccountAlreadyHasChannelHold,

	#[snafu(display("Channel hold not ready for claim"))]
	ChannelHoldNotReadyForClaim,

	#[snafu(display("Channel hold already claimed"))]
	ChannelHoldAlreadyClaimed,

	#[snafu(display("Channel hold not ready for settle"))]
	ChannelHoldNotReadyForSettle,

	#[snafu(display("This account is locked with a channel hold"))]
	AccountLocked,

	#[snafu(display("A channel hold note is required to unlock this account"))]
	MissingChannelHoldNote,

	#[snafu(display("Invalid channel hold note"))]
	InvalidChannelHoldNote,

	#[snafu(display("Invalid channel claimers"))]
	InvalidChannelClaimers,

	#[snafu(display(
		"This channel note is below the minimum amount required ({MIN_CHANNEL_NOTE_MILLIGONS})"
	))]
	ChannelNoteBelowMinimum,

	#[snafu(display("Tax notes can only be created from deposit accounts"))]
	InvalidTaxNoteAccount,

	#[snafu(display("Invalid tax account operation"))]
	InvalidTaxOperation,

	#[snafu(display("Invalid tax amount included (sent: {tax_sent}, owed: {tax_owed}) for account {account_id:?}"))]
	InsufficientTaxIncluded { tax_sent: u128, tax_owed: u128, account_id: AccountId32 },
}

impl From<AccountHistoryLookupError> for VerifyError {
	fn from(e: AccountHistoryLookupError) -> Self {
		VerifyError::HistoryLookupError { source: e }
	}
}
