use crate::{
	Error, Event,
	mock::*,
	pallet::{
		AccountOriginLastChangedNotebookByNotary, LastNotebookDetailsByNotary,
		NotariesLockedForFailedAudit, NotebookChangedAccountsRootByNotary,
	},
};
use argon_notary_audit::{AccountHistoryLookupError, VerifyError};
use argon_primitives::{
	BalanceProof, BlockVote, MerkleProof, NotebookAuditResult, NotebookDigest, NotebookProvider,
	SignedNotebookHeader,
	localchain::{AccountType, BalanceChange, Note, NoteType},
	notary::{
		NotaryNotebookAuditSummary, NotaryNotebookAuditSummaryDetails, NotaryNotebookKeyDetails,
		NotaryState,
	},
	notebook::{
		AccountOrigin, BalanceTip, ChainTransfer, NewAccountOrigin, Notarization, NotebookHeader,
		NotebookNumber,
	},
};
use binary_merkle_tree::{merkle_proof, merkle_root};
use pallet_prelude::*;
use sp_core::{Pair, bounded_vec, ed25519};
use sp_keyring::{
	Ed25519Keyring,
	Sr25519Keyring::{Alice, Bob},
};
use std::collections::BTreeMap;

fn notebook_digest(
	books: Vec<(NotaryId, NotebookNumber, Tick, bool)>,
) -> NotebookDigest<VerifyError> {
	NotebookDigest {
		notebooks: books
			.into_iter()
			.map(|(notary_id, notebook_number, tick, has_error)| NotebookAuditResult {
				notebook_number,
				notary_id,
				tick,
				audit_first_failure: if has_error {
					Some(VerifyError::InvalidBlockVoteRoot)
				} else {
					None
				},
			})
			.collect(),
	}
}

#[test]
fn it_ensures_only_a_single_inherent_is_submitted() {
	new_test_ext().execute_with(|| {
		let digest = notebook_digest(vec![]);
		Digests::mutate(|d| d.notebooks = digest.clone());
		Notebook::on_initialize(1);
		assert_ok!(Notebook::submit(RuntimeOrigin::none(), vec![]));
		assert_err!(
			Notebook::submit(RuntimeOrigin::none(), vec![]),
			Error::<Test>::MultipleNotebookInherentsProvided
		);
	});
}

#[test]
fn it_locks_notaries_on_audit_failure() {
	new_test_ext().execute_with(|| {
		let digest = notebook_digest(vec![(1, 1, 1, false), (2, 1, 1, true)]);
		Digests::mutate(|d| d.notebooks = digest.clone());
		Notebook::on_initialize(1);
		CurrentTick::set(2);
		assert_err!(
			Notebook::submit(RuntimeOrigin::none(), vec![]),
			Error::<Test>::InvalidNotebookDigest
		);
		let header1 = make_header(1, 1);
		let mut header2 = make_header(1, 1);
		header2.notary_id = 2;

		assert_ok!(Notebook::submit(
			RuntimeOrigin::none(),
			vec![
				SignedNotebookHeader {
					header: header1.clone(),
					signature: Ed25519Keyring::Bob.pair().sign(header1.hash().as_ref())
				},
				SignedNotebookHeader {
					header: header2.clone(),
					signature: Ed25519Keyring::Bob.pair().sign(header2.hash().as_ref())
				}
			]
		));
		let callbacks = NotebookEvents::get();
		// should only callback for the valid notebook
		assert_eq!(callbacks.len(), 1);
		assert_eq!(callbacks[0], header1);
		// should store that it's no longer valid
		assert_eq!(LastNotebookDetailsByNotary::<Test>::get(2).len(), 0);
		// this is the default verify error
		assert_eq!(Notebook::notary_failed_audit_by_id(1), None);
		assert_eq!(
			Notebook::notary_failed_audit_by_id(2),
			Some((1, 1, VerifyError::InvalidBlockVoteRoot))
		);
		assert!(Notebook::is_notary_locked_at_tick(2, 1));
		assert!(Notebook::is_notary_locked_at_tick(2, 2));
		assert!(!Notebook::is_notary_locked_at_tick(2, 0));
		assert_eq!(
			Notebook::get_state(2),
			NotaryState::Locked {
				failed_audit_reason: VerifyError::InvalidBlockVoteRoot,
				at_tick: 1,
				notebook_number: 1
			}
		);
		assert_eq!(Notebook::get_state(1), NotaryState::Active);
	});
}

