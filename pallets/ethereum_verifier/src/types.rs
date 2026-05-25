// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use crate::{
	config::MAX_BRANCH_PROOF_SIZE_U32, ring_buffer::RingBufferMapImpl, FinalizedBeaconState,
	FinalizedBeaconStateIndex, FinalizedBeaconStateMapping, MaxFinalizedHeadersToKeep,
};
use argon_primitives::{EthereumBeaconPreset, EthereumBlockNumber};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use polkadot_sdk::{
	frame_support::{pallet_prelude::ConstU32, storage::types::OptionQuery, BoundedVec},
	sp_core::H256,
};
use scale_info::TypeInfo;
use snowbridge_beacon_primitives::{
	self as snowbridge, BeaconHeader, Fork as SnowbridgeFork, ForkVersion,
	ForkVersions as SnowbridgeForkVersions, PublicKey, PublicKeyPrepared, Signature,
	VersionedExecutionPayloadHeader,
};

const SYNC_COMMITTEE_BOUND_SIZE: u32 = EthereumBeaconPreset::Mainnet.sync_committee_size() as u32;
const SYNC_COMMITTEE_BITS_BOUND_SIZE: u32 =
	EthereumBeaconPreset::Mainnet.sync_committee_bits_size() as u32;
const MAINNET_SYNC_COMMITTEE_SIZE: usize = EthereumBeaconPreset::Mainnet.sync_committee_size();
#[cfg(any(test, feature = "fuzzing"))]
const MAINNET_SYNC_COMMITTEE_BITS_SIZE: usize =
	EthereumBeaconPreset::Mainnet.sync_committee_bits_size();
const MINIMAL_SYNC_COMMITTEE_SIZE: usize = EthereumBeaconPreset::Minimal.sync_committee_size();

type RawSyncCommittee<const COMMITTEE_SIZE: usize> = snowbridge::SyncCommittee<COMMITTEE_SIZE>;
type RawSyncCommitteePrepared<const COMMITTEE_SIZE: usize> =
	snowbridge::SyncCommitteePrepared<COMMITTEE_SIZE>;
type RawSyncAggregate<const COMMITTEE_SIZE: usize, const BITS_SIZE: usize> =
	snowbridge::SyncAggregate<COMMITTEE_SIZE, BITS_SIZE>;
type RawNextSyncCommitteeUpdate<const COMMITTEE_SIZE: usize> =
	snowbridge::NextSyncCommitteeUpdate<COMMITTEE_SIZE>;
type BranchProof = BoundedVec<H256, ConstU32<{ MAX_BRANCH_PROOF_SIZE_U32 }>>;

#[cfg(any(test, feature = "fuzzing"))]
pub(crate) type MainnetCheckpointUpdate =
	snowbridge::CheckpointUpdate<{ MAINNET_SYNC_COMMITTEE_SIZE }>;
#[cfg(any(test, feature = "fuzzing"))]
pub(crate) type MainnetUpdate =
	snowbridge::Update<{ MAINNET_SYNC_COMMITTEE_SIZE }, { MAINNET_SYNC_COMMITTEE_BITS_SIZE }>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeError {
	InvalidBoundedLength,
	HashTreeRootFailed,
	PreparePublicKeysFailed,
}

#[derive(
	Default, Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct SyncCommittee {
	pub pubkeys: BoundedVec<PublicKey, ConstU32<{ SYNC_COMMITTEE_BOUND_SIZE }>>,
	pub aggregate_pubkey: PublicKey,
}

impl SyncCommittee {
	pub fn matches_preset(&self, preset: EthereumBeaconPreset) -> bool {
		self.pubkeys.len() == preset.sync_committee_size()
	}

	pub fn hash_tree_root(&self, preset: EthereumBeaconPreset) -> Result<H256, TypeError> {
		match preset {
			EthereumBeaconPreset::Mainnet => self
				.to_raw::<{ MAINNET_SYNC_COMMITTEE_SIZE }>()?
				.hash_tree_root()
				.map_err(|_| TypeError::HashTreeRootFailed),
			EthereumBeaconPreset::Minimal => self
				.to_raw::<{ MINIMAL_SYNC_COMMITTEE_SIZE }>()?
				.hash_tree_root()
				.map_err(|_| TypeError::HashTreeRootFailed),
		}
	}

