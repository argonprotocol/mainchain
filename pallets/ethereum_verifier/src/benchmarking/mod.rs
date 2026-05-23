// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use super::*;
mod util;

use crate::{
	fixture_conversions::{
		checkpoint_update_from_fixture, execution_proof_from_fixture, update_from_fixture,
	},
	Fork, ForkVersions, Pallet as EthereumBeaconClient,
};
use alloy_consensus::Header as AlloyHeader;
use alloy_primitives::B256;
use alloy_rlp::Encodable;
use argon_primitives::{
	ethereum::{EthereumExecutionHeader, MAX_ETHEREUM_HEADER_CHAIN_LEN},
	EthereumBlockNumber, EthereumCombinedReceiptProof, EthereumExecutionBlockProof, EthereumLog,
	EthereumReceiptLog, EthereumReceiptLogProofBatch, EthereumReceiptLogProofBlock,
	EthereumReceiptProofReceipt, EthereumVerifyProvider,
};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use hex_literal::hex;
use polkadot_sdk::sp_core::H256;
use snowbridge_beacon_primitives::{
	fast_aggregate_verify,
	merkle_proof::{generalized_index_length, subtree_index},
	prepare_aggregate_pubkey, prepare_aggregate_signature, verify_merkle_branch,
};
use snowbridge_pallet_ethereum_client_fixtures::*;
use util::*;

