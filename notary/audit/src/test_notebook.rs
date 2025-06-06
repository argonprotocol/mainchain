use std::collections::{BTreeMap, BTreeSet};

use binary_merkle_tree::{merkle_proof, merkle_root};
use codec::Encode;
use frame_support::{assert_err, assert_ok, parameter_types};
use polkadot_sdk::*;
use sp_core::{
	Blake2Hasher, H256, Pair, bounded::BoundedVec, bounded_vec, crypto::AccountId32, ed25519,
	sr25519::Signature,
};
use sp_keyring::{
	Ed25519Keyring,
	Ed25519Keyring::{Dave, Ferdie},
	Sr25519Keyring::{Alice, Bob},
};
use sp_runtime::traits::BlakeTwo256;

use crate::{
	AccountHistoryLookupError, NotebookHistoryLookup, VerifyError, verify_previous_balance_proof,
};
use argon_primitives::{
	AccountId, AccountType, Balance, BalanceTip, BlockVote, ChainTransfer, LocalchainAccountId,
	MerkleProof, MultiSignatureBytes, NewAccountOrigin, Notarization, Notebook, NotebookHeader,
	NotebookNumber, TransferToLocalchainId,
	balance_change::{AccountOrigin, BalanceChange, BalanceProof},
	note::{Note, NoteType},
	tick::Tick,
};

use super::notebook_verify;

fn empty_signature() -> MultiSignatureBytes {
	Signature::from_raw([0u8; 64]).into()
}

struct TestLookup;

parameter_types! {
	pub static NotebookRoots: BTreeMap<u32, H256> = BTreeMap::new();
	pub static LastChangedNotebook: BTreeMap<AccountOrigin, u32> = BTreeMap::new();
	pub static ValidLocalchainTransfers: BTreeSet<(AccountId32, TransferToLocalchainId)> = BTreeSet::new();
}

impl NotebookHistoryLookup for TestLookup {
	fn get_account_changes_root(
		&self,
		_notary_id: u32,
		notebook_number: NotebookNumber,
	) -> Result<H256, AccountHistoryLookupError> {
		NotebookRoots::get()
			.get(&notebook_number)
			.ok_or(AccountHistoryLookupError::RootNotFound)
			.cloned()
	}
	fn get_last_changed_notebook(
		&self,
		_notary_id: u32,
		account_origin: AccountOrigin,
	) -> Result<u32, AccountHistoryLookupError> {
		LastChangedNotebook::get()
			.get(&account_origin)
			.cloned()
			.ok_or(AccountHistoryLookupError::LastChangeNotFound)
	}
	fn is_valid_transfer_to_localchain(
		&self,
		_notary_id: u32,
		transfer_to_localchain_id: TransferToLocalchainId,
		account_id: &AccountId32,
		_microgons: Balance,
		_at_tick: Tick,
	) -> Result<bool, AccountHistoryLookupError> {
		ValidLocalchainTransfers::get()
			.get(&(account_id.clone(), transfer_to_localchain_id))
			.cloned()
			.ok_or(AccountHistoryLookupError::InvalidTransferToLocalchain)
			.map(|_| true)
	}
}