#[test]
fn it_cannot_submit_additional_notebooks_once_locked() {
	new_test_ext().execute_with(|| {
		NotariesLockedForFailedAudit::<Test>::insert(1, (1, 1, VerifyError::InvalidBlockVoteRoot));

		let digest = notebook_digest(vec![(1, 1, 1, false)]);
		Digests::mutate(|d| d.notebooks = digest.clone());
		Notebook::on_initialize(2);
		CurrentTick::set(2);

		let header1 = make_header(1, 1);

		assert_err!(
			Notebook::submit(
				RuntimeOrigin::none(),
				vec![SignedNotebookHeader {
					header: header1.clone(),
					signature: Ed25519Keyring::Bob.pair().sign(header1.hash().as_ref())
				},]
			),
			Error::<Test>::NotebookSubmittedForLockedNotary
		);
	});
}

#[test]
fn it_can_submit_transactions_to_unlock_audits() {
	new_test_ext().execute_with(|| {
		NotariesLockedForFailedAudit::<Test>::insert(1, (1, 1, VerifyError::InvalidBlockVoteRoot));

		assert_eq!(
			Notebook::get_state(1),
			NotaryState::Locked {
				failed_audit_reason: VerifyError::InvalidBlockVoteRoot,
				at_tick: 1,
				notebook_number: 1
			}
		);

		let digest = notebook_digest(vec![(1, 1, 1, false)]);
		Digests::mutate(|d| d.notebooks = digest.clone());
		Notebook::on_initialize(2);
		CurrentTick::set(2);

		let header1 = make_header(1, 1);

		// test cannot submit when locked
		assert_err!(
			Notebook::submit(
				RuntimeOrigin::none(),
				vec![SignedNotebookHeader {
					header: header1.clone(),
					signature: Ed25519Keyring::Bob.pair().sign(header1.hash().as_ref())
				},]
			),
			Error::<Test>::NotebookSubmittedForLockedNotary
		);

		assert_err!(
			Notebook::unlock(RuntimeOrigin::signed(Ed25519Keyring::Alice.to_account_id()), 1),
			Error::<Test>::InvalidNotaryOperator
		);

		assert_ok!(Notebook::unlock(RuntimeOrigin::signed(Ed25519Keyring::Bob.to_account_id()), 1));
		assert_eq!(
			Notebook::get_state(1),
			NotaryState::Reactivated { reprocess_notebook_number: 1 }
		);

		// should be able to submit again
		assert_ok!(Notebook::submit(
			RuntimeOrigin::none(),
			vec![SignedNotebookHeader {
				header: header1.clone(),
				signature: Ed25519Keyring::Bob.pair().sign(header1.hash().as_ref())
			},]
		));
	});
}

#[test]
fn it_supports_multiple_notebooks() {
	new_test_ext().execute_with(|| {
		let digest = notebook_digest(vec![(1, 1, 1, false), (2, 1, 1, false)]);

		Digests::mutate(|d| d.notebooks = digest.clone());
		Notebook::on_initialize(1);
		CurrentTick::set(2);
		let header1 = make_header(1, 1);
		let mut header2 = make_header(1, 1);
		header2.notary_id = 2;
		assert_ok!(Notebook::submit(
			RuntimeOrigin::none(),
			vec![
				SignedNotebookHeader {
					header: header1.clone(),
					signature: Ed25519Keyring::Bob.pair().sign(header1.hash().as_ref())
				},
				SignedNotebookHeader {
					header: header2.clone(),
					signature: Ed25519Keyring::Bob.pair().sign(header2.hash().as_ref())
				}
			]
		));

		assert_eq!(LastNotebookDetailsByNotary::<Test>::get(1).len(), 1);
		assert_eq!(LastNotebookDetailsByNotary::<Test>::get(2).len(), 1);
		assert!(LastNotebookDetailsByNotary::<Test>::get(1)[0].1);
		assert!(LastNotebookDetailsByNotary::<Test>::get(2)[0].1);
		assert_eq!(Notebook::notebooks_at_tick(1), vec![(2, 1, None), (1, 1, None),]);
		assert_eq!(Notebook::notebooks_in_block(), vec![(1, 1, 1), (2, 1, 1),]);
		assert_eq!(
			Notebook::get_eligible_tick_votes_root(1, 1),
			Some((header1.block_votes_root, 1))
		);
	});
}

#[test]
fn it_tracks_notebooks_at_tick() {
	new_test_ext().execute_with(|| {
		CurrentTick::set(1);
		let header1 = make_header(1, 1);
		Notebook::process_notebook(header1);

		let mut header2 = make_header(1, 1);
		header2.notary_id = 2;
		Notebook::process_notebook(header2);

		assert_eq!(Notebook::notebooks_at_tick(1), vec![(2, 1, None), (1, 1, None),]);

		Digests::mutate(|d| d.notebooks = notebook_digest(vec![]));
		let mut header3 = make_header(2, 4);
		header3.notary_id = 2;
		Notebook::process_notebook(header3);
		assert_eq!(Notebook::notebooks_at_tick(4), vec![(2, 2, None)]);
	});
}

