use super::*;
use crate::{types::VersionedExecutionPayloadHeader, CurrentSyncCommittee};
use polkadot_sdk::frame_support::traits::{OnRuntimeUpgrade, StorageVersion};

#[derive(serde::Deserialize)]
struct GloasExecutionFixture {
	header: BeaconHeader,
	execution_header_rlp: String,
	execution_branch: Vec<H256>,
}

#[derive(serde::Deserialize)]
struct HistoricalExecutionFixture {
	fork: String,
	header: BeaconHeader,
	execution_header_rlp: String,
	execution_branch: Vec<H256>,
	payload_header: serde_json::Value,
	payload_branch: Vec<H256>,
}

fn execution_witness() -> (BeaconHeader, ExecutionHeaderProof) {
	// Plataberget vector copied from Snowbridge bd8f292, gloas_end_to_end.
	let fixture: GloasExecutionFixture = load_fixture("gloas-execution-proof.json").unwrap();
	let rlp = alloy_primitives::hex::decode(fixture.execution_header_rlp).unwrap();
	(
		fixture.header,
		ExecutionHeaderProof {
			execution_header: VersionedExecutionPayloadHeader::Rlp(rlp.try_into().unwrap()),
			execution_branch: fixture.execution_branch.try_into().unwrap(),
		},
	)
}

#[test]
fn verifies_real_gloas_execution_hash_and_retained_roots() {
	new_tester().execute_with(|| {
		let (header, proof) = execution_witness();
		let mut forks = ChainForkVersions::get();
		forks.gloas.epoch = header.slot / MAINNET_SLOTS_PER_EPOCH;
		let anchor =
			EthereumBeaconClient::verify_execution_proof_for_header(&header, &proof, &forks)
				.unwrap();
		assert_eq!(anchor.block_number, 113_202);
		assert_eq!(anchor.timestamp_millis, 0x6a92edb4 * 1_000);
		assert_eq!(
			anchor.state_root,
			H256::from(hex!("e6b0dc37415b8d711a5b79dbd90549f08302c4e3f3adf15a3154c7979cf7816e"))
		);
		assert_eq!(
			anchor.receipts_root,
			H256::from(hex!("6e8de80294645f0e074c0e3d322adb756aa9c1896282dd4d9be22739b8006542"))
		);

		let VersionedExecutionPayloadHeader::Rlp(mut rlp) = proof.execution_header.clone() else {
			unreachable!()
		};
		rlp[50] ^= 1;
		let changed = ExecutionHeaderProof {
			execution_header: VersionedExecutionPayloadHeader::Rlp(rlp),
			..proof.clone()
		};
		assert_err!(
			EthereumBeaconClient::verify_execution_proof_for_header(&header, &changed, &forks),
			Error::<Test>::InvalidExecutionHeaderProof
		);
		let mut changed_branch = proof;
		changed_branch.execution_branch[0] = H256::zero();
		assert_err!(
			EthereumBeaconClient::verify_execution_proof_for_header(
				&header,
				&changed_branch,
				&forks
			),
			Error::<Test>::InvalidExecutionHeaderProof
		);
	});
}

#[test]
fn authenticated_slot_not_submitted_variant_selects_execution_scheme() {
	new_tester().execute_with(|| {
		let (mut header, proof) = execution_witness();
		let mut forks = ChainForkVersions::get();
		forks.gloas.epoch = header.slot / MAINNET_SLOTS_PER_EPOCH;
		header.slot = forks.gloas.epoch * MAINNET_SLOTS_PER_EPOCH - 1;
		assert_err!(
			EthereumBeaconClient::verify_execution_proof_for_header(&header, &proof, &forks),
			Error::<Test>::InvalidExecutionHeaderProof
		);

		let old = anchor_execution_proof();
		let old_proof = ExecutionHeaderProof::from(&old);
		assert_ok!(EthereumBeaconClient::verify_execution_proof_for_header(
			&old.header,
			&old_proof,
			&ChainForkVersions::get()
		));
		forks.gloas.epoch = 0;
		assert_err!(
			EthereumBeaconClient::verify_execution_proof_for_header(
				&old.header,
				&old_proof,
				&forks
			),
			Error::<Test>::InvalidExecutionHeaderProof
		);
	});
}

