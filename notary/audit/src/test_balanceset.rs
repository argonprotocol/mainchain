use std::collections::BTreeMap;

use frame_support::{assert_err, assert_ok};
use sp_core::{bounded_vec, sr25519::Signature, H256};
use sp_keyring::{
	AccountKeyring::Charlie,
	Ed25519Keyring::{Dave, Ferdie},
	Sr25519Keyring::{Alice, Bob},
};

use argon_primitives::{
	balance_change::{AccountOrigin, BalanceChange, BalanceProof},
	note::{Note, NoteType},
	AccountType, BlockVote, LocalchainAccountId, MultiSignatureBytes, CHANNEL_HOLD_CLAWBACK_TICKS,
};

use crate::{
	track_block_votes, verify_changeset_signatures, verify_notarization_allocation,
	verify_voting_sources, BalanceChangesetState, NotebookVerifyState, VerifyError,
};

fn empty_proof(balance: u128) -> Option<BalanceProof> {
	Some(BalanceProof {
		notary_id: 1,
		notebook_number: 1,
		tick: 1,
		balance,
		notebook_proof: Default::default(),
		account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
	})
}

fn empty_signature() -> MultiSignatureBytes {
	Signature::from_raw([0u8; 64]).into()
}

#[test]
fn test_balance_change_allocation_errs_non_zero() {
	assert_err!(
		verify_notarization_allocation(
			&vec![BalanceChange {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				balance: 100_000,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![Note::create(100_000, NoteType::Claim)],
				signature: empty_signature(),
			}
			.sign(Alice.pair())
			.clone()],
			&[],
			&[],
			None,
			2
		),
		VerifyError::BalanceChangeNotNetZero { sent: 0, claimed: 100_000 }
	);
}

#[test]
fn must_supply_zero_balance_on_first_nonce() {
	let balance_change = vec![BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 2,
		balance: 100_000,
		previous_balance_proof: None,
		channel_hold_note: None,
		notes: Default::default(),
		signature: empty_signature(),
	}];

	assert_err!(
		verify_notarization_allocation(&balance_change, &[], &[], None, 2),
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
			previous_balance_proof: empty_proof(100_000),
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				100_000,
				NoteType::Send { to: Some(bounded_vec!(Alice.to_account_id())) }
			)],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 100_000,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(100_000, NoteType::Claim)],
			signature: empty_signature(),
		},
	];

	assert_ok!(verify_notarization_allocation(&balance_change, &[], &[], None, 2));
}

#[test]
fn test_notes_must_add_up() {
	let mut balance_change = vec![
		BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 2,
			balance: 0,
			previous_balance_proof: empty_proof(250_000),
			channel_hold_note: None,
			notes: bounded_vec![Note::create(250_000, NoteType::Send { to: None })],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 100_000,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(100_000, NoteType::Claim)],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Dave.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 100_000, // WRONG BALANCE - should be 150_000
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(150_000, NoteType::Claim)],
			signature: empty_signature(),
		},
	];
	assert_err!(
		verify_notarization_allocation(&balance_change, &[], &[], None, 2),
		VerifyError::BalanceChangeMismatch {
			change_index: 2,
			provided_balance: 100_000,
			calculated_balance: 150_000
		}
	);

	balance_change[2].balance = 150_000;
	assert_ok!(verify_notarization_allocation(&balance_change, &[], &[], None, 2));
}

#[test]
fn test_recipients() {
	let mut balance_change = vec![
		BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 2,

			balance: 0,
			previous_balance_proof: empty_proof(250_000),
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				250_000,
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
			balance: 200_000,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(200_000, NoteType::Claim)],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Dave.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 50_000,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(50_000, NoteType::Claim)],
			signature: empty_signature(),
		},
	];
	assert_err!(
		verify_notarization_allocation(&balance_change, &[], &[], None, 2),
		VerifyError::InvalidNoteRecipients
	);

	balance_change[1].balance = 250_000;
	balance_change[1].notes[0].microgons = 250_000;
	balance_change.pop();
	assert_ok!(verify_notarization_allocation(&balance_change, &[], &[], None, 2));
}

