// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use super::*;
mod util;

use crate::{
	fixture_conversions::{
		checkpoint_update_from_fixture, execution_proof_from_fixture, update_from_fixture,
	},
	types::ExecutionHeaderAnchor,
	Fork, ForkVersions, Pallet as EthereumBeaconClient,
};
use alloy_consensus::{Header as AlloyHeader, Receipt, ReceiptEnvelope};
use alloy_primitives::{Address, Bytes, Log, B256};
use alloy_rlp::Encodable;
use alloy_trie::{proof::ProofRetainer, HashBuilder, Nibbles};
use argon_primitives::{
	ethereum::EthereumExecutionHeader, EthereumCombinedReceiptProof, EthereumExecutionBlockProof,
	EthereumLog, EthereumReceiptLog, EthereumReceiptLogProofBatch, EthereumReceiptLogProofBlock,
	EthereumReceiptProofReceipt, EthereumVerifyProvider,
};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use hex_literal::hex;
use polkadot_sdk::{
	frame_support::traits::ConstU32,
	sp_core::{H160, H256},
};
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
	fn provider_verify_receipt_logs(
		b: Linear<1, 10>,
		e: Linear<0, 15>,
	) -> Result<(), BenchmarkError> {
		let (proof_batch, argon_finalized_execution_header) = build_gateway_proof_batch(b, e)?;

		ExecutionHeaderAnchors::<T>::insert(
			argon_finalized_execution_header.block_hash,
			argon_finalized_execution_header,
		);

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

fn build_gateway_proof_batch(
	proof_blocks: u32,
	extra_activities: u32,
) -> Result<
	(EthereumReceiptLogProofBatch<ConstU32<10>, ConstU32<16>>, ExecutionHeaderAnchor),
	BenchmarkError,
> {
	let mut proof_blocks_with_roots = Vec::with_capacity(proof_blocks as usize);
	for block_offset in 0..proof_blocks {
		let receipt_logs = if block_offset == 0 { extra_activities.saturating_add(1) } else { 1 };
		let target_block_number = 100u64.saturating_add(u64::from(block_offset));
		proof_blocks_with_roots.push(build_gateway_proof_block(
			receipt_logs,
			target_block_number,
			block_offset as u8,
		)?);
	}

	let mut execution_headers = Vec::with_capacity(proof_blocks_with_roots.len());
	let mut parent_hash = H256::repeat_byte(1);
	let mut last_block_hash = parent_hash;
	for (_, receipts_root, target_block_number) in &proof_blocks_with_roots {
		let (header, block_hash) =
			make_execution_header(*target_block_number, parent_hash, *receipts_root);
		execution_headers.push(header);
		parent_hash = block_hash;
		last_block_hash = block_hash;
	}

	let argon_finalized_execution_header = ExecutionHeaderAnchor {
		block_number: 100u64.saturating_add(u64::from(proof_blocks)),
		block_hash: H256::repeat_byte(0x80),
		parent_hash: last_block_hash,
		receipts_root: H256::repeat_byte(0x81),
	};

	Ok((
		EthereumReceiptLogProofBatch {
			execution_block_proof: EthereumExecutionBlockProof {
				anchor_block_hash: argon_finalized_execution_header.block_hash,
				target_to_anchor_header_chain: execution_headers
					.try_into()
					.expect("benchmark header chain stays within bounded length"),
			},
			blocks: proof_blocks_with_roots
				.into_iter()
				.map(|(proof_block, _, _)| proof_block)
				.collect::<Vec<_>>()
				.try_into()
				.expect("benchmark proof blocks stay within bounded count"),
		},
		argon_finalized_execution_header,
	))
}

fn build_gateway_proof_block(
	receipt_logs: u32,
	target_block_number: u64,
	seed: u8,
) -> Result<(EthereumReceiptLogProofBlock<ConstU32<16>>, H256, u64), BenchmarkError> {
	let mut paths = Vec::with_capacity(receipt_logs as usize);
	let mut trie_receipts = Vec::with_capacity(receipt_logs as usize);
	let mut event_logs = Vec::with_capacity(receipt_logs as usize);

	for log_index in 0..receipt_logs {
		let address = Address::repeat_byte(seed.wrapping_add(log_index as u8).wrapping_add(1));
		let topic = B256::repeat_byte(seed.wrapping_add(log_index as u8).wrapping_add(0x40));
		let data = Bytes::from(vec![seed, log_index as u8]);
		let receipt = ReceiptEnvelope::Legacy(
			Receipt {
				status: true.into(),
				cumulative_gas_used: u64::from(log_index).saturating_add(1),
				logs: vec![Log::new_unchecked(address, vec![topic], data.clone())],
			}
			.with_bloom(),
		);
		let mut receipt_bytes = Vec::new();
		receipt.encode(&mut receipt_bytes);

		let path = Nibbles::unpack(alloy_rlp::encode(u64::from(log_index)));
		paths.push(path);
		trie_receipts.push((path, receipt_bytes));
		event_logs.push(EthereumReceiptLog {
			transaction_index: u64::from(log_index),
			event_log: EthereumLog {
				address: H160::from_slice(address.as_slice()),
				topics: vec![H256::from_slice(topic.as_slice())]
					.try_into()
					.expect("benchmark topics stay within bounded Ethereum log topics"),
				data: data
					.to_vec()
					.try_into()
					.expect("benchmark event data stays within bounded Ethereum log payload"),
			},
		});
	}

	let mut hash_builder =
		HashBuilder::default().with_proof_retainer(ProofRetainer::new(paths.clone()));
	trie_receipts.sort_unstable_by_key(|(path, _)| *path);
	for (path, receipt_bytes) in &trie_receipts {
		hash_builder.add_leaf(*path, receipt_bytes);
	}

	let receipts_root = H256::from_slice(hash_builder.root().as_slice());
	let proof_nodes = hash_builder.take_proof_nodes();
	let sorted_nodes = proof_nodes.nodes_sorted();
	let node_paths = sorted_nodes.iter().map(|(path, _)| *path).collect::<Vec<_>>();
	let receipt_proof = EthereumCombinedReceiptProof {
		nodes: sorted_nodes
			.iter()
			.map(|(_, node)| {
				node.to_vec()
					.try_into()
					.expect("benchmark receipt proof node stays within bounded size")
			})
			.collect::<Vec<_>>()
			.try_into()
			.expect("benchmark receipt proof stays within bounded node count"),
		receipts: paths
			.iter()
			.enumerate()
			.map(|(transaction_index, path)| EthereumReceiptProofReceipt {
				transaction_index: transaction_index as u64,
				node_indexes: proof_nodes
					.matching_nodes_sorted(path)
					.into_iter()
					.map(|(path, _)| {
						node_paths
							.iter()
							.position(|candidate| *candidate == path)
							.expect("benchmark receipt proof nodes should be retained") as u16
					})
					.collect::<Vec<_>>()
					.try_into()
					.expect("benchmark node refs stay within bounded receipt proof refs"),
			})
			.collect::<Vec<_>>()
			.try_into()
			.expect("benchmark receipts stay within bounded combined proof count"),
	};

	Ok((
		EthereumReceiptLogProofBlock {
			target_block_number,
			receipt_proof,
			receipt_logs: event_logs
				.try_into()
				.map_err(|_| BenchmarkError::Stop("benchmark receipt logs exceeded block bound"))?,
		},
		receipts_root,
		target_block_number,
	))
}

fn make_execution_header(
	block_number: u64,
	parent_hash: H256,
	receipts_root: H256,
) -> (EthereumExecutionHeader, H256) {
	let header = AlloyHeader {
		number: block_number,
		parent_hash: b256_from_h256(parent_hash),
		receipts_root: b256_from_h256(receipts_root),
		..Default::default()
	};
	let block_hash = H256::from_slice(header.hash_slow().as_slice());
	let mut rlp = Vec::new();
	header.encode(&mut rlp);
	(
		EthereumExecutionHeader {
			rlp: rlp.try_into().expect("benchmark headers stay within bounded RLP size"),
		},
		block_hash,
	)
}

fn b256_from_h256(value: H256) -> B256 {
	B256::from_slice(value.as_bytes())
}
