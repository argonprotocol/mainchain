use frame_support::{assert_err, assert_ok};
use sp_core::{bounded_vec, sr25519::Signature, H256};
use sp_keyring::{
	Ed25519Keyring::{Dave, Ferdie},
	Sr25519Keyring::{Alice, Bob},
};
use sp_runtime::MultiSignature;
use std::collections::{BTreeMap, BTreeSet};

use ulx_notary_primitives::{
	balance_change::{AccountOrigin, BalanceChange, BalanceProof},
	note::{AccountType, Note, NoteType},
	BlockVote, ChannelPass,
};

use crate::{
	verify_changeset_signatures, verify_notarization_allocation, verify_voting_sources,
	BalanceChangesetState, VerifyError,
};

fn empty_proof(balance: u128) -> Option<BalanceProof> {
	Some(BalanceProof {
		notary_id: 1,
		notebook_number: 1,
		balance,
		notebook_proof: Default::default(),
		account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
	})
}

fn empty_signature() -> MultiSignature {
	Signature([0u8; 64]).into()
}

#[test]
fn test_balance_change_allocation_errs_non_zero() {
	assert_err!(
		verify_notarization_allocation(
			&vec![BalanceChange {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				balance: 100,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![Note::create(100, NoteType::Claim)],
				signature: empty_signature(),
			}],
			&vec![],
			None
		),
		VerifyError::BalanceChangeNotNetZero { sent: 0, claimed: 100 }
	);
}

#[test]
fn must_supply_zero_balance_on_first_nonce() {
	let balance_change = vec![BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 2,
		balance: 100,
		previous_balance_proof: None,
		channel_hold_note: None,
		notes: Default::default(),
		signature: empty_signature(),
	}];

	assert_err!(
		verify_notarization_allocation(&balance_change, &vec![], None),
		VerifyError::MissingBalanceProof
	);
}

#[test]
fn test_balance_change_allocation_must_be_zero() {
	let balance_change = vec![
		BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 2,
			balance: 0,
			previous_balance_proof: empty_proof(100),
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				100,
				NoteType::Send { to: Some(bounded_vec!(Alice.to_account_id())) }
			)],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 100,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(100, NoteType::Claim)],
			signature: empty_signature(),
		},
	];

	assert_ok!(verify_notarization_allocation(&balance_change, &vec![], None));
}

#[test]
fn test_notes_must_add_up() {
	let mut balance_change = vec![
		BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 2,
			balance: 0,
			previous_balance_proof: empty_proof(250),
			channel_hold_note: None,
			notes: bounded_vec![Note::create(250, NoteType::Send { to: None })],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 100,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(100, NoteType::Claim)],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Dave.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 100, // WRONG BALANCE - should be 150
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(150, NoteType::Claim)],
			signature: empty_signature(),
		},
	];
	assert_err!(
		verify_notarization_allocation(&balance_change, &vec![], None),
		VerifyError::BalanceChangeMismatch {
			change_index: 2,
			provided_balance: 100,
			calculated_balance: 150
		}
	);

	balance_change[2].balance = 150;
	assert_ok!(verify_notarization_allocation(&balance_change, &vec![], None));
}

#[test]
fn test_recipients() {
	let mut balance_change = vec![
		BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 2,

			balance: 0,
			previous_balance_proof: empty_proof(250),
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				250,
				NoteType::Send {
					to: Some(bounded_vec!(Alice.to_account_id(), Ferdie.to_account_id()))
				}
			)],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 200,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(200, NoteType::Claim)],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Dave.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 50,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(50, NoteType::Claim)],
			signature: empty_signature(),
		},
	];
	assert_err!(
		verify_notarization_allocation(&balance_change, &vec![], None),
		VerifyError::InvalidNoteRecipients
	);

	balance_change[1].balance = 250;
	balance_change[1].notes[0].milligons = 250;
	balance_change.pop();
	assert_ok!(verify_notarization_allocation(&balance_change, &vec![], None));
}

#[test]
fn test_sending_to_localchain() {
	let balance_change = vec![BalanceChange {
		// We look for an transfer to localchain using this id
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 1,
		balance: 250,
		previous_balance_proof: None,
		channel_hold_note: None,
		notes: bounded_vec![Note {
			milligons: 250,
			note_type: NoteType::ClaimFromMainchain { account_nonce: 1 },
		}],
		signature: empty_signature(),
	}];

	assert_ok!(verify_notarization_allocation(&balance_change, &vec![], None),);
}