#[test]
fn test_sending_to_localchain() {
	let balance_change = vec![BalanceChange {
		// We look for an transfer to localchain using this id
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 1,
		balance: 250_000,
		previous_balance_proof: None,
		channel_hold_note: None,
		notes: bounded_vec![Note {
			microgons: 250_000,
			note_type: NoteType::ClaimFromMainchain { transfer_id: 1 },
		}],
		signature: empty_signature(),
	}];

	assert_ok!(verify_notarization_allocation(&balance_change, &[], &[], None, 2),);
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
			balance: 100_000,
			previous_balance_proof: empty_proof(50_000),
			channel_hold_note: None,
			notes: bounded_vec![
				Note::create(250_000, NoteType::ClaimFromMainchain { transfer_id: 15 }),
				Note::create(200_000, NoteType::Send { to: None }),
			],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 50_000,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![
				Note::create(200_000, NoteType::Claim),
				Note::create(150_000, NoteType::SendToMainchain),
			],
			signature: empty_signature(),
		},
	];

	assert_ok!(verify_notarization_allocation(&balance_change, &[], &[], None, 2));
}

#[test]
fn test_can_lock_with_a_channel_hold_note() -> anyhow::Result<()> {
	let channel_hold_note = Note::create(
		250_000,
		NoteType::ChannelHold {
			recipient: Alice.to_account_id(),
			delegated_signer: None,
			domain_hash: None,
		},
	);
	let balance_change = BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 2,
		balance: 250_000,
		previous_balance_proof: empty_proof(250_000),
		channel_hold_note: None,
		notes: bounded_vec![channel_hold_note.clone()],
		signature: empty_signature(),
	};
	{
		let res = verify_notarization_allocation(&vec![balance_change], &[], &[], Some(1), 2)
			.expect("should be ok");
		assert!(!res.needs_channel_hold_settle_followup);
		assert_eq!(res.unclaimed_channel_hold_balances.len(), 0);
		assert_eq!(res.sent_deposits, 0);
		assert_ok!(res.verify_taxes());
	}

	assert_err!(
		verify_notarization_allocation(
			&vec![BalanceChange {
				account_id: Bob.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 3,
				balance: 250_000,
				previous_balance_proof: empty_proof(250_000),
				channel_hold_note: Some(channel_hold_note.clone()),
				notes: bounded_vec![Note::create(250_000, NoteType::Send { to: None })],
				signature: empty_signature(),
			}],
			&[],
			&[],
			Some(2),
			2
		),
		VerifyError::AccountLocked
	);

	let mut channel_hold_settle = BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 3,
		balance: 200_000,
		previous_balance_proof: empty_proof(250_000),
		channel_hold_note: Some(channel_hold_note.clone()),
		notes: bounded_vec![Note::create(50_000, NoteType::ChannelHoldSettle)],
		signature: empty_signature(),
	};

	assert!(matches!(
		verify_notarization_allocation(&vec![channel_hold_settle.clone()], &[], &[], Some(2), 2),
		Err(VerifyError::ChannelHoldNotReadyForClaim { .. })
	));

	// try to clear out change
	channel_hold_settle.balance = 250_000;
	channel_hold_settle.notes[0].microgons = 0;

	let proof_tick = channel_hold_settle.clone().previous_balance_proof.unwrap().tick;

	let channel_hold_expiration_ticks = 2;

	// it won't let you claim your own note back before the clawback period
	assert_err!(
		verify_notarization_allocation(
			&vec![channel_hold_settle.clone()],
			&[],
			&[],
			Some(channel_hold_expiration_ticks + proof_tick),
			channel_hold_expiration_ticks
		),
		VerifyError::InvalidChannelHoldClaimers
	);

	// it WILL let you claim back your own note if it's past the grace period
	{
		let res = verify_notarization_allocation(
			&vec![channel_hold_settle.clone()],
			&[],
			&[],
			Some(1 + CHANNEL_HOLD_CLAWBACK_TICKS + channel_hold_expiration_ticks),
			2,
		)
		.expect("should be ok");
		assert!(!res.needs_channel_hold_settle_followup);
		assert_eq!(res.unclaimed_channel_hold_balances.len(), 0);
		assert_eq!(res.sent_deposits, 0);
		assert_ok!(res.verify_taxes());
	}

	let changes = vec![
		BalanceChange {
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 3,
			balance: 200_000,
			previous_balance_proof: empty_proof(250_000),
			channel_hold_note: Some(channel_hold_note.clone()),
			notes: bounded_vec![Note::create(50_000, NoteType::ChannelHoldSettle)],
			signature: empty_signature(),
		},
		BalanceChange {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 50_000,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(50_000, NoteType::ChannelHoldClaim)],
			signature: empty_signature(),
		},
	];

	assert!(
		verify_notarization_allocation(&changes, &[], &[], None, 2)?
			.needs_channel_hold_settle_followup
	);
	// a valid claim is also acceptable
	{
		let res =
			verify_notarization_allocation(&changes, &[], &[], Some(61), 2).expect("should be ok");
		assert!(!res.needs_channel_hold_settle_followup);
		assert_eq!(res.unclaimed_channel_hold_balances.len(), 0);
		assert_eq!(res.claimed_channel_hold_deposits_per_account.len(), 1);
		assert_eq!(
			res.claimed_channel_hold_deposits_per_account
				.get(&LocalchainAccountId::new(Alice.to_account_id(), AccountType::Deposit)),
			Some(&50_000)
		);
		assert_err!(
			res.verify_taxes(),
			VerifyError::InsufficientTaxIncluded {
				account_id: Alice.to_account_id(),
				tax_sent: 0,
				tax_owed: 10_000
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
		balance: 250_000,
		previous_balance_proof: None,
		channel_hold_note: None,
		notes: bounded_vec![Note::create(250_000, NoteType::ClaimFromMainchain { transfer_id: 1 }),],
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
		previous_balance_proof: empty_proof(250_000),
		balance: 250_000,
		notes: bounded_vec![],
		channel_hold_note: None,
		signature: empty_signature(),
	};
	balance_change.push_note(
		250_000,
		NoteType::ChannelHold {
			recipient: Alice.to_account_id(),
			delegated_signer: None,
			domain_hash: None,
		},
	);
	balance_change.sign(Bob.pair());

	assert_ok!(verify_changeset_signatures(&vec![balance_change.clone()]));

	let mut balance_change2 = BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 6,
		previous_balance_proof: empty_proof(250_000),
		balance: 200_000,
		notes: bounded_vec![],
		channel_hold_note: Some(balance_change.notes[0].clone()),
		signature: empty_signature(),
	};
	balance_change2.push_note(50_000, NoteType::ChannelHoldSettle);
	balance_change2.sign(Bob.pair());
	assert_ok!(verify_changeset_signatures(&vec![balance_change2.clone()]));

	let mut channel_hold_note = balance_change.notes[0].clone();

	balance_change2.sign(Bob.pair());
	balance_change2.channel_hold_note = Some(Note::create(100_000, NoteType::Tax));
	assert_err!(
		verify_changeset_signatures(&vec![balance_change2.clone()]),
		VerifyError::InvalidChannelHoldNote
	);

	channel_hold_note.microgons = 102;
	balance_change2.channel_hold_note = Some(channel_hold_note.clone());
	assert_err!(
		verify_changeset_signatures(&vec![balance_change2.clone()]),
		VerifyError::InvalidBalanceChangeSignature { change_index: 0 }
	);
}