#[test]
fn historical_hash_witnesses_match_legacy_payload_proofs() {
	let fixtures: Vec<HistoricalExecutionFixture> =
		load_fixture("historical-execution-hash-proofs.json").unwrap();
	for fixture in fixtures {
		new_tester().execute_with(|| {
			let mut payload = fixture.payload_header;
			for field in [
				"block_number",
				"gas_limit",
				"gas_used",
				"timestamp",
				"base_fee_per_gas",
				"blob_gas_used",
				"excess_blob_gas",
			] {
				if let Some(value) = payload.get_mut(field) {
					*value = serde_json::json!(value.as_str().unwrap().parse::<u64>().unwrap());
				}
			}
			let execution_header = match fixture.fork.as_str() {
				"capella" => VersionedExecutionPayloadHeader::Capella(
					serde_json::from_value(payload).unwrap(),
				),
				"deneb" =>
					VersionedExecutionPayloadHeader::Deneb(serde_json::from_value(payload).unwrap()),
				_ => unreachable!("shared fixture fork"),
			};
			let legacy = ExecutionHeaderProof {
				execution_header,
				execution_branch: fixture.payload_branch.try_into().unwrap(),
			};
			let rlp = alloy_primitives::hex::decode(fixture.execution_header_rlp).unwrap();
			let proof = ExecutionHeaderProof {
				execution_header: VersionedExecutionPayloadHeader::Rlp(rlp.try_into().unwrap()),
				execution_branch: fixture.execution_branch.try_into().unwrap(),
			};
			let mut forks = ChainForkVersions::get();
			forks.capella.epoch = 1;
			forks.deneb.epoch = 2;
			forks.gloas.epoch = 3;
			for preset in [EthereumBeaconPreset::Mainnet, EthereumBeaconPreset::Minimal] {
				crate::BeaconPreset::<Test>::put(preset);
				let mut header = fixture.header;
				header.slot = fixture.header.slot / 8 * preset.slots_per_epoch() as u64;
				let legacy_anchor = EthereumBeaconClient::verify_execution_proof_for_header(
					&header, &legacy, &forks,
				)
				.unwrap();
				assert_eq!(
					EthereumBeaconClient::verify_execution_proof_for_header(
						&header, &proof, &forks
					)
					.unwrap(),
					legacy_anchor
				);
				let mut corrupted = proof.clone();
				corrupted.execution_branch[0] = H256::repeat_byte(99);
				assert_err!(
					EthereumBeaconClient::verify_execution_proof_for_header(
						&header, &corrupted, &forks
					),
					Error::<Test>::InvalidExecutionHeaderProof
				);
				let wrong_epoch = if fixture.fork == "capella" { 2 } else { 1 };
				header.slot = wrong_epoch * preset.slots_per_epoch() as u64;
				assert_err!(
					EthereumBeaconClient::verify_execution_proof_for_header(
						&header, &proof, &forks
					),
					Error::<Test>::InvalidExecutionHeaderProof
				);
				header.slot = 0;
				assert_err!(
					EthereumBeaconClient::verify_execution_proof_for_header(
						&header, &proof, &forks
					),
					Error::<Test>::InvalidExecutionHeaderProof
				);
			}
		});
	}
}