frame_support::parameter_types! {
	pub const BenchmarkForkVersions: ForkVersions = ForkVersions {
		genesis: Fork { version: hex!("00000000"), epoch: 0 },
		altair: Fork { version: hex!("01000000"), epoch: 0 },
		bellatrix: Fork { version: hex!("02000000"), epoch: 0 },
		capella: Fork { version: hex!("03000000"), epoch: 0 },
		deneb: Fork { version: hex!("04000000"), epoch: 0 },
		electra: Fork { version: hex!("05000000"), epoch: 0 },
		fulu: Fork { version: hex!("06000000"), epoch: 100_000_000 },
	};
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn force_checkpoint() -> Result<(), BenchmarkError> {
		let checkpoint_update =
			checkpoint_update_from_fixture(*make_checkpoint()).map_err(BenchmarkError::Stop)?;
		let block_root: H256 = checkpoint_update.header.hash_tree_root().unwrap();

		#[extrinsic_call]
		_(RawOrigin::Root, Box::new(checkpoint_update), BenchmarkForkVersions::get());

		assert!(<LatestFinalizedBlockRoot<T>>::get() == block_root);
		assert!(<FinalizedBeaconState<T>>::get(block_root).is_some());

		Ok(())
	}

	#[benchmark]
	fn submit() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let checkpoint_update =
			checkpoint_update_from_fixture(*make_checkpoint()).map_err(BenchmarkError::Stop)?;
		let finalized_header_update =
			update_from_fixture(*make_finalized_header_update()).map_err(BenchmarkError::Stop)?;
		let block_root: H256 = finalized_header_update.finalized_header.hash_tree_root().unwrap();
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&checkpoint_update,
			&BenchmarkForkVersions::get(),
		)?;

		#[extrinsic_call]
		submit(RawOrigin::Signed(caller.clone()), Box::new(finalized_header_update));

		assert!(<LatestFinalizedBlockRoot<T>>::get() == block_root);
		assert!(<FinalizedBeaconState<T>>::get(block_root).is_some());

		Ok(())
	}

	#[benchmark]
	fn submit_with_sync_committee() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let checkpoint_update =
			checkpoint_update_from_fixture(*make_checkpoint()).map_err(BenchmarkError::Stop)?;
		let sync_committee_update =
			update_from_fixture(*make_sync_committee_update()).map_err(BenchmarkError::Stop)?;
		let finalized_header_update =
			update_from_fixture(*make_finalized_header_update()).map_err(BenchmarkError::Stop)?;
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&checkpoint_update,
			&BenchmarkForkVersions::get(),
		)?;
		EthereumBeaconClient::<T>::process_update(&finalized_header_update)?;

		#[extrinsic_call]
		submit(RawOrigin::Signed(caller.clone()), Box::new(sync_committee_update));

		assert!(<NextSyncCommittee<T>>::exists());

		Ok(())
	}

	#[benchmark]
	fn import_execution_header_anchor() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let checkpoint_update =
			checkpoint_update_from_fixture(*make_checkpoint()).map_err(BenchmarkError::Stop)?;
		let execution_proof =
			execution_proof_from_fixture(*make_execution_proof()).map_err(BenchmarkError::Stop)?;
		let block_hash = execution_proof.execution_header.block_hash();
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&checkpoint_update,
			&BenchmarkForkVersions::get(),
		)?;
		EthereumBeaconClient::<T>::store_finalized_header(execution_proof.header)?;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), execution_proof);

		assert!(<ExecutionHeaderAnchors<T>>::get(block_hash).is_some());

		Ok(())
	}

	#[benchmark]
	fn provider_verify_receipt_logs() -> Result<(), BenchmarkError> {
		let inbound_fixture = make_inbound_fixture();
		let anchor_block_hash = H256::repeat_byte(9);
		let receipt_proof = inbound_fixture.event.proof.receipt_proof;
		let receipts_root =
			inbound_fixture.event.proof.execution_proof.execution_header.receipts_root();
		let (target_to_anchor_header_chain, anchor) =
			max_header_chain(anchor_block_hash, 100, receipts_root);
		let receipt_log = EthereumReceiptLog {
			transaction_index: 0,
			event_log: EthereumLog {
				address: inbound_fixture.event.event_log.address,
				topics: inbound_fixture
					.event
					.event_log
					.topics
					.try_into()
					.expect("fixture topics stay within bounded Ethereum log topics"),
				data: inbound_fixture
					.event
					.event_log
					.data
					.try_into()
					.expect("fixture event data stays within bounded Ethereum log payload"),
			},
		};
		let proof_batch = EthereumReceiptLogProofBatch {
			execution_block_proof: EthereumExecutionBlockProof {
				anchor_block_hash,
				target_to_anchor_header_chain,
			},
			blocks: vec![EthereumReceiptLogProofBlock {
				target_block_number: 100,
				receipt_proof: EthereumCombinedReceiptProof {
					nodes: receipt_proof
						.into_iter()
						.map(|node| {
							node.try_into()
								.expect("fixture receipt proof node stays within bounded size")
						})
						.collect::<Vec<_>>()
						.try_into()
						.expect("fixture receipt proof stays within bounded node count"),
					receipts: vec![EthereumReceiptProofReceipt {
						transaction_index: 0,
						node_indexes: (0..receipt_proof.len())
							.map(|index| index as u16)
							.collect::<Vec<_>>()
							.try_into()
							.expect("fixture node indexes stay within bounded receipt proof refs"),
					}]
					.try_into()
					.expect("single receipt reference stays within bounded receipt proof count"),
				},
				receipt_logs: vec![receipt_log]
					.try_into()
					.expect("single receipt log stays within bounded log count"),
			}]
			.try_into()
			.expect("single proof block stays within bounded block count"),
		};

		ExecutionHeaderAnchors::<T>::insert(anchor_block_hash, anchor);

		#[block]
		{
			<EthereumBeaconClient<T> as EthereumVerifyProvider>::verify_receipt_logs(&proof_batch)
				.map_err(|_| BenchmarkError::Stop("provider_verify_receipt_logs failed"))?;
		}

		Ok(())
	}

	#[benchmark(extra)]
	fn bls_fast_aggregate_verify_pre_aggregated() -> Result<(), BenchmarkError> {
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&checkpoint_update_from_fixture(*make_checkpoint()).map_err(BenchmarkError::Stop)?,
			&BenchmarkForkVersions::get(),
		)?;
		let update =
			update_from_fixture(*make_sync_committee_update()).map_err(BenchmarkError::Stop)?;
		let participant_pubkeys = participant_pubkeys::<T>(&update)?;
		let signing_root = signing_root::<T>(&update)?;
		let agg_sig =
			prepare_aggregate_signature(&update.sync_aggregate.sync_committee_signature).unwrap();
		let agg_pub_key = prepare_aggregate_pubkey(&participant_pubkeys).unwrap();

		#[block]
		{
			agg_sig.fast_aggregate_verify_pre_aggregated(signing_root.as_bytes(), &agg_pub_key);
		}

		Ok(())
	}

	#[benchmark(extra)]
	fn bls_fast_aggregate_verify() -> Result<(), BenchmarkError> {
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&checkpoint_update_from_fixture(*make_checkpoint()).map_err(BenchmarkError::Stop)?,
			&BenchmarkForkVersions::get(),
		)?;
		let update =
			update_from_fixture(*make_sync_committee_update()).map_err(BenchmarkError::Stop)?;
		let current_sync_committee = <CurrentSyncCommittee<T>>::get();
		let absent_pubkeys = absent_pubkeys::<T>(&update)?;
		let signing_root = signing_root::<T>(&update)?;

		#[block]
		{
			fast_aggregate_verify(
				&current_sync_committee.aggregate_pubkey,
				&absent_pubkeys,
				signing_root,
				&update.sync_aggregate.sync_committee_signature,
			)
			.unwrap();
		}

		Ok(())
	}

	#[benchmark(extra)]
	fn verify_merkle_proof() -> Result<(), BenchmarkError> {
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&checkpoint_update_from_fixture(*make_checkpoint()).map_err(BenchmarkError::Stop)?,
			&BenchmarkForkVersions::get(),
		)?;
		let update =
			update_from_fixture(*make_sync_committee_update()).map_err(BenchmarkError::Stop)?;
		let block_root: H256 = update.finalized_header.hash_tree_root().unwrap();

		let fork_versions = ForkVersions {
			genesis: Fork { version: hex!("00000000"), epoch: 0 },
			altair: Fork { version: hex!("01000000"), epoch: 0 },
			bellatrix: Fork { version: hex!("02000000"), epoch: 0 },
			capella: Fork { version: hex!("03000000"), epoch: 0 },
			deneb: Fork { version: hex!("04000000"), epoch: 0 },
			electra: Fork { version: hex!("05000000"), epoch: 80000000000 },
			fulu: Fork { version: hex!("06000000"), epoch: 80000000001 },
		};
		let finalized_root_gindex = EthereumBeaconClient::<T>::finalized_root_gindex_at_slot(
			update.attested_header.slot,
			&fork_versions,
		);
		#[block]
		{
			verify_merkle_branch(
				block_root,
				&update.finality_branch,
				subtree_index(finalized_root_gindex),
				generalized_index_length(finalized_root_gindex),
				update.attested_header.state_root,
			);
		}

		Ok(())
	}

	impl_benchmark_test_suite!(EthereumBeaconClient, crate::mock::new_tester(), crate::mock::Test);
}

