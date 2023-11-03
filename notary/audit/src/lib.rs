#![cfg_attr(not(feature = "std"), no_std)]

use binary_merkle_tree::{merkle_root, verify_proof, Leaf};
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{
	scale_info::TypeInfo,
	traits::{BlakeTwo256, Verify},
	RuntimeString,
};
use sp_std::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	vec::Vec,
};

use ulx_notary_primitives::{
	ensure, AccountId, AccountOrigin, AccountOriginUid, AccountType, BalanceChange, BalanceProof,
	BalanceTip, ChainTransfer, NewAccountOrigin, NotaryId, NoteId, NoteType, Notebook,
	NotebookNumber,
};

#[derive(Debug, PartialEq, Clone, Snafu, TypeInfo, Encode, Decode, Serialize, Deserialize)]
pub enum VerifyError {
	#[snafu(display("Missing account origin {account_id:?}, {account_type:?}"))]
	MissingAccountOrigin { account_id: AccountId32, account_type: AccountType },
	#[snafu(display("Account history lookup error {source}"))]
	HistoryLookupError {
		#[snafu(source(from(AccountHistoryLookupError, AccountHistoryLookupError::from)))]
		source: AccountHistoryLookupError,
	},
	#[snafu(display("Invalid account changelist"))]
	InvalidAccountChangelist,
	#[snafu(display("Invalid chain transfers list"))]
	InvalidChainTransfersList,
	#[snafu(display("Invalid balance change root"))]
	InvalidBalanceChangeRoot,

	#[snafu(display("Invalid previous nonce"))]
	InvalidPreviousNonce,
	#[snafu(display("Invalid previous balance"))]
	InvalidPreviousBalance,
	#[snafu(display("Invalid previous account origin"))]
	InvalidPreviousAccountOrigin,

	#[snafu(display("Invalid previous balance change notebook"))]
	InvalidPreviousBalanceChangeNotebook,

	#[snafu(display("Invalid net balance change calculated"))]
	InvalidBalanceChange,

	#[snafu(display("Invalid note signature"))]
	InvalidNoteSignature,
	#[snafu(display("Invalid note id calculated"))]
	InvalidNoteIdCalculated,

	#[snafu(display("A claimed note id is not in the balance changeset"))]
	NoteIdNotInBalanceChanges { note_id: NoteId },

	#[snafu(display("Invalid note recipient for a claimed note"))]
	InvalidNoteRecipient,

	#[snafu(display("The note id was already used"))]
	NoteIdAlreadyUsed,

	#[snafu(display(
		"An invalid balance change was submitted ({change_index}.{note_index}): {message:?}"
	))]
	BalanceChangeError { change_index: u16, note_index: u16, message: RuntimeString },

	#[snafu(display("Invalid net balance changeset. Must account for all funds."))]
	InvalidNetBalanceChangeset,

	#[snafu(display("Insufficient balance for account  (balance: {balance}, amount: {amount}) (change: {change_index}.{note_index})"))]
	InsufficientBalance { balance: u128, amount: u128, note_index: u16, change_index: u16 },

	#[snafu(display("Exceeded max balance for account (pre-balance: {balance}, amount: {amount}), (change: {change_index}.{note_index})"))]
	ExceededMaxBalance { balance: u128, amount: u128, note_index: u16, change_index: u16 },
	#[snafu(display("Balance change mismatch (provided_balance: {provided_balance}, calculated_balance: {calculated_balance}) (#{change_index})"))]
	BalanceChangeMismatch { change_index: u16, provided_balance: u128, calculated_balance: i128 },

	#[snafu(display("Balance change not net zero (unaccounted: {unaccounted})"))]
	BalanceChangeNotNetZero { unaccounted: i128 },

	#[snafu(display("Must include proof of previous balance"))]
	MissingBalanceProof,
	#[snafu(display("Invalid previous balance proof"))]
	InvalidPreviousBalanceProof,
	#[snafu(display("Invalid notebook hash"))]
	InvalidNotebookHash,

	#[snafu(display("Duplicate chain transfer"))]
	DuplicateChainTransfer,

	#[snafu(display("Duplicated account origin uid"))]
	DuplicatedAccountOriginUid,

	#[snafu(display("Invalid notary signature"))]
	InvalidNotarySignature,

	#[snafu(display("Submitted notebook older than most recent in storage"))]
	NotebookTooOld,

	#[snafu(display("Error decoding notebook"))]
	DecodeError,
}

impl From<AccountHistoryLookupError> for VerifyError {
	fn from(e: AccountHistoryLookupError) -> Self {
		VerifyError::HistoryLookupError { source: e }
	}
}

