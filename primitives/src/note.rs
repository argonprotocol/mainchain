use alloc::{
	fmt::{Display, Formatter, Result},
	vec::Vec,
};
use polkadot_sdk::*;

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
use crate::{tick::Tick, AccountId, Balance, DomainHash, TransferToLocalchainId, ADDRESS_PREFIX};

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
	/// Amount transferred (units are in 1/1_000_000th of an argon)
	#[codec(compact)]
	#[cfg_attr(feature = "std", serde(with = "serialize_unsafe_u128_as_string"))]
	pub microgons: Balance,
	/// Type
	pub note_type: NoteType,
}

impl Note {
	pub fn create(amount: Balance, note_type: NoteType) -> Self {
		Self { microgons: amount, note_type }
	}

	pub fn calculate_transfer_tax(amount: Balance) -> Balance {
		// less than one argon is percent based
		if amount < 1_000_000 {
			round_up(amount, TAX_PERCENT_BASE)
		} else {
			TRANSFER_TAX_CAP
		}
	}

	pub fn calculate_channel_hold_tax(amount: u128) -> u128 {
		round_up(amount, TAX_PERCENT_BASE)
	}
}

impl Display for Note {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		let argons = self.microgons as f64 / 1_000_000.0;

		if self.microgons % 1_000_000 == 0 || self.microgons % 100_000 == 0 {
			write!(f, "{} ₳{:.1}", self.note_type, argons)
		} else if self.microgons % 10_000 == 0 {
			write!(f, "{} ₳{:.2}", self.note_type, argons)
		} else if self.microgons % 1_000 == 0 {
			write!(f, "{} ₳{:.3}", self.note_type, argons)
		} else if self.microgons % 100 == 0 {
			write!(f, "{} ₳{:.4}", self.note_type, argons)
		} else if self.microgons % 10 == 0 {
			write!(f, "{} ₳{:.5}", self.note_type, argons)
		} else {
			write!(f, "{} ₳{:.6}", self.note_type, argons)
		}
	}
}

pub fn round_up(value: u128, percentage: u128) -> u128 {
	let numerator = value * percentage;

	let round = if numerator % 100 == 0 { 0 } else { 1 };

	numerator.saturating_div(100) + round
}

pub const CHANNEL_HOLD_CLAWBACK_TICKS: Tick = 15;
// 15 after expiration
pub const MINIMUM_CHANNEL_HOLD_SETTLEMENT: Balance = 5_000u128;

pub type MaxNoteRecipients = ConstU32<10>;

pub const TAX_PERCENT_BASE: u128 = 20;
pub const TRANSFER_TAX_CAP: Balance = 200_000;

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
	/// Lease a domain
	LeaseDomain,
	/// Pay a fee to a notary or mainchain service
	Fee,
	/// This note is a tax note
	Tax,
	/// Send this tax to a BlockVote
	SendToVote,
	/// Channel hold notes
	#[serde(rename_all = "camelCase")]
	ChannelHold {
		/// The account id of the recipient
		recipient: AccountId,
		/// Delegate signing permissions to another account
		delegated_signer: Option<AccountId>,
		/// Optional domain hash this channel is held for
		domain_hash: Option<DomainHash>,
	},
	/// ChannelHold settlement note - applied to channel hold creator balance
	ChannelHoldSettle,
	/// Claim funds from one or more channel_holds - must be the recipient of the hold
	ChannelHoldClaim,
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
			NoteType::ChannelHold { recipient, domain_hash, delegated_signer } => {
				write!(
					f,
					"ChannelHold(recipient: {:?}, delegated_signer: {:?}, domain_hash: {:?})",
					recipient.to_ss58check_with_version(Ss58AddressFormat::from(ADDRESS_PREFIX)),
					delegated_signer
						.as_ref()
						.map(|a| a
							.to_ss58check_with_version(Ss58AddressFormat::from(ADDRESS_PREFIX))),
					domain_hash
				)
			},
			NoteType::ChannelHoldSettle => {
				write!(f, "ChannelHoldSettle")
			},
			NoteType::ChannelHoldClaim => {
				write!(f, "ChannelHoldClaim")
			},
		}
	}
}
