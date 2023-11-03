#![cfg_attr(not(feature = "std"), no_std)]

use binary_merkle_tree::{merkle_root, verify_proof, Leaf};
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{scale_info::TypeInfo, traits::BlakeTwo256};
use sp_std::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	vec::Vec,
};

pub use error::VerifyError;
use ulx_notary_primitives::{
	ensure, AccountId, AccountOrigin, AccountOriginUid, AccountType, BalanceChange, BalanceProof,
	BalanceTip, ChainTransfer, NewAccountOrigin, NotaryId, Note, NoteType, Notebook,
	NotebookHeader, NotebookNumber, CHANNEL_CLAWBACK_NOTEBOOKS, CHANNEL_EXPIRATION_NOTEBOOKS,
	MIN_CHANNEL_NOTE_MILLIGONS,
};

pub mod error;

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
	let mut state = NotebookVerifyState::default();

	state.load_new_origins(notebook.new_account_origins.to_vec())?;
	let header = &notebook.header;

	for changeset in notebook.balance_changes.iter() {
		verify_balance_changeset_allocation(&changeset, Some(notebook.header.notebook_number))?;
		verify_changeset_signatures(&changeset)?;
		verify_balance_sources::<T>(&mut state, header, changeset)?;
	}

	ensure!(
		state.chain_transfers == header.chain_transfers.to_vec(),
		VerifyError::InvalidChainTransfersList
	);
	ensure!(
		BTreeSet::from_iter(header.changed_account_origins.to_vec()) == state.account_changelist,
		VerifyError::InvalidAccountChangelist
	);

	ensure!(
		state.get_merkle_root() == header.changed_accounts_root,
		VerifyError::InvalidBalanceChangeRoot
	);

	ensure!(*header_hash == header.hash(), VerifyError::InvalidNotebookHash);

	Ok(true)
}

#[derive(Clone, Default)]
struct NotebookVerifyState {
	account_changelist: BTreeSet<AccountOrigin>,
	final_balances: BTreeMap<(AccountId32, AccountType), BalanceTip>,
	chain_transfers: Vec<ChainTransfer>,
	seen_transfers_in: BTreeSet<(AccountId32, u32)>,
	new_account_origins: BTreeMap<(AccountId, AccountType), AccountOriginUid>,
}

impl NotebookVerifyState {
	pub fn track_final_balance(
		&mut self,
		key: &(AccountId, AccountType),
		change: &BalanceChange,
		account_origin: AccountOrigin,
		channel_hold_note: Option<Note>,
	) -> anyhow::Result<(), VerifyError> {
		self.account_changelist.insert(account_origin.clone());

		let tip = BalanceTip {
			account_id: change.account_id.clone(),
			account_type: change.account_type.clone(),
			change_number: change.change_number,
			balance: change.balance,
			account_origin,
			channel_hold_note,
		};
		self.final_balances.insert(key.clone(), tip);
		Ok(())
	}
	pub fn load_new_origins(
		&mut self,
		origins: Vec<NewAccountOrigin>,
	) -> anyhow::Result<(), VerifyError> {
		let mut all_new_account_uids = BTreeSet::<AccountOriginUid>::new();
		for NewAccountOrigin { account_id, account_type, account_uid } in origins {
			self.new_account_origins.insert((account_id, account_type), account_uid);
			ensure!(
				all_new_account_uids.insert(account_uid),
				VerifyError::DuplicatedAccountOriginUid
			);
		}
		Ok(())
	}

	pub fn get_merkle_root(&self) -> H256 {
		let merkle_leafs = self.final_balances.iter().map(|(_, v)| v.encode()).collect::<Vec<_>>();

		merkle_root::<BlakeTwo256, _>(merkle_leafs)
	}

	pub fn track_chain_transfer(
		&mut self,
		account_id: AccountId,
		note: &Note,
	) -> anyhow::Result<(), VerifyError> {
		match note.note_type {
			NoteType::SendToMainchain => {
				self.chain_transfers.push(ChainTransfer::ToMainchain {
					amount: note.milligons,
					account_id: account_id.clone(),
				});
			},
			NoteType::ClaimFromMainchain { account_nonce } => {
				ensure!(
					self.seen_transfers_in.insert((account_id.clone(), account_nonce,)),
					VerifyError::DuplicateChainTransfer
				);
				self.chain_transfers.push(ChainTransfer::ToLocalchain {
					account_id: account_id.clone(),
					account_nonce: account_nonce.clone(),
				});
			},
			_ => {},
		}
		Ok(())
	}
}

