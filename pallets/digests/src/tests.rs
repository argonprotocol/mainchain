use crate::{mock::*, pallet::TempDigests};
use argon_notary_audit::VerifyError;
use argon_primitives::{
	AUTHOR_DIGEST_ID, BlockVoteDigest, Digestset, NotebookAuditResult, NotebookDigest,
	tick::TickDigest,
};
use codec::Encode;
use frame_support::pallet_prelude::Hooks;
use pallet_prelude::*;
use sp_runtime::DigestItem;
use std::panic::catch_unwind;

#[test]
fn it_throws_if_digests_missing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let err = catch_unwind(|| Digests::on_initialize(1));
		assert!(err.is_err());
	});
}

#[test]
fn it_disallows_duplicates() {
	new_test_ext().execute_with(|| {
		let mut pre_digest = Digestset {
			block_vote: BlockVoteDigest { voting_power: 500, votes_count: 1 },
			author: 1u64,
			voting_key: None,
			tick: TickDigest(2),
			fork_power: None,
			frame_info: None,
			notebooks: NotebookDigest {
				notebooks: BoundedVec::truncate_from(vec![NotebookAuditResult::<VerifyError> {
					notary_id: 1,
					notebook_number: 1,
					tick: 1,
					audit_first_failure: None,
				}]),
			},
		}
		.create_pre_runtime_digest();
		pre_digest.logs.push(author_digest(1));
		new_test_ext().execute_with(|| {
			System::set_block_number(1);
			let err = catch_unwind(|| Digests::on_initialize(1));
			assert!(err.is_err());
		});
	});
}

#[test]
fn it_should_read_and_clear_the_digests() {
	new_test_ext().execute_with(|| {
		let pre_digest = Digestset {
			block_vote: BlockVoteDigest { voting_power: 500, votes_count: 1 },
			author: 1u64,
			voting_key: None,
			fork_power: None,
			frame_info: None,
			tick: TickDigest(2),
			notebooks: NotebookDigest::<VerifyError> {
				notebooks: BoundedVec::truncate_from(vec![NotebookAuditResult {
					notary_id: 1,
					notebook_number: 1,
					tick: 1,
					audit_first_failure: Some(VerifyError::InvalidSecretProvided),
				}]),
			},
		}
		.create_pre_runtime_digest();

		System::initialize(&42, &System::parent_hash(), &pre_digest);
		Digests::on_initialize(42);

		let digests = TempDigests::<Test>::get();
		assert!(digests.is_some());
		let digests = digests.unwrap();
		assert_eq!(&digests.author, &1);
		assert_eq!(&digests.block_vote.votes_count, &1);
		assert_eq!(&digests.tick.0, &2);
		assert_eq!(digests.notebooks.notebooks.len(), 1);
		assert_eq!(digests.notebooks.notebooks[0].notary_id, 1);
		assert_eq!(digests.notebooks.notebooks[0].notebook_number, 1);
		assert_eq!(digests.notebooks.notebooks[0].tick, 1);

		Digests::on_finalize(42);
		assert!(TempDigests::<Test>::get().is_none());
	});
}

fn author_digest(author: u64) -> DigestItem {
	DigestItem::PreRuntime(AUTHOR_DIGEST_ID, author.encode())
}