#[test]
fn test_sending_to_mainchain() {
	// This probably never happens - but in this scenario, funds are sent to a localchain to
	// transfer to a different mainchain account
	let balance_change = vec![
		BalanceChange {
			// We look for an transfer to localchain using this id
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 2,
			balance: 100,
			previous_balance_proof: empty_proof(50),
			channel_hold_note: None,
			notes: bounded_vec![
				Note::create(250, NoteType::ClaimFromMainchain { account_nonce: 15 }),
				Note::create(200, NoteType::Send { to: None }),
			],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 50,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![
				Note::create(200, NoteType::Claim),
				Note::create(150, NoteType::SendToMainchain),
			],
			signature: empty_signature(),
		},
	];

	assert_ok!(verify_notarization_allocation(&balance_change, &vec![], None));
}

#[test]
fn test_can_lock_with_a_channel_note() -> anyhow::Result<()> {
	let channel_note =
		Note::create(250, NoteType::ChannelHold { recipient: Alice.to_account_id() });
	let balance_change = BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 2,
		balance: 250,
		previous_balance_proof: empty_proof(250),
		channel_hold_note: None,
		notes: bounded_vec![channel_note.clone()],
		signature: empty_signature(),
	};
	{
		let res = verify_notarization_allocation(&vec![balance_change], &vec![], Some(1))
			.expect("should be ok");
		assert_eq!(res.needs_channel_settle_followup, false);
		assert_eq!(res.unclaimed_channel_balances.len(), 0);
		assert_eq!(res.sent_deposits, 0);
		assert_ok!(res.verify_taxes());
	}

	assert_err!(
		verify_notarization_allocation(
			&vec![BalanceChange {
				account_id: Bob.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 3,
				balance: 250,
				previous_balance_proof: empty_proof(250),
				channel_hold_note: Some(channel_note.clone()),
				notes: bounded_vec![Note::create(250, NoteType::Send { to: None })],
				signature: empty_signature(),
			}],
			&vec![],
			Some(2)
		),
		VerifyError::AccountLocked
	);

	let mut channel_settle = BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 3,
		balance: 200,
		previous_balance_proof: empty_proof(250),
		channel_hold_note: Some(channel_note.clone()),
		notes: bounded_vec![Note::create(
			50,
			NoteType::ChannelSettle { channel_pass_hash: H256::zero() }
		)],
		signature: empty_signature(),
	};

	assert_err!(
		verify_notarization_allocation(&vec![channel_settle.clone()], &vec![], Some(2)),
		VerifyError::ChannelHoldNotReadyForClaim
	);

	// try to clear out change
	channel_settle.balance = 250;
	channel_settle.notes[0].milligons = 0;

	// it won't let you claim your own note back
	assert_err!(
		verify_notarization_allocation(&vec![channel_settle.clone()], &vec![], Some(61)),
		VerifyError::InvalidChannelClaimers
	);

	// it WILL let you claim back your own note if it's past the grace period
	{
		let res = verify_notarization_allocation(&vec![channel_settle.clone()], &vec![], Some(71))
			.expect("should be ok");
		assert_eq!(res.needs_channel_settle_followup, false);
		assert_eq!(res.unclaimed_channel_balances.len(), 0);
		assert_eq!(res.sent_deposits, 0);
		assert_ok!(res.verify_taxes());
	}

	let changes = vec![
		BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 3,
			balance: 200,
			previous_balance_proof: empty_proof(250),
			channel_hold_note: Some(channel_note.clone()),
			notes: bounded_vec![Note::create(
				50,
				NoteType::ChannelSettle { channel_pass_hash: H256::zero() }
			)],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 50,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(50, NoteType::ChannelClaim)],
			signature: empty_signature(),
		},
	];

	assert_eq!(
		verify_notarization_allocation(&changes, &vec![], None)?.needs_channel_settle_followup,
		true
	);
	// a valid claim is also acceptable
	{
		let res =
			verify_notarization_allocation(&changes, &vec![], Some(61)).expect("should be ok");
		assert_eq!(res.needs_channel_settle_followup, false);
		assert_eq!(res.unclaimed_channel_balances.len(), 0);
		assert_eq!(res.claimed_channel_deposits_per_account.len(), 1);
		assert_eq!(res.claimed_channel_deposits_per_account.get(&Alice.to_account_id()), Some(&50));
		assert_err!(
			res.verify_taxes(),
			VerifyError::InsufficientTaxIncluded {
				account_id: Alice.to_account_id(),
				tax_sent: 0,
				tax_owed: 10
			}
		);
	}

	Ok(())
}

