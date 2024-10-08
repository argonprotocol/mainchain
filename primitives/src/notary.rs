use alloc::{
	collections::btree_map::BTreeMap,
	string::{String, ToString},
	vec::Vec,
};
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use core::fmt::Debug;
use frame_support::pallet_prelude::ConstU32;
use frame_support_procedural::{CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ed25519, Get, RuntimeDebug, H256};
use sp_runtime::{traits::Block as BlockT, BoundedVec};

use crate::{
	host::Host, tick::Tick, AccountOrigin, BlockVotingPower, NotebookHeader, NotebookNumber,
	NotebookSecret, NotebookSecretHash, TransferToLocalchainId,
};

pub type NotaryId = u32;
pub type NotaryPublic = ed25519::Public;
pub type NotarySignature = ed25519::Signature;

pub trait NotaryProvider<B: BlockT> {
	fn verify_signature(
		notary_id: NotaryId,
		at_tick: Tick,
		message: &H256,
		signature: &NotarySignature,
	) -> bool;
	fn active_notaries() -> Vec<NotaryId>;
}

#[derive(
	CloneNoBound, PartialEqNoBound, Eq, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxHosts))]
pub struct NotaryMeta<MaxHosts: Get<u32>> {
	pub name: NotaryName,
	pub public: NotaryPublic,
	pub hosts: BoundedVec<Host, MaxHosts>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[repr(transparent)]
pub struct NotaryName(pub BoundedVec<u8, ConstU32<55>>);

impl From<String> for NotaryName {
	fn from(name: String) -> Self {
		name.into_bytes().to_vec().into()
	}
}

impl From<&str> for NotaryName {
	fn from(name: &str) -> Self {
		name.as_bytes().to_vec().into()
	}
}
impl From<Vec<u8>> for NotaryName {
	fn from(name: Vec<u8>) -> Self {
		Self(BoundedVec::truncate_from(name))
	}
}

impl TryInto<String> for NotaryName {
	type Error = String;

	fn try_into(self) -> Result<String, Self::Error> {
		String::from_utf8(self.0.into_inner()).map_err(|_| "Invalid UTF-8".to_string())
	}
}

impl Serialize for NotaryName {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let string_value: String = self.clone().try_into().map_err(serde::ser::Error::custom)?;
		serializer.serialize_str(&string_value)
	}
}

impl<'de> Deserialize<'de> for NotaryName {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		Ok(NotaryName::from(s))
	}
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenesisNotary<AccountId> {
	pub account_id: AccountId,
	pub public: NotaryPublic,
	pub hosts: Vec<Host>,
	pub name: NotaryName,
}

#[derive(
	CloneNoBound, PartialEqNoBound, Eq, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxHosts))]
pub struct NotaryRecord<
	AccountId: Codec + MaxEncodedLen + Clone + PartialEq + Eq + Debug,
	BlockNumber: Codec + MaxEncodedLen + Clone + PartialEq + Eq + Debug,
	MaxHosts: Get<u32>,
> {
	#[codec(compact)]
	pub notary_id: NotaryId,
	pub operator_account_id: AccountId,
	#[codec(compact)]
	pub activated_block: BlockNumber,

	#[codec(compact)]
	pub meta_updated_block: BlockNumber,
	#[codec(compact)]
	pub meta_updated_tick: Tick,
	pub meta: NotaryMeta<MaxHosts>,
}
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
pub struct NotaryNotebookTickState {
	#[codec(compact)]
	pub notaries: u16,
	pub notebook_key_details_by_notary: BTreeMap<NotaryId, NotaryNotebookVoteDigestDetails>,
	pub raw_headers_by_notary: BTreeMap<NotaryId, Vec<u8>>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NotaryNotebookDetails<Hash: Codec> {
	#[codec(compact)]
	pub notary_id: NotaryId,
	#[codec(compact)]
	pub version: u32,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	#[codec(compact)]
	pub tick: Tick,
	pub header_hash: H256,
	#[codec(compact)]
	pub block_votes_count: u32,
	#[codec(compact)]
	pub block_voting_power: BlockVotingPower,
	pub blocks_with_votes: Vec<Hash>,
	pub raw_audit_summary: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NotaryNotebookRawVotes {
	#[codec(compact)]
	pub notary_id: NotaryId,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	pub raw_votes: Vec<(Vec<u8>, BlockVotingPower)>,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NotaryNotebookAuditSummary {
	#[codec(compact)]
	pub notary_id: NotaryId,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	#[codec(compact)]
	pub tick: Tick,
	#[codec(compact)]
	pub version: u32,
	// Encoded NotaryNotebookAuditSummaryDetails
	pub raw_data: Vec<u8>,
}

impl TryInto<NotaryNotebookAuditSummaryDecoded> for NotaryNotebookAuditSummary {
	type Error = String;

	fn try_into(self) -> Result<NotaryNotebookAuditSummaryDecoded, Self::Error> {
		let details = NotaryNotebookAuditSummaryDetails::decode(&mut &self.raw_data[..])
			.map_err(|_| "Invalid NotaryNotebookAuditSummaryDetails".to_string())?;
		Ok(NotaryNotebookAuditSummaryDecoded {
			notary_id: self.notary_id,
			notebook_number: self.notebook_number,
			tick: self.tick,
			version: self.version,
			details,
		})
	}
}

#[derive(Clone, PartialEq, Eq)]
pub struct NotaryNotebookAuditSummaryDecoded {
	pub notary_id: NotaryId,
	pub notebook_number: NotebookNumber,
	pub tick: Tick,
	pub version: u32,
	pub details: NotaryNotebookAuditSummaryDetails,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NotaryNotebookAuditSummaryDetails {
	pub changed_accounts_root: H256,
	pub account_changelist: Vec<AccountOrigin>,
	pub used_transfers_to_localchain: Vec<TransferToLocalchainId>,
	pub secret_hash: NotebookSecretHash,
	pub block_votes_root: H256,
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NotaryNotebookVoteDigestDetails {
	#[codec(compact)]
	pub notary_id: NotaryId,
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	#[codec(compact)]
	pub tick: Tick,
	#[codec(compact)]
	pub block_votes_count: u32,
	#[codec(compact)]
	pub block_voting_power: BlockVotingPower,
}

impl From<&NotebookHeader> for NotaryNotebookVoteDigestDetails {
	fn from(header: &NotebookHeader) -> Self {
		Self {
			notary_id: header.notary_id,
			notebook_number: header.notebook_number,
			tick: header.tick,
			block_votes_count: header.block_votes_count,
			block_voting_power: header.block_voting_power,
		}
	}
}
impl<H: Codec> From<&NotaryNotebookDetails<H>> for NotaryNotebookVoteDigestDetails {
	fn from(header: &NotaryNotebookDetails<H>) -> Self {
		Self {
			notary_id: header.notary_id,
			notebook_number: header.notebook_number,
			tick: header.tick,
			block_votes_count: header.block_votes_count,
			block_voting_power: header.block_voting_power,
		}
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct NotaryNotebookKeyDetails {
	#[codec(compact)]
	pub notebook_number: NotebookNumber,
	#[codec(compact)]
	pub tick: Tick,
	pub block_votes_root: H256,
	pub secret_hash: NotebookSecretHash,
	pub parent_secret: Option<NotebookSecret>,
}
