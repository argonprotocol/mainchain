#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	vec::Vec,
};
use binary_merkle_tree::{merkle_root, verify_proof, Leaf};
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use sp_core::H256;
use sp_runtime::{
	scale_info::TypeInfo,
	traits::{BlakeTwo256, Verify},
};

use argon_primitives::{
	ensure, round_up, tick::Tick, AccountId, AccountOrigin, AccountOriginUid, AccountType, Balance,
	BalanceChange, BalanceProof, BalanceTip, BlockVote, ChainTransfer, DataDomainHash,
	LocalchainAccountId, NewAccountOrigin, NotaryId, Note, NoteType, Notebook, NotebookHeader,
	NotebookNumber, TransferToLocalchainId, VoteMinimum, DATA_DOMAIN_LEASE_COST,
	ESCROW_CLAWBACK_TICKS, MINIMUM_ESCROW_SETTLEMENT, TAX_PERCENT_BASE,
};

pub use crate::error::VerifyError;

pub mod error;
#[cfg(test)]
mod test_balanceset;
#[cfg(test)]
mod test_notebook;

#[derive(Debug, Clone, PartialEq, TypeInfo, Encode, Decode, Serialize, Deserialize, Snafu)]
pub enum AccountHistoryLookupError {
	#[snafu(display("Notebook root not found"))]
	RootNotFound,
	#[snafu(display("Last change not found"))]
	LastChangeNotFound,
	#[snafu(display("Invalid transfer to localchain"))]
	InvalidTransferToLocalchain,
	#[snafu(display("The block given block specification could not be found"))]
	BlockSpecificationNotFound,
}

pub trait NotebookHistoryLookup {
	fn get_account_changes_root(
		&self,
		notary_id: NotaryId,
		notebook_number: NotebookNumber,
	) -> Result<H256, AccountHistoryLookupError>;
	fn get_last_changed_notebook(
		&self,
		notary_id: NotaryId,
		account_origin: AccountOrigin,
	) -> Result<NotebookNumber, AccountHistoryLookupError>;

	fn is_valid_transfer_to_localchain(
		&self,
		notary_id: NotaryId,
		transfer_id: TransferToLocalchainId,
		account_id: &AccountId,
		milligons: Balance,
		for_notebook_tick: Tick,
	) -> Result<bool, AccountHistoryLookupError>;
}

pub fn notebook_verify<T: NotebookHistoryLookup>(
	lookup: &T,
	notebook: &Notebook,
	vote_minimums: &BTreeMap<H256, VoteMinimum>,
	escrow_expiration_ticks: Tick,
) -> anyhow::Result<bool, VerifyError> {
	let mut state = NotebookVerifyState::default();

	state.load_new_origins(notebook.new_account_origins.to_vec())?;
	let header = &notebook.header;

	for notarization in notebook.notarizations.iter() {
		let changeset = &notarization.balance_changes;
		let block_votes = &notarization.block_votes;
		let data_domains = &notarization.data_domains;

		let result = verify_notarization_allocation(
			changeset,
			block_votes,
			data_domains,
			Some(notebook.header.tick),
			escrow_expiration_ticks,
		)?;
		result.verify_taxes()?;
		state.record_tax(result)?;
		verify_changeset_signatures(changeset)?;
		verify_balance_sources(lookup, &mut state, header, changeset)?;
		track_block_votes(&mut state, block_votes)?;
		verify_voting_sources(&state.escrow_data_domains, block_votes, vote_minimums)?;
	}

	ensure!(
		state.chain_transfers == header.chain_transfers.to_vec(),
		VerifyError::InvalidChainTransfersList
	);
	ensure!(
		BTreeSet::from_iter(header.changed_account_origins.to_vec()) == state.account_changelist,
		VerifyError::InvalidAccountChangelist
	);
	ensure!(state.tax == header.tax, VerifyError::InvalidHeaderTaxRecorded);
	ensure!(
		state.get_merkle_root() == header.changed_accounts_root,
		VerifyError::InvalidBalanceChangeRoot
	);
	ensure!(
		state.get_block_votes_root() == header.block_votes_root,
		VerifyError::InvalidBlockVoteRoot
	);
	ensure!(
		state.block_votes.len() == header.block_votes_count as usize,
		VerifyError::InvalidBlockVotesCount
	);
	ensure!(state.block_power == header.block_voting_power, VerifyError::InvalidBlockVotingPower);
	ensure!(
		state.blocks_voted_on == BTreeSet::from_iter(header.blocks_with_votes.clone()),
		VerifyError::InvalidBlockVoteList
	);

	ensure!(notebook.verify_hash(), VerifyError::InvalidNotebookHash);

	Ok(true)
}