#[test]
fn it_requires_notebooks_in_order() {
	new_test_ext().execute_with(|| {
		let header = make_header(2, 1);
		assert_noop!(
			Notebook::verify_notebook_order(&header),
			Error::<Test>::MissingNotebookNumber
		);

		LastNotebookDetailsByNotary::<Test>::mutate(1, |v| {
			*v = bounded_vec!((
				NotaryNotebookKeyDetails {
					tick: 1,
					parent_secret: None,
					secret_hash: H256::random(),
					block_votes_root: H256::random(),
					notebook_number: 1,
				},
				true
			))
		});

		// only 1 notebook per tick per notary
		let header = make_header(2, 1);
		assert_noop!(
			Notebook::verify_notebook_order(&header),
			Error::<Test>::NotebookTickAlreadyUsed
		);

		// only 1 notebook per tick per notary
		let header = make_header(3, 1);
		assert_noop!(
			Notebook::verify_notebook_order(&header),
			Error::<Test>::MissingNotebookNumber
		);

		let header = make_header(2, 2);
		assert_ok!(Notebook::verify_notebook_order(&header),);
	});
}

#[test]
fn it_tracks_changed_accounts() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = Bob.to_account_id();
		set_argons(&who, 5000);
		ChainTransfers::mutate(|v| v.push((1, who.clone(), 1, 5000)));

		System::set_block_number(3);
		System::on_initialize(3);
		let changed_accounts_root = H256::random();
		let secrets = [H256::random(), H256::random(), H256::random()];
		let mut secret_hashes = vec![];
		// block number must be 1 prior to the current block number
		let mut header = make_header(1, 2);
		header.chain_transfers = bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }];
		header.changed_accounts_root = changed_accounts_root;
		header.changed_account_origins =
			bounded_vec![AccountOrigin { notebook_number: 1, account_uid: 1 }];
		header.secret_hash =
			NotebookHeader::create_secret_hash(secrets[0], header.block_votes_root, 1);
		secret_hashes.push(header.secret_hash);
		let first_votes = header.block_votes_root;
		CurrentTick::set(3);
		Notebook::process_notebook(header.clone());

		assert_eq!(
			NotebookChangedAccountsRootByNotary::<Test>::get(1, 1),
			Some(changed_accounts_root)
		);
		assert_eq!(
			AccountOriginLastChangedNotebookByNotary::<Test>::get(
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 }
			),
			Some(1)
		);

		System::set_block_number(4);
		System::on_initialize(4);

		let change_root_2 = H256::random();

		let mut header = make_header(2, 3);
		header.chain_transfers =
			bounded_vec![ChainTransfer::ToMainchain { account_id: who.clone(), amount: 5000 }];
		header.changed_accounts_root = change_root_2;
		header.changed_account_origins =
			bounded_vec![AccountOrigin { notebook_number: 1, account_uid: 1 }];
		header.secret_hash =
			NotebookHeader::create_secret_hash(secrets[1], header.block_votes_root, 2);
		secret_hashes.push(header.secret_hash);
		header.parent_secret = Some(secrets[0]);
		let second_votes = header.block_votes_root;
		CurrentTick::set(4);
		assert_ok!(Notebook::verify_notebook_order(&header),);
		Notebook::process_notebook(header);
		assert_eq!(Balances::free_balance(&who), 5000);
		assert_eq!(
			NotebookChangedAccountsRootByNotary::<Test>::get(1, 1),
			Some(changed_accounts_root)
		);
		assert_eq!(
			AccountOriginLastChangedNotebookByNotary::<Test>::get(
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 }
			),
			Some(2)
		);
		assert_eq!(NotebookChangedAccountsRootByNotary::<Test>::get(1, 2), Some(change_root_2));
		assert_eq!(
			LastNotebookDetailsByNotary::<Test>::get(1).into_inner(),
			vec![
				(
					NotaryNotebookKeyDetails {
						parent_secret: Some(secrets[0]),
						notebook_number: 2,
						tick: 3,
						secret_hash: secret_hashes[1],
						block_votes_root: second_votes
					},
					true
				),
				(
					NotaryNotebookKeyDetails {
						parent_secret: None,
						notebook_number: 1,
						tick: 2,
						secret_hash: secret_hashes[0],
						block_votes_root: first_votes
					},
					true
				),
			]
		);

		System::set_block_number(5);
		System::on_initialize(5);
		let mut header = make_header(3, 4);
		header.parent_secret = Some(secrets[1]);
		header.secret_hash =
			NotebookHeader::create_secret_hash(secrets[2], header.block_votes_root, 3);
		secret_hashes.push(header.secret_hash);

		CurrentTick::set(5);
		Notebook::process_notebook(header.clone());
		assert_eq!(
			LastNotebookDetailsByNotary::<Test>::get(1),
			vec![
				(
					NotaryNotebookKeyDetails {
						parent_secret: Some(secrets[1]),
						notebook_number: 3,
						tick: 4,
						secret_hash: secret_hashes[2],
						block_votes_root: header.block_votes_root
					},
					true
				),
				(
					NotaryNotebookKeyDetails {
						parent_secret: Some(secrets[0]),
						notebook_number: 2,
						tick: 3,
						secret_hash: secret_hashes[1],
						block_votes_root: second_votes
					},
					true
				),
				(
					NotaryNotebookKeyDetails {
						parent_secret: None,
						notebook_number: 1,
						tick: 2,
						secret_hash: secret_hashes[0],
						block_votes_root: first_votes
					},
					true
				),
			]
		);

		assert_eq!(
			AccountOriginLastChangedNotebookByNotary::<Test>::get(
				1,
				AccountOrigin { notebook_number: 1, account_uid: 1 }
			),
			Some(2)
		);
	});
}

