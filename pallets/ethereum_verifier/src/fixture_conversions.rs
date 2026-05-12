use crate::types::{
	CheckpointUpdate, ExecutionProof, NextSyncCommitteeUpdate, Update, SC_BITS_SIZE, SC_SIZE,
};

pub(crate) fn checkpoint_update_from_fixture(
	update: snowbridge_beacon_primitives::CheckpointUpdate<SC_SIZE>,
) -> Result<CheckpointUpdate, &'static str> {
	Ok(CheckpointUpdate {
		header: update.header,
		current_sync_committee: update.current_sync_committee,
		current_sync_committee_branch: update
			.current_sync_committee_branch
			.try_into()
			.map_err(|_| "sync committee branch exceeds configured maximum")?,
		validators_root: update.validators_root,
	})
}

pub(crate) fn update_from_fixture(
	update: snowbridge_beacon_primitives::Update<SC_SIZE, SC_BITS_SIZE>,
) -> Result<Update, &'static str> {
	Ok(Update {
		attested_header: update.attested_header,
		sync_aggregate: update.sync_aggregate,
		signature_slot: update.signature_slot,
		next_sync_committee_update: update
			.next_sync_committee_update
			.map(next_sync_committee_update_from_fixture)
			.transpose()?,
		finalized_header: update.finalized_header,
		finality_branch: update
			.finality_branch
			.try_into()
			.map_err(|_| "finality branch exceeds configured maximum")?,
	})
}

pub(crate) fn execution_proof_from_fixture(
	proof: snowbridge_beacon_primitives::ExecutionProof,
) -> Result<ExecutionProof, &'static str> {
	Ok(ExecutionProof {
		header: proof.header,
		execution_header: proof.execution_header,
		execution_branch: proof
			.execution_branch
			.try_into()
			.map_err(|_| "execution branch exceeds configured maximum")?,
	})
}

fn next_sync_committee_update_from_fixture(
	update: snowbridge_beacon_primitives::NextSyncCommitteeUpdate<SC_SIZE>,
) -> Result<NextSyncCommitteeUpdate, &'static str> {
	Ok(NextSyncCommitteeUpdate {
		next_sync_committee: update.next_sync_committee,
		next_sync_committee_branch: update
			.next_sync_committee_branch
			.try_into()
			.map_err(|_| "next sync committee branch exceeds configured maximum")?,
	})
}