#[test]
fn test_verify_previous_balance() {
	let mut final_balances = BTreeMap::<LocalchainAccountId, BalanceTip>::new();
	let account_id = Alice.to_account_id();
	let account_type = AccountType::Deposit;
	let localchain_account_id = LocalchainAccountId::new(account_id.clone(), account_type);

	let mut change = BalanceChange {
		account_id,
		account_type,
		change_number: 500_000,
		balance: 0,
		previous_balance_proof: None,
		channel_hold_note: None,
		notes: bounded_vec![],
		signature: empty_signature(),
	};
	let leaves = vec![
		BalanceTip {
			account_id: Dave.to_account_id(),
			account_type: AccountType::Deposit,
			balance: 20,
			change_number: 3,
			account_origin: AccountOrigin { notebook_number: 5, account_uid: 2 },
			channel_hold_note: None,
		}
		.encode(),
		BalanceTip {
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			balance: 100,
			change_number: 1,
			account_origin: AccountOrigin { notebook_number: 6, account_uid: 1 },
			channel_hold_note: None,
		}
		.encode(),
		BalanceTip {
			account_id: change.account_id.clone(),
			account_type: change.account_type,
			balance: 100,
			change_number: change.change_number - 1,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			channel_hold_note: None,
		}
		.encode(),
	];
	let merkle_root = merkle_root::<Blake2Hasher, _>(&leaves);
	NotebookRoots::mutate(|a| {
		a.insert(7, H256::from_slice([&[0u8], &merkle_root[0..31]].concat().as_ref()))
	});
	let origin = AccountOrigin { notebook_number: 1, account_uid: 1 };
	LastChangedNotebook::mutate(|c| c.insert(origin.clone(), 10));

	let proof = merkle_proof::<Blake2Hasher, _, _>(leaves, 2);
	change.previous_balance_proof = Some(BalanceProof {
		notary_id: 1,
		notebook_number: 7,
		tick: 7,
		notebook_proof: Some(MerkleProof {
			proof: BoundedVec::truncate_from(proof.proof),
			leaf_index: proof.leaf_index as u32,
			number_of_leaves: proof.number_of_leaves as u32,
		}),
		account_origin: origin.clone(),
		balance: 100,
	});

	assert_err!(
		verify_previous_balance_proof(
			&TestLookup,
			&change.previous_balance_proof.clone().unwrap(),
			7,
			&mut final_balances,
			&change,
			&localchain_account_id,
		),
		VerifyError::InvalidPreviousBalanceChangeNotebook
	);

	LastChangedNotebook::mutate(|c| c.insert(origin, 7));
	assert_err!(
		verify_previous_balance_proof(
			&TestLookup,
			&change.previous_balance_proof.clone().unwrap(),
			7,
			&mut final_balances,
			&change,
			&localchain_account_id,
		),
		VerifyError::InvalidPreviousBalanceProof
	);

	NotebookRoots::mutate(|a| a.insert(7, merkle_root));
	assert_ok!(verify_previous_balance_proof(
		&TestLookup,
		&change.previous_balance_proof.clone().unwrap(),
		7,
		&mut final_balances,
		&change,
		&localchain_account_id,
	));
}

#[test]
fn test_verify_notebook() {
	let note = Note::create(1_000_000, NoteType::ClaimFromMainchain { transfer_id: 1 });

	let alice_balance_changeset = vec![
		BalanceChange {
			balance: 1_000_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![note],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone(),
	];
	let notebook_header1 = NotebookHeader {
		version: 1,
		notary_id: 1,
		notebook_number: 1,
		tick: 1,
		changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![
			BalanceTip {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 1_000_000,
				change_number: 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
				channel_hold_note: None,
			}
			.encode(),
		]),
		chain_transfers: bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }],
		tax: 0,
		changed_account_origins: bounded_vec![AccountOrigin { notebook_number: 1, account_uid: 1 }],
		// Block Votes
		parent_secret: None,
		secret_hash: H256::from_slice(&[0u8; 32]),
		block_voting_power: 0,
		block_votes_root: H256::from_slice(&[0u8; 32]),
		block_votes_count: 0,
		blocks_with_votes: bounded_vec![],
		domains: bounded_vec![],
	};

	ValidLocalchainTransfers::mutate(|a| a.insert((Alice.to_account_id(), 1)));
	let hash = notebook_header1.hash();

	let mut notebook1 = Notebook {
		header: notebook_header1.clone(),
		notarizations: bounded_vec![Notarization::new(
			alice_balance_changeset.clone(),
			vec![],
			vec![]
		),],
		new_account_origins: bounded_vec![NewAccountOrigin::new(
			Alice.to_account_id(),
			AccountType::Deposit,
			1
		)],
		hash,
		signature: ed25519::Signature::from_raw([0u8; 64]),
	};

	let operator = notary_operator();
	notebook1.hash = notebook1.calculate_hash();

	// must have a default vote
	assert_err!(
		notebook_verify(&TestLookup, &notebook1, &operator, 2),
		VerifyError::NoDefaultBlockVote
	);
	notebook1.header.block_votes_count = 1;
	notebook1
		.notarizations
		.try_push(Notarization::new(
			vec![],
			vec![BlockVote::create_default_vote(notary_operator(), 1)],
			vec![],
		))
		.unwrap();
	notebook1.header.block_votes_root = block_votes_root(notebook1.notarizations.to_vec());

	notebook1.hash = notebook1.calculate_hash();
	assert_ok!(notebook_verify(&TestLookup, &notebook1, &operator, 2),);

	let mut bad_hash = hash;
	bad_hash.0[0] = 1;
	notebook1.hash = bad_hash;
	assert_err!(
		notebook_verify(&TestLookup, &notebook1, &operator, 2),
		VerifyError::InvalidNotebookHash
	);

	let mut bad_notebook1 = notebook1.clone();
	let _ = bad_notebook1
		.header
		.chain_transfers
		.try_insert(0, ChainTransfer::ToLocalchain { transfer_id: 2 });
	bad_notebook1.hash = hash;

	assert_err!(
		notebook_verify(&TestLookup, &bad_notebook1, &operator, 2),
		VerifyError::InvalidChainTransfersList
	);

	let mut bad_notebook = notebook1.clone();
	bad_notebook.header.changed_accounts_root.0[0] = 1;
	bad_notebook1.hash = hash;

	assert_err!(
		notebook_verify(&TestLookup, &bad_notebook, &operator, 2),
		VerifyError::InvalidBalanceChangeRoot
	);
}

