use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{bounded::BoundedVec, ed25519::Signature, ConstU32, H256};
use sp_crypto_hashing::blake2_256;
use sp_debug_derive::RuntimeDebug;
use sp_std::vec::Vec;

use crate::{
	balance_change::BalanceChange, block_vote::BlockVote, notary::NotarySignature, tick::Tick,
	AccountId, AccountType, DataDomainHash, NotaryId,
};
pub use crate::{AccountOrigin, BalanceTip};

pub const MAX_BALANCE_CHANGES_PER_NOTARIZATION: u32 = 25;
pub const MAX_BLOCK_VOTES_PER_NOTARIZATION: u32 = 100;
pub const MAX_DOMAINS_PER_NOTARIZATION: u32 = 100;

pub type NotarizationBalanceChangeset =
	BoundedVec<BalanceChange, ConstU32<MAX_BALANCE_CHANGES_PER_NOTARIZATION>>;
pub type NotarizationBlockVotes = BoundedVec<BlockVote, ConstU32<MAX_BLOCK_VOTES_PER_NOTARIZATION>>;
pub type NotarizationDataDomains =
	BoundedVec<(DataDomainHash, AccountId), ConstU32<MAX_DOMAINS_PER_NOTARIZATION>>;

pub const MAX_NOTEBOOK_TRANSFERS: u32 = 10_000;
pub const MAX_NOTARIZATIONS_PER_NOTEBOOK: u32 = 100_000;
pub const MAX_BLOCK_VOTES_PER_NOTEBOOK: u32 =
	MAX_BLOCK_VOTES_PER_NOTARIZATION * MAX_NOTARIZATIONS_PER_NOTEBOOK;
pub const MAX_DOMAINS_PER_NOTEBOOK: u32 =
	MAX_NOTARIZATIONS_PER_NOTEBOOK * MAX_DOMAINS_PER_NOTARIZATION;
pub type MaxNotebookTransfers = ConstU32<MAX_NOTEBOOK_TRANSFERS>;
pub type MaxNotebookNotarizations = ConstU32<MAX_NOTARIZATIONS_PER_NOTEBOOK>;
pub type MaxNotebookBlockVotes = ConstU32<MAX_BLOCK_VOTES_PER_NOTEBOOK>;
pub type MaxDataDomainsPerNotebook = ConstU32<MAX_DOMAINS_PER_NOTEBOOK>;
pub type AccountOriginUid = u32;
pub type NotebookNumber = u32;

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
#[serde(rename_all = "camelCase")]
pub struct Notebook {
	pub header: NotebookHeader,
	pub notarizations: BoundedVec<Notarization, MaxNotebookNotarizations>,
	pub new_account_origins: BoundedVec<NewAccountOrigin, MaxNotebookNotarizations>,
	pub hash: H256,
	pub signature: Signature,
}

#[derive(Encode)]
struct NotebookHashMessage {
	header_hash: H256,
	notarizations: BoundedVec<Notarization, MaxNotebookNotarizations>,
	new_account_origins: BoundedVec<NewAccountOrigin, MaxNotebookNotarizations>,
}

impl Notebook {
	pub fn build(
		header: NotebookHeader,
		notarizations: Vec<Notarization>,
		new_account_origins: Vec<NewAccountOrigin>,
	) -> Self {
		let notarizations = BoundedVec::truncate_from(notarizations);
		let new_account_origins = BoundedVec::truncate_from(new_account_origins);
		let hash = NotebookHashMessage {
			header_hash: header.hash(),
			notarizations: notarizations.clone(),
			new_account_origins: new_account_origins.clone(),
		}
		.using_encoded(blake2_256);
		Self {
			header,
			notarizations,
			new_account_origins,
			hash: H256::from_slice(&hash[..]),
			signature: Signature::from_raw([0u8; 64]),
		}
	}

	pub fn calculate_hash(&self) -> H256 {
		Self::create_hash(
			self.header.hash(),
			self.notarizations.clone().into(),
			self.new_account_origins.clone().into(),
		)
	}

	pub fn create_hash(
		header_hash: H256,
		notarizations: Vec<Notarization>,
		new_account_origins: Vec<NewAccountOrigin>,
	) -> H256 {
		let notarizations = BoundedVec::truncate_from(notarizations);
		let new_account_origins = BoundedVec::truncate_from(new_account_origins);
		let hash = NotebookHashMessage { header_hash, notarizations, new_account_origins }
			.using_encoded(blake2_256);
		H256::from_slice(&hash[..])
	}