	pub fn prepare(
		&self,
		preset: EthereumBeaconPreset,
	) -> Result<SyncCommitteePrepared, TypeError> {
		match preset {
			EthereumBeaconPreset::Mainnet => {
				let committee = self.to_raw::<{ MAINNET_SYNC_COMMITTEE_SIZE }>()?;
				RawSyncCommitteePrepared::<{ MAINNET_SYNC_COMMITTEE_SIZE }>::try_from(&committee)
					.map_err(|_| TypeError::PreparePublicKeysFailed)?
					.try_into()
			},
			EthereumBeaconPreset::Minimal => {
				let committee = self.to_raw::<{ MINIMAL_SYNC_COMMITTEE_SIZE }>()?;
				RawSyncCommitteePrepared::<{ MINIMAL_SYNC_COMMITTEE_SIZE }>::try_from(&committee)
					.map_err(|_| TypeError::PreparePublicKeysFailed)?
					.try_into()
			},
		}
	}

	fn to_raw<const COMMITTEE_SIZE: usize>(
		&self,
	) -> Result<RawSyncCommittee<COMMITTEE_SIZE>, TypeError> {
		Ok(RawSyncCommittee {
			pubkeys: self
				.pubkeys
				.to_vec()
				.try_into()
				.map_err(|_| TypeError::InvalidBoundedLength)?,
			aggregate_pubkey: self.aggregate_pubkey,
		})
	}
}

impl<const COMMITTEE_SIZE: usize> TryFrom<RawSyncCommittee<COMMITTEE_SIZE>> for SyncCommittee {
	type Error = TypeError;

	fn try_from(sync_committee: RawSyncCommittee<COMMITTEE_SIZE>) -> Result<Self, Self::Error> {
		Ok(Self {
			pubkeys: sync_committee
				.pubkeys
				.to_vec()
				.try_into()
				.map_err(|_| TypeError::InvalidBoundedLength)?,
			aggregate_pubkey: sync_committee.aggregate_pubkey,
		})
	}
}

#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct SyncCommitteePrepared {
	pub root: H256,
	pub pubkeys: BoundedVec<PublicKeyPrepared, ConstU32<{ SYNC_COMMITTEE_BOUND_SIZE }>>,
	pub aggregate_pubkey: PublicKeyPrepared,
}

impl DecodeWithMemTracking for SyncCommitteePrepared {}

impl<const COMMITTEE_SIZE: usize> TryFrom<RawSyncCommitteePrepared<COMMITTEE_SIZE>>
	for SyncCommitteePrepared
{
	type Error = TypeError;

	fn try_from(
		sync_committee: RawSyncCommitteePrepared<COMMITTEE_SIZE>,
	) -> Result<Self, Self::Error> {
		Ok(Self {
			root: sync_committee.root,
			pubkeys: sync_committee
				.pubkeys
				.as_ref()
				.to_vec()
				.try_into()
				.map_err(|_| TypeError::InvalidBoundedLength)?,
			aggregate_pubkey: sync_committee.aggregate_pubkey,
		})
	}
}

#[derive(Default, Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Debug, TypeInfo)]
pub struct SyncAggregate {
	pub sync_committee_bits: BoundedVec<u8, ConstU32<{ SYNC_COMMITTEE_BITS_BOUND_SIZE }>>,
	pub sync_committee_signature: Signature,
}

impl SyncAggregate {
	pub fn matches_preset(&self, preset: EthereumBeaconPreset) -> bool {
		self.sync_committee_bits.len() == preset.sync_committee_bits_size()
	}
}

impl<const COMMITTEE_SIZE: usize, const BITS_SIZE: usize>
	TryFrom<RawSyncAggregate<COMMITTEE_SIZE, BITS_SIZE>> for SyncAggregate
{
	type Error = TypeError;

	fn try_from(
		sync_aggregate: RawSyncAggregate<COMMITTEE_SIZE, BITS_SIZE>,
	) -> Result<Self, Self::Error> {
		Ok(Self {
			sync_committee_bits: sync_aggregate
				.sync_committee_bits
				.to_vec()
				.try_into()
				.map_err(|_| TypeError::InvalidBoundedLength)?,
			sync_committee_signature: sync_aggregate.sync_committee_signature,
		})
	}
}

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
pub struct ExecutionHeaderProof {
	pub execution_header: VersionedExecutionPayloadHeader,
	pub execution_branch: BranchProof,
}