#[derive(Clone, Default)]
struct NotebookVerifyState {
	account_changelist: BTreeSet<AccountOrigin>,
	final_balances: BTreeMap<LocalchainAccountId, BalanceTip>,
	chain_transfers: Vec<ChainTransfer>,
	/// Block votes is keyed off of account id and the index supplied by the user. If index is
	/// duplicated, only the last entry will be used.
	block_votes: BTreeMap<(AccountId, u32), BlockVote>,
	seen_transfers_in: BTreeSet<(AccountId, TransferToLocalchainId)>,
	new_account_origins: BTreeMap<LocalchainAccountId, AccountOriginUid>,
	escrow_data_domains: BTreeMap<(DataDomainHash, AccountId), u32>,
	blocks_voted_on: BTreeSet<H256>,
	block_power: u128,
	tax: u128,
}

impl NotebookVerifyState {
	pub fn track_final_balance(
		&mut self,
		key: &LocalchainAccountId,
		change: &BalanceChange,
		account_origin: AccountOrigin,
		escrow_hold_note: Option<Note>,
	) -> anyhow::Result<(), VerifyError> {
		self.account_changelist.insert(account_origin.clone());

		let tip = BalanceTip {
			account_id: change.account_id.clone(),
			account_type: change.account_type,
			change_number: change.change_number,
			balance: change.balance,
			account_origin,
			escrow_hold_note,
		};
		self.final_balances.insert(key.clone(), tip);
		Ok(())
	}

	pub fn record_tax(
		&mut self,
		change_state: BalanceChangesetState,
	) -> anyhow::Result<(), VerifyError> {
		for (_, amount) in change_state.tax_created_per_account {
			self.tax += amount;
		}
		Ok(())
	}
	pub fn load_new_origins(
		&mut self,
		origins: Vec<NewAccountOrigin>,
	) -> anyhow::Result<(), VerifyError> {
		let mut all_new_account_uids = BTreeSet::<AccountOriginUid>::new();
		for NewAccountOrigin { account_id, account_type, account_uid } in origins {
			self.new_account_origins
				.insert(LocalchainAccountId::new(account_id, account_type), account_uid);
			ensure!(
				all_new_account_uids.insert(account_uid),
				VerifyError::DuplicatedAccountOriginUid
			);
		}
		Ok(())
	}

	pub fn get_merkle_root(&self) -> H256 {
		let merkle_leafs = self.final_balances.values().map(|v| v.encode()).collect::<Vec<_>>();

		merkle_root::<BlakeTwo256, _>(merkle_leafs)
	}

	pub fn get_block_votes_root(&self) -> H256 {
		let merkle_leafs = self.block_votes.values().map(|v| v.encode()).collect::<Vec<_>>();

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
			NoteType::ClaimFromMainchain { transfer_id } => {
				ensure!(
					self.seen_transfers_in.insert((account_id.clone(), transfer_id,)),
					VerifyError::DuplicateChainTransfer
				);
				self.chain_transfers.push(ChainTransfer::ToLocalchain { transfer_id });
			},
			_ => {},
		}
		Ok(())
	}
}

fn track_block_votes(
	state: &mut NotebookVerifyState,
	block_votes: &Vec<BlockVote>,
) -> anyhow::Result<(), VerifyError> {
	for block_vote in block_votes {
		state.blocks_voted_on.insert(block_vote.block_hash);
		state
			.block_votes
			.insert((block_vote.account_id.clone(), block_vote.index), block_vote.clone());
		state.block_power = state.block_power.saturating_add(block_vote.power);
	}

	Ok(())
}

