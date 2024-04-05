use std::{
	cmp::Ordering,
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	sync::Arc,
};

use codec::{Decode, Encode};
use log::info;
use parking_lot::Mutex;
use sc_client_api::{self, backend::AuxStore};
use sc_consensus::BlockImportParams;
use sp_core::{H256, U256};
use sp_runtime::traits::{Block as BlockT, Header};

use ulx_node_runtime::{AccountId, BlockNumber, NotebookVerifyError};
use ulx_primitives::{
	notary::{NotaryNotebookTickState, NotaryNotebookVoteDetails, NotaryNotebookVoteDigestDetails},
	tick::Tick,
	BlockSealDigest, BlockVotingPower, ComputeDifficulty, NotaryId, NotaryNotebookVotes,
	NotebookDigestRecord, NotebookHeaderData, NotebookNumber,
};

use crate::{convert_u32, error::Error};

pub struct UlxAux<B: BlockT, C: AuxStore> {
	client: Arc<C>,
	_block: std::marker::PhantomData<B>,
	pub lock: Arc<Mutex<()>>,
	missing_notebook_lock: Arc<Mutex<()>>,
}

impl<B: BlockT, C: AuxStore> Clone for UlxAux<B, C> {
	fn clone(&self) -> Self {
		Self {
			client: self.client.clone(),
			_block: Default::default(),
			lock: self.lock.clone(),
			missing_notebook_lock: self.missing_notebook_lock.clone(),
		}
	}
}

impl<B: BlockT, C: AuxStore> UlxAux<B, C> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_block: Default::default(),
			lock: Default::default(),
			missing_notebook_lock: Default::default(),
		}
	}
}

///
/// Stores auxiliary data for Ulx consensus (eg - cross block data)
///
/// We store several types of data
/// - `ForkPower` - stored at each block to determine the aggregate voting power for a fork
///   (++voting_power, --nonce)
/// - `BlockVotes` - all block votes submitted (voting for a block hash)
/// - `StrongestVoteAtHeight` - the strongest vote at a given height - helps determine if we should
///   create a block
/// - `AuthorsAtHeight` - the authors at a given height for every voting key. A block will only be
///   accepted once per author per key
impl<B: BlockT, C: AuxStore> UlxAux<B, C> {
	pub fn record_block(
		&self,
		best_header: B::Header,
		block: &mut BlockImportParams<B>,
		author: AccountId,
		voting_key: Option<H256>,
		notebooks: u32,
		tick: Tick,
		block_voting_power: BlockVotingPower,
		seal_digest: BlockSealDigest,
		compute_difficulty: Option<ComputeDifficulty>,
	) -> Result<(ForkPower, ForkPower), Error<B>> {
		let _lock = self.lock.lock();
		let block_number = convert_u32::<B>(&block.header.number());
		let strongest_at_height = self.strongest_fork_at_tick(tick)?;

		// add author to voting key
		if let Some(voting_key) = voting_key {
			let mut authors_at_height = self.authors_by_voting_key_at_height(block_number)?;
			let is_new_entry = authors_at_height
				.entry(voting_key)
				.or_insert_with(BTreeSet::new)
				.insert(author.clone());
			if !is_new_entry {
				return Err(Error::<B>::DuplicateAuthoredBlock(author).into());
			}
		}

		let parent_hash = block.header.parent_hash();
		let mut fork_power = self.get_fork_voting_power(&parent_hash)?;
		fork_power.add(block_voting_power, notebooks, seal_digest, compute_difficulty);

		if fork_power > strongest_at_height {
			let key = get_strongest_fork_at_tick_key(tick);
			block.auxiliary.push((key, Some(fork_power.encode())));
		}

		let best_header_fork_power = self.get_fork_voting_power(&best_header.hash())?;

		block.auxiliary.push((
			get_fork_voting_power_aux_key::<B>(&block.post_hash()),
			Some(fork_power.encode()),
		));

		// cleanup old votes (None deletes)
		if tick >= 10 {
			let cleanup_height = tick - 10;
			block.auxiliary.push((get_votes_key(cleanup_height), None));
			block.auxiliary.push((get_strongest_fork_at_tick_key(cleanup_height), None));
			block
				.auxiliary
				.push((get_authors_at_height_key(block_number.saturating_sub(5)), None));
		}
		// Cleanup old notary state. We keep this longer because we might need to catchup on notebooks
		if tick >= 256 {
			block.auxiliary.push((notary_state_key(tick - 256), None));
		}

		Ok((fork_power, best_header_fork_power))
	}