fn max_header_chain(
	anchor_block_hash: H256,
	target_block_number: EthereumBlockNumber,
	receipts_root: H256,
) -> (argon_primitives::ethereum::EthereumExecutionHeaderChain, ExecutionHeaderAnchor) {
	let mut headers = Vec::with_capacity(MAX_ETHEREUM_HEADER_CHAIN_LEN as usize);
	let mut previous_block_hash = H256::repeat_byte(1);
	let mut last_block_hash = H256::repeat_byte(1);

	for offset in 0..MAX_ETHEREUM_HEADER_CHAIN_LEN {
		let header = AlloyHeader {
			number: target_block_number + offset as u64,
			parent_hash: b256_from_h256(previous_block_hash),
			receipts_root: b256_from_h256(if offset == 0 {
				receipts_root
			} else {
				H256::repeat_byte(offset as u8)
			}),
			..Default::default()
		};
		let block_hash = H256::from_slice(header.hash_slow().as_slice());
		let mut rlp = Vec::new();
		header.encode(&mut rlp);
		headers.push(EthereumExecutionHeader {
			rlp: rlp.try_into().expect("benchmark headers stay within bounded RLP size"),
		});
		previous_block_hash = block_hash;
		last_block_hash = block_hash;
	}

	(
		headers
			.try_into()
			.expect("benchmark chain stays within bounded header chain length"),
		ExecutionHeaderAnchor {
			block_number: target_block_number + MAX_ETHEREUM_HEADER_CHAIN_LEN as u64,
			timestamp_millis: 0,
			block_hash: anchor_block_hash,
			parent_hash: last_block_hash,
			receipts_root: H256::repeat_byte(8),
		},
	)
}

fn b256_from_h256(value: H256) -> B256 {
	B256::from_slice(value.as_bytes())
}