#[test]
fn it_tracks_notebooks_received_out_of_tick() {
	new_test_ext().execute_with(|| {
		System::set_block_number(3);
		System::on_initialize(3);
		let secrets = [H256::random(), H256::random()];
		let mut secret_hashes = vec![];
		// block number must be 1 prior to the current block number
		let mut header1 = make_header(1, 2);
		header1.secret_hash =
			NotebookHeader::create_secret_hash(secrets[0], header1.block_votes_root, 1);
		secret_hashes.push(header1.secret_hash);
		CurrentTick::set(4);
		Notebook::process_notebook(header1.clone());

		let mut header2 = make_header(2, 3);
		header2.parent_secret = Some(secrets[0]);
		header2.secret_hash =
			NotebookHeader::create_secret_hash(secrets[1], header2.block_votes_root, 2);
		secret_hashes.push(header2.secret_hash);
		Notebook::process_notebook(header2.clone());

		let last_details = LastNotebookDetailsByNotary::<Test>::get(1);

		let (details_2, at_tick_2) = last_details.first().unwrap();
		assert_eq!(details_2.tick, header2.tick);
		assert_eq!(at_tick_2, &true);

		let (details_1, at_tick_1) = last_details.get(1).unwrap();
		assert_eq!(details_1.tick, header1.tick);
		assert_eq!(at_tick_1, &false);

		assert_eq!(Notebook::get_eligible_tick_votes_root(1, header1.tick), None);
		assert_eq!(
			Notebook::get_eligible_tick_votes_root(1, header2.tick),
			Some((header2.block_votes_root, header2.notebook_number))
		);
	})
}

#[test]
fn it_can_audit_notebooks() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = Bob.to_account_id();
		let notary_id = 1;
		set_argons(&who, 2000);
		ChainTransfers::mutate(|v| v.push((notary_id, who.clone(), 1, 2000)));

		System::set_block_number(2);

		let header = NotebookHeader {
			notary_id,
			notebook_number: 1,
			tick: 1,
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }],
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![
				BalanceTip {
					account_id: who.clone(),
					account_type: AccountType::Deposit,
					change_number: 1,
					balance: 2000,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					channel_hold_note: None,
				}
				.encode(),
			]),
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: bounded_vec![],
			block_votes_root: H256::zero(),
			block_votes_count: 1,
			secret_hash: H256::random(),
			parent_secret: None,
			domains: Default::default(),
		};

		let mut notebook = argon_primitives::notebook::Notebook {
			header,
			new_account_origins: bounded_vec![NewAccountOrigin::new(
				who.clone(),
				AccountType::Deposit,
				1
			)],
			hash: Default::default(),
			notarizations: bounded_vec![
				Notarization::new(
					vec![BalanceChange {
						account_id: who.clone(),
						account_type: AccountType::Deposit,
						balance: 2000,
						previous_balance_proof: None,
						change_number: 1,
						notes: bounded_vec![Note::create(
							2000,
							NoteType::ClaimFromMainchain { transfer_id: 1 },
						)],
						channel_hold_note: None,
						signature: ed25519::Signature::from_raw([0u8; 64]).into(),
					},],
					vec![],
					vec![]
				),
				Notarization::new(
					vec![],
					vec![BlockVote::create_default_vote(NotaryOperator::get(), 1)],
					vec![]
				)
			],
			signature: ed25519::Signature::from_raw([0u8; 64]),
		};
		notebook.header.block_votes_root =
			block_votes_root(notebook.notarizations.to_vec().clone());
		notebook.notarizations[0].balance_changes[0].sign(Bob.pair());
		notebook.hash = notebook.calculate_hash();
		notebook.signature = Ed25519Keyring::Bob.pair().sign(notebook.hash.as_ref());
		let header_hash = notebook.header.hash();

		assert_ok!(Notebook::audit_notebook(
			1,
			notary_id,
			1,
			1,
			header_hash,
			&notebook.encode(),
			vec![]
		));
	});
}

