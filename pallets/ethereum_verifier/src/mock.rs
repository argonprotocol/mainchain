// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use crate as ethereum_beacon_client;
use crate::{
	config,
	fixture_conversions::{checkpoint_update_from_fixture, update_from_fixture},
	pallet_timestamp, sp_io, sp_runtime,
	types::{CheckpointUpdate, Update},
	Fork, ForkVersions,
};
use core::default::Default;
use frame_support::{derive_impl, dispatch::DispatchResult, parameter_types, traits::ConstU32};
use polkadot_sdk::*;
use sp_runtime::BuildStorage;
use std::{fs::File, path::PathBuf};

type Block = frame_system::mocking::MockBlock<Test>;
use hex_literal::hex;

pub const INBOUND_FIXTURE_RECEIPT_INDEX: u64 = 0;

fn load_fixture<T>(basename: String) -> Result<T, serde_json::Error>
where
	T: for<'de> serde::Deserialize<'de>,
{
	let filepath: PathBuf =
		[env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", &basename].iter().collect();
	serde_json::from_reader(File::open(filepath).unwrap())
}

pub fn load_execution_proof_fixture() -> snowbridge_beacon_primitives::ExecutionProof {
	load_fixture("execution-proof.json".to_string()).unwrap()
}

pub fn load_checkpoint_update_fixture() -> CheckpointUpdate {
	let update: snowbridge_beacon_primitives::CheckpointUpdate<{ config::SYNC_COMMITTEE_SIZE }> =
		load_fixture("initial-checkpoint.json".to_string()).unwrap();
	checkpoint_update_from_fixture(update)
		.expect("checkpoint fixture stays within bounded branch size")
}

pub fn load_sync_committee_update_fixture() -> Update {
	let update: snowbridge_beacon_primitives::Update<
		{ config::SYNC_COMMITTEE_SIZE },
		{ config::SYNC_COMMITTEE_BITS_SIZE },
	> = load_fixture("sync-committee-update.json".to_string()).unwrap();
	update_from_fixture(update).expect("sync committee fixture stays within bounded branch size")
}

pub fn load_finalized_header_update_fixture() -> Update {
	let update: snowbridge_beacon_primitives::Update<
		{ config::SYNC_COMMITTEE_SIZE },
		{ config::SYNC_COMMITTEE_BITS_SIZE },
	> = load_fixture("finalized-header-update.json".to_string()).unwrap();
	update_from_fixture(update).expect("finalized header fixture stays within bounded branch size")
}

pub fn load_next_sync_committee_update_fixture() -> Update {
	let update: snowbridge_beacon_primitives::Update<
		{ config::SYNC_COMMITTEE_SIZE },
		{ config::SYNC_COMMITTEE_BITS_SIZE },
	> = load_fixture("next-sync-committee-update.json".to_string()).unwrap();
	update_from_fixture(update)
		.expect("next sync committee fixture stays within bounded branch size")
}

pub fn load_next_finalized_header_update_fixture() -> Update {
	let update: snowbridge_beacon_primitives::Update<
		{ config::SYNC_COMMITTEE_SIZE },
		{ config::SYNC_COMMITTEE_BITS_SIZE },
	> = load_fixture("next-finalized-header-update.json".to_string()).unwrap();
	update_from_fixture(update)
		.expect("next finalized header fixture stays within bounded branch size")
}

pub fn load_sync_committee_update_period_0() -> Box<Update> {
	let update: snowbridge_beacon_primitives::Update<
		{ config::SYNC_COMMITTEE_SIZE },
		{ config::SYNC_COMMITTEE_BITS_SIZE },
	> = load_fixture("sync-committee-update-period-0.json".to_string()).unwrap();
	Box::new(
		update_from_fixture(update).expect("period-0 fixture stays within bounded branch size"),
	)
}

pub fn load_sync_committee_update_period_0_older_fixture() -> Box<Update> {
	let update: snowbridge_beacon_primitives::Update<
		{ config::SYNC_COMMITTEE_SIZE },
		{ config::SYNC_COMMITTEE_BITS_SIZE },
	> = load_fixture("sync-committee-update-period-0-older.json".to_string()).unwrap();
	Box::new(
		update_from_fixture(update).expect("older period fixture stays within bounded branch size"),
	)
}

pub fn load_sync_committee_update_period_0_newer_fixture() -> Box<Update> {
	let update: snowbridge_beacon_primitives::Update<
		{ config::SYNC_COMMITTEE_SIZE },
		{ config::SYNC_COMMITTEE_BITS_SIZE },
	> = load_fixture("sync-committee-update-period-0-newer.json".to_string()).unwrap();
	Box::new(
		update_from_fixture(update).expect("newer period fixture stays within bounded branch size"),
	)
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
			epoch: 100000000,
		}
	};
	pub static EventLogVerifierEnabled: bool = true;
}

pub const FREE_SLOTS_INTERVAL: u32 = config::SLOTS_PER_EPOCH as u32;

impl ethereum_beacon_client::Config for Test {
	type FreeHeadersInterval = ConstU32<FREE_SLOTS_INTERVAL>;
	type EventLogVerifierEnabled = EventLogVerifierEnabled;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_tester() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	sp_io::TestExternalities::new(t)
}

pub fn initialize_storage() -> DispatchResult {
	let inbound_fixture = snowbridge_pallet_ethereum_client_fixtures::make_inbound_fixture();
	ethereum_beacon_client::ForkVersionSchedule::<Test>::put(ChainForkVersions::get());
	EthereumBeaconClient::store_finalized_header(inbound_fixture.finalized_header)
}