#[test]
fn test_change_signature() {
	let mut balance_change = vec![BalanceChange {
		// We look for an transfer to localchain using this id
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 1,
		balance: 250,
		previous_balance_proof: None,
		channel_hold_note: None,
		notes: bounded_vec![Note::create(250, NoteType::ClaimFromMainchain { account_nonce: 1 }),],
		signature: empty_signature(),
	}];

	assert_err!(
		verify_changeset_signatures(&balance_change),
		VerifyError::InvalidBalanceChangeSignature { change_index: 0 }
	);

	balance_change[0].sign(Bob.pair());
	assert_ok!(verify_changeset_signatures(&balance_change));
}
#[test]
fn test_with_note_claim_signatures() {
	let mut balance_change = BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 5,
		previous_balance_proof: empty_proof(250),
		balance: 250,
		notes: bounded_vec![],
		channel_hold_note: None,
		signature: empty_signature(),
	};
	balance_change.push_note(250, NoteType::ChannelHold { recipient: Alice.to_account_id() });
	balance_change.sign(Bob.pair());

	assert_ok!(verify_changeset_signatures(&vec![balance_change.clone()]));

	let mut balance_change2 = BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 6,
		previous_balance_proof: empty_proof(250),
		balance: 200,
		notes: bounded_vec![],
		channel_hold_note: Some(balance_change.notes[0].clone()),
		signature: empty_signature(),
	};
	balance_change2.push_note(50, NoteType::ChannelSettle { channel_pass_hash: H256::zero() });
	balance_change2.sign(Bob.pair());
	assert_ok!(verify_changeset_signatures(&vec![balance_change2.clone()]));

	let mut channel_note = balance_change.notes[0].clone();

	balance_change2.sign(Bob.pair());
	balance_change2.channel_hold_note = Some(Note::create(100, NoteType::Tax));
	assert_err!(
		verify_changeset_signatures(&vec![balance_change2.clone()]),
		VerifyError::InvalidChannelHoldNote
	);

	channel_note.milligons = 102;
	balance_change2.channel_hold_note = Some(channel_note.clone());
	assert_err!(
		verify_changeset_signatures(&vec![balance_change2.clone()]),
		VerifyError::InvalidBalanceChangeSignature { change_index: 0 }
	);
}

#[test]
fn test_tax_must_be_claimed_on_tax_account() {
	let set = vec![
		BalanceChange {
			balance: 20_000,
			change_number: 2,
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: empty_proof(21_000),
			channel_hold_note: None,
			notes: Default::default(),
			signature: empty_signature(),
		}
		.push_note(1000, NoteType::Send { to: None })
		.sign(Bob.pair())
		.clone(),
		BalanceChange {
			balance: 800,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			notes: Default::default(),
			signature: empty_signature(),
			channel_hold_note: None,
		}
		.push_note(1000, NoteType::Claim)
		.push_note(200, NoteType::Tax)
		.sign(Alice.pair())
		.clone(),
	];

	assert_err!(
		verify_notarization_allocation(&set, &vec![], Some(1)),
		VerifyError::TaxBalanceChangeNotNetZero { sent: 200, claimed: 0 }
	);

	let mut claim_tax_on_deposit = set.clone();
	claim_tax_on_deposit.push(
		BalanceChange {
			balance: 200,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			notes: Default::default(),
			signature: empty_signature(),
			channel_hold_note: None,
		}
		.push_note(200, NoteType::Claim)
		.clone(),
	);
	assert_err!(
		verify_notarization_allocation(&claim_tax_on_deposit, &vec![], Some(1)),
		VerifyError::BalanceChangeNotNetZero { sent: 1000, claimed: 1200 }
	);

	let mut claim_tax_on_deposit = set.clone();
	claim_tax_on_deposit.push(
		BalanceChange {
			balance: 200,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: None,
			notes: Default::default(),
			signature: empty_signature(),
			channel_hold_note: None,
		}
		.push_note(200, NoteType::Claim)
		.clone(),
	);

	let result = verify_notarization_allocation(&claim_tax_on_deposit, &vec![], Some(1))
		.expect("should unwrap");
	assert_eq!(result.claimed_deposits, 1000);
	assert_eq!(result.sent_tax, 200);
	assert_eq!(result.claimed_tax, 200);
}

