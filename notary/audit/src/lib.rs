use binary_merkle_tree::{merkle_root, verify_proof, Leaf};
use codec::Encode;
use sp_core::{crypto::AccountId32, Blake2Hasher, H256};
use sp_runtime::traits::Verify;
use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};

use ulx_notary_primitives::{
	ensure, AccountOrigin, BalanceChange, BalanceProof, BalanceTip, Chain, ChainTransfer,
	ChainTransferDestination, NotaryId, NoteType, NotebookHeader,
};

#[derive(Debug, PartialEq, Clone, thiserror::Error)]
pub enum VerifyError {
	#[error("Missing account origin {0:?}")]
	MissingAccountOrigin((AccountId32, Chain)),
	#[error("Account history lookup error {0}")]
	HistoryLookupError(#[from] AccountHistoryLookupError),
	#[error("Invalid account changelist")]
	InvalidAccountChangelist,
	#[error("Invalid chain transfers list")]
	InvalidChainTransfersList,
	#[error("Invalid balance change root")]
	InvalidBalanceChangeRoot,

	#[error("Invalid previous nonce")]
	InvalidPreviousNonce,
	#[error("Invalid previous balance")]
	InvalidPreviousBalance,
	#[error("Invalid previous account origin")]
	InvalidPreviousAccountOrigin,

	#[error("Invalid previous balance change notebook")]
	InvalidPreviousBalanceChangeNotebook,

	#[error("Invalid net balance change calculated")]
	InvalidBalanceChange,

	#[error("Invalid note signature")]
	InvalidNoteSignature,
	#[error("Invalid note id calculated")]
	InvalidNoteIdCalculated,
	#[error("An invalid balance change was submitted ({change_index}.{note_index}): {message}")]
	BalanceChangeError { change_index: usize, note_index: usize, message: String },

	#[error("Invalid net balance changeset. Must account for all funds.")]
	InvalidNetBalanceChangeset,

	#[error("Insufficient balance for account  (balance: {balance}, amount: {amount}) (change: {change_index}.{note_index})")]
	InsufficientBalance { balance: u128, amount: u128, note_index: usize, change_index: usize },

	#[error("Exceeded max balance for account (pre-balance: {balance}, amount: {amount}), (change: {change_index}.{note_index})")]
	ExceededMaxBalance { balance: u128, amount: u128, note_index: usize, change_index: usize },
	#[error("Balance change mismatch (provided_balance: {provided_balance}, calculated_balance: {calculated_balance}) (#{change_index})")]
	BalanceChangeMismatch { change_index: usize, provided_balance: u128, calculated_balance: i128 },

	#[error("Balance change not net zero (unaccounted: {unaccounted})")]
	BalanceChangeNotNetZero { unaccounted: i128 },

	#[error("Must include proof of previous balance")]
	MissingBalanceProof,
	#[error("Invalid previous balance proof")]
	InvalidPreviousBalanceProof,
	#[error("Invalid notebook hash")]
	InvalidNotebookHash,

	#[error("Duplicate chain transfer")]
	DuplicateChainTransfer,

	#[error("Duplicated account origin uid")]
	DuplicatedAccountOriginUid,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AccountHistoryLookupError {
	#[error("Notebook root not found")]
	RootNotFound,
	#[error("Last change not found")]
	LastChangeNotFound,
	#[error("Invalid transfer to localchain")]
	InvalidTransferToLocalchain,
}

pub trait NotebookHistoryLookup {
	fn get_account_changes_root(
		&self,
		notary_id: u32,
		notebook_number: u32,
	) -> Result<H256, AccountHistoryLookupError>;
	fn get_last_changed_notebook(
		&self,
		notary_id: u32,
		account_id: AccountId32,
		chain: Chain,
	) -> Result<u32, AccountHistoryLookupError>;

	fn is_valid_transfer_to_localchain(
		&self,
		notary_id: u32,
		account_id: &AccountId32,
		nonce: u32,
	) -> Result<bool, AccountHistoryLookupError>;
}

pub fn notebook_verify<'a, T: NotebookHistoryLookup>(
	hash: &'a H256,
	header: &'a NotebookHeader,
	changesets: &'a Vec<Vec<BalanceChange>>,
	new_account_origins: &'a BTreeMap<(AccountId32, Chain), u32>,
	lookup: &'a T,
) -> anyhow::Result<bool, VerifyError> {
	let mut account_changelist = BTreeSet::<AccountOrigin>::new();
	let mut final_balances = BTreeMap::<(AccountId32, Chain), BalanceTip>::new();
	let mut chain_transfers = Vec::<ChainTransfer>::new();
	let mut seen_transfers_in = BTreeSet::<(NotaryId, AccountId32, u32)>::new();

	ensure!(
		BTreeSet::from_iter(new_account_origins.iter().map(|a| a.1)).len() ==
			new_account_origins.len(),
		VerifyError::DuplicatedAccountOriginUid
	);

	for changeset in changesets {
		verify_balance_changeset_allocation(changeset)?;
		verify_changeset_signatures(changeset)?;

		for change in changeset.into_iter() {
			for note in &change.notes {
				// if this note is a chain transfer, track it in chain_transfers
				if let NoteType::ChainTransfer { destination, .. } = &note.note_type {
					match destination {
						ChainTransferDestination::ToLocalchain { nonce } => {
							lookup.is_valid_transfer_to_localchain(
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
								nonce: nonce.clone(),
							});
						},
						ChainTransferDestination::ToMainchain => {
							chain_transfers.push(ChainTransfer::ToMainchain {
								amount: note.milligons,
								account_id: change.account_id.clone(),
							});
						},
					};
				}
			}

			let key = (change.account_id.clone(), change.chain.clone());

			if change.nonce == 1 {
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
							chain: change.chain.clone(),
							balance: change.balance,
							nonce: change.nonce,
							account_origin,
						},
					);
				} else {
					return Err(VerifyError::MissingAccountOrigin(key))
				}
			} else {
				let proof = change
					.previous_balance_proof
					.as_ref()
					.expect("Should have been unwrapped in verify_balance_changeset_allocation");
				verify_previous_balance_proof(proof, lookup, &mut final_balances, change, &key)?;

				account_changelist.insert(proof.account_origin.clone());

				final_balances.insert(
					key.clone(),
					BalanceTip {
						account_id: change.account_id.clone(),
						chain: change.chain.clone(),
						balance: change.balance,
						nonce: change.nonce,
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

	let merkle_root = merkle_root::<Blake2Hasher, _>(merkle_leafs);

	ensure!(merkle_root == header.changed_accounts_root, VerifyError::InvalidBalanceChangeRoot);

	ensure!(*hash == header.hash(), VerifyError::InvalidNotebookHash);

	Ok(true)
}

fn verify_previous_balance_proof<'a, T: NotebookHistoryLookup>(
	proof: &BalanceProof,
	lookup: &T,
	final_balances: &mut BTreeMap<(AccountId32, Chain), BalanceTip>,
	change: &BalanceChange,
	key: &(AccountId32, Chain),
) -> anyhow::Result<bool, VerifyError> {
	// if we've changed balance in this notebook before, it must match the previous
	// entry
	if final_balances.contains_key(&key) {
		let previous_balance = final_balances.get(&key).unwrap();
		ensure!(
			previous_balance.balance == change.previous_balance,
			VerifyError::InvalidPreviousBalance
		);
		ensure!(previous_balance.nonce == change.nonce - 1, VerifyError::InvalidPreviousNonce);
		ensure!(
			previous_balance.account_origin == proof.account_origin,
			VerifyError::InvalidPreviousAccountOrigin
		);
	} else {
		let last_notebook_change = lookup.get_last_changed_notebook(
			proof.notary_id,
			change.account_id.clone(),
			change.chain.clone(),
		)?;
		ensure!(
			last_notebook_change == proof.notebook_number,
			VerifyError::InvalidPreviousBalanceChangeNotebook
		);

		let root = lookup.get_account_changes_root(proof.notary_id, proof.notebook_number)?;
		let leaf = BalanceTip {
			account_id: change.account_id.clone(),
			chain: change.chain.clone(),
			balance: change.previous_balance,
			nonce: change.nonce - 1,
			account_origin: proof.account_origin.clone(),
		};

		ensure!(
			verify_proof::<'_, Blake2Hasher, _, _>(
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
		for note in &change.notes {
			ensure!(
				note.get_note_id(&change.account_id, &change.chain, change.nonce) == note.note_id,
				VerifyError::InvalidNoteIdCalculated
			);
			ensure!(
				note.signature.verify(&note.note_id[..], &change.account_id),
				VerifyError::InvalidNoteSignature
			);
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
	let mut new_accounts = BTreeSet::<(AccountId32, Chain)>::new();

	for change in changes {
		ensure!(change.nonce > 0, VerifyError::InvalidBalanceChange);
		if change.nonce == 1 {
			new_accounts.insert((change.account_id.clone(), change.chain.clone()));

			ensure!(
				change.previous_balance_proof.is_none(),
				VerifyError::InvalidPreviousBalanceProof
			);
			ensure!(change.previous_balance == 0, VerifyError::InvalidPreviousBalance);
		}
		if change.nonce > 1 &&
			!new_accounts.contains(&(change.account_id.clone(), change.chain.clone()))
		{
			ensure!(change.previous_balance_proof.is_some(), VerifyError::MissingBalanceProof);
		}
		let mut balance = change.previous_balance as i128;
		let mut note_index = 0;
		for note in &change.notes {
			match note.note_type {
				NoteType::Send { .. } => {
					transferred_balances += note.milligons as i128;
				},
				NoteType::Claim => {
					transferred_balances -= note.milligons as i128;
				},
				_ => {},
			}

			match note.note_type {
				NoteType::ChainTransfer {
					destination: ChainTransferDestination::ToLocalchain { .. },
					..
				} |
				NoteType::Claim =>
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
				NoteType::ChainTransfer {
					destination: ChainTransferDestination::ToMainchain,
					..
				} |
				NoteType::Send { .. } => balance -= note.milligons as i128,
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
	use std::collections::{BTreeMap, BTreeSet};

	use crate::{
		verify_previous_balance_proof, AccountHistoryLookupError, NotebookHistoryLookup,
		VerifyError,
	};
	use ulx_notary_primitives::{
		balance_change::{AccountOrigin, BalanceChange, BalanceProof},
		note::{Chain, ChainTransferDestination, Note, NoteType},
		BalanceTip, ChainTransfer, NotebookHeader,
	};

	#[test]
	fn test_balance_change_allocation_errs_non_zero() {
		let balance_change = vec![BalanceChange {
			account_id: AccountKeyring::Alice.to_account_id(),
			chain: Chain::Argon,
			nonce: 1,
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
				chain: Chain::Argon,
				nonce: 1,
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
				chain: Chain::Argon,
				nonce: 1,
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
		balance_change[0].nonce = 2;
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
				chain: Chain::Argon,
				nonce: 2,
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
					note_id: Default::default(),
				}],
			},
			BalanceChange {
				account_id: AccountKeyring::Alice.to_account_id(),
				chain: Chain::Argon,
				nonce: 1,
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

		assert_ok!(super::verify_balance_changeset_allocation(&balance_change));
	}

	#[test]
	fn test_notes_must_add_up() {
		let mut balance_change = vec![
			BalanceChange {
				account_id: AccountKeyring::Bob.to_account_id(),
				chain: Chain::Argon,
				nonce: 2,
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
					note_id: Default::default(),
				}],
			},
			BalanceChange {
				account_id: AccountKeyring::Alice.to_account_id(),
				chain: Chain::Argon,
				nonce: 1,
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
			BalanceChange {
				account_id: AccountKeyring::Dave.to_account_id(),
				chain: Chain::Argon,
				nonce: 1,
				previous_balance: 0,
				balance: 100, // WRONG BALANCE - should be 150
				previous_balance_proof: None,
				notes: bounded_vec![Note {
					milligons: 150,
					note_type: NoteType::Claim,
					signature: MultiSignature::Sr25519(Signature::from_slice(&[0u8; 64]).unwrap()),
					note_id: Default::default(),
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
	fn test_sending_to_localchain() {
		let balance_change = vec![BalanceChange {
			// We look for an transfer to localchain using this id
			account_id: AccountKeyring::Bob.to_account_id(),
			chain: Chain::Argon,
			nonce: 1,
			previous_balance: 0,
			balance: 250,
			previous_balance_proof: None,
			notes: bounded_vec![Note {
				milligons: 250,
				note_type: NoteType::ChainTransfer {
					destination: ChainTransferDestination::ToLocalchain { nonce: 1 },
					finalized_at_block: 0,
				},
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
				chain: Chain::Argon,
				nonce: 2,
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
						note_type: NoteType::ChainTransfer {
							destination: ChainTransferDestination::ToLocalchain { nonce: 15 },
							finalized_at_block: 0,
						},
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[0u8; 64]).unwrap()
						),
						note_id: Default::default(),
					},
					Note {
						milligons: 200,
						note_type: NoteType::Send { recipient: None },
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[0u8; 64]).unwrap()
						),
						note_id: Default::default(),
					}
				],
			},
			BalanceChange {
				account_id: AccountKeyring::Alice.to_account_id(),
				chain: Chain::Argon,
				nonce: 1,
				previous_balance: 0,
				balance: 50,
				previous_balance_proof: None,
				notes: bounded_vec![
					Note {
						milligons: 200,
						note_type: NoteType::Claim,
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[0u8; 64]).unwrap()
						),
						note_id: Default::default(),
					},
					Note {
						milligons: 150,
						note_type: NoteType::ChainTransfer {
							destination: ChainTransferDestination::ToMainchain,
							finalized_at_block: 0,
						},
						signature: MultiSignature::Sr25519(
							Signature::from_slice(&[0u8; 64]).unwrap()
						),
						note_id: Default::default(),
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
			chain: Chain::Argon,
			nonce: 1,
			previous_balance: 0,
			balance: 250,
			previous_balance_proof: None,
			notes: bounded_vec![Note {
				milligons: 250,
				note_type: NoteType::ChainTransfer {
					destination: ChainTransferDestination::ToLocalchain { nonce: 1 },
					finalized_at_block: 0,
				},
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
			&balance_change[0].chain,
			balance_change[0].nonce,
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
		pub static LastChangedNotebook: BTreeMap<(AccountId32, Chain), u32> = BTreeMap::new();
		pub static ValidLocalchainTransfers: BTreeSet<(AccountId32, u32)> = BTreeSet::new();
	}
	impl NotebookHistoryLookup for TestLookup {
		fn get_account_changes_root(
			&self,
			_notary_id: u32,
			notebook_number: u32,
		) -> Result<H256, AccountHistoryLookupError> {
			NotebookRoots::get()
				.get(&notebook_number)
				.ok_or(AccountHistoryLookupError::RootNotFound)
				.cloned()
		}
		fn get_last_changed_notebook(
			&self,
			_notary_id: u32,
			account_id: AccountId32,
			chain: Chain,
		) -> Result<u32, AccountHistoryLookupError> {
			println!(
				"get_last_changed_notebook: {:?} {:?}, {:?}",
				account_id,
				chain,
				LastChangedNotebook::get()
			);
			LastChangedNotebook::get()
				.get(&(account_id, chain))
				.cloned()
				.ok_or(AccountHistoryLookupError::LastChangeNotFound)
		}
		fn is_valid_transfer_to_localchain(
			&self,
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
		let mut final_balances = BTreeMap::<(AccountId32, Chain), BalanceTip>::new();
		let account_id = AccountKeyring::Alice.to_account_id();
		let chain = Chain::Argon;
		let key = (account_id.clone(), chain.clone());

		let mut change = BalanceChange {
			account_id,
			chain,
			nonce: 500,
			previous_balance: 100,
			balance: 0,
			previous_balance_proof: None,
			notes: bounded_vec![],
		};
		let leaves = vec![
			BalanceTip {
				account_id: Dave.to_account_id(),
				chain: Chain::Argon,
				balance: 20,
				nonce: 3,
				account_origin: AccountOrigin { notebook_number: 5, account_uid: 2 },
			}
			.encode(),
			BalanceTip {
				account_id: Bob.to_account_id(),
				chain: Chain::Argon,
				balance: 100,
				nonce: 1,
				account_origin: AccountOrigin { notebook_number: 6, account_uid: 1 },
			}
			.encode(),
			BalanceTip {
				account_id: change.account_id.clone(),
				chain: change.chain.clone(),
				balance: change.previous_balance,
				nonce: change.nonce - 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			}
			.encode(),
		];
		let merkle_root = merkle_root::<Blake2Hasher, _>(&leaves);
		NotebookRoots::mutate(|a| {
			a.insert(7, H256::from_slice([&[0u8], &merkle_root[0..31]].concat().as_ref()))
		});
		LastChangedNotebook::mutate(|c| c.insert(key.clone(), 10));

		let proof = merkle_proof::<Blake2Hasher, _, _>(leaves, 2);
		change.previous_balance_proof = Some(BalanceProof {
			notary_id: 1,
			notebook_number: 7,
			proof: BoundedVec::truncate_from(proof.proof),
			leaf_index: proof.leaf_index as u32,
			number_of_leaves: proof.number_of_leaves as u32,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
		});

		assert_err!(
			verify_previous_balance_proof(
				&change.previous_balance_proof.clone().unwrap(),
				&TestLookup,
				&mut final_balances,
				&change,
				&key,
			),
			VerifyError::InvalidPreviousBalanceChangeNotebook
		);

		LastChangedNotebook::mutate(|c| c.insert(key.clone(), 7));
		assert_err!(
			verify_previous_balance_proof(
				&change.previous_balance_proof.clone().unwrap(),
				&TestLookup,
				&mut final_balances,
				&change,
				&key,
			),
			VerifyError::InvalidPreviousBalanceProof
		);

		NotebookRoots::mutate(|a| a.insert(7, merkle_root));
		assert_ok!(verify_previous_balance_proof(
			&change.previous_balance_proof.clone().unwrap(),
			&TestLookup,
			&mut final_balances,
			&change,
			&key,
		));
	}

	#[tokio::test]
	async fn test_verify_notebook() {
		let mut note = Note::create_unsigned(
			&Alice.to_account_id(),
			&Chain::Argon,
			1,
			1000,
			NoteType::ChainTransfer {
				destination: ChainTransferDestination::ToLocalchain { nonce: 1 },
				finalized_at_block: 100,
			},
		);
		note.signature = Alice.sign(&note.note_id[..]).into();

		let alice_balance_changeset = vec![BalanceChange {
			balance: 1000,
			nonce: 1,
			account_id: Alice.to_account_id(),
			chain: Chain::Argon,
			previous_balance: 0,
			previous_balance_proof: None,
			notes: bounded_vec![note],
		}];
		let notebook1 = NotebookHeader {
			version: 1,
			notary_id: 1,
			notebook_number: 1,
			finalized_block_number: 100,
			pinned_to_block_number: 0,
			start_time: Utc::now().timestamp_millis() as u64 - 60_000,
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![BalanceTip {
				account_id: Alice.to_account_id(),
				chain: Chain::Argon,
				balance: 1000,
				nonce: 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			}
			.encode()]),
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain {
				account_id: Alice.to_account_id(),
				nonce: 1,
			}],
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			end_time: Utc::now().timestamp_millis() as u64,
		};

		ValidLocalchainTransfers::mutate(|a| a.insert((Alice.to_account_id(), 1)));
		let hash = notebook1.hash();
		let new_accounts = BTreeMap::from_iter(vec![((Alice.to_account_id(), Chain::Argon), 1)]);
		assert_ok!(super::notebook_verify(
			&hash,
			&notebook1,
			&vec![alice_balance_changeset.clone()],
			&new_accounts,
			&TestLookup
		));

		let mut bad_hash = hash.clone();
		bad_hash.0[0] = 1;
		assert_err!(
			super::notebook_verify(
				&bad_hash,
				&notebook1,
				&vec![alice_balance_changeset.clone()],
				&new_accounts,
				&TestLookup
			),
			VerifyError::InvalidNotebookHash
		);

		let mut bad_notebook1 = notebook1.clone();
		let _ = bad_notebook1.chain_transfers.try_insert(
			0,
			ChainTransfer::ToLocalchain { account_id: Bob.to_account_id(), nonce: 2 },
		);
		assert_err!(
			super::notebook_verify(
				&hash,
				&bad_notebook1,
				&vec![alice_balance_changeset.clone()],
				&new_accounts,
				&TestLookup
			),
			VerifyError::InvalidChainTransfersList
		);

		let mut bad_notebook = notebook1.clone();
		bad_notebook.changed_accounts_root.0[0] = 1;
		assert_err!(
			super::notebook_verify(
				&hash,
				&bad_notebook,
				&vec![alice_balance_changeset.clone()],
				&new_accounts,
				&TestLookup
			),
			VerifyError::InvalidBalanceChangeRoot
		);
	}

	#[tokio::test]
	async fn test_disallows_double_claim() {
		let mut note = Note::create_unsigned(
			&Alice.to_account_id(),
			&Chain::Argon,
			1,
			1000,
			NoteType::ChainTransfer {
				destination: ChainTransferDestination::ToLocalchain { nonce: 1 },
				finalized_at_block: 100,
			},
		);
		note.signature = Alice.sign(&note.note_id[..]).into();

		let alice_balance_changeset = vec![BalanceChange {
			balance: 2000,
			nonce: 1,
			account_id: Alice.to_account_id(),
			chain: Chain::Argon,
			previous_balance: 0,
			previous_balance_proof: None,
			notes: bounded_vec![note.clone(), note],
		}];
		let notebook1 = NotebookHeader {
			version: 1,
			notary_id: 1,
			notebook_number: 1,
			finalized_block_number: 100,
			pinned_to_block_number: 0,
			start_time: Utc::now().timestamp_millis() as u64 - 60_000,
			changed_accounts_root: merkle_root::<Blake2Hasher, _>(vec![BalanceTip {
				account_id: Alice.to_account_id(),
				chain: Chain::Argon,
				balance: 2000,
				nonce: 1,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			}
			.encode()]),
			chain_transfers: bounded_vec![ChainTransfer::ToLocalchain {
				account_id: Alice.to_account_id(),
				nonce: 1,
			}],
			changed_account_origins: bounded_vec![AccountOrigin {
				notebook_number: 1,
				account_uid: 1
			}],
			end_time: Utc::now().timestamp_millis() as u64,
		};

		ValidLocalchainTransfers::mutate(|a| a.insert((Alice.to_account_id(), 1)));
		let hash = notebook1.hash();
		let new_accounts = BTreeMap::from_iter(vec![((Alice.to_account_id(), Chain::Argon), 1)]);

		assert_err!(
			super::notebook_verify(
				&hash,
				&notebook1,
				&vec![alice_balance_changeset.clone()],
				&new_accounts,
				&TestLookup
			),
			VerifyError::DuplicateChainTransfer
		);
	}
}
