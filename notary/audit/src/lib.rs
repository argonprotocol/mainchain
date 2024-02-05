#![cfg_attr(not(feature = "std"), no_std)]

use binary_merkle_tree::{merkle_root, verify_proof, Leaf};
use codec::{Decode, Encode};
use log::trace;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use sp_core::{crypto::AccountId32, H256};
use sp_runtime::{
	scale_info::TypeInfo,
	traits::{BlakeTwo256, Verify},
};
use sp_std::{
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	vec::Vec,
};

use ulx_primitives::{
	ensure, round_up, tick::Tick, AccountId, AccountOrigin, AccountOriginUid, AccountType, Balance,
	BalanceChange, BalanceProof, BalanceTip, BlockVote, ChainTransfer, DataDomainHash,
	NewAccountOrigin, NotaryId, Note, NoteType, Notebook, NotebookHeader, NotebookNumber,
	VoteMinimum, CHANNEL_CLAWBACK_TICKS, CHANNEL_EXPIRATION_TICKS, DATA_DOMAIN_LEASE_COST,
	MIN_CHANNEL_NOTE_MILLIGONS, TAX_PERCENT_BASE,
};

pub use crate::error::VerifyError;

pub mod error;
#[cfg(test)]
mod test_balanceset;
#[cfg(test)]
mod test_notebook;

const LOG_TARGET: &str = "notary::audit";

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
		account_id: &AccountId32,
		nonce: u32,
		milligons: Balance,
	) -> Result<bool, AccountHistoryLookupError>;
}

pub fn notebook_verify<T: NotebookHistoryLookup>(
	lookup: &T,
	notebook: &Notebook,
	vote_minimums: &BTreeMap<H256, VoteMinimum>,
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
		)?;
		result.verify_taxes()?;
		state.record_tax(result)?;
		verify_changeset_signatures(&changeset)?;
		verify_balance_sources(lookup, &mut state, header, changeset)?;
		track_block_votes(&mut state, block_votes)?;
		verify_voting_sources(&state.channel_data_domains, block_votes, &vote_minimums)?;
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
	final_balances: BTreeMap<(AccountId32, AccountType), BalanceTip>,
	chain_transfers: Vec<ChainTransfer>,
	/// Block votes is keyed off of account id and the index supplied by the user. If index is
	/// duplicated, only the last entry will be used.
	block_votes: BTreeMap<(AccountId, u32), BlockVote>,
	seen_transfers_in: BTreeSet<(AccountId32, u32)>,
	new_account_origins: BTreeMap<(AccountId, AccountType), AccountOriginUid>,
	channel_data_domains: BTreeMap<(DataDomainHash, AccountId), u32>,
	blocks_voted_on: BTreeSet<H256>,
	block_power: u128,
	tax: u128,
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

	pub fn get_block_votes_root(&self) -> H256 {
		let merkle_leafs = self.block_votes.iter().map(|(_, v)| v.encode()).collect::<Vec<_>>();

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

fn track_block_votes(
	state: &mut NotebookVerifyState,
	block_votes: &Vec<BlockVote>,
) -> anyhow::Result<(), VerifyError> {
	for block_vote in block_votes {
		state.blocks_voted_on.insert(block_vote.block_hash.clone());
		state
			.block_votes
			.insert((block_vote.account_id.clone(), block_vote.index), block_vote.clone());
		state.block_power = state.block_power.saturating_add(block_vote.power);
	}

	Ok(())
}

fn verify_balance_sources<'a, T: NotebookHistoryLookup>(
	lookup: &T,
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
					lookup.is_valid_transfer_to_localchain(
						notary_id,
						account_id,
						account_nonce.clone(),
						note.milligons,
					)?;
					state.track_chain_transfer(account_id.clone(), note)?;
				},
				NoteType::ChannelHold { .. } => {
					channel_hold_note = Some(note.clone());
				},
				// this condition is redundant, but leaving for clarity
				NoteType::ChannelSettle => {
					if let Some(hold_note) = &change.channel_hold_note {
						match &hold_note.note_type {
							&NoteType::ChannelHold { ref data_domain_hash, ref recipient } =>
								if let Some(data_domain_hash) = data_domain_hash {
									let count = state
										.channel_data_domains
										.entry((data_domain_hash.clone(), recipient.clone()))
										.or_insert(0);
									*count += 1u32;
								},
							_ => return Err(VerifyError::InvalidChannelHoldNote),
						}
					}
					channel_hold_note = None;
				},
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
			verify_previous_balance_proof(
				lookup,
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

pub fn verify_voting_sources(
	channel_data_domains: &BTreeMap<(DataDomainHash, AccountId), u32>,
	block_votes: &Vec<BlockVote>,
	vote_minimums: &BTreeMap<H256, VoteMinimum>,
) -> anyhow::Result<(), VerifyError> {
	let mut data_domain_tracker = channel_data_domains.clone();
	for block_vote in block_votes {
		let minimum = vote_minimums
			.get(&block_vote.block_hash)
			.ok_or(VerifyError::InvalidBlockVoteSource)?;

		ensure!(block_vote.power >= *minimum, VerifyError::InsufficientBlockVoteMinimum);

		let count = data_domain_tracker
			.get_mut(&(block_vote.data_domain_hash.clone(), block_vote.data_domain_account.clone()))
			.ok_or(VerifyError::BlockVoteDataDomainMismatch)?;
		ensure!(*count > 0, VerifyError::BlockVoteChannelReused);
		ensure!(
			block_vote.signature.verify(&block_vote.hash()[..], &block_vote.account_id),
			VerifyError::BlockVoteInvalidSignature
		);
		*count -= 1;
	}
	Ok(())
}

fn verify_previous_balance_proof<'a, T: NotebookHistoryLookup>(
	lookup: &T,
	proof: &BalanceProof,
	notebook_number: NotebookNumber,
	final_balances: &mut BTreeMap<(AccountId32, AccountType), BalanceTip>,
	change: &BalanceChange,
	key: &(AccountId32, AccountType),
) -> anyhow::Result<bool, VerifyError> {
	// NOTE: something about adding logging is making lookups work for localchain transfers. if I
	// comment out the logging, it fails with InvalidLocalchainTransfer in the notebook_closer
	// test.
	trace!(target:LOG_TARGET,
		"Verify balance for uid={}. In current set of balance changes? {}",
		proof.account_origin.account_uid,
		final_balances.contains_key(&key)
	);
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
			previous_balance.account_origin.account_uid == proof.account_origin.account_uid &&
				previous_balance.account_origin.notebook_number ==
					proof.account_origin.notebook_number,
			VerifyError::InvalidPreviousAccountOrigin
		);
		// if none, we can add changes.. if set, we can't do anything else
		ensure!(
			previous_balance.channel_hold_note == change.channel_hold_note,
			VerifyError::InvalidChannelHoldNote
		);
	} else {
		let last_notebook_change =
			lookup.get_last_changed_notebook(proof.notary_id, proof.account_origin.clone())?;
		ensure!(
			last_notebook_change == proof.notebook_number,
			VerifyError::InvalidPreviousBalanceChangeNotebook
		);
		let Some(notebook_proof) = proof.notebook_proof.as_ref() else {
			return Err(VerifyError::MissingBalanceProof)
		};

		let root = lookup.get_account_changes_root(proof.notary_id, proof.notebook_number)?;
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
	pub tax_created_per_account: BTreeMap<AccountId, u128>,
	/// How much was deposited per account
	pub claimed_deposits_per_account: BTreeMap<AccountId, u128>,

	/// How much was allocated to domains
	pub allocated_to_domains: u128,

	/// How much tax was sent per account to block seals
	unclaimed_block_vote_tax_per_account: BTreeMap<AccountId, u128>,
	unclaimed_restricted_balance: BTreeMap<BTreeSet<(AccountId, AccountType)>, i128>,
	unclaimed_channel_balances: BTreeMap<BTreeSet<AccountId>, i128>,
}