#[test]
fn gloas_indexes_switch_at_the_fork_for_both_presets() {
	new_tester().execute_with(|| {
		let mut forks = ChainForkVersions::get();
		forks.gloas.epoch = 70;
		for preset in [EthereumBeaconPreset::Mainnet, EthereumBeaconPreset::Minimal] {
			crate::BeaconPreset::<Test>::put(preset);
			let slot = forks.gloas.epoch * preset.slots_per_epoch() as u64;
			assert_eq!(
				EthereumBeaconClient::finalized_root_gindex_at_slot(slot - 1, &forks),
				crate::config::electra::FINALIZED_ROOT_INDEX
			);
			assert_eq!(EthereumBeaconClient::finalized_root_gindex_at_slot(slot, &forks), 735);
			assert_eq!(
				EthereumBeaconClient::current_sync_committee_gindex_at_slot(slot, &forks),
				2945
			);
			assert_eq!(
				EthereumBeaconClient::next_sync_committee_gindex_at_slot(slot, &forks),
				2946
			);
		}
		assert_eq!(EthereumBeaconClient::select_fork_version(&forks, 69), forks.electra.version);
		assert_eq!(EthereumBeaconClient::select_fork_version(&forks, 70), forks.gloas.version);
	});
}

#[test]
fn capella_and_deneb_scale_encodings_remain_compatible() {
	use snowbridge_beacon_primitives::VersionedExecutionPayloadHeader as OldHeader;
	for old in [OldHeader::Capella(Default::default()), OldHeader::Deneb(Default::default())] {
		let encoded = old.encode();
		let header = VersionedExecutionPayloadHeader::decode(&mut encoded.as_slice()).unwrap();
		assert_eq!(header, old.into());
		assert_eq!(header.encode(), encoded);
	}
}

#[test]
fn gloas_signatures_use_the_fork_at_the_previous_slot() {
	new_tester().execute_with(|| {
		let mut forks = ChainForkVersions::get();
		forks.fulu.epoch = 0;
		forks.gloas.epoch = 100;
		ForkVersionSchedule::<Test>::put(forks);
		let slot = forks.gloas.epoch * MAINNET_SLOTS_PER_EPOCH;
		let header = anchor_execution_proof().header;
		let validators = H256::repeat_byte(8);
		for (signature_slot, version) in
			[(slot, forks.fulu.version), (slot + 1, forks.gloas.version)]
		{
			let domain = EthereumBeaconClient::compute_domain(
				crate::config::DOMAIN_SYNC_COMMITTEE.to_vec(),
				version,
				validators,
			)
			.unwrap();
			let expected = EthereumBeaconClient::compute_signing_root(&header, domain).unwrap();
			assert_eq!(
				EthereumBeaconClient::signing_root(&header, validators, signature_slot).unwrap(),
				expected
			);
		}
	});
}

#[test]
fn gloas_rlp_decoding_is_bounded() {
	#[derive(Encode)]
	enum UnboundedHeader {
		#[codec(index = 2)]
		Rlp(Vec<u8>),
	}
	let bytes = UnboundedHeader::Rlp(vec![0; MAX_ETHEREUM_EXECUTION_HEADER_RLP_BYTES as usize + 1])
		.encode();
	assert!(VersionedExecutionPayloadHeader::decode(&mut bytes.as_slice()).is_err());
}

#[test]
fn malformed_execution_rlp_is_rejected() {
	let (_, proof) = execution_witness();
	let VersionedExecutionPayloadHeader::Rlp(rlp) = proof.execution_header else { unreachable!() };
	let mut trailing = rlp.to_vec();
	trailing.push(0);
	for bytes in
		[trailing, rlp[..rlp.len() - 1].to_vec(), vec![0xc0], vec![0x80], vec![0xf8, 0x01, 0x80]]
	{
		assert_eq!(
			ExecutionHeaderAnchor::from_rlp(&bytes),
			Err(EthereumVerifyError::InvalidHeader)
		);
	}
}

