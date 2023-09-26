use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::{BoundedVec, CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use sp_core::Get;
use sp_std::{fmt::Debug, prelude::*};

use crate::{
	block_seal::{BlockSealAuthorityId, BlockSealAuthoritySignature},
	notary::NotaryId,
};

const NOTEBOOK_HASH_PREFIX: [u8; 13] = *b"notebook_hash";

#[derive(Encode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxTransfers, RequiredNotebookAuditors))]
pub struct NotebookHashMessage<AccountId, Balance, Nonce, MaxTransfers, RequiredNotebookAuditors>
where
	Nonce: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	Balance: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	AccountId: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	MaxTransfers: Get<u32>,
	RequiredNotebookAuditors: Get<u32>,
{
	pub prefix: [u8; 13],
	#[codec(compact)]
	pub pinned_to_block_number: u32,
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub transfers: BoundedVec<ChainTransfer<AccountId, Balance, Nonce>, MaxTransfers>,
	pub auditors:
		BoundedVec<(BlockSealAuthorityId, BlockSealAuthoritySignature), RequiredNotebookAuditors>,
}

pub fn to_notebook_post_hash<AccountId, Balance, Nonce, MaxTransfers, RequiredNotebookAuditors>(
	notebook: &Notebook<AccountId, Balance, Nonce, MaxTransfers, RequiredNotebookAuditors>,
) -> NotebookHashMessage<AccountId, Balance, Nonce, MaxTransfers, RequiredNotebookAuditors>
where
	Nonce: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	Balance: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	AccountId: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	MaxTransfers: Get<u32>,
	RequiredNotebookAuditors: Get<u32>,
{
	NotebookHashMessage {
		prefix: NOTEBOOK_HASH_PREFIX,
		pinned_to_block_number: notebook.pinned_to_block_number,
		notary_id: notebook.notary_id,
		transfers: notebook.transfers.clone(),
		auditors: notebook.auditors.clone(),
	}
}

const NOTEBOOK_AUDITOR_HASH_PREFIX: [u8; 20] = *b"notebook_audit_hash_";

#[derive(Encode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxTransfers))]
pub struct NotebookAuditorSignatureMessage<AccountId, Balance, Nonce, MaxTransfers>
where
	Nonce: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	Balance: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	AccountId: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	MaxTransfers: Get<u32>,
{
	pub prefix: [u8; 20],
	#[codec(compact)]
	pub pinned_to_block_number: u32,
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub transfers: BoundedVec<ChainTransfer<AccountId, Balance, Nonce>, MaxTransfers>,
}

pub fn to_notebook_audit_signature_message<
	AccountId,
	Balance,
	Nonce,
	MaxTransfers,
	RequiredNotebookAuditors,
>(
	notebook: &Notebook<AccountId, Balance, Nonce, MaxTransfers, RequiredNotebookAuditors>,
) -> NotebookAuditorSignatureMessage<AccountId, Balance, Nonce, MaxTransfers>
where
	Nonce: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	Balance: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	AccountId: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	MaxTransfers: Get<u32>,
	RequiredNotebookAuditors: Get<u32>,
{
	NotebookAuditorSignatureMessage {
		prefix: NOTEBOOK_AUDITOR_HASH_PREFIX,
		pinned_to_block_number: notebook.pinned_to_block_number,
		notary_id: notebook.notary_id,
		transfers: notebook.transfers.clone(),
	}
}

#[derive(
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
	Encode,
	Decode,
	RuntimeDebugNoBound,
	TypeInfo,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxTransfers, RequiredNotebookAuditors))]
pub struct Notebook<AccountId, Balance, Nonce, MaxTransfers, RequiredNotebookAuditors>
where
	Nonce: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	Balance: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	AccountId: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	MaxTransfers: Get<u32>,
	RequiredNotebookAuditors: Get<u32>,
{
	#[codec(compact)]
	pub pinned_to_block_number: u32,
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub transfers: BoundedVec<ChainTransfer<AccountId, Balance, Nonce>, MaxTransfers>,
	pub auditors:
		BoundedVec<(BlockSealAuthorityId, BlockSealAuthoritySignature), RequiredNotebookAuditors>,
}

#[derive(
	CloneNoBound, PartialEq, Eq, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
)]
pub enum ChainTransfer<AccountId, Balance, Nonce>
where
	Nonce: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	Balance: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	AccountId: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
{
	ToMainchain {
		account_id: AccountId,
		#[codec(compact)]
		amount: Balance,
	},
	ToLocalchain {
		account_id: AccountId,
		#[codec(compact)]
		nonce: Nonce,
	},
}

#[derive(
	CloneNoBound, PartialEq, Eq, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[codec(mel_bound(LocalchainAccountId: MaxEncodedLen, LocalchainSignature: MaxEncodedLen))]
pub struct ChainTransferSignature<
	LocalchainAccountId: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
	LocalchainSignature: MaxEncodedLen + Codec + Debug + Clone + PartialEq + Eq,
> {
	#[codec(compact)]
	pub transfer_index: u32,

	pub source_balance_account_id: Option<LocalchainAccountId>,
	pub source_balance_signature: Option<LocalchainSignature>,
}