fn verify_balance_sources<T: NotebookHistoryLookup>(
	lookup: &T,
	state: &mut NotebookVerifyState,
	header: &NotebookHeader,
	changeset: &[BalanceChange],
) -> anyhow::Result<(), VerifyError> {
	let notary_id = header.notary_id;
	for change in changeset.iter() {
		let account_id = &change.account_id;
		let key = LocalchainAccountId::new(account_id.clone(), change.account_type);
		let mut escrow_hold_note = None;

		for note in &change.notes {
			// if this note is a chain transfer, track it in chain_transfers
			match &note.note_type {
				NoteType::SendToMainchain => {
					state.track_chain_transfer(account_id.clone(), note)?;
				},
				NoteType::ClaimFromMainchain { transfer_id } => {
					lookup.is_valid_transfer_to_localchain(
						notary_id,
						*transfer_id,
						account_id,
						note.milligons,
						header.tick,
					)?;
					state.track_chain_transfer(account_id.clone(), note)?;
				},
				NoteType::EscrowHold { .. } => {
					escrow_hold_note = Some(note.clone());
				},
				// this condition is redundant, but leaving for clarity
				NoteType::EscrowSettle => {
					if let Some(hold_note) = &change.escrow_hold_note {
						match &hold_note.note_type {
							NoteType::EscrowHold { data_domain_hash, recipient, .. } =>
								if let Some(data_domain_hash) = data_domain_hash {
									let count = state
										.escrow_data_domains
										.entry((*data_domain_hash, recipient.clone()))
										.or_insert(0);
									*count += 1u32;
								},
							_ => return Err(VerifyError::InvalidEscrowHoldNote),
						}
					}
					escrow_hold_note = None;
				},
				_ => {},
			}
		}

		if change.change_number == 1 {
			if let Some(account_uid) = state.new_account_origins.get(&key) {
				state.track_final_balance(
					&key,
					change,
					AccountOrigin {
						notebook_number: header.notebook_number,
						account_uid: *account_uid,
					},
					escrow_hold_note,
				)?;
			} else {
				return Err(VerifyError::MissingAccountOrigin {
					account_id: change.account_id.clone(),
					account_type: change.account_type,
				});
			}
		} else {
			let proof = change
				.previous_balance_proof
				.as_ref()
				.expect("Should have been unwrapped in verify_balance_changeset_allocation");
			verify_previous_balance_proof(
				lookup,
				proof,
				header.notebook_number,
				&mut state.final_balances,
				change,
				&key,
			)?;

			state.track_final_balance(
				&key,
				change,
				proof.account_origin.clone(),
				escrow_hold_note,
			)?;
		}
	}
	Ok(())
}

pub fn verify_voting_sources(
	escrow_data_domains: &BTreeMap<(DataDomainHash, AccountId), u32>,
	block_votes: &Vec<BlockVote>,
	vote_minimums: &BTreeMap<H256, VoteMinimum>,
) -> anyhow::Result<(), VerifyError> {
	let mut data_domain_tracker = escrow_data_domains.clone();
	for block_vote in block_votes {
		let minimum = vote_minimums
			.get(&block_vote.block_hash)
			.ok_or(VerifyError::InvalidBlockVoteSource)?;

		ensure!(block_vote.power >= *minimum, VerifyError::InsufficientBlockVoteMinimum);

		let count = data_domain_tracker
			.get_mut(&(block_vote.data_domain_hash, block_vote.data_domain_account.clone()))
			.ok_or(VerifyError::BlockVoteDataDomainMismatch)?;
		ensure!(*count > 0, VerifyError::BlockVoteEscrowReused);
		ensure!(
			block_vote.signature.verify(&block_vote.hash()[..], &block_vote.account_id),
			VerifyError::BlockVoteInvalidSignature
		);
		*count -= 1;
	}
	Ok(())
}