#[test]
fn receipt_header_chains_accept_the_same_gloas_rlp() {
	new_tester().execute_with(|| {
		let (_, proof) = execution_witness();
		let VersionedExecutionPayloadHeader::Rlp(rlp) = proof.execution_header else {
			unreachable!()
		};
		let target = ExecutionHeaderAnchor::from_rlp(&rlp).unwrap();
		let anchor = ExecutionHeaderAnchor {
			block_number: target.block_number + 1,
			block_hash: H256::repeat_byte(17),
			parent_hash: target.block_hash,
			..target
		};
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		let proof = EthereumExecutionBlockProof {
			anchor_block_hash: anchor.block_hash,
			target_to_anchor_header_chain: vec![EthereumExecutionHeader { rlp }]
				.try_into()
				.unwrap(),
		};
		assert_eq!(
			EthereumBeaconClient::verify_execution_block_proof(&proof),
			Ok(target.receipts_root)
		);
	});
}

#[test]
fn shared_execution_anchor_survives_until_its_last_beacon_reference_is_evicted() {
	new_tester().execute_with(|| {
		let (mut header, proof) = execution_witness();
		let anchor = ExecutionHeaderAnchor::from_payload_header(&proof.execution_header).unwrap();
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		ExecutionHeaderAnchorsByBlockNumber::<Test>::insert(
			anchor.block_number.to_be_bytes(),
			anchor,
		);
		LatestExecutionHeaderAnchorBlockHash::<Test>::put(anchor.block_hash);
		let index_before_first_reference = crate::FinalizedBeaconStateIndex::<Test>::get();
		assert_ok!(EthereumBeaconClient::store_finalized_header(header, anchor.block_hash, true));
		header.slot += 32;
		assert_ok!(EthereumBeaconClient::store_finalized_header(header, anchor.block_hash, true));
		let retained_root = header.hash_tree_root().unwrap();
		assert_eq!(crate::ExecutionHeaderAnchorReferences::<Test>::get(anchor.block_hash), 2);

		crate::FinalizedBeaconStateIndex::<Test>::put(index_before_first_reference);
		header.slot += 32;
		assert_ok!(EthereumBeaconClient::store_finalized_header(
			header,
			H256::repeat_byte(20),
			false
		));
		assert_eq!(ExecutionHeaderAnchors::<Test>::get(anchor.block_hash), Some(anchor));
		assert_eq!(
			ExecutionHeaderAnchorsByBlockNumber::<Test>::get(anchor.block_number.to_be_bytes()),
			Some(anchor)
		);
		assert_eq!(LatestExecutionHeaderAnchorBlockHash::<Test>::get(), Some(anchor.block_hash));
		assert!(FinalizedBeaconState::<Test>::contains_key(retained_root));
		assert_eq!(crate::ExecutionHeaderAnchorReferences::<Test>::get(anchor.block_hash), 1);
		let proof = EthereumExecutionBlockProof {
			anchor_block_hash: anchor.block_hash,
			target_to_anchor_header_chain: Default::default(),
		};
		assert_eq!(
			EthereumBeaconClient::verify_execution_block_proof(&proof),
			Ok(anchor.receipts_root)
		);

		header.slot += 32;
		assert_ok!(EthereumBeaconClient::store_finalized_header(
			header,
			H256::repeat_byte(21),
			false
		));
		assert!(!ExecutionHeaderAnchors::<Test>::contains_key(anchor.block_hash));
		assert!(!ExecutionHeaderAnchorsByBlockNumber::<Test>::contains_key(
			anchor.block_number.to_be_bytes()
		));
		assert!(!crate::ExecutionHeaderAnchorReferences::<Test>::contains_key(anchor.block_hash));
		assert_eq!(LatestExecutionHeaderAnchorBlockHash::<Test>::get(), None);
	});
}

