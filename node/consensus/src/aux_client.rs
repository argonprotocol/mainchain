#![allow(clippy::type_complexity)]
use std::{
	any::Any,
	cmp::Ordering,
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	sync::Arc,
};

use codec::{Decode, Encode};
use log::info;
use parking_lot::RwLock;
use sc_client_api::{self, backend::AuxStore};
use sc_consensus::BlockImportParams;
use schnellru::{ByLength, LruMap};
use sp_arithmetic::traits::UniqueSaturatedInto;
use sp_core::{H256, U256};
use sp_runtime::traits::{Block as BlockT, Header};

use ulx_node_runtime::{AccountId, BlockNumber, NotebookVerifyError};
use ulx_primitives::{
	notary::{NotaryNotebookTickState, NotaryNotebookVoteDetails, NotaryNotebookVoteDigestDetails},
	tick::Tick,
	BlockSealDigest, BlockVotingPower, ComputeDifficulty, NotaryId, NotaryNotebookVotes,
	NotebookAuditSummary, NotebookDigestRecord, NotebookHeaderData, NotebookNumber,
};

use crate::{aux_data::AuxData, error::Error};

pub enum AuxState<C: AuxStore> {
	NotaryStateAtTick(Arc<AuxData<NotaryNotebookTickState, C>>),
	AuthorsAtHeight(Arc<AuxData<BTreeMap<H256, BTreeSet<AccountId>>, C>>),
	NotaryNotebooks(Arc<AuxData<Vec<NotebookAuditResult>, C>>),
	NotaryMissingNotebooks(Arc<AuxData<BTreeSet<NotebookNumber>, C>>),
	VotesAtTick(Arc<AuxData<Vec<NotaryNotebookVotes>, C>>),
	NotaryAuditSummaries(Arc<AuxData<Vec<NotebookAuditSummary>, C>>),
	ForkVotingPower(Arc<AuxData<ForkPower, C>>),
	MaxVotingPowerAtTick(Arc<AuxData<ForkPower, C>>),
}
trait AuxStateData {
	fn as_any(&self) -> &dyn Any;
}

impl<C: AuxStore + 'static> AuxStateData for AuxState<C> {
	fn as_any(&self) -> &dyn Any {
		match self {
			AuxState::NotaryStateAtTick(a) => a,
			AuxState::AuthorsAtHeight(a) => a,
			AuxState::NotaryNotebooks(a) => a,
			AuxState::NotaryMissingNotebooks(a) => a,
			AuxState::VotesAtTick(a) => a,
			AuxState::NotaryAuditSummaries(a) => a,
			AuxState::ForkVotingPower(a) => a,
			AuxState::MaxVotingPowerAtTick(a) => a,
		}
	}
}
#[derive(Clone, Encode, Decode, Debug, Hash, Eq, PartialEq)]
pub enum AuxKey {
	NotaryStateAtTick(Tick),
	AuthorsAtHeight(BlockNumber),
	NotaryNotebooks(NotaryId),
	NotaryMissingNotebooks(NotaryId),
	VotesAtTick(Tick),
	NotaryAuditSummaries(NotaryId),
	ForkVotingPower(H256),
	MaxVotingPowerAtTick(Tick),
}

impl AuxKey {
	pub fn default_state<C: AuxStore>(&self, client: Arc<C>) -> AuxState<C> {
		match self {
			AuxKey::NotaryStateAtTick(_) =>
				AuxState::NotaryStateAtTick(AuxData::new(client, self.clone()).into()),
			AuxKey::AuthorsAtHeight(_) =>
				AuxState::AuthorsAtHeight(AuxData::new(client, self.clone()).into()),
			AuxKey::NotaryNotebooks(_) =>
				AuxState::NotaryNotebooks(AuxData::new(client, self.clone()).into()),
			AuxKey::NotaryMissingNotebooks(_) =>
				AuxState::NotaryMissingNotebooks(AuxData::new(client, self.clone()).into()),
			AuxKey::VotesAtTick(_) =>
				AuxState::VotesAtTick(AuxData::new(client, self.clone()).into()),
			AuxKey::NotaryAuditSummaries(_) =>
				AuxState::NotaryAuditSummaries(AuxData::new(client, self.clone()).into()),
			AuxKey::ForkVotingPower(_) =>
				AuxState::ForkVotingPower(AuxData::new(client, self.clone()).into()),
			AuxKey::MaxVotingPowerAtTick(_) =>
				AuxState::MaxVotingPowerAtTick(AuxData::new(client, self.clone()).into()),
		}
	}
}

pub struct UlxAux<B: BlockT, C: AuxStore> {
	pub lock: Arc<RwLock<()>>,
	client: Arc<C>,
	state: Arc<RwLock<LruMap<AuxKey, AuxState<C>>>>,
	_block: std::marker::PhantomData<B>,
}

impl<B: BlockT, C: AuxStore> Clone for UlxAux<B, C> {
	fn clone(&self) -> Self {
		Self {
			client: self.client.clone(),
			state: self.state.clone(),
			lock: self.lock.clone(),
			_block: Default::default(),
		}
	}
}