	pub fn get_notary_notebooks_for_header(
		&self,
		notary_id: NotaryId,
		latest_runtime_notebook_number: NotebookNumber,
		submitting_tick: Tick,
	) -> Result<
		(
			NotebookHeaderData<NotebookVerifyError, BlockNumber>,
			Option<NotaryNotebookVoteDigestDetails>,
		),
		Error<B>,
	> {
		let mut headers = NotebookHeaderData::default();
		let mut tick_notebook = None;
		let audit_results = self.get_notary_audit_history(notary_id)?;

		for notebook in audit_results {
			if notebook.notebook_number <= latest_runtime_notebook_number
				|| notebook.tick > submitting_tick
			{
				continue;
			}
			let tick = notebook.tick;

			let state = self.get_notebook_tick_state(tick).unwrap_or_default();
			if tick == submitting_tick {
				let details =
					state.notebook_key_details_by_notary.get(&notary_id).ok_or_else(|| {
						Error::NotebookHeaderBuildError(format!(
							"Unable to find notebook  #{} key details for notary {} at tick {}",
							notebook.notebook_number, notary_id, tick
						))
					})?;
				tick_notebook = Some(details.clone());
				headers.latest_finalized_block_needed =
					state.latest_finalized_block_needed.max(headers.latest_finalized_block_needed);
			}
			if let Some(raw_data) = state.raw_headers_by_notary.get(&notary_id) {
				headers.signed_headers.push(raw_data.clone());
				headers.notebook_digest.notebooks.push(NotebookDigestRecord {
					notary_id,
					notebook_number: notebook.notebook_number,
					tick,
					audit_first_failure: notebook.first_error_reason.clone(),
				});
			} else {
				return Err(Error::NotebookHeaderBuildError(format!(
					"Unable to find notebook #{} for notary {} at tick {}",
					notebook.notebook_number, notary_id, tick
				)));
			}
		}
		Ok((headers, tick_notebook))
	}

	pub fn get_missing_notebooks(
		&self,
		notary_id: NotaryId,
	) -> Result<Vec<NotebookNumber>, Error<B>> {
		let key = notary_missing_notebooks_key(notary_id);
		Ok(match self.client.get_aux(&key)? {
			Some(bytes) => <Vec<NotebookNumber>>::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		})
	}

	pub fn update_missing_notebooks(
		&self,
		notary_id: NotaryId,
		new_missing: Vec<NotebookNumber>,
		remove_missing: Vec<NotebookNumber>,
	) -> Result<(), Error<B>> {
		let lock = self.missing_notebook_lock.lock();
		let mut missing = self.get_missing_notebooks(notary_id)?;
		for notebook in new_missing {
			if !missing.contains(&notebook) {
				missing.push(notebook);
			}
		}
		missing.retain(|x| !remove_missing.contains(x));
		let key = notary_missing_notebooks_key(notary_id);
		self.client.insert_aux(&[(key.as_slice(), missing.encode().as_slice())], &[])?;
		drop(lock);
		Ok(())
	}

	pub fn get_notary_audit_history(
		&self,
		notary_id: NotaryId,
	) -> Result<Vec<NotebookAuditResult>, Error<B>> {
		let key = notary_notebooks_key(notary_id);
		Ok(match self.client.get_aux(&key)? {
			Some(bytes) => <Vec<NotebookAuditResult>>::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		})
	}