	pub fn verify_hash(&self) -> bool {
		self.hash ==
			Self::create_hash(
				self.header.hash(),
				self.notarizations.clone().into(),
				self.new_account_origins.clone().into(),
			)
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
pub struct Notarization {
	pub balance_changes: NotarizationBalanceChangeset,
	pub block_votes: NotarizationBlockVotes,
	pub data_domains: NotarizationDataDomains,
}

impl Notarization {
	pub fn new(
		balance_changes: Vec<BalanceChange>,
		block_votes: Vec<BlockVote>,
		data_domains: Vec<(DataDomainHash, AccountId)>,
	) -> Self {
		Self {
			balance_changes: BoundedVec::truncate_from(balance_changes),
			block_votes: BoundedVec::truncate_from(block_votes),
			data_domains: BoundedVec::truncate_from(data_domains),
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
pub struct NewAccountOrigin {
	pub account_id: AccountId,
	pub account_type: AccountType,
	#[codec(compact)]
	pub account_uid: AccountOriginUid,
}
impl NewAccountOrigin {
	pub fn new(
		account_id: AccountId,
		account_type: AccountType,
		account_uid: AccountOriginUid,
	) -> Self {
		Self { account_id, account_type, account_uid }
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
pub struct NotebookHeader {
	#[codec(compact)]
	pub version: u16,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	/// A network "tick" that this header represents. This is a minute (on the minute) since the
	/// genesis block.
	#[codec(compact)]
	pub tick: u32,
	/// The amount of tax generated by this notebook
	#[codec(compact)]
	#[cfg_attr(feature = "std", serde(with = "serialize_unsafe_u128_as_string"))]
	pub tax: u128,
	#[codec(compact)]
	pub notary_id: NotaryId,
	/// Transfers between localchain and mainchain
	pub chain_transfers: BoundedVec<ChainTransfer, MaxNotebookTransfers>,
	/// A merkle root for all account balances changed in this notebook.
	/// Leafs are in order of account_id,account_type and must occur only once.
	/// Nodes contain the account id, account_type, nonce, balance and account origin.
	/// If a node is in the balance changes twice, only the last entry will be used.
	/// Nodes are encoded as Scale and hashed with Blake2 256
	pub changed_accounts_root: H256,
	/// All ids (AccountOrigins) that are changed in this notebook.
	pub changed_account_origins: BoundedVec<AccountOrigin, ConstU32<{ u32::MAX }>>,
	/// A merkle root for all tax and compute votes created in this notebook.
	/// Nodes are BlockVote records Scale encoded with Blake2 256 hashing.
	/// They are sorted by account id and index.
	pub block_votes_root: H256,
	/// The number of block votes created in this notebook.
	#[codec(compact)]
	pub block_votes_count: u32,
	/// The hashes of all blocks receiving votes
	pub blocks_with_votes: BoundedVec<H256, ConstU32<1000>>,
	/// The aggregate block voting power of the votes in this notebook
	#[codec(compact)]
	#[cfg_attr(feature = "std", serde(with = "serialize_unsafe_u128_as_string"))]
	pub block_voting_power: BlockVotingPower,
	/// A precommitment hash for the next notebook
	pub secret_hash: NotebookSecretHash,
	/// The revealed secret of the parent notebook. Only optional in first notebook.
	pub parent_secret: Option<NotebookSecret>,
	/// Registered data domains
	pub data_domains: BoundedVec<(DataDomainHash, AccountId), MaxDataDomainsPerNotebook>,
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
pub struct SignedNotebookHeader {
	/// The header details
	pub header: NotebookHeader,
	/// A signature of the hash by the notary public key
	pub signature: NotarySignature,
}

impl NotebookHeader {
	pub fn hash(&self) -> H256 {
		self.using_encoded(blake2_256).into()
	}

	pub fn hash_secret(&self, secret: NotebookSecret) -> NotebookSecretHash {
		Self::create_secret_hash(secret, self.block_votes_root, self.notebook_number)
	}

	pub fn create_secret_hash(
		secret: NotebookSecret,
		block_votes_root: H256,
		notebook_number: NotebookNumber,
	) -> NotebookSecretHash {
		NotebookSecretHash::from(blake2_256(
			&[&secret[..], &block_votes_root[..], &notebook_number.to_be_bytes()].concat(),
		))
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
pub struct NotebookMeta {
	pub finalized_notebook_number: NotebookNumber,
	pub finalized_tick: Tick,
}

#[derive(Encode, TypeInfo, MaxEncodedLen)]
pub struct BlockVotingKey {
	pub parent_vote_root: H256,
	pub parent_secret: NotebookSecret,
}
impl BlockVotingKey {
	pub fn hash(&self) -> H256 {
		self.using_encoded(blake2_256).into()
	}
	pub fn create_key(keys: Vec<BlockVotingKey>) -> H256 {
		blake2_256(&keys.encode()).into()
	}
}

pub const MAX_NOTARIES: u32 = 100;

pub type BlockVotingPower = u128;

pub type NotebookSecret = H256;
pub type NotebookSecretHash = H256;

pub type TransferToLocalchainId = u32;

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
#[serde(tag = "type")]
pub enum ChainTransfer {
	#[serde(rename_all = "camelCase")]
	ToMainchain {
		account_id: AccountId,
		#[codec(compact)]
		#[cfg_attr(feature = "std", serde(with = "serialize_unsafe_u128_as_string"))]
		amount: u128,
	},
	#[serde(rename_all = "camelCase")]
	ToLocalchain {
		#[codec(compact)]
		transfer_id: TransferToLocalchainId,
	},
}

#[cfg(feature = "std")]
pub mod serialize_unsafe_u128_as_string {
	use serde::{Deserializer, Serializer};
	// JavaScript's maximum safe integer (2^53 - 1)
	const JS_MAX_SAFE_INTEGER: u128 = 9_007_199_254_740_991;

	pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		if *value <= JS_MAX_SAFE_INTEGER {
			serializer.serialize_u128(*value)
		} else {
			serializer.serialize_str(&value.to_string())
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct U128Visitor;

		impl<'de> serde::de::Visitor<'de> for U128Visitor {
			type Value = u128;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("a string or number representing a u128")
			}

			fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				Ok(value as u128)
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				value.parse::<u128>().map_err(E::custom)
			}
		}

		deserializer.deserialize_any(U128Visitor)
	}
}