#[test]
fn it_sorts_block_votes_correctly() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = Bob.to_account_id();
		let notary_id = 1;
		set_argons(&who, 2000);
		ChainTransfers::mutate(|v| v.push((notary_id, who.clone(), 1, 500_000)));

		System::set_block_number(2);

		let vote_1 = BlockVote {
			account_id: Alice.to_account_id(),
			tick: 1,
			block_hash: H256::random(),
			signature: ed25519::Signature::from_raw([0u8; 64]).into(),
			index: 0,
			power: 1000,
			block_rewards_account_id: who.clone(),
		};

		let vote_2 = BlockVote {
			account_id: Alice.to_account_id(),
			tick: 1,
			block_hash: H256::random(),
			signature: ed25519::Signature::from_raw([0u8; 64]).into(),
			index: 1,
			power: 1000,
			block_rewards_account_id: Alice.to_account_id(),
		};

		let header = NotebookHeader {
			notary_id,
			notebook_number: 1,
			tick: 1,
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }],
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![
				BalanceTip {
					account_id: who.clone(),
					account_type: AccountType::Deposit,
					change_number: 2,
					balance: 400_000,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					channel_hold_note: None,
				}
				.encode(),
				BalanceTip {
					account_id: Alice.to_account_id(),
					account_type: AccountType::Tax,
					change_number: 1,
					balance: 18_000,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 2 },
					channel_hold_note: None,
				}
				.encode(),
				BalanceTip {
					account_id: Alice.to_account_id(),
					account_type: AccountType::Deposit,
					change_number: 1,
					balance: 80_000,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 3 },
					channel_hold_note: None,
				}
				.encode(),
			]),
			changed_account_origins: bounded_vec![
				AccountOrigin { notebook_number: 1, account_uid: 1 },
				AccountOrigin { notebook_number: 1, account_uid: 2 },
				AccountOrigin { notebook_number: 1, account_uid: 3 }
			],
			version: 1,
			tax: 20000,
			block_voting_power: 2000,
			blocks_with_votes: bounded_vec![],
			block_votes_root: H256::zero(),
			block_votes_count: 2,
			secret_hash: H256::random(),
			parent_secret: None,
			domains: Default::default(),
		};

		let mut notebook = argon_primitives::notebook::Notebook {
			header,
			new_account_origins: bounded_vec![
				NewAccountOrigin::new(who.clone(), AccountType::Deposit, 1),
				NewAccountOrigin::new(Alice.to_account_id(), AccountType::Tax, 2),
				NewAccountOrigin::new(Alice.to_account_id(), AccountType::Deposit, 3),
			],
			hash: Default::default(),
			notarizations: bounded_vec![Notarization::new(
				vec![
					BalanceChange {
						account_id: who.clone(),
						account_type: AccountType::Deposit,
						balance: 500_000,
						previous_balance_proof: None,
						change_number: 1,
						notes: bounded_vec![Note::create(
							500_000,
							NoteType::ClaimFromMainchain { transfer_id: 1 },
						)],
						channel_hold_note: None,
						signature: ed25519::Signature::from_raw([0u8; 64]).into(),
					},
					BalanceChange {
						account_id: Bob.to_account_id(),
						account_type: AccountType::Deposit,
						balance: 400_000,
						previous_balance_proof: Some(BalanceProof {
							notary_id: 1,
							balance: 500_000,
							tick: 1,
							account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
							notebook_number: 1,
							notebook_proof: None
						}),
						change_number: 2,
						notes: bounded_vec![Note::create(100_000, NoteType::Send { to: None }),],
						channel_hold_note: None,
						signature: ed25519::Signature::from_raw([0u8; 64]).into(),
					},
					BalanceChange {
						account_id: Alice.to_account_id(),
						account_type: AccountType::Deposit,
						balance: 80_000,
						previous_balance_proof: None,
						change_number: 1,
						notes: bounded_vec![
							Note::create(100_000, NoteType::Claim),
							Note::create(20_000, NoteType::Tax)
						],
						channel_hold_note: None,
						signature: ed25519::Signature::from_raw([0u8; 64]).into(),
					},
					BalanceChange {
						account_id: Alice.to_account_id(),
						account_type: AccountType::Tax,
						balance: 18_000,
						previous_balance_proof: None,
						change_number: 1,
						notes: bounded_vec![
							Note::create(20_000, NoteType::Claim),
							Note::create(2_000, NoteType::SendToVote)
						],
						channel_hold_note: None,
						signature: ed25519::Signature::from_raw([0u8; 64]).into(),
					},
				],
				vec![vote_1, vote_2],
				vec![]
			),],
			signature: ed25519::Signature::from_raw([0u8; 64]),
		};
		notebook.notarizations[0].balance_changes[0].sign(Bob.pair());
		notebook.notarizations[0].balance_changes[1].sign(Bob.pair());
		notebook.notarizations[0].balance_changes[2].sign(Alice.pair());
		notebook.notarizations[0].balance_changes[3].sign(Alice.pair());
		notebook.notarizations[0].block_votes[0].sign(Alice.pair());
		notebook.notarizations[0].block_votes[1].sign(Alice.pair());
		notebook.header.block_votes_root =
			block_votes_root(notebook.notarizations.to_vec().clone());
		notebook.header.blocks_with_votes = BoundedVec::truncate_from(
			notebook.notarizations[0]
				.block_votes
				.iter()
				.map(|a| a.block_hash)
				.collect::<Vec<_>>(),
		);
		notebook.hash = notebook.calculate_hash();
		notebook.signature = Ed25519Keyring::Bob.pair().sign(notebook.hash.as_ref());
		let header_hash = notebook.header.hash();

		assert_ok!(Notebook::audit_notebook(
			1,
			notary_id,
			1,
			1,
			header_hash,
			&notebook.encode(),
			vec![]
		));
	});
}