#[test]
fn test_disallows_double_claim() {
	let note1 = Note::create(1_000_000, NoteType::ClaimFromMainchain { transfer_id: 1 });
	let note2 = Note::create(1_000_000, NoteType::ClaimFromMainchain { transfer_id: 1 });

	let alice_balance_changeset = vec![
		BalanceChange {
			balance: 2_000_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![note1, note2],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone(),
	];
	let notebook_header1 = NotebookHeader {
		version: 1,
		notary_id: 1,
		notebook_number: 1,
		tick: 0,
		changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![
			BalanceTip {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 2_000_000,
				change_number: 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
				channel_hold_note: None,
			}
			.encode(),
		]),
		chain_transfers: bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }],
		tax: 0,
		changed_account_origins: bounded_vec![AccountOrigin { notebook_number: 1, account_uid: 1 }],
		// Block Votes
		parent_secret: None,
		secret_hash: H256::from_slice(&[0u8; 32]),
		block_voting_power: 0,
		block_votes_root: H256::from_slice(&[0u8; 32]),
		block_votes_count: 0,
		blocks_with_votes: bounded_vec![],
		domains: bounded_vec![],
	};

	ValidLocalchainTransfers::mutate(|a| a.insert((Alice.to_account_id(), 1)));
	let mut notebook1 = Notebook {
		header: notebook_header1.clone(),
		notarizations: bounded_vec![Notarization::new(
			alice_balance_changeset.clone(),
			vec![],
			vec![]
		)],
		new_account_origins: bounded_vec![NewAccountOrigin::new(
			Alice.to_account_id(),
			AccountType::Deposit,
			1
		)],
		hash: H256::from_slice(&[0u8; 32]),
		signature: Ed25519Keyring::Alice.pair().sign(&notebook_header1.hash()[..]),
	};
	notebook1.hash = notebook1.calculate_hash();
	notebook1.signature = Ed25519Keyring::Alice.pair().sign(&notebook1.hash[..]);

	assert_err!(
		notebook_verify(&TestLookup, &notebook1, &notary_operator(), 2),
		VerifyError::DuplicateChainTransfer
	);
}

