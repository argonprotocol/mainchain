// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
pub use crate::config::{SYNC_COMMITTEE_BITS_SIZE as SC_BITS_SIZE, SYNC_COMMITTEE_SIZE as SC_SIZE};
use crate::{config::MAX_BRANCH_PROOF_SIZE_U32, ring_buffer::RingBufferMapImpl};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::storage::types::OptionQuery;
use polkadot_sdk::{
	sp_core::{ConstU32, H256},
	sp_runtime::BoundedVec,
	*,
};
use scale_info::TypeInfo;
use snowbridge_beacon_primitives::{
	BeaconHeader, Fork as SnowbridgeFork, ForkVersion, ForkVersions as SnowbridgeForkVersions,
	VersionedExecutionPayloadHeader,
};

// Specialize types based on configured sync committee size.
pub type SyncCommittee = snowbridge_beacon_primitives::SyncCommittee<SC_SIZE>;
pub type SyncCommitteePrepared = snowbridge_beacon_primitives::SyncCommitteePrepared<SC_SIZE>;
pub type SyncAggregate = snowbridge_beacon_primitives::SyncAggregate<SC_SIZE, SC_BITS_SIZE>;
type BranchProof = BoundedVec<H256, ConstU32<{ MAX_BRANCH_PROOF_SIZE_U32 }>>;

// Argon verifier models.

/// Governance-controlled verifier operating mode.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
pub enum BasicOperatingMode {
	/// Normal mode, when verifier updates are allowed.
	#[default]
	Normal,
	/// The verifier is halted. All non-governance updates are disabled.
	Halted,
}

impl BasicOperatingMode {
	pub fn is_halted(&self) -> bool {
		*self == BasicOperatingMode::Halted
	}
}

/// Ethereum fork version active from an epoch.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct Fork {
	pub version: ForkVersion,
	#[codec(compact)]
	pub epoch: u64,
}

impl From<Fork> for SnowbridgeFork {
	fn from(fork: Fork) -> Self {
		Self { version: fork.version, epoch: fork.epoch }
	}
}

/// Ethereum fork schedule used by the verifier.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct ForkVersions {
	pub genesis: Fork,
	pub altair: Fork,
	pub bellatrix: Fork,
	pub capella: Fork,
	pub deneb: Fork,
	pub electra: Fork,
	pub fulu: Fork,
}

impl From<ForkVersions> for SnowbridgeForkVersions {
	fn from(forks: ForkVersions) -> Self {
		Self {
			genesis: forks.genesis.into(),
			altair: forks.altair.into(),
			bellatrix: forks.bellatrix.into(),
			capella: forks.capella.into(),
			deneb: forks.deneb.into(),
			electra: forks.electra.into(),
			fulu: forks.fulu.into(),
		}
	}
}

/// Argon-owned bootstrap checkpoint shape without ancestry or `block_roots` data.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Debug, TypeInfo)]
pub struct CheckpointUpdate {
	pub header: BeaconHeader,
	pub current_sync_committee: SyncCommittee,
	pub current_sync_committee_branch: BranchProof,
	pub validators_root: H256,
}

/// Argon-owned next sync committee witness carried inside an update.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Debug, TypeInfo)]
pub struct NextSyncCommitteeUpdate {
	pub next_sync_committee: SyncCommittee,
	pub next_sync_committee_branch: BranchProof,
}

/// Argon-owned finalized update shape without ancestry or `block_roots` data.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Debug, TypeInfo)]
pub struct Update {
	pub attested_header: BeaconHeader,
	pub sync_aggregate: SyncAggregate,
	#[codec(compact)]
	pub signature_slot: u64,
	pub next_sync_committee_update: Option<NextSyncCommitteeUpdate>,
	pub finalized_header: BeaconHeader,
	pub finality_branch: BranchProof,
}

/// Minimal finalized beacon state retained by Argon for direct-finalized verification.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct FinalizedBeaconHeaderState {
	#[codec(compact)]
	pub slot: u64,
}

pub type ExecutionBlockHash = H256;
pub type ExecutionBlockNumber = u64;
pub type ReceiptsRoot = H256;

/// Execution-layer header fields retained after the beacon proof has been accepted.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct ExecutionHeaderAnchor {
	#[codec(compact)]
	pub block_number: ExecutionBlockNumber,
	pub block_hash: ExecutionBlockHash,
	pub parent_hash: ExecutionBlockHash,
	pub receipts_root: ReceiptsRoot,
}

impl ExecutionHeaderAnchor {
	pub fn from_payload_header(header: &VersionedExecutionPayloadHeader) -> Self {
		match header {
			VersionedExecutionPayloadHeader::Capella(header) => Self {
				block_number: header.block_number,
				block_hash: header.block_hash,
				parent_hash: header.parent_hash,
				receipts_root: header.receipts_root,
			},
			VersionedExecutionPayloadHeader::Deneb(header) => Self {
				block_number: header.block_number,
				block_hash: header.block_hash,
				parent_hash: header.parent_hash,
				receipts_root: header.receipts_root,
			},
		}
	}
}

/// Proof that an execution payload header is contained in a finalized beacon block.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Debug, TypeInfo)]
pub struct ExecutionProof {
	/// Header for the beacon block containing the execution payload.
	pub header: BeaconHeader,
	/// The execution payload header being anchored.
	pub execution_header: VersionedExecutionPayloadHeader,
	/// Merkle proof that the execution payload header is contained within `header`.
	pub execution_branch: BranchProof,
}

// Storage adapters.

/// Finalized state ring buffer implementation.
pub type FinalizedBeaconStateBuffer<T> = RingBufferMapImpl<
	u32,
	crate::MaxFinalizedHeadersToKeep<T>,
	crate::FinalizedBeaconStateIndex<T>,
	crate::FinalizedBeaconStateMapping<T>,
	crate::FinalizedBeaconState<T>,
	OptionQuery,
>;

/// Execution header anchor ring buffer implementation.
pub type ExecutionHeaderAnchorBuffer<T> = RingBufferMapImpl<
	u32,
	crate::MaxFinalizedHeadersToKeep<T>,
	crate::ExecutionHeaderAnchorIndex<T>,
	crate::ExecutionHeaderAnchorMapping<T>,
	crate::ExecutionHeaderAnchors<T>,
	OptionQuery,
>;