#[test]
fn it_handles_bad_secrets() {
	new_test_ext().execute_with(|| {
		new_test_ext().execute_with(|| {
			// Go past genesis block so events get deposited
			System::set_block_number(1);

			System::set_block_number(3);
			System::on_initialize(3);

			let secrets = [H256::random(), H256::random(), H256::random()];
			let mut secret_hashes = vec![];
			// block number must be 1 prior to the current block number
			let mut header = make_header(1, 2);
			header.secret_hash =
				NotebookHeader::create_secret_hash(secrets[0], header.block_votes_root, 1);
			secret_hashes.push(header.secret_hash);
			CurrentTick::set(2);
			Notebook::process_notebook(header.clone());

			System::set_block_number(4);
			System::on_initialize(4);

			let mut header = make_header(2, 3);
			header.secret_hash =
				NotebookHeader::create_secret_hash(secrets[1], header.block_votes_root, 2);
			secret_hashes.push(header.secret_hash);
			// wrong secret hash
			header.parent_secret = Some(secrets[1]);

			assert!(
				!Notebook::check_audit_result(
					1,
					header.notebook_number,
					header.tick,
					header.hash(),
					&NotebookDigest {
						notebooks: vec![NotebookAuditResult {
							notary_id: 1,
							notebook_number: header.notebook_number,
							tick: header.tick,
							audit_first_failure: None
						}]
					},
					header.parent_secret
				)
				.expect("shouldn't throw an error ")
			);
			System::assert_last_event(
				Event::<Test>::NotebookAuditFailure {
					notary_id: 1,
					notebook_number: 2,
					notebook_hash: header.hash(),
					first_failure_reason: VerifyError::InvalidSecretProvided,
				}
				.into(),
			);
		});
	})
}