#[test]
fn test_multiple_changesets_in_a_notebook() {
	let alice_balance_changeset = vec![
		BalanceChange {
			balance: 0,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![
				Note::create(1_000_000, NoteType::ClaimFromMainchain { transfer_id: 1 }),
				Note::create(1_000_000, NoteType::Send { to: None }),
			],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone(),
		BalanceChange {
			balance: 800_000,
			change_number: 1,
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![
				Note::create(1_000_000, NoteType::Claim),
				Note::create(200_000, NoteType::Tax),
			],
			signature: empty_signature(),
		}
		.sign(Bob.pair())
		.clone(),
		BalanceChange {
			balance: 200_000,
			change_number: 1,
			account_id: Bob.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(200_000, NoteType::Claim),],
			signature: empty_signature(),
		}
		.sign(Bob.pair())
		.clone(),
	];

	ValidLocalchainTransfers::mutate(|a| a.insert((Alice.to_account_id(), 1)));
	// NOTE: this is in sorted order by account_id, account_type
	let mut balance_tips = BTreeMap::from([
		(
			(Alice.to_account_id(), AccountType::Deposit),
			BalanceTip {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 0,
				change_number: 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
				channel_hold_note: None,
			},
		),
		(
			(Bob.to_account_id(), AccountType::Deposit),
			BalanceTip {
				account_id: Bob.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 800_000,
				change_number: 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 2 },
				channel_hold_note: None,
			},
		),
		(
			(Bob.to_account_id(), AccountType::Tax),
			BalanceTip {
				account_id: Bob.to_account_id(),
				account_type: AccountType::Tax,
				balance: 200_000,
				change_number: 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 3 },
				channel_hold_note: None,
			},
		),
	]);

	let mut notebook = Notebook {
		header: NotebookHeader {
			version: 1,
			notary_id: 1,
			notebook_number: 1,
			tick: 0,
			tax: 200_000,
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(
				balance_tips.values().map(|v| v.encode()).collect::<Vec<_>>(),
			),
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }],
			changed_account_origins: bounded_vec![
				AccountOrigin { notebook_number: 1, account_uid: 1 },
				AccountOrigin { notebook_number: 1, account_uid: 2 },
				AccountOrigin { notebook_number: 1, account_uid: 3 }
			],
			// Block Votes
			parent_secret: None,
			secret_hash: H256::from_slice(&[0u8; 32]),
			block_voting_power: 0,
			block_votes_root: H256::from_slice(&[0u8; 32]),
			block_votes_count: 1,
			blocks_with_votes: bounded_vec![],
			domains: bounded_vec![],
		},
		notarizations: bounded_vec![
			Notarization::new(alice_balance_changeset.clone(), vec![], vec![]),
			Notarization::new(
				vec![],
				vec![BlockVote::create_default_vote(notary_operator(), 0)],
				vec![]
			)
		],
		new_account_origins: bounded_vec![
			NewAccountOrigin::new(Alice.to_account_id(), AccountType::Deposit, 1),
			NewAccountOrigin::new(Bob.to_account_id(), AccountType::Deposit, 2),
			NewAccountOrigin::new(Bob.to_account_id(), AccountType::Tax, 3)
		],
		hash: H256::from_slice(&[0u8; 32]),
		signature: ed25519::Signature::from_raw([0u8; 64]),
	};

	notebook.header.block_votes_root = block_votes_root(notebook.notarizations.to_vec());
	notebook.hash = notebook.calculate_hash();
	assert_ok!(notebook_verify(&TestLookup, &notebook, &notary_operator(), 2),);

	let changeset2 = vec![
		BalanceChange {
			balance: 0,
			change_number: 2,
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(800_000, NoteType::Send { to: None }),],
			signature: empty_signature(),
		}
		.sign(Bob.pair())
		.clone(),
		BalanceChange {
			balance: 600_000,
			change_number: 2,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![
				Note::create(800_000, NoteType::Claim),
				Note::create(200_000, NoteType::Tax),
			],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone(),
		BalanceChange {
			balance: 200_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(200_000, NoteType::Claim),],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone(),
	];
	notebook.header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(
		balance_tips.values().map(|v| v.encode()).collect::<Vec<_>>(),
	);
	notebook
		.notarizations
		.try_push(Notarization::new(changeset2.clone(), vec![], vec![]))
		.expect("should insert");
	if let Some(tip) = balance_tips.get_mut(&(Bob.to_account_id(), AccountType::Deposit)) {
		tip.change_number = 2;
		tip.balance = 0;
	}
	if let Some(tip) = balance_tips.get_mut(&(Alice.to_account_id(), AccountType::Deposit)) {
		tip.change_number = 2;
		tip.balance = 600_000;
	}
	balance_tips.insert(
		(Alice.to_account_id(), AccountType::Tax),
		BalanceTip {
			balance: 200_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Tax,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 4 },
			channel_hold_note: None,
		},
	);
	notebook
		.new_account_origins
		.try_push(NewAccountOrigin::new(Alice.to_account_id(), AccountType::Tax, 4))
		.expect("should insert");
	notebook.hash = notebook.calculate_hash();
	assert_err!(
		notebook_verify(&TestLookup, &notebook, &notary_operator(), 2),
		VerifyError::MissingBalanceProof
	);
	notebook.header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(
		balance_tips.values().map(|v| v.encode()).collect::<Vec<_>>(),
	);
	notebook.notarizations[2].balance_changes[0].previous_balance_proof = Some(BalanceProof {
		notary_id: 1,
		notebook_number: 1,
		tick: 1,
		notebook_proof: None,
		account_origin: AccountOrigin { notebook_number: 1, account_uid: 2 },
		balance: 800_000,
	});
	notebook.notarizations[2].balance_changes[1].previous_balance_proof = Some(BalanceProof {
		notary_id: 1,
		notebook_number: 1,
		tick: 1,
		notebook_proof: None,
		account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
		balance: 0,
	});
	notebook.notarizations[2].balance_changes[2].previous_balance_proof = Some(BalanceProof {
		notary_id: 1,
		notebook_number: 1,
		tick: 1,
		notebook_proof: None,
		account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
		balance: 0,
	});
	notebook.header.tax = 400_000;
	notebook.hash = notebook.calculate_hash();
	assert_err!(
		notebook_verify(&TestLookup, &notebook, &notary_operator(), 2),
		VerifyError::InvalidPreviousBalanceProof
	);
	notebook
		.header
		.changed_account_origins
		.try_push(AccountOrigin { notebook_number: 1, account_uid: 4 })
		.expect("should insert");

	notebook.notarizations[2].balance_changes[2].previous_balance_proof = None;
	notebook.hash = notebook.calculate_hash();
	assert_ok!(notebook_verify(&TestLookup, &notebook, &notary_operator(), 2),);
}