#[test]
fn incoming_reference_keeps_an_evicted_anchor_and_duplicate_roots_do_not_leak_references() {
	new_tester().execute_with(|| {
		let (mut header, proof) = execution_witness();
		let anchor = ExecutionHeaderAnchor::from_payload_header(&proof.execution_header).unwrap();
		ExecutionHeaderAnchors::<Test>::insert(anchor.block_hash, anchor);
		assert_ok!(EthereumBeaconClient::store_finalized_header(header, anchor.block_hash, true));
		let index = crate::FinalizedBeaconStateIndex::<Test>::get();
		assert_ok!(EthereumBeaconClient::store_finalized_header(header, anchor.block_hash, true));
		assert_eq!(crate::FinalizedBeaconStateIndex::<Test>::get(), index);
		assert_eq!(crate::ExecutionHeaderAnchorReferences::<Test>::get(anchor.block_hash), 1);

		crate::FinalizedBeaconStateIndex::<Test>::put(0);
		header.slot += 32;
		assert_ok!(EthereumBeaconClient::store_finalized_header(header, anchor.block_hash, true));
		assert_eq!(ExecutionHeaderAnchors::<Test>::get(anchor.block_hash), Some(anchor));
		assert_eq!(crate::ExecutionHeaderAnchorReferences::<Test>::get(anchor.block_hash), 1);
	});
}

#[test]
fn migration_preserves_schedule_and_checkpoint_and_runs_once() {
	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&Box::new(load_checkpoint_update_fixture())));
		let before = LatestFinalizedBlockRoot::<Test>::get();
		let anchor_hash = LatestExecutionHeaderAnchorBlockHash::<Test>::get().unwrap();
		crate::FinalizedExecutionHeaderAnchor::<Test>::insert(H256::repeat_byte(25), anchor_hash);
		crate::ExecutionHeaderAnchorReferences::<Test>::remove(anchor_hash);
		let forks = ChainForkVersions::get();
		let old = [
			forks.genesis,
			forks.altair,
			forks.bellatrix,
			forks.capella,
			forks.deneb,
			forks.electra,
			forks.fulu,
		];
		polkadot_sdk::sp_io::storage::set(
			&ForkVersionSchedule::<Test>::hashed_key(),
			&old.encode(),
		);
		StorageVersion::new(1).put::<EthereumBeaconClient>();
		crate::migrations::AddGloasForkMigration::<Test>::on_runtime_upgrade();
		assert_eq!(ForkVersionSchedule::<Test>::get(), Some(forks));
		assert_eq!(LatestFinalizedBlockRoot::<Test>::get(), before);
		assert_eq!(crate::ExecutionHeaderAnchorReferences::<Test>::get(anchor_hash), 2);
		assert_eq!(StorageVersion::get::<EthereumBeaconClient>(), StorageVersion::new(2));

		let epoch =
			FinalizedBeaconState::<Test>::get(before).unwrap().slot / MAINNET_SLOTS_PER_EPOCH;
		let gloas = Fork { version: [7, 0, 0, 1], epoch: epoch.max(forks.fulu.epoch) + 100 };
		assert_ok!(EthereumBeaconClient::set_gloas_fork(RuntimeOrigin::root(), gloas));
		let configured = ForkVersions { gloas, ..forks };
		crate::migrations::AddGloasForkMigration::<Test>::on_runtime_upgrade();
		assert_eq!(ForkVersionSchedule::<Test>::get(), Some(configured));
		assert_eq!(LatestFinalizedBlockRoot::<Test>::get(), before);
	});
}

