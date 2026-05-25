// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use crate::types::{
	CheckpointUpdate, ExecutionHeaderProof, ExecutionProof, NextSyncCommitteeUpdate, Update,
};

pub fn checkpoint_update_from_fixture<const COMMITTEE_SIZE: usize>(
	update: snowbridge_beacon_primitives::CheckpointUpdate<COMMITTEE_SIZE>,
	execution_proof: snowbridge_beacon_primitives::ExecutionProof,
) -> Result<CheckpointUpdate, &'static str> {
	Ok(CheckpointUpdate {
		header: update.header,
		current_sync_committee: update
			.current_sync_committee
			.try_into()
			.map_err(|_| "sync committee exceeds verifier bounds")?,
		current_sync_committee_branch: update
			.current_sync_committee_branch
			.try_into()
			.map_err(|_| "sync committee branch exceeds configured maximum")?,
		validators_root: update.validators_root,
		execution_header_proof: execution_header_proof_from_fixture(execution_proof)?,
	})
}

pub fn committee_update_from_fixture<const COMMITTEE_SIZE: usize, const BITS_SIZE: usize>(
	update: snowbridge_beacon_primitives::Update<COMMITTEE_SIZE, BITS_SIZE>,
	execution_proof: snowbridge_beacon_primitives::ExecutionProof,
) -> Result<Update, &'static str> {
	let Some(next_sync_committee_update) = update.next_sync_committee_update else {
		return Err("fixture does not include a next sync committee update");
	};

	Ok(Update {
		attested_header: update.attested_header,
		sync_aggregate: update
			.sync_aggregate
			.try_into()
			.map_err(|_| "sync aggregate exceeds verifier bounds")?,
		signature_slot: update.signature_slot,
		next_sync_committee_update: Some(next_sync_committee_update_from_fixture(
			next_sync_committee_update,
		)?),
		finalized_header: update.finalized_header,
		finality_branch: update
			.finality_branch
			.try_into()
			.map_err(|_| "finality branch exceeds configured maximum")?,
		execution_header_proof: execution_header_proof_from_fixture(execution_proof)?,
	})
}

pub fn anchored_update_from_fixture<const COMMITTEE_SIZE: usize, const BITS_SIZE: usize>(
	update: snowbridge_beacon_primitives::Update<COMMITTEE_SIZE, BITS_SIZE>,
	execution_proof: snowbridge_beacon_primitives::ExecutionProof,
) -> Result<Update, &'static str> {
	let execution_header_proof = execution_header_proof_from_fixture(execution_proof)?;
	let attested_header = update.attested_header;
	let sync_aggregate = update
		.sync_aggregate
		.try_into()
		.map_err(|_| "sync aggregate exceeds verifier bounds")?;
	let signature_slot = update.signature_slot;
	let finalized_header = update.finalized_header;
	let finality_branch = update
		.finality_branch
		.try_into()
		.map_err(|_| "finality branch exceeds configured maximum")?;

	Ok(Update {
		attested_header,
		sync_aggregate,
		signature_slot,
		next_sync_committee_update: update
			.next_sync_committee_update
			.map(next_sync_committee_update_from_fixture)
			.transpose()?,
		finalized_header,
		finality_branch,
		execution_header_proof,
	})
}

pub fn execution_proof_from_fixture(
	proof: snowbridge_beacon_primitives::ExecutionProof,
) -> Result<ExecutionProof, &'static str> {
	proof.try_into().map_err(|_| "execution branch exceeds configured maximum")
}

pub fn execution_header_proof_from_fixture(
	proof: snowbridge_beacon_primitives::ExecutionProof,
) -> Result<ExecutionHeaderProof, &'static str> {
	execution_proof_from_fixture(proof).map(ExecutionHeaderProof::from)
}

fn next_sync_committee_update_from_fixture<const COMMITTEE_SIZE: usize>(
	update: snowbridge_beacon_primitives::NextSyncCommitteeUpdate<COMMITTEE_SIZE>,
) -> Result<NextSyncCommitteeUpdate, &'static str> {
	Ok(NextSyncCommitteeUpdate {
		next_sync_committee: update
			.next_sync_committee
			.try_into()
			.map_err(|_| "next sync committee exceeds verifier bounds")?,
		next_sync_committee_branch: update
			.next_sync_committee_branch
			.try_into()
			.map_err(|_| "next sync committee branch exceeds configured maximum")?,
	})
}