#[test]
fn test_cannot_remove_lock_between_changesets_in_a_notebook() {
	let alice_balance_changeset = vec![
		BalanceChange {
			balance: 1_000_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				1_000_000,
				NoteType::ClaimFromMainchain { transfer_id: 1 }
			),],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone(),
	];
	let alice_balance_changeset2 = vec![
		BalanceChange {
			balance: 1_000_000,
			change_number: 2,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: Some(BalanceProof {
				notary_id: 1,
				notebook_number: 1,
				tick: 1,
				notebook_proof: None,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
				balance: 1_000_000,
			}),
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				1_000_000,
				NoteType::ChannelHold {
					recipient: Bob.to_account_id(),
					delegated_signer: None,
					domain_hash: None
				}
			)],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone(),
	];

	ValidLocalchainTransfers::mutate(|a| a.insert((Alice.to_account_id(), 1)));
	let mut notebook = Notebook {
		header: NotebookHeader {
			version: 1,
			notary_id: 1,
			notebook_number: 1,
			tick: 0,
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![
				BalanceTip {
					account_id: Alice.to_account_id(),
					account_type: AccountType::Deposit,
					balance: 1_000_000,
					change_number: 2,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					channel_hold_note: None,
				}
				.encode(),
			]),
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain { transfer_id: 1 }],
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			tax: 0,
			// Block Votes
			parent_secret: None,
			secret_hash: H256::from_slice(&[0u8; 32]),
			block_voting_power: 0,
			block_votes_root: H256::from_slice(&[0u8; 32]),
			block_votes_count: 1,
			blocks_with_votes: bounded_vec![],
			domains: bounded_vec![],
		},
		notarizations: bounded_vec![
			Notarization::new(alice_balance_changeset.clone(), vec![], vec![]),
			Notarization::new(
				alice_balance_changeset2.clone(),
				vec![BlockVote::create_default_vote(notary_operator(), 0)],
				vec![]
			),
		],
		new_account_origins: bounded_vec![NewAccountOrigin::new(
			Alice.to_account_id(),
			AccountType::Deposit,
			1
		)],
		hash: H256::from_slice(&[0u8; 32]),
		signature: ed25519::Signature::from_raw([0u8; 64]),
	};
	notebook.header.block_votes_root = block_votes_root(notebook.notarizations.to_vec());
	notebook.hash = notebook.calculate_hash();

	// test that the change root records the hold note
	assert_err!(
		notebook_verify(&TestLookup, &notebook, &notary_operator(), 2),
		VerifyError::InvalidBalanceChangeRoot
	);

	let hold_note = notebook.notarizations[1].balance_changes[0].notes[0].clone();

	notebook.header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(vec![
		BalanceTip {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			balance: 1_000_000,
			change_number: 2,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			channel_hold_note: Some(hold_note),
		}
		.encode(),
	]);
	notebook.hash = notebook.calculate_hash();
	assert_ok!(notebook_verify(&TestLookup, &notebook, &notary_operator(), 2),);

	// now confirm we can't remove the hold in the same set of changes
	{
		// Try 1: pretend it didn't happen
		let alice_balance_changeset3 = vec![
			BalanceChange {
				balance: 1_000_000,
				change_number: 3,
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				previous_balance_proof: Some(BalanceProof {
					notary_id: 1,
					notebook_number: 1,
					tick: 1,
					notebook_proof: None,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					balance: 1_000_000,
				}),
				channel_hold_note: None,
				notes: bounded_vec![Note::create(
					1_000_000,
					NoteType::ChannelHold {
						recipient: Ferdie.to_account_id(),
						delegated_signer: None,
						domain_hash: None
					}
				)],
				signature: empty_signature(),
			}
			.sign(Alice.pair())
			.clone(),
		];
		let mut notebook = notebook.clone();
		let _ = notebook.notarizations.try_push(Notarization::new(
			alice_balance_changeset3,
			vec![],
			vec![],
		));
		let hold_note = notebook.notarizations[2].balance_changes[0].notes[0].clone();
		notebook.header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(vec![
			BalanceTip {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 1_000_000,
				change_number: 3,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
				channel_hold_note: Some(hold_note),
			}
			.encode(),
		]);
		assert_err!(
			notebook_verify(&TestLookup, &notebook, &notary_operator(), 2),
			VerifyError::InvalidChannelHoldNote
		);
	}
	{
		// Try 2: try to remove the hold
		let alice_balance_changeset3 = vec![
			BalanceChange {
				balance: 1_000_000,
				change_number: 3,
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				previous_balance_proof: Some(BalanceProof {
					notary_id: 1,
					notebook_number: 1,
					tick: 1,
					notebook_proof: None,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					balance: 1_000_000,
				}),
				channel_hold_note: Some(Note::create(
					1_000_000,
					NoteType::ChannelHold {
						recipient: Bob.to_account_id(),
						delegated_signer: None,
						domain_hash: None,
					},
				)),
				notes: bounded_vec![Note::create(0, NoteType::ChannelHoldSettle)],
				signature: empty_signature(),
			}
			.sign(Alice.pair())
			.clone(),
		];

		let mut notebook = notebook.clone();
		let _ = notebook.notarizations.try_push(Notarization::new(
			alice_balance_changeset3,
			vec![],
			vec![],
		));
		let hold_note = notebook.notarizations[2].balance_changes[0].notes[0].clone();

		notebook.header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(vec![
			BalanceTip {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 1_000_000,
				change_number: 3,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
				channel_hold_note: Some(hold_note),
			}
			.encode(),
		]);
		assert!(matches!(
			notebook_verify(&TestLookup, &notebook, &notary_operator(), 2),
			Err(VerifyError::ChannelHoldNotReadyForClaim { .. })
		),);
	}
}