#[test]
fn test_with_delegated_note_claim_signatures() {
	let mut balance_change = BalanceChange {
		account_id: Bob.to_account_id(),
		account_type: AccountType::Deposit,
		change_number: 5,
		previous_balance_proof: empty_proof(250_000),
		balance: 0,
		notes: bounded_vec![Note::create(250_000, NoteType::ChannelHoldSettle,)],
		channel_hold_note: Some(Note::create(
			250_000,
			NoteType::ChannelHold {
				recipient: Alice.to_account_id(),
				delegated_signer: Some(Charlie.to_account_id()),
				domain_hash: None,
			},
		)),
		signature: empty_signature(),
	};
	balance_change.sign(Bob.pair());

	assert_ok!(verify_changeset_signatures(&vec![balance_change.clone()]));
	balance_change.sign(Charlie.pair());
	assert_ok!(verify_changeset_signatures(&vec![balance_change.clone()]));
}

#[test]
fn test_tax_must_be_claimed_on_tax_account() {
	let set = vec![
		BalanceChange {
			balance: 20_000_000,
			change_number: 2,
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: empty_proof(21_000_000),
			channel_hold_note: None,
			notes: Default::default(),
			signature: empty_signature(),
		}
		.push_note(1_000_000, NoteType::Send { to: None })
		.sign(Bob.pair())
		.clone(),
		BalanceChange {
			balance: 800_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			notes: Default::default(),
			signature: empty_signature(),
			channel_hold_note: None,
		}
		.push_note(1_000_000, NoteType::Claim)
		.push_note(200_000, NoteType::Tax)
		.sign(Alice.pair())
		.clone(),
	];

	assert_err!(
		verify_notarization_allocation(&set, &[], &[], Some(1), 2),
		VerifyError::TaxBalanceChangeNotNetZero { sent: 200_000, claimed: 0 }
	);

	let mut claim_tax_on_deposit = set.clone();
	claim_tax_on_deposit.push(
		BalanceChange {
			balance: 200_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			notes: Default::default(),
			signature: empty_signature(),
			channel_hold_note: None,
		}
		.push_note(200_000, NoteType::Claim)
		.clone(),
	);
	assert_err!(
		verify_notarization_allocation(&claim_tax_on_deposit, &[], &[], Some(1), 2),
		VerifyError::BalanceChangeNotNetZero { sent: 1_000_000, claimed: 1_200_000 }
	);

	let mut claim_tax_on_deposit = set.clone();
	claim_tax_on_deposit.push(
		BalanceChange {
			balance: 200_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: None,
			notes: Default::default(),
			signature: empty_signature(),
			channel_hold_note: None,
		}
		.push_note(200_000, NoteType::Claim)
		.clone(),
	);

	let result = verify_notarization_allocation(&claim_tax_on_deposit, &[], &[], Some(1), 2)
		.expect("should unwrap");
	assert_eq!(result.claimed_deposits, 1_000_000);
	assert_eq!(result.sent_tax, 200_000);
	assert_eq!(result.claimed_tax, 200_000);
}

