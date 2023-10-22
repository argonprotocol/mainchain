use std::fmt::Debug;

use codec::{Codec, Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{blake2_256, crypto::AccountId32, ed25519::Signature, ByteArray, RuntimeDebug, H256};
use sp_runtime::MultiSignature;

pub type NoteId = H256;
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
	/// Hash of Scale encoded (milligons, note_type)
	pub note_id: NoteId,
	/// Number of milligons transferred
	#[codec(compact)]
	pub milligons: u128,
	/// Type
	pub note_type: NoteType,
	/// Hash signed by sender address
	pub signature: MultiSignature,
}

impl Note {
	pub fn create_unsigned(
		account_id: &AccountId32,
		chain: &Chain,
		balance_change_nonce: u32,
		milligons: u128,
		note_type: NoteType,
	) -> Self {
		Self {
			note_id: Self::compute_note_id(
				account_id,
				chain,
				balance_change_nonce,
				milligons,
				&note_type,
			),
			milligons,
			note_type,
			signature: Signature([0u8; 64]).into(),
		}
	}
	pub fn get_note_id(
		&self,
		account_id: &AccountId32,
		chain: &Chain,
		balance_change_nonce: u32,
	) -> NoteId {
		blake2_256(
			&[
				&account_id.as_slice()[..],
				&chain.encode()[..],
				&balance_change_nonce.to_le_bytes()[..],
				&self.milligons.to_le_bytes()[..],
				&self.note_type.encode()[..],
			]
			.concat(),
		)
		.into()
	}

	pub fn compute_note_id(
		account_id: &AccountId32,
		chain: &Chain,
		balance_change_nonce: u32,
		milligons: u128,
		note_type: &NoteType,
	) -> NoteId {
		blake2_256(
			&[
				&account_id.as_slice()[..],
				&chain.encode()[..],
				&balance_change_nonce.to_le_bytes()[..],
				&milligons.to_le_bytes()[..],
				&note_type.encode()[..],
			]
			.concat(),
		)
		.into()
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
pub enum Chain {
	Tax = 0,
	Argon = 1,
}

impl TryFrom<i32> for Chain {
	type Error = String;

	fn try_from(value: i32) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Chain::Tax),
			1 => Ok(Chain::Argon),
			_ => Err(format!("Invalid chain value {}", value)),
		}
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
pub enum NoteType {
	/// Transfer between local and mainchain
	ChainTransfer { destination: ChainTransferDestination, finalized_at_block: u32 },
	/// Claim funds from a note
	Claim,
	/// Transfer funds to another address
	Send {
		/// Recipient address  (address of recipient party) - If this is a tax note, it must be a
		/// proof of tax address (or it won’t be able to be used).
		recipient: Option<AccountId32>,
	},
	/// Pay a fee to a notary or mainchain service
	Fee,
	/// This note is a tax note
	Tax,
	/// Channel notes
	Channel,
	/// Channel settlement note
	ChannelSettle {
		/// Source channel note
		source_note_id: NoteId,
	},
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
pub enum ChainTransferDestination {
	ToMainchain,
	ToLocalchain {
		#[codec(compact)]
		nonce: u32,
	},
}
pub trait MemberEncode: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq {}