#[test]
fn test_votes_must_add_up() {
	let notebook_1_tips = vec![
		BalanceTip {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Tax,
			channel_hold_note: None,
			balance: 1_000_000,
			change_number: 1,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
		},
		BalanceTip {
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			channel_hold_note: Some(Note::create(
				500_000,
				NoteType::ChannelHold {
					recipient: Alice.to_account_id(),
					delegated_signer: None,
					domain_hash: None,
				},
			)),
			balance: 500_000,
			change_number: 1,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 2 },
		},
		BalanceTip {
			account_id: Ferdie.to_account_id(),
			account_type: AccountType::Deposit,
			channel_hold_note: Some(Note::create(
				500_000,
				NoteType::ChannelHold {
					recipient: Alice.to_account_id(),
					delegated_signer: None,
					domain_hash: None,
				},
			)),
			balance: 500_000,
			change_number: 1,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 3 },
		},
	];

	let vote_block_hash = H256::random();
	let channel_hold_expiration_ticks: Tick = 2;
	let mut notebook = Notebook {
		header: NotebookHeader {
			version: 1,
			notary_id: 1,
			notebook_number: 62,
			tick: channel_hold_expiration_ticks + 1,
			changed_accounts_root: Default::default(),
			chain_transfers: Default::default(),
			changed_account_origins: Default::default(),
			tax: 0,
			// Block Votes
			parent_secret: None,
			secret_hash: H256::from_slice(&[0u8; 32]),
			block_voting_power: 10_000,
			block_votes_root: H256::from_slice(&[0u8; 32]),
			block_votes_count: 3,
			blocks_with_votes: bounded_vec![],
			domains: bounded_vec![],
		},
		notarizations: bounded_vec![Notarization::new(
			vec![
				BalanceChange {
					balance: 0,
					change_number: 2,
					account_id: Bob.to_account_id(),
					channel_hold_note: Some(Note::create(
						500_000,
						NoteType::ChannelHold {
							recipient: Alice.to_account_id(),
							delegated_signer: None,
							domain_hash: None
						}
					)),
					account_type: AccountType::Deposit,
					previous_balance_proof: Some(BalanceProof {
						notary_id: 1,
						notebook_number: 1,
						tick: 1,
						notebook_proof: Some(proof(notebook_1_tips.clone(), 1),),
						account_origin: AccountOrigin { notebook_number: 1, account_uid: 2 },
						balance: 500_000,
					}),
					signature: empty_signature(),
					notes: bounded_vec![Note::create(500_000, NoteType::ChannelHoldSettle)],
				}
				.sign(Bob.pair())
				.clone(),
				BalanceChange {
					balance: 0,
					change_number: 2,
					account_id: Ferdie.to_account_id(),
					channel_hold_note: Some(Note::create(
						500_000,
						NoteType::ChannelHold {
							recipient: Alice.to_account_id(),
							delegated_signer: None,
							domain_hash: None
						}
					)),
					account_type: AccountType::Deposit,
					previous_balance_proof: Some(BalanceProof {
						notary_id: 1,
						notebook_number: 1,
						tick: 1,
						notebook_proof: Some(proof(notebook_1_tips.clone(), 2),),
						account_origin: AccountOrigin { notebook_number: 1, account_uid: 3 },
						balance: 500_000,
					}),
					signature: empty_signature(),
					notes: bounded_vec![Note::create(500_000, NoteType::ChannelHoldSettle)],
				}
				.sign(Ferdie.pair())
				.clone(),
				BalanceChange {
					balance: 800_000,
					change_number: 1,
					account_id: Alice.to_account_id(),
					account_type: AccountType::Deposit,
					previous_balance_proof: None,
					channel_hold_note: None,
					notes: bounded_vec![
						Note::create(200_000, NoteType::Tax),
						Note::create(1_000_000, NoteType::ChannelHoldClaim),
					],
					signature: empty_signature(),
				}
				.sign(Alice.pair())
				.clone(),
				BalanceChange {
					balance: 1_000_000 - 34_000 + 200_000,
					change_number: 2,
					account_id: Alice.to_account_id(),
					account_type: AccountType::Tax,
					previous_balance_proof: Some(BalanceProof {
						notary_id: 1,
						notebook_number: 1,
						tick: 1,
						notebook_proof: Some(proof(notebook_1_tips.clone(), 0),),
						account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
						balance: 1_000_000,
					}),
					channel_hold_note: None,
					notes: bounded_vec![
						Note::create(200_000, NoteType::Claim),
						Note::create(34_000, NoteType::SendToVote),
					],
					signature: empty_signature(),
				}
				.sign(Alice.pair())
				.clone(),
			],
			vec![
				BlockVote {
					index: 0,
					tick: channel_hold_expiration_ticks + 1,
					power: 4_000,
					block_hash: vote_block_hash,
					account_id: Alice.to_account_id(),
					block_rewards_account_id: Alice.to_account_id(),
					signature: Signature::from_raw([0u8; 64]).into(),
				}
				.sign(Alice.pair())
				.clone(),
				BlockVote {
					index: 1,
					tick: channel_hold_expiration_ticks + 1,
					power: 30_000,
					block_hash: vote_block_hash,
					account_id: Alice.to_account_id(),
					block_rewards_account_id: Alice.to_account_id(),
					signature: Signature::from_raw([0u8; 64]).into(),
				}
				.sign(Bob.pair())
				.clone(),
			],
			vec![]
		),],
		new_account_origins: bounded_vec![NewAccountOrigin::new(
			Alice.to_account_id(),
			AccountType::Deposit,
			1
		)],
		hash: H256::from_slice(&[0u8; 32]),
		signature: ed25519::Signature::from_raw([0u8; 64]),
	};

	notebook.header.tax = 200_000;
	notebook.header.changed_account_origins = bounded_vec![
		AccountOrigin { notebook_number: 1, account_uid: 1 },
		AccountOrigin { notebook_number: 1, account_uid: 2 },
		AccountOrigin { notebook_number: 1, account_uid: 3 },
		AccountOrigin { notebook_number: 62, account_uid: 1 }
	];
	notebook.header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(
		BTreeMap::from_iter(vec![
			(
				(Alice.to_account_id(), AccountType::Tax),
				BalanceTip {
					account_id: Alice.to_account_id(),
					account_type: AccountType::Tax,
					balance: 1_000_000 - 34_000 + 200_000,
					change_number: 2,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					channel_hold_note: None,
				},
			),
			(
				(Alice.to_account_id(), AccountType::Deposit),
				BalanceTip {
					account_id: Alice.to_account_id(),
					account_type: AccountType::Deposit,
					balance: 800_000,
					change_number: 1,
					account_origin: AccountOrigin { notebook_number: 62, account_uid: 1 },
					channel_hold_note: None,
				},
			),
			(
				(Bob.to_account_id(), AccountType::Deposit),
				BalanceTip {
					account_id: Bob.to_account_id(),
					account_type: AccountType::Deposit,
					balance: 0,
					change_number: 2,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 2 },
					channel_hold_note: None,
				},
			),
			(
				(Ferdie.to_account_id(), AccountType::Deposit),
				BalanceTip {
					account_id: Ferdie.to_account_id(),
					account_type: AccountType::Deposit,
					balance: 0,
					change_number: 2,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 3 },
					channel_hold_note: None,
				},
			),
		])
		.iter()
		.map(|v| v.1.encode())
		.collect::<Vec<_>>(),
	);
	notebook.hash = notebook.calculate_hash();

	LastChangedNotebook::mutate(|a| {
		a.insert(AccountOrigin { account_uid: 1, notebook_number: 1 }, 1);
		a.insert(AccountOrigin { account_uid: 2, notebook_number: 1 }, 1);
		a.insert(AccountOrigin { account_uid: 3, notebook_number: 1 }, 1);
	});
	NotebookRoots::mutate(|a| {
		a.insert(
			1,
			merkle_root::<Blake2Hasher, _>(
				notebook_1_tips.iter().map(|v| v.encode()).collect::<Vec<_>>(),
			),
		)
	});

	// One of the votes had the wrong signature
	assert_err!(
		notebook_verify(&TestLookup, &notebook, &notary_operator(), 2,),
		VerifyError::BlockVoteInvalidSignature
	);
	notebook.notarizations[0].block_votes[1].sign(Alice.pair());

	// Once vote minimums are allowed, the "vote root is wrong"
	assert_err!(
		notebook_verify(&TestLookup, &notebook, &notary_operator(), 2,),
		VerifyError::InvalidBlockVoteRoot
	);
	notebook.header.block_votes_root = block_votes_root(notebook.notarizations.to_vec());

	// The votes must add up
	assert_err!(
		notebook_verify(&TestLookup, &notebook, &notary_operator(), 2,),
		VerifyError::InvalidBlockVotesCount
	);
	notebook.header.block_votes_count = 2;

	// The summed voting power must also add up
	assert_err!(
		notebook_verify(&TestLookup, &notebook, &notary_operator(), 2,),
		VerifyError::InvalidBlockVotingPower
	);
	notebook.header.block_voting_power =
		notebook.notarizations[0].block_votes.iter().fold(0, |acc, v| acc + v.power);

	// The list of blocks voted on must match the list of votes
	assert_err!(
		notebook_verify(&TestLookup, &notebook, &notary_operator(), 2,),
		VerifyError::InvalidBlockVoteList
	);
	notebook.header.blocks_with_votes = bounded_vec![vote_block_hash];

	notebook.hash = notebook.calculate_hash();
	assert_ok!(notebook_verify(&TestLookup, &notebook, &notary_operator(), 2,),);
}

fn proof(leaves: Vec<BalanceTip>, index: usize) -> MerkleProof {
	let leaves = leaves.iter().map(|v| v.encode()).collect::<Vec<_>>();
	let proof = merkle_proof::<Blake2Hasher, _, _>(leaves, index as u32);
	MerkleProof {
		proof: BoundedVec::truncate_from(proof.proof),
		leaf_index: proof.leaf_index as u32,
		number_of_leaves: proof.number_of_leaves as u32,
	}
}

fn notary_operator() -> AccountId {
	AccountId32::new([0u8; 32])
}

fn block_votes_root(notarizations: Vec<Notarization>) -> H256 {
	let mut votes = vec![];
	for notarization in notarizations {
		votes.extend(notarization.block_votes.iter().cloned());
	}
	merkle_root::<BlakeTwo256, _>(votes.iter().map(|v| v.encode()).collect::<Vec<_>>())
}