#[test]
fn it_can_audit_notebooks_with_history() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = Bob.to_account_id();
		let notary_id = 1;
		set_argons(&who, 2000);
		ChainTransfers::mutate(|v| v.push((notary_id, who.clone(), 1, 2000)));

		System::set_block_number(2);

		let notebook_number = 5;
		let tick = 5;
		let account_root = BalanceTip {
			account_id: who.clone(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 2000,
			account_origin: AccountOrigin { notebook_number, account_uid: 1 },
			channel_hold_note: None,
		};

		let changed_accounts_root = merkle_root::<Blake2Hasher, _>(vec![account_root.encode()]);

		let mut header = make_header(notebook_number, tick);
		header.changed_accounts_root = changed_accounts_root;
		header.chain_transfers = bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }];
		header.changed_account_origins =
			bounded_vec![AccountOrigin { notebook_number, account_uid: 1 }];

		let merkle_leaves = vec![account_root.tip()];
		let account_1_proof = merkle_proof::<Blake2Hasher, _, _>(&merkle_leaves, 0);

		let mut notebook = argon_primitives::notebook::Notebook {
			header: header.clone(),
			new_account_origins: bounded_vec![NewAccountOrigin::new(
				who.clone(),
				AccountType::Deposit,
				1
			)],
			hash: Default::default(),
			notarizations: bounded_vec![
				Notarization::new(
					vec![BalanceChange {
						account_id: who.clone(),
						account_type: AccountType::Deposit,
						balance: 2000,
						previous_balance_proof: None,
						change_number: 1,
						notes: bounded_vec![Note::create(
							2000,
							NoteType::ClaimFromMainchain { transfer_id: 1 },
						)],
						channel_hold_note: None,
						signature: ed25519::Signature::from_raw([0u8; 64]).into(),
					},],
					vec![],
					vec![]
				),
				Notarization::new(
					vec![],
					vec![BlockVote::create_default_vote(NotaryOperator::get(), tick)],
					vec![]
				)
			],
			signature: ed25519::Signature::from_raw([0u8; 64]),
		};
		notebook.header.block_votes_root =
			block_votes_root(notebook.notarizations.to_vec().clone());
		notebook.notarizations[0].balance_changes[0].sign(Bob.pair());
		notebook.hash = notebook.calculate_hash();
		notebook.signature = Ed25519Keyring::Bob.pair().sign(notebook.hash.as_ref());

		LastNotebookDetailsByNotary::<Test>::mutate(notary_id, |v| {
			v.try_insert(
				0,
				(
					NotaryNotebookKeyDetails {
						tick: tick - 2,
						parent_secret: None,
						secret_hash: H256::random(),
						block_votes_root: H256::random(),
						notebook_number: notebook_number - 2,
					},
					true,
				),
			)
		})
		.expect("Couldn't insert details");
		let header_hash = notebook.header.hash();

		assert_err!(
			Notebook::audit_notebook(
				1,
				notary_id,
				notebook_number,
				tick,
				header_hash,
				&notebook.encode(),
				vec![]
			),
			VerifyError::CatchupNotebooksMissing
		);

		assert_err!(
			Notebook::audit_notebook(
				1,
				notary_id,
				notebook_number,
				tick,
				header_hash,
				&notebook.encode(),
				vec![NotaryNotebookAuditSummary {
					notary_id,
					notebook_number: notebook_number - 1,
					tick: tick - 1,
					version: 1,
					raw_data: NotaryNotebookAuditSummaryDetails {
						changed_accounts_root: H256::random(),
						account_changelist: vec![],
						used_transfers_to_localchain: vec![1],
						block_votes_root: H256::random(),
						secret_hash: H256::random(),
					}
					.encode()
				}]
			),
			VerifyError::HistoryLookupError {
				source: AccountHistoryLookupError::InvalidTransferToLocalchain
			}
		);
		assert_ok!(Notebook::audit_notebook(
			1,
			notary_id,
			notebook_number,
			tick,
			header_hash,
			&notebook.encode(),
			vec![NotaryNotebookAuditSummary {
				notary_id,
				notebook_number: notebook_number - 1,
				tick: tick - 1,
				version: 1,
				raw_data: NotaryNotebookAuditSummaryDetails {
					changed_accounts_root: H256::random(),
					account_changelist: vec![],
					used_transfers_to_localchain: vec![],
					block_votes_root: H256::random(),
					secret_hash: H256::random(),
				}
				.encode()
			}]
		),);

		LastNotebookDetailsByNotary::<Test>::mutate(notary_id, |v| {
			v.try_insert(
				0,
				(
					NotaryNotebookKeyDetails {
						tick,
						parent_secret: None,
						secret_hash: H256::random(),
						block_votes_root: header.changed_accounts_root,
						notebook_number,
					},
					true,
				),
			)
		})
		.expect("Couldn't insert details");
		AccountOriginLastChangedNotebookByNotary::<Test>::mutate(
			1,
			AccountOrigin { notebook_number, account_uid: 1 },
			|a| *a = Some(5),
		);
		<NotebookChangedAccountsRootByNotary<Test>>::insert(
			notary_id,
			notebook_number,
			changed_accounts_root,
		);

		// Test that account change history takes too
		let notebook_number = 7;
		let tick = 7;
		let mut header = make_header(notebook_number, tick);
		header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(vec![
			BalanceTip {
				account_id: who.clone(),
				account_type: AccountType::Deposit,
				change_number: 2,
				balance: 1000,
				account_origin: AccountOrigin { notebook_number: 5, account_uid: 1 },
				channel_hold_note: None,
			}
			.encode(),
			BalanceTip {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Tax,
				change_number: 1,
				balance: 200,
				account_origin: AccountOrigin { notebook_number, account_uid: 2 },
				channel_hold_note: None,
			}
			.encode(),
			BalanceTip {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				balance: 800,
				account_origin: AccountOrigin { notebook_number, account_uid: 1 },
				channel_hold_note: None,
			}
			.encode(),
		]);
		header.changed_account_origins = bounded_vec![
			AccountOrigin { notebook_number: 5, account_uid: 1 },
			AccountOrigin { notebook_number: 7, account_uid: 1 },
			AccountOrigin { notebook_number: 7, account_uid: 2 }
		];
		header.tax = 200;

		let mut notebook = argon_primitives::notebook::Notebook {
			header,
			new_account_origins: bounded_vec![
				NewAccountOrigin::new(Alice.to_account_id(), AccountType::Deposit, 1),
				NewAccountOrigin::new(Alice.to_account_id(), AccountType::Tax, 2)
			],
			hash: Default::default(),
			notarizations: bounded_vec![
				Notarization::new(
					vec![
						BalanceChange {
							account_id: who.clone(),
							account_type: AccountType::Deposit,
							balance: 1000,
							previous_balance_proof: Some(BalanceProof {
								notary_id,
								notebook_number: 5,
								notebook_proof: Some(MerkleProof {
									proof: BoundedVec::truncate_from(account_1_proof.proof),
									number_of_leaves: 1,
									leaf_index: 0
								}),
								tick: 5,
								balance: 2000,
								account_origin: AccountOrigin {
									notebook_number: 5,
									account_uid: 1
								},
							}),
							change_number: 2,
							notes: bounded_vec![Note::create(1000, NoteType::Send { to: None })],
							channel_hold_note: None,
							signature: ed25519::Signature::from_raw([0u8; 64]).into(),
						},
						BalanceChange {
							account_id: Alice.to_account_id(),
							account_type: AccountType::Deposit,
							balance: 800,
							previous_balance_proof: None,
							change_number: 1,
							notes: bounded_vec![
								Note::create(1000, NoteType::Claim),
								Note::create(200, NoteType::Tax)
							],
							channel_hold_note: None,
							signature: ed25519::Signature::from_raw([0u8; 64]).into(),
						},
						BalanceChange {
							account_id: Alice.to_account_id(),
							account_type: AccountType::Tax,
							balance: 200,
							previous_balance_proof: None,
							change_number: 1,
							notes: bounded_vec![Note::create(200, NoteType::Claim)],
							channel_hold_note: None,
							signature: ed25519::Signature::from_raw([0u8; 64]).into(),
						},
					],
					vec![],
					vec![]
				),
				Notarization::new(
					vec![],
					vec![BlockVote::create_default_vote(NotaryOperator::get(), tick)],
					vec![]
				)
			],
			signature: ed25519::Signature::from_raw([0u8; 64]),
		};
		notebook.notarizations[0].balance_changes[0].sign(Bob.pair());
		notebook.notarizations[0].balance_changes[1].sign(Alice.pair());
		notebook.notarizations[0].balance_changes[2].sign(Alice.pair());
		notebook.header.block_votes_root =
			block_votes_root(notebook.notarizations.to_vec().clone());
		notebook.hash = notebook.calculate_hash();
		notebook.signature = Ed25519Keyring::Bob.pair().sign(notebook.hash.as_ref());

		let header_hash = notebook.header.hash();

		assert_err!(
			Notebook::audit_notebook(
				1,
				notary_id,
				notebook_number,
				tick,
				header_hash,
				&notebook.encode(),
				vec![NotaryNotebookAuditSummary {
					notary_id,
					notebook_number: notebook_number - 1,
					tick: tick - 1,
					version: 1,
					raw_data: NotaryNotebookAuditSummaryDetails {
						changed_accounts_root: H256::random(),
						account_changelist: vec![AccountOrigin {
							notebook_number: 5,
							account_uid: 1
						}],
						used_transfers_to_localchain: vec![],
						block_votes_root: H256::random(),
						secret_hash: H256::random(),
					}
					.encode()
				}]
			),
			VerifyError::InvalidPreviousBalanceChangeNotebook
		);
		assert_ok!(Notebook::audit_notebook(
			1,
			notary_id,
			notebook_number,
			tick,
			header_hash,
			&notebook.encode(),
			vec![NotaryNotebookAuditSummary {
				notary_id,
				notebook_number: notebook_number - 1,
				tick: tick - 1,
				version: 1,
				raw_data: NotaryNotebookAuditSummaryDetails {
					changed_accounts_root: H256::random(),
					account_changelist: vec![],
					used_transfers_to_localchain: vec![],
					block_votes_root: H256::random(),
					secret_hash: H256::random(),
				}
				.encode()
			}]
		),);
	});
}

fn block_votes_root(notarizations: Vec<Notarization>) -> H256 {
	let mut votes = BTreeMap::new();
	for notarization in &notarizations {
		for vote in &notarization.block_votes {
			votes.insert((vote.account_id.clone(), vote.index), vote.clone());
		}
	}
	let merkle_leafs = votes.values().map(|x| x.encode()).collect::<Vec<_>>();
	merkle_root::<Blake2Hasher, _>(merkle_leafs)
}

fn make_header(notebook_number: NotebookNumber, tick: Tick) -> NotebookHeader {
	NotebookHeader {
		notary_id: 1,
		notebook_number,
		tick,
		chain_transfers: Default::default(),
		changed_accounts_root: H256::random(),
		changed_account_origins: Default::default(),
		version: 1,
		tax: 0,
		block_voting_power: 0,
		blocks_with_votes: Default::default(),
		block_votes_root: H256::zero(),
		secret_hash: H256::random(),
		parent_secret: None,
		block_votes_count: 1,
		domains: Default::default(),
	}
}