	pub fn authors_by_voting_key_at_height(
		&self,
		block_number: BlockNumber,
	) -> Result<BTreeMap<H256, BTreeSet<AccountId>>, Error<B>> {
		let state_key = get_authors_at_height_key(block_number);
		let aux = match self.client.get_aux(&state_key)? {
			Some(bytes) => BTreeMap::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		Ok(aux)
	}

	/// Retrieves aggregate voting power for a fork
	pub fn get_fork_voting_power(&self, block_hash: &B::Hash) -> Result<ForkPower, Error<B>> {
		let key = get_fork_voting_power_aux_key::<B>(block_hash);
		let aux = match self.client.get_aux(&key)? {
			Some(bytes) => ForkPower::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		Ok(aux)
	}

	pub fn store_votes(
		&self,
		tick: Tick,
		votes: NotaryNotebookVotes,
	) -> Result<Vec<NotaryNotebookVotes>, Error<B>> {
		let mut existing = self.get_votes(tick)?;

		if !existing
			.iter()
			.any(|x| x.notary_id == votes.notary_id && x.notebook_number == votes.notebook_number)
		{
			existing.push(votes);
		}

		let key = get_votes_key(tick);
		self.client.insert_aux(&[(key.as_slice(), existing.encode().as_slice())], &[])?;
		Ok(existing)
	}

	pub fn get_votes(&self, tick: Tick) -> Result<Vec<NotaryNotebookVotes>, Error<B>> {
		let key = get_votes_key(tick);
		Ok(match self.client.get_aux(&key)? {
			Some(bytes) => <Vec<NotaryNotebookVotes>>::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		})
	}

	pub fn strongest_fork_at_tick(&self, tick: Tick) -> Result<ForkPower, Error<B>> {
		let state_key = get_strongest_fork_at_tick_key(tick);
		let aux = match self.client.get_aux(&state_key)? {
			Some(bytes) => ForkPower::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		Ok(aux)
	}

	fn get_notebook_tick_state(&self, tick: Tick) -> Result<NotaryNotebookTickState, Error<B>> {
		let state_key = notary_state_key(tick);
		let notary_state = match self.client.get_aux(&state_key)? {
			Some(bytes) => NotaryNotebookTickState::decode(&mut &bytes[..]).unwrap_or_default(),
			None => Default::default(),
		};
		Ok(notary_state)
	}

	pub fn store_notebook_result(
		&self,
		notary_id: NotaryId,
		audit_result: NotebookAuditResult,
		raw_signed_header: Vec<u8>,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<NotaryNotebookTickState, Error<B>> {
		let lock = self.lock.lock();
		let notary_state = self.update_tick_state(raw_signed_header, &vote_details)?;
		let mut validated_notebooks = self.get_notary_audit_history(notary_id)?;
		if !validated_notebooks
			.iter()
			.any(|n| n.notebook_number == audit_result.notebook_number)
		{
			validated_notebooks.push(audit_result);
			self.store_notebook_audit_result(notary_id, validated_notebooks)?;
		}
		drop(lock);
		Ok(notary_state)
	}

	fn store_notebook_audit_result(
		&self,
		notary_id: NotaryId,
		validated_notebooks: Vec<NotebookAuditResult>,
	) -> Result<(), Error<B>> {
		let key = notary_notebooks_key(notary_id);

		self.client
			.insert_aux(&[(key.as_slice(), validated_notebooks.encode().as_slice())], &[])?;

		Ok(())
	}

	fn update_tick_state(
		&self,
		raw_signed_header: Vec<u8>,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<NotaryNotebookTickState, Error<B>> {
		let tick = vote_details.tick;
		let notary_id = vote_details.notary_id;
		let mut state = self.get_notebook_tick_state(tick)?;

		state.latest_finalized_block_needed =
			state.latest_finalized_block_needed.max(vote_details.finalized_block_number);

		let vote_details = NotaryNotebookVoteDigestDetails::from(vote_details);

		info!(
			"Storing vote details for tick {} and notary {} at notebook #{}",
			tick, notary_id, vote_details.notebook_number
		);

		if state.notebook_key_details_by_notary.insert(notary_id, vote_details).is_none() {
			state.raw_headers_by_notary.insert(notary_id, raw_signed_header);

			self.update_notebook_tick_state(tick, state.clone())?;
		}
		Ok(state)
	}

	fn update_notebook_tick_state(
		&self,
		tick: Tick,
		notary_state: NotaryNotebookTickState,
	) -> Result<(), Error<B>> {
		let state_key = notary_state_key(tick);
		self.client
			.insert_aux(&[(state_key.as_slice(), notary_state.encode().as_slice())], &[])?;
		Ok(())
	}
}

fn notary_state_key(tick: Tick) -> Vec<u8> {
	("NotaryStateAtTick", tick).encode()
}

fn get_authors_at_height_key(block_number: BlockNumber) -> Vec<u8> {
	("AuthorsAtHeight", block_number).encode()
}

fn notary_notebooks_key(notary_id: NotaryId) -> Vec<u8> {
	("NotaryNotebooksFor", notary_id).encode()
}

fn notary_missing_notebooks_key(notary_id: NotaryId) -> Vec<u8> {
	("MissingNotebooksFor", notary_id).encode()
}

fn get_votes_key(tick: Tick) -> Vec<u8> {
	("VotesAtTick", tick).encode()
}
fn get_fork_voting_power_aux_key<B: BlockT>(block_hash: &B::Hash) -> Vec<u8> {
	("ForkVotingPower", block_hash).encode()
}
fn get_strongest_fork_at_tick_key(tick: Tick) -> Vec<u8> {
	("MaxVotingPowerAtTick", tick).encode()
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq)]
pub struct ForkPower {
	pub notebooks: u64,
	pub voting_power: U256,
	pub seal_strength: U256,
	pub total_compute_difficulty: U256,
	pub vote_created_blocks: u128,
}

impl ForkPower {
	pub fn add(
		&mut self,
		block_voting_power: BlockVotingPower,
		notebooks: u32,
		seal_digest: BlockSealDigest,
		compute_difficulty: Option<ComputeDifficulty>,
	) {
		match seal_digest {
			BlockSealDigest::Vote { seal_strength } => {
				self.add_vote(block_voting_power, notebooks, seal_strength);
			},
			BlockSealDigest::Compute { .. } => {
				self.add_compute(
					block_voting_power,
					notebooks,
					compute_difficulty.unwrap_or_default(),
				);
			},
		}
	}

	pub fn add_vote(
		&mut self,
		block_voting_power: BlockVotingPower,
		notebooks: u32,
		seal_strength: U256,
	) {
		self.seal_strength = self.seal_strength.saturating_add(seal_strength);
		self.vote_created_blocks = self.vote_created_blocks.saturating_add(1);
		self.voting_power = self.voting_power.saturating_add(U256::from(block_voting_power));
		self.notebooks = self.notebooks.saturating_add(notebooks as u64);
	}

	pub fn add_compute(
		&mut self,
		block_voting_power: BlockVotingPower,
		notebooks: u32,
		compute_difficulty: ComputeDifficulty,
	) {
		self.voting_power = self.voting_power.saturating_add(U256::from(block_voting_power));
		self.notebooks = self.notebooks.saturating_add(notebooks as u64);
		self.total_compute_difficulty =
			self.total_compute_difficulty.saturating_add(compute_difficulty.into());
	}
}

impl Default for ForkPower {
	fn default() -> Self {
		Self {
			voting_power: U256::zero(),
			notebooks: 0,
			seal_strength: U256::MAX,
			total_compute_difficulty: U256::zero(),
			vote_created_blocks: 0,
		}
	}
}

impl PartialOrd for ForkPower {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		let mut cmp = self.notebooks.cmp(&other.notebooks);
		if cmp == Ordering::Equal {
			cmp = self.voting_power.cmp(&other.voting_power);
		}
		if cmp == Ordering::Equal {
			// count forks with tax votes over compute
			cmp = self.vote_created_blocks.cmp(&other.vote_created_blocks);
		}
		if cmp == Ordering::Equal {
			// smaller vote proof is better
			cmp = other.seal_strength.cmp(&self.seal_strength)
		}
		if cmp == Ordering::Equal {
			cmp = self.total_compute_difficulty.cmp(&other.total_compute_difficulty)
		}
		Some(cmp)
	}
}

pub type IsValidNotebook = bool;
#[derive(Clone, Encode, Decode, Debug)]
pub struct NotebookAuditResult {
	pub notebook_number: NotebookNumber,
	pub tick: Tick,
	pub is_valid: IsValidNotebook,
	pub first_error_reason: Option<NotebookVerifyError>,
	pub body_hash: [u8; 32],
}

#[cfg(test)]
mod test {
	use crate::aux_client::ForkPower;

	#[test]
	fn it_should_compare_fork_power() {
		assert_eq!(ForkPower::default(), ForkPower::default());

		assert!(
			ForkPower { voting_power: 1.into(), ..Default::default() }
				> ForkPower { voting_power: 0.into(), ..Default::default() }
		);

		assert!(
			ForkPower { notebooks: 1, ..Default::default() }
				> ForkPower { notebooks: 0, ..Default::default() }
		);

		assert!(
			ForkPower { seal_strength: 200.into(), ..Default::default() }
				> ForkPower { seal_strength: 201.into(), ..Default::default() }
		);

		assert!(
			ForkPower { total_compute_difficulty: 1000.into(), ..Default::default() }
				> ForkPower { total_compute_difficulty: 999.into(), ..Default::default() }
		);
	}
}