impl BalanceChangesetState {
	pub fn verify_taxes(&self) -> anyhow::Result<(), VerifyError> {
		let mut tax_owed_per_account = BTreeMap::new();
		for (account_id, amount) in self.claimed_deposits_per_account.iter() {
			let amount = *amount;
			let tax = Note::calculate_transfer_tax(amount);
			*tax_owed_per_account.entry(account_id).or_insert(0) += tax;
		}
		for (account_id, amount) in self.claimed_channel_deposits_per_account.iter() {
			let tax = round_up(*amount, TAX_PERCENT_BASE);
			*tax_owed_per_account.entry(account_id).or_insert(0) += tax;
		}
		for (account_id, tax) in tax_owed_per_account {
			let tax_sent = self.tax_created_per_account.get(&account_id).unwrap_or(&0);
			ensure!(
				*tax_sent >= tax,
				VerifyError::InsufficientTaxIncluded {
					account_id: account_id.clone(),
					tax_sent: *tax_sent,
					tax_owed: tax,
				}
			);
		}
		Ok(())
	}

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
			self.unclaimed_restricted_balance.insert(set, milligons as i128);
		}
	}

	fn record_tax(
		&mut self,
		milligons: u128,
		claimer: &AccountId,
	) -> anyhow::Result<(), VerifyError> {
		self.sent_tax += milligons;
		*self.tax_created_per_account.entry(claimer.clone()).or_insert(0) += milligons;

		Ok(())
	}

	fn record_tax_sent_to_vote(
		&mut self,
		milligons: u128,
		account_id: &AccountId,
	) -> anyhow::Result<(), VerifyError> {
		*self.unclaimed_block_vote_tax_per_account.entry(account_id.clone()).or_insert(0) +=
			milligons;

		Ok(())
	}

	fn used_tax_vote_amount(
		&mut self,
		milligons: u128,
		account_id: &AccountId,
	) -> anyhow::Result<(), VerifyError> {
		let amount = self
			.unclaimed_block_vote_tax_per_account
			.get_mut(account_id)
			.ok_or(VerifyError::InsufficientBlockVoteTax)?;

		ensure!(*amount >= milligons, VerifyError::InsufficientBlockVoteTax);
		*amount = amount.saturating_sub(milligons);
		if *amount == 0 {
			self.unclaimed_block_vote_tax_per_account.remove(account_id);
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
			*self.claimed_deposits_per_account.entry(claimer.clone()).or_insert(0) += milligons;
			self.claimed_deposits += milligons;
		}

		let key = (claimer.clone(), account_type.clone());
		self.unclaimed_restricted_balance.retain(|accounts, amount| {
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
		*self.claimed_channel_deposits_per_account.entry(claimer.clone()).or_insert(0) += milligons;

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
		source_change_tick: Tick,
		notebook_tick: Option<Tick>,
	) -> anyhow::Result<(), VerifyError> {
		let mut recipients = BTreeSet::new();

		// only add the recipient restrictions once we know what notebook we're in
		if let Some(tick) = notebook_tick {
			let expiration_tick = source_change_tick + CHANNEL_EXPIRATION_TICKS;
			ensure!(
				tick >= expiration_tick,
				VerifyError::ChannelHoldNotReadyForClaim {
					current_tick: tick,
					claim_tick: expiration_tick
				}
			);

			let NoteType::ChannelHold { recipient, .. } = &channel_hold_note.note_type else {
				return Err(VerifyError::InvalidChannelHoldNote)
			};

			recipients.insert(recipient.clone());
			if tick >= expiration_tick + CHANNEL_CLAWBACK_TICKS {
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

/// This function verifies the proposed balance changes PRIOR to accessing storage or verifying
/// proofs
/// 1. Confirm each proposed balance change adds up properly
/// 2. Confirm the changes net out to 0 (no funds are left outside an account)
///
/// Does NOT: lookup anything in storage, verify signatures, or confirm the merkle proofs
pub fn verify_notarization_allocation(
	changes: &Vec<BalanceChange>,
	block_votes: &Vec<BlockVote>,
	data_domains: &Vec<(DataDomainHash, AccountId)>,
	notebook_tick: Option<Tick>,
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
				!matches!(note.note_type, NoteType::ChannelSettle)
			{
				return Err(VerifyError::AccountLocked)
			}

			if change.account_type == AccountType::Tax {
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
					let Some(source_change_tick) =
						change.previous_balance_proof.as_ref().map(|a| a.tick)
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
						source_change_tick,
						notebook_tick,
					)?;
				},
				NoteType::Tax => {
					ensure!(
						change.account_type == AccountType::Deposit,
						VerifyError::InvalidTaxOperation
					);
					state.record_tax(note.milligons, &change.account_id)?;
				},
				NoteType::LeaseDomain => {
					ensure!(
						change.account_type == AccountType::Deposit,
						VerifyError::InvalidTaxOperation
					);
					state.record_tax(note.milligons, &change.account_id)?;
					state.allocated_to_domains =
						state.allocated_to_domains.saturating_add(note.milligons);
				},
				NoteType::SendToVote { .. } => {
					ensure!(
						change.account_type == AccountType::Tax,
						VerifyError::InvalidTaxOperation
					);
					state.record_tax_sent_to_vote(note.milligons, &change.account_id)?;
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
				NoteType::LeaseDomain |
				NoteType::Tax |
				NoteType::SendToVote => balance -= note.milligons as i128,
				_ => {},
			};
			note_index += 1;
		}

		ensure!(
			balance == change.balance as i128,
			VerifyError::BalanceChangeMismatch {
				change_index,
				provided_balance: change.balance,
				calculated_balance: balance,
			}
		);
		change_index += 1;
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
	// this works by removing all restricted balances as the approved users draw from them
	ensure!(state.unclaimed_restricted_balance.is_empty(), VerifyError::InvalidNoteRecipients);
	ensure!(state.unclaimed_channel_balances.is_empty(), VerifyError::InvalidChannelClaimers);

	for block_vote in block_votes {
		state.used_tax_vote_amount(block_vote.power, &block_vote.account_id)?;
	}
	ensure!(
		state.unclaimed_block_vote_tax_per_account.is_empty(),
		VerifyError::InvalidBlockVoteAllocation
	);

	Ok(state)
}