#[derive(Debug, Clone, PartialEq, TypeInfo, Encode, Decode, Serialize, Deserialize, Snafu)]
pub enum AccountHistoryLookupError {
	#[snafu(display("Notebook root not found"))]
	RootNotFound,
	#[snafu(display("Last change not found"))]
	LastChangeNotFound,
	#[snafu(display("Invalid transfer to localchain"))]
	InvalidTransferToLocalchain,
}

pub trait NotebookHistoryLookup {
	fn get_account_changes_root(
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
	) -> Result<H256, AccountHistoryLookupError>;
	fn get_last_changed_notebook(
		notary_id: NotaryId,
		account_origin: AccountOrigin,
	) -> Result<NotebookNumber, AccountHistoryLookupError>;

	fn is_valid_transfer_to_localchain(
		notary_id: NotaryId,
		account_id: &AccountId32,
		nonce: u32,
	) -> Result<bool, AccountHistoryLookupError>;
}

pub fn notebook_verify<'a, T: NotebookHistoryLookup>(
	header_hash: &'a H256,
	notebook: &'a Notebook,
) -> anyhow::Result<bool, VerifyError> {
	let mut account_changelist = BTreeSet::<AccountOrigin>::new();
	let mut final_balances = BTreeMap::<(AccountId32, AccountType), BalanceTip>::new();
	let mut chain_transfers = Vec::<ChainTransfer>::new();
	let mut seen_transfers_in = BTreeSet::<(NotaryId, AccountId32, u32)>::new();

	let mut all_new_account_uids = BTreeSet::new();

	let Notebook { header, balance_changes, new_account_origins: flat_account_origins } = notebook;

	let mut new_account_origins = BTreeMap::<(AccountId32, AccountType), AccountOriginUid>::new();
	for NewAccountOrigin { account_id, account_type, account_uid: uid } in flat_account_origins {
		new_account_origins.insert((account_id.clone(), account_type.clone()), *uid);
		ensure!(all_new_account_uids.insert(*uid), VerifyError::DuplicatedAccountOriginUid);
	}

	for changeset in balance_changes.iter() {
		verify_balance_changeset_allocation(&changeset)?;
		verify_changeset_signatures(&changeset)?;

		for change in changeset.into_iter() {
			for note in &change.notes {
				// if this note is a chain transfer, track it in chain_transfers
				match &note.note_type {
					NoteType::SendToMainchain => {
						chain_transfers.push(ChainTransfer::ToMainchain {
							amount: note.milligons,
							account_id: change.account_id.clone(),
						});
					},
					NoteType::ClaimFromMainchain { account_nonce: nonce } => {
						T::is_valid_transfer_to_localchain(
							header.notary_id,
							&change.account_id,
							*nonce,
						)?;

						ensure!(
							seen_transfers_in.insert((
								header.notary_id,
								change.account_id.clone(),
								*nonce,
							)),
							VerifyError::DuplicateChainTransfer
						);

						chain_transfers.push(ChainTransfer::ToLocalchain {
							account_id: change.account_id.clone(),
							account_nonce: nonce.clone(),
						});
					},
					_ => {},
				}
			}

			let key = (change.account_id.clone(), change.account_type.clone());

			if change.change_number == 1 {
				if let Some(account_uid) = new_account_origins.get(&key) {
					let account_origin = AccountOrigin {
						notebook_number: header.notebook_number,
						account_uid: *account_uid,
					};
					account_changelist.insert(account_origin.clone());

					final_balances.insert(
						key.clone(),
						BalanceTip {
							account_id: change.account_id.clone(),
							account_type: change.account_type.clone(),
							balance: change.balance,
							change_number: change.change_number,
							account_origin,
						},
					);
				} else {
					return Err(VerifyError::MissingAccountOrigin {
						account_id: change.account_id.clone(),
						account_type: change.account_type.clone(),
					})
				}
			} else {
				let proof = change
					.previous_balance_proof
					.as_ref()
					.expect("Should have been unwrapped in verify_balance_changeset_allocation");
				verify_previous_balance_proof::<T>(proof, &mut final_balances, &change, &key)?;

				account_changelist.insert(proof.account_origin.clone());

				final_balances.insert(
					key.clone(),
					BalanceTip {
						account_id: change.account_id.clone(),
						account_type: change.account_type.clone(),
						balance: change.balance.clone(),
						change_number: change.change_number.clone(),
						account_origin: proof.account_origin.clone(),
					},
				);
			}
		}
	}

	ensure!(
		chain_transfers == header.chain_transfers.clone().into_iter().collect::<Vec<_>>(),
		VerifyError::InvalidChainTransfersList
	);
	ensure!(
		BTreeSet::from_iter(header.changed_account_origins.clone().into_iter()) ==
			account_changelist,
		VerifyError::InvalidAccountChangelist
	);

	let merkle_leafs = final_balances.into_iter().map(|(_, v)| v.encode()).collect::<Vec<_>>();

	let merkle_root = merkle_root::<BlakeTwo256, _>(merkle_leafs);

	ensure!(merkle_root == header.changed_accounts_root, VerifyError::InvalidBalanceChangeRoot);

	ensure!(*header_hash == header.hash(), VerifyError::InvalidNotebookHash);

	Ok(true)
}