#[test]
fn test_can_transfer_tax() {
	let set = vec![BalanceChange {
		balance: 20_000_000,
		change_number: 1,
		account_id: Bob.to_account_id(),
		account_type: AccountType::Tax,
		previous_balance_proof: None,
		channel_hold_note: None,
		notes: bounded_vec!(Note::create(
			20_000_000,
			NoteType::ClaimFromMainchain { transfer_id: 1 }
		)),
		signature: empty_signature(),
	}];

	assert_err!(
		verify_notarization_allocation(&set, &[], &[], Some(1), 2),
		VerifyError::InvalidTaxOperation
	);

	let set = vec![
		BalanceChange {
			balance: 0,
			change_number: 2,
			account_id: Bob.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: empty_proof(20_000_000),
			channel_hold_note: None,
			notes: bounded_vec!(Note::create(
				20_000_000,
				NoteType::Send {
					to: Some(bounded_vec!(Alice.to_account_id(), Ferdie.to_account_id()))
				}
			)),
			signature: empty_signature(),
		},
		BalanceChange {
			balance: 9_000_000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec!(Note::create(9_000_000, NoteType::Claim)),
			signature: empty_signature(),
		},
		BalanceChange {
			balance: 11_000_000,
			change_number: 1,
			account_id: Ferdie.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec!(Note::create(11_000_000, NoteType::Claim)),
			signature: empty_signature(),
		},
	];

	let result = verify_notarization_allocation(&set, &[], &[], Some(1), 2).expect("should unwrap");

	assert_eq!(result.claimed_deposits, 0);
	assert_eq!(result.claimed_tax, 20_000_000);
	assert_eq!(result.sent_tax, 20_000_000);
	assert_ok!(result.verify_taxes());
}

#[test]
fn test_can_buy_domains() {
	let set = vec![
		BalanceChange {
			balance: 19_000_000,
			change_number: 2,
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: empty_proof(20_000_000),
			channel_hold_note: None,
			notes: bounded_vec![Note::create(1_000_000, NoteType::LeaseDomain),],
			signature: empty_signature(),
		},
		BalanceChange {
			balance: 1_000_000,
			change_number: 1,
			account_id: Bob.to_account_id(),
			account_type: AccountType::Tax,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(1_000_000, NoteType::Claim)],
			signature: empty_signature(),
		},
	];

	let result = verify_notarization_allocation(
		&set,
		&[],
		&[(H256::random(), Alice.to_account_id())],
		Some(1),
		2,
	)
	.expect("should unwrap");

	assert_eq!(result.claimed_deposits, 0);
	assert_eq!(result.claimed_tax, 1_000_000);
	assert_eq!(result.sent_tax, 1_000_000);
	assert_eq!(result.allocated_to_domains, 1_000_000);
	assert_ok!(result.verify_taxes());
}