/// Argon-owned bootstrap checkpoint shape without ancestry or `block_roots` data.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Debug, TypeInfo)]
pub struct CheckpointUpdate {
	pub header: BeaconHeader,
	pub current_sync_committee: SyncCommittee,
	pub current_sync_committee_branch: BranchProof,
	pub validators_root: H256,
	pub execution_header_proof: ExecutionHeaderProof,
}

/// Argon-owned next sync committee witness carried inside an update.
#[derive(Default, Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Debug, TypeInfo)]
pub struct NextSyncCommitteeUpdate {
	pub next_sync_committee: SyncCommittee,
	pub next_sync_committee_branch: BranchProof,
}

impl<const COMMITTEE_SIZE: usize> TryFrom<RawNextSyncCommitteeUpdate<COMMITTEE_SIZE>>
	for NextSyncCommitteeUpdate
{
	type Error = TypeError;

	fn try_from(update: RawNextSyncCommitteeUpdate<COMMITTEE_SIZE>) -> Result<Self, Self::Error> {
		Ok(Self {
			next_sync_committee: update.next_sync_committee.try_into()?,
			next_sync_committee_branch: update
				.next_sync_committee_branch
				.try_into()
				.map_err(|_| TypeError::InvalidBoundedLength)?,
		})
	}
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
	pub execution_header_proof: ExecutionHeaderProof,
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
pub type ExecutionBlockNumber = EthereumBlockNumber;
/// Big-endian execution block-number key used for ordered scans over retained anchors.
///
/// Clients start the scan at the target execution block number and take the first retained anchor
/// at or after that block instead of walking every retained finalized beacon root.
pub type ExecutionHeaderAnchorScanKey = [u8; 8];
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
	#[codec(compact)]
	pub timestamp_millis: u64,
	pub block_hash: ExecutionBlockHash,
	pub parent_hash: ExecutionBlockHash,
	pub receipts_root: ReceiptsRoot,
}

impl ExecutionHeaderAnchor {
	pub fn from_payload_header(header: &VersionedExecutionPayloadHeader) -> Self {
		match header {
			VersionedExecutionPayloadHeader::Capella(header) => Self {
				block_number: header.block_number,
				timestamp_millis: header.timestamp.saturating_mul(1_000),
				block_hash: header.block_hash,
				parent_hash: header.parent_hash,
				receipts_root: header.receipts_root,
			},
			VersionedExecutionPayloadHeader::Deneb(header) => Self {
				block_number: header.block_number,
				timestamp_millis: header.timestamp.saturating_mul(1_000),
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

impl TryFrom<snowbridge::ExecutionProof> for ExecutionProof {
	type Error = TypeError;

	fn try_from(proof: snowbridge::ExecutionProof) -> Result<Self, Self::Error> {
		Ok(Self {
			header: proof.header,
			execution_header: proof.execution_header,
			execution_branch: proof
				.execution_branch
				.try_into()
				.map_err(|_| TypeError::InvalidBoundedLength)?,
		})
	}
}

impl From<ExecutionProof> for ExecutionHeaderProof {
	fn from(proof: ExecutionProof) -> Self {
		Self { execution_header: proof.execution_header, execution_branch: proof.execution_branch }
	}
}

impl From<&ExecutionProof> for ExecutionHeaderProof {
	fn from(proof: &ExecutionProof) -> Self {
		Self {
			execution_header: proof.execution_header.clone(),
			execution_branch: proof.execution_branch.clone(),
		}
	}
}

// Storage adapters.

/// Finalized state ring buffer implementation.
pub type FinalizedBeaconStateBuffer<T> = RingBufferMapImpl<
	u32,
	MaxFinalizedHeadersToKeep<T>,
	FinalizedBeaconStateIndex<T>,
	FinalizedBeaconStateMapping<T>,
	FinalizedBeaconState<T>,
	OptionQuery,
>;