impl<B: BlockT, C: AuxStore> UlxAux<B, C> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			state: Arc::new(RwLock::new(LruMap::new(ByLength::new(500)))),
			lock: Default::default(),
			_block: Default::default(),
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
impl<B: BlockT, C: AuxStore + 'static> UlxAux<B, C> {
	#[allow(clippy::too_many_arguments)]
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
	) -> Result<(ForkPower, ForkPower), Error> {
		let _lock = self.lock.write();
		let block_number =
			UniqueSaturatedInto::<u32>::unique_saturated_into(*block.header.number());
		let strongest_at_height = self.strongest_fork_at_tick(tick)?.get();

		// add author to voting key
		if let Some(voting_key) = voting_key {
			self.authors_by_voting_key_at_height(block_number)?
				.mutate(|authors_at_height| {
					if !authors_at_height.entry(voting_key).or_default().insert(author.clone()) {
						return Err(Error::DuplicateAuthoredBlock(author).into());
					}
					Ok::<(), Error>(())
				})??;
		}

		let parent_hash = block.header.parent_hash();
		let mut fork_power = self.get_fork_voting_power(parent_hash)?.get();
		fork_power.add(block_voting_power, notebooks, seal_digest, compute_difficulty);

		if fork_power > strongest_at_height {
			let key = AuxKey::MaxVotingPowerAtTick(tick).encode();
			block.auxiliary.push((key, Some(fork_power.encode())));
		}

		let best_header_fork_power = self.get_fork_voting_power(&best_header.hash())?.get();

		block.auxiliary.push((
			AuxKey::ForkVotingPower(H256::from_slice(block.post_hash().as_ref())).encode(),
			Some(fork_power.encode()),
		));

		// cleanup old votes (None deletes)
		if tick >= 10 {
			let cleanup_height = tick - 10;
			block.auxiliary.push((AuxKey::VotesAtTick(cleanup_height).encode(), None));
			block
				.auxiliary
				.push((AuxKey::MaxVotingPowerAtTick(cleanup_height).encode(), None));
			block
				.auxiliary
				.push((AuxKey::AuthorsAtHeight(block_number.saturating_sub(5)).encode(), None));
		}
		// Cleanup old notary state. We keep this longer because we might need to catchup on
		// notebooks
		if tick >= 256 {
			block.auxiliary.push((AuxKey::NotaryStateAtTick(tick - 256).encode(), None));
		}

		Ok((fork_power, best_header_fork_power))
	}

	pub fn get_notary_notebooks_for_header(
		&self,
		notary_id: NotaryId,
		latest_runtime_notebook_number: NotebookNumber,
		submitting_tick: Tick,
	) -> Result<
		(NotebookHeaderData<NotebookVerifyError>, Option<NotaryNotebookVoteDigestDetails>),
		Error,
	> {
		let mut headers = NotebookHeaderData::default();
		let mut tick_notebook = None;
		let audit_results = self.get_notary_audit_history(notary_id)?;

		for notebook in audit_results.get() {
			if notebook.notebook_number <= latest_runtime_notebook_number ||
				notebook.tick > submitting_tick
			{
				continue;
			}
			let tick = notebook.tick;

			let state = self.get_notebook_tick_state(tick)?.get();
			if tick == submitting_tick {
				let details =
					state.notebook_key_details_by_notary.get(&notary_id).ok_or_else(|| {
						Error::NotebookHeaderBuildError(format!(
							"Unable to find notebook  #{} key details for notary {} at tick {}",
							notebook.notebook_number, notary_id, tick
						))
					})?;
				tick_notebook = Some(details.clone());
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
	) -> Result<Arc<AuxData<BTreeSet<NotebookNumber>, C>>, Error> {
		let key = AuxKey::NotaryMissingNotebooks(notary_id);
		self.get_or_insert_state(key)
	}

	/// Keeps a manually truncated vec of the last 2000 notary audit results
	pub fn get_notary_audit_history(
		&self,
		notary_id: NotaryId,
	) -> Result<Arc<AuxData<Vec<NotebookAuditResult>, C>>, Error> {
		let key = AuxKey::NotaryNotebooks(notary_id);
		self.get_or_insert_state(key)
	}

	pub fn authors_by_voting_key_at_height(
		&self,
		block_number: BlockNumber,
	) -> Result<Arc<AuxData<BTreeMap<H256, BTreeSet<AccountId>>, C>>, Error> {
		let key = AuxKey::AuthorsAtHeight(block_number);
		self.get_or_insert_state(key)
	}

	/// Retrieves aggregate voting power for a fork
	pub fn get_fork_voting_power(
		&self,
		block_hash: &B::Hash,
	) -> Result<Arc<AuxData<ForkPower, C>>, Error> {
		let key = AuxKey::ForkVotingPower(H256::from_slice(block_hash.as_ref()));
		self.get_or_insert_state(key)
	}

	pub fn store_votes(&self, tick: Tick, votes: NotaryNotebookVotes) -> Result<(), Error> {
		self.get_votes(tick)?.mutate(|existing| {
			if !existing.iter().any(|x| {
				x.notary_id == votes.notary_id && x.notebook_number == votes.notebook_number
			}) {
				existing.push(votes);
			}
		})?;
		Ok(())
	}

	pub fn store_audit_summary(
		&self,
		summary: NotebookAuditSummary,
		oldest_tick_to_keep: Tick,
	) -> Result<(), Error> {
		let notary_id = summary.notary_id;
		self.get_audit_summaries(notary_id)?.mutate(|summaries| {
			summaries.retain(|s| s.tick >= oldest_tick_to_keep);
			if !summaries.iter().any(|s| s.notebook_number == summary.notebook_number) {
				summaries.push(summary);
				summaries.sort_by(|a, b| a.notebook_number.cmp(&b.notebook_number));
			}
		})?;
		Ok(())
	}

	pub fn get_votes(
		&self,
		tick: Tick,
	) -> Result<Arc<AuxData<Vec<NotaryNotebookVotes>, C>>, Error> {
		let key = AuxKey::VotesAtTick(tick);
		self.get_or_insert_state(key)
	}

	pub fn get_audit_summaries(
		&self,
		notary_id: NotaryId,
	) -> Result<Arc<AuxData<Vec<NotebookAuditSummary>, C>>, Error> {
		let key = AuxKey::NotaryAuditSummaries(notary_id);
		self.get_or_insert_state(key)
	}

	pub fn strongest_fork_at_tick(&self, tick: Tick) -> Result<Arc<AuxData<ForkPower, C>>, Error> {
		let key = AuxKey::MaxVotingPowerAtTick(tick);
		self.get_or_insert_state(key)
	}

	pub fn store_notebook_result(
		&self,
		notary_id: NotaryId,
		audit_result: NotebookAuditResult,
		raw_signed_header: Vec<u8>,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<NotaryNotebookTickState, Error> {
		let notary_state = self.update_tick_state(raw_signed_header, &vote_details)?;

		self.get_notary_audit_history(notary_id)?.mutate(|notebooks| {
			if !notebooks.iter().any(|n| n.notebook_number == audit_result.notebook_number) {
				// look backwards for the first index where the notebook number is less than the
				// current
				let mut index = notebooks.len();
				for (i, n) in notebooks.iter().enumerate().rev() {
					if n.notebook_number < audit_result.notebook_number {
						index = i + 1;
						break;
					}
				}
				notebooks.insert(index, audit_result.clone());
				if notebooks.len() > 2000 {
					notebooks.remove(0);
				}
			}
		})?;
		Ok(notary_state)
	}

	fn get_notebook_tick_state(
		&self,
		tick: Tick,
	) -> Result<Arc<AuxData<NotaryNotebookTickState, C>>, Error> {
		let key = AuxKey::NotaryStateAtTick(tick);
		self.get_or_insert_state(key)
	}

	fn update_tick_state(
		&self,
		raw_signed_header: Vec<u8>,
		vote_details: &NotaryNotebookVoteDetails<B::Hash>,
	) -> Result<NotaryNotebookTickState, Error> {
		let tick = vote_details.tick;
		let notary_id = vote_details.notary_id;
		self.get_notebook_tick_state(tick)?.mutate(|state| {
			let vote_details = NotaryNotebookVoteDigestDetails::from(vote_details);

			info!(
				"Storing vote details for tick {} and notary {} at notebook #{}",
				tick, notary_id, vote_details.notebook_number
			);

			if state.notebook_key_details_by_notary.insert(notary_id, vote_details).is_none() {
				state.raw_headers_by_notary.insert(notary_id, raw_signed_header);
			}
			Ok(state.clone())
		})?
	}

	fn get_or_insert_state<T: 'static + Clone>(
		&self,
		key: AuxKey,
	) -> Result<Arc<AuxData<T, C>>, Error> {
		let mut state = self.state.write();
		let entry = state
			.get_or_insert(key.clone(), || key.default_state(self.client.clone()).into())
			.ok_or(Error::StringError(format!("Error unlocking notary state for {key:?}")))?;
		if let Some(data) = entry.as_any().downcast_ref::<Arc<AuxData<T, C>>>() {
			Ok(data.clone())
		} else {
			Err(format!("Could not downcast AuxState for {key:?}").into())
		}
	}
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
			ForkPower { voting_power: 1.into(), ..Default::default() } >
				ForkPower { voting_power: 0.into(), ..Default::default() }
		);

		assert!(
			ForkPower { notebooks: 1, ..Default::default() } >
				ForkPower { notebooks: 0, ..Default::default() }
		);

		assert!(
			ForkPower { seal_strength: 200.into(), ..Default::default() } >
				ForkPower { seal_strength: 201.into(), ..Default::default() }
		);

		assert!(
			ForkPower { total_compute_difficulty: 1000.into(), ..Default::default() } >
				ForkPower { total_compute_difficulty: 999.into(), ..Default::default() }
		);
	}
}
