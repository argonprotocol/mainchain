use std::collections::BTreeMap;

use binary_merkle_tree::merkle_root;
use codec::Encode;
use frame_support::{assert_err, assert_noop, assert_ok, traits::OnInitialize};
use sp_core::{blake2_256, bounded_vec, ed25519, Blake2Hasher, Pair};
use sp_keyring::{AccountKeyring::Bob, Ed25519Keyring};
use sp_runtime::{
	testing::H256,
	traits::{UniqueSaturatedInto, ValidateUnsigned},
	transaction_validity::{InvalidTransaction, TransactionSource},
};

use ulx_primitives::{
	localchain::{AccountType, BalanceChange, Note, NoteType},
	notary::NotaryNotebookKeyDetails,
	notebook::{
		AccountOrigin, BalanceTip, ChainTransfer, NewAccountOrigin, Notarization, NotebookHeader,
		NotebookNumber,
	},
};

use crate::{
	mock::*,
	pallet::{
		AccountOriginLastChangedNotebookByNotary, LastNotebookDetailsByNotary,
		NotebookChangedAccountsRootByNotary,
	},
	Error,
};

#[test]
fn it_validates_unsigned() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		let mut header = make_header(1, 3);
		assert_noop!(
			Notebook::validate_unsigned(
				TransactionSource::Local,
				&crate::Call::submit {
					header: header.clone(),
					hash: H256::random(),
					signature: ed25519::Signature([0u8; 64]),
				},
			),
			InvalidTransaction::Future
		);

		System::set_block_number(3);
		// in past, should mark stale
		header.block_number = 1;
		assert_noop!(
			Notebook::validate_unsigned(
				TransactionSource::Local,
				&crate::Call::submit {
					header: header.clone(),
					hash: H256::random(),
					signature: ed25519::Signature([0u8; 64]),
				},
			),
			InvalidTransaction::Stale
		);

		// should have a bad hash now
		header.block_number = 2;
		assert_noop!(
			Notebook::validate_unsigned(
				TransactionSource::Local,
				&crate::Call::submit {
					header: header.clone(),
					hash: H256::random(),
					signature: ed25519::Signature([0u8; 64]),
				},
			),
			InvalidTransaction::BadProof
		);

		assert_ok!(Notebook::validate_unsigned(
			TransactionSource::Local,
			&crate::Call::submit {
				header: header.clone(),
				hash: header.hash(),
				signature: ed25519::Signature([0u8; 64]),
			},
		));
	});
}

#[test]
fn it_requires_notebooks_in_order() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(2);
		LastNotebookDetailsByNotary::<Test>::mutate(1, |v| {
			*v = bounded_vec!(NotaryNotebookKeyDetails {
				block_number: 1,
				secret_hash: H256::random(),
				block_votes_root: H256::random(),
				notebook_number: 10,
			})
		});
		let header = make_header(8, 2);
		assert_noop!(
			Notebook::validate_unsigned(
				TransactionSource::Local,
				&crate::Call::submit {
					header: header.clone(),
					hash: header.hash(),
					signature: ed25519::Signature([0u8; 64]),
				},
			),
			InvalidTransaction::Stale
		);
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
		ChainTransfers::mutate(|v| v.push((1, who.clone(), nonce)));

		System::set_block_number(3);
		System::on_initialize(3);
		let changed_accounts_root = H256::random();
		let secrets = vec![H256::random(), H256::random(), H256::random()];
		let secret_hashes =
			secrets.iter().map(|s| blake2_256(&s[..]).into()).collect::<Vec<H256>>();
		// block number must be 1 prior to the current block number
		let mut header = make_header(1, 2);
		header.chain_transfers = bounded_vec![ChainTransfer::ToLocalchain {
			account_id: who.clone(),
			account_nonce: nonce.unique_saturated_into()
		}];
		header.changed_accounts_root = changed_accounts_root.clone();
		header.changed_account_origins =
			bounded_vec![AccountOrigin { notebook_number: 1, account_uid: 1 }];
		header.secret_hash = secret_hashes[0];
		let first_votes = header.block_votes_root;
		assert_ok!(Notebook::submit(
			RuntimeOrigin::none(),
			header.clone(),
			header.hash(),
			ed25519::Signature([0u8; 64]),
		));

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
		header.secret_hash = secret_hashes[1];
		// wrong secret hash
		header.parent_secret = Some(secrets[1]);

		assert_err!(
			Notebook::submit(
				RuntimeOrigin::none(),
				header.clone(),
				header.hash(),
				ed25519::Signature([0u8; 64]),
			),
			Error::<Test>::InvalidSecretProvided
		);
		header.parent_secret = Some(secrets[0]);
		let second_votes = header.block_votes_root;
		assert_ok!(Notebook::submit(
			RuntimeOrigin::none(),
			header.clone(),
			header.hash(),
			ed25519::Signature([0u8; 64]),
		));
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
				NotaryNotebookKeyDetails {
					notebook_number: 2,
					block_number: 4,
					secret_hash: secret_hashes[1],
					block_votes_root: second_votes
				},
				NotaryNotebookKeyDetails {
					notebook_number: 1,
					block_number: 3,
					secret_hash: secret_hashes[0],
					block_votes_root: first_votes
				},
			]
		);

		System::set_block_number(5);
		System::on_initialize(5);
		let mut header = make_header(3, 4);
		header.parent_secret = Some(secrets[1]);
		header.secret_hash = secret_hashes[2];

		assert_ok!(Notebook::submit(
			RuntimeOrigin::none(),
			header.clone(),
			header.hash(),
			ed25519::Signature([0u8; 64]),
		));
		assert_eq!(
			LastNotebookDetailsByNotary::<Test>::get(1),
			vec![
				NotaryNotebookKeyDetails {
					notebook_number: 3,
					block_number: 5,
					secret_hash: secret_hashes[2],
					block_votes_root: header.block_votes_root
				},
				NotaryNotebookKeyDetails {
					notebook_number: 2,
					block_number: 4,
					secret_hash: secret_hashes[1],
					block_votes_root: second_votes
				},
				NotaryNotebookKeyDetails {
					notebook_number: 1,
					block_number: 3,
					secret_hash: secret_hashes[0],
					block_votes_root: first_votes
				},
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
fn it_can_audit_notebooks() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let who = Bob.to_account_id();
		let nonce = System::account_nonce(&who);
		let notary_id = 1;
		System::inc_account_nonce(&who);
		set_argons(&who, 2000);
		ChainTransfers::mutate(|v| v.push((notary_id, who.clone(), nonce)));

		System::set_block_number(2);
		Notebook::on_initialize(2);

		let header = NotebookHeader {
			notary_id,
			notebook_number: 1,
			block_number: 1,
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
			start_time: 0,
			end_time: 0,
			tax: 0,
			block_voting_power: 0,
			blocks_with_votes: bounded_vec![],
			block_votes_root: H256::zero(),
			secret_hash: H256::random(),
			best_block_nonces: bounded_vec![],
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

fn make_header(notebook_number: NotebookNumber, block_number: u32) -> NotebookHeader {
	NotebookHeader {
		notary_id: 1,
		notebook_number,
		block_number,
		chain_transfers: Default::default(),
		changed_accounts_root: H256::random(),
		changed_account_origins: Default::default(),
		version: 1,
		finalized_block_number: 1,
		start_time: 0,
		end_time: 0,
		tax: 0,
		block_voting_power: 0,
		blocks_with_votes: Default::default(),
		block_votes_root: H256::random(),
		secret_hash: H256::random(),
		best_block_nonces: Default::default(),
		parent_secret: None,
		block_votes_count: 0,
	}
}
