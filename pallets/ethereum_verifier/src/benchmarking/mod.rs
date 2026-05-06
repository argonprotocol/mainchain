// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use super::*;
mod util;

use crate::{Fork, ForkVersions, Pallet as EthereumBeaconClient};
use argon_primitives::{
	EthereumExecutionBlockProof, EthereumLog, EthereumProof, EthereumReceiptProof,
	EthereumVerifyProvider,
};
use frame_benchmarking::v2::*;
use hex_literal::hex;
use polkadot_sdk::*;
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
		let checkpoint_update = make_checkpoint();
		let block_root: H256 = checkpoint_update.header.hash_tree_root().unwrap();

		#[extrinsic_call]
		_(RawOrigin::Root, Box::new(*checkpoint_update), BenchmarkForkVersions::get());

		assert!(<LatestFinalizedBlockRoot<T>>::get() == block_root);
		assert!(<FinalizedBeaconState<T>>::get(block_root).is_some());

		Ok(())
	}

	#[benchmark]
	fn submit() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let checkpoint_update = make_checkpoint();
		let finalized_header_update = make_finalized_header_update();
		let block_root: H256 = finalized_header_update.finalized_header.hash_tree_root().unwrap();
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&checkpoint_update,
			&BenchmarkForkVersions::get(),
		)?;

		#[extrinsic_call]
		submit(RawOrigin::Signed(caller.clone()), Box::new(*finalized_header_update));

		assert!(<LatestFinalizedBlockRoot<T>>::get() == block_root);
		assert!(<FinalizedBeaconState<T>>::get(block_root).is_some());

		Ok(())
	}

	#[benchmark]
	fn submit_with_sync_committee() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let checkpoint_update = make_checkpoint();
		let sync_committee_update = make_sync_committee_update();
		let finalized_header_update = make_finalized_header_update();
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&checkpoint_update,
			&BenchmarkForkVersions::get(),
		)?;
		EthereumBeaconClient::<T>::process_update(&finalized_header_update)?;

		#[extrinsic_call]
		submit(RawOrigin::Signed(caller.clone()), Box::new(*sync_committee_update));

		assert!(<NextSyncCommittee<T>>::exists());

		Ok(())
	}

	#[benchmark]
	fn import_execution_header_anchor() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let checkpoint_update = make_checkpoint();
		let finalized_header_update = make_finalized_header_update();
		let execution_proof: ExecutionProof = (*make_execution_proof()).into();
		let block_hash = execution_proof.execution_header.block_hash();
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&checkpoint_update,
			&BenchmarkForkVersions::get(),
		)?;
		EthereumBeaconClient::<T>::process_update(&finalized_header_update)?;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), execution_proof);

		assert!(<ExecutionHeaderAnchors<T>>::get(block_hash).is_some());

		Ok(())
	}

	#[benchmark]
	fn provider_verify_event_log() -> Result<(), BenchmarkError> {
		let inbound_fixture = make_inbound_fixture();
		let anchor_block_hash = H256::repeat_byte(9);
		let receipt_proof = inbound_fixture.event.proof.receipt_proof;
		let receipts_root =
			inbound_fixture.event.proof.execution_proof.execution_header.receipts_root();
		let event_log = EthereumLog {
			address: inbound_fixture.event.event_log.address,
			topics: inbound_fixture.event.event_log.topics,
			data: inbound_fixture.event.event_log.data,
		};
		let proof = EthereumProof {
			execution_block_proof: EthereumExecutionBlockProof {
				anchor_block_hash,
				target_to_anchor_header_chain: Vec::new(),
			},
			receipt_proof: EthereumReceiptProof { transaction_index: 0, nodes: receipt_proof },
		};

		ExecutionHeaderAnchors::<T>::insert(
			anchor_block_hash,
			ExecutionHeaderAnchor {
				block_number: 100,
				block_hash: anchor_block_hash,
				parent_hash: H256::repeat_byte(8),
				receipts_root,
			},
		);

		#[block]
		{
			<EthereumBeaconClient<T> as EthereumVerifyProvider>::verify_event_log(
				&event_log, &proof,
			)
			.map_err(|_| BenchmarkError::Stop("provider_verify_event_log failed"))?;
		}

		Ok(())
	}

	#[benchmark(extra)]
	fn bls_fast_aggregate_verify_pre_aggregated() -> Result<(), BenchmarkError> {
		EthereumBeaconClient::<T>::process_checkpoint_update(
			&make_checkpoint(),
			&BenchmarkForkVersions::get(),
		)?;
		let update = make_sync_committee_update();
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
			&make_checkpoint(),
			&BenchmarkForkVersions::get(),
		)?;
		let update = make_sync_committee_update();
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
			&make_checkpoint(),
			&BenchmarkForkVersions::get(),
		)?;
		let update = make_sync_committee_update();
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