#[test]
fn test_can_transfer_tax() {
	let set = vec![BalanceChange {
		balance: 20_000,
		change_number: 1,
		account_id: Bob.to_account_id(),
		account_type: AccountType::Tax,
		previous_balance_proof: None,
		channel_hold_note: None,
		notes: bounded_vec!(Note::create(
			20_000,
			NoteType::ClaimFromMainchain { account_nonce: 1 }
		)),
		signature: empty_signature(),
	}];

	assert_err!(
		verify_notarization_allocation(&set, &vec![], Some(1)),
		VerifyError::InvalidTaxOperation
	);

	let set = vec![
		BalanceChange {
			balance: 0,
			change_number: 2,
			account_id: Bob.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: empty_proof(20_000),
			channel_hold_note: None,
			notes: bounded_vec!(Note::create(
				20_000,
				NoteType::Send {
					to: Some(bounded_vec!(Alice.to_account_id(), Ferdie.to_account_id()))
				}
			)),
			signature: empty_signature(),
		},
		BalanceChange {
			balance: 9_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec!(Note::create(9_000, NoteType::Claim)),
			signature: empty_signature(),
		},
		BalanceChange {
			balance: 11_000,
			change_number: 1,
			account_id: Ferdie.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec!(Note::create(11_000, NoteType::Claim)),
			signature: empty_signature(),
		},
	];

	let result = verify_notarization_allocation(&set, &vec![], Some(1)).expect("should unwrap");

	assert_eq!(result.claimed_deposits, 0);
	assert_eq!(result.claimed_tax, 20_000);
	assert_eq!(result.sent_tax, 20_000);
	assert_ok!(result.verify_taxes());
}

#[test]
fn verify_taxes() {
	let mut set = BalanceChangesetState::default();
	assert_ok!(set.verify_taxes());

	set.claimed_deposits_per_account.insert(Alice.to_account_id(), 100);
	assert_err!(
		set.verify_taxes(),
		VerifyError::InsufficientTaxIncluded {
			account_id: Alice.to_account_id(),
			tax_sent: 0,
			tax_owed: 20
		}
	);
	set.tax_created_per_account.insert(Alice.to_account_id(), 22);
	assert_ok!(set.verify_taxes());

	set.claimed_channel_deposits_per_account.insert(Alice.to_account_id(), 1000);
	assert_err!(
		set.verify_taxes(),
		VerifyError::InsufficientTaxIncluded {
			account_id: Alice.to_account_id(),
			tax_sent: 22,
			tax_owed: 220
		}
	);
}

#[test]
fn verify_tax_votes() {
	let set = vec![BalanceChange {
		balance: 0,
		change_number: 2,
		account_id: Bob.to_account_id(),
		account_type: AccountType::Tax,
		previous_balance_proof: empty_proof(20_000),
		channel_hold_note: None,
		notes: bounded_vec!(Note::create(20_000, NoteType::SendToVote)),
		signature: empty_signature(),
	}];

	assert_err!(
		verify_notarization_allocation(&set, &vec![], Some(1)),
		VerifyError::InvalidBlockVoteAllocation
	);

	let votes = vec![BlockVote {
		account_id: Bob.to_account_id(),
		grandparent_block_hash: H256::zero(),
		index: 0,
		power: 20_000,

		channel_pass: ChannelPass {
			id: 1,
			zone_record_hash: H256::zero(),
			miner_index: 0,
			at_block_height: 100,
		},
	}];

	let result = verify_notarization_allocation(&set, &votes, Some(1)).expect("should unwrap");

	assert_eq!(result.claimed_deposits, 0);
	assert_eq!(result.unclaimed_block_vote_tax_per_account.len(), 0);
}

#[test]
fn test_vote_sources() {
	let grandparent_block_hash = H256::from([1u8; 32]);
	let channel_pass_1 = ChannelPass {
		id: 1,
		zone_record_hash: H256::from([3u8; 32]),
		miner_index: 0,
		at_block_height: 100,
	};

	let mut votes = vec![
		BlockVote {
			account_id: Bob.to_account_id(),
			grandparent_block_hash,
			index: 0,
			power: 20_000,
			channel_pass: channel_pass_1.clone(),
		},
		BlockVote {
			account_id: Bob.to_account_id(),
			grandparent_block_hash,
			index: 1,
			power: 400,
			channel_pass: channel_pass_1.clone(),
		},
	];

	let vote_minimums = BTreeMap::from([(grandparent_block_hash, 500)]);

	assert_err!(
		verify_voting_sources(&BTreeSet::new(), &votes, &vote_minimums),
		VerifyError::InvalidBlockVoteChannelPass
	);

	assert_err!(
		verify_voting_sources(&BTreeSet::from([channel_pass_1.hash()]), &votes, &vote_minimums),
		VerifyError::InsufficientBlockVoteMinimum
	);

	votes[1].power = 500;

	assert_err!(
		verify_voting_sources(&BTreeSet::new(), &votes, &vote_minimums),
		VerifyError::InvalidBlockVoteChannelPass
	);

	votes.truncate(1);

	assert_ok!(verify_voting_sources(
		&BTreeSet::from([channel_pass_1.hash()]),
		&votes,
		&vote_minimums
	),);
}