#[test]
fn scheduling_gloas_preserves_the_checkpoint_committees_and_anchors() {
	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&Box::new(load_checkpoint_update_fixture())));
		let forks = ForkVersionSchedule::<Test>::get().unwrap();
		let root = LatestFinalizedBlockRoot::<Test>::get();
		let committee = CurrentSyncCommittee::<Test>::get().encode();
		let anchor_hash = LatestExecutionHeaderAnchorBlockHash::<Test>::get().unwrap();
		let anchor = ExecutionHeaderAnchors::<Test>::get(anchor_hash);
		let epoch = FinalizedBeaconState::<Test>::get(root).unwrap().slot / MAINNET_SLOTS_PER_EPOCH;
		let fork = Fork { version: [7, 0, 0, 0], epoch: epoch.max(forks.fulu.epoch) + 100 };

		assert_ok!(EthereumBeaconClient::set_gloas_fork(RuntimeOrigin::root(), fork));
		assert_eq!(ForkVersionSchedule::<Test>::get(), Some(ForkVersions { gloas: fork, ..forks }));
		assert_eq!(LatestFinalizedBlockRoot::<Test>::get(), root);
		assert_eq!(CurrentSyncCommittee::<Test>::get().encode(), committee);
		assert_eq!(LatestExecutionHeaderAnchorBlockHash::<Test>::get(), Some(anchor_hash));
		assert_eq!(ExecutionHeaderAnchors::<Test>::get(anchor_hash), anchor);

		let postponed = Fork { epoch: fork.epoch + 10, ..fork };
		assert_ok!(EthereumBeaconClient::set_gloas_fork(RuntimeOrigin::root(), postponed));
		assert_eq!(ForkVersionSchedule::<Test>::get().unwrap().gloas, postponed);
	});
}

#[test]
fn scheduling_gloas_requires_root_and_a_valid_bootstrapped_schedule() {
	new_tester().execute_with(|| {
		let forks = ChainForkVersions::get();
		let fork = Fork { version: [7, 0, 0, 0], epoch: forks.fulu.epoch + 100 };
		assert_noop!(
			EthereumBeaconClient::set_gloas_fork(RuntimeOrigin::signed(1), fork),
			DispatchError::BadOrigin
		);
		assert_noop!(
			EthereumBeaconClient::set_gloas_fork(RuntimeOrigin::root(), fork),
			Error::<Test>::NotBootstrapped
		);
		assert_ok!(process_checkpoint_update(&Box::new(load_checkpoint_update_fixture())));
		for invalid in
			[Fork { epoch: forks.fulu.epoch, ..fork }, Fork { version: forks.fulu.version, ..fork }]
		{
			assert_noop!(
				EthereumBeaconClient::set_gloas_fork(RuntimeOrigin::root(), invalid),
				Error::<Test>::InvalidForkSchedule
			);
		}
	});
}

#[test]
fn scheduling_gloas_cannot_reinterpret_finalized_history() {
	new_tester().execute_with(|| {
		assert_ok!(process_checkpoint_update(&Box::new(load_checkpoint_update_fixture())));
		let forks = ForkVersionSchedule::<Test>::get().unwrap();
		let mut header = load_checkpoint_update_fixture().header;
		let epoch = header.slot / MAINNET_SLOTS_PER_EPOCH;
		let fork = Fork { version: [7, 0, 0, 0], epoch: epoch.max(forks.fulu.epoch) + 100 };
		header.slot = fork.epoch * MAINNET_SLOTS_PER_EPOCH;
		let anchor = LatestExecutionHeaderAnchorBlockHash::<Test>::get().unwrap();
		assert_ok!(EthereumBeaconClient::store_finalized_header(header, anchor, true));
		assert_noop!(
			EthereumBeaconClient::set_gloas_fork(RuntimeOrigin::root(), fork),
			Error::<Test>::GloasForkAlreadyActive
		);
		ForkVersionSchedule::<Test>::put(ForkVersions { gloas: fork, ..forks });
		assert_ok!(EthereumBeaconClient::set_gloas_fork(RuntimeOrigin::root(), fork));
		assert_noop!(
			EthereumBeaconClient::set_gloas_fork(
				RuntimeOrigin::root(),
				Fork { epoch: fork.epoch + 1, ..fork }
			),
			Error::<Test>::GloasForkAlreadyActive
		);
	});
}

#[test]
fn migration_keeps_unbootstrapped_verifier_unbootstrapped() {
	new_tester().execute_with(|| {
		ForkVersionSchedule::<Test>::kill();
		StorageVersion::new(1).put::<EthereumBeaconClient>();
		crate::migrations::AddGloasForkMigration::<Test>::on_runtime_upgrade();
		assert_eq!(ForkVersionSchedule::<Test>::get(), None);
	});
}