fn verify_previous_balance_proof<'a, T: NotebookHistoryLookup>(
	proof: &BalanceProof,
	final_balances: &mut BTreeMap<(AccountId32, AccountType), BalanceTip>,
	change: &BalanceChange,
	key: &(AccountId32, AccountType),
) -> anyhow::Result<bool, VerifyError> {
	// if we've changed balance in this notebook before, it must match the previous
	// entry
	if final_balances.contains_key(&key) {
		let previous_balance = final_balances.get(&key).unwrap();
		ensure!(
			previous_balance.balance == change.previous_balance,
			VerifyError::InvalidPreviousBalance
		);
		ensure!(
			previous_balance.change_number == change.change_number - 1,
			VerifyError::InvalidPreviousNonce
		);
		ensure!(
			previous_balance.account_origin == proof.account_origin,
			VerifyError::InvalidPreviousAccountOrigin
		);
	} else {
		let last_notebook_change =
			T::get_last_changed_notebook(proof.notary_id, proof.account_origin.clone())?;
		ensure!(
			last_notebook_change == proof.notebook_number,
			VerifyError::InvalidPreviousBalanceChangeNotebook
		);

		let root = T::get_account_changes_root(proof.notary_id, proof.notebook_number)?;
		let leaf = BalanceTip {
			account_id: change.account_id.clone(),
			account_type: change.account_type.clone(),
			balance: change.previous_balance,
			change_number: change.change_number - 1,
			account_origin: proof.account_origin.clone(),
		};

		ensure!(
			verify_proof::<'_, BlakeTwo256, _, _>(
				&root,
				proof.proof.clone().into_inner(),
				proof.number_of_leaves as usize,
				proof.leaf_index as usize,
				Leaf::Value(&leaf.encode()),
			),
			VerifyError::InvalidPreviousBalanceProof
		);
	}
	Ok(true)
}

pub fn verify_changeset_signatures(
	changeset: &Vec<BalanceChange>,
) -> anyhow::Result<(), VerifyError> {
	// Since this is a little more expensive, confirm signatures in a second pass
	for change in changeset {
		let mut index = 0;
		for note in &change.notes {
			ensure!(
				note.get_note_id(
					&change.account_id,
					&change.account_type,
					change.change_number,
					index
				) == note.note_id,
				VerifyError::InvalidNoteIdCalculated
			);
			ensure!(
				note.signature.verify(&note.note_id[..], &change.account_id),
				VerifyError::InvalidNoteSignature
			);
			index += 1;
		}
	}
	Ok(())
}