#[test]
fn verify_taxes() {
	let mut set = BalanceChangesetState::default();
	assert_ok!(set.verify_taxes());
	let localchain_account_id =
		LocalchainAccountId::new(Alice.to_account_id(), AccountType::Deposit);

	set.claims_per_account.insert(localchain_account_id.clone(), 100_000);
	assert_err!(
		set.verify_taxes(),
		VerifyError::InsufficientTaxIncluded {
			account_id: Alice.to_account_id(),
			tax_sent: 0,
			tax_owed: 20_000
		}
	);
	set.tax_created_per_account.insert(localchain_account_id.clone(), 22_000);
	assert_ok!(set.verify_taxes());

	set.claimed_channel_hold_deposits_per_account
		.insert(localchain_account_id.clone(), 1_000_000);
	assert_err!(
		set.verify_taxes(),
		VerifyError::InsufficientTaxIncluded {
			account_id: Alice.to_account_id(),
			tax_sent: 22_000,
			tax_owed: 220_000
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
		previous_balance_proof: empty_proof(20_000_000),
		channel_hold_note: None,
		notes: bounded_vec!(Note::create(20_000_000, NoteType::SendToVote)),
		signature: Signature::from_raw([0u8; 64]).into(),
	}];

	assert_err!(
		verify_notarization_allocation(&set, &[], &[], Some(1), 2),
		VerifyError::InvalidBlockVoteAllocation
	);

	let votes = vec![BlockVote {
		account_id: Bob.to_account_id(),
		block_hash: H256::zero(),
		index: 0,
		power: 20_000_000,
		tick: 1,
		block_rewards_account_id: Bob.to_account_id(),
		signature: Signature::from_raw([0u8; 64]).into(),
	}
	.sign(Bob.pair())
	.clone()];

	let result =
		verify_notarization_allocation(&set, &votes, &[], Some(1), 2).expect("should unwrap");

	assert_eq!(result.claimed_deposits, 0);
	assert_eq!(result.unclaimed_block_vote_tax_per_account.len(), 0);
}

#[test]
fn test_vote_sources() {
	let vote_block_hash = H256::from([1u8; 32]);

	let mut votes = vec![
		BlockVote {
			account_id: Bob.to_account_id(),
			block_hash: vote_block_hash,
			index: 0,
			tick: 1,
			power: 20_000_000,
			block_rewards_account_id: Bob.to_account_id(),
			signature: Signature::from_raw([0u8; 64]).into(),
		}
		.sign(Bob.pair())
		.clone(),
		BlockVote {
			account_id: Bob.to_account_id(),
			block_hash: vote_block_hash,
			index: 1,
			tick: 1,
			power: 400_000,
			block_rewards_account_id: Bob.to_account_id(),
			signature: Signature::from_raw([0u8; 64]).into(),
		}
		.sign(Alice.pair())
		.clone(),
	];

	let vote_minimums = BTreeMap::from([(vote_block_hash, 500_000)]);

	let operator = Ferdie.to_account_id();
	assert_err!(
		verify_voting_sources(&votes, 1, &operator, &vote_minimums),
		VerifyError::InsufficientBlockVoteMinimum
	);

	votes[1].power = 500_000;
	assert_err!(
		verify_voting_sources(&votes, 1, &operator, &vote_minimums),
		VerifyError::BlockVoteInvalidSignature
	);
	votes[1].sign(Bob.pair());

	assert_err!(
		verify_voting_sources(&votes, 2, &operator, &vote_minimums),
		VerifyError::InvalidBlockVoteTick { tick: 1, notebook_tick: 2 }
	);
	assert_ok!(verify_voting_sources(&votes, 1, &operator, &vote_minimums),);
}

#[test]
fn test_default_votes() {
	let operator = Ferdie.to_account_id();
	let mut votes = vec![BlockVote::create_default_vote(Bob.to_account_id(), 10)];

	let vote_minimums = BTreeMap::from([(H256::zero(), 500_000)]);

	assert_err!(
		verify_voting_sources(&votes, 1, &operator, &vote_minimums),
		VerifyError::InvalidBlockVoteTick { tick: 10, notebook_tick: 1 }
	);
	assert_err!(
		verify_voting_sources(&votes, 10, &operator, &vote_minimums),
		VerifyError::InvalidDefaultBlockVoteAuthor {
			author: Bob.to_account_id(),
			expected: operator.clone()
		}
	);

	votes[0].account_id = operator.clone();
	assert_ok!(verify_voting_sources(&votes, 10, &operator, &vote_minimums),);
	// if there's any power, not a default vote anymore
	votes[0].power = 1;
	assert_err!(
		verify_voting_sources(&votes, 10, &operator, &vote_minimums),
		VerifyError::InsufficientBlockVoteMinimum
	);

	// can't have more than one default vote
	let mut notebook_state = NotebookVerifyState::default();
	assert_err!(
		track_block_votes(
			&mut notebook_state,
			&vec![
				BlockVote::create_default_vote(operator.clone(), 10),
				BlockVote::create_default_vote(operator.clone(), 10)
			],
		),
		VerifyError::InvalidDefaultBlockVote
	);
}
