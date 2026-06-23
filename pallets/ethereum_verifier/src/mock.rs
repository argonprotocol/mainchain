// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use crate as ethereum_beacon_client;
use crate::{
	fixture_conversions::{
		anchored_update_from_fixture, checkpoint_update_from_fixture, committee_update_from_fixture,
	},
	pallet_timestamp, sp_io, sp_runtime,
	types::{CheckpointUpdate, MainnetCheckpointUpdate, MainnetUpdate, Update},
	Fork, ForkVersions,
};
use argon_primitives::EthereumBeaconPreset;
use core::default::Default;
use frame_support::{derive_impl, parameter_types, traits::ConstU32};
use polkadot_sdk::*;
use snowbridge::types::{BeaconHeader, VersionedExecutionPayloadHeader};
use snowbridge_beacon_primitives as snowbridge;
use sp_core::H256;
use sp_runtime::BuildStorage;
use std::{fs::File, path::PathBuf};

type Block = frame_system::mocking::MockBlock<Test>;
use hex_literal::hex;

pub const INBOUND_FIXTURE_RECEIPT_INDEX: u64 = 0;

pub(crate) fn load_fixture<T>(basename: impl AsRef<str>) -> Result<T, serde_json::Error>
where
	T: for<'de> serde::Deserialize<'de>,
{
	let filepath: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", basename.as_ref()]
		.iter()
		.collect();
	serde_json::from_reader(File::open(filepath).unwrap())
}

pub fn load_execution_proof_fixture() -> snowbridge::ExecutionProof {
	load_fixture("execution-proof.json".to_string()).unwrap()
}

#[derive(serde::Deserialize)]
struct ExecutionWitnessFixture {
	header: BeaconHeader,
	execution_header: VersionedExecutionPayloadHeader,
	execution_branch: Vec<H256>,
}

#[derive(serde::Deserialize)]
struct UpdateExecutionWitnessFixture {
	finalized_header: BeaconHeader,
	execution_header: VersionedExecutionPayloadHeader,
	execution_branch: Vec<H256>,
}

fn load_execution_witness_fixture(basename: &str) -> snowbridge::ExecutionProof {
	let proof: ExecutionWitnessFixture = load_fixture(basename.to_string()).unwrap();
	snowbridge::ExecutionProof {
		header: proof.header,
		ancestry_proof: None,
		execution_header: proof.execution_header,
		execution_branch: proof.execution_branch,
	}
}

fn load_update_execution_witness_fixture(basename: &str) -> snowbridge::ExecutionProof {
	let proof: UpdateExecutionWitnessFixture = load_fixture(basename.to_string()).unwrap();
	snowbridge::ExecutionProof {
		header: proof.finalized_header,
		ancestry_proof: None,
		execution_header: proof.execution_header,
		execution_branch: proof.execution_branch,
	}
}

pub fn load_checkpoint_update_fixture() -> CheckpointUpdate {
	let update: MainnetCheckpointUpdate =
		load_fixture("initial-checkpoint.json".to_string()).unwrap();
	let execution_proof = load_execution_witness_fixture("initial-checkpoint-execution-proof.json");
	checkpoint_update_from_fixture(update, execution_proof)
		.expect("checkpoint fixture stays within bounded branch size")
}

pub fn load_later_checkpoint_update_fixture() -> CheckpointUpdate {
	let update: MainnetCheckpointUpdate =
		load_fixture("initial-checkpoint-later.json".to_string()).unwrap();
	let execution_proof =
		load_execution_witness_fixture("initial-checkpoint-later-execution-proof.json");
	checkpoint_update_from_fixture(update, execution_proof)
		.expect("later checkpoint fixture stays within bounded branch size")
}

pub fn load_sync_committee_update_fixture() -> Update {
	let update: MainnetUpdate = load_fixture("sync-committee-update.json".to_string()).unwrap();
	committee_update_from_fixture(
		update,
		load_update_execution_witness_fixture("sync-committee-update.json"),
	)
	.expect("sync committee fixture stays within bounded branch size")
}

pub fn load_finalized_header_update_fixture() -> Update {
	let update: MainnetUpdate = load_fixture("finalized-header-update.json".to_string()).unwrap();
	anchored_update_from_fixture(
		update,
		load_update_execution_witness_fixture("finalized-header-update.json"),
	)
	.expect("finalized header fixture stays within bounded branch size")
}

pub fn load_next_sync_committee_update_fixture() -> Update {
	let update: MainnetUpdate =
		load_fixture("next-sync-committee-update.json".to_string()).unwrap();
	committee_update_from_fixture(
		update,
		load_update_execution_witness_fixture("next-sync-committee-update.json"),
	)
	.expect("next sync committee fixture stays within bounded branch size")
}

pub fn load_next_finalized_header_update_fixture() -> Update {
	let update: MainnetUpdate =
		load_fixture("next-finalized-header-update.json".to_string()).unwrap();
	anchored_update_from_fixture(
		update,
		load_update_execution_witness_fixture("next-finalized-header-update.json"),
	)
	.expect("next finalized header fixture stays within bounded branch size")
}

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system::{Pallet, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		EthereumBeaconClient: ethereum_beacon_client::{Pallet, Call, Storage, Event<T>},
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const ChainForkVersions: ForkVersions = ForkVersions {
		genesis: Fork {
			version: hex!("00000000"),
			epoch: 0,
		},
		altair: Fork {
			version: hex!("01000000"),
			epoch: 0,
		},
		bellatrix: Fork {
			version: hex!("02000000"),
			epoch: 0,
		},
		capella: Fork {
			version: hex!("03000000"),
			epoch: 0,
		},
		deneb: Fork {
			version: hex!("04000000"),
			epoch: 0,
		},
		electra: Fork {
			version: hex!("05000000"),
			epoch: 0,
		},
		fulu: Fork {
			version: hex!("06000000"),
			epoch: 411_392,
		}
	};
}

pub const FREE_SLOTS_INTERVAL: u32 = EthereumBeaconPreset::Mainnet.slots_per_epoch() as u32;

impl ethereum_beacon_client::Config for Test {
	type FreeHeadersInterval = ConstU32<FREE_SLOTS_INTERVAL>;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_tester() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	sp_io::TestExternalities::new(t)
}
