use crate::{AccountOrigin, BalanceChange, Chain};
use codec::{Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{
	blake2_256, bounded::BoundedVec, crypto::AccountId32, ed25519, ConstU32, RuntimeDebug, H256,
};
use sp_runtime::scale_info::TypeInfo;

pub const PINNED_BLOCKS_OFFSET: u32 = 100u32;
pub const MAX_TRANSFERS: u32 = 1000;
pub type MaxTransfers = ConstU32<MAX_TRANSFERS>;
pub type MaxBalanceChanges = ConstU32<10_000>;
pub type NotaryId = u32;
pub type AccountOriginUid = u32;

pub type NotebookAccountOrigin = (AccountId32, Chain, AccountOriginUid);
pub type RequiredNotebookAuditors = ConstU32<10>;
pub const NOTEBOOK_VERSION: u16 = 1;

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
pub struct Notebook {
	pub header: NotebookHeader,
	pub balance_changes:
		BoundedVec<BoundedVec<BalanceChange, MaxBalanceChanges>, MaxBalanceChanges>,
	pub new_account_origins: BoundedVec<NotebookAccountOrigin, MaxBalanceChanges>,
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
pub struct NotebookHeader {
	#[codec(compact)]
	pub version: u16,
	#[codec(compact)]
	pub notebook_number: u32,
	#[codec(compact)]
	pub finalized_block_number: u32,
	#[codec(compact)]
	pub pinned_to_block_number: u32,

	pub start_time: u64,
	pub end_time: Option<u64>,
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub chain_transfers: BoundedVec<ChainTransfer, MaxTransfers>,
	/// A merkle root for all account balances changed in this notebook.
	/// Nodes are in the order of when each account is seen in the notebook.
	/// Nodes contain the account id, chain, nonce, balance and account origin.
	/// If a node is in the balance changes twice, only the last entry will be used.
	/// Nodes are encoded as Scale and hashed with Blake2 256  
	pub changed_accounts_root: H256,
	/// All ids that are changed in this notebook. A notebook id is a tuple of (origin notebook,
	/// counter)
	pub changed_account_origins: BoundedVec<AccountOrigin, ConstU32<{ u32::MAX }>>,
	pub auditors: BoundedVec<(ed25519::Public, ed25519::Signature), RequiredNotebookAuditors>,
}

impl NotebookHeader {
	pub fn hash(&self) -> H256 {
		Into::<NotebookHeaderHashMessage>::into(self).using_encoded(blake2_256).into()
	}
}

#[derive(Encode)]
pub struct NotebookHeaderHashMessage {
	#[codec(compact)]
	pub version: u16,
	#[codec(compact)]
	pub notebook_number: u32,
	#[codec(compact)]
	pub finalized_block_number: u32,
	#[codec(compact)]
	pub pinned_to_block_number: u32,

	#[codec(compact)]
	pub start_time: u64,
	#[codec(compact)]
	pub end_time: u64,
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub chain_transfers: BoundedVec<ChainTransfer, MaxTransfers>,
	pub changed_accounts_root: H256,
	pub changed_account_origins: BoundedVec<AccountOrigin, ConstU32<{ u32::MAX }>>,
}

impl Into<NotebookHeaderHashMessage> for &NotebookHeader {
	fn into(self) -> NotebookHeaderHashMessage {
		NotebookHeaderHashMessage {
			version: self.version,
			notebook_number: self.notebook_number,
			finalized_block_number: self.finalized_block_number,
			pinned_to_block_number: self.pinned_to_block_number,
			start_time: self.start_time,
			end_time: self.end_time.unwrap_or(0),
			notary_id: self.notary_id,
			chain_transfers: self.chain_transfers.clone(),
			changed_accounts_root: self.changed_accounts_root,
			changed_account_origins: self.changed_account_origins.clone(),
		}
	}
}

#[derive(
	Clone,
	PartialEq,
	PartialOrd,
	Ord,
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
pub enum ChainTransfer {
	ToMainchain {
		account_id: AccountId32,
		#[codec(compact)]
		amount: u128,
	},
	ToLocalchain {
		account_id: AccountId32,
		#[codec(compact)]
		nonce: u32,
	},
}
