use alloc::{
	fmt::{Display, Formatter, Result},
	vec::Vec,
};

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{
	crypto::{Ss58AddressFormat, Ss58Codec},
	ConstU32, RuntimeDebug,
};
use sp_runtime::BoundedVec;

#[cfg(feature = "std")]
use crate::serialize_unsafe_u128_as_string;
use crate::{AccountId, DataDomainHash, TransferToLocalchainId, ADDRESS_PREFIX};

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
	#[cfg_attr(feature = "std", serde(with = "serialize_unsafe_u128_as_string"))]
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

impl Display for Note {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		let argons = self.milligons as f64 / 1000.0;
		if self.milligons % 1000 == 0 || self.milligons % 100 == 0 {
			write!(f, "{} ₳{:.1}", self.note_type, argons)
		} else if self.milligons % 10 == 0 {
			write!(f, "{} ₳{:.2}", self.note_type, argons)
		} else {
			write!(f, "{} ₳{:.3}", self.note_type, argons)
		}
	}
}

pub fn round_up(value: u128, percentage: u128) -> u128 {
	let numerator = value * percentage;

	let round = if numerator % 100 == 0 { 0 } else { 1 };

	numerator.saturating_div(100) + round
}

pub const ESCROW_CLAWBACK_TICKS: u32 = 15;
// 15 after expiration
pub const MINIMUM_ESCROW_SETTLEMENT: u128 = 5u128;

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
		transfer_id: TransferToLocalchainId,
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
		/// Delegate signing permissions to another account
		delegated_signer: Option<AccountId>,
	},
	/// Escrow settlement note - applied to escrow hold creator balance
	EscrowSettle,
	/// Claim funds from one or more escrows - must be the recipient of the hold
	EscrowClaim,
}

impl Display for NoteType {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		match self {
			NoteType::SendToMainchain => {
				write!(f, "SendToMainchain")
			},
			NoteType::ClaimFromMainchain { transfer_id } => {
				write!(f, "ClaimFromMainchain(transfer_id={})", transfer_id)
			},
			NoteType::Claim => {
				write!(f, "Claim")
			},
			NoteType::Send { to } =>
				if let Some(to) = to {
					write!(
						f,
						"Send(restrictedTo: {:?})",
						to.iter()
							.map(|a| a
								.to_ss58check_with_version(Ss58AddressFormat::from(ADDRESS_PREFIX)))
							.collect::<Vec<_>>()
					)
				} else {
					write!(f, "Send")
				},
			NoteType::LeaseDomain => {
				write!(f, "LeaseDomain")
			},
			NoteType::Fee => {
				write!(f, "Fee")
			},
			NoteType::Tax => {
				write!(f, "Tax")
			},
			NoteType::SendToVote => {
				write!(f, "SendToVote")
			},
			NoteType::EscrowHold { data_domain_hash, recipient, delegated_signer } => {
				write!(
					f,
					"EscrowHold(data_domain_hash: {:?}, recipient: {:?}, delegated_signer: {:?})",
					data_domain_hash,
					recipient.to_ss58check_with_version(Ss58AddressFormat::from(ADDRESS_PREFIX)),
					delegated_signer
						.as_ref()
						.map(|a| a
							.to_ss58check_with_version(Ss58AddressFormat::from(ADDRESS_PREFIX)))
				)
			},
			NoteType::EscrowSettle => {
				write!(f, "EscrowSettle")
			},
			NoteType::EscrowClaim => {
				write!(f, "EscrowClaim")
			},
		}
	}
}