/// This function verifies the proposed balance changes prior to accessing storage or verifying
/// proofs
/// 1. Confirm each proposed balance change adds up properly
/// 2. Confirm the changes net out to 0 (no funds are left outside an account)
pub fn verify_balance_changeset_allocation(
	changes: &Vec<BalanceChange>,
) -> anyhow::Result<(), VerifyError> {
	let mut transferred_balances: i128 = 0i128;
	let mut change_index = 0;
	let mut new_accounts = BTreeSet::<(AccountId32, AccountType)>::new();

	let mut used_note_ids = BTreeSet::<NoteId>::new();
	let mut restricted_balance = BTreeMap::<AccountId, i128>::new();

	for change in changes {
		ensure!(change.change_number > 0, VerifyError::InvalidBalanceChange);
		if change.change_number == 1 {
			new_accounts.insert((change.account_id.clone(), change.account_type.clone()));

			ensure!(
				change.previous_balance_proof.is_none(),
				VerifyError::InvalidPreviousBalanceProof
			);
			ensure!(change.previous_balance == 0, VerifyError::InvalidPreviousBalance);
		}
		if change.change_number > 1 &&
			!new_accounts.contains(&(change.account_id.clone(), change.account_type.clone()))
		{
			ensure!(change.previous_balance_proof.is_some(), VerifyError::MissingBalanceProof);
		}
		let mut balance = change.previous_balance as i128;
		let mut note_index = 0;
		for note in &change.notes {
			if used_note_ids.contains(&note.note_id) {
				return Err(VerifyError::NoteIdAlreadyUsed)
			}
			used_note_ids.insert(note.note_id.clone());

			match &note.note_type {
				NoteType::Send { recipient } => {
					transferred_balances += note.milligons as i128;
					if let Some(recipient) = recipient {
						restricted_balance.insert(recipient.clone(), note.milligons as i128);
					}
				},
				NoteType::Claim => {
					transferred_balances -= note.milligons as i128;
					if restricted_balance.len() > 0 {
						let restricted_amount = restricted_balance.remove(&change.account_id);
						let restricted_amount = restricted_amount.unwrap_or_default();
						ensure!(
							restricted_amount >= note.milligons as i128,
							VerifyError::InvalidNoteRecipient
						);
						let restricted_change =
							restricted_amount.saturating_sub(note.milligons as i128);
						if restricted_change > 0 {
							restricted_balance.insert(change.account_id.clone(), restricted_change);
						}
					}
				},
				_ => {},
			}

			match note.note_type {
				NoteType::ClaimFromMainchain { .. } | NoteType::Claim { .. } =>
					if let Some(new_balance) = balance.checked_add(note.milligons as i128) {
						balance = new_balance;
					} else {
						return Err(VerifyError::ExceededMaxBalance {
							balance: balance as u128,
							amount: note.milligons,
							note_index,
							change_index,
						})
					},
				NoteType::SendToMainchain | NoteType::Send { .. } =>
					balance -= note.milligons as i128,
				_ => {},
			};
			note_index += 1;
		}

		ensure!(
			balance as u128 == change.balance,
			VerifyError::BalanceChangeMismatch {
				change_index,
				provided_balance: change.balance,
				calculated_balance: balance,
			}
		);
		change_index += 1;
	}

	ensure!(
		transferred_balances == 0,
		VerifyError::BalanceChangeNotNetZero { unaccounted: transferred_balances }
	);
	Ok(())
}
#[cfg(test)]
mod tests {
	use std::collections::{BTreeMap, BTreeSet};

	use binary_merkle_tree::{merkle_proof, merkle_root};
	use chrono::Utc;
	use codec::Encode;
	use frame_support::{assert_err, assert_ok, parameter_types};
	use sp_core::{
		bounded::BoundedVec, bounded_vec, crypto::AccountId32, sr25519::Signature, Blake2Hasher,
		H256,
	};
	use sp_keyring::{
		AccountKeyring,
		AccountKeyring::{Alice, Bob},
		Ed25519Keyring::Dave,
	};
	use sp_runtime::MultiSignature;

	use ulx_notary_primitives::{
		balance_change::{AccountOrigin, BalanceChange, BalanceProof},
		note::{AccountType, Note, NoteType},
		BalanceTip, ChainTransfer, NewAccountOrigin, Notebook, NotebookHeader, NotebookNumber,
	};

	use crate::{
		verify_previous_balance_proof, AccountHistoryLookupError, NotebookHistoryLookup,
		VerifyError,
	};

	#[test]
	fn test_balance_change_allocation_errs_non_zero() {
		let balance_change = vec![BalanceChange {
			account_id: AccountKeyring::Alice.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			previous_balance: 0,
			balance: 100,
			previous_balance_proof: None,
			notes: bounded_vec![Note {
				milligons: 100,
				note_type: NoteType::Claim,
				signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
				note_id: Default::default(),
			}],
		}];

		assert_err!(
			super::verify_balance_changeset_allocation(&balance_change),
			VerifyError::BalanceChangeNotNetZero { unaccounted: -100 }
		);
	}

	#[test]
	fn must_supply_zero_balance_on_first_nonce() {
		let mut balance_change = vec![
			BalanceChange {
				account_id: AccountKeyring::Bob.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				previous_balance: 100, // should flag as invalid since nonce is 1
				balance: 0,
				previous_balance_proof: None,
				notes: bounded_vec![Note {
					milligons: 100,
					note_type: NoteType::Send { recipient: None },
					signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
					note_id: Default::default(),
				}],
			},
			BalanceChange {
				account_id: AccountKeyring::Alice.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				previous_balance: 0,
				balance: 100,
				previous_balance_proof: None,
				notes: bounded_vec![Note {
					milligons: 100,
					note_type: NoteType::Claim,
					signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
					note_id: Default::default(),
				}],
			},
		];

		assert_err!(
			super::verify_balance_changeset_allocation(&balance_change),
			VerifyError::InvalidPreviousBalance
		);

		// now that we have history, you need to supply proof
		balance_change[0].change_number = 2;
		assert_err!(
			super::verify_balance_changeset_allocation(&balance_change),
			VerifyError::MissingBalanceProof
		);
	}

