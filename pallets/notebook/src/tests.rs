use std::collections::BTreeMap;

use binary_merkle_tree::merkle_root;
use codec::Encode;
use frame_support::{assert_err, assert_noop, assert_ok, traits::OnInitialize};
use sp_core::{bounded_vec, ed25519, Blake2Hasher, Pair};
use sp_keyring::{AccountKeyring::Bob, Ed25519Keyring};
use sp_runtime::{testing::H256, traits::UniqueSaturatedInto, Digest, DigestItem};

use ulx_notary_audit::VerifyError;
use ulx_primitives::{
	localchain::{AccountType, BalanceChange, Note, NoteType},
	notary::NotaryNotebookKeyDetails,
	notebook::{
		AccountOrigin, BalanceTip, ChainTransfer, NewAccountOrigin, Notarization, NotebookHeader,
		NotebookNumber,
	},
	tick::Tick,
	NotaryId, NotebookDigest, NotebookDigestRecord, NotebookProvider, SignedNotebookHeader,
	NOTEBOOKS_DIGEST_ID,
};

use crate::{
	mock::*,
	pallet::{
		AccountOriginLastChangedNotebookByNotary, LastNotebookDetailsByNotary,
		NotebookChangedAccountsRootByNotary,
	},
	Error,
};

fn notebook_digest(books: Vec<(NotaryId, NotebookNumber, Tick, bool)>) -> DigestItem {
	DigestItem::PreRuntime(
		NOTEBOOKS_DIGEST_ID,
		NotebookDigest {
			notebooks: books
				.into_iter()
				.map(|(notary_id, notebook_number, tick, has_error)| NotebookDigestRecord {
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
		.encode(),
	)
}

#[test]
#[should_panic]
fn it_should_panic_if_no_notebook_digest() {
	new_test_ext().execute_with(|| Notebook::on_initialize(1));
}

#[test]
fn it_ensures_only_a_single_inherent_is_submitted() {
	new_test_ext().execute_with(|| {
		let digest = notebook_digest(vec![]);
		System::initialize(&1, &System::parent_hash(), &Digest { logs: vec![digest.clone()] });
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
		System::initialize(&1, &System::parent_hash(), &Digest { logs: vec![digest.clone()] });
		Notebook::on_initialize(1);
		CurrentTick::set(1);
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
		// should store that it's no longer valid
		assert_eq!(LastNotebookDetailsByNotary::<Test>::get(2)[0].1, false);
		// this is the default verify error
		assert_eq!(Notebook::notary_failed_audit_by_id(1), None);
		assert_eq!(
			Notebook::notary_failed_audit_by_id(2),
			Some((1, 1, VerifyError::InvalidBlockVoteRoot))
		);
		assert_eq!(Notebook::is_notary_locked_at_tick(2, 1), true);
		assert_eq!(Notebook::is_notary_locked_at_tick(2, 2), true);
		assert_eq!(Notebook::is_notary_locked_at_tick(2, 0), false);
	});
}

#[test]
fn it_supports_multiple_notebooks() {
	new_test_ext().execute_with(|| {
		let digest = notebook_digest(vec![(1, 1, 1, false), (2, 1, 1, false)]);
		System::initialize(&1, &System::parent_hash(), &Digest { logs: vec![digest.clone()] });
		Notebook::on_initialize(1);
		CurrentTick::set(1);
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
		assert_eq!(LastNotebookDetailsByNotary::<Test>::get(1)[0].1, true);
		assert_eq!(LastNotebookDetailsByNotary::<Test>::get(2)[0].1, true);
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
		assert_ok!(Notebook::process_notebook(header1, true),);

		let mut header2 = make_header(1, 1);
		header2.notary_id = 2;
		assert_ok!(Notebook::process_notebook(header2, true),);

		assert_eq!(Notebook::notebooks_at_tick(1), vec![(2, 1, None), (1, 1, None),]);

		System::initialize(
			&2,
			&System::parent_hash(),
			&Digest { logs: vec![notebook_digest(vec![])] },
		);

		let mut header3 = make_header(2, 4);
		header3.notary_id = 2;
		assert_ok!(Notebook::process_notebook(header3, true),);
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
		let nonce = System::account_nonce(&who).into();
		set_argons(&who, 5000);
		ChainTransfers::mutate(|v| v.push((1, who.clone(), nonce, 5000)));

		System::set_block_number(3);
		System::on_initialize(3);
		let changed_accounts_root = H256::random();
		let secrets = vec![H256::random(), H256::random(), H256::random()];
		let mut secret_hashes = vec![];
		// block number must be 1 prior to the current block number
		let mut header = make_header(1, 2);
		header.chain_transfers = bounded_vec![ChainTransfer::ToLocalchain {
			account_id: who.clone(),
			account_nonce: nonce.unique_saturated_into()
		}];
		header.changed_accounts_root = changed_accounts_root.clone();
		header.changed_account_origins =
			bounded_vec![AccountOrigin { notebook_number: 1, account_uid: 1 }];
		header.secret_hash =
			NotebookHeader::create_secret_hash(secrets[0], header.block_votes_root, 1);
		secret_hashes.push(header.secret_hash);
		let first_votes = header.block_votes_root;
		CurrentTick::set(2);
		assert_ok!(Notebook::process_notebook(header.clone(), true));

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
		header.changed_accounts_root = change_root_2.clone();
		header.changed_account_origins =
			bounded_vec![AccountOrigin { notebook_number: 1, account_uid: 1 }];
		header.secret_hash =
			NotebookHeader::create_secret_hash(secrets[1], header.block_votes_root, 2);
		secret_hashes.push(header.secret_hash);
		// wrong secret hash
		header.parent_secret = Some(secrets[1]);

		assert_err!(Notebook::verify_notebook_order(&header), Error::<Test>::InvalidSecretProvided);
		header.parent_secret = Some(secrets[0]);
		let second_votes = header.block_votes_root;
		CurrentTick::set(3);
		assert_ok!(Notebook::verify_notebook_order(&header),);
		assert_ok!(Notebook::process_notebook(header, true),);
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

		CurrentTick::set(4);
		assert_ok!(Notebook::process_notebook(header.clone(), true),);
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
		let secrets = vec![H256::random(), H256::random()];
		let mut secret_hashes = vec![];
		// block number must be 1 prior to the current block number
		let mut header1 = make_header(1, 2);
		header1.secret_hash =
			NotebookHeader::create_secret_hash(secrets[0], header1.block_votes_root, 1);
		secret_hashes.push(header1.secret_hash);
		CurrentTick::set(3);
		assert_ok!(Notebook::process_notebook(header1.clone(), true),);
		let mut header2 = make_header(2, 3);
		header2.parent_secret = Some(secrets[0]);
		header2.secret_hash =
			NotebookHeader::create_secret_hash(secrets[1], header2.block_votes_root, 2);
		secret_hashes.push(header2.secret_hash);
		assert_ok!(Notebook::process_notebook(header2.clone(), true),);

		let last_details = LastNotebookDetailsByNotary::<Test>::get(1);

		let (details_2, at_tick_2) = last_details.get(0).unwrap();
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
		let nonce = System::account_nonce(&who);
		let notary_id = 1;
		System::inc_account_nonce(&who);
		set_argons(&who, 2000);
		ChainTransfers::mutate(|v| v.push((notary_id, who.clone(), nonce, 2000)));

		System::set_block_number(2);

		let header = NotebookHeader {
			notary_id,
			notebook_number: 1,
			tick: 1,
			finalized_block_number: 1,
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain {
				account_id: who.clone(),
				account_nonce: nonce.unique_saturated_into()
			}],
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![BalanceTip {
				account_id: who.clone(),
				account_type: AccountType::Deposit,
				change_number: 1,
				balance: 2000,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
				channel_hold_note: None,
			}
			.encode()]),
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			version: 1,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: bounded_vec![],
			block_votes_root: H256::zero(),
			secret_hash: H256::random(),
			parent_secret: None,
			block_votes_count: 0,
		};
		let header_hash = header.hash();

		let mut notebook = ulx_primitives::notebook::Notebook {
			header,
			new_account_origins: bounded_vec![NewAccountOrigin::new(
				who.clone(),
				AccountType::Deposit,
				1
			)],
			hash: Default::default(),
			notarizations: bounded_vec![Notarization::new(
				vec![BalanceChange {
					account_id: who.clone(),
					account_type: AccountType::Deposit,
					balance: 2000,
					previous_balance_proof: None,
					change_number: 1,
					notes: bounded_vec![Note::create(
						2000,
						NoteType::ClaimFromMainchain {
							account_nonce: nonce.unique_saturated_into()
						},
					)],
					channel_hold_note: None,
					signature: ed25519::Signature([0u8; 64]).into(),
				},],
				vec![]
			)],
			signature: ed25519::Signature([0u8; 64]),
		};
		notebook.notarizations[0].balance_changes[0].sign(Bob.pair());
		notebook.hash = notebook.calculate_hash();
		notebook.signature = Ed25519Keyring::Bob.pair().sign(notebook.hash.as_ref());

		let eligibility = BTreeMap::new();

		assert_ok!(Notebook::audit_notebook(
			1,
			notary_id,
			1,
			header_hash,
			&eligibility,
			&notebook.encode(),
		));
	});
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
		finalized_block_number: 1,
		tax: 0,
		block_voting_power: 0,
		blocks_with_votes: Default::default(),
		block_votes_root: H256::random(),
		secret_hash: H256::random(),
		parent_secret: None,
		block_votes_count: 0,
	}
}