fn verify_balance_sources<'a, T: NotebookHistoryLookup>(
	state: &mut NotebookVerifyState,
	header: &NotebookHeader,
	changeset: &Vec<BalanceChange>,
) -> anyhow::Result<(), VerifyError> {
	let notary_id = header.notary_id;
	for change in changeset.into_iter() {
		let account_id = &change.account_id;
		let key = (account_id.clone(), change.account_type.clone());
		let mut channel_hold_note = None;

		for note in &change.notes {
			// if this note is a chain transfer, track it in chain_transfers
			match &note.note_type {
				NoteType::SendToMainchain => {
					state.track_chain_transfer(account_id.clone(), note)?;
				},
				NoteType::ClaimFromMainchain { account_nonce } => {
					T::is_valid_transfer_to_localchain(
						notary_id,
						account_id,
						account_nonce.clone(),
					)?;
					state.track_chain_transfer(account_id.clone(), note)?;
				},
				NoteType::ChannelHold { .. } => {
					channel_hold_note = Some(note.clone());
				},
				// this condition is redundant, but leaving for clarity
				NoteType::ChannelSettle { .. } => channel_hold_note = None,
				_ => {},
			}
		}

		if change.change_number == 1 {
			if let Some(account_uid) = state.new_account_origins.get(&key) {
				state.track_final_balance(
					&key,
					&change,
					AccountOrigin {
						notebook_number: header.notebook_number,
						account_uid: *account_uid,
					},
					channel_hold_note,
				)?;
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
			verify_previous_balance_proof::<T>(
				proof,
				header.notebook_number,
				&mut state.final_balances,
				&change,
				&key,
			)?;

			state.track_final_balance(
				&key,
				&change,
				proof.account_origin.clone(),
				channel_hold_note,
			)?;
		}
	}
	Ok(())
}

fn verify_previous_balance_proof<'a, T: NotebookHistoryLookup>(
	proof: &BalanceProof,
	notebook_number: NotebookNumber,
	final_balances: &mut BTreeMap<(AccountId32, AccountType), BalanceTip>,
	change: &BalanceChange,
	key: &(AccountId32, AccountType),
) -> anyhow::Result<bool, VerifyError> {
	// if we've changed balance in this notebook before, it must match the previous
	// entry
	if final_balances.contains_key(&key) {
		let previous_balance = final_balances.get(&key).unwrap();
		ensure!(proof.notebook_number == notebook_number, VerifyError::InvalidPreviousBalanceProof);
		let cited_balance = change.previous_balance_proof.as_ref().map(|a| a.balance).unwrap_or(0);
		ensure!(previous_balance.balance == cited_balance, VerifyError::InvalidPreviousBalance);
		ensure!(
			previous_balance.change_number == change.change_number - 1,
			VerifyError::InvalidPreviousNonce
		);
		ensure!(
			previous_balance.account_origin == proof.account_origin,
			VerifyError::InvalidPreviousAccountOrigin
		);
		// if none, we can add changes.. if set, we can't do anything else
		ensure!(
			previous_balance.channel_hold_note == change.channel_hold_note,
			VerifyError::InvalidChannelHoldNote
		);
	} else {
		let last_notebook_change =
			T::get_last_changed_notebook(proof.notary_id, proof.account_origin.clone())?;
		ensure!(
			last_notebook_change == proof.notebook_number,
			VerifyError::InvalidPreviousBalanceChangeNotebook
		);
		let Some(notebook_proof) = proof.notebook_proof.as_ref() else {
			return Err(VerifyError::MissingBalanceProof)
		};

		let root = T::get_account_changes_root(proof.notary_id, proof.notebook_number)?;
		let channel_hold_note = change.channel_hold_note.as_ref().cloned();

		let leaf = BalanceTip {
			account_id: change.account_id.clone(),
			account_type: change.account_type.clone(),
			balance: proof.balance,
			change_number: change.change_number - 1,
			account_origin: proof.account_origin.clone(),
			channel_hold_note,
		};

		ensure!(
			verify_proof::<'_, BlakeTwo256, _, _>(
				&root,
				notebook_proof.proof.clone().into_inner(),
				notebook_proof.number_of_leaves as usize,
				notebook_proof.leaf_index as usize,
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
	for (index, change) in changeset.iter().enumerate() {
		// check that note id is valid for a hold note
		if let Some(channel_note) = &change.channel_hold_note {
			ensure!(
				matches!(channel_note.note_type, NoteType::ChannelHold { .. }),
				VerifyError::InvalidChannelHoldNote
			);
		}

		ensure!(
			change.verify_signature(),
			VerifyError::InvalidBalanceChangeSignature { change_index: index as u16 }
		);
	}
	Ok(())
}

#[derive(Default, PartialEq, Eq, Clone, Debug)]
pub struct BalanceChangesetState {
	/// How much was sent
	pub sent_deposits: u128,
	/// How much was claimed
	pub claimed_deposits: u128,
	/// How much tax was sent
	pub sent_tax: u128,
	/// How much tax was claimed
	pub claimed_tax: u128,
	/// All new accounts that were created (change_number = 1)
	pub new_accounts: BTreeSet<(AccountId, AccountType)>,
	/// All channel hold notes created per account (each account can only create one)
	pub accounts_with_new_holds: BTreeSet<AccountId>,
	/// Whether or not the current notebook number is needed to confirm channel settles
	pub needs_channel_settle_followup: bool,
	/// How much in channel funds was claimed by each account id
	pub claimed_channel_deposits_per_account: BTreeMap<AccountId, u128>,
	/// How much tax was sent per account
	pub tax_per_account: BTreeMap<AccountId, u128>,
	/// How much was deposited per account
	pub claimed_deposits_per_account: BTreeMap<AccountId, u128>,

	restricted_balance: BTreeMap<BTreeSet<(AccountId, AccountType)>, i128>,
	unclaimed_channel_balances: BTreeMap<BTreeSet<AccountId>, i128>,
}

impl BalanceChangesetState {
	fn verify_change_number(
		&mut self,
		change: &BalanceChange,
		key: &(AccountId, AccountType),
	) -> anyhow::Result<(), VerifyError> {
		ensure!(change.change_number > 0, VerifyError::InvalidBalanceChange);
		if change.change_number == 1 {
			self.new_accounts.insert(key.clone());

			ensure!(
				change.previous_balance_proof.is_none(),
				VerifyError::InvalidPreviousBalanceProof
			);
		}
		if change.change_number > 1 && !self.new_accounts.contains(&key) {
			ensure!(change.previous_balance_proof.is_some(), VerifyError::MissingBalanceProof);
		}
		Ok(())
	}

	fn send_balance(
		&mut self,
		milligons: u128,
		recipients: &Vec<AccountId>,
		account_type: &AccountType,
	) {
		if account_type == &AccountType::Tax {
			self.sent_tax += milligons;
		} else {
			self.sent_deposits += milligons;
		}

		if recipients.len() > 0 {
			let mut set = BTreeSet::new();
			for rec in recipients {
				set.insert((rec.clone(), account_type.clone()));
			}
			self.restricted_balance.insert(set, milligons as i128);
		}
	}

	fn record_tax(
		&mut self,
		milligons: u128,
		claimer: &AccountId,
	) -> anyhow::Result<(), VerifyError> {
		self.sent_tax += milligons;
		if let Some(starting) = self.tax_per_account.insert(claimer.clone(), milligons) {
			self.tax_per_account.get_mut(claimer).map(|a| *a += starting);
		}

		Ok(())
	}

	fn claim_balance(
		&mut self,
		milligons: u128,
		claimer: &AccountId,
		account_type: &AccountType,
	) -> anyhow::Result<(), VerifyError> {
		if account_type == &AccountType::Tax {
			self.claimed_tax += milligons;
		} else {
			if let Some(starting) =
				self.claimed_deposits_per_account.insert(claimer.clone(), milligons)
			{
				self.claimed_deposits_per_account.get_mut(claimer).map(|a| *a += starting);
			}
			self.claimed_deposits += milligons;
		}

		let key = (claimer.clone(), account_type.clone());
		self.restricted_balance.retain(|accounts, amount| {
			if accounts.contains(&key) {
				let restricted_change = amount.saturating_sub(milligons as i128);
				if restricted_change > 0 {
					*amount = restricted_change;
					return true
				}
				return false
			}
			return true
		});

		Ok(())
	}

	fn claim_channel_balance(
		&mut self,
		milligons: u128,
		claimer: &AccountId,
	) -> anyhow::Result<(), VerifyError> {
		self.claimed_deposits += milligons;
		if let Some(starting) =
			self.claimed_channel_deposits_per_account.insert(claimer.clone(), milligons)
		{
			self.claimed_channel_deposits_per_account
				.get_mut(claimer)
				.map(|a| *a += starting);
		}

		self.unclaimed_channel_balances.retain(|accounts, amount| {
			if accounts.contains(claimer) {
				let restricted_change = amount.saturating_sub(milligons as i128);
				if restricted_change > 0 {
					*amount = restricted_change;
					return true
				}
				return false
			}
			return true
		});

		Ok(())
	}

	/// Records the channel settles. If this is the second pass once we know a notebook number, it
	/// will also check if the channel is ready to be claimed
	fn record_channel_settle(
		&mut self,
		key: &(AccountId, AccountType),
		milligons: i128,
		channel_hold_note: &Note,
		source_change_notebook: NotebookNumber,
		notebook_number: Option<NotebookNumber>,
	) -> anyhow::Result<(), VerifyError> {
		let mut recipients = BTreeSet::new();

		// only add the recipient restrictions once we know what notebook we're in
		if let Some(notebook_number) = notebook_number {
			let expiration_notebook = source_change_notebook + CHANNEL_EXPIRATION_NOTEBOOKS;
			ensure!(
				notebook_number >= expiration_notebook,
				VerifyError::ChannelHoldNotReadyForClaim
			);

			let NoteType::ChannelHold { recipient, .. } = &channel_hold_note.note_type else {
				return Err(VerifyError::InvalidChannelHoldNote)
			};

			recipients.insert(recipient.clone());
			if notebook_number >= expiration_notebook + CHANNEL_CLAWBACK_NOTEBOOKS {
				// no claim necessary for a 0 claim
				if milligons == 0 {
					recipients.clear();
				} else {
					recipients.insert(key.0.clone());
				}
			}
		} else {
			self.needs_channel_settle_followup = true;
		}

		self.sent_deposits += milligons as u128;
		if !recipients.is_empty() {
			self.unclaimed_channel_balances
				.insert(BTreeSet::from_iter(recipients), milligons);
		}
		Ok(())
	}
}

/// This function verifies the proposed balance changes prior to accessing storage or verifying
/// proofs
/// 1. Confirm each proposed balance change adds up properly
/// 2. Confirm the changes net out to 0 (no funds are left outside an account)
pub fn verify_balance_changeset_allocation(
	changes: &Vec<BalanceChange>,
	notebook_number: Option<NotebookNumber>,
) -> anyhow::Result<BalanceChangesetState, VerifyError> {
	let mut state = BalanceChangesetState::default();

	let mut change_index = 0;
	for change in changes {
		let key = (change.account_id.clone(), change.account_type.clone());
		state.verify_change_number(change, &key)?;

		let mut balance =
			change.previous_balance_proof.as_ref().map(|a| a.balance).unwrap_or_default() as i128;
		let mut note_index = 0;

		for note in &change.notes {
			if change.channel_hold_note.is_some() &&
				!matches!(note.note_type, NoteType::ChannelSettle { .. })
			{
				return Err(VerifyError::AccountLocked)
			}

			if change.account_type == AccountType::Tax {
				match note.note_type {
					NoteType::Claim | NoteType::Send { .. } => {},
					_ => Err(VerifyError::InvalidTaxOperation)?,
				}
			}

			match &note.note_type {
				NoteType::Send { to: recipients } => {
					state.send_balance(
						note.milligons,
						&recipients.as_ref().map(|a| a.to_vec()).unwrap_or_default(),
						&change.account_type,
					);
				},
				NoteType::Claim => {
					state.claim_balance(
						note.milligons,
						&change.account_id,
						&change.account_type,
					)?;
				},
				NoteType::ChannelHold { .. } => {
					ensure!(
						note.milligons >= MIN_CHANNEL_NOTE_MILLIGONS,
						VerifyError::InvalidChannelHoldNote
					);
					// A channel doesn't change the source balance
					ensure!(
						change.balance ==
							change
								.previous_balance_proof
								.as_ref()
								.map(|a| a.balance)
								.unwrap_or_default(),
						VerifyError::InvalidPreviousBalanceProof
					);
					ensure!(
						change.channel_hold_note.is_none() &&
							state.accounts_with_new_holds.insert(key.0.clone()),
						VerifyError::AccountAlreadyHasChannelHold
					);
				},
				NoteType::ChannelClaim => {
					state.claim_channel_balance(note.milligons, &change.account_id)?;
				},
				NoteType::ChannelSettle => {
					let Some(source_change_notebook) =
						change.previous_balance_proof.as_ref().map(|a| a.notebook_number)
					else {
						return Err(VerifyError::MissingBalanceProof)
					};

					let channel_hold_note = change
						.channel_hold_note
						.as_ref()
						.ok_or(VerifyError::MissingChannelHoldNote)?;

					state.record_channel_settle(
						&key,
						note.milligons as i128,
						channel_hold_note,
						source_change_notebook,
						notebook_number,
					)?;
				},
				NoteType::Tax => {
					ensure!(
						change.account_type == AccountType::Deposit,
						VerifyError::InvalidTaxOperation
					);
					state.record_tax(note.milligons, &change.account_id)?;
				},
				_ => {},
			}

			// track the balances moved in this note
			match note.note_type {
				NoteType::ClaimFromMainchain { .. } |
				NoteType::Claim { .. } |
				NoteType::ChannelClaim =>
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
				NoteType::SendToMainchain |
				NoteType::Send { .. } |
				NoteType::ChannelSettle |
				NoteType::Tax => balance -= note.milligons as i128,
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
		state.claimed_deposits == state.sent_deposits,
		VerifyError::BalanceChangeNotNetZero {
			sent: state.sent_deposits,
			claimed: state.claimed_deposits
		}
	);
	ensure!(
		state.claimed_tax == state.sent_tax,
		VerifyError::TaxBalanceChangeNotNetZero {
			sent: state.sent_tax,
			claimed: state.claimed_tax
		}
	);
	// this works by removing all restricted balances as the approved users draw from them
	ensure!(state.restricted_balance.is_empty(), VerifyError::InvalidNoteRecipients);
	ensure!(state.unclaimed_channel_balances.is_empty(), VerifyError::InvalidChannelClaimers);
	Ok(state)
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
		Ed25519Keyring::{Dave, Ferdie},
		Sr25519Keyring::{Alice, Bob},
	};
	use sp_runtime::MultiSignature;

	use ulx_notary_primitives::{
		balance_change::{AccountOrigin, BalanceChange, BalanceProof},
		note::{AccountType, Note, NoteType},
		BalanceTip, ChainTransfer, MerkleProof, NewAccountOrigin, Notebook, NotebookHeader,
		NotebookNumber,
	};

	use crate::{
		verify_balance_changeset_allocation, verify_changeset_signatures,
		verify_previous_balance_proof, AccountHistoryLookupError, NotebookHistoryLookup,
		VerifyError,
	};

	use super::notebook_verify;

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

	fn test_balance_change(
		account: AccountKeyring,
		change_number: u32,
		balance: u128,
		prev_balance: u128,
		milligons: u128,
		note_type: NoteType,
	) -> BalanceChange {
		BalanceChange {
			account_id: account.to_account_id(),
			account_type: AccountType::Deposit,
			change_number,
			balance,
			previous_balance_proof: if change_number == 1 {
				None
			} else {
				empty_proof(prev_balance)
			},
			channel_hold_note: None,
			notes: Default::default(),
			signature: empty_signature(),
		}
		.push_note(milligons, note_type)
		.sign(account.pair())
		.clone()
	}

	#[test]
	fn test_balance_change_allocation_errs_non_zero() {
		let balance_change = test_balance_change(Alice, 1, 100, 0, 100, NoteType::Claim);

		assert_err!(
			verify_balance_changeset_allocation(&vec![balance_change], None),
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
			verify_balance_changeset_allocation(&balance_change, None),
			VerifyError::MissingBalanceProof
		);
	}

	#[test]
	fn test_balance_change_allocation_must_be_zero() {
		let balance_change = vec![
			test_balance_change(
				Bob,
				2,
				0,
				100,
				100,
				NoteType::Send { to: Some(bounded_vec!(Alice.to_account_id())) },
			),
			test_balance_change(Alice, 1, 100, 0, 100, NoteType::Claim),
		];

		assert_ok!(verify_balance_changeset_allocation(&balance_change, None));
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
			verify_balance_changeset_allocation(&balance_change, None),
			VerifyError::BalanceChangeMismatch {
				change_index: 2,
				provided_balance: 100,
				calculated_balance: 150
			}
		);

		balance_change[2].balance = 150;
		assert_ok!(verify_balance_changeset_allocation(&balance_change, None));
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
			verify_balance_changeset_allocation(&balance_change, None),
			VerifyError::InvalidNoteRecipients
		);

		balance_change[1].balance = 250;
		balance_change[1].notes[0].milligons = 250;
		balance_change.pop();
		assert_ok!(verify_balance_changeset_allocation(&balance_change, None));
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

		assert_ok!(verify_balance_changeset_allocation(&balance_change, None),);
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

		assert_ok!(verify_balance_changeset_allocation(&balance_change, None));
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
		assert_ok!(verify_balance_changeset_allocation(&vec![balance_change], Some(1)),);

		assert_err!(
			verify_balance_changeset_allocation(
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
			notes: bounded_vec![Note::create(50, NoteType::ChannelSettle)],
			signature: empty_signature(),
		};

		assert_err!(
			verify_balance_changeset_allocation(&vec![channel_settle.clone()], Some(2)),
			VerifyError::ChannelHoldNotReadyForClaim
		);

		// try to clear out change
		channel_settle.balance = 250;
		channel_settle.notes[0].milligons = 0;

		// it won't let you claim your own note back
		assert_err!(
			verify_balance_changeset_allocation(&vec![channel_settle.clone()], Some(61)),
			VerifyError::InvalidChannelClaimers
		);

		// it WILL let you claim back your own note if it's past the grace period
		assert_ok!(verify_balance_changeset_allocation(&vec![channel_settle.clone()], Some(71)),);

		let changes = vec![
			BalanceChange {
				account_id: Bob.to_account_id(),
				account_type: AccountType::Deposit,
				change_number: 3,
				balance: 200,
				previous_balance_proof: empty_proof(250),
				channel_hold_note: Some(channel_note.clone()),
				notes: bounded_vec![Note::create(50, NoteType::ChannelSettle)],
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
			verify_balance_changeset_allocation(&changes, None)?.needs_channel_settle_followup,
			true
		);
		// a valid claim is also acceptable
		assert_ok!(verify_balance_changeset_allocation(&changes, Some(61)),);

		Ok(())
	}

	#[test]
	fn test_note_signatures() {
		let mut balance_change = vec![BalanceChange {
			// We look for an transfer to localchain using this id
			account_id: Bob.to_account_id(),
			account_type: AccountType::Deposit,
			change_number: 1,
			balance: 250,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				250,
				NoteType::ClaimFromMainchain { account_nonce: 1 }
			),],
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
	fn test_note_claim_signatures() {
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
		balance_change2.push_note(50, NoteType::ChannelSettle);
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
			verify_balance_changeset_allocation(&set, Some(1)),
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
			verify_balance_changeset_allocation(&claim_tax_on_deposit, Some(1)),
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
		assert_ok!(verify_balance_changeset_allocation(&claim_tax_on_deposit, Some(1)),);
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
		let account_id = Alice.to_account_id();
		let account_type = AccountType::Deposit;
		let key = (account_id.clone(), account_type.clone());

		let mut change = BalanceChange {
			account_id,
			account_type,
			change_number: 500,
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
				account_type: change.account_type.clone(),
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
			notebook_proof: Some(MerkleProof {
				proof: BoundedVec::truncate_from(proof.proof),
				leaf_index: proof.leaf_index as u32,
				number_of_leaves: proof.number_of_leaves as u32,
			}),
			account_origin: origin.clone(),
			balance: 100,
		});

		assert_err!(
			verify_previous_balance_proof::<TestLookup>(
				&change.previous_balance_proof.clone().unwrap(),
				7,
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
				7,
				&mut final_balances,
				&change,
				&key,
			),
			VerifyError::InvalidPreviousBalanceProof
		);

		NotebookRoots::mutate(|a| a.insert(7, merkle_root));
		assert_ok!(verify_previous_balance_proof::<TestLookup>(
			&change.previous_balance_proof.clone().unwrap(),
			7,
			&mut final_balances,
			&change,
			&key,
		));
	}

	#[test]
	fn test_verify_notebook() {
		let note = Note::create(1000, NoteType::ClaimFromMainchain { account_nonce: 1 });

		let alice_balance_changeset = vec![BalanceChange {
			balance: 1000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![note],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone()];
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
				channel_hold_note: None,
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

		assert_ok!(notebook_verify::<TestLookup>(&hash, &notebook1));

		let mut bad_hash = hash.clone();
		bad_hash.0[0] = 1;
		assert_err!(
			notebook_verify::<TestLookup>(&bad_hash, &notebook1),
			VerifyError::InvalidNotebookHash
		);

		let mut bad_notebook1 = notebook1.clone();
		let _ = bad_notebook1.header.chain_transfers.try_insert(
			0,
			ChainTransfer::ToLocalchain { account_id: Bob.to_account_id(), account_nonce: 2 },
		);
		assert_err!(
			notebook_verify::<TestLookup>(&hash, &bad_notebook1),
			VerifyError::InvalidChainTransfersList
		);

		let mut bad_notebook = notebook1.clone();
		bad_notebook.header.changed_accounts_root.0[0] = 1;
		assert_err!(
			notebook_verify::<TestLookup>(&hash, &bad_notebook),
			VerifyError::InvalidBalanceChangeRoot
		);
	}

	#[test]
	fn test_disallows_double_claim() {
		let note1 = Note::create(1000, NoteType::ClaimFromMainchain { account_nonce: 1 });
		let note2 = Note::create(1000, NoteType::ClaimFromMainchain { account_nonce: 1 });

		let alice_balance_changeset = vec![BalanceChange {
			balance: 2000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![note1, note2],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone()];
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
				channel_hold_note: None,
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
			notebook_verify::<TestLookup>(&hash, &notebook1),
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
					Note::create(1000, NoteType::ClaimFromMainchain { account_nonce: 1 }),
					Note::create(1000, NoteType::Send { to: None }),
				],
				signature: empty_signature(),
			}
			.sign(Alice.pair())
			.clone(),
			BalanceChange {
				balance: 800,
				change_number: 1,
				account_id: Bob.to_account_id(),
				account_type: AccountType::Deposit,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![
					Note::create(1000, NoteType::Claim),
					Note::create(200, NoteType::Tax),
				],
				signature: empty_signature(),
			}
			.sign(Bob.pair())
			.clone(),
			BalanceChange {
				balance: 200,
				change_number: 1,
				account_id: Bob.to_account_id(),
				account_type: AccountType::Tax,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![Note::create(200, NoteType::Claim),],
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
					balance: 800,
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
					balance: 200,
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
				finalized_block_number: 100,
				pinned_to_block_number: 0,
				start_time: Utc::now().timestamp_millis() as u64 - 60_000,
				changed_accounts_root: merkle_root::<Blake2Hasher, _>(
					balance_tips.iter().map(|(_, v)| v.encode()).collect::<Vec<_>>(),
				),
				chain_transfers: bounded_vec![ChainTransfer::ToLocalchain {
					account_id: Alice.to_account_id(),
					account_nonce: 1,
				}],
				changed_account_origins: bounded_vec![
					AccountOrigin { notebook_number: 1, account_uid: 1 },
					AccountOrigin { notebook_number: 1, account_uid: 2 },
					AccountOrigin { notebook_number: 1, account_uid: 3 }
				],
				end_time: Utc::now().timestamp_millis() as u64,
			},
			balance_changes: bounded_vec![BoundedVec::truncate_from(alice_balance_changeset),],
			new_account_origins: bounded_vec![
				NewAccountOrigin::new(Alice.to_account_id(), AccountType::Deposit, 1),
				NewAccountOrigin::new(Bob.to_account_id(), AccountType::Deposit, 2),
				NewAccountOrigin::new(Bob.to_account_id(), AccountType::Tax, 3)
			],
		};

		assert_ok!(notebook_verify::<TestLookup>(&notebook.header.hash(), &notebook),);

		let changeset2 = vec![
			BalanceChange {
				balance: 0,
				change_number: 2,
				account_id: Bob.to_account_id(),
				account_type: AccountType::Deposit,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![Note::create(800, NoteType::Send { to: None }),],
				signature: empty_signature(),
			}
			.sign(Bob.pair())
			.clone(),
			BalanceChange {
				balance: 600,
				change_number: 2,
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![
					Note::create(800, NoteType::Claim),
					Note::create(200, NoteType::Tax),
				],
				signature: empty_signature(),
			}
			.sign(Alice.pair())
			.clone(),
			BalanceChange {
				balance: 200,
				change_number: 1,
				account_id: Alice.to_account_id(),
				account_type: AccountType::Tax,
				previous_balance_proof: None,
				channel_hold_note: None,
				notes: bounded_vec![Note::create(200, NoteType::Claim),],
				signature: empty_signature(),
			}
			.sign(Alice.pair())
			.clone(),
		];
		notebook.header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(
			balance_tips.iter().map(|(_, v)| v.encode()).collect::<Vec<_>>(),
		);
		notebook
			.balance_changes
			.try_push(BoundedVec::truncate_from(changeset2))
			.expect("should insert");
		if let Some(tip) = balance_tips.get_mut(&(Bob.to_account_id(), AccountType::Deposit)) {
			tip.change_number = 2;
			tip.balance = 0;
		}
		if let Some(tip) = balance_tips.get_mut(&(Alice.to_account_id(), AccountType::Deposit)) {
			tip.change_number = 2;
			tip.balance = 600;
		}
		balance_tips.insert(
			(Alice.to_account_id(), AccountType::Tax),
			BalanceTip {
				balance: 200,
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
		assert_err!(
			notebook_verify::<TestLookup>(&notebook.header.hash(), &notebook),
			VerifyError::MissingBalanceProof
		);
		notebook.header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(
			balance_tips.iter().map(|(_, v)| v.encode()).collect::<Vec<_>>(),
		);
		notebook.balance_changes[1][0].previous_balance_proof = Some(BalanceProof {
			notary_id: 1,
			notebook_number: 1,
			notebook_proof: None,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 2 },
			balance: 800,
		});
		notebook.balance_changes[1][1].previous_balance_proof = Some(BalanceProof {
			notary_id: 1,
			notebook_number: 1,
			notebook_proof: None,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			balance: 0,
		});
		notebook.balance_changes[1][2].previous_balance_proof = Some(BalanceProof {
			notary_id: 1,
			notebook_number: 1,
			notebook_proof: None,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			balance: 0,
		});
		assert_err!(
			notebook_verify::<TestLookup>(&notebook.header.hash(), &notebook),
			VerifyError::InvalidPreviousBalanceProof
		);
		notebook
			.header
			.changed_account_origins
			.try_push(AccountOrigin { notebook_number: 1, account_uid: 4 })
			.expect("should insert");

		notebook.balance_changes[1][2].previous_balance_proof = None;
		assert_ok!(notebook_verify::<TestLookup>(&notebook.header.hash(), &notebook),);
	}

	#[test]
	fn test_cannot_remove_lock_between_changesets_in_a_notebook() {
		let alice_balance_changeset = vec![BalanceChange {
			balance: 1000,
			change_number: 1,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: None,
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				1000,
				NoteType::ClaimFromMainchain { account_nonce: 1 }
			),],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone()];
		let alice_balance_changeset2 = vec![BalanceChange {
			balance: 1000,
			change_number: 2,
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			previous_balance_proof: Some(BalanceProof {
				notary_id: 1,
				notebook_number: 1,
				notebook_proof: None,
				account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
				balance: 1000,
			}),
			channel_hold_note: None,
			notes: bounded_vec![Note::create(
				1000,
				NoteType::ChannelHold { recipient: Bob.to_account_id() }
			)],
			signature: empty_signature(),
		}
		.sign(Alice.pair())
		.clone()];

		ValidLocalchainTransfers::mutate(|a| a.insert((Alice.to_account_id(), 1)));
		let mut notebook = Notebook {
			header: NotebookHeader {
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
					change_number: 2,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					channel_hold_note: None,
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
			},
			balance_changes: bounded_vec![
				BoundedVec::truncate_from(alice_balance_changeset),
				BoundedVec::truncate_from(alice_balance_changeset2),
			],
			new_account_origins: bounded_vec![NewAccountOrigin::new(
				Alice.to_account_id(),
				AccountType::Deposit,
				1
			)],
		};

		// test that the change root records the hold note
		assert_err!(
			notebook_verify::<TestLookup>(&notebook.header.hash(), &notebook),
			VerifyError::InvalidBalanceChangeRoot
		);

		let hold_note = notebook.balance_changes[1][0].notes[0].clone();

		notebook.header.changed_accounts_root = merkle_root::<Blake2Hasher, _>(vec![BalanceTip {
			account_id: Alice.to_account_id(),
			account_type: AccountType::Deposit,
			balance: 1000,
			change_number: 2,
			account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
			channel_hold_note: Some(hold_note),
		}
		.encode()]);
		assert_ok!(notebook_verify::<TestLookup>(&notebook.header.hash(), &notebook),);

		// now confirm we can't remove the hold in the same set of changes
		{
			// Try 1: pretend it didn't happen
			let alice_balance_changeset3 = vec![BalanceChange {
				balance: 1000,
				change_number: 3,
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				previous_balance_proof: Some(BalanceProof {
					notary_id: 1,
					notebook_number: 1,
					notebook_proof: None,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					balance: 1000,
				}),
				channel_hold_note: None,
				notes: bounded_vec![Note::create(
					1000,
					NoteType::ChannelHold { recipient: Ferdie.to_account_id() }
				)],
				signature: empty_signature(),
			}
			.sign(Alice.pair())
			.clone()];
			let mut notebook = notebook.clone();
			let _ = notebook
				.balance_changes
				.try_push(BoundedVec::truncate_from(alice_balance_changeset3));
			let hold_note = notebook.balance_changes[2][0].notes[0].clone();
			notebook.header.changed_accounts_root =
				merkle_root::<Blake2Hasher, _>(vec![BalanceTip {
					account_id: Alice.to_account_id(),
					account_type: AccountType::Deposit,
					balance: 1000,
					change_number: 3,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					channel_hold_note: Some(hold_note),
				}
				.encode()]);
			assert_err!(
				notebook_verify::<TestLookup>(&notebook.header.hash(), &notebook),
				VerifyError::InvalidChannelHoldNote
			);
		}
		{
			// Try 2: try to remove the hold
			let alice_balance_changeset3 = vec![BalanceChange {
				balance: 1000,
				change_number: 3,
				account_id: Alice.to_account_id(),
				account_type: AccountType::Deposit,
				previous_balance_proof: Some(BalanceProof {
					notary_id: 1,
					notebook_number: 1,
					notebook_proof: None,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					balance: 1000,
				}),
				channel_hold_note: Some(Note::create(
					1000,
					NoteType::ChannelHold { recipient: Bob.to_account_id() },
				)),
				notes: bounded_vec![Note::create(0, NoteType::ChannelSettle)],
				signature: empty_signature(),
			}
			.sign(Alice.pair())
			.clone()];

			let mut notebook = notebook.clone();
			let _ = notebook
				.balance_changes
				.try_push(BoundedVec::truncate_from(alice_balance_changeset3));
			let hold_note = notebook.balance_changes[2][0].notes[0].clone();

			notebook.header.changed_accounts_root =
				merkle_root::<Blake2Hasher, _>(vec![BalanceTip {
					account_id: Alice.to_account_id(),
					account_type: AccountType::Deposit,
					balance: 1000,
					change_number: 3,
					account_origin: AccountOrigin { notebook_number: 1, account_uid: 1 },
					channel_hold_note: Some(hold_note),
				}
				.encode()]);
			assert_err!(
				notebook_verify::<TestLookup>(&notebook.header.hash(), &notebook),
				VerifyError::ChannelHoldNotReadyForClaim
			);
		}
	}
}
