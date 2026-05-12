// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use crate::types::{CheckpointUpdate, ExecutionProof, NextSyncCommitteeUpdate, Update};

pub fn checkpoint_update_from_fixture<const COMMITTEE_SIZE: usize>(
	update: snowbridge_beacon_primitives::CheckpointUpdate<COMMITTEE_SIZE>,
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
	})
}

pub fn update_from_fixture<const COMMITTEE_SIZE: usize, const BITS_SIZE: usize>(
	update: snowbridge_beacon_primitives::Update<COMMITTEE_SIZE, BITS_SIZE>,
) -> Result<Update, &'static str> {
	Ok(Update {
		attested_header: update.attested_header,
		sync_aggregate: update
			.sync_aggregate
			.try_into()
			.map_err(|_| "sync aggregate exceeds verifier bounds")?,
		signature_slot: update.signature_slot,
		next_sync_committee_update: update
			.next_sync_committee_update
			.map(|next_sync_committee_update| -> Result<NextSyncCommitteeUpdate, &'static str> {
				Ok(NextSyncCommitteeUpdate {
					next_sync_committee: next_sync_committee_update
						.next_sync_committee
						.try_into()
						.map_err(|_| "next sync committee exceeds verifier bounds")?,
					next_sync_committee_branch: next_sync_committee_update
						.next_sync_committee_branch
						.try_into()
						.map_err(|_| "next sync committee branch exceeds configured maximum")?,
				})
			})
			.transpose()?,
		finalized_header: update.finalized_header,
		finality_branch: update
			.finality_branch
			.try_into()
			.map_err(|_| "finality branch exceeds configured maximum")?,
	})
}

pub fn execution_proof_from_fixture(
	proof: snowbridge_beacon_primitives::ExecutionProof,
) -> Result<ExecutionProof, &'static str> {
	proof.try_into().map_err(|_| "execution branch exceeds configured maximum")
}