fn verify_previous_balance_proof<T: NotebookHistoryLookup>(
	lookup: &T,
	proof: &BalanceProof,
	notebook_number: NotebookNumber,
	final_balances: &mut BTreeMap<LocalchainAccountId, BalanceTip>,
	change: &BalanceChange,
	key: &LocalchainAccountId,
) -> anyhow::Result<bool, VerifyError> {
	// if we've changed balance in this notebook before, it must match the previous
	// entry
	if final_balances.contains_key(key) {
		let previous_balance = final_balances.get(key).unwrap();
		ensure!(proof.notebook_number == notebook_number, VerifyError::InvalidPreviousBalanceProof);
		let cited_balance = change.previous_balance_proof.as_ref().map(|a| a.balance).unwrap_or(0);
		ensure!(previous_balance.balance == cited_balance, VerifyError::InvalidPreviousBalance);
		ensure!(
			previous_balance.change_number == change.change_number - 1,
			VerifyError::InvalidPreviousNonce
		);
		ensure!(
			previous_balance.account_origin.account_uid == proof.account_origin.account_uid &&
				previous_balance.account_origin.notebook_number ==
					proof.account_origin.notebook_number,
			VerifyError::InvalidPreviousAccountOrigin
		);
		// if none, we can add changes.. if set, we can't do anything else
		ensure!(
			previous_balance.escrow_hold_note == change.escrow_hold_note,
			VerifyError::InvalidEscrowHoldNote
		);
	} else {
		let last_notebook_change =
			lookup.get_last_changed_notebook(proof.notary_id, proof.account_origin.clone())?;
		ensure!(
			last_notebook_change == proof.notebook_number,
			VerifyError::InvalidPreviousBalanceChangeNotebook
		);
		let Some(notebook_proof) = proof.notebook_proof.as_ref() else {
			return Err(VerifyError::MissingBalanceProof);
		};

		let root = lookup.get_account_changes_root(proof.notary_id, proof.notebook_number)?;
		let escrow_hold_note = change.escrow_hold_note.as_ref().cloned();

		let leaf = BalanceTip {
			account_id: change.account_id.clone(),
			account_type: change.account_type,
			balance: proof.balance,
			change_number: change.change_number - 1,
			account_origin: proof.account_origin.clone(),
			escrow_hold_note,
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

pub fn verify_changeset_signatures(changeset: &[BalanceChange]) -> anyhow::Result<(), VerifyError> {
	// Since this is a little more expensive, confirm signatures in a second pass
	for (index, change) in changeset.iter().enumerate() {
		// check that note id is valid for a hold note
		if let Some(escrow_note) = &change.escrow_hold_note {
			ensure!(
				matches!(escrow_note.note_type, NoteType::EscrowHold { .. }),
				VerifyError::InvalidEscrowHoldNote
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
	pub new_accounts: BTreeSet<LocalchainAccountId>,
	/// All escrow hold notes created per account (each account can only create one)
	pub accounts_with_new_holds: BTreeSet<LocalchainAccountId>,
	/// Whether or not the current notebook number is needed to confirm escrow settles
	pub needs_escrow_settle_followup: bool,
	/// How much in escrow funds was claimed by each account id
	pub claimed_escrow_deposits_per_account: BTreeMap<LocalchainAccountId, u128>,
	/// How much tax was sent per LocalchainAccountId
	pub tax_created_per_account: BTreeMap<LocalchainAccountId, u128>,
	/// How much was deposited per account
	pub claims_per_account: BTreeMap<LocalchainAccountId, u128>,

	/// How much was allocated to domains
	pub allocated_to_domains: u128,

	/// How much tax was sent per account to block votes
	unclaimed_block_vote_tax_per_account: BTreeMap<LocalchainAccountId, u128>,
	unclaimed_restricted_balance: BTreeMap<BTreeSet<LocalchainAccountId>, i128>,
	unclaimed_escrow_balances: BTreeMap<BTreeSet<LocalchainAccountId>, i128>,
}

impl BalanceChangesetState {
	pub fn verify_taxes(&self) -> anyhow::Result<(), VerifyError> {
		let mut tax_owed_per_account = BTreeMap::new();
		for (local_account_id, amount) in self.claims_per_account.iter() {
			if local_account_id.account_type == AccountType::Tax {
				continue;
			}
			let amount = *amount;
			let tax = Note::calculate_transfer_tax(amount);
			*tax_owed_per_account.entry(local_account_id).or_insert(0) += tax;
		}
		for (local_account_id, amount) in self.claimed_escrow_deposits_per_account.iter() {
			let tax = round_up(*amount, TAX_PERCENT_BASE);
			*tax_owed_per_account.entry(local_account_id).or_insert(0) += tax;
		}
		for (local_account_id, tax) in tax_owed_per_account {
			let tax_sent = self.tax_created_per_account.get(local_account_id).unwrap_or(&0);
			ensure!(
				*tax_sent >= tax,
				VerifyError::InsufficientTaxIncluded {
					account_id: local_account_id.account_id.clone(),
					tax_sent: *tax_sent,
					tax_owed: tax,
				}
			);
		}
		Ok(())
	}

	fn verify_note_claim_restrictions(&mut self) -> anyhow::Result<(), VerifyError> {
		for (claimer, amount) in self.claims_per_account.iter() {
			let mut balance = *amount as i128;
			self.unclaimed_restricted_balance.retain(|accounts, amount| {
				if balance == 0 {
					return true;
				}
				if accounts.contains(claimer) {
					if *amount >= balance {
						*amount -= balance;
						balance = 0;
					} else {
						balance -= *amount;
						*amount = 0;
					};
					return *amount != 0;
				}
				true
			});
		}
		ensure!(self.unclaimed_restricted_balance.is_empty(), VerifyError::InvalidNoteRecipients);
		Ok(())
	}

	fn verify_escrow_claim_restrictions(&mut self) -> anyhow::Result<(), VerifyError> {
		for (claimer, amount) in self.claimed_escrow_deposits_per_account.iter() {
			let mut balance = *amount as i128;
			self.unclaimed_escrow_balances.retain(|accounts, amount| {
				if balance == 0 {
					return true;
				}
				if accounts.contains(claimer) {
					if *amount >= balance {
						*amount -= balance;
						balance = 0;
					} else {
						balance -= *amount;
						*amount = 0;
					};
					return *amount != 0;
				}
				true
			});
		}

		ensure!(self.unclaimed_escrow_balances.is_empty(), VerifyError::InvalidEscrowClaimers);
		Ok(())
	}

	fn verify_change_number(
		&mut self,
		change: &BalanceChange,
		key: &LocalchainAccountId,
	) -> anyhow::Result<(), VerifyError> {
		ensure!(change.change_number > 0, VerifyError::InvalidBalanceChange);
		if change.change_number == 1 {
			self.new_accounts.insert(key.clone());

			ensure!(
				change.previous_balance_proof.is_none(),
				VerifyError::InvalidPreviousBalanceProof
			);
		}
		if change.change_number > 1 && !self.new_accounts.contains(key) {
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

		if !recipients.is_empty() {
			let mut set = BTreeSet::new();
			for rec in recipients {
				set.insert(LocalchainAccountId::new(rec.clone(), *account_type));
			}
			let entry = self.unclaimed_restricted_balance.entry(set.clone()).or_insert(0i128);
			*entry += milligons as i128;
		}
	}

	fn record_tax(
		&mut self,
		milligons: u128,
		claimer: &LocalchainAccountId,
	) -> anyhow::Result<(), VerifyError> {
		self.sent_tax += milligons;
		*self.tax_created_per_account.entry(claimer.clone()).or_insert(0) += milligons;

		Ok(())
	}

	fn record_tax_sent_to_vote(
		&mut self,
		milligons: u128,
		local_account_id: &LocalchainAccountId,
	) -> anyhow::Result<(), VerifyError> {
		*self
			.unclaimed_block_vote_tax_per_account
			.entry(local_account_id.clone())
			.or_insert(0) += milligons;

		Ok(())
	}

	fn used_tax_vote_amount(
		&mut self,
		milligons: u128,
		account_id: &LocalchainAccountId,
	) -> anyhow::Result<(), VerifyError> {
		let amount = self
			.unclaimed_block_vote_tax_per_account
			.get_mut(account_id)
			.ok_or(VerifyError::IneligibleTaxVoter)?;

		ensure!(*amount >= milligons, VerifyError::InsufficientBlockVoteTax);
		*amount -= milligons;
		if *amount == 0 {
			self.unclaimed_block_vote_tax_per_account.remove(account_id);
		}
		Ok(())
	}

	fn claim_balance(
		&mut self,
		milligons: u128,
		localchain_account_id: &LocalchainAccountId,
	) -> anyhow::Result<(), VerifyError> {
		if localchain_account_id.account_type == AccountType::Tax {
			self.claimed_tax += milligons;
		} else {
			self.claimed_deposits += milligons;
		}
		*self.claims_per_account.entry(localchain_account_id.clone()).or_insert(0) += milligons;

		Ok(())
	}

	fn claim_escrow_balance(
		&mut self,
		milligons: u128,
		claimer: &LocalchainAccountId,
	) -> anyhow::Result<(), VerifyError> {
		self.claimed_deposits += milligons;
		*self.claimed_escrow_deposits_per_account.entry(claimer.clone()).or_insert(0) += milligons;

		Ok(())
	}

	/// Records the escrow settles. If this is the second pass once we know a notebook number, it
	/// will also check if the escrow is ready to be claimed
	fn record_escrow_settle(
		&mut self,
		localchain_account_id: &LocalchainAccountId,
		milligons: i128,
		escrow_hold_note: &Note,
		expiration_tick: Tick,
		notebook_tick: Option<Tick>,
	) -> anyhow::Result<(), VerifyError> {
		let mut recipients = BTreeSet::new();

		// only add the recipient restrictions once we know what notebook we're in
		if let Some(tick) = notebook_tick {
			ensure!(
				tick >= expiration_tick,
				VerifyError::EscrowHoldNotReadyForClaim {
					current_tick: tick,
					claim_tick: expiration_tick
				}
			);

			let NoteType::EscrowHold { recipient, .. } = &escrow_hold_note.note_type else {
				return Err(VerifyError::InvalidEscrowHoldNote);
			};

			recipients.insert(LocalchainAccountId::new(recipient.clone(), AccountType::Deposit));
			if tick >= expiration_tick + ESCROW_CLAWBACK_TICKS {
				// no claim necessary for a 0 claim
				if milligons == 0 {
					recipients.clear();
				} else {
					recipients.insert(localchain_account_id.clone());
				}
			}
		} else {
			self.needs_escrow_settle_followup = true;
		}

		self.sent_deposits += milligons as u128;
		if !recipients.is_empty() {
			*self
				.unclaimed_escrow_balances
				.entry(BTreeSet::from_iter(recipients))
				.or_insert(0) += milligons;
		}
		Ok(())
	}
}

/// This function verifies the proposed balance changes PRIOR to accessing storage or verifying
/// proofs
/// 1. Confirm each proposed balance change adds up properly
/// 2. Confirm the changes net out to 0 (no funds are left outside an account)
///
/// Does NOT: lookup anything in storage, verify signatures, or confirm the merkle proofs
pub fn verify_notarization_allocation(
	changes: &[BalanceChange],
	block_votes: &[BlockVote],
	data_domains: &[(DataDomainHash, AccountId)],
	notebook_tick: Option<Tick>,
	escrow_expiration_ticks: Tick,
) -> anyhow::Result<BalanceChangesetState, VerifyError> {
	let mut state = BalanceChangesetState::default();

	for (change_index, change) in changes.iter().enumerate() {
		let localchain_account_id =
			LocalchainAccountId::new(change.account_id.clone(), change.account_type);
		state.verify_change_number(change, &localchain_account_id)?;

		let mut balance =
			change.previous_balance_proof.as_ref().map(|a| a.balance).unwrap_or_default() as i128;

		for (note_index, note) in (&change.notes).into_iter().enumerate() {
			if change.escrow_hold_note.is_some() &&
				!matches!(note.note_type, NoteType::EscrowSettle)
			{
				return Err(VerifyError::AccountLocked);
			}

			if localchain_account_id.is_tax() {
				match note.note_type {
					NoteType::Claim | NoteType::Send { .. } | NoteType::SendToVote => {},
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
					state.claim_balance(note.milligons, &localchain_account_id)?;
				},
				NoteType::EscrowHold { .. } => {
					ensure!(
						note.milligons >= MINIMUM_ESCROW_SETTLEMENT,
						VerifyError::InvalidEscrowHoldNote
					);
					// NOTE: a escrow doesn't change the source balance
					ensure!(
						change.escrow_hold_note.is_none() &&
							state.accounts_with_new_holds.insert(localchain_account_id.clone()),
						VerifyError::AccountAlreadyHasEscrowHold
					);
				},
				NoteType::EscrowClaim => {
					if note.milligons < MINIMUM_ESCROW_SETTLEMENT {
						return Err(VerifyError::EscrowNoteBelowMinimum);
					}
					state.claim_escrow_balance(note.milligons, &localchain_account_id)?;
				},
				NoteType::EscrowSettle => {
					let Some(source_change_tick) =
						change.previous_balance_proof.as_ref().map(|a| a.tick)
					else {
						return Err(VerifyError::MissingBalanceProof);
					};

					let escrow_hold_note = change
						.escrow_hold_note
						.as_ref()
						.ok_or(VerifyError::MissingEscrowHoldNote)?;

					state.record_escrow_settle(
						&localchain_account_id,
						note.milligons as i128,
						escrow_hold_note,
						source_change_tick + escrow_expiration_ticks,
						notebook_tick,
					)?;
				},
				NoteType::Tax => {
					ensure!(localchain_account_id.is_deposit(), VerifyError::InvalidTaxOperation);
					state.record_tax(note.milligons, &localchain_account_id)?;
				},
				NoteType::LeaseDomain => {
					ensure!(localchain_account_id.is_deposit(), VerifyError::InvalidTaxOperation);
					state.record_tax(note.milligons, &localchain_account_id)?;
					state.allocated_to_domains =
						state.allocated_to_domains.saturating_add(note.milligons);
				},
				NoteType::SendToVote { .. } => {
					ensure!(localchain_account_id.is_tax(), VerifyError::InvalidTaxOperation);
					state.record_tax_sent_to_vote(note.milligons, &localchain_account_id)?;
				},
				_ => {},
			}

			// track the balances moved in this note
			match note.note_type {
				NoteType::ClaimFromMainchain { .. } |
				NoteType::Claim { .. } |
				NoteType::EscrowClaim => {
					if let Some(new_balance) = balance.checked_add(note.milligons as i128) {
						balance = new_balance;
					} else {
						return Err(VerifyError::ExceededMaxBalance {
							balance: balance as u128,
							amount: note.milligons,
							note_index: note_index as u16,
							change_index: change_index as u16,
						});
					}
				},
				NoteType::SendToMainchain |
				NoteType::Send { .. } |
				NoteType::EscrowSettle |
				NoteType::LeaseDomain |
				NoteType::Tax |
				NoteType::SendToVote => balance -= note.milligons as i128,
				_ => {},
			};
		}

		ensure!(
			balance == change.balance as i128,
			VerifyError::BalanceChangeMismatch {
				change_index: change_index as u16,
				provided_balance: change.balance,
				calculated_balance: balance,
			}
		);
	}

	ensure!(
		state.allocated_to_domains == DATA_DOMAIN_LEASE_COST * data_domains.len() as u128,
		VerifyError::InvalidDomainLeaseAllocation
	);

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

	state.verify_note_claim_restrictions()?;
	state.verify_escrow_claim_restrictions()?;

	for block_vote in block_votes {
		state.used_tax_vote_amount(
			block_vote.power,
			&LocalchainAccountId::new(block_vote.account_id.clone(), AccountType::Tax),
		)?;
	}
	ensure!(
		state.unclaimed_block_vote_tax_per_account.is_empty(),
		VerifyError::InvalidBlockVoteAllocation
	);

	Ok(state)
}