	#[test]
	fn test_balance_change_allocation_must_be_zero() {
		let balance_change = vec![
			BalanceChange {
				account_id: AccountKeyring::Bob.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 2,
				previous_balance: 100,
				balance: 0,
				previous_balance_proof: Some(BalanceProof {
					notary_id: 0,
					notebook_number: 0,
					proof: bounded_vec![],
					leaf_index: 0,
					number_of_leaves: 0,
					account_origin: AccountOrigin { notebook_number: 0, account_uid: 1 },
				}),
				notes: bounded_vec![Note {
					milligons: 100,
					note_type: NoteType::Send {
						recipient: Some(AccountKeyring::Alice.to_account_id())
					},
					signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
					note_id: H256([0u8; 32]),
				}],
			},
			BalanceChange {
				account_id: AccountKeyring::Alice.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				previous_balance: 0,
				balance: 100,
				previous_balance_proof: None,
				notes: bounded_vec![Note {
					milligons: 100,
					note_type: NoteType::Claim,
					signature: MultiSignature::Sr25519(Signature::from_slice(&[1u8; 64]).unwrap()),
					note_id: H256([1u8; 32]),
				}],
			},
		];

		assert_ok!(super::verify_balance_changeset_allocation(&balance_change));
	}

	#[test]
	fn test_notes_cannot_be_reused() {
		let balance_change = vec![
			BalanceChange {
				account_id: AccountKeyring::Bob.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 2,
				previous_balance: 200,
				balance: 0,
				previous_balance_proof: Some(BalanceProof {
					notary_id: 0,
					notebook_number: 0,
					proof: bounded_vec![],
					leaf_index: 0,
					number_of_leaves: 0,
					account_origin: AccountOrigin { notebook_number: 0, account_uid: 1 },
				}),
				notes: bounded_vec![
					Note {
						milligons: 100,
						note_type: NoteType::Send { recipient: None },
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[0u8; 64]).unwrap()
						),
						note_id: H256([0u8; 32]),
					},
					// We sneak in a copy of the signed note.
					Note {
						milligons: 100,
						note_type: NoteType::Send { recipient: None },
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[0u8; 64]).unwrap()
						),
						note_id: H256([0u8; 32]),
					}
				],
			},
			BalanceChange {
				account_id: AccountKeyring::Alice.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				previous_balance: 0,
				balance: 100,
				previous_balance_proof: None,
				notes: bounded_vec![Note {
					milligons: 200,
					note_type: NoteType::Claim,
					signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
					note_id: H256([1u8; 32]),
				},],
			},
		];
		assert_err!(
			super::verify_balance_changeset_allocation(&balance_change),
			VerifyError::NoteIdAlreadyUsed
		);
	}
	#[test]
	fn test_notes_must_add_up() {
		let mut balance_change = vec![
			BalanceChange {
				account_id: AccountKeyring::Bob.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 2,
				previous_balance: 250,
				balance: 0,
				previous_balance_proof: Some(BalanceProof {
					notary_id: 0,
					notebook_number: 0,
					proof: bounded_vec![],
					leaf_index: 0,
					number_of_leaves: 0,
					account_origin: AccountOrigin { notebook_number: 0, account_uid: 1 },
				}),
				notes: bounded_vec![Note {
					milligons: 250,
					note_type: NoteType::Send { recipient: None },
					signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
					note_id: H256([0u8; 32]),
				}],
			},
			BalanceChange {
				account_id: AccountKeyring::Alice.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				previous_balance: 0,
				balance: 100,
				previous_balance_proof: None,
				notes: bounded_vec![Note {
					milligons: 100,
					note_type: NoteType::Claim,
					signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
					note_id: H256([1u8; 32]),
				}],
			},
			BalanceChange {
				account_id: AccountKeyring::Dave.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				previous_balance: 0,
				balance: 100, // WRONG BALANCE - should be 150
				previous_balance_proof: None,
				notes: bounded_vec![Note {
					milligons: 150,
					note_type: NoteType::Claim,
					signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
					note_id: H256([2u8; 32]),
				}],
			},
		];
		assert_err!(
			super::verify_balance_changeset_allocation(&balance_change),
			VerifyError::BalanceChangeMismatch {
				change_index: 2,
				provided_balance: 100,
				calculated_balance: 150
			}
		);

		balance_change[2].balance = 150;
		assert_ok!(super::verify_balance_changeset_allocation(&balance_change));
	}

	#[test]
	fn test_recipients() {
		let mut balance_change = vec![
			BalanceChange {
				account_id: AccountKeyring::Bob.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 2,
				previous_balance: 250,
				balance: 0,
				previous_balance_proof: Some(BalanceProof {
					notary_id: 0,
					notebook_number: 0,
					proof: bounded_vec![],
					leaf_index: 0,
					number_of_leaves: 0,
					account_origin: AccountOrigin { notebook_number: 0, account_uid: 1 },
				}),
				notes: bounded_vec![Note {
					milligons: 250,
					note_type: NoteType::Send { recipient: Some(Alice.to_account_id()) },
					signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
					note_id: H256([0u8; 32]),
				}],
			},
			BalanceChange {
				account_id: AccountKeyring::Alice.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				previous_balance: 0,
				balance: 200,
				previous_balance_proof: None,
				notes: bounded_vec![Note {
					milligons: 200,
					note_type: NoteType::Claim,
					signature: MultiSignature::Sr25519(Signature::from_slice(&[1u8; 64]).unwrap()),
					note_id: H256([1u8; 32]),
				}],
			},
			BalanceChange {
				account_id: AccountKeyring::Dave.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				previous_balance: 0,
				balance: 50,
				previous_balance_proof: None,
				notes: bounded_vec![Note {
					milligons: 50,
					note_type: NoteType::Claim,
					signature: MultiSignature::Sr25519(Signature::from_slice(&[2u8; 64]).unwrap()),
					note_id: H256([2u8; 32]),
				}],
			},
		];
		assert_err!(
			super::verify_balance_changeset_allocation(&balance_change),
			VerifyError::InvalidNoteRecipient
		);

		balance_change[1].balance = 250;
		balance_change[1].notes[0].milligons = 250;
		balance_change.pop();
		assert_ok!(super::verify_balance_changeset_allocation(&balance_change));
	}
	#[test]
	fn test_sending_to_localchain() {
		let balance_change = vec![BalanceChange {
			// We look for an transfer to localchain using this id
			account_id: AccountKeyring::Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			previous_balance: 0,
			balance: 250,
			previous_balance_proof: None,
			notes: bounded_vec![Note {
				milligons: 250,
				note_type: NoteType::ClaimFromMainchain { account_nonce: 1 },
				signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
				note_id: Default::default(),
			}],
		}];

		assert_ok!(super::verify_balance_changeset_allocation(&balance_change),);
	}

	#[test]
	fn test_sending_to_mainchain() {
		// This probably never happens - but in this scenario, funds are sent to a localchain to
		// transfer to a different mainchain account
		let balance_change = vec![
			BalanceChange {
				// We look for an transfer to localchain using this id
				account_id: AccountKeyring::Bob.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 2,
				previous_balance: 50,
				balance: 100,
				previous_balance_proof: Some(BalanceProof {
					notary_id: 0,
					notebook_number: 0,
					proof: bounded_vec![],
					leaf_index: 0,
					number_of_leaves: 0,
					account_origin: AccountOrigin { notebook_number: 0, account_uid: 1 },
				}),
				notes: bounded_vec![
					Note {
						milligons: 250,
						note_type: NoteType::ClaimFromMainchain { account_nonce: 15 },
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[0u8; 64]).unwrap()
						),
						note_id: H256([0u8; 32]),
					},
					Note {
						milligons: 200,
						note_type: NoteType::Send { recipient: None },
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[1u8; 64]).unwrap()
						),
						note_id: H256([1u8; 32]),
					}
				],
			},
			BalanceChange {
				account_id: AccountKeyring::Alice.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 1,
				previous_balance: 0,
				balance: 50,
				previous_balance_proof: None,
				notes: bounded_vec![
					Note {
						milligons: 200,
						note_type: NoteType::Claim,
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[2u8; 64]).unwrap()
						),
						note_id: H256([2u8; 32]),
					},
					Note {
						milligons: 150,
						note_type: NoteType::SendToMainchain,
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[3u8; 64]).unwrap()
						),
						note_id: H256([3u8; 32]),
					}
				],
			},
		];

		assert_ok!(super::verify_balance_changeset_allocation(&balance_change));
	}

	#[test]
	fn test_note_signatures() {
		let mut balance_change = vec![BalanceChange {
			// We look for an transfer to localchain using this id
			account_id: AccountKeyring::Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			previous_balance: 0,
			balance: 250,
			previous_balance_proof: None,
			notes: bounded_vec![Note {
				milligons: 250,
				note_type: NoteType::ClaimFromMainchain { account_nonce: 1 },
				signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
				note_id: Default::default(),
			}],
		}];

		assert_err!(
			super::verify_changeset_signatures(&balance_change),
			VerifyError::InvalidNoteIdCalculated
		);

		balance_change[0].notes[0].note_id = balance_change[0].notes[0].get_note_id(
			&balance_change[0].account_id,
			&balance_change[0].account_type,
			balance_change[0].change_number,
			0,
		);
		assert_err!(
			super::verify_changeset_signatures(&balance_change),
			VerifyError::InvalidNoteSignature
		);

		balance_change[0].notes[0].signature =
			AccountKeyring::Bob.sign(&balance_change[0].notes[0].note_id[..]).into();
		assert_ok!(super::verify_changeset_signatures(&balance_change));
	}

	struct TestLookup;

	parameter_types! {
		pub static NotebookRoots: BTreeMap<u32, H256> = BTreeMap::new();
		pub static LastChangedNotebook: BTreeMap<AccountOrigin, u32> = BTreeMap::new();
		pub static ValidLocalchainTransfers: BTreeSet<(AccountId32, u32)> = BTreeSet::new();
	}
	impl NotebookHistoryLookup for TestLookup {
		fn get_account_changes_root(
			_notary_id: u32,
			notebook_number: NotebookNumber,
		) -> Result<H256, AccountHistoryLookupError> {
			NotebookRoots::get()
				.get(&notebook_number)
				.ok_or(AccountHistoryLookupError::RootNotFound)
				.cloned()
		}
		fn get_last_changed_notebook(
			_notary_id: u32,
			account_origin: AccountOrigin,
		) -> Result<u32, AccountHistoryLookupError> {
			LastChangedNotebook::get()
				.get(&account_origin)
				.cloned()
				.ok_or(AccountHistoryLookupError::LastChangeNotFound)
		}
		fn is_valid_transfer_to_localchain(
			_notary_id: u32,
			account_id: &AccountId32,
			nonce: u32,
		) -> Result<bool, AccountHistoryLookupError> {
			ValidLocalchainTransfers::get()
				.get(&(account_id.clone(), nonce))
				.cloned()
				.ok_or(AccountHistoryLookupError::InvalidTransferToLocalchain)
				.map(|_| true)
		}
	}

	#[test]
	fn test_verify_previous_balance() {
		let mut final_balances = BTreeMap::<(AccountId32, AccountType), BalanceTip>::new();
		let account_id = AccountKeyring::Alice.to_account_id();
		let account_type = AccountType::Deposit;
		let key = (account_id.clone(), account_type.clone());

		let mut change = BalanceChange {
			account_id,
			account_type,
			change_number: 500,
			previous_balance: 100,
			balance: 0,
			previous_balance_proof: None,
			notes: bounded_vec![],
		};
		let leaves = vec![
			BalanceTip {
				account_id: Dave.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 20,
				change_number: 3,
				account_origin: AccountOrigin { notebook_number: 5, account_uid: 2 },
			}
			.encode(),
			BalanceTip {
				account_id: Bob.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 100,
				change_number: 1,
				account_origin: AccountOrigin { notebook_number: 6, account_uid: 1 },
			}
			.encode(),
			BalanceTip {
				account_id: change.account_id.clone(),
				account_type: change.account_type.clone(),
				balance: change.previous_balance,
				change_number: change.change_number - 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
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
			proof: BoundedVec::truncate_from(proof.proof),
			leaf_index: proof.leaf_index as u32,
			number_of_leaves: proof.number_of_leaves as u32,
			account_origin: origin.clone(),
		});

		assert_err!(
			verify_previous_balance_proof::<TestLookup>(
				&change.previous_balance_proof.clone().unwrap(),
				&mut final_balances,
				&change,
				&key,
			),
			VerifyError::InvalidPreviousBalanceChangeNotebook
		);

		LastChangedNotebook::mutate(|c| c.insert(origin, 7));
		assert_err!(
			verify_previous_balance_proof::<TestLookup>(
				&change.previous_balance_proof.clone().unwrap(),
				&mut final_balances,
				&change,
				&key,
			),
			VerifyError::InvalidPreviousBalanceProof
		);

		NotebookRoots::mutate(|a| a.insert(7, merkle_root));
		assert_ok!(verify_previous_balance_proof::<TestLookup>(
			&change.previous_balance_proof.clone().unwrap(),
			&mut final_balances,
			&change,
			&key,
		));
	}

	#[tokio::test]
	async fn test_verify_notebook() {
		let mut note = Note::create_unsigned(
			&Alice.to_account_id(),
			&AccountType::Deposit,
			1,
			0,
			1000,
			NoteType::ClaimFromMainchain { account_nonce: 1 },
		);
		note.signature = Alice.sign(&note.note_id[..]).into();

		let alice_balance_changeset = vec![BalanceChange {
			balance: 1000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance: 0,
			previous_balance_proof: None,
			notes: bounded_vec![note],
		}];
		let notebook_header1 = NotebookHeader {
			version: 1,
			notary_id: 1,
			notebook_number: 1,
			finalized_block_number: 100,
			pinned_to_block_number: 0,
			start_time: Utc::now().timestamp_millis() as u64 - 60_000,
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![BalanceTip {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 1000,
				change_number: 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			}
			.encode()]),
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain {
				account_id: Alice.to_account_id(),
				account_nonce: 1,
			}],
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			end_time: Utc::now().timestamp_millis() as u64,
		};

		ValidLocalchainTransfers::mutate(|a| a.insert((Alice.to_account_id(), 1)));
		let hash = notebook_header1.hash();

		let notebook1 = Notebook {
			header: notebook_header1.clone(),
			balance_changes: bounded_vec![BoundedVec::truncate_from(
				alice_balance_changeset.clone()
			)],
			new_account_origins: bounded_vec![NewAccountOrigin::new(
				Alice.to_account_id(),
				AccountType::Deposit,
				1
			)],
		};

		assert_ok!(super::notebook_verify::<TestLookup>(&hash, &notebook1));

		let mut bad_hash = hash.clone();
		bad_hash.0[0] = 1;
		assert_err!(
			super::notebook_verify::<TestLookup>(&bad_hash, &notebook1),
			VerifyError::InvalidNotebookHash
		);

		let mut bad_notebook1 = notebook1.clone();
		let _ = bad_notebook1.header.chain_transfers.try_insert(
			0,
			ChainTransfer::ToLocalchain { account_id: Bob.to_account_id(), account_nonce: 2 },
		);
		assert_err!(
			super::notebook_verify::<TestLookup>(&hash, &bad_notebook1),
			VerifyError::InvalidChainTransfersList
		);

		let mut bad_notebook = notebook1.clone();
		bad_notebook.header.changed_accounts_root.0[0] = 1;
		assert_err!(
			super::notebook_verify::<TestLookup>(&hash, &bad_notebook),
			VerifyError::InvalidBalanceChangeRoot
		);
	}

	#[tokio::test]
	async fn test_disallows_double_claim() {
		let mut note1 = Note::create_unsigned(
			&Alice.to_account_id(),
			&AccountType::Deposit,
			1,
			0,
			1000,
			NoteType::ClaimFromMainchain { account_nonce: 1 },
		);
		note1.signature = Alice.sign(&note1.note_id[..]).into();
		let mut note2 = Note::create_unsigned(
			&Alice.to_account_id(),
			&AccountType::Deposit,
			1,
			1,
			1000,
			NoteType::ClaimFromMainchain { account_nonce: 1 },
		);
		note2.signature = Alice.sign(&note2.note_id[..]).into();

		let alice_balance_changeset = vec![BalanceChange {
			balance: 2000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance: 0,
			previous_balance_proof: None,
			notes: bounded_vec![note1, note2],
		}];
		let notebook_header1 = NotebookHeader {
			version: 1,
			notary_id: 1,
			notebook_number: 1,
			finalized_block_number: 100,
			pinned_to_block_number: 0,
			start_time: Utc::now().timestamp_millis() as u64 - 60_000,
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![BalanceTip {
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				balance: 2000,
				change_number: 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			}
			.encode()]),
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain {
				account_id: Alice.to_account_id(),
				account_nonce: 1,
			}],
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			end_time: Utc::now().timestamp_millis() as u64,
		};

		ValidLocalchainTransfers::mutate(|a| a.insert((Alice.to_account_id(), 1)));
		let notebook1 = Notebook {
			header: notebook_header1.clone(),
			balance_changes: bounded_vec![BoundedVec::truncate_from(
				alice_balance_changeset.clone()
			)],
			new_account_origins: bounded_vec![NewAccountOrigin::new(
				Alice.to_account_id(),
				AccountType::Deposit,
				1
			)],
		};
		let hash = notebook_header1.hash();

		assert_err!(
			super::notebook_verify::<TestLookup>(&hash, &notebook1),
			VerifyError::DuplicateChainTransfer
		);
	}
}